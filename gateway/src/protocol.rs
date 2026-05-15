use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: u32 = 1;
pub const CMD_START_LUX_STREAM: &str = "start_lux_stream";
pub const CMD_STOP_LUX_STREAM: &str = "stop_lux_stream";
pub const CMD_LUX_STREAM_FRAME: &str = "lux_stream_frame";
pub const CMD_LUX_INPUT_EVENT: &str = "lux_input_event";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartLuxStreamRequest {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuxStreamFrame {
    pub session_id: String,
    pub frame: String,
    pub sequence: u64,
    pub timestamp: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuxInputEvent {
    pub session_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub button: Option<i32>,
    pub key_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[allow(dead_code)] // Bridge protocol type — reserved for future capture/streaming integration
pub enum CaptureCommand {
    StartLuxStream {
        width: u32,
        height: u32,
        fps: u32,
    },
    StopLuxStream {
        session_id: String,
    },
    LuxFrame {
        session_id: String,
        frame_data: Vec<u8>,
        sequence: u64,
    },
    LuxInputEvent {
        session_id: String,
        event_type: String,
        payload: Value,
    },
}

/// Stable identifier for the origin of an event.
/// Every LuxEvent carries exactly one source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventSource {
    /// User action inside the Unity Editor (Inspector, menu, console, etc.)
    Editor,
    /// AI agent work step (codex, opencode, MCP tool invocation, skill execution)
    Ai,
    /// Gameplay runtime event (playmode callbacks, player input, scene lifecycle)
    Runtime,
}

/// High-level classification of an event within the LUX pipeline.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventCategory {
    Playmode,
    Scene,
    Log,
    AiActionLog,
    Tool,
    Input,
    Screenshot,
    Hierarchy,
}

/// Records which fields were redacted, why, and when — without leaking original values.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactionMetadata {
    pub redacted_fields: Vec<String>,
    pub redaction_classes: Vec<String>,
    /// ISO 8601 timestamp of when redaction was applied.
    pub timestamp: Option<String>,
}

/// Describes how long an event or log entry should be retained.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionMetadata {
    pub max_age_days: Option<u32>,
    pub max_lines: Option<usize>,
    pub policy: Option<String>,
    /// ISO 8601 timestamp when the retention window was created.
    pub created_at: Option<String>,
    /// ISO 8601 timestamp when the retention window expires.
    pub expires_at: Option<String>,
}

/// Unified event schema representing Editor user actions, AI work steps, and gameplay runtime events.
///
/// This is the canonical wire format for every event that flows through the LUX pipeline.
/// It carries stable identifiers, timestamps, project context, classification, a human-readable
/// summary, structured payload, and optional redaction/retention metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventEnvelope {
    pub schema_version: u32,
    /// Stable UUID-style event identifier.
    pub event_id: String,
    pub category: EventCategory,
    /// Origin of the event (editor user action, AI work step, or runtime).
    pub source: EventSource,
    /// Session that produced this event.
    pub session_id: String,
    /// ISO 8601 UTC timestamp of when the event was captured.
    pub captured_at_utc: String,
    /// File-system path to the Unity project (redacted in transit).
    pub project_path: Option<String>,
    /// Human-readable one-line summary of what happened.
    pub summary: Option<String>,
    pub redaction_metadata: Option<RedactionMetadata>,
    pub retention_metadata: Option<RetentionMetadata>,
    /// Arbitrary structured payload specific to the event category.
    pub payload: Value,
}

#[allow(dead_code)]
pub type LuxEvent = EventEnvelope;

impl EventEnvelope {
    pub fn schema_example() -> Self {
        Self {
            schema_version: PROTOCOL_VERSION,
            event_id: "example-event".to_string(),
            category: EventCategory::Tool,
            source: EventSource::Editor,
            session_id: "example-session".to_string(),
            captured_at_utc: "2026-04-30T00:00:00.0000000Z".to_string(),
            project_path: Some("/Users/example/UnityProjects/NeonGlitch".to_string()),
            summary: Some("Lux gateway event envelope prototype".to_string()),
            redaction_metadata: Some(RedactionMetadata {
                redacted_fields: vec!["summary".to_string(), "payload.token".to_string()],
                redaction_classes: vec!["secret".to_string(), "project_path".to_string()],
                timestamp: Some("2026-04-30T00:01:00Z".to_string()),
            }),
            retention_metadata: Some(RetentionMetadata {
                max_age_days: Some(30),
                max_lines: Some(10_000),
                policy: Some("default".to_string()),
                created_at: Some("2026-04-30T00:00:00Z".to_string()),
                expires_at: Some("2026-05-30T00:00:00Z".to_string()),
            }),
            payload: json!({
                "kind": "example",
                "message": "Lux gateway event envelope prototype"
            }),
        }
    }

    pub fn normalize(mut self) -> Self {
        if self.schema_version == 0 {
            self.schema_version = PROTOCOL_VERSION;
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_example_has_phase_one_categories() {
        let json = serde_json::to_value(EventEnvelope::schema_example()).unwrap();
        assert_eq!(json["schema_version"], PROTOCOL_VERSION);
        assert_eq!(json["category"], "tool");
    }

    #[test]
    fn all_categories_serialize_as_protocol_names() {
        let names = [
            EventCategory::Playmode,
            EventCategory::Scene,
            EventCategory::Log,
            EventCategory::AiActionLog,
            EventCategory::Tool,
            EventCategory::Input,
            EventCategory::Screenshot,
            EventCategory::Hierarchy,
        ]
        .map(|category| serde_json::to_value(category).unwrap());

        assert_eq!(
            names,
            [
                json!("playmode"),
                json!("scene"),
                json!("log"),
                json!("ai-action-log"),
                json!("tool"),
                json!("input"),
                json!("screenshot"),
                json!("hierarchy"),
            ]
        );
    }

    #[test]
    fn ai_action_log_roundtrips_through_serde() {
        let serialized = serde_json::to_value(EventCategory::AiActionLog).unwrap();
        assert_eq!(serialized, json!("ai-action-log"));

        let deserialized: EventCategory = serde_json::from_value(json!("ai-action-log")).unwrap();
        assert_eq!(deserialized, EventCategory::AiActionLog);
    }

    #[test]
    fn enriched_event_schema_roundtrips_through_serde() {
        let event = EventEnvelope {
            schema_version: PROTOCOL_VERSION,
            event_id: "event-1".to_string(),
            category: EventCategory::AiActionLog,
            source: EventSource::Ai,
            session_id: "session-1".to_string(),
            captured_at_utc: "2026-05-05T00:00:00Z".to_string(),
            project_path: Some("/project".to_string()),
            summary: Some("AI work step completed".to_string()),
            redaction_metadata: Some(RedactionMetadata {
                redacted_fields: vec!["payload.email".to_string()],
                redaction_classes: vec!["gameplay_sensitive".to_string()],
                timestamp: Some("2026-05-05T00:00:01Z".to_string()),
            }),
            retention_metadata: Some(RetentionMetadata {
                max_age_days: Some(7),
                max_lines: Some(100),
                policy: Some("aggressive".to_string()),
                created_at: Some("2026-05-05T00:00:00Z".to_string()),
                expires_at: Some("2026-05-12T00:00:00Z".to_string()),
            }),
            payload: json!({ "kind": "work-step" }),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("project_path"));
        assert!(serialized.contains("redaction_metadata"));
        assert!(serialized.contains("retention_metadata"));
        assert!(serialized.contains("summary"));
        assert!(serialized.contains("\"source\":\"ai\""));

        let deserialized: EventEnvelope = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, event);
        assert_eq!(
            deserialized.summary.as_deref(),
            Some("AI work step completed")
        );
        assert_eq!(deserialized.source, EventSource::Ai);
        assert_eq!(
            deserialized
                .redaction_metadata
                .as_ref()
                .and_then(|r| r.timestamp.as_deref()),
            Some("2026-05-05T00:00:01Z")
        );
        assert_eq!(
            deserialized
                .retention_metadata
                .as_ref()
                .and_then(|r| r.created_at.as_deref()),
            Some("2026-05-05T00:00:00Z")
        );
        assert_eq!(
            deserialized
                .retention_metadata
                .as_ref()
                .and_then(|r| r.expires_at.as_deref()),
            Some("2026-05-12T00:00:00Z")
        );
    }

    #[test]
    fn test_start_lux_stream_serialization() {
        let request = StartLuxStreamRequest {
            width: 1920,
            height: 1080,
            fps: 60,
            session_id: "capture-1".to_string(),
        };

        let serialized = serde_json::to_value(request).unwrap();

        assert_eq!(
            serialized,
            json!({
                "width": 1920,
                "height": 1080,
                "fps": 60,
                "sessionId": "capture-1"
            })
        );
    }

    #[test]
    fn test_lux_stream_frame_deserialization() {
        let frame: LuxStreamFrame = serde_json::from_value(json!({
            "sessionId": "capture-1",
            "frame": "/9j/4AAQSkZJRg==",
            "sequence": 12,
            "timestamp": "2026-05-10T00:00:00Z"
        }))
        .unwrap();

        assert_eq!(frame.session_id, "capture-1");
        assert_eq!(frame.frame, "/9j/4AAQSkZJRg==");
        assert_eq!(frame.sequence, 12);
        assert_eq!(frame.timestamp, "2026-05-10T00:00:00Z");
    }

    #[test]
    fn test_lux_input_event_deserialization() {
        let event: LuxInputEvent = serde_json::from_value(json!({
            "sessionId": "capture-1",
            "type": "mouseDown",
            "x": 10.5,
            "y": 20.25,
            "button": 0,
            "keyCode": "Space"
        }))
        .unwrap();

        assert_eq!(event.session_id, "capture-1");
        assert_eq!(event.event_type, "mouseDown");
        assert_eq!(event.x, Some(10.5));
        assert_eq!(event.y, Some(20.25));
        assert_eq!(event.button, Some(0));
        assert_eq!(event.key_code.as_deref(), Some("Space"));
    }

    #[test]
    fn test_command_constants() {
        assert_eq!(CMD_START_LUX_STREAM, "start_lux_stream");
        assert_eq!(CMD_STOP_LUX_STREAM, "stop_lux_stream");
        assert_eq!(CMD_LUX_STREAM_FRAME, "lux_stream_frame");
        assert_eq!(CMD_LUX_INPUT_EVENT, "lux_input_event");
    }

    #[test]
    fn test_capture_command_serde() {
        let commands = vec![
            CaptureCommand::StartLuxStream {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            CaptureCommand::StopLuxStream {
                session_id: "capture-1".to_string(),
            },
            CaptureCommand::LuxFrame {
                session_id: "capture-1".to_string(),
                frame_data: vec![1, 2, 3, 4],
                sequence: 12,
            },
            CaptureCommand::LuxInputEvent {
                session_id: "capture-1".to_string(),
                event_type: "mouseDown".to_string(),
                payload: json!({
                    "x": 10.5,
                    "y": 20.25,
                    "button": 0,
                    "keyCode": "Space"
                }),
            },
        ];

        for command in commands {
            let serialized = serde_json::to_value(&command).unwrap();
            let deserialized: CaptureCommand = serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized, command);
        }
    }
}
