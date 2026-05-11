use std::sync::{Arc, Mutex};

use lux::lux_events::{EventRouter, LuxEvent, LuxEventMessage};
use lux::protocol::EventEnvelope;
use serde_json::{json, Value};

fn sample_events() -> Vec<LuxEvent> {
    vec![
        LuxEvent::SpecUpdate {
            domain: "combat".to_string(),
            changes: json!({"hp": 120}),
        },
        LuxEvent::KanbanUpdate {
            ticket_id: "LUX-1".to_string(),
            new_status: "done".to_string(),
        },
        LuxEvent::TerminalOutput {
            session_id: "sess-1".to_string(),
            data: "hello".to_string(),
        },
        LuxEvent::TerminalInput {
            session_id: "sess-1".to_string(),
            data: "ls".to_string(),
        },
        LuxEvent::BuildProgress {
            build_id: "build-1".to_string(),
            progress: 0.5,
            message: "halfway".to_string(),
        },
        LuxEvent::BuildComplete {
            build_id: "build-1".to_string(),
            success: true,
            artifact_path: Some("/tmp/game.apk".to_string()),
        },
        LuxEvent::PlayEvent {
            session_id: "play-1".to_string(),
            event: json!({"kind": "jump"}),
        },
        LuxEvent::PlayFeedback {
            session_id: "play-1".to_string(),
            feedback: json!({"quality": "good"}),
        },
        LuxEvent::AiMessage {
            session_id: "ai-1".to_string(),
            message: "done".to_string(),
            phase: 2,
        },
        LuxEvent::AiRequestInput {
            session_id: "ai-1".to_string(),
            prompt: "continue?".to_string(),
            phase: 3,
        },
        LuxEvent::VerificationResult {
            passed: true,
            details: json!({"checks": 4}),
        },
    ]
}

#[test]
fn lux_events_test_all_event_types_serialize_deserialize() {
    let router = EventRouter::new();

    for event in sample_events() {
        let serialized = router.serialize(&event).unwrap();
        let deserialized = router.deserialize(&serialized).unwrap();
        assert_eq!(deserialized, event);
    }
}

#[test]
fn lux_events_test_event_tagged_serialization() {
    let event = LuxEvent::AiMessage {
        session_id: "ai-2".to_string(),
        message: "hi".to_string(),
        phase: 1,
    };

    let value: Value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["type"], "ai:message");
    assert_eq!(value["data"]["session_id"], "ai-2");
}

#[test]
fn lux_events_test_event_router_dispatch() {
    let mut router = EventRouter::new();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&hits);

    router.register(
        "terminal:output",
        Box::new(move |event| {
            captured
                .lock()
                .unwrap()
                .push(event.event_type().to_string());
        }),
    );

    router.route(&LuxEvent::TerminalOutput {
        session_id: "sess-9".to_string(),
        data: "ok".to_string(),
    });

    assert_eq!(hits.lock().unwrap().as_slice(), &["terminal:output"]);
}

#[test]
fn lux_events_test_event_router_multiple_handlers() {
    let mut router = EventRouter::new();
    let count = Arc::new(Mutex::new(0usize));

    for _ in 0..2 {
        let count = Arc::clone(&count);
        router.register(
            "build:complete",
            Box::new(move |_| {
                *count.lock().unwrap() += 1;
            }),
        );
    }

    router.route(&LuxEvent::BuildComplete {
        build_id: "build-9".to_string(),
        success: false,
        artifact_path: None,
    });

    assert_eq!(*count.lock().unwrap(), 2);
}

#[test]
fn lux_events_test_backward_compatibility() {
    let envelope = EventEnvelope::schema_example();
    let serialized = serde_json::to_value(&envelope).unwrap();
    assert!(serialized.get("type").is_none());
    assert_eq!(serialized["schema_version"], 1);
    assert_eq!(serialized["event_id"], "example-event");
}

#[test]
fn lux_events_test_event_message_wrapper() {
    let message = LuxEventMessage {
        event: LuxEvent::VerificationResult {
            passed: false,
            details: json!({"reason": "missing"}),
        },
        timestamp: "2026-05-11T00:00:00Z".to_string(),
        source: "server".to_string(),
    };

    let serialized = serde_json::to_value(&message).unwrap();
    assert_eq!(serialized["timestamp"], "2026-05-11T00:00:00Z");
    assert_eq!(serialized["source"], "server");
}
