mod common;

use std::{fs, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use futures_util::SinkExt;
use lux::{
    addon_auth::AddonAuthConfig,
    capture::CaptureSessionManager,
    protocol::{
        CMD_LUX_INPUT_EVENT, CMD_LUX_STREAM_FRAME, CMD_START_LUX_STREAM, CMD_STOP_LUX_STREAM,
    },
    server::{router, GatewayConfig, GatewayState},
};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "capture-smoke-token";

struct BridgeStub {
    project_path: std::path::PathBuf,
    requests: Arc<Mutex<Vec<Value>>>,
}

impl Drop for BridgeStub {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.project_path);
    }
}

async fn start_bridge_stub() -> BridgeStub {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured_requests = requests.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let captured_requests = captured_requests.clone();
            tokio::spawn(async move {
                let (read_half, mut write_half) = stream.into_split();
                let mut reader = BufReader::new(read_half);
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    return;
                }
                let request: Value = serde_json::from_str(line.trim_end()).unwrap();
                let command = request
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                captured_requests.lock().await.push(request.clone());
                let response = if command == CMD_START_LUX_STREAM {
                    json!({
                        "command": CMD_LUX_STREAM_FRAME,
                        "params": {
                            "sessionId": request["params"]["sessionId"],
                            "frame": "/9j/4AAQSkZJRg==",
                            "sequence": 1_u64,
                            "timestamp": "2026-05-10T00:00:00Z"
                        }
                    })
                } else {
                    json!({ "ok": true })
                };
                let _ = write_half
                    .write_all(
                        format!("{}\n", serde_json::to_string(&response).unwrap()).as_bytes(),
                    )
                    .await;
                let _ = write_half.flush().await;
            });
        }
    });

    let project_path = common::temp_dir_unique("capture-smoke-project");
    let discovery_dir = project_path.join("Library/UnityAiBridge");
    fs::create_dir_all(&discovery_dir).unwrap();
    fs::write(
        discovery_dir.join("server.json"),
        json!({ "host": "127.0.0.1", "port": port, "token": "bridge-token" }).to_string(),
    )
    .unwrap();
    BridgeStub {
        project_path,
        requests,
    }
}

fn test_state(project_root: Option<std::path::PathBuf>) -> GatewayState {
    GatewayState::new(GatewayConfig {
        token: TOKEN.to_string(),
        history_capacity: 16,
        project_root,
        addon_auth: AddonAuthConfig {
            github_client_id: "capture-smoke-client".to_string(),
            github_client_secret: None,
        },
    })
}

fn authed_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-lux-token", TOKEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn create_session(state: &GatewayState, project_path: &std::path::Path) -> String {
    let response = router(state.clone())
        .oneshot(authed_request(
            Method::POST,
            "/api/unity/capture/sessions",
            json!({
                "projectPath": project_path,
                "width": 640,
                "height": 360,
                "fps": 15
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice::<Value>(&bytes).unwrap()["session"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn wait_for_command(requests: &Arc<Mutex<Vec<Value>>>, command: &str) -> Value {
    for _ in 0..250 {
        if let Some(request) = requests
            .lock()
            .await
            .iter()
            .find(|request| request["command"] == command)
            .cloned()
        {
            return request;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("bridge stub did not receive {command}");
}

#[tokio::test]
async fn capture_session_crud_via_http() {
    let bridge = start_bridge_stub().await;
    let state = test_state(None);

    let session_id = create_session(&state, &bridge.project_path).await;
    let created_command = wait_for_command(&bridge.requests, CMD_START_LUX_STREAM).await;
    assert_eq!(created_command["params"]["width"], 640);

    let response = router(state.clone())
        .oneshot(authed_request(
            Method::GET,
            &format!("/api/unity/capture/sessions/{session_id}"),
            json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(state.clone())
        .oneshot(authed_request(
            Method::DELETE,
            &format!("/api/unity/capture/sessions/{session_id}"),
            json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_command(&bridge.requests, CMD_STOP_LUX_STREAM).await;

    let response = router(state)
        .oneshot(authed_request(
            Method::GET,
            &format!("/api/unity/capture/sessions/{session_id}"),
            json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn mjpeg_stream_endpoint_returns_multipart_content_type() {
    let bridge = start_bridge_stub().await;
    let state = test_state(None);
    let session_id = create_session(&state, &bridge.project_path).await;

    let response = router(state)
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/unity/capture/sessions/{session_id}/stream"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "multipart/x-mixed-replace; boundary=FRAME_BOUNDARY"
    );
}

#[tokio::test]
async fn input_websocket_accepts_json_messages() {
    let bridge = start_bridge_stub().await;
    let state = test_state(None);
    let session_id = create_session(&state, &bridge.project_path).await;
    let app = router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let (mut socket, response) = tokio_tungstenite::connect_async(format!(
        "ws://{address}/api/unity/capture/sessions/{session_id}/input"
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({
                "sessionId": "client-supplied",
                "type": "mouseMove",
                "x": 24.0,
                "y": 48.0
            })
            .to_string(),
        ))
        .await
        .unwrap();

    let command = wait_for_command(&bridge.requests, CMD_LUX_INPUT_EVENT).await;
    let forwarded_session_id = command["params"]["sessionId"]
        .as_str()
        .expect("forwarded session id");
    assert!(Uuid::parse_str(forwarded_session_id).is_ok());
    assert_eq!(command["params"]["type"], "mouseMove");
}

#[tokio::test]
async fn session_not_found_returns_404() {
    let response = router(test_state(None))
        .oneshot(authed_request(
            Method::GET,
            "/api/unity/capture/sessions/missing-session",
            json!(null),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_session_without_project_path_returns_400() {
    let response = router(test_state(None))
        .oneshot(authed_request(
            Method::POST,
            "/api/unity/capture/sessions",
            json!({ "width": 320, "height": 240, "fps": 10 }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mjpeg_stream_missing_session_returns_404() {
    let response = router(test_state(None))
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/unity/capture/sessions/missing-session/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn capture_manager_default_is_empty() {
    let manager = CaptureSessionManager::default();
    assert!(manager.get_session("missing").await.is_none());
}
