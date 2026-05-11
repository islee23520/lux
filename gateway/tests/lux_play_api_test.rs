use std::{fs, path::PathBuf};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use lux::{
    addon_auth::AddonAuthConfig,
    server::{router, GatewayConfig, GatewayState},
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN: &str = "lux-play-api-token";

struct TempProject {
    path: PathBuf,
}

impl TempProject {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("lux-play-api-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        Self { path }
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn test_state(project: Option<&TempProject>) -> GatewayState {
    GatewayState::new(GatewayConfig {
        token: TOKEN.to_string(),
        history_capacity: 16,
        project_root: project.map(|project| project.path.clone()),
        addon_auth: AddonAuthConfig {
            github_client_id: "lux-play-api-client".to_string(),
            github_client_secret: None,
        },
    })
}

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .expect("build request")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response json")
}

fn event(session_id: &str, event_type: &str, timestamp: &str, sequence: u64) -> Value {
    json!({
        "session_id": session_id,
        "timestamp": timestamp,
        "event_type": event_type,
        "payload": { "sequence": sequence },
        "player_id": "player-1",
        "game_state": { "hp": 100 - sequence },
        "sequence": sequence
    })
}

async fn start_session(state: &GatewayState, project: &TempProject) -> String {
    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/session/start",
            json!({
                "project_path": project.path,
                "player_id": "player-1",
                "webgl_build_version": "1.2.3",
                "metadata": { "difficulty": "normal" }
            }),
        ))
        .await
        .expect("start session response");
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["session_id"]
        .as_str()
        .expect("session_id")
        .to_string()
}

#[tokio::test]
async fn test_start_play_session() {
    let project = TempProject::new();
    let state = test_state(None);

    let session_id = start_session(&state, &project).await;

    assert!(project
        .path
        .join(format!(".lux/logs/{session_id}.meta.json"))
        .exists());
}

#[tokio::test]
async fn test_post_play_event() {
    let project = TempProject::new();
    let state = test_state(Some(&project));
    let session_id = start_session(&state, &project).await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/event",
            event(&session_id, "Action", "2026-05-11T10:00:00Z", 1),
        ))
        .await
        .expect("post event response");

    assert_eq!(response.status(), StatusCode::OK);
    let log = fs::read_to_string(project.path.join(format!(".lux/logs/{session_id}.jsonl")))
        .expect("event log");
    assert_eq!(log.lines().count(), 1);
}

#[tokio::test]
async fn test_post_play_events_batch() {
    let project = TempProject::new();
    let state = test_state(Some(&project));
    let session_id = start_session(&state, &project).await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/events/batch",
            json!([
                event(&session_id, "Action", "2026-05-11T10:00:00Z", 1),
                event(&session_id, "Decision", "2026-05-11T10:00:01Z", 2)
            ]),
        ))
        .await
        .expect("post batch response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["count"], 2);
    let log = fs::read_to_string(project.path.join(format!(".lux/logs/{session_id}.jsonl")))
        .expect("event log");
    assert_eq!(log.lines().count(), 2);
}

#[tokio::test]
async fn test_end_play_session() {
    let project = TempProject::new();
    let state = test_state(Some(&project));
    let session_id = start_session(&state, &project).await;
    router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/event",
            event(&session_id, "LevelStart", "2026-05-11T10:00:00Z", 1),
        ))
        .await
        .expect("post event response");

    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/session/end",
            json!({ "project_path": project.path, "session_id": session_id }),
        ))
        .await
        .expect("end session response");

    assert_eq!(response.status(), StatusCode::OK);
    let meta = response_json(response).await;
    assert_eq!(meta["event_count"], 1);
    assert!(meta["duration_secs"].as_f64().expect("duration") >= 0.0);
    assert!(meta["ended_at"].is_string());
}

#[tokio::test]
async fn test_list_play_sessions() {
    let project = TempProject::new();
    let state = test_state(None);
    let session_id = start_session(&state, &project).await;

    let response = router(state.clone())
        .oneshot(get_request(&format!(
            "/api/lux/play/sessions?project_path={}",
            project.path.display()
        )))
        .await
        .expect("list sessions response");

    assert_eq!(response.status(), StatusCode::OK);
    let sessions = response_json(response).await;
    assert_eq!(sessions.as_array().expect("sessions").len(), 1);
    assert_eq!(sessions[0]["session_id"], session_id);
}

#[tokio::test]
async fn test_get_session_events() {
    let project = TempProject::new();
    let state = test_state(Some(&project));
    let session_id = start_session(&state, &project).await;
    router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/events/batch",
            json!([
                event(&session_id, "Action", "2026-05-11T10:00:00Z", 1),
                event(&session_id, "Decision", "2026-05-11T10:00:01Z", 2),
                event(&session_id, "Decision", "2026-05-11T10:00:02Z", 3)
            ]),
        ))
        .await
        .expect("post batch response");

    let response = router(state.clone())
        .oneshot(get_request(&format!(
            "/api/lux/play/sessions/{session_id}/events?project_path={}&event_type=Decision&limit=1",
            project.path.display()
        )))
        .await
        .expect("get events response");

    assert_eq!(response.status(), StatusCode::OK);
    let events = response_json(response).await;
    assert_eq!(events.as_array().expect("events").len(), 1);
    assert_eq!(events[0]["event_type"], "Decision");
}

#[tokio::test]
async fn test_play_feedback() {
    let project = TempProject::new();
    let state = test_state(None);
    let session_id = start_session(&state, &project).await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/play/feedback",
            json!({
                "project_path": project.path,
                "session_id": session_id,
                "rating": 4,
                "text": "felt good",
                "issues": ["camera"]
            }),
        ))
        .await
        .expect("feedback response");

    assert_eq!(response.status(), StatusCode::CREATED);
    let feedback_path = project
        .path
        .join(format!(".lux/logs/{session_id}.feedback.json"));
    let saved: Value = serde_json::from_str(&fs::read_to_string(feedback_path).expect("feedback"))
        .expect("feedback json");
    assert_eq!(saved["rating"], 4);
    assert_eq!(saved["issues"][0], "camera");
}
