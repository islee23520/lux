//! Shared I/O helpers for LUX — atomic writes and append-only JSONL event logs.
//! Enforces invariant #4 (Atomicity): all .lux/ writes use write-to-tmp + rename.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Context;
use serde::Serialize;

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
            Err(e) => {
                eprintln!(
                    "⚠️  [lux-io] Skipping malformed jsonl line {}: {}",
                    line_num + 1,
                    e
                );
            }
        }
    }
    Ok(events)
}
