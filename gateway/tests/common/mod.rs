//! Shared test utilities for LUX smoke test suites.
//!
//! Consolidates duplicated helper functions (temp_dir, jsonl I/O, cleanup)
//! into a single module so each smoke test file can `use common::*;`.

#![allow(dead_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use lux::ai_log::AiWorkStep;
use lux::protocol::LuxEvent;
use serde_json::json;

// ---------------------------------------------------------------------------
// Temp directory — AtomicU64 counter (from jsonl_persistence_smoke baseline)
// ---------------------------------------------------------------------------

/// Create a uniquely-named temporary directory inside the OS temp dir.
///
/// Uses a global `AtomicU64` counter so concurrent tests never collide,
/// even when called within the same nanosecond window.
pub fn temp_dir_unique(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("lux-test-{prefix}-{pid}-{tid:?}-{id}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------------------
// JSONL I/O helpers
// ---------------------------------------------------------------------------

/// Write an array of string slices as JSONL (one JSON object per line).
pub fn write_jsonl(path: &Path, lines: &[&str]) {
    let mut f = File::create(path).unwrap_or_else(|e| panic!("create {path:?}: {e}"));
    for line in lines {
        writeln!(f, "{line}").unwrap();
    }
    f.flush().unwrap();
}

/// Read all JSON values from a JSONL file.
pub fn read_jsonl_values(path: &Path) -> Vec<serde_json::Value> {
    use std::io::BufReader;
    let f = File::open(path).unwrap_or_else(|e| panic!("open {path:?}: {e}"));
    lux::ai_log::parse_jsonl_values(BufReader::new(f))
}

/// Create a uniquely-named temporary path (file or dir) inside OS temp dir.
/// Uses AtomicU64 counter for uniqueness across concurrent tests.
pub fn tmp_path(suffix: &str) -> PathBuf {
    let dir = temp_dir_unique(suffix);
    dir.join(format!("{suffix}.jsonl"))
}

/// Create a sample log file with 3 standard entries and return its path.
pub fn sample_log_path() -> PathBuf {
    let dir = temp_dir_unique("sample");
    let p = dir.join("sample.jsonl");
    write_jsonl(
        &p,
        &[
            r#"{"timestampUtc":"2026-05-01T10:00:00Z","actor":"codex","category":"tool","source":"gateway","action":"compile","eventType":"start","summary":"Started cargo build"}"#,
            r#"{"timestampUtc":"2026-05-01T10:01:00Z","actor":"opencode","category":"ai-action-log","source":"gateway","action":"append","eventType":"append","message":"Appended log entry"}"#,
            r#"{"timestampUtc":"2026-05-01T10:02:00Z","actor":"codex","category":"tool","source":"gateway","action":"test","eventType":"complete","description":"Tests passed"}"#,
        ],
    );
    p
}

/// Read all lines from a JSONL file and return them as raw strings.
pub fn read_jsonl_lines(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    content.lines().map(String::from).collect()
}

// ---------------------------------------------------------------------------
// File / log helpers
// ---------------------------------------------------------------------------

/// Create a temporary log file inside *dir* with the given *name* and return its path.
pub fn create_temp_log_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    File::create(&path).unwrap_or_else(|e| panic!("create {path:?}: {e}"));
    path
}

/// Cleanup helper — removes file or directory tree silently.
pub fn cleanup(path: &Path) {
    let _ = if path.is_file() {
        fs::remove_file(path)
    } else {
        fs::remove_dir_all(path)
    };
}

/// Write raw *content* to a temp jsonl path and return the path.
pub fn write_temp_jsonl(label: &str, content: &str) -> PathBuf {
    let dir = temp_dir_unique(label);
    let path = dir.join(format!("{label}.jsonl"));
    fs::write(&path, content).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    path
}

// ---------------------------------------------------------------------------
// Sample data builders
// ---------------------------------------------------------------------------

/// Build a sample `AiWorkStep` with the given *step_name*.
pub fn make_step(name: &str) -> AiWorkStep {
    AiWorkStep {
        step_name: name.to_string(),
        status: "completed".to_string(),
        tool: Some("test-runner".to_string()),
        action: Some(name.to_string()),
        summary: Some(format!("Test step {name}")),
        redaction_metadata: None,
        timestamp_utc: "2026-05-10T07:00:00Z".to_string(),
    }
}

/// Build a minimal `LuxEvent` (alias for `EventEnvelope`) for smoke testing.
pub fn make_sample_event(source: &str) -> LuxEvent {
    use lux::protocol::{EventCategory, EventSource};
    LuxEvent {
        schema_version: 1,
        event_id: format!("evt-{source}-smoke"),
        category: EventCategory::Tool,
        source: match source {
            "editor" => EventSource::Editor,
            "ai" => EventSource::Ai,
            "runtime" => EventSource::Runtime,
            _ => EventSource::Editor,
        },
        session_id: format!("sess-{source}-smoke"),
        captured_at_utc: "2026-05-10T12:00:00Z".to_string(),
        project_path: None,
        summary: Some(format!("Smoke test event from {source}")),
        redaction_metadata: None,
        retention_metadata: None,
        payload: json!({"message": format!("sample {source}")}),
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert that a JSONL file contains exactly *expected* non-empty lines.
pub fn assert_jsonl_line_count(path: &Path, expected: usize) {
    let lines = read_jsonl_lines(path);
    let non_empty: Vec<&str> = lines
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(
        non_empty.len(),
        expected,
        "expected {expected} JSONL lines in {:?}, got {}",
        path,
        non_empty.len()
    );
}
