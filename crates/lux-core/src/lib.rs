use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
    reject_or_remove_legacy_temp(&path.with_extension("json.tmp"))?;
    let tmp_path = unique_temp_path(path)?;
    write_new_file_synced(&tmp_path, content.as_bytes())?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace file {}", path.display()))
}

pub fn append_jsonl<T: Serialize>(path: &Path, event: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let line = serde_json::to_string(event).context("failed to serialize jsonl event")?;
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        anyhow::bail!("jsonl file must not be a symlink: {}", path.display());
    }
    #[cfg(unix)]
    if fs::metadata(path)
        .map(|metadata| metadata.nlink() > 1)
        .unwrap_or(false)
    {
        anyhow::bail!("jsonl file must not be hardlinked: {}", path.display());
    }
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .with_context(|| format!("failed to open jsonl file for append {}", path.display()))?;
    #[cfg(unix)]
    if file
        .metadata()
        .with_context(|| format!("failed to inspect jsonl file {}", path.display()))?
        .nlink()
        > 1
    {
        anyhow::bail!("jsonl file must not be hardlinked: {}", path.display());
    }
    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", line)
        .with_context(|| format!("failed to append jsonl line to {}", path.display()))?;
    let file = writer
        .into_inner()
        .context("failed to flush jsonl writer before sync")?;
    file.sync_all()
        .with_context(|| format!("failed to sync jsonl file {}", path.display()))?;
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
    reject_or_remove_legacy_temp(&abs_path.with_extension("txt.tmp"))?;
    let tmp_path = unique_temp_path(&abs_path)?;
    write_new_file_synced(&tmp_path, truncated.as_bytes())?;
    fs::rename(&tmp_path, &abs_path).with_context(|| {
        format!(
            "failed to atomically replace evidence file {}",
            abs_path.display()
        )
    })?;
    Ok(relative_path.to_string())
}

fn write_new_file_synced(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        anyhow::bail!("temporary file must not be a symlink: {}", path.display());
    }
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to write temporary file {}", path.display()))?;
    file.write_all(content)
        .with_context(|| format!("failed to write temporary file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync temporary file {}", path.display()))
}

fn reject_or_remove_legacy_temp(path: &Path) -> anyhow::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect temporary file {}", path.display()));
        }
    };
    if metadata.file_type().is_symlink() {
        anyhow::bail!("temporary file must not be a symlink: {}", path.display());
    }
    #[cfg(unix)]
    if metadata.nlink() > 1 {
        anyhow::bail!("temporary file must not be hardlinked: {}", path.display());
    }
    if metadata.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove stale temporary file {}", path.display()))?;
    }
    Ok(())
}

fn unique_temp_path(path: &Path) -> anyhow::Result<std::path::PathBuf> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no valid UTF-8 file name", path.display()))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos();
    Ok(parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nanos)))
}
