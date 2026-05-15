mod common;
use common::*;

use lux::ai_log::{
    append_work_step, compact_log_file, ensure_log_path, parse_jsonl_values, read_log_entries,
    AiLogFilter, AiWorkStep,
};
use std::fs;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

// ===========================================================================
// AC2-a: Default path resolution returns .lux/ai-action-log.jsonl
// ===========================================================================

#[test]
fn default_path_resolves_to_dot_lux_ai_action_log() {
    let _guard = env_guard();
    std::env::remove_var("LUX_EVENT_LOG_PATH");
    let root = Path::new("/unity/project");
    let resolved = lux::ai_log::resolve_log_path(root);
    assert_eq!(
        resolved,
        PathBuf::from("/unity/project/.lux/ai-action-log.jsonl"),
        "Default path must be <project>/.lux/ai-action-log.jsonl"
    );
}

#[test]
fn default_path_uses_forward_slashes_on_windows_paths() {
    let _guard = env_guard();
    std::env::remove_var("LUX_EVENT_LOG_PATH");
    let resolved = lux::ai_log::resolve_log_path(r#"C:\Unity\MyProject"#);
    assert_eq!(
        resolved.to_string_lossy(),
        "C:/Unity/MyProject/.lux/ai-action-log.jsonl"
    );
}

#[test]
fn ensure_log_path_creates_dot_lux_directory_and_returns_path() {
    let _guard = env_guard();
    std::env::remove_var("LUX_EVENT_LOG_PATH");
    let root = temp_dir_unique("ensure-default");
    let log_path = ensure_log_path(&root).unwrap();

    assert_eq!(log_path, root.join(".lux/ai-action-log.jsonl"));
    assert!(
        log_path.parent().unwrap().exists(),
        ".lux directory must be created"
    );

    cleanup(&root);
}

// ===========================================================================
// AC2-b: LUX_EVENT_LOG_PATH env var overrides default path
// ===========================================================================

#[test]
fn env_var_lux_event_log_path_overrides_default_resolution() {
    let _guard = env_guard();
    let custom = "/custom/log/events.jsonl";
    std::env::set_var("LUX_EVENT_LOG_PATH", custom);

    let resolved = lux::ai_log::resolve_log_path("/any/project");
    assert_eq!(resolved, PathBuf::from(custom));

    std::env::remove_var("LUX_EVENT_LOG_PATH");
}

#[test]
fn empty_env_var_falls_back_to_default_path() {
    let _guard = env_guard();
    std::env::remove_var("LUX_EVENT_LOG_PATH");
    std::env::set_var("LUX_EVENT_LOG_PATH", "");

    let resolved = lux::ai_log::resolve_log_path("/project");
    assert_eq!(resolved, PathBuf::from("/project/.lux/ai-action-log.jsonl"));

    std::env::remove_var("LUX_EVENT_LOG_PATH");
}

#[test]
fn ensure_log_path_respects_env_override() {
    let _guard = env_guard();
    let root = temp_dir_unique("ensure-env");
    let custom = root.join("custom-dir/my-log.jsonl");
    std::env::set_var("LUX_EVENT_LOG_PATH", custom.to_str().unwrap());

    let log_path = ensure_log_path(&root).unwrap();
    assert_eq!(log_path, custom);
    // Parent dir should be created
    assert!(log_path.parent().unwrap().exists());

    std::env::remove_var("LUX_EVENT_LOG_PATH");
    cleanup(&root);
}

// ===========================================================================
// AC2-c: Append-safe write — multiple appends produce all lines in order
// ===========================================================================

#[test]
fn append_work_step_writes_multiple_entries_in_order() {
    let dir = temp_dir_unique("append-order");
    let log_path = dir.join("append-test.jsonl");

    let steps: Vec<&str> = vec!["alpha", "beta", "gamma", "delta"];
    for name in &steps {
        append_work_step(&log_path, &make_step(name)).unwrap();
    }

    let content = fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(
        lines.len(),
        4,
        "Expected 4 appended lines, got {}",
        lines.len()
    );
    for (i, name) in steps.iter().enumerate() {
        let expected = format!("\"stepName\":\"{name}\"");
        assert!(
            lines[i].contains(expected.as_str()),
            "Line {} should contain stepName={}, got: {}",
            i + 1,
            name,
            lines[i]
        );
    }

    cleanup(&dir);
}

#[test]
fn append_work_step_is_idempotent_across_calls() {
    let dir = temp_dir_unique("append-idempotent");
    let log_path = dir.join("append-test.jsonl");

    for _ in 0..5 {
        append_work_step(&log_path, &make_step("repeat")).unwrap();
    }

    let entries = read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 5);
    for e in &entries {
        assert_eq!(e.value["stepName"], "repeat");
    }

    cleanup(&dir);
}

#[test]
fn append_work_step_creates_parent_directory_automatically() {
    let dir = temp_dir_unique("append-autocreate");
    let nested = dir.join("a/b/c/log.jsonl");

    append_work_step(&nested, &make_step("auto-mkdir")).unwrap();
    assert!(nested.exists());
    let entries = read_log_entries(&nested, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);

    cleanup(&dir);
}

// ===========================================================================
// AC2-d: Corrupt line in middle of file — reader skips it, returns valid lines
// ===========================================================================

#[test]
fn parse_jsonl_values_skips_corrupt_line_in_middle() {
    // Build input programmatically to avoid escape issues
    let mut input = String::new();
    input.push_str(r#"{"n":1}"#);
    input.push('\n');
    input.push_str("THIS IS NOT JSON\n");
    input.push_str(r#"{"n":2}"#);
    input.push('\n');
    input.push_str("{broken}\n");
    input.push_str(r#"{"n":3}"#);
    input.push('\n');

    let values = parse_jsonl_values(Cursor::new(input));

    assert_eq!(values.len(), 3, "Should return 3 valid JSON lines");
    assert_eq!(values[0]["n"], 1);
    assert_eq!(values[1]["n"], 2);
    assert_eq!(values[2]["n"], 3);
}

#[test]
fn read_log_entries_skips_corrupt_lines_in_file() {
    let dir = temp_dir_unique("corrupt-file");
    let log_path = dir.join("corrupt.jsonl");

    let mut content = String::new();
    content.push_str(r#"{"actor":"alice"}"#);
    content.push('\n');
    content.push_str("garbage_here\n");
    content.push_str(r#"{"actor":"bob"}"#);
    content.push('\n');
    content.push_str("more-garbage\n");
    content.push_str(r#"{"actor":"carol"}"#);
    content.push('\n');

    fs::write(&log_path, content).unwrap();

    let entries = read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].value["actor"], "alice");
    assert_eq!(entries[1].value["actor"], "bob");
    assert_eq!(entries[2].value["actor"], "carol");

    cleanup(&dir);
}

#[test]
fn parse_jsonl_values_all_corrupt_returns_empty() {
    let values = parse_jsonl_values(Cursor::new("not json\n{{{{\n!!!\n"));
    assert!(values.is_empty(), "All-corrupt input must yield empty vec");
}

// ===========================================================================
// AC2-e: Truncated/incomplete last line handled gracefully
// ===========================================================================

#[test]
fn truncated_last_line_does_not_crash_reader() {
    let mut input = String::new();
    input.push_str(r#"{"n":1}"#);
    input.push('\n');
    input.push_str(r#"{"n":2}"#);
    input.push('\n');
    input.push_str("{\"incomplete"); // no closing quote or brace

    let values = parse_jsonl_values(Cursor::new(input));
    assert_eq!(values.len(), 2, "Truncated last line should be skipped");
    assert_eq!(values[0]["n"], 1);
    assert_eq!(values[1]["n"], 2);
}

#[test]
fn truncated_last_line_in_file_read() {
    let dir = temp_dir_unique("truncated-file");
    let log_path = dir.join("truncated.jsonl");

    // Write without trailing newline to simulate truncation
    let mut content = String::new();
    content.push_str(r#"{"ok":1}"#);
    content.push('\n');
    content.push_str("{\"truncated"); // truncated, no newline
    fs::write(&log_path, content).unwrap();

    let entries = read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value["ok"], 1);

    cleanup(&dir);
}

#[test]
fn only_truncated_line_returns_empty() {
    let values = parse_jsonl_values(Cursor::new("{\"hello"));
    assert!(values.is_empty());
}

// ===========================================================================
// AC2-f: Empty file returns empty result, no crash
// ===========================================================================

#[test]
fn empty_file_returns_empty_entries_no_crash() {
    let dir = temp_dir_unique("empty-file");
    let log_path = dir.join("empty.jsonl");

    File::create(&log_path).unwrap(); // truly empty

    let entries = read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    assert!(entries.is_empty(), "Empty file must yield zero entries");

    cleanup(&dir);
}

#[test]
fn whitespace_only_file_returns_empty() {
    let values = parse_jsonl_values(Cursor::new("   \n\n  \n  \n"));
    assert!(values.is_empty());
}

#[test]
fn parse_jsonl_on_empty_cursor() {
    let values = parse_jsonl_values(Cursor::new(""));
    assert!(values.is_empty());
}

// ===========================================================================
// AC2-g: Legacy path migration (UserSettings → .lux)
// ===========================================================================

#[test]
fn legacy_log_migrated_from_user_settings_to_lux() {
    let _guard = env_guard();
    let root = temp_dir_unique("legacy-migrate");
    let user_settings = root.join("UserSettings");
    fs::create_dir_all(&user_settings).unwrap();
    let legacy_path = user_settings.join("LuxAiActionLog.jsonl");
    fs::write(&legacy_path, r#"{"actor":"legacy-agent"}"#).unwrap();
    fs::write(&legacy_path, "\n").unwrap(); // append newline from prior write... actually overwrite

    // Rewrite properly
    fs::write(&legacy_path, r#"{"actor":"legacy-agent"}"#).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&legacy_path)
        .unwrap()
        .write_all(b"\n")
        .ok();

    let log_path = ensure_log_path(&root).unwrap();

    // New location should have the data
    assert_eq!(log_path, root.join(".lux/ai-action-log.jsonl"));
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("legacy-agent"));

    // Old location should be gone
    assert!(
        !legacy_path.exists(),
        "Legacy file must be removed after migration"
    );

    cleanup(&root);
}

#[test]
fn legacy_migration_skipped_when_new_path_already_exists() {
    let _guard = env_guard();
    let root = temp_dir_unique("legacy-skip-new-exists");
    let user_settings = root.join("UserSettings");
    fs::create_dir_all(&user_settings).unwrap();
    let legacy_path = user_settings.join("LuxAiActionLog.jsonl");
    fs::write(&legacy_path, r#"{"actor":"old"}"#).unwrap();

    // Pre-create new path
    let lux_dir = root.join(".lux");
    fs::create_dir_all(&lux_dir).unwrap();
    let new_path = lux_dir.join("ai-action-log.jsonl");
    fs::write(&new_path, r#"{"actor":"new"}"#).unwrap();

    let log_path = ensure_log_path(&root).unwrap();
    assert_eq!(log_path, new_path);

    // Legacy should still exist (not migrated because new exists)
    assert!(
        legacy_path.exists(),
        "Legacy must remain when new path already exists"
    );
    // New content unchanged
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains(r#""actor":"new""#));

    cleanup(&root);
}

#[test]
fn no_legacy_path_means_clean_new_path() {
    let _guard = env_guard();
    std::env::remove_var("LUX_EVENT_LOG_PATH");
    let root = temp_dir_unique("no-legacy");

    let log_path = ensure_log_path(&root).unwrap();
    assert_eq!(log_path, root.join(".lux/ai-action-log.jsonl"));
    assert!(log_path.parent().unwrap().exists());

    cleanup(&root);
}

// ===========================================================================
// AC2-h: Atomic rename pattern in compact_log_file (write tmp → rename)
// ===========================================================================

#[test]
fn compact_log_file_uses_atomic_rename_pattern() {
    let dir = temp_dir_unique("compact-atomic");
    let log_path = dir.join("compact.jsonl");

    // Write a file with mixed valid/invalid lines
    let mut content = String::new();
    content.push_str(r#"{"n":1}"#);
    content.push('\n');
    content.push_str("invalid-line\n");
    content.push_str(r#"{"n":2}"#);
    content.push('\n');
    content.push_str("  \n");
    content.push_str(r#"{"n":3}"#);
    content.push('\n');
    content.push_str(r#"{"n":4}"#);
    content.push('\n');
    content.push_str(r#"{"n":5}"#);
    content.push('\n');

    fs::write(&log_path, content).unwrap();

    let result = compact_log_file(&log_path, 2).unwrap();

    // Verify result metadata
    assert_eq!(result.valid_before, 5); // 5 valid JSON lines
    assert_eq!(result.valid_after, 2); // kept last 2
    assert_eq!(result.invalid_dropped, 1); // 1 non-empty invalid line
    assert_eq!(result.lines_dropped, 4); // 3 valid dropped + 1 invalid

    // Verify file contents after compaction
    let compacted = fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = compacted.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains(r#""n":4"#));
    assert!(lines[1].contains(r#""n":5"#));

    // No .tmp file should remain
    let tmp_path = log_path.with_extension("jsonl.tmp");
    assert!(
        !tmp_path.exists(),
        "Temp file must not exist after atomic rename"
    );

    cleanup(&dir);
}

#[test]
fn compact_log_file_on_all_valid_keeps_tail() {
    let dir = temp_dir_unique("compact-valid-only");
    let log_path = dir.join("valid-only.jsonl");

    let mut content = String::new();
    content.push_str(r#"{"v":1}"#);
    content.push('\n');
    content.push_str(r#"{"v":2}"#);
    content.push('\n');
    content.push_str(r#"{"v":3}"#);
    content.push('\n');
    fs::write(&log_path, content).unwrap();

    let result = compact_log_file(&log_path, 2).unwrap();
    assert_eq!(result.valid_before, 3);
    assert_eq!(result.valid_after, 2);
    assert_eq!(result.invalid_dropped, 0);

    let compacted = fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        compacted,
        r#"{"v":2}
{"v":3}
"#
    );

    cleanup(&dir);
}

// ===========================================================================
// Bonus: Round-trip integrity — write then read preserves all fields
// ===========================================================================

#[test]
fn round_trip_preserves_all_ai_work_step_fields() {
    let dir = temp_dir_unique("roundtrip");
    let log_path = dir.join("roundtrip.jsonl");

    let original = AiWorkStep {
        step_name: "deep-analysis".to_string(),
        status: "running".to_string(),
        tool: Some("opencode-v2".to_string()),
        action: Some("analyze-codebase".to_string()),
        summary: Some("Full codebase scan completed".to_string()),
        redaction_metadata: None,
        timestamp_utc: "2026-05-10T06:30:00Z".to_string(),
    };

    append_work_step(&log_path, &original).unwrap();
    let entries = read_log_entries(&log_path, &AiLogFilter::default()).unwrap();

    assert_eq!(entries.len(), 1);
    let v = &entries[0].value;
    assert_eq!(v["stepName"], "deep-analysis");
    assert_eq!(v["status"], "running");
    assert_eq!(v["tool"], "opencode-v2");
    assert_eq!(v["action"], "analyze-codebase");
    assert_eq!(v["summary"], "Full codebase scan completed");
    assert_eq!(v["timestampUtc"], "2026-05-10T06:30:00Z");

    cleanup(&dir);
}

// ===========================================================================
// Bonus: Filter with limit works on persisted file
// ===========================================================================

#[test]
fn filter_limit_tails_persisted_events() {
    let dir = temp_dir_unique("filter-tail");
    let log_path = dir.join("filter.jsonl");

    for i in 0..10u32 {
        let step = AiWorkStep {
            step_name: format!("step-{i}"),
            status: "done".to_string(),
            tool: None,
            action: None,
            summary: None,
            redaction_metadata: None,
            timestamp_utc: format!("2026-05-10T07:{i:02}:00Z"),
        };
        append_work_step(&log_path, &step).unwrap();
    }

    let filter = AiLogFilter {
        limit: Some(3),
        ..AiLogFilter::default()
    };
    let entries = read_log_entries(&log_path, &filter).unwrap();
    assert_eq!(entries.len(), 3);
    // Should get last 3: step-7, step-8, step-9
    assert_eq!(entries[0].value["stepName"], "step-7");
    assert_eq!(entries[2].value["stepName"], "step-9");

    cleanup(&dir);
}
