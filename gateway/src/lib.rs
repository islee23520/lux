extern crate self as lux;

pub mod addon_auth;
pub mod addon_routes;
pub mod addon_store;
pub mod ai_log;
pub mod auto_update;
pub mod capture;
pub mod config;
pub mod cross_platform;
pub mod lux_agents_install;
pub mod lux_ai_session;
pub mod lux_ambiguity;
pub mod lux_api;
pub mod lux_bridge_lease;
pub mod lux_build;
pub mod lux_continuation_state;
pub mod lux_doctor;
pub mod lux_event_log;
pub mod lux_events;
pub mod lux_io;
pub mod lux_lock;
pub mod lux_loop;
pub mod lux_metrics;
pub mod lux_roadmap;
pub mod lux_run;
pub mod lux_run_recover;
pub mod lux_run_state;
pub mod lux_spec;
pub mod lux_spec_loop;
pub mod lux_task_dag;
pub mod lux_team_profile;
pub mod lux_terminal;
pub mod lux_ticket;
pub mod lux_ticket_executor;
pub mod lux_triage;
pub mod lux_unity_maneuver;
pub mod lux_verification;
pub mod lux_worktree;
pub mod project;
pub mod protocol;
pub mod server;
pub mod session;
pub mod skill_adapter;
pub mod uloop_runner;
pub mod uloop_sync;
pub mod unity_hub;
pub mod unity_launch;
pub mod visual_regression;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context};
use serde_json::{json, Value};

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

fn send_unity_tcp_line_with_timeout(
    discovery: &UnityBridgeDiscovery,
    request_line: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let address = format!("{}:{}", discovery.host, discovery.port);
    let socket_addr = address
        .parse()
        .with_context(|| format!("invalid Unity AI Bridge address {address}"))?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, timeout)
        .with_context(|| format!("failed to connect to Unity AI Bridge at {address}"))?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    stream
        .write_all(request_line.as_bytes())
        .context("Unity TCP request write failed")?;
    stream.flush().context("Unity TCP request flush failed")?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .context("Unity TCP response read failed")?;
    if line.trim().is_empty() {
        bail!("Unity TCP response was empty");
    }
    Ok(line.trim_end().to_string())
}
