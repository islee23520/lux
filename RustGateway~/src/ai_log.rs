use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{cross_platform, protocol::RedactionMetadata};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const LOG_RELATIVE_PATH: [&str; 2] = [".lux", "ai-action-log.jsonl"];
const LEGACY_LOG_RELATIVE_PATH: [&str; 2] = ["UserSettings", "LuxAiActionLog.jsonl"];
const DEFAULT_RETENTION_MAX_LINES: usize = 10_000;
const SENSITIVE_REDACTION: &str = "[REDACTED]";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiLogFilter {
    pub limit: Option<usize>,
    pub actor: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub action: Option<String>,
    pub event_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AiLogEntry {
    pub line_number: usize,
    pub timestamp: String,
    pub value: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiLogCompactResult {
    pub path: PathBuf,
    pub valid_before: usize,
    pub valid_after: usize,
    pub invalid_dropped: usize,
    pub lines_dropped: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWorkStep {
    pub step_name: String,
    pub status: String,
    pub tool: Option<String>,
    pub action: Option<String>,
    pub summary: Option<String>,
    pub redaction_metadata: Option<RedactionMetadata>,
    pub timestamp_utc: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetentionPolicy {
    pub max_lines: usize,
    pub max_age_days: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_lines: std::env::var("LUX_AI_LOG_MAX_LINES")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|max_lines| *max_lines > 0)
                .unwrap_or(DEFAULT_RETENTION_MAX_LINES),
            max_age_days: None,
        }
    }
}

pub fn resolve_log_path(project_root: impl AsRef<Path>) -> PathBuf {
    cross_platform::join_forward(project_root.as_ref(), &LOG_RELATIVE_PATH)
}

pub fn ensure_log_path(project_root: impl AsRef<Path>) -> Result<PathBuf> {
    let project_root = project_root.as_ref();
    let log_path = resolve_log_path(project_root);
    migrate_legacy_log_if_needed(project_root, &log_path)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create AI log directory {}", parent.display()))?;
    }
    Ok(log_path)
}

fn migrate_legacy_log_if_needed(project_root: &Path, log_path: &Path) -> Result<()> {
    if log_path.exists() {
        return Ok(());
    }

    let legacy_path = cross_platform::join_forward(project_root, &LEGACY_LOG_RELATIVE_PATH);
    if !legacy_path.exists() {
        return Ok(());
    }

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create AI log directory {}", parent.display()))?;
    }

    fs::rename(&legacy_path, log_path).with_context(|| {
        format!(
            "failed to migrate AI log from {} to {}",
            legacy_path.display(),
            log_path.display()
        )
    })
}

pub fn parse_jsonl_values(reader: impl BufRead) -> Vec<Value> {
    reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| parse_jsonl_line(&line))
        .collect()
}

pub fn read_log_entries(path: impl AsRef<Path>, filter: &AiLogFilter) -> Result<Vec<AiLogEntry>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open AI log {}", path.as_ref().display()))?;
    Ok(filter_entries(parse_entries(BufReader::new(file)), filter))
}

pub fn append_work_step(log_path: &Path, step: &AiWorkStep) -> Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create AI log directory {}", parent.display()))?;
    }

    let serialized = serde_json::to_string(step).context("failed to serialize AI work step")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to open AI log {}", log_path.display()))?;
    writeln!(file, "{serialized}")
        .with_context(|| format!("failed to append AI work step to {}", log_path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush AI log {}", log_path.display()))?;
    Ok(())
}

pub fn filter_entries(entries: Vec<AiLogEntry>, filter: &AiLogFilter) -> Vec<AiLogEntry> {
    let mut filtered: Vec<_> = entries
        .into_iter()
        .filter(|entry| matches_filter(&entry.value, filter))
        .collect();

    if let Some(limit) = filter.limit {
        if filtered.len() > limit {
            filtered = filtered.split_off(filtered.len() - limit);
        }
    }

    filtered
}

pub fn build_continuation_context(entries: &[AiLogEntry], limit: Option<usize>) -> Value {
    let mut ordered = entries.to_vec();
    ordered.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then(left.line_number.cmp(&right.line_number))
    });

    if let Some(limit) = limit {
        if ordered.len() > limit {
            ordered = ordered.split_off(ordered.len() - limit);
        }
    }

    let items: Vec<Value> = ordered
        .iter()
        .map(|entry| {
            json!({
                "timestampUtc": entry.timestamp,
                "actor": string_field(&entry.value, "actor"),
                "category": string_field(&entry.value, "category"),
                "source": string_field(&entry.value, "source"),
                "action": string_field(&entry.value, "action"),
                "eventType": event_type(&entry.value),
                "summary": compact_summary(&entry.value),
            })
        })
        .collect();

    json!({
        "count": items.len(),
        "entries": items,
    })
}

pub fn compact_log_file(path: impl AsRef<Path>, max_lines: usize) -> Result<AiLogCompactResult> {
    let path = path.as_ref();
    let file =
        File::open(path).with_context(|| format!("failed to open AI log {}", path.display()))?;
    let mut valid_lines = Vec::new();
    let mut invalid_dropped = 0usize;

    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("failed to read AI log {}", path.display()))?;
        if parse_jsonl_line(&line).is_some() {
            valid_lines.push(line.trim().to_string());
        } else if !line.trim().is_empty() {
            invalid_dropped += 1;
        }
    }

    let valid_before = valid_lines.len();
    let keep_from = valid_before.saturating_sub(max_lines);
    let kept_lines = &valid_lines[keep_from..];

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create AI log directory {}", parent.display()))?;
    }

    let temp_path = path.with_extension("jsonl.tmp");
    let mut output = File::create(&temp_path)
        .with_context(|| format!("failed to write temporary AI log {}", temp_path.display()))?;
    for line in kept_lines {
        writeln!(output, "{line}")
            .with_context(|| format!("failed to write temporary AI log {}", temp_path.display()))?;
    }
    output
        .sync_all()
        .with_context(|| format!("failed to sync temporary AI log {}", temp_path.display()))?;
    drop(output);
    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to atomically replace AI log {} with {}",
            path.display(),
            temp_path.display()
        )
    })?;

    Ok(AiLogCompactResult {
        path: path.to_path_buf(),
        valid_before,
        valid_after: kept_lines.len(),
        invalid_dropped,
        lines_dropped: keep_from + invalid_dropped,
    })
}

pub fn apply_retention_policy(log_path: &Path, policy: &RetentionPolicy) -> Result<()> {
    let file = File::open(log_path)
        .with_context(|| format!("failed to open AI log {}", log_path.display()))?;
    let max_age_days = policy.max_age_days;
    let current_day = current_unix_day();
    let mut retained_lines = Vec::new();

    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("failed to read AI log {}", log_path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        if max_age_days
            .and_then(|max_age_days| {
                line_age_days(&line, current_day).map(|age| age <= max_age_days as i64)
            })
            .unwrap_or(true)
        {
            retained_lines.push(line.trim().to_string());
        }
    }

    let keep_from = retained_lines.len().saturating_sub(policy.max_lines);
    let retained_lines = &retained_lines[keep_from..];
    write_lines_atomically(log_path, retained_lines)
}

pub fn redact_secrets(text: &str) -> String {
    text.split_whitespace()
        .map(|part| {
            if part.eq_ignore_ascii_case("bearer") || part.to_ascii_lowercase().contains("token=") {
                "[REDACTED]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn redact_project_paths(text: &str, project_root: &str) -> String {
    let trimmed_root = project_root.trim_end_matches(['/', '\\']);
    if trimmed_root.is_empty() || !text.contains(trimmed_root) {
        return text.to_string();
    }

    let project_name = Path::new(trimmed_root)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("project");
    text.replace(trimmed_root, &format!("~/{project_name}"))
}

pub fn redact_gameplay_sensitive(value: &mut Value) {
    redact_gameplay_sensitive_with_metadata(value, "", None);
}

pub fn redact_entry(value: &mut Value, project_root: &str) -> RedactionMetadata {
    let mut metadata = RedactionMetadata::default();
    redact_value(value, project_root, "", &mut metadata);
    metadata
}

fn parse_entries(reader: impl BufRead) -> Vec<AiLogEntry> {
    reader
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let value = parse_jsonl_line(&line.ok()?)?;
            Some(AiLogEntry {
                line_number: index + 1,
                timestamp: timestamp(&value).unwrap_or_default(),
                value,
            })
        })
        .collect()
}

fn write_lines_atomically(path: &Path, lines: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create AI log directory {}", parent.display()))?;
    }

    let temp_path = path.with_extension("jsonl.tmp");
    let mut output = File::create(&temp_path)
        .with_context(|| format!("failed to write temporary AI log {}", temp_path.display()))?;
    for line in lines {
        writeln!(output, "{line}")
            .with_context(|| format!("failed to write temporary AI log {}", temp_path.display()))?;
    }
    output
        .sync_all()
        .with_context(|| format!("failed to sync temporary AI log {}", temp_path.display()))?;
    drop(output);
    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to atomically replace AI log {} with {}",
            path.display(),
            temp_path.display()
        )
    })
}

fn redact_value(
    value: &mut Value,
    project_root: &str,
    path: &str,
    metadata: &mut RedactionMetadata,
) {
    match value {
        Value::String(text) => {
            let original = text.clone();
            let without_secrets = redact_secrets(&original);
            if without_secrets != original {
                add_redaction(metadata, path, "secret");
            }
            let without_paths = redact_project_paths(&without_secrets, project_root);
            if without_paths != without_secrets {
                add_redaction(metadata, path, "project_path");
            }
            *text = without_paths;
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                let child_path = join_path(path, &index.to_string());
                redact_value(item, project_root, &child_path, metadata);
            }
        }
        Value::Object(map) => redact_map(map, project_root, path, metadata),
        _ => {}
    }
}

fn redact_map(
    map: &mut Map<String, Value>,
    project_root: &str,
    path: &str,
    metadata: &mut RedactionMetadata,
) {
    for (key, child) in map.iter_mut() {
        let child_path = join_path(path, key);
        if is_gameplay_sensitive_key(key) {
            if !child.is_null() {
                *child = Value::String(SENSITIVE_REDACTION.to_string());
                add_redaction(metadata, &child_path, "gameplay_sensitive");
            }
        } else {
            redact_value(child, project_root, &child_path, metadata);
        }
    }
}

fn redact_gameplay_sensitive_with_metadata(
    value: &mut Value,
    path: &str,
    metadata: Option<&mut RedactionMetadata>,
) {
    match value {
        Value::Object(map) => {
            let mut metadata = metadata;
            for (key, child) in map.iter_mut() {
                let child_path = join_path(path, key);
                if is_gameplay_sensitive_key(key) {
                    if !child.is_null() {
                        *child = Value::String(SENSITIVE_REDACTION.to_string());
                        if let Some(metadata) = metadata.as_deref_mut() {
                            add_redaction(metadata, &child_path, "gameplay_sensitive");
                        }
                    }
                } else {
                    redact_gameplay_sensitive_with_metadata(
                        child,
                        &child_path,
                        metadata.as_deref_mut(),
                    );
                }
            }
        }
        Value::Array(items) => {
            let mut metadata = metadata;
            for (index, item) in items.iter_mut().enumerate() {
                let child_path = join_path(path, &index.to_string());
                redact_gameplay_sensitive_with_metadata(item, &child_path, metadata.as_deref_mut());
            }
        }
        _ => {}
    }
}

fn is_gameplay_sensitive_key(key: &str) -> bool {
    matches!(
        key,
        "playerId" | "player_id" | "email" | "ipAddress" | "ip_address"
    )
}

fn add_redaction(metadata: &mut RedactionMetadata, field: &str, class: &str) {
    let field = if field.is_empty() { "$" } else { field };
    if !metadata
        .redacted_fields
        .iter()
        .any(|existing| existing == field)
    {
        metadata.redacted_fields.push(field.to_string());
    }
    if !metadata
        .redaction_classes
        .iter()
        .any(|existing| existing == class)
    {
        metadata.redaction_classes.push(class.to_string());
    }
}

fn join_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}.{child}")
    }
}

fn current_unix_day() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_secs() / 86_400) as i64)
        .unwrap_or_default()
}

fn line_age_days(line: &str, current_day: i64) -> Option<i64> {
    let value = parse_jsonl_line(line)?;
    let timestamp = timestamp(&value)?;
    let event_day = parse_utc_date_day(&timestamp)?;
    Some(current_day.saturating_sub(event_day))
}

fn parse_utc_date_day(timestamp: &str) -> Option<i64> {
    let date = timestamp.get(0..10)?;
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era - 719_468) as i64
}

fn parse_jsonl_line(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    serde_json::from_str::<Value>(trimmed).ok()
}

fn matches_filter(value: &Value, filter: &AiLogFilter) -> bool {
    matches_optional(value, "actor", filter.actor.as_deref())
        && matches_optional(value, "category", filter.category.as_deref())
        && matches_optional(value, "source", filter.source.as_deref())
        && matches_optional(value, "action", filter.action.as_deref())
        && filter.event_type.as_deref().map_or(true, |expected| {
            event_type(value).as_deref() == Some(expected)
        })
}

fn matches_optional(value: &Value, field: &str, expected: Option<&str>) -> bool {
    expected.map_or(true, |expected| {
        string_field(value, field).as_deref() == Some(expected)
    })
}

fn timestamp(value: &Value) -> Option<String> {
    string_field(value, "timestampUtc").or_else(|| string_field(value, "captured_at_utc"))
}

fn event_type(value: &Value) -> Option<String> {
    string_field(value, "eventType").or_else(|| string_field(value, "event_type"))
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field)?.as_str().map(ToOwned::to_owned)
}

fn compact_summary(value: &Value) -> Option<String> {
    ["summary", "message", "description"]
        .iter()
        .find_map(|field| string_field(value, field))
        .map(|text| redact_secrets(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ai_log_resolves_project_local_path() {
        let path = resolve_log_path(Path::new("/project"));
        assert_eq!(path, PathBuf::from("/project/.lux/ai-action-log.jsonl"));
    }

    #[test]
    fn resolve_log_path_uses_forward_slashes_on_all_platforms() {
        let path = resolve_log_path(r#"C:\Users\dev\Project"#);
        assert_eq!(
            path.to_string_lossy(),
            "C:/Users/dev/Project/.lux/ai-action-log.jsonl"
        );
    }

    #[test]
    fn ai_log_migrates_legacy_log_to_lux_directory() {
        let project_root = temp_project_root("ai-log-migration");
        let legacy_dir = project_root.join("UserSettings");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy_path = legacy_dir.join("LuxAiActionLog.jsonl");
        fs::write(&legacy_path, "{\"actor\":\"legacy\"}\n").unwrap();

        let log_path = ensure_log_path(&project_root).unwrap();

        assert_eq!(log_path, project_root.join(".lux/ai-action-log.jsonl"));
        assert_eq!(
            fs::read_to_string(&log_path).unwrap(),
            "{\"actor\":\"legacy\"}\n"
        );
        assert!(!legacy_path.exists());
        fs::remove_dir_all(project_root).unwrap();
    }

    #[test]
    fn ai_log_jsonl_parsing_ignores_invalid_and_blank_lines() {
        let input =
            Cursor::new("\n{\"actor\":\"codex\"}\nnot-json\n  \n{\"actor\":\"opencode\"}\n");
        let values = parse_jsonl_values(input);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["actor"], "codex");
        assert_eq!(values[1]["actor"], "opencode");
    }

    #[test]
    fn ai_log_filter_applies_actor_category_event_type_and_tail_limit() {
        let entries = parse_entries(Cursor::new(
            "{\"timestampUtc\":\"2026-05-04T00:00:00Z\",\"actor\":\"codex\",\"category\":\"tool\",\"eventType\":\"start\"}\n\
             {\"timestampUtc\":\"2026-05-04T00:00:01Z\",\"actor\":\"codex\",\"category\":\"ai-action-log\",\"eventType\":\"append\"}\n\
             {\"timestampUtc\":\"2026-05-04T00:00:02Z\",\"actor\":\"opencode\",\"category\":\"ai-action-log\",\"eventType\":\"append\"}\n\
             {\"timestampUtc\":\"2026-05-04T00:00:03Z\",\"actor\":\"codex\",\"category\":\"ai-action-log\",\"eventType\":\"append\"}\n",
        ));

        let filter = AiLogFilter {
            limit: Some(1),
            actor: Some("codex".to_string()),
            category: Some("ai-action-log".to_string()),
            event_type: Some("append".to_string()),
            ..AiLogFilter::default()
        };

        let filtered = filter_entries(entries, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].timestamp, "2026-05-04T00:00:03Z");
    }

    #[test]
    fn ai_log_continuation_context_orders_by_timestamp() {
        let entries = parse_entries(Cursor::new(
            "{\"timestampUtc\":\"2026-05-04T00:00:02Z\",\"actor\":\"codex\",\"summary\":\"second\"}\n\
             {\"captured_at_utc\":\"2026-05-04T00:00:01Z\",\"actor\":\"opencode\",\"message\":\"first\"}\n",
        ));

        let context = build_continuation_context(&entries, None);
        let items = context["entries"].as_array().unwrap();
        assert_eq!(items[0]["timestampUtc"], "2026-05-04T00:00:01Z");
        assert_eq!(items[0]["summary"], "first");
        assert_eq!(items[1]["summary"], "second");
    }

    #[test]
    fn ai_log_compact_preserves_valid_jsonl_and_drops_excess_invalid_lines() {
        let path = std::env::temp_dir().join(format!(
            "lux-ai-log-test-{}.jsonl",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "{\"n\":1}\ninvalid\n\n{\"n\":2}\n{\"n\":3}\n{\"n\":4}\n",
        )
        .unwrap();

        let result = compact_log_file(&path, 2).unwrap();
        let compacted = fs::read_to_string(&path).unwrap();

        assert_eq!(result.valid_before, 4);
        assert_eq!(result.valid_after, 2);
        assert_eq!(result.invalid_dropped, 1);
        assert_eq!(result.lines_dropped, 3);
        assert_eq!(compacted, "{\"n\":3}\n{\"n\":4}\n");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn ai_log_redacts_bearer_and_token_values_in_summaries() {
        assert_eq!(
            redact_secrets("Authorization: Bearer abc token=secret keep"),
            "Authorization: [REDACTED] abc [REDACTED] keep"
        );
    }

    #[test]
    fn ai_work_step_serializes_with_required_fields() {
        let step = AiWorkStep {
            step_name: "compile".to_string(),
            status: "started".to_string(),
            tool: Some("opencode".to_string()),
            action: Some("compile".to_string()),
            summary: Some("Running cargo build".to_string()),
            redaction_metadata: None,
            timestamp_utc: "2026-05-05T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(step).unwrap();
        assert_eq!(value["stepName"], "compile");
        assert_eq!(value["status"], "started");
        assert_eq!(value["timestampUtc"], "2026-05-05T00:00:00Z");
    }

    #[test]
    fn ai_work_step_appends_to_jsonl_and_reads_back() {
        let path = std::env::temp_dir().join(format!(
            "lux-ai-work-step-test-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let step = AiWorkStep {
            step_name: "review".to_string(),
            status: "completed".to_string(),
            tool: Some("codex".to_string()),
            action: Some("review".to_string()),
            summary: Some("Reviewed event schema".to_string()),
            redaction_metadata: None,
            timestamp_utc: "2026-05-05T00:00:00Z".to_string(),
        };

        append_work_step(&path, &step).unwrap();
        let entries = read_log_entries(&path, &AiLogFilter::default()).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value["stepName"], "review");
        assert_eq!(entries[0].value["status"], "completed");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn redact_project_paths_replaces_absolute_paths() {
        let redacted = redact_project_paths(
            "/Users/me/Projects/MyGame/Assets/Scene.unity failed",
            "/Users/me/Projects/MyGame",
        );

        assert_eq!(redacted, "~/MyGame/Assets/Scene.unity failed");
    }

    #[test]
    fn redact_gameplay_sensitive_masks_player_ids() {
        let mut value = json!({
            "payload": {
                "playerId": "player-123",
                "email": "player@example.test",
                "safe": "ok"
            }
        });

        redact_gameplay_sensitive(&mut value);

        assert_eq!(value["payload"]["playerId"], SENSITIVE_REDACTION);
        assert_eq!(value["payload"]["email"], SENSITIVE_REDACTION);
        assert_eq!(value["payload"]["safe"], "ok");
    }

    #[test]
    fn redact_entry_produces_redaction_metadata() {
        let mut value = json!({
            "summary": "token=secret at /Users/me/Projects/MyGame/Assets/A.cs",
            "payload": {
                "ipAddress": "127.0.0.1"
            }
        });

        let metadata = redact_entry(&mut value, "/Users/me/Projects/MyGame");

        assert_eq!(value["summary"], "[REDACTED] at ~/MyGame/Assets/A.cs");
        assert_eq!(value["payload"]["ipAddress"], SENSITIVE_REDACTION);
        assert!(metadata.redacted_fields.contains(&"summary".to_string()));
        assert!(metadata
            .redacted_fields
            .contains(&"payload.ipAddress".to_string()));
        assert!(metadata.redaction_classes.contains(&"secret".to_string()));
        assert!(metadata
            .redaction_classes
            .contains(&"project_path".to_string()));
        assert!(metadata
            .redaction_classes
            .contains(&"gameplay_sensitive".to_string()));
    }

    #[test]
    fn retention_policy_truncates_log_file() {
        let path = std::env::temp_dir().join(format!(
            "lux-ai-retention-test-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, "{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n").unwrap();

        apply_retention_policy(
            &path,
            &RetentionPolicy {
                max_lines: 2,
                max_age_days: None,
            },
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"n\":2}\n{\"n\":3}\n");

        let _ = fs::remove_file(path);
    }

    fn temp_project_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lux-{prefix}-{nanos}"));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
