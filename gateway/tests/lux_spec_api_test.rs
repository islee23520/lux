use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use lux::{
    addon_auth::AddonAuthConfig,
    lux_spec::{DomainSpec, DomainStatus, Requirement, RequirementStatus, SpecProject, SpecStatus},
    server::{router, GatewayConfig, GatewayState},
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN: &str = "lux-spec-api-token";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    path: PathBuf,
}

impl TempProject {
    fn new() -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lux-spec-api-{nonce}-{count}"));
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

#[tokio::test]
async fn test_lux_progress_summary_api() {
    let project = TempProject::new();
    let state = test_state();
    let mut spec = init_project(&state, &project).await;
    spec.overall_ambiguity = 0.45;
    let mut design = DomainSpec::new("design", "design.md", 0.3);
    design.status = DomainStatus::Defined;
    let mut implemented = Requirement::default();
    implemented.id = "req-1".to_string();
    implemented.text = "Implemented requirement".to_string();
    implemented.status = RequirementStatus::Implemented;
    let mut proposed = Requirement::default();
    proposed.id = "req-2".to_string();
    proposed.text = "Proposed requirement".to_string();
    design.requirements = vec![implemented, proposed];
    spec.domains.design = Some(design);
    lux::lux_spec::lux_save(&project.path, &spec).expect("save spec");

    let ticket_response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/kanban/tickets",
            json!({
                "project_path": project.path,
                "title": "Feature A",
                "description": "Do the thing",
                "priority": "High",
                "tags": [],
                "spec_ref": null
            }),
        ))
        .await
        .expect("create ticket response");
    assert_eq!(ticket_response.status(), StatusCode::CREATED);

    let response = router(state)
        .oneshot(get_request(&format!(
            "/api/lux/progress/summary?project_path={}",
            project.path.display()
        )))
        .await
        .expect("progress summary response");

    assert_eq!(response.status(), StatusCode::OK);
    let summary = response_json(response).await;
    assert_eq!(summary["spec"]["overall_ambiguity"], 0.45);
    assert_eq!(summary["spec"]["domains"]["design"]["ambiguity"], 0.3);
    assert_eq!(summary["spec"]["domains"]["design"]["status"], "Defined");
    assert_eq!(
        summary["spec"]["domains"]["design"]["requirements_total"],
        2
    );
    assert_eq!(summary["spec"]["domains"]["design"]["requirements_done"], 1);
    assert_eq!(summary["kanban"]["by_status"]["Backlog"], 1);
    assert_eq!(summary["kanban"]["total"], 1);
    assert_eq!(summary["kanban"]["active_count"], 0);
    assert_eq!(summary["loop"]["state"], "Idle");
    assert!(summary["loop"]["iteration"].is_null());
}

#[tokio::test]
async fn test_lux_progress_summary_uninitialized_returns_empty_not_found() {
    let project = TempProject::new();
    let state = test_state();

    let response = router(state)
        .oneshot(get_request(&format!(
            "/api/lux/progress/summary?project_path={}",
            project.path.display()
        )))
        .await
        .expect("progress summary response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let summary = response_json(response).await;
    assert_eq!(summary["spec"]["overall_ambiguity"], 1.0);
    assert!(summary["spec"]["domains"].as_object().unwrap().is_empty());
    assert_eq!(summary["kanban"]["total"], 0);
    assert_eq!(summary["kanban"]["active_count"], 0);
    assert_eq!(summary["kanban"]["by_status"]["Backlog"], 0);
    assert_eq!(summary["loop"]["state"], "Idle");
    assert!(summary["loop"]["iteration"].is_null());
}
