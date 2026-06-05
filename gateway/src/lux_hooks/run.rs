use super::gate;
use super::rules::load_project_governance;
use super::{
    resolve_project_path, write_json_atomic, HookRunReport, HooksRunArgs, OmxUltraworkStatus,
};
use crate::lux_io::append_jsonl;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn run_hook_bridge(args: &HooksRunArgs) -> Result<HookRunReport> {
    if args.event.trim().is_empty() {
        bail!("--event must not be empty");
    }
    let project_path = resolve_project_path(args.project_path.as_ref())?;
    let mut stdin_body = String::new();
    io::stdin()
        .read_to_string(&mut stdin_body)
        .context("failed to read hook stdin")?;
    reject_symlinked_lux_root(&project_path)?;
    let governance = load_project_governance(&project_path)?;
    let event_id = format!("lux-hook-{}", Uuid::new_v4());
    let timestamp_utc = Utc::now().to_rfc3339();
    let parsed_stdin_result = serde_json::from_str::<Value>(&stdin_body);
    let stdin_json_error = parsed_stdin_result
        .as_ref()
        .err()
        .map(std::string::ToString::to_string);
    let parsed_stdin = match &parsed_stdin_result {
        Ok(value) => Some(value),
        Err(_) => None,
    };
    let prompt_excerpt = prompt_excerpt(parsed_stdin, &stdin_body);
    let ulw_detected = contains_ulw_signal(&stdin_body)
        || prompt_excerpt
            .as_ref()
            .is_some_and(|excerpt| contains_ulw_signal(excerpt));
    let omx_ultrawork = inspect_omx_ultrawork(&project_path);
    let gate_result = gate::evaluate_gate(&args.event, &project_path, &governance, parsed_stdin)?;
    let source = gate::hook_source(&args.event);
    let hook_dir = project_path.join(".lux").join("hooks");
    fs::create_dir_all(&hook_dir)
        .with_context(|| format!("failed to create {}", hook_dir.display()))?;
    let event_log_path = hook_dir.join("events.jsonl");
    let record = json!({
        "schema_version": 1,
        "event_id": event_id,
        "timestamp_utc": timestamp_utc,
        "event": args.event,
        "source": source,
        "ulw_detected": ulw_detected,
        "prompt_excerpt": prompt_excerpt,
        "stdin_json_valid": parsed_stdin_result.is_ok(),
        "stdin_json_error": stdin_json_error,
        "omx_ultrawork": omx_ultrawork,
        "project_settings": governance.settings,
        "loaded_rule_paths": governance.loaded_rule_paths,
        "gate_result": gate_result,
    });
    append_jsonl(&event_log_path, &record)?;
    if ulw_detected {
        let latest_path = hook_dir.join("ulw-check.json");
        write_json_atomic(&latest_path, &record)?;
    }
    let report = HookRunReport {
        event_id,
        event: args.event.clone(),
        project_path,
        event_log_path,
        source: source.to_string(),
        ulw_detected,
        omx_ultrawork,
        project_settings: governance.settings,
        loaded_rule_paths: governance.loaded_rule_paths,
        gate_result,
    };
    if matches!(report.gate_result.status.as_str(), "failed" | "unsupported") {
        bail!(HookRunFailure { report });
    }
    Ok(report)
}

fn reject_symlinked_lux_root(project_path: &Path) -> Result<()> {
    let lux_root = project_path.join(".lux");
    if fs::symlink_metadata(&lux_root)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        bail!(".lux runtime root must not be a symlink");
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct HookRunFailure {
    pub(super) report: HookRunReport,
}

impl std::fmt::Display for HookRunFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.report.gate_result.findings.is_empty() {
            return write!(formatter, "hook gate {}", self.report.gate_result.status);
        }
        let markers = self
            .report
            .gate_result
            .findings
            .iter()
            .map(|finding| finding.marker.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            formatter,
            "hook gate {}: {markers}",
            self.report.gate_result.status
        )
    }
}

impl std::error::Error for HookRunFailure {}

fn prompt_excerpt(parsed: Option<&Value>, raw: &str) -> Option<String> {
    let from_json = parsed.and_then(first_prompt_like_string);
    let text = from_json.unwrap_or(raw);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(240).collect())
    }
}

fn first_prompt_like_string(value: &Value) -> Option<&str> {
    match value {
        Value::Object(object) => {
            for key in ["prompt", "user_prompt", "message", "text", "input"] {
                if let Some(text) = object.get(key).and_then(Value::as_str) {
                    return Some(text);
                }
            }
            for child in object.values() {
                if let Some(text) = first_prompt_like_string(child) {
                    return Some(text);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(first_prompt_like_string),
        Value::String(text) => Some(text),
        _ => None,
    }
}

pub(super) fn contains_ulw_signal(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| {
            let normalized = token.to_ascii_lowercase();
            normalized == "ulw" || normalized == "ultrawork"
        })
}

fn inspect_omx_ultrawork(project_path: &Path) -> OmxUltraworkStatus {
    let sessions_dir = project_path.join(".omx").join("state").join("sessions");
    let Ok(entries) = fs::read_dir(&sessions_dir) else {
        return OmxUltraworkStatus::NotFound;
    };
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let path = entry.path().join("ultrawork-state.json");
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if newest
            .as_ref()
            .is_none_or(|(_, newest_modified)| modified > *newest_modified)
        {
            newest = Some((path, modified));
        }
    }
    let Some((state_path, _)) = newest else {
        return OmxUltraworkStatus::NotFound;
    };
    let text = match fs::read_to_string(&state_path) {
        Ok(text) => text,
        Err(error) => {
            return OmxUltraworkStatus::Invalid {
                state_path,
                error: error.to_string(),
            }
        }
    };
    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(parsed) => parsed,
        Err(error) => {
            return OmxUltraworkStatus::Invalid {
                state_path,
                error: error.to_string(),
            }
        }
    };
    let object = parsed.as_object().cloned().unwrap_or_else(Map::new);
    let active = object
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reinforcement_count = object.get("reinforcement_count").and_then(Value::as_u64);
    if active {
        OmxUltraworkStatus::Active {
            state_path,
            reinforcement_count,
        }
    } else {
        OmxUltraworkStatus::Inactive {
            state_path,
            reinforcement_count,
        }
    }
}
