use lux::lux_event_log::{
    EventBuffer, EventFilter, EventLogStore, FileEventLogStore, PlayEvent, PlayEventType,
    SessionMetadata,
};
use serde_json::json;
use std::{collections::HashMap, fs, path::PathBuf};

fn temp_log_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lux-event-log-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn event(session_id: &str, timestamp: &str, event_type: PlayEventType, sequence: u64) -> PlayEvent {
    PlayEvent {
        session_id: session_id.to_string(),
        timestamp: timestamp.to_string(),
        event_type,
        payload: json!({"kind": "test", "sequence": sequence}),
        player_id: Some("player-1".to_string()),
        game_state: Some(json!({"hp": 99})),
        sequence,
    }
}

#[test]
fn test_event_create_valid() {
    let event = event(
        "session-1",
        "2026-05-11T10:00:00Z",
        PlayEventType::Action,
        1,
    );
    assert_eq!(event.session_id, "session-1");
    assert_eq!(event.sequence, 1);
    assert!(matches!(event.event_type, PlayEventType::Action));
}

#[test]
fn test_event_type_serialization() {
    let variants = vec![
        PlayEventType::Action,
        PlayEventType::Decision,
        PlayEventType::Trigger,
        PlayEventType::Death,
        PlayEventType::LevelComplete,
        PlayEventType::LevelStart,
        PlayEventType::ItemCollect,
        PlayEventType::Damage,
        PlayEventType::MenuOpen,
        PlayEventType::MenuClose,
        PlayEventType::CutsceneStart,
        PlayEventType::CutsceneEnd,
        PlayEventType::Save,
        PlayEventType::Load,
        PlayEventType::Custom("boss_phase_change".to_string()),
    ];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap();
        let roundtrip: PlayEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, variant);
    }
}

#[test]
fn test_event_buffer_append_flush() {
    let dir = temp_log_dir("append-flush");
    let mut buffer = EventBuffer::with_buffer_size("session-a", &dir, 10);
    buffer
        .append(event(
            "session-a",
            "2026-05-11T10:00:00Z",
            PlayEventType::Action,
            1,
        ))
        .unwrap();
    buffer
        .append(event(
            "session-a",
            "2026-05-11T10:00:01Z",
            PlayEventType::Decision,
            2,
        ))
        .unwrap();
    buffer.flush().unwrap();

    let log = fs::read_to_string(dir.join("session-a.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2);
}

#[test]
fn test_event_buffer_auto_flush() {
    let dir = temp_log_dir("auto-flush");
    let mut buffer = EventBuffer::with_buffer_size("session-b", &dir, 2);
    buffer
        .append(event(
            "session-b",
            "2026-05-11T10:00:00Z",
            PlayEventType::Action,
            1,
        ))
        .unwrap();
    buffer
        .append(event(
            "session-b",
            "2026-05-11T10:00:01Z",
            PlayEventType::Decision,
            2,
        ))
        .unwrap();

    let log = fs::read_to_string(dir.join("session-b.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2);
    assert_eq!(buffer.get_session_metadata().event_count, 0);
}

#[test]
fn test_session_metadata_lifecycle() {
    let dir = temp_log_dir("lifecycle");
    let store = FileEventLogStore::new(&dir);
    let meta = SessionMetadata {
        session_id: "session-c".to_string(),
        started_at: "2026-05-11T10:00:00Z".to_string(),
        ended_at: None,
        duration_secs: None,
        event_count: 0,
        webgl_build_version: Some("1.2.3".to_string()),
        player_id: Some("player-1".to_string()),
        metadata: HashMap::new(),
    };

    store.create_session(meta.clone()).unwrap();
    store
        .append_event(event(
            "session-c",
            "2026-05-11T10:00:01Z",
            PlayEventType::LevelStart,
            1,
        ))
        .unwrap();
    let ended = store.end_session("session-c").unwrap();
    let loaded = store.get_session("session-c").unwrap().unwrap();

    assert_eq!(ended.session_id, "session-c");
    assert_eq!(ended.event_count, 1);
    assert_eq!(loaded.ended_at, ended.ended_at);
}

#[test]
fn test_event_query_by_type() {
    let dir = temp_log_dir("query-type");
    let store = FileEventLogStore::new(&dir);
    store
        .append_event(event(
            "session-d",
            "2026-05-11T10:00:00Z",
            PlayEventType::Action,
            1,
        ))
        .unwrap();
    store
        .append_event(event(
            "session-d",
            "2026-05-11T10:00:01Z",
            PlayEventType::Decision,
            2,
        ))
        .unwrap();

    let events = store
        .query_events(EventFilter {
            session_id: Some("session-d".to_string()),
            event_type: Some(PlayEventType::Decision),
            from_time: None,
            to_time: None,
            limit: None,
        })
        .unwrap();

    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event_type, PlayEventType::Decision));
}

#[test]
fn test_event_query_by_time_range() {
    let dir = temp_log_dir("query-time");
    let store = FileEventLogStore::new(&dir);
    store
        .append_event(event(
            "session-e",
            "2026-05-11T10:00:00Z",
            PlayEventType::Action,
            1,
        ))
        .unwrap();
    store
        .append_event(event(
            "session-e",
            "2026-05-11T10:00:10Z",
            PlayEventType::Decision,
            2,
        ))
        .unwrap();

    let events = store
        .query_events(EventFilter {
            session_id: Some("session-e".to_string()),
            event_type: None,
            from_time: Some("2026-05-11T10:00:05Z".to_string()),
            to_time: Some("2026-05-11T10:00:20Z".to_string()),
            limit: None,
        })
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence, 2);
}

#[test]
fn test_jsonl_format_valid() {
    let dir = temp_log_dir("jsonl-valid");
    let mut buffer = EventBuffer::with_buffer_size("session-f", &dir, 10);
    buffer
        .append(event(
            "session-f",
            "2026-05-11T10:00:00Z",
            PlayEventType::Action,
            1,
        ))
        .unwrap();
    buffer.flush().unwrap();

    let log = fs::read_to_string(dir.join("session-f.jsonl")).unwrap();
    for line in log.lines() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["session_id"], "session-f");
    }
}
