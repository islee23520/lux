use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::protocol::{RedactionMetadata, RetentionMetadata};

pub type SessionId = String;

const SESSION_DIR: [&str; 2] = [".lux", "sessions"];
const TOOL_SESSION_DIR: [&str; 3] = [".lux", "sessions", "tools"];
const CURRENT_SESSION_FILE: &str = "current-session";
const HEADER_RECORD_TYPE: &str = "session_header";
const EVENT_RECORD_TYPE: &str = "session_event";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCommandHistoryEntry {
    pub id: String,
    pub command: String,
    pub timestamp: String,
    pub output_preview: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSessionRecord {
    pub id: String,
    pub tool_type: String,
    pub status: String,
    pub created_at_utc: String,
    pub updated_at_utc: String,
    pub command_history: Vec<ToolCommandHistoryEntry>,
    pub last_output: Option<String>,
}

static ACTIVE_SESSIONS: OnceLock<Mutex<BTreeMap<String, PathBuf>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    pub id: String,
    pub created_at: String,
    pub project_path: PathBuf,
    pub unity_version: Option<String>,
    pub lux_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_metadata: Option<RetentionMetadata>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub timestamp_utc: String,
    pub event_type: String,
    pub category: String,
    pub source: String,
    pub summary: String,
    pub payload: Value,
    pub redaction_metadata: Option<RedactionMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_metadata: Option<RetentionMetadata>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFile {
    pub header: SessionHeader,
    pub events: Vec<SessionEvent>,
}

#[derive(Clone, Debug)]
pub struct ReplayOptions {
    pub speed: f64,
    pub stop_on_error: bool,
    pub filter_types: Vec<String>,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            speed: 1.0,
            stop_on_error: false,
            filter_types: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub total_events: usize,
    pub replayed_events: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineResult {
    pub session_id: String,
    pub events: Vec<SessionEvent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReport {
    pub session_id: String,
    pub total_events: usize,
    pub duration_ms: u64,
    pub error_count: usize,
    pub event_type_counts: BTreeMap<String, usize>,
    pub errors: Vec<String>,
}

pub fn start_session(project_root: &Path) -> Result<(SessionId, PathBuf)> {
    let project_root = project_root.to_path_buf();
    let session_id = format!("lux-session-{}-{}", unix_millis(), uuid::Uuid::new_v4());
    let session_path = session_path(project_root.as_path(), &session_id);
    if let Some(parent) = session_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create session directory {}", parent.display()))?;
    }

    let header = SessionHeader {
        id: session_id.clone(),
        created_at: current_timestamp_utc(),
        project_path: project_root.clone(),
        unity_version: read_unity_version(project_root.as_path()),
        lux_version: env!("CARGO_PKG_VERSION").to_string(),
        retention_metadata: Some(RetentionMetadata {
            max_age_days: None,
            max_lines: Some(10_000),
            policy: Some("session-default".to_string()),
            created_at: None,
            expires_at: None,
        }),
    };

    let record = json!({
        "recordType": HEADER_RECORD_TYPE,
        "header": header,
    });
    fs::write(
        &session_path,
        format!("{}\n", serde_json::to_string(&record)?),
    )
    .with_context(|| format!("failed to write session header {}", session_path.display()))?;
    fs::write(
        sessions_root(project_root.as_path()).join(CURRENT_SESSION_FILE),
        &session_id,
    )
    .context("failed to write current session marker")?;
    remember_session_path(&session_id, &session_path);

    Ok((session_id, session_path))
}

pub fn record_session_event(session_id: &str, event: SessionEvent) -> Result<()> {
    let path = resolve_session_path(session_id)?;
    append_event_line(&path, event)
}

pub fn record_session_event_in_project(
    project_root: &Path,
    session_id: &str,
    event: SessionEvent,
) -> Result<()> {
    let path = session_path(project_root, session_id);
    append_event_line(&path, event)
}

pub fn read_tool_sessions(project_root: &Path) -> Result<Vec<ToolSessionRecord>> {
    let root = tool_sessions_root(project_root);
    let read_dir = match fs::read_dir(&root) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to read tool sessions directory {}", root.display())
            })
        }
    };

    let mut sessions: Vec<ToolSessionRecord> = Vec::new();
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read tool session {}", path.display()))?;
        sessions.push(
            serde_json::from_str(&content)
                .with_context(|| format!("failed to parse tool session {}", path.display()))?,
        );
    }
    sessions.sort_by(|left, right| left.updated_at_utc.cmp(&right.updated_at_utc));
    Ok(sessions)
}

pub fn write_tool_session(project_root: &Path, record: &ToolSessionRecord) -> Result<PathBuf> {
    let path = tool_session_path(project_root, &record.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create tool sessions directory {}",
                parent.display()
            )
        })?;
    }
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serde_json::to_string_pretty(record)?)
        .with_context(|| format!("failed to write tool session {}", temp_path.display()))?;
    fs::rename(&temp_path, &path).with_context(|| {
        format!(
            "failed to move tool session {} to {}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(path)
}

pub fn delete_tool_session(project_root: &Path, session_id: &str) -> Result<bool> {
    let path = tool_session_path(project_root, session_id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("failed to delete tool session {}", path.display()))
        }
    }
}

pub fn stop_session(session_id: &str) -> Result<SessionFile> {
    let path = resolve_session_path(session_id)?;
    let session_file = read_session_file(&path)?;
    if let Some(parent) = path.parent() {
        let marker = parent.join(CURRENT_SESSION_FILE);
        if fs::read_to_string(&marker)
            .map(|id| id.trim() == session_id)
            .unwrap_or(false)
        {
            let _ = fs::remove_file(marker);
        }
    }
    active_sessions()
        .lock()
        .expect("active sessions lock")
        .remove(session_id);
    Ok(session_file)
}

pub fn stop_session_in_project(project_root: &Path, session_id: &str) -> Result<SessionFile> {
    let path = session_path(project_root, session_id);
    let session_file = read_session_file(&path)?;
    let marker = sessions_root(project_root).join(CURRENT_SESSION_FILE);
    if fs::read_to_string(&marker)
        .map(|id| id.trim() == session_id)
        .unwrap_or(false)
    {
        let _ = fs::remove_file(marker);
    }
    active_sessions()
        .lock()
        .expect("active sessions lock")
        .remove(session_id);
    Ok(session_file)
}

pub fn replay_session(session_id: &str, options: ReplayOptions) -> Result<ReplayResult> {
    let path = resolve_session_path(session_id)?;
    replay_session_path(&path, options)
}

pub fn replay_session_in_project(
    project_root: &Path,
    session_id: &str,
    options: ReplayOptions,
) -> Result<ReplayResult> {
    replay_session_path(&session_path(project_root, session_id), options)
}

pub fn timeline_session(
    project_root: &Path,
    session_id: Option<&str>,
    filter_type: Option<&str>,
    limit: Option<usize>,
) -> Result<TimelineResult> {
    let session_id = match session_id {
        Some(id) => id.to_string(),
        None => most_recent_session_id(project_root)?,
    };
    let session_file = read_session_file(&session_path(project_root, &session_id))?;
    let mut events: Vec<_> = session_file
        .events
        .into_iter()
        .filter(|event| filter_type.map_or(true, |expected| event.event_type == expected))
        .collect();
    events.sort_by(|left, right| left.timestamp_utc.cmp(&right.timestamp_utc));
    if let Some(limit) = limit {
        if events.len() > limit {
            events = events.split_off(events.len() - limit);
        }
    }
    Ok(TimelineResult { session_id, events })
}

pub fn report_session(project_root: &Path, session_id: &str) -> Result<SessionReport> {
    let session_file = read_session_file(&session_path(project_root, session_id))?;
    let mut event_type_counts = BTreeMap::new();
    let mut errors = Vec::new();
    for event in &session_file.events {
        *event_type_counts
            .entry(event.event_type.clone())
            .or_insert(0) += 1;
        if event_is_error(event) {
            errors.push(event.summary.clone());
        }
    }
    Ok(SessionReport {
        session_id: session_id.to_string(),
        total_events: session_file.events.len(),
        duration_ms: event_span_ms(&session_file.events),
        error_count: errors.len(),
        event_type_counts,
        errors,
    })
}

pub fn current_session_id(project_root: &Path) -> Result<String> {
    let marker = sessions_root(project_root).join(CURRENT_SESSION_FILE);
    let id = fs::read_to_string(&marker)
        .with_context(|| format!("failed to read current session marker {}", marker.display()))?;
    Ok(id.trim().to_string())
}

pub fn session_path(project_root: &Path, session_id: &str) -> PathBuf {
    sessions_root(project_root).join(format!("{session_id}.jsonl"))
}

fn replay_session_path(path: &Path, options: ReplayOptions) -> Result<ReplayResult> {
    if options.speed <= 0.0 || !options.speed.is_finite() {
        bail!("replay speed must be a positive finite number");
    }
    let session_file = read_session_file(path)?;
    let total_events = session_file.events.len();
    let mut replayed_events = 0usize;
    let mut errors = Vec::new();
    let mut previous_timestamp: Option<u64> = None;
    let start = Instant::now();

    for event in session_file.events.iter().filter(|event| {
        options.filter_types.is_empty()
            || options.filter_types.iter().any(|t| t == &event.event_type)
    }) {
        if let Some(timestamp) = timestamp_millis(&event.timestamp_utc) {
            if let Some(previous) = previous_timestamp {
                let delay = timestamp.saturating_sub(previous) as f64 / options.speed;
                if delay > 0.0 {
                    std::thread::sleep(Duration::from_millis(delay.round() as u64));
                }
            }
            previous_timestamp = Some(timestamp);
        }

        if let Err(error) = replay_event(event) {
            errors.push(error.to_string());
            if options.stop_on_error {
                break;
            }
        } else {
            replayed_events += 1;
        }
    }

    let timeline_duration = event_span_ms(&session_file.events);
    Ok(ReplayResult {
        total_events,
        replayed_events,
        errors,
        duration_ms: ((timeline_duration as f64) / options.speed).round() as u64
            + start.elapsed().as_millis().min(1) as u64,
    })
}

fn replay_event(event: &SessionEvent) -> Result<()> {
    if event.payload.get("replayError").and_then(Value::as_bool) == Some(true) {
        bail!("{}", event.summary);
    }
    match event.event_type.as_str() {
        "input" => {
            println!("replay input: {}", event.summary);
            Ok(())
        }
        "ai_action" | "tool_call" => {
            println!("replay info {}: {}", event.event_type, event.summary);
            Ok(())
        }
        "editor_hook" => {
            println!("replay skipped editor_hook: {}", event.summary);
            Ok(())
        }
        _ => {
            println!("replay event {}: {}", event.event_type, event.summary);
            Ok(())
        }
    }
}

fn append_event_line(path: &Path, event: SessionEvent) -> Result<()> {
    if !path.exists() {
        bail!("session file not found: {}", path.display());
    }
    let record = json!({
        "recordType": EVENT_RECORD_TYPE,
        "event": event,
    });
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open session file {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(&record)?)
        .with_context(|| format!("failed to append session event {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush session file {}", path.display()))?;
    Ok(())
}

fn read_session_file(path: &Path) -> Result<SessionFile> {
    let file =
        File::open(path).with_context(|| format!("failed to open session {}", path.display()))?;
    let mut header = None;
    let mut events = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("failed to read session {}", path.display()))?;
        let Some(value) = parse_jsonl_line(&line) else {
            continue;
        };
        match value.get("recordType").and_then(Value::as_str) {
            Some(HEADER_RECORD_TYPE) => {
                header = value
                    .get("header")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok());
            }
            Some(EVENT_RECORD_TYPE) => {
                if let Some(event) = value
                    .get("event")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok())
                {
                    events.push(event);
                }
            }
            _ => {
                if let Ok(event) = serde_json::from_value::<SessionEvent>(value) {
                    events.push(event);
                }
            }
        }
    }
    let header = header.with_context(|| format!("session header missing in {}", path.display()))?;
    Ok(SessionFile { header, events })
}

fn parse_jsonl_line(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn resolve_session_path(session_id: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(session_id);
    if candidate.is_file() {
        return Ok(candidate);
    }
    if let Some(path) = active_sessions()
        .lock()
        .expect("active sessions lock")
        .get(session_id)
    {
        return Ok(path.clone());
    }
    for var in ["LUX_SESSION_PROJECT_PATH", "LUX_PROJECT_PATH"] {
        if let Some(project_root) = std::env::var_os(var) {
            let path = session_path(Path::new(&project_root), session_id);
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    let path = session_path(&std::env::current_dir()?, session_id);
    if path.is_file() {
        return Ok(path);
    }
    bail!("session {session_id} not found; pass a session file path or run from the project root")
}

fn remember_session_path(session_id: &str, path: &Path) {
    active_sessions()
        .lock()
        .expect("active sessions lock")
        .insert(session_id.to_string(), path.to_path_buf());
}

fn active_sessions() -> &'static Mutex<BTreeMap<String, PathBuf>> {
    ACTIVE_SESSIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn sessions_root(project_root: &Path) -> PathBuf {
    SESSION_DIR
        .iter()
        .fold(project_root.to_path_buf(), |path, segment| {
            path.join(segment)
        })
}

fn tool_sessions_root(project_root: &Path) -> PathBuf {
    TOOL_SESSION_DIR
        .iter()
        .fold(project_root.to_path_buf(), |path, segment| {
            path.join(segment)
        })
}

fn tool_session_path(project_root: &Path, session_id: &str) -> PathBuf {
    tool_sessions_root(project_root).join(format!("{session_id}.json"))
}

fn most_recent_session_id(project_root: &Path) -> Result<String> {
    if let Ok(id) = current_session_id(project_root) {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let root = sessions_root(project_root);
    let mut newest: Option<(SystemTime, String)> = None;
    for entry in
        fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let modified = entry.metadata()?.modified().unwrap_or(UNIX_EPOCH);
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        if newest.as_ref().map_or(true, |(time, _)| modified > *time) {
            newest = Some((modified, id));
        }
    }
    newest.map(|(_, id)| id).context("no session files found")
}

fn event_is_error(event: &SessionEvent) -> bool {
    event.category.eq_ignore_ascii_case("error")
        || event.event_type.eq_ignore_ascii_case("error")
        || event.payload.get("ok").and_then(Value::as_bool) == Some(false)
        || event.payload.get("error").is_some()
        || event.payload.get("replayError").and_then(Value::as_bool) == Some(true)
}

fn event_span_ms(events: &[SessionEvent]) -> u64 {
    let mut timestamps = events
        .iter()
        .filter_map(|event| timestamp_millis(&event.timestamp_utc));
    let Some(first) = timestamps.next() else {
        return 0;
    };
    let mut min = first;
    let mut max = first;
    for timestamp in timestamps {
        min = min.min(timestamp);
        max = max.max(timestamp);
    }
    max.saturating_sub(min)
}

fn timestamp_millis(timestamp: &str) -> Option<u64> {
    let numeric = timestamp.trim_end_matches('Z');
    if let Ok(value) = numeric.parse::<u64>() {
        return Some(value);
    }
    let seconds = timestamp.get(17..19)?.parse::<u64>().ok()?;
    let minutes = timestamp.get(14..16)?.parse::<u64>().ok()?;
    let hours = timestamp.get(11..13)?.parse::<u64>().ok()?;
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000)
}

fn read_unity_version(project_root: &Path) -> Option<String> {
    let version =
        fs::read_to_string(project_root.join("ProjectSettings/ProjectVersion.txt")).ok()?;
    version
        .lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion: ").map(str::to_string))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn current_timestamp_utc() -> String {
    format!("{}Z", unix_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_start_creates_header_and_returns_id() {
        let project = temp_project("session-start");
        let (id, path) = start_session(&project).unwrap();
        assert!(id.starts_with("lux-session-"));
        assert!(path.is_file());
        let session = read_session_file(&path).unwrap();
        assert_eq!(session.header.id, id);
        assert_eq!(session.header.project_path, project);
    }

    #[test]
    fn session_record_appends_events_to_jsonl() {
        let project = temp_project("session-record");
        let (id, path) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content.lines().count(), 2);
        assert!(content.contains("first"));
    }

    #[test]
    fn session_stop_returns_metadata() {
        let project = temp_project("session-stop");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        let stopped = stop_session(&id).unwrap();
        assert_eq!(stopped.header.id, id);
        assert_eq!(stopped.events.len(), 1);
    }

    #[test]
    fn session_record_replay_plays_all_events_in_order() {
        let project = temp_project("session-replay-order");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        record_session_event(&id, sample_event("1001Z", "tool_call", "second")).unwrap();
        let result = replay_session(
            &id,
            ReplayOptions {
                speed: 1000.0,
                ..ReplayOptions::default()
            },
        )
        .unwrap();
        assert_eq!(result.total_events, 2);
        assert_eq!(result.replayed_events, 2);
    }

    #[test]
    fn session_replay_with_half_speed_doubles_timestamps() {
        let project = temp_project("session-replay-half");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        record_session_event(&id, sample_event("1100Z", "input", "second")).unwrap();
        let result = replay_session(
            &id,
            ReplayOptions {
                speed: 0.5,
                ..ReplayOptions::default()
            },
        )
        .unwrap();
        assert!(result.duration_ms >= 200);
    }

    #[test]
    fn session_replay_stops_on_error_when_configured() {
        let project = temp_project("session-replay-error");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        let mut failing = sample_event("1001Z", "input", "boom");
        failing.payload = json!({ "replayError": true });
        record_session_event(&id, failing).unwrap();
        record_session_event(&id, sample_event("1002Z", "input", "after")).unwrap();
        let result = replay_session(
            &id,
            ReplayOptions {
                speed: 1000.0,
                stop_on_error: true,
                filter_types: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(result.replayed_events, 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn session_timeline_returns_ordered_events() {
        let project = temp_project("session-timeline");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("3000Z", "input", "third")).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "first")).unwrap();
        let timeline = timeline_session(&project, Some(&id), None, None).unwrap();
        assert_eq!(timeline.events[0].summary, "first");
        assert_eq!(timeline.events[1].summary, "third");
    }

    #[test]
    fn session_report_includes_error_summary() {
        let project = temp_project("session-report");
        let (id, _) = start_session(&project).unwrap();
        record_session_event(&id, sample_event("1000Z", "input", "ok")).unwrap();
        let mut error = sample_event("2000Z", "tool_call", "failed tool");
        error.payload = json!({ "ok": false, "error": "failed tool" });
        record_session_event(&id, error).unwrap();
        let report = report_session(&project, &id).unwrap();
        assert_eq!(report.total_events, 2);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.errors[0], "failed tool");
    }

    #[test]
    fn session_file_roundtrips_through_serde() {
        let file = SessionFile {
            header: SessionHeader {
                id: "session-1".to_string(),
                created_at: "1000Z".to_string(),
                project_path: PathBuf::from("/tmp/project"),
                unity_version: Some("2022.3".to_string()),
                lux_version: "0.1.0".to_string(),
                retention_metadata: None,
            },
            events: vec![sample_event("1000Z", "input", "first")],
        };
        let serialized = serde_json::to_string(&file).unwrap();
        let deserialized: SessionFile = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, file);
    }

    fn sample_event(timestamp: &str, event_type: &str, summary: &str) -> SessionEvent {
        SessionEvent {
            timestamp_utc: timestamp.to_string(),
            event_type: event_type.to_string(),
            category: "test".to_string(),
            source: "unit-test".to_string(),
            summary: summary.to_string(),
            payload: json!({ "summary": summary }),
            redaction_metadata: None,
            retention_metadata: None,
        }
    }

    fn temp_project(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lux-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
