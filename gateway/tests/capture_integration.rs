mod common;

use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    body::{Body, BodyDataStream},
    http::{header, Method, Request, StatusCode},
};
use futures_util::{SinkExt, StreamExt};
use lux::{
    addon_auth::AddonAuthConfig,
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

const TOKEN: &str = "capture-integration-token";

struct BridgeStub {
    project_path: PathBuf,
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
                            "timestamp": "2026-05-11T00:00:00Z"
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

    let project_path = common::temp_dir_unique("capture-integration-project");
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

fn test_state(project_root: PathBuf) -> GatewayState {
    GatewayState::new(GatewayConfig {
        token: TOKEN.to_string(),
        history_capacity: 16,
        project_root: Some(project_root),
        addon_auth: AddonAuthConfig {
            github_client_id: "capture-integration-client".to_string(),
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

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn wait_for_command(requests: &Arc<Mutex<Vec<Value>>>, command: &str) -> Value {
    for _ in 0..50 {
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

async fn read_first_stream_chunk(mut stream: BodyDataStream) -> Vec<u8> {
    for _ in 0..50 {
        if let Some(chunk) = stream.next().await {
            let bytes = chunk.unwrap();
            if !bytes.is_empty() {
                return bytes.to_vec();
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("MJPEG stream did not produce data");
}

#[tokio::test]
async fn capture_integration_session_stream_input_stop_and_health() {
    let bridge = start_bridge_stub().await;
    let state = test_state(bridge.project_path.clone());

    let create_response = router(state.clone())
        .oneshot(authed_request(
            Method::POST,
            "/api/unity/runs",
            json!({ "width": 1280, "height": 720, "fps": 30 }),
        ))
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = response_json(create_response).await;
    let session = &create_body["session"];
    let session_id = session["id"].as_str().unwrap().to_string();
    assert_eq!(session["width"], 1280);
    assert_eq!(session["height"], 720);
    assert_eq!(session["fps"], 30);
    assert_eq!(session["status"], "streaming");
    let start_command = wait_for_command(&bridge.requests, CMD_START_LUX_STREAM).await;
    assert_eq!(start_command["params"]["sessionId"], session_id);

    let stream_response = router(state.clone())
        .oneshot(get_request(&format!("/api/unity/runs/{session_id}/stream")))
        .await
        .unwrap();
    assert_eq!(stream_response.status(), StatusCode::OK);
    assert_eq!(
        stream_response.headers().get(header::CONTENT_TYPE).unwrap(),
        "multipart/x-mixed-replace; boundary=FRAME_BOUNDARY"
    );
    let stream_chunk =
        read_first_stream_chunk(stream_response.into_body().into_data_stream()).await;
    assert!(stream_chunk.starts_with(b"--FRAME_BOUNDARY\r\n"));
    assert!(stream_chunk
        .windows(b"Content-Type: image/jpeg".len())
        .any(|window| window == b"Content-Type: image/jpeg"));
    assert!(stream_chunk
        .windows(b"Content-Length: 10".len())
        .any(|window| window == b"Content-Length: 10"));
    assert!(stream_chunk.ends_with(b"\r\n"));

    let app = router(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut socket, ws_response) = tokio_tungstenite::connect_async(format!(
        "ws://{address}/api/unity/runs/{session_id}/input"
    ))
    .await
    .unwrap();
    assert_eq!(ws_response.status(), StatusCode::SWITCHING_PROTOCOLS);
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({
                "type": "mouseDown",
                "x": 320.0,
                "y": 180.0,
                "button": 0
            })
            .to_string(),
        ))
        .await
        .unwrap();
    let input_command = wait_for_command(&bridge.requests, CMD_LUX_INPUT_EVENT).await;
    assert_eq!(input_command["params"]["sessionId"], session_id);
    assert_eq!(input_command["params"]["type"], "mouseDown");
    assert_eq!(input_command["params"]["x"], 320.0);
    assert_eq!(input_command["params"]["button"], 0);

    let delete_response = router(state.clone())
        .oneshot(authed_request(
            Method::DELETE,
            &format!("/api/unity/runs/{session_id}"),
            json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_body = response_json(delete_response).await;
    assert_eq!(delete_body["session"]["id"], session_id);
    wait_for_command(&bridge.requests, CMD_STOP_LUX_STREAM).await;

    let health_response = router(state)
        .oneshot(get_request("/api/health"))
        .await
        .unwrap();
    assert_eq!(health_response.status(), StatusCode::OK);
    assert_eq!(response_json(health_response).await["ok"], true);
}
