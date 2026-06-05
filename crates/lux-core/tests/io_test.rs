use std::path::PathBuf;

use lux_core::{append_jsonl, atomic_write_json, read_jsonl, write_evidence_file, CRATE_NAME};
use serde_json::{json, Value};

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

#[cfg(unix)]
#[test]
fn atomic_write_json_rejects_symlinked_temp_file() -> anyhow::Result<()> {
    let dir = test_dir("atomic-write-symlink")?;
    let path = dir.join("state.json");
    let outside = dir.join("outside.json");
    std::fs::write(&outside, "outside-original")?;
    std::os::unix::fs::symlink(&outside, path.with_extension("json.tmp"))?;

    let error =
        atomic_write_json(&path, &json!({ "status": "ok" })).expect_err("symlink temp rejected");

    assert!(error
        .to_string()
        .contains("temporary file must not be a symlink"));
    assert_eq!(std::fs::read_to_string(outside)?, "outside-original");
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn atomic_write_json_removes_stale_legacy_temp_file() -> anyhow::Result<()> {
    let dir = test_dir("atomic-write-stale-temp")?;
    let path = dir.join("state.json");
    std::fs::write(path.with_extension("json.tmp"), "stale")?;

    atomic_write_json(&path, &json!({ "status": "ok" }))?;

    assert!(!path.with_extension("json.tmp").exists());
    assert!(std::fs::read_to_string(path)?.contains("\"status\": \"ok\""));
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

#[cfg(unix)]
#[test]
fn append_jsonl_rejects_symlinked_log_file() -> anyhow::Result<()> {
    let dir = test_dir("jsonl-symlink")?;
    let path = dir.join("events.jsonl");
    let outside = dir.join("outside.jsonl");
    std::fs::write(&outside, "")?;
    std::os::unix::fs::symlink(&outside, &path)?;

    let error = append_jsonl(&path, &json!({ "id": 1 })).expect_err("symlink rejected");

    assert!(error
        .to_string()
        .contains("jsonl file must not be a symlink"));
    assert_eq!(std::fs::read_to_string(outside)?, "");
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn append_jsonl_rejects_hardlinked_log_file() -> anyhow::Result<()> {
    let dir = test_dir("jsonl-hardlink")?;
    let path = dir.join("events.jsonl");
    let outside = dir.join("outside.jsonl");
    std::fs::write(&outside, "outside-original")?;
    std::fs::hard_link(&outside, &path)?;

    let error = append_jsonl(&path, &json!({ "id": 1 })).expect_err("hardlink rejected");

    assert!(error
        .to_string()
        .contains("jsonl file must not be hardlinked"));
    assert_eq!(std::fs::read_to_string(outside)?, "outside-original");
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

#[cfg(unix)]
#[test]
fn write_evidence_file_rejects_symlinked_temp_file() -> anyhow::Result<()> {
    let dir = test_dir("evidence-symlink")?;
    std::fs::create_dir_all(dir.join("evidence"))?;
    let outside = dir.join("outside.txt");
    std::fs::write(&outside, "outside-original")?;
    std::os::unix::fs::symlink(&outside, dir.join("evidence/task.txt.tmp"))?;

    let error = write_evidence_file(&dir, "evidence/task.txt", "abcdef", 6)
        .expect_err("symlink temp rejected");

    assert!(error
        .to_string()
        .contains("temporary file must not be a symlink"));
    assert_eq!(std::fs::read_to_string(outside)?, "outside-original");
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn write_evidence_file_removes_stale_legacy_temp_file() -> anyhow::Result<()> {
    let dir = test_dir("evidence-stale-temp")?;
    std::fs::create_dir_all(dir.join("evidence"))?;
    std::fs::write(dir.join("evidence/task.txt.tmp"), "stale")?;

    let written = write_evidence_file(&dir, "evidence/task.txt", "abcdef", 6)?;

    assert_eq!(written, "evidence/task.txt");
    assert!(!dir.join("evidence/task.txt.tmp").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join("evidence/task.txt"))?,
        "abcdef"
    );
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
