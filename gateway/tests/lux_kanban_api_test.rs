use std::{fs, path::PathBuf};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use lux::{
    addon_auth::AddonAuthConfig,
    lux_ticket::{Ticket, TicketStatus},
    server::{router, GatewayConfig, GatewayState},
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN: &str = "lux-kanban-api-token";

struct TempProject {
    path: PathBuf,
}

impl TempProject {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("lux-kanban-api-{}", uuid::Uuid::new_v4()));
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
            github_client_id: "lux-kanban-api-client".to_string(),
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

async fn create_ticket(state: &GatewayState, project: &TempProject, title: &str) -> Ticket {
    let response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            "/api/lux/kanban/tickets",
            json!({
                "project_path": project.path,
                "title": title,
                "description": "Do the thing",
                "priority": "High",
                "tags": ["ui"],
                "spec_ref": ".lux/spec/gameplay"
            }),
        ))
        .await
        .expect("create ticket response");
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_value(response_json(response).await).expect("created ticket")
}

#[tokio::test]
async fn test_create_ticket_api() {
    let project = TempProject::new();
    let state = test_state();

    let ticket = create_ticket(&state, &project, "Feature A").await;

    assert_eq!(ticket.title, "Feature A");
    assert_eq!(ticket.status, TicketStatus::Backlog);
    assert!(project
        .path
        .join(".lux/tickets")
        .join(format!("{}.json", ticket.id))
        .exists());
}

#[tokio::test]
async fn test_list_tickets_api() {
    let project = TempProject::new();
    let state = test_state();
    let ticket = create_ticket(&state, &project, "Feature A").await;

    let response = router(state.clone())
        .oneshot(get_request(&format!(
            "/api/lux/kanban/tickets?project_path={}&status=Backlog&tag=ui",
            project.path.display()
        )))
        .await
        .expect("list tickets response");

    assert_eq!(response.status(), StatusCode::OK);
    let tickets: Vec<Ticket> =
        serde_json::from_value(response_json(response).await).expect("tickets");
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].id, ticket.id);
}

#[tokio::test]
async fn test_update_ticket_status_valid() {
    let project = TempProject::new();
    let state = test_state();
    let ticket = create_ticket(&state, &project, "Feature A").await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/lux/kanban/tickets/{}/status", ticket.id),
            json!({ "project_path": project.path, "new_status": "ToDo" }),
        ))
        .await
        .expect("update status response");

    assert_eq!(response.status(), StatusCode::OK);
    let updated: Ticket =
        serde_json::from_value(response_json(response).await).expect("updated ticket");
    assert_eq!(updated.status, TicketStatus::ToDo);
}

#[tokio::test]
async fn test_update_ticket_status_invalid() {
    let project = TempProject::new();
    let state = test_state();
    let ticket = create_ticket(&state, &project, "Feature A").await;

    let response = router(state.clone())
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/lux/kanban/tickets/{}/status", ticket.id),
            json!({ "project_path": project.path, "new_status": "InProgress" }),
        ))
        .await
        .expect("update status response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"]
        .as_str()
        .expect("error string")
        .contains("transition denied"));
}

#[tokio::test]
async fn test_blocker_crud() {
    let project = TempProject::new();
    let state = test_state();
    let blocker = create_ticket(&state, &project, "Blocking Feature").await;
    let target = create_ticket(&state, &project, "Dependent Feature").await;

    let add_response = router(state.clone())
        .oneshot(json_request(
            Method::POST,
            &format!("/api/lux/kanban/tickets/{}/blockers", target.id),
            json!({ "project_path": project.path, "blocker_ticket_id": blocker.id }),
        ))
        .await
        .expect("add blocker response");
    assert_eq!(add_response.status(), StatusCode::OK);
    let blockers: Vec<Ticket> =
        serde_json::from_value(response_json(add_response).await).expect("blockers");
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].id, blocker.id);

    let get_response = router(state.clone())
        .oneshot(get_request(&format!(
            "/api/lux/kanban/tickets/{}/blockers?project_path={}",
            target.id,
            project.path.display()
        )))
        .await
        .expect("get blockers response");
    assert_eq!(get_response.status(), StatusCode::OK);
    let blockers: Vec<Ticket> =
        serde_json::from_value(response_json(get_response).await).expect("blockers");
    assert_eq!(blockers.len(), 1);

    let remove_response = router(state.clone())
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/lux/kanban/tickets/{}/blockers", target.id),
            json!({ "project_path": project.path, "blocker_ticket_id": blocker.id }),
        ))
        .await
        .expect("remove blocker response");
    assert_eq!(remove_response.status(), StatusCode::OK);
    let blockers: Vec<Ticket> =
        serde_json::from_value(response_json(remove_response).await).expect("blockers");
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn test_kanban_board_api() {
    let project = TempProject::new();
    let state = test_state();
    let ticket = create_ticket(&state, &project, "Feature A").await;

    let response = router(state.clone())
        .oneshot(get_request(&format!(
            "/api/lux/kanban/board?project_path={}",
            project.path.display()
        )))
        .await
        .expect("board response");

    assert_eq!(response.status(), StatusCode::OK);
    let board = response_json(response).await;
    assert_eq!(board["backlog"].as_array().expect("backlog").len(), 1);
    assert_eq!(board["backlog"][0]["id"], ticket.id);
    assert!(board["blocked"].as_array().expect("blocked").is_empty());
    assert!(board["to_do"].as_array().expect("to_do").is_empty());
    assert!(board["in_progress"]
        .as_array()
        .expect("in_progress")
        .is_empty());
    assert!(board["done"].as_array().expect("done").is_empty());
}
