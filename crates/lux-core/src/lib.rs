use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Context;
use serde::Serialize;

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .context("failed to serialize value for atomic write")?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace file {}", path.display()))
}

pub fn append_jsonl<T: Serialize>(path: &Path, event: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let line = serde_json::to_string(event).context("failed to serialize jsonl event")?;
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open jsonl file for append {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", line)
        .with_context(|| format!("failed to append jsonl line to {}", path.display()))?;
    writer.flush().context("failed to flush jsonl file")?;
    Ok(())
}

pub fn read_jsonl<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read jsonl file {}", path.display()))?;
    let mut events = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(event) => events.push(event),
            Err(err) => {
                eprintln!(
                    "[lux-io] Skipping malformed jsonl line {}: {}",
                    line_num + 1,
                    err
                );
            }
        }
    }
    Ok(events)
}

pub fn write_evidence_file(
    project_root: &Path,
    relative_path: &str,
    content: &str,
    max_bytes: usize,
) -> anyhow::Result<String> {
    let abs_path = project_root.join(relative_path);
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create evidence directory {}", parent.display()))?;
    }
    let truncated = if content.len() > max_bytes {
        &content[..max_bytes]
    } else {
        content
    };
    let tmp_path = abs_path.with_extension("txt.tmp");
    fs::write(&tmp_path, truncated)
        .with_context(|| format!("failed to write evidence tmp {}", tmp_path.display()))?;
    fs::rename(&tmp_path, &abs_path).with_context(|| {
        format!(
            "failed to atomically replace evidence file {}",
            abs_path.display()
        )
    })?;
    Ok(relative_path.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{json, Value};

    use super::{append_jsonl, atomic_write_json, read_jsonl, write_evidence_file, CRATE_NAME};

    fn test_dir(name: &str) -> anyhow::Result<PathBuf> {
        let dir = std::env::temp_dir().join(format!("lux-core-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-core");
    }

    #[test]
    fn atomic_write_json_is_exported_for_shared_io_when_called() -> anyhow::Result<()> {
        let dir = test_dir("atomic-write-json")?;
        let path = dir.join("nested").join("state.json");

        atomic_write_json(&path, &json!({ "status": "ok" }))?;

        let content = std::fs::read_to_string(path)?;
        assert!(content.contains("\"status\": \"ok\""));
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn jsonl_helpers_preserve_valid_events_when_malformed_lines_exist() -> anyhow::Result<()> {
        let dir = test_dir("jsonl-malformed")?;
        let path = dir.join("events.jsonl");

        append_jsonl(&path, &json!({ "id": 1 }))?;
        std::fs::write(&path, "{\"id\":1}\nnot-json\n{\"id\":2}\n")?;

        let events = read_jsonl::<Value>(&path)?;

        assert_eq!(events, vec![json!({ "id": 1 }), json!({ "id": 2 })]);
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn write_evidence_file_truncates_and_returns_relative_path() -> anyhow::Result<()> {
        let dir = test_dir("evidence")?;
        let relative_path = "evidence/task.txt";

        let written = write_evidence_file(&dir, relative_path, "abcdef", 3)?;

        assert_eq!(written, relative_path);
        assert_eq!(std::fs::read_to_string(dir.join(relative_path))?, "abc");
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }
}
