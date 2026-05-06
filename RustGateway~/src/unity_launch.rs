use std::path::PathBuf;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::server::GatewayState;

#[derive(Clone, Debug, Serialize)]
pub struct UnityProcessInfo {
    pub pid: u32,
    pub executable: String,
    pub project_path: String,
    pub started_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityLaunchRequest {
    pub project_path: Option<String>,
    pub no_wait: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UnityLaunchResponse {
    pub status: String,
    pub pid: Option<u32>,
    pub executable: Option<String>,
    pub project_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UnityStatusResponse {
    pub running: bool,
    pub pid: Option<u32>,
    pub executable: Option<String>,
    pub project_path: Option<String>,
    pub started_at: Option<String>,
    pub unity_version: Option<String>,
}

pub fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/launch", post(unity_launch))
        .route("/status", get(unity_status))
        .route("/version", get(unity_version))
}

fn require_token(
    state: &GatewayState,
    headers: &HeaderMap,
) -> Result<(), Response> {
    let token = headers
        .get("x-lux-token")
        .and_then(|value| value.to_str().ok());

    if state.accepts_token(token) {
        Ok(())
    } else {
        Err(
            (
                StatusCode::UNAUTHORIZED,
                "invalid or missing Lux gateway token",
            )
                .into_response(),
        )
    }
}

async fn unity_launch(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<UnityLaunchRequest>,
) -> Result<Json<UnityLaunchResponse>, Response> {
    require_token(&state, &headers)?;

    let project_root = resolve_project_root_from_state(&state, body.project_path.as_deref())?;

    let launch_target = crate::resolve_unity_launch_target(&project_root).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("cannot resolve Unity editor: {e}"),
        )
            .into_response()
    })?;

    let exe_str = launch_target.executable.display().to_string();
    let project_str = project_root.display().to_string();

    let mut cmd = std::process::Command::new(&launch_target.executable);
    cmd.args(&launch_target.prefix_args)
        .arg("-projectPath")
        .arg(&project_root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let child = cmd.spawn().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to launch Unity: {e}"),
        )
            .into_response()
    })?;

    let pid = child.id();
    let now = chrono_like_now();

    let info = UnityProcessInfo {
        pid,
        executable: exe_str.clone(),
        project_path: project_str.clone(),
        started_at: now,
    };

    {
        let mut lock = state.unity_process.lock().await;
        *lock = Some(info);
    }

    let response = UnityLaunchResponse {
        status: "launched".to_string(),
        pid: Some(pid),
        executable: Some(exe_str),
        project_path: Some(project_str),
        message: "Unity editor process spawned successfully".to_string(),
    };

    Ok(Json(response))
}

async fn unity_status(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<UnityStatusResponse>, Response> {
    require_token(&state, &headers)?;

    let lock = state.unity_process.lock().await;
    let unity_version = state
        .config
        .project_root
        .as_ref()
        .and_then(|root| crate::read_unity_editor_version(root).ok());

    if let Some(ref info) = *lock {
        Ok(Json(UnityStatusResponse {
            running: true,
            pid: Some(info.pid),
            executable: Some(info.executable.clone()),
            project_path: Some(info.project_path.clone()),
            started_at: Some(info.started_at.clone()),
            unity_version,
        }))
    } else {
        Ok(Json(UnityStatusResponse {
            running: false,
            pid: None,
            executable: None,
            project_path: None,
            started_at: None,
            unity_version,
        }))
    }
}

async fn unity_version(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, Response> {
    require_token(&state, &headers)?;

    let project_root = state.config.project_root.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "project path is not configured",
        )
            .into_response()
    })?;

    let version =
        crate::read_unity_editor_version(project_root).map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("cannot read Unity version: {e}"),
            )
                .into_response()
        })?;

    Ok(Json(json!({
        "version": version,
        "project_path": project_root.display().to_string(),
    })))
}

fn resolve_project_root_from_state(
    state: &GatewayState,
    override_path: Option<&str>,
) -> Result<PathBuf, Response> {
    if let Some(path) = override_path {
        let root = PathBuf::from(path);
        if root.is_dir() {
            return Ok(root);
        }
        return Err((
            StatusCode::BAD_REQUEST,
            format!("project path does not exist: {path}"),
        )
            .into_response());
    }

    state
        .config
        .project_root
        .clone()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "project path is not configured; start server with --project-path or provide projectPath in the request body",
            )
                .into_response()
        })
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_launch_request_deserializes_with_defaults() {
        let req: UnityLaunchRequest =
            serde_json::from_str("{}").expect("empty object should parse");
        assert!(req.project_path.is_none());
        assert!(req.no_wait.is_none());
    }

    #[test]
    fn unity_launch_request_deserializes_with_values() {
        let req: UnityLaunchRequest = serde_json::from_str(
            r#"{"projectPath": "/tmp/myproject", "noWait": true}"#,
        )
        .expect("full object should parse");
        assert_eq!(req.project_path.as_deref(), Some("/tmp/myproject"));
        assert_eq!(req.no_wait, Some(true));
    }

    #[test]
    fn chrono_like_now_returns_unix_prefix() {
        let now = chrono_like_now();
        assert!(now.starts_with("unix:"));
        let seconds: u64 = now.strip_prefix("unix:").unwrap().parse().unwrap();
        assert!(seconds > 1700000000);
    }
}
