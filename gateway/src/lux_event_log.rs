use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_BUFFER_SIZE: usize = 50;

fn now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid ISO 8601 timestamp: {value}"))?
        .with_timezone(&Utc))
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    Ok(())
}

fn read_jsonl_events(path: &Path) -> Result<Vec<PlayEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file =
        File::open(path).with_context(|| format!("failed to open event log {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read event log {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let event: PlayEvent = serde_json::from_str(&line)
            .with_context(|| format!("invalid JSONL event in {}", path.display()))?;
        events.push(event);
    }

    Ok(events)
}

fn write_jsonl_events(path: &Path, events: &[PlayEvent]) -> Result<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open event log {}", path.display()))?;

    for event in events {
        let line = serde_json::to_string(event).context("failed to serialize play event")?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to write event log {}", path.display()))?;
    }

    file.flush()
        .with_context(|| format!("failed to flush event log {}", path.display()))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayEventType {
    Action,
    Decision,
    Trigger,
    Death,
    LevelComplete,
    LevelStart,
    ItemCollect,
    Damage,
    MenuOpen,
    MenuClose,
    CutsceneStart,
    CutsceneEnd,
    Save,
    Load,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayEvent {
    pub session_id: String,
    pub timestamp: String,
    pub event_type: PlayEventType,
    pub payload: Value,
    pub player_id: Option<String>,
    pub game_state: Option<Value>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_secs: Option<f64>,
    pub event_count: u64,
    pub webgl_build_version: Option<String>,
    pub player_id: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventFilter {
    pub session_id: Option<String>,
    pub event_type: Option<PlayEventType>,
    pub from_time: Option<String>,
    pub to_time: Option<String>,
    pub limit: Option<usize>,
}

pub struct EventBuffer {
    events: Vec<PlayEvent>,
    buffer_size: usize,
    session_id: String,
    base_path: PathBuf,
}

impl EventBuffer {
    pub fn new(session_id: impl Into<String>, base_path: impl Into<PathBuf>) -> Self {
        Self {
            events: Vec::new(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            session_id: session_id.into(),
            base_path: base_path.into(),
        }
    }

    pub fn with_buffer_size(
        session_id: impl Into<String>,
        base_path: impl Into<PathBuf>,
        buffer_size: usize,
    ) -> Self {
        Self {
            events: Vec::new(),
            buffer_size: buffer_size.max(1),
            session_id: session_id.into(),
            base_path: base_path.into(),
        }
    }

    fn event_log_path(&self) -> PathBuf {
        self.base_path.join(format!("{}.jsonl", self.session_id))
    }

    pub fn append(&mut self, event: PlayEvent) -> Result<()> {
        self.events.push(event);
        if self.events.len() >= self.buffer_size {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.events.is_empty() {
            return Ok(());
        }

        let path = self.event_log_path();
        write_jsonl_events(&path, &self.events)?;
        self.events.clear();
        Ok(())
    }

    pub fn get_session_metadata(&self) -> SessionMetadata {
        let started_at = self
            .events
            .first()
            .map(|event| event.timestamp.clone())
            .unwrap_or_else(now_iso8601);
        let player_id = self.events.iter().find_map(|event| event.player_id.clone());

        SessionMetadata {
            session_id: self.session_id.clone(),
            started_at,
            ended_at: None,
            duration_secs: None,
            event_count: self.events.len() as u64,
            webgl_build_version: None,
            player_id,
            metadata: HashMap::new(),
        }
    }
}

pub trait EventLogStore {
    fn create_session(&self, meta: SessionMetadata) -> Result<()>;
    fn append_event(&self, event: PlayEvent) -> Result<()>;
    fn end_session(&self, session_id: &str) -> Result<SessionMetadata>;
    fn get_session(&self, session_id: &str) -> Result<Option<SessionMetadata>>;
    fn list_sessions(&self) -> Result<Vec<SessionMetadata>>;
    fn query_events(&self, filter: EventFilter) -> Result<Vec<PlayEvent>>;
}

#[derive(Debug, Clone)]
pub struct FileEventLogStore {
    base_path: PathBuf,
}

impl FileEventLogStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    fn meta_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(format!("{session_id}.meta.json"))
    }

    fn events_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(format!("{session_id}.jsonl"))
    }

    fn load_metadata(&self, session_id: &str) -> Result<Option<SessionMetadata>> {
        let path = self.meta_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)
            .with_context(|| format!("failed to open session metadata {}", path.display()))?;
        let meta = serde_json::from_reader(file)
            .with_context(|| format!("failed to parse session metadata {}", path.display()))?;
        Ok(Some(meta))
    }

    fn save_metadata(&self, meta: &SessionMetadata) -> Result<()> {
        ensure_parent_dir(&self.meta_path(&meta.session_id))?;
        let file = File::create(self.meta_path(&meta.session_id)).with_context(|| {
            format!(
                "failed to create session metadata {}",
                self.meta_path(&meta.session_id).display()
            )
        })?;
        serde_json::to_writer_pretty(file, meta).context("failed to serialize session metadata")?;
        Ok(())
    }
}

impl EventLogStore for FileEventLogStore {
    fn create_session(&self, meta: SessionMetadata) -> Result<()> {
        self.save_metadata(&meta)
    }

    fn append_event(&self, event: PlayEvent) -> Result<()> {
        let path = self.events_path(&event.session_id);
        write_jsonl_events(&path, &[event])
    }

    fn end_session(&self, session_id: &str) -> Result<SessionMetadata> {
        let mut meta = self
            .load_metadata(session_id)?
            .unwrap_or_else(|| SessionMetadata {
                session_id: session_id.to_string(),
                started_at: now_iso8601(),
                ended_at: None,
                duration_secs: None,
                event_count: 0,
                webgl_build_version: None,
                player_id: None,
                metadata: HashMap::new(),
            });

        let events = read_jsonl_events(&self.events_path(session_id))?;
        meta.event_count = events.len() as u64;
        if meta.player_id.is_none() {
            meta.player_id = events.iter().find_map(|event| event.player_id.clone());
        }

        let ended_at = now_iso8601();
        meta.duration_secs = Some(
            (parse_rfc3339_utc(&ended_at)? - parse_rfc3339_utc(&meta.started_at)?)
                .num_milliseconds() as f64
                / 1000.0,
        );
        meta.ended_at = Some(ended_at);
        self.save_metadata(&meta)?;
        Ok(meta)
    }

    fn get_session(&self, session_id: &str) -> Result<Option<SessionMetadata>> {
        self.load_metadata(session_id)
    }

    fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(&self.base_path)
            .with_context(|| format!("failed to read {}", self.base_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.ends_with(".meta.json") {
                continue;
            }
            let file = File::open(&path)
                .with_context(|| format!("failed to open session metadata {}", path.display()))?;
            let meta: SessionMetadata = serde_json::from_reader(file)
                .with_context(|| format!("failed to parse session metadata {}", path.display()))?;
            sessions.push(meta);
        }

        sessions.sort_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then(left.session_id.cmp(&right.session_id))
        });
        Ok(sessions)
    }

    fn query_events(&self, filter: EventFilter) -> Result<Vec<PlayEvent>> {
        let mut events = match filter.session_id.as_deref() {
            Some(session_id) => read_jsonl_events(&self.events_path(session_id))?,
            None => {
                let mut collected = Vec::new();
                for entry in fs::read_dir(&self.base_path)
                    .with_context(|| format!("failed to read {}", self.base_path.display()))?
                {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                        continue;
                    }
                    collected.extend(read_jsonl_events(&path)?);
                }
                collected
            }
        };

        let from_time = match filter.from_time.as_deref() {
            Some(value) => Some(parse_rfc3339_utc(value)?),
            None => None,
        };
        let to_time = match filter.to_time.as_deref() {
            Some(value) => Some(parse_rfc3339_utc(value)?),
            None => None,
        };

        events.retain(|event| {
            if let Some(expected) = filter.event_type.as_ref() {
                if &event.event_type != expected {
                    return false;
                }
            }
            let timestamp = match parse_rfc3339_utc(&event.timestamp) {
                Ok(timestamp) => timestamp,
                Err(_) => return false,
            };
            if from_time.as_ref().is_some_and(|from| timestamp < *from) {
                return false;
            }
            if to_time.as_ref().is_some_and(|to| timestamp > *to) {
                return false;
            }
            true
        });

        if let Some(limit) = filter.limit {
            events.truncate(limit);
        }

        Ok(events)
    }
}
