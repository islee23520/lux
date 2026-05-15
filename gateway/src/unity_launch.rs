use std::{
    fs,
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{config, server::GatewayState};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_path: Option<String>,
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

#[derive(Debug, serde::Deserialize)]
struct UnityBridgeDiscovery {
    host: String,
    port: u16,
    token: String,
}

#[derive(Debug)]
pub struct UnityBridgeBackendPing {
    pub host: String,
    pub port: u16,
    pub discovery_path: PathBuf,
    pub ping: Value,
}

pub struct UnityLaunchTarget {
    pub executable: PathBuf,
    pub prefix_args: Vec<String>,
}

pub fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/launch", post(unity_launch))
        .route("/status", get(unity_status))
        .route("/version", get(unity_version))
}

fn require_token(state: &GatewayState, headers: &HeaderMap) -> Result<(), Response> {
    let token = headers
        .get("x-lux-token")
        .and_then(|value| value.to_str().ok());

    if state.accepts_token(token) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "invalid or missing Lux gateway token",
        )
            .into_response())
    }
}

async fn unity_launch(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<UnityLaunchRequest>,
) -> Result<Json<UnityLaunchResponse>, Response> {
    require_token(&state, &headers)?;

    let project_root = resolve_project_root_from_state(&state, body.project_path.as_deref())?;
    let project_str = project_root.display().to_string();

    if let Ok(backend) = try_ping_unity_bridge_backend(&project_root, Duration::from_secs(1)) {
        return Ok(Json(UnityLaunchResponse {
            status: "already_running".to_string(),
            pid: None,
            executable: None,
            project_path: Some(project_str),
            bridge_ready: Some(true),
            backend_host: Some(backend.host),
            backend_port: Some(backend.port),
            discovery_path: Some(backend.discovery_path.display().to_string()),
            message: "Unity editor is already running with a reachable Lux backend; launch skipped"
                .to_string(),
        }));
    }

    let launch_target = resolve_unity_launch_target(&project_root).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("cannot resolve Unity editor: {e}"),
        )
            .into_response()
    })?;

    let exe_str = launch_target.executable.display().to_string();

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
        bridge_ready: Some(false),
        backend_host: None,
        backend_port: None,
        discovery_path: None,
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
        .and_then(|root| read_unity_editor_version(root).ok());

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

    let version = read_unity_editor_version(project_root).map_err(|e| {
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

fn read_unity_bridge_discovery(project_root: &Path) -> anyhow::Result<UnityBridgeDiscovery> {
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let text = fs::read_to_string(&discovery_path).with_context(|| {
        format!(
            "Unity AI Bridge discovery file not found at {}",
            discovery_path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse Unity AI Bridge discovery file at {}",
            discovery_path.display()
        )
    })
}

pub fn try_ping_unity_bridge_backend(
    project_root: &Path,
    timeout: Duration,
) -> anyhow::Result<UnityBridgeBackendPing> {
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let discovery = read_unity_bridge_discovery(project_root)?;
    let ping = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "ping",
        "token": discovery.token,
        "params": {}
    });
    let response_line = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&ping)?),
        timeout,
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true)
        || response_json
            .get("payload")
            .and_then(|payload| payload.get("ping"))
            .and_then(|ping| ping.get("status"))
            .and_then(Value::as_str)
            != Some("ok")
    {
        bail!("Unity TCP ping was not ready: {}", response_json);
    }

    Ok(UnityBridgeBackendPing {
        host: discovery.host,
        port: discovery.port,
        discovery_path,
        ping: response_json,
    })
}

fn send_unity_tcp_line_with_timeout(
    discovery: &UnityBridgeDiscovery,
    request_line: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let deadline = Instant::now() + timeout;
    let mut stream = connect_unity_tcp_with_retry(discovery, deadline)?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    write_unity_tcp_with_retry(&mut stream, request_line.as_bytes(), deadline)?;

    let mut buffer = String::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let size = match stream.read(&mut chunk) {
            Ok(size) => size,
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(error) => return Err(error).context("Unity TCP response read failed"),
        };
        if size == 0 {
            break;
        }
        buffer.push_str(
            std::str::from_utf8(&chunk[..size]).context("Unity TCP response was not UTF-8")?,
        );
        if let Some(index) = buffer.find('\n') {
            return Ok(buffer[..index].to_string());
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for Unity TCP response");
        }
    }

    bail!("Unity TCP connection closed before sending a response")
}

fn connect_unity_tcp_with_retry(
    discovery: &UnityBridgeDiscovery,
    deadline: Instant,
) -> anyhow::Result<std::net::TcpStream> {
    loop {
        match std::net::TcpStream::connect((discovery.host.as_str(), discovery.port)) {
            Ok(stream) => return Ok(stream),
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to connect to Unity AI Bridge at {}:{}",
                        discovery.host, discovery.port
                    )
                });
            }
        }
    }
}

fn write_unity_tcp_with_retry(
    stream: &mut std::net::TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> anyhow::Result<()> {
    while !bytes.is_empty() {
        match stream.write(bytes) {
            Ok(0) => bail!("Unity TCP connection closed while writing request"),
            Ok(size) => bytes = &bytes[size..],
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error).context("Unity TCP request write failed"),
        }
    }

    stream.flush().context("Unity TCP request flush failed")
}

fn is_transient_socket_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::Interrupted | ErrorKind::TimedOut
    )
}

pub fn resolve_unity_launch_target(project_root: &Path) -> anyhow::Result<UnityLaunchTarget> {
    let config = config::load()?;
    if let Some(editor) = config.unity.editor_path.as_ref() {
        return Ok(UnityLaunchTarget {
            executable: editor.clone(),
            prefix_args: Vec::new(),
        });
    }

    if let Some(editor) = std::env::var_os("LUX_UNITY_EDITOR") {
        return Ok(UnityLaunchTarget {
            executable: PathBuf::from(editor),
            prefix_args: Vec::new(),
        });
    }

    let version = read_unity_editor_version(project_root)?;

    #[cfg(target_os = "macos")]
    {
        let hub_editor = PathBuf::from(format!(
            "/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity"
        ));
        if hub_editor.is_file() {
            return Ok(UnityLaunchTarget {
                executable: hub_editor,
                prefix_args: Vec::new(),
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();

        if let Some(hub_path) = config.unity.hub_path.as_ref() {
            candidates.push(
                crate::unity_hub::editor_install_path_for_hub(hub_path)
                    .join(&version)
                    .join("Editor")
                    .join("Unity.exe"),
            );
        }

        if let Some(install_path) = config.unity.custom_install_path.as_ref() {
            candidates.push(install_path.join(&version).join("Editor").join("Unity.exe"));
        }

        if let Some(hub_path) = std::env::var_os("LUX_UNITY_HUB_PATH") {
            candidates.push(
                PathBuf::from(hub_path)
                    .join("Editor")
                    .join(&version)
                    .join("Editor")
                    .join("Unity.exe"),
            );
        }

        candidates.push(PathBuf::from(format!(
            "C:\\Program Files\\Unity\\Hub\\Editor\\{version}\\Editor\\Unity.exe"
        )));
        candidates.push(PathBuf::from(format!(
            "C:\\Program Files\\Unity Hub\\Editor\\{version}\\Editor\\Unity.exe"
        )));

        for hub_editor in candidates {
            if hub_editor.is_file() {
                return Ok(UnityLaunchTarget {
                    executable: hub_editor,
                    prefix_args: Vec::new(),
                });
            }
        }

        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(unity_editor_key) =
            current_user.open_subkey("Software\\Unity Technologies\\Unity Editor 5.x")
        {
            let value_name = format!("{version}_Location_x64");
            if let Ok(editor_path) = unity_editor_key.get_value::<String, _>(&value_name) {
                let editor_path = PathBuf::from(editor_path);
                if editor_path.is_file() {
                    return Ok(UnityLaunchTarget {
                        executable: editor_path,
                        prefix_args: Vec::new(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let hub_editor = PathBuf::from(format!("/opt/Unity/Hub/Editor/{version}/Editor/Unity"));
        if hub_editor.is_file() {
            return Ok(UnityLaunchTarget {
                executable: hub_editor,
                prefix_args: Vec::new(),
            });
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home_editor =
                PathBuf::from(home).join(format!("Unity/Hub/Editor/{version}/Editor/Unity"));
            if home_editor.is_file() {
                return Ok(UnityLaunchTarget {
                    executable: home_editor,
                    prefix_args: Vec::new(),
                });
            }
        }
    }

    bail!(
        "Unity Editor {version} not found in standard Hub locations. \
         Set LUX_UNITY_EDITOR to the Unity executable path."
    )
}

pub fn read_unity_editor_version(project_root: &Path) -> anyhow::Result<String> {
    let version_path = project_root
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    let text = fs::read_to_string(&version_path)
        .with_context(|| format!("failed to read {}", version_path.display()))?;
    text.lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion:"))
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .context("ProjectSettings/ProjectVersion.txt did not contain m_EditorVersion")
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
        let req: UnityLaunchRequest =
            serde_json::from_str(r#"{"projectPath": "/tmp/myproject", "noWait": true}"#)
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
