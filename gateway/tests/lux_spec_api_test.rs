use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use lux::{
    addon_auth::AddonAuthConfig,
    lux_spec::{SpecProject, SpecStatus},
    server::{router, GatewayConfig, GatewayState},
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN: &str = "lux-spec-api-token";

struct TempProject {
    path: PathBuf,
}

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lux-spec-api-{nonce}"));
        fs::create_dir_all(&path).expect("create temp project");
        Self { path }
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn test_state() -> GatewayState {
    GatewayState::new(GatewayConfig {
        token: TOKEN.to_string(),
        history_capacity: 16,
        project_root: None,
        addon_auth: AddonAuthConfig {
            github_client_id: "lux-spec-api-client".to_string(),
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response json")
}

async fn init_project(state: &GatewayState, project: &TempProject) -> SpecProject {
    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/init",
            json!({ "project_path": project.path }),
        ))
        .await
        .expect("init response");
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_value(response_json(response).await).expect("init spec")
}

#[tokio::test]
async fn test_lux_init_api() {
    let project = TempProject::new();
    let state = test_state();

    let spec = init_project(&state, &project).await;

    assert_eq!(
        spec.project_name,
        project.path.file_name().unwrap().to_string_lossy()
    );
    assert!(project.path.join(".lux/spec.json").exists());
}

#[tokio::test]
async fn test_lux_get_spec_api() {
    let project = TempProject::new();
    let state = test_state();
    let initialized = init_project(&state, &project).await;

    let response = router(state)
        .oneshot(json_request(
            Method::GET,
            &format!("/api/lux/spec?project_path={}", project.path.display()),
            json!(null),
        ))
        .await
        .expect("get spec response");

    assert_eq!(response.status(), StatusCode::OK);
    let spec: SpecProject =
        serde_json::from_value(response_json(response).await).expect("spec json");
    assert_eq!(spec.project_id, initialized.project_id);
}

#[tokio::test]
async fn test_lux_put_spec_api() {
    let project = TempProject::new();
    let state = test_state();
    let mut spec = init_project(&state, &project).await;
    spec.status = SpecStatus::Active;

    let response = router(state)
        .oneshot(json_request(
            Method::PUT,
            "/api/lux/spec",
            json!({ "project_path": project.path, "spec": spec }),
        ))
        .await
        .expect("put spec response");

    assert_eq!(response.status(), StatusCode::OK);
    let updated: SpecProject =
        serde_json::from_value(response_json(response).await).expect("updated spec");
    assert_eq!(updated.status, SpecStatus::Active);
}

#[tokio::test]
async fn test_lux_get_domain_api() {
    let project = TempProject::new();
    let state = test_state();
    init_project(&state, &project).await;

    let response = router(state)
        .oneshot(json_request(
            Method::GET,
            &format!(
                "/api/lux/spec/design?project_path={}",
                project.path.display()
            ),
            json!(null),
        ))
        .await
        .expect("get domain response");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["domain"], "design");
    assert!(json["content"]
        .as_str()
        .unwrap_or_default()
        .contains("Design"));
}

#[tokio::test]
async fn test_lux_put_domain_api() {
    let project = TempProject::new();
    let state = test_state();
    init_project(&state, &project).await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::PUT,
            "/api/lux/spec/design",
            json!({ "project_path": project.path, "content": "# Updated Design\n" }),
        ))
        .await
        .expect("put domain response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(state)
        .oneshot(json_request(
            Method::GET,
            &format!(
                "/api/lux/spec/design?project_path={}",
                project.path.display()
            ),
            json!(null),
        ))
        .await
        .expect("get updated domain response");
    let json = response_json(response).await;
    assert_eq!(json["content"], "# Updated Design\n");
}

#[tokio::test]
async fn test_lux_validate_spec_api() {
    let project = TempProject::new();
    let state = test_state();
    init_project(&state, &project).await;

    let response = router(state)
        .oneshot(json_request(
            Method::POST,
            "/api/lux/spec/validate",
            json!({ "project_path": project.path }),
        ))
        .await
        .expect("validate response");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["valid"], true);
    assert!(json["errors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_lux_ambiguity_api() {
    let project = TempProject::new();
    let state = test_state();
    init_project(&state, &project).await;

    let response = router(state)
        .oneshot(json_request(
            Method::GET,
            &format!(
                "/api/lux/spec/ambiguity?project_path={}",
                project.path.display()
            ),
            json!(null),
        ))
        .await
        .expect("ambiguity response");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["overall"], 1.0);
    assert!(json["domains"].as_object().unwrap().contains_key("design"));
}
