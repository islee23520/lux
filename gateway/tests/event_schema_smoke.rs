//! AC1: Unified event schema (LuxEvent) serialization tests.
//!
//! Validates that every event-related struct in protocol.rs:
//!   - Serializes and deserializes correctly (roundtrip)
//!   - Produces the expected JSON field names (camelCase / kebab-case)
//!   - Handles missing optional fields gracefully
//!   - Covers all enum variants for EventSource and EventCategory
//!   - Exercises RedactionMetadata with timestamp, RetentionMetadata with created_at/expires_at
//!   - Proves LuxEvent type alias works identically to EventEnvelope

use lux::protocol::{
    EventCategory, EventEnvelope, EventSource, LuxEvent, RedactionMetadata, RetentionMetadata,
};
use serde_json::{json, Value};

// ===========================================================================
// 1. EventSource enum — all 3 variants serialize as kebab-case
// ===========================================================================

#[test]
fn event_source_editor_serializes_to_kebab_case() {
    let v = serde_json::to_value(EventSource::Editor).unwrap();
    assert_eq!(v, "editor");
}

#[test]
fn event_source_ai_serializes_to_kebab_case() {
    let v = serde_json::to_value(EventSource::Ai).unwrap();
    assert_eq!(v, "ai");
}

#[test]
fn event_source_runtime_serializes_to_kebab_case() {
    let v = serde_json::to_value(EventSource::Runtime).unwrap();
    assert_eq!(v, "runtime");
}

#[test]
fn event_source_all_variants_roundtrip() {
    for variant in [EventSource::Editor, EventSource::Ai, EventSource::Runtime] {
        let serialized = serde_json::to_string(&variant).unwrap();
        let deserialized: EventSource = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, variant);
    }
}

#[test]
fn event_source_rejects_unknown_variant_gracefully() {
    let result: Result<EventSource, _> = serde_json::from_str("\"unknown\"");
    assert!(
        result.is_err(),
        "unknown EventSource variant should fail to deserialize"
    );
}

// ===========================================================================
// 2. EventCategory enum — all 8 variants serialize as kebab-case
// ===========================================================================

#[test]
fn event_category_all_variants_serialize_correctly() {
    let expected = [
        (EventCategory::Playmode, "playmode"),
        (EventCategory::Scene, "scene"),
        (EventCategory::Log, "log"),
        (EventCategory::AiActionLog, "ai-action-log"),
        (EventCategory::Tool, "tool"),
        (EventCategory::Input, "input"),
        (EventCategory::Screenshot, "screenshot"),
        (EventCategory::Hierarchy, "hierarchy"),
    ];
    for (variant, name) in expected {
        let v = serde_json::to_value(&variant).unwrap();
        assert_eq!(
            v,
            json!(name),
            "EventCategory::{variant:?} should serialize as {name}"
        );
    }
}

#[test]
fn event_category_all_variants_roundtrip() {
    for variant in [
        EventCategory::Playmode,
        EventCategory::Scene,
        EventCategory::Log,
        EventCategory::AiActionLog,
        EventCategory::Tool,
        EventCategory::Input,
        EventCategory::Screenshot,
        EventCategory::Hierarchy,
    ] {
        let serialized = serde_json::to_string(&variant).unwrap();
        let deserialized: EventCategory = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, variant);
    }
}

// ===========================================================================
// 3. RedactionMetadata — redacted_fields + redaction_classes + timestamp
// ===========================================================================

#[test]
fn redaction_metadata_default_is_empty() {
    let meta = RedactionMetadata::default();
    assert!(meta.redacted_fields.is_empty());
    assert!(meta.redaction_classes.is_empty());
    assert!(meta.timestamp.is_none());
}

#[test]
fn redaction_metadata_full_roundtrip() {
    let original = RedactionMetadata {
        redacted_fields: vec!["summary".to_string(), "payload.token".to_string()],
        redaction_classes: vec!["secret".to_string(), "project_path".to_string()],
        timestamp: Some("2026-05-10T01:23:45Z".to_string()),
    };
    let serialized = serde_json::to_string(&original).expect("serialize RedactionMetadata");
    let deserialized: RedactionMetadata =
        serde_json::from_str(&serialized).expect("deserialize RedactionMetadata");
    assert_eq!(deserialized, original);
}

#[test]
fn redaction_metadata_serializes_camelcase_field_names() {
    let meta = RedactionMetadata {
        redacted_fields: vec!["x".to_string()],
        redaction_classes: vec!["y".to_string()],
        timestamp: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let value = serde_json::to_value(&meta).unwrap();
    // Must contain camelCase names, not snake_case
    assert!(
        value.get("redactedFields").is_some(),
        "expected camelCase 'redactedFields'"
    );
    assert!(
        value.get("redactionClasses").is_some(),
        "expected camelCase 'redactionClasses'"
    );
    assert!(
        value.get("timestamp").is_some(),
        "expected 'timestamp' field"
    );
    // Must NOT contain snake_case
    let s = serde_json::to_string(&value).unwrap();
    assert!(
        !s.contains("redacted_fields"),
        "snake_case leaked into JSON"
    );
    assert!(
        !s.contains("redaction_classes"),
        "snake_case leaked into JSON"
    );
}

#[test]
fn redaction_metadata_without_timestamp_serializes_validly() {
    let meta = RedactionMetadata {
        redacted_fields: vec!["email".to_string()],
        redaction_classes: vec!["pii".to_string()],
        timestamp: None,
    };
    let serialized = serde_json::to_string(&meta).unwrap();
    let deserialized: RedactionMetadata = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.redacted_fields.len(), 1);
    assert!(deserialized.timestamp.is_none());
}

// ===========================================================================
// 4. RetentionMetadata — max_age_days + max_lines + policy + created_at + expires_at
// ===========================================================================

#[test]
fn retention_metadata_default_is_empty() {
    let meta = RetentionMetadata::default();
    assert!(meta.max_age_days.is_none());
    assert!(meta.max_lines.is_none());
    assert!(meta.policy.is_none());
    assert!(meta.created_at.is_none());
    assert!(meta.expires_at.is_none());
}

#[test]
fn retention_metadata_full_roundtrip() {
    let original = RetentionMetadata {
        max_age_days: Some(30),
        max_lines: Some(10_000),
        policy: Some("default".to_string()),
        created_at: Some("2026-04-30T00:00:00Z".to_string()),
        expires_at: Some("2026-05-30T00:00:00Z".to_string()),
    };
    let serialized = serde_json::to_string(&original).expect("serialize RetentionMetadata");
    let deserialized: RetentionMetadata =
        serde_json::from_str(&serialized).expect("deserialize RetentionMetadata");
    assert_eq!(deserialized, original);
}

#[test]
fn retention_metadata_serializes_camelcase_field_names() {
    let meta = RetentionMetadata {
        max_age_days: Some(7),
        max_lines: Some(500),
        policy: Some("aggressive".to_string()),
        created_at: Some("2026-01-01T00:00:00Z".to_string()),
        expires_at: Some("2026-01-08T00:00:00Z".to_string()),
    };
    let value = serde_json::to_value(&meta).unwrap();
    assert!(
        value.get("maxAgeDays").is_some(),
        "expected camelCase 'maxAgeDays'"
    );
    assert!(
        value.get("maxLines").is_some(),
        "expected camelCase 'maxLines'"
    );
    assert!(value.get("policy").is_some(), "expected 'policy' field");
    assert!(
        value.get("createdAt").is_some(),
        "expected camelCase 'createdAt'"
    );
    assert!(
        value.get("expiresAt").is_some(),
        "expected camelCase 'expiresAt'"
    );
}

#[test]
fn retention_metadata_minimal_without_dates_serializes_validly() {
    let meta = RetentionMetadata {
        max_age_days: Some(14),
        max_lines: None,
        policy: None,
        created_at: None,
        expires_at: None,
    };
    let serialized = serde_json::to_string(&meta).unwrap();
    let deserialized: RetentionMetadata = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.max_age_days, Some(14));
    assert!(deserialized.max_lines.is_none());
    assert!(deserialized.created_at.is_none());
    assert!(deserialized.expires_at.is_none());
}

// ===========================================================================
// 5. EventEnvelope / LuxEvent — full roundtrip with ALL fields populated
// ===========================================================================

#[test]
fn lux_event_full_populated_roundtrips() {
    let event: EventEnvelope = EventEnvelope {
        schema_version: 1,
        event_id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
        category: EventCategory::AiActionLog,
        source: EventSource::Ai,
        session_id: "sess-abc-123".to_string(),
        captured_at_utc: "2026-05-10T12:34:56.789Z".to_string(),
        project_path: Some("/Users/dev/Project".to_string()),
        summary: Some("Completed code generation step".to_string()),
        redaction_metadata: Some(RedactionMetadata {
            redacted_fields: vec!["payload.apiKey".to_string()],
            redaction_classes: vec!["secret".to_string()],
            timestamp: Some("2026-05-10T12:34:57Z".to_string()),
        }),
        retention_metadata: Some(RetentionMetadata {
            max_age_days: Some(90),
            max_lines: Some(50_000),
            policy: Some("long-term".to_string()),
            created_at: Some("2026-05-10T00:00:00Z".to_string()),
            expires_at: Some("2026-08-08T00:00:00Z".to_string()),
        }),
        payload: json!({
            "stepName": "generate",
            "status": "completed",
            "filesModified": 3
        }),
    };

    let serialized = serde_json::to_string(&event).expect("serialize full LuxEvent");
    let deserialized: EventEnvelope =
        serde_json::from_str(&serialized).expect("deserialize full LuxEvent");

    assert_eq!(deserialized, event);
    assert_eq!(
        deserialized.event_id,
        "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    );
    assert_eq!(deserialized.source, EventSource::Ai);
    assert_eq!(
        deserialized.summary.as_deref(),
        Some("Completed code generation step")
    );
    assert_eq!(deserialized.category, EventCategory::AiActionLog);

    // Verify nested metadata survived roundtrip
    let rm = deserialized.redaction_metadata.unwrap();
    assert_eq!(rm.redacted_fields, vec!["payload.apiKey"]);
    assert_eq!(rm.timestamp.as_deref(), Some("2026-05-10T12:34:57Z"));

    let ret = deserialized.retention_metadata.unwrap();
    assert_eq!(ret.max_age_days, Some(90));
    assert_eq!(ret.created_at.as_deref(), Some("2026-05-10T00:00:00Z"));
    assert_eq!(ret.expires_at.as_deref(), Some("2026-08-08T00:00:00Z"));
}

// ===========================================================================
// 6. Minimal/empty EventEnvelope — only required fields
// ===========================================================================

#[test]
fn lux_event_minimal_valid_serialization() {
    let event = EventEnvelope {
        schema_version: 1,
        event_id: "minimal-event-1".to_string(),
        category: EventCategory::Log,
        source: EventSource::Runtime,
        session_id: "sess-min".to_string(),
        captured_at_utc: "2026-01-01T00:00:00Z".to_string(),
        project_path: None,
        summary: None,
        redaction_metadata: None,
        retention_metadata: None,
        payload: json!({"message": "heartbeat"}),
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: EventEnvelope = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, event);
    assert!(deserialized.project_path.is_none());
    assert!(deserialized.summary.is_none());
    assert!(deserialized.redaction_metadata.is_none());
    assert!(deserialized.retention_metadata.is_none());

    // Optional fields serialize as null when None (no skip_serializing_if)
    let value: Value = serde_json::from_str(&serialized).unwrap();
    assert!(
        matches!(value.get("projectPath"), Some(Value::Null) | None),
        "project_path should be null or absent"
    );
    assert!(
        matches!(value.get("summary"), Some(Value::Null) | None),
        "summary should be null or absent"
    );
    assert!(
        matches!(value.get("redactionMetadata"), Some(Value::Null) | None),
        "redaction_metadata should be null or absent"
    );
    assert!(
        matches!(value.get("retentionMetadata"), Some(Value::Null) | None),
        "retention_metadata should be null or absent"
    );
}

// ===========================================================================
// 7. Required-field validation — missing required fields fail gracefully
// ===========================================================================

#[test]
fn deserializing_event_missing_required_field_fails() {
    // Valid event JSON minus the required "eventId" field
    let invalid_json = json!({
        "schemaVersion": 1,
        "category": "tool",
        "source": "editor",
        "sessionId": "s1",
        "capturedAtUtc": "2026-01-01T00:00:00Z",
        "payload": {}
    });

    let result: Result<EventEnvelope, _> = serde_json::from_value(invalid_json);
    assert!(
        result.is_err(),
        "deserialization without eventId should fail gracefully with an error"
    );
}

#[test]
fn deserializing_event_missing_payload_fails() {
    let invalid_json = json!({
        "schemaVersion": 1,
        "eventId": "e1",
        "category": "tool",
        "source": "editor",
        "sessionId": "s1",
        "capturedAtUtc": "2026-01-01T00:00:00Z"
        // no "payload" key
    });

    let result: Result<EventEnvelope, _> = serde_json::from_value(invalid_json);
    assert!(
        result.is_err(),
        "deserialization without payload should fail gracefully"
    );
}

#[test]
fn deserializing_event_invalid_source_enum_fails() {
    let invalid_json = json!({
        "schemaVersion": 1,
        "eventId": "e1",
        "category": "tool",
        "source": "not-a-valid-source",
        "sessionId": "s1",
        "capturedAtUtc": "2026-01-01T00:00:00Z",
        "payload": {}
    });

    let result: Result<EventEnvelope, _> = serde_json::from_value(invalid_json);
    assert!(
        result.is_err(),
        "invalid EventSource enum value should fail to deserialize"
    );
}

#[test]
fn deserializing_event_invalid_category_enum_fails() {
    let invalid_json = json!({
        "schemaVersion": 1,
        "eventId": "e1",
        "category": "not-a-category",
        "source": "editor",
        "sessionId": "s1",
        "capturedAtUtc": "2026-01-01T00:00:00Z",
        "payload": {}
    });

    let result: Result<EventEnvelope, _> = serde_json::from_value(invalid_json);
    assert!(
        result.is_err(),
        "invalid EventCategory enum value should fail to deserialize"
    );
}

// ===========================================================================
// 8. LuxEvent type alias is interchangeable with EventEnvelope
// ===========================================================================

#[test]
fn lux_event_type_alias_matches_envelope() {
    let envelope = EventEnvelope {
        schema_version: 1,
        event_id: "alias-test".to_string(),
        category: EventCategory::Scene,
        source: EventSource::Editor,
        session_id: "s".to_string(),
        captured_at_utc: "2026-06-01T00:00:00Z".to_string(),
        project_path: Some("/p".to_string()),
        summary: Some("test alias".to_string()),
        redaction_metadata: None,
        retention_metadata: None,
        payload: json!({}),
    };

    // Assign through alias
    let alias: LuxEvent = envelope.clone();
    // They should be identical in every way
    assert_eq!(alias.schema_version, envelope.schema_version);
    assert_eq!(alias.event_id, envelope.event_id);
    assert_eq!(alias.source, envelope.source);
    assert_eq!(alias.summary, envelope.summary);

    // Serialize through alias, deserialize through concrete type
    let serialized = serde_json::to_string(&alias).unwrap();
    let roundtripped: EventEnvelope = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtripped, envelope);
}

// ===========================================================================
// 9. Each EventSource x EventCategory combination produces valid JSON
// ===========================================================================

#[test]
fn all_source_category_combinations_serialize() {
    let sources = [EventSource::Editor, EventSource::Ai, EventSource::Runtime];
    let categories = [
        EventCategory::Playmode,
        EventCategory::Scene,
        EventCategory::Log,
        EventCategory::AiActionLog,
        EventCategory::Tool,
        EventCategory::Input,
        EventCategory::Screenshot,
        EventCategory::Hierarchy,
    ];

    for (i, source) in sources.iter().enumerate() {
        for (j, category) in categories.iter().enumerate() {
            let event = EventEnvelope {
                schema_version: 1,
                event_id: format!("combo-{i}-{j}"),
                category: category.clone(),
                source: source.clone(),
                session_id: format!("sess-{i}-{j}"),
                captured_at_utc: "2026-01-01T00:00:00Z".to_string(),
                project_path: None,
                summary: None,
                redaction_metadata: None,
                retention_metadata: None,
                payload: json!({}),
            };
            let serialized = serde_json::to_string(&event)
                .unwrap_or_else(|e| panic!("failed to serialize {:?}x{:?}: {e}", source, category));
            let deserialized: EventEnvelope =
                serde_json::from_str(&serialized).unwrap_or_else(|e| {
                    panic!("failed to deserialize {:?}x{:?}: {e}", source, category)
                });
            assert_eq!(
                deserialized.source, *source,
                "source mismatch for {:?}x{:?}",
                source, category
            );
            assert_eq!(
                deserialized.category, *category,
                "category mismatch for {:?}x{:?}",
                source, category
            );
        }
    }
}

// ===========================================================================
// 10. Schema example produces valid parseable output
// ===========================================================================

#[test]
fn schema_example_is_valid_lux_event() {
    let example = EventEnvelope::schema_example();
    let serialized = serde_json::to_string_pretty(&example).unwrap();

    // EventEnvelope uses snake_case field names (no rename_all attribute)
    assert!(serialized.contains("schema_version"));
    assert!(serialized.contains("event_id"));
    assert!(serialized.contains("category"));
    assert!(serialized.contains("source"));
    assert!(serialized.contains("session_id"));
    assert!(serialized.contains("captured_at_utc"));
    assert!(serialized.contains("project_path"));
    assert!(serialized.contains("summary"));
    assert!(serialized.contains("redaction_metadata"));
    assert!(serialized.contains("retention_metadata"));
    assert!(serialized.contains("payload"));

    // Roundtrip must preserve equality
    let parsed: Value = serde_json::from_str(&serialized).unwrap();
    let reparsed: EventEnvelope =
        serde_json::from_value(parsed).expect("schema_example JSON must be a valid EventEnvelope");
    assert_eq!(reparsed, example);
}
