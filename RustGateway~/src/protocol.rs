use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: u32 = 1;

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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactionMetadata {
    pub redacted_fields: Vec<String>,
    pub redaction_classes: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionMetadata {
    pub max_age_days: Option<u32>,
    pub max_lines: Option<usize>,
    pub policy: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventEnvelope {
    pub schema_version: u32,
    pub event_id: String,
    pub category: EventCategory,
    pub source: String,
    pub session_id: String,
    pub captured_at_utc: String,
    pub project_path: Option<String>,
    pub redaction_metadata: Option<RedactionMetadata>,
    pub retention_metadata: Option<RetentionMetadata>,
    pub payload: Value,
}

impl EventEnvelope {
    pub fn schema_example() -> Self {
        Self {
            schema_version: PROTOCOL_VERSION,
            event_id: "example-event".to_string(),
            category: EventCategory::Tool,
            source: "unity-editor".to_string(),
            session_id: "example-session".to_string(),
            captured_at_utc: "2026-04-30T00:00:00.0000000Z".to_string(),
            project_path: Some("/Users/example/UnityProjects/NeonGlitch".to_string()),
            redaction_metadata: Some(RedactionMetadata {
                redacted_fields: vec!["summary".to_string(), "payload.token".to_string()],
                redaction_classes: vec!["secret".to_string(), "project_path".to_string()],
            }),
            retention_metadata: Some(RetentionMetadata {
                max_age_days: Some(30),
                max_lines: Some(10_000),
                policy: Some("default".to_string()),
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
            source: "lux-gateway".to_string(),
            session_id: "session-1".to_string(),
            captured_at_utc: "2026-05-05T00:00:00Z".to_string(),
            project_path: Some("/project".to_string()),
            redaction_metadata: Some(RedactionMetadata {
                redacted_fields: vec!["payload.email".to_string()],
                redaction_classes: vec!["gameplay_sensitive".to_string()],
            }),
            retention_metadata: Some(RetentionMetadata {
                max_age_days: Some(7),
                max_lines: Some(100),
                policy: Some("aggressive".to_string()),
            }),
            payload: json!({ "kind": "work-step" }),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("project_path"));
        assert!(serialized.contains("redaction_metadata"));
        assert!(serialized.contains("retention_metadata"));

        let deserialized: EventEnvelope = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, event);
    }
}
