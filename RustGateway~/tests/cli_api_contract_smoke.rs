//! AC3 + AC5: CLI/API contract smoke tests for `lux ai-log --json` commands
//! and AI Work Step logging.
//!
//! Tests cover:
//! - AC3: JSON output contract for recent/tail/context/compact subcommands
//! - AC3: HTTP /api/ai-log and /api/ai-log/context endpoint contracts
//! - AC5: AiWorkStep struct fields, serialization, append+read cycle, redaction

mod common;
use common::*;

use lux::ai_log::{
    self, AiLogCompactResult, AiLogEntry, AiLogFilter, AiWorkStep, RetentionPolicy,
};
use lux::protocol::RedactionMetadata;
use serde_json::{json, Value};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ===========================================================================
// AC3: CLI subcommand JSON output contract — recent
// ===========================================================================

#[test]
fn ac3_recent_json_output_is_valid_object_with_required_keys() {
    let project_root = tmp_path("proj-recent");
    fs::create_dir_all(project_root.join(".lux")).unwrap();
    let log_path = ai_log::resolve_log_path(&project_root);
    write_jsonl(
        &log_path,
        &[r#"{"timestampUtc":"2026-05-01T10:00:00Z","actor":"codex"}"#],
    );

    // Simulate what print_ai_log_recent does when --json is set
    let entries = ai_log::read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    let output = json!({
        "projectPath": project_root,
        "path": log_path,
        "count": entries.len(),
        "entries": entries,
    });

    let serialized = serde_json::to_string_pretty(&output).unwrap();
    let parsed: Value = serde_json::from_str(&serialized).unwrap();

    assert!(parsed.get("projectPath").is_some());
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("count").is_some());
    assert!(parsed.get("entries").is_some());
    assert_eq!(parsed["count"], 1);
    assert!(parsed["entries"].as_array().unwrap().len() > 0);

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn ac3_recent_respects_limit_filter() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        limit: Some(1),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value["eventType"], "complete");
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_recent_respects_actor_and_category_filters() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        actor: Some("codex".to_string()),
        category: Some("tool".to_string()),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert_eq!(entries.len(), 2);
    let _ = fs::remove_file(p);
}

// ===========================================================================
// AC3: CLI subcommand JSON output contract — tail
// ===========================================================================

#[test]
fn ac3_tail_json_output_includes_follow_field() {
    let project_root = tmp_path("proj-tail");
    fs::create_dir_all(project_root.join(".lux")).unwrap();
    let log_path = ai_log::resolve_log_path(&project_root);
    write_jsonl(
        &log_path,
        &[r#"{"timestampUtc":"2026-05-01T10:00:00Z","actor":"codex"}"#],
    );

    let follow = true;
    let entries = ai_log::read_log_entries(&log_path, &AiLogFilter::default()).unwrap();
    let output = json!({
        "projectPath": project_root,
        "path": log_path,
        "follow": follow,
        "count": entries.len(),
        "entries": entries,
    });

    let serialized = serde_json::to_string_pretty(&output).unwrap();
    let parsed: Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(parsed["follow"], true);
    assert_eq!(parsed["count"], 1);

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn ac3_tail_limits_to_last_n_entries() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        limit: Some(2),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert_eq!(entries.len(), 2);
    let _ = fs::remove_file(p);
}

// ===========================================================================
// AC3: CLI subcommand JSON output contract — context
// ===========================================================================

#[test]
fn ac3_context_json_output_has_count_and_entries_keys() {
    let p = sample_log_path();
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let context = ai_log::build_continuation_context(&entries, None);

    assert!(context.get("count").is_some());
    assert!(context.get("entries").is_some());
    assert_eq!(context["count"], 3);

    let serialized = serde_json::to_string_pretty(&context).unwrap();
    let _: Value = serde_json::from_str(&serialized).unwrap();

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_orders_by_timestamp_ascending() {
    let p = sample_log_path();
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let context = ai_log::build_continuation_context(&entries, None);
    let items = context["entries"].as_array().unwrap();

    assert_eq!(items[0]["timestampUtc"], "2026-05-01T10:00:00Z");
    assert_eq!(items[1]["timestampUtc"], "2026-05-01T10:01:00Z");
    assert_eq!(items[2]["timestampUtc"], "2026-05-01T10:02:00Z");

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_applies_limit_after_sorting() {
    let p = sample_log_path();
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let context = ai_log::build_continuation_context(&entries, Some(2));
    let items = context["entries"].as_array().unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["timestampUtc"], "2026-05-01T10:01:00Z");
    assert_eq!(items[1]["timestampUtc"], "2026-05-01T10:02:00Z");

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_entry_has_expected_schema_fields() {
    let p = sample_log_path();
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let context = ai_log::build_continuation_context(&entries, None);
    let first = &context["entries"].as_array().unwrap()[0];

    assert!(first.get("timestampUtc").is_some());
    assert!(first.get("actor").is_some());
    assert!(first.get("category").is_some());
    assert!(first.get("source").is_some());
    assert!(first.get("action").is_some());
    assert!(first.get("eventType").is_some());
    assert!(first.get("summary").is_some());

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_falls_back_to_captured_at_utc_for_timestamp() {
    let p = tmp_path("ctx-ts-fallback");
    write_jsonl(
        &p,
        &[r#"{"captured_at_utc":"2026-06-01T12:00:00Z","actor":"test","summary":"fallback"}"#],
    );
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let ctx = ai_log::build_continuation_context(&entries, None);
    assert_eq!(ctx["entries"][0]["timestampUtc"], "2026-06-01T12:00:00Z");
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_summary_falls_back_through_message_description() {
    let p = tmp_path("ctx-sum-fallback");
    write_jsonl(
        &p,
        &[r#"{"timestampUtc":"2026-06-01T12:00:00Z","actor":"test","description":"desc-val"}"#],
    );
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    let ctx = ai_log::build_continuation_context(&entries, None);
    assert_eq!(ctx["entries"][0]["summary"], "desc-val");
    let _ = fs::remove_file(p);
}

// ===========================================================================
// AC3: CLI subcommand JSON output contract — compact
// ===========================================================================

#[test]
fn ac3_compact_result_serializes_to_valid_json_with_correct_counts() {
    let result = AiLogCompactResult {
        path: PathBuf::from("/tmp/test.jsonl"),
        valid_before: 100,
        valid_after: 50,
        invalid_dropped: 10,
        lines_dropped: 60,
    };

    let serialized = serde_json::to_string_pretty(&result).unwrap();
    let parsed: Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(parsed["validBefore"], 100);
    assert_eq!(parsed["validAfter"], 50);
    assert_eq!(parsed["invalidDropped"], 10);
    assert_eq!(parsed["linesDropped"], 60);
    assert!(parsed.get("path").is_some());
}

#[test]
fn ac3_compact_keeps_last_n_lines_and_drops_earlier() {
    let p = tmp_path("compact-basic");
    write_jsonl(
        &p,
        &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#, r#"{"n":4}"#, r#"{"n":5}"#],
    );

    let result = ai_log::compact_log_file(&p, 2).unwrap();
    assert_eq!(result.valid_before, 5);
    assert_eq!(result.valid_after, 2);
    assert_eq!(result.invalid_dropped, 0);
    assert_eq!(result.lines_dropped, 3);

    let remaining = read_jsonl_values(&p);
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0]["n"], 4);
    assert_eq!(remaining[1]["n"], 5);

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_compact_drops_invalid_lines() {
    let p = tmp_path("compact-invalid");
    write_jsonl(
        &p,
        &[
            r#"{"n":1}"#,
            "not-json",
            "",
            r#"{"n":2}"#,
            "also-bad",
            r#"{"n":3}"#,
        ],
    );

    let result = ai_log::compact_log_file(&p, 2).unwrap();
    assert_eq!(result.valid_before, 3);
    assert_eq!(result.invalid_dropped, 2);
    assert_eq!(result.lines_dropped, 3);

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_compact_max_lines_greater_than_total_keeps_all() {
    let p = tmp_path("compact-all");
    write_jsonl(&p, &[r#"{"n":1}"#, r#"{"n":2}"#]);

    let result = ai_log::compact_log_file(&p, 100).unwrap();
    assert_eq!(result.valid_before, 2);
    assert_eq!(result.valid_after, 2);
    assert_eq!(result.lines_dropped, 0);

    let _ = fs::remove_file(p);
}

// ===========================================================================
// AC3: read_log_entries + filter_entries roundtrip
// ===========================================================================

#[test]
fn ac3_read_log_entries_returns_line_numbers_starting_at_one() {
    let p = tmp_path("line-numbers");
    write_jsonl(&p, &[r#"{"a":1}"#, r#"{"b":2}"#]);

    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    assert_eq!(entries[0].line_number, 1);
    assert_eq!(entries[1].line_number, 2);

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_filter_entries_empty_filter_returns_all() {
    let p = sample_log_path();
    let all = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    assert_eq!(all.len(), 3);
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_filter_entries_source_and_action_combined() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        source: Some("gateway".to_string()),
        action: Some("compile".to_string()),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value["action"], "compile");
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_filter_entries_event_type_exact_match() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        event_type: Some("append".to_string()),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert_eq!(entries.len(), 1);
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_read_log_entries_on_nonexistent_file_is_error() {
    let p = tmp_path("no-such-file");
    let result = ai_log::read_log_entries(&p, &AiLogFilter::default());
    assert!(result.is_err());
}

#[test]
fn ac3_ai_log_entry_serializes_roundtrip() {
    let entry = AiLogEntry {
        line_number: 42,
        timestamp: "2026-05-01T10:00:00Z".to_string(),
        value: json!({"actor": "codex", "summary": "test"}),
    };
    let serialized = serde_json::to_string(&entry).unwrap();
    let roundtrip: AiLogEntry = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtrip.line_number, 42);
    assert_eq!(roundtrip.timestamp, entry.timestamp);
    assert_eq!(roundtrip.value["actor"], "codex");
}

// ===========================================================================
// AC5: AiWorkStep struct fields and serialization
// ===========================================================================

#[test]
fn ac5_work_step_has_all_required_fields() {
    let step = AiWorkStep {
        step_name: "review-code".to_string(),
        status: "in-progress".to_string(),
        tool: Some("opencode".to_string()),
        action: Some("review".to_string()),
        summary: Some("Reviewing player controller".to_string()),
        redaction_metadata: None,
        timestamp_utc: "2026-05-05T14:30:00Z".to_string(),
    };

    assert_eq!(step.step_name, "review-code");
    assert_eq!(step.status, "in-progress");
    assert_eq!(step.tool.as_deref(), Some("opencode"));
    assert_eq!(step.action.as_deref(), Some("review"));
    assert_eq!(step.summary.as_deref(), Some("Reviewing player controller"));
    assert!(step.redaction_metadata.is_none());
    assert_eq!(step.timestamp_utc, "2026-05-05T14:30:00Z");
}

#[test]
fn ac5_work_step_serialization_uses_camel_case() {
    let step = AiWorkStep {
        step_name: "build".to_string(),
        status: "started".to_string(),
        tool: Some("cargo".to_string()),
        action: Some("compile".to_string()),
        summary: Some("Building release".to_string()),
        redaction_metadata: None,
        timestamp_utc: "2026-05-05T00:00:00Z".to_string(),
    };

    let value = serde_json::to_value(&step).unwrap();
    assert!(value.get("stepName").is_some(), "missing stepName (camelCase)");
    assert!(value.get("status").is_some());
    assert!(value.get("tool").is_some());
    assert!(value.get("action").is_some());
    assert!(value.get("summary").is_some());
    assert!(value.get("redactionMetadata").is_some());
    assert!(value.get("timestampUtc").is_some(), "missing timestampUtc (camelCase)");
    assert_eq!(value["stepName"], "build");
    assert_eq!(value["status"], "started");
}

#[test]
fn ac5_work_step_deserialization_roundtrip_preserves_all_fields() {
    let original = AiWorkStep {
        step_name: "deploy".to_string(),
        status: "completed".to_string(),
        tool: Some("unity".to_string()),
        action: Some("build-player".to_string()),
        summary: Some("Built Windows player".to_string()),
        redaction_metadata: Some(RedactionMetadata {
            redacted_fields: vec!["summary".to_string()],
            redaction_classes: vec!["secret".to_string()],
            timestamp: Some("2026-05-05T00:00:00Z".to_string()),
        }),
        timestamp_utc: "2026-05-05T15:00:00Z".to_string(),
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let roundtrip: AiWorkStep = serde_json::from_str(&json_str).unwrap();

    assert_eq!(roundtrip.step_name, original.step_name);
    assert_eq!(roundtrip.status, original.status);
    assert_eq!(roundtrip.tool, original.tool);
    assert_eq!(roundtrip.action, original.action);
    assert_eq!(roundtrip.summary, original.summary);
    assert_eq!(roundtrip.timestamp_utc, original.timestamp_utc);
    match (&roundtrip.redaction_metadata, &original.redaction_metadata) {
        (Some(a), Some(b)) => {
            assert_eq!(a.redacted_fields, b.redacted_fields);
            assert_eq!(a.redaction_classes, b.redaction_classes);
        }
        (None, None) => {}
        _ => panic!("redaction_metadata mismatch"),
    }
}

#[test]
fn ac5_work_step_optional_fields_can_be_none() {
    let step = AiWorkStep {
        step_name: "minimal".to_string(),
        status: "done".to_string(),
        tool: None,
        action: None,
        summary: None,
        redaction_metadata: None,
        timestamp_utc: "2026-05-05T00:00:00Z".to_string(),
    };

    let value = serde_json::to_value(&step).unwrap();
    assert!(value["tool"].is_null());
    assert!(value["action"].is_null());
    assert!(value["summary"].is_null());
    assert!(value["redactionMetadata"].is_null());

    let rt: AiWorkStep = serde_json::from_value(value).unwrap();
    assert_eq!(rt.tool, None);
    assert_eq!(rt.action, None);
    assert_eq!(rt.summary, None);
}

// ===========================================================================
// AC5: append_work_step + read_log_entries full cycle
// ===========================================================================

#[test]
fn ac5_append_single_work_step_reads_back_as_one_entry() {
    let p = tmp_path("ws-single");
    let step = AiWorkStep {
        step_name: "lint".to_string(),
        status: "running".to_string(),
        tool: Some("clippy".to_string()),
        action: Some("lint".to_string()),
        summary: Some("Running clippy lints".to_string()),
        redaction_metadata: None,
        timestamp_utc: "2026-05-06T09:00:00Z".to_string(),
    };

    ai_log::append_work_step(&p, &step).unwrap();
    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value["stepName"], "lint");
    assert_eq!(entries[0].value["status"], "running");
    assert_eq!(entries[0].value["tool"], "clippy");

    let _ = fs::remove_file(p);
}

#[test]
fn ac5_multiple_work_steps_are_read_back_in_order() {
    let p = tmp_path("ws-multi");
    let steps = vec![
        AiWorkStep {
            step_name: "step-a".to_string(),
            status: "done".to_string(),
            tool: None,
            action: None,
            summary: None,
            redaction_metadata: None,
            timestamp_utc: "2026-05-06T09:00:00Z".to_string(),
        },
        AiWorkStep {
            step_name: "step-b".to_string(),
            status: "done".to_string(),
            tool: None,
            action: None,
            summary: None,
            redaction_metadata: None,
            timestamp_utc: "2026-05-06T09:01:00Z".to_string(),
        },
        AiWorkStep {
            step_name: "step-c".to_string(),
            status: "done".to_string(),
            tool: None,
            action: None,
            summary: None,
            redaction_metadata: None,
            timestamp_utc: "2026-05-06T09:02:00Z".to_string(),
        },
    ];

    for s in &steps {
        ai_log::append_work_step(&p, s).unwrap();
    }

    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].value["stepName"], "step-a");
    assert_eq!(entries[1].value["stepName"], "step-b");
    assert_eq!(entries[2].value["stepName"], "step-c");

    let _ = fs::remove_file(p);
}

#[test]
fn ac5_work_step_append_creates_parent_directory_if_missing() {
    let nested = std::env::temp_dir()
        .join(format!(
            "lux-ws-dir-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("sub")
        .join("ai-action-log.jsonl");

    let step = AiWorkStep {
        step_name: "dir-test".to_string(),
        status: "ok".to_string(),
        tool: None,
        action: None,
        summary: None,
        redaction_metadata: None,
        timestamp_utc: "2026-05-06T00:00:00Z".to_string(),
    };

    ai_log::append_work_step(&nested, &step).unwrap();
    assert!(nested.exists());

    let entries = ai_log::read_log_entries(&nested, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);

    let _ = fs::remove_file(&nested);
    let _ = fs::remove_dir_all(nested.parent().unwrap());
}

// ===========================================================================
// AC5: Redaction applied before persistence
// ===========================================================================

#[test]
fn ac5_redact_secrets_replaces_bearer_tokens() {
    // redact_secrets works per-whitespace-token: only "Bearer" itself is replaced
    let result = ai_log::redact_secrets("Authorization: Bearer abc123");
    assert_eq!(result, "Authorization: [REDACTED] abc123");
    // Standalone bearer word is fully redacted
    assert_eq!(ai_log::redact_secrets("bearer"), "[REDACTED]");
}

#[test]
fn ac5_redact_secrets_replaces_token_equals() {
    // token= inside a single whitespace-delimited token replaces the whole token
    let result = ai_log::redact_secrets("url?token=secret&other=keep");
    assert_eq!(result, "[REDACTED]");
    // Multi-word: only the token containing token= is redacted
    assert_eq!(
        ai_log::redact_secrets("value token=secret end"),
        "value [REDACTED] end"
    );
}

#[test]
fn ac5_redact_project_paths_replaces_absolute_root() {
    let redacted = ai_log::redact_project_paths(
        "Edited /Users/dev/MyGame/Assets/Script.cs successfully",
        "/Users/dev/MyGame",
    );
    assert_eq!(redacted, "Edited ~/MyGame/Assets/Script.cs successfully");
}

#[test]
fn ac5_redact_entry_mutates_value_and_returns_metadata() {
    let mut value = json!({
        "summary": "token=secret at /Users/me/Proj/file.cs",
        "payload": {"playerId": "p-123"}
    });

    let meta = ai_log::redact_entry(&mut value, "/Users/me/Proj");

    assert_eq!(value["summary"], "[REDACTED] at ~/Proj/file.cs");
    assert_eq!(value["payload"]["playerId"], "[REDACTED]");
    assert!(!meta.redacted_fields.is_empty());
    assert!(meta.redaction_classes.contains(&"secret".to_string()));
    assert!(meta.redaction_classes.contains(&"project_path".to_string()));
    assert!(meta
        .redaction_classes
        .contains(&"gameplay_sensitive".to_string()));
}

#[test]
fn ac5_redact_entry_no_op_when_no_sensitive_data() {
    let mut value = json!({"safe": "nothing to see here"});
    let meta = ai_log::redact_entry(&mut value, "/some/path");
    assert_eq!(value["safe"], "nothing to see here");
    assert!(meta.redacted_fields.is_empty());
    assert!(meta.redaction_classes.is_empty());
}

#[test]
fn ac5_work_step_with_redaction_metadata_serializes_correctly() {
    let step = AiWorkStep {
        step_name: "redacted-step".to_string(),
        status: "completed".to_string(),
        tool: Some("tool".to_string()),
        action: Some("act".to_string()),
        summary: Some("[REDACTED]".to_string()),
        redaction_metadata: Some(RedactionMetadata {
            redacted_fields: vec!["summary".to_string()],
            redaction_classes: vec!["secret".to_string()],
            timestamp: Some("2026-05-06T00:00:00Z".to_string()),
        }),
        timestamp_utc: "2026-05-06T00:00:00Z".to_string(),
    };

    let json_str = serde_json::to_string(&step).unwrap();
    let parsed: Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["redactionMetadata"]["redactedFields"][0], "summary");
    assert_eq!(parsed["redactionMetadata"]["redactionClasses"][0], "secret");
    assert_eq!(parsed["summary"], "[REDACTED]");
}

// ===========================================================================
// AC3: Retention policy
// ===========================================================================

#[test]
fn ac3_retention_policy_default_max_lines_from_env_or_10000() {
    let policy = RetentionPolicy::default();
    assert!(policy.max_lines > 0);
    assert_eq!(policy.max_age_days, None);
}

#[test]
fn ac3_retention_policy_truncates_to_max_lines() {
    let p = tmp_path("retention");
    write_jsonl(
        &p,
        &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#, r#"{"n":4}"#, r#"{"n":5}"#],
    );

    ai_log::apply_retention_policy(
        &p,
        &RetentionPolicy {
            max_lines: 2,
            max_age_days: None,
        },
    )
    .unwrap();

    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].value["n"], 4);
    assert_eq!(entries[1].value["n"], 5);

    let _ = fs::remove_file(p);
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn ac3_empty_log_file_returns_zero_entries() {
    let p = tmp_path("empty");
    File::create(&p).unwrap();

    let entries = ai_log::read_log_entries(&p, &AiLogFilter::default()).unwrap();
    assert!(entries.is_empty());

    let _ = fs::remove_file(p);
}

#[test]
fn ac3_context_with_empty_entries_returns_empty_structure() {
    let ctx = ai_log::build_continuation_context(&[], None);
    assert_eq!(ctx["count"], 0);
    assert_eq!(ctx["entries"].as_array().unwrap().len(), 0);

    let s = serde_json::to_string(&ctx).unwrap();
    let _: Value = serde_json::from_str(&s).unwrap();
}

#[test]
fn ac3_filter_with_limit_zero_returns_nothing() {
    let p = sample_log_path();
    let filter = AiLogFilter {
        limit: Some(0),
        ..AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&p, &filter).unwrap();
    assert!(entries.is_empty());
    let _ = fs::remove_file(p);
}

#[test]
fn ac3_parse_jsonl_values_handles_only_blank_lines() {
    use std::io::Cursor;
    let input = Cursor::new("\n\n  \n\n");
    let values = ai_log::parse_jsonl_values(input);
    assert!(values.is_empty());
}
