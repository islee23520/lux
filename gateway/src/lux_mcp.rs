#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    lux_io::atomic_write_json,
    lux_spec,
    lux_ticket::{
        DispatchPolicy, FileTicketStore, Ticket, TicketPriority, TicketStatus, TicketStore,
    },
};

const TOOL_BRIDGE_INSTALL: &str = "lux_bridge_install";
const TOOL_BRIDGE_DIAGNOSTICS: &str = "lux_bridge_diagnostics";
const TOOL_SPEC_WRITE: &str = "lux_game_spec_write";
const TOOL_TICKET_PREPARE: &str = "lux_game_ticket_prepare";
const TOOL_UNITY_MANEUVER: &str = "lux_unity_maneuver";
const TOOL_LOOP_ONCE: &str = "lux_game_dev_loop_once";
const FIRST_LOOP_TICKET_ID: &str = "game-dev-loop-001";

pub fn install_project_mcp_config(project_path: &Path, lux_exe: &Path) -> Result<Value> {
    if !project_path.exists() {
        anyhow::bail!("Project path does not exist: {}", project_path.display());
    }
    if !project_path.is_dir() {
        anyhow::bail!(
            "Project path is not a directory: {}",
            project_path.display()
        );
    }

    let project_path = project_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", project_path.display()))?;
    let config_path = project_path.join(".mcp.json");
    reject_linked_mcp_config(&config_path)?;
    let mut root = match fs::read_to_string(&config_path) {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .with_context(|| format!("failed to parse {}", config_path.display()))?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => json!({}),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    let previous = root.clone();

    let object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!(".mcp.json root must be a JSON object"))?;
    let servers = object
        .entry("mcpServers".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow!(".mcp.json mcpServers field must be a JSON object"))?;

    servers.insert(
        "lux".to_string(),
        json!({
            "command": lux_exe,
            "args": [
                "mcp",
                "--project-path",
                project_path,
            ],
        }),
    );

    let changed = root != previous;
    if changed {
        atomic_write_json(&config_path, &root)?;
    }

    Ok(json!({
        "ok": true,
        "changed": changed,
        "configPath": config_path,
        "serverName": "lux",
        "command": lux_exe,
        "args": ["mcp", "--project-path", project_path],
        "message": if changed { "Lux MCP project config installed" } else { "Lux MCP project config already installed" },
    }))
}

fn reject_linked_mcp_config(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() {
        anyhow::bail!(".mcp.json must not be a symlink: {}", path.display());
    }
    #[cfg(unix)]
    if metadata.nlink() > 1 {
        anyhow::bail!(".mcp.json must not be hardlinked: {}", path.display());
    }
    Ok(())
}

pub fn run_mcp_stdio(project_path: Option<&Path>) -> Result<()> {
    let default_project = match project_path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().context("failed to resolve current directory")?,
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP stdin line")?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                writeln!(
                    stdout,
                    "{}",
                    json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32700,"message":error.to_string()}})
                )?;
                stdout.flush()?;
                continue;
            }
        };
        let response = handle_json_rpc(&default_project, &request);
        if let Some(response) = response {
            writeln!(stdout, "{}", response)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle_json_rpc(default_project: &Path, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "lux", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => handle_tool_call(
            default_project,
            request.get("params").unwrap_or(&Value::Null),
        ),
        "ping" => Ok(json!({})),
        "notifications/initialized" => return None,
        _ => Err(anyhow!("unknown MCP method: {method}")),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32603, "message": error.to_string() }
        }),
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool_definition(
            TOOL_BRIDGE_INSTALL,
            "Install or refresh the Lux Unity bridge.",
        ),
        tool_definition(TOOL_BRIDGE_DIAGNOSTICS, "Run Lux bridge diagnostics."),
        tool_definition(
            TOOL_SPEC_WRITE,
            "Write or import a minimal game spec into .lux/spec.json.",
        ),
        tool_definition(
            TOOL_TICKET_PREPARE,
            "Create or select one safe first-loop game-dev ticket.",
        ),
        tool_definition(
            TOOL_UNITY_MANEUVER,
            "Perform one safe Unity maneuver or return an explicit unavailable result.",
        ),
        tool_definition(
            TOOL_LOOP_ONCE,
            "Run one safe game-development loop and stop.",
        ),
    ]
}

fn tool_definition(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": {
                "project_path": { "type": "string" },
                "project_name": { "type": "string" },
                "objective": { "type": "string" },
                "seed": { "type": "object" },
                "spec_seed": { "type": "object" },
                "validation_policy": { "type": "string" },
                "autonomy_policy": { "type": "string" },
                "ticket_id": { "type": "string" },
                "verification_policy": { "type": "string" },
                "non_goals": { "type": "array", "items": { "type": "string" } }
            },
            "additionalProperties": false
        }
    })
}

fn handle_tool_call(default_project: &Path, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call missing params.name"))?;
    let arguments = params.get("arguments").unwrap_or(&Value::Null);

    match name {
        TOOL_BRIDGE_INSTALL => {
            wrap_tool_result(name, bridge_install(arguments, Some(default_project)))
        }
        TOOL_BRIDGE_DIAGNOSTICS => {
            wrap_tool_result(name, bridge_diagnostics(arguments, Some(default_project)))
        }
        TOOL_SPEC_WRITE => {
            wrap_tool_result(name, game_spec_write(arguments, Some(default_project)))
        }
        TOOL_TICKET_PREPARE => {
            wrap_tool_result(name, game_ticket_prepare(arguments, Some(default_project)))
        }
        TOOL_UNITY_MANEUVER => {
            wrap_tool_result(name, unity_maneuver(arguments, Some(default_project)))
        }
        TOOL_LOOP_ONCE => {
            let value = game_dev_loop_once(arguments, Some(default_project))?;
            if value.get("ok").and_then(Value::as_bool) == Some(false) {
                Ok(tool_error_result(value))
            } else {
                Ok(tool_success_result(value))
            }
        }
        _ => Ok(tool_error_result(json!({
            "tool": name,
            "ok": false,
            "message": format!("Unknown tool: {name}")
        }))),
    }
}

fn wrap_tool_result(name: &str, result: Result<Value>) -> Result<Value> {
    Ok(match result {
        Ok(value) if value.get("ok").and_then(Value::as_bool) == Some(false) => {
            tool_error_result(value)
        }
        Ok(value) => tool_success_result(value),
        Err(error) => tool_error_result(json!({
            "tool": name,
            "ok": false,
            "message": error.to_string()
        })),
    })
}

fn tool_success_result(structured: Value) -> Value {
    let text = structured
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Lux MCP tool completed");
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": false
    })
}

fn tool_error_result(structured: Value) -> Value {
    let text = structured
        .get("message")
        .or_else(|| structured.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("Lux MCP tool failed");
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": true
    })
}

fn project_path_from_args(
    arguments: &Value,
    default_project_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = arguments.get("project_path").and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    default_project_path
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("project_path is required"))
}

fn bridge_install(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    if !project_path.exists() {
        anyhow::bail!("Project path does not exist: {}", project_path.display());
    }

    let assets_editor = project_path.join("Assets/Editor");
    fs::create_dir_all(&assets_editor)
        .with_context(|| format!("failed to create {}", assets_editor.display()))?;
    let marker = assets_editor.join("LuxMcpBridgeInstall.marker");
    fs::write(
        &marker,
        format!(
            "Lux MCP bridge install requested at {}\n",
            Utc::now().to_rfc3339()
        ),
    )
    .with_context(|| format!("failed to write {}", marker.display()))?;

    Ok(json!({
        "ok": true,
        "protocol": "lux.mcp.bridge_install.v1",
        "projectPath": project_path,
        "installedMarker": marker,
        "message": "MCP bridge install marker written idempotently"
    }))
}

fn bridge_diagnostics(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    if !discovery_path.is_file() {
        return Ok(json!({
            "ok": false,
            "protocol": "lux.mcp.bridge_diagnostics.v1",
            "projectPath": project_path,
            "discoveryPath": discovery_path,
            "stopReason": "unity_bridge_unavailable",
            "message": format!(
                "Unity bridge discovery file not found: {}. Open the project in Unity after installing the Lux bridge.",
                discovery_path.display()
            )
        }));
    }
    let discovery: Value = serde_json::from_str(
        &fs::read_to_string(&discovery_path)
            .with_context(|| format!("failed to read {}", discovery_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", discovery_path.display()))?;

    Ok(json!({
        "ok": true,
        "protocol": "lux.mcp.bridge_diagnostics.v1",
        "projectPath": project_path,
        "discoveryPath": discovery_path,
        "discovery": discovery,
        "message": "Unity bridge discovery file is present"
    }))
}

fn game_spec_write(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let mut spec = lux_spec::lux_load_or_init(&project_path)?;
    let mut changed = false;

    if let Some(project_name) = arguments.get("project_name").and_then(Value::as_str) {
        let project_name = project_name.trim();
        if !project_name.is_empty() && spec.project_name != project_name {
            spec.project_name = project_name.to_string();
            changed = true;
        }
        if !project_name.is_empty() && spec.source != "lux-mcp" {
            spec.source = "lux-mcp".to_string();
            changed = true;
        }
    }

    let seed = arguments
        .get("seed")
        .or_else(|| arguments.get("spec_seed"))
        .and_then(Value::as_object);
    if let Some(seed) = seed {
        if let Some(value) = seed.get("game_title").and_then(Value::as_str) {
            let value = value.trim();
            if !value.is_empty() && spec.meta.game_title.as_deref() != Some(value) {
                spec.meta.game_title = Some(value.to_string());
                changed = true;
            }
        }
        if let Some(value) = seed.get("genre").and_then(Value::as_str) {
            let value = value.trim();
            if !value.is_empty() && spec.meta.genre.as_deref() != Some(value) {
                spec.meta.genre = Some(value.to_string());
                changed = true;
            }
        }
        if let Some(value) = seed.get("elevator_pitch").and_then(Value::as_str) {
            let value = value.trim();
            if !value.is_empty() && spec.meta.elevator_pitch.as_deref() != Some(value) {
                spec.meta.elevator_pitch = Some(value.to_string());
                changed = true;
            }
        }
    }

    if let Some(objective) = arguments.get("objective").and_then(Value::as_str) {
        let objective = objective.trim();
        if !objective.is_empty() && spec.meta.elevator_pitch.as_deref() != Some(objective) {
            spec.meta.elevator_pitch = Some(objective.to_string());
            changed = true;
        }
    }

    if changed {
        lux_spec::lux_save(&project_path, &spec)?;
        spec = lux_spec::lux_load(&project_path)?;
    }

    let validation = spec.validate();
    Ok(json!({
        "ok": validation.is_ok(),
        "changed": changed,
        "projectPath": project_path,
        "specPath": project_path.join(".lux/spec.json"),
        "validation": validation.err().unwrap_or_else(|| "valid".to_string()),
        "ambiguity": spec.overall_ambiguity,
        "projectName": spec.project_name,
        "source": spec.source,
        "message": "Lux game spec written"
    }))
}

fn game_ticket_prepare(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    lux_spec::lux_init(&project_path)?;
    let objective = arguments
        .get("objective")
        .and_then(Value::as_str)
        .unwrap_or("Perform one safe Lux Unity game-development loop")
        .trim()
        .to_string();
    let verification_policy = arguments
        .get("verification_policy")
        .and_then(Value::as_str)
        .unwrap_or("compile_test_playmode_or_explicit_unavailable")
        .trim()
        .to_string();
    let non_goals = arguments
        .get("non_goals")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| {
            vec![
                "destructive rewrites".to_string(),
                "external service changes".to_string(),
                "Unity Editor window UI".to_string(),
            ]
        });

    let store = FileTicketStore::new(&project_path);
    let existing = store.get(FIRST_LOOP_TICKET_ID)?;
    let (ticket, created) = match existing {
        Some(ticket) => (ticket, false),
        None => {
            let now = Utc::now().to_rfc3339();
            let ticket = Ticket {
                id: FIRST_LOOP_TICKET_ID.to_string(),
                title: "Lux one-loop game-dev change".to_string(),
                description: objective.clone(),
                status: TicketStatus::ToDo,
                priority: TicketPriority::High,
                assignee: Some("lux-mcp".to_string()),
                blockers: Vec::new(),
                tags: vec!["lux-game-dev-loop-once".to_string()],
                spec_ref: Some(".lux/spec.json".to_string()),
                created_at: now.clone(),
                updated_at: now,
                execution_objective: Some(objective.clone()),
                allowed_executor: Some(TOOL_LOOP_ONCE.to_string()),
                dispatch_policy: Some(DispatchPolicy::Manual),
                verification_policy: Some(verification_policy.clone()),
                command_allowlist: Some(vec![
                    "lux bridge install".to_string(),
                    "lux unity compile".to_string(),
                    "lux unity run-tests".to_string(),
                    "lux unity screenshot".to_string(),
                ]),
                evidence_refs: Some(Vec::new()),
                blocker_policy: None,
                non_goals: Some(non_goals.clone()),
            };
            (store.create(ticket)?, true)
        }
    };

    Ok(json!({
        "ok": true,
        "protocol": "lux.game_ticket_prepare.v1",
        "projectPath": project_path,
        "ticketId": ticket.id,
        "ticketPath": format!(".lux/tickets/{}.json", ticket.id),
        "created": created,
        "status": format!("{:?}", ticket.status),
        "objective": ticket.execution_objective.unwrap_or(objective),
        "message": "Lux game-dev ticket prepared"
    }))
}

fn unity_maneuver(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let ticket_id = arguments
        .get("ticket_id")
        .and_then(Value::as_str)
        .unwrap_or(FIRST_LOOP_TICKET_ID);
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    let evidence_rel = format!(".lux/evidence/{ticket_id}-maneuver.json");
    let evidence_path = project_path.join(&evidence_rel);
    fs::create_dir_all(
        evidence_path
            .parent()
            .ok_or_else(|| anyhow!("invalid evidence path"))?,
    )?;

    let available = discovery_path.is_file();
    let status = if available { "ready" } else { "blocked" };
    let stop_reason = if available {
        "unity_maneuver_complete"
    } else {
        "unity_maneuver_unavailable"
    };
    let evidence = json!({
        "schemaVersion": 1,
        "kind": "unity_maneuver_evidence",
        "capturedAtUtc": Utc::now().to_rfc3339(),
        "ticketId": ticket_id,
        "status": status,
        "stopReason": stop_reason,
        "discoveryPath": discovery_path,
        "message": if available { "Unity bridge discovery is available" } else { "Unity bridge discovery file is missing; unavailable is explicit, not a silent fallback" },
    });
    fs::write(&evidence_path, serde_json::to_string_pretty(&evidence)?)?;
    append_ticket_evidence(&project_path, ticket_id, &evidence_rel)?;

    Ok(json!({
        "ok": available,
        "protocol": "lux.unity_maneuver.v1",
        "projectPath": project_path,
        "ticketId": ticket_id,
        "evidenceRefs": [evidence_rel],
        "stopReason": stop_reason,
        "status": status,
        "message": if available { "Unity maneuver readiness evidence recorded".to_string() } else { format!("Unity maneuver unavailable; evidence recorded at {evidence_rel}") }
    }))
}

fn append_ticket_evidence(project_path: &Path, ticket_id: &str, evidence_ref: &str) -> Result<()> {
    let store = FileTicketStore::new(project_path);
    let Some(mut ticket) = store.get(ticket_id)? else {
        return Ok(());
    };
    let mut refs = ticket.evidence_refs.take().unwrap_or_default();
    if !refs.iter().any(|value| value == evidence_ref) {
        refs.push(evidence_ref.to_string());
    }
    ticket.evidence_refs = Some(refs);
    ticket.updated_at = Utc::now().to_rfc3339();
    store.update(ticket_id, ticket)?;
    Ok(())
}

fn game_dev_loop_once(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let mut steps = Vec::new();

    let spec = game_spec_write(arguments, Some(&project_path))?;
    steps.push(step("spec_write", true, spec));

    let ticket = game_ticket_prepare(arguments, Some(&project_path))?;
    let ticket_id = ticket
        .get("ticketId")
        .and_then(Value::as_str)
        .unwrap_or(FIRST_LOOP_TICKET_ID)
        .to_string();
    steps.push(step("ticket_prepare", true, ticket));

    let maneuver_args = merge_args(
        arguments,
        json!({"project_path": project_path, "ticket_id": ticket_id}),
    );
    let maneuver = unity_maneuver(&maneuver_args, Some(&project_path))?;
    let ok = maneuver.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let stop_reason = if ok {
        "one_verified_loop_complete"
    } else {
        maneuver
            .get("stopReason")
            .and_then(Value::as_str)
            .unwrap_or("unity_maneuver_unavailable")
    };

    Ok(json!({
        "ok": ok,
        "protocol": "lux.game_dev_loop_once.v1",
        "projectPath": project_path,
        "ticketId": ticket_id,
        "steps": steps,
        "maneuver": maneuver,
        "stopReason": stop_reason,
        "verified": ok,
        "message": if ok {
            "One verified Lux game-dev loop completed".to_string()
        } else {
            format!(
                "Lux game-dev loop stopped with explicit blocker; evidence: {}",
                maneuver
                    .get("evidenceRefs")
                    .and_then(Value::as_array)
                    .and_then(|refs| refs.first())
                    .and_then(Value::as_str)
                    .unwrap_or("<missing evidence>")
            )
        }
    }))
}

fn merge_args(base: &Value, patch: Value) -> Value {
    let mut merged = base.as_object().cloned().unwrap_or_default();
    if let Some(patch) = patch.as_object() {
        for (key, value) in patch {
            merged.insert(key.clone(), value.clone());
        }
    }
    Value::Object(merged)
}

fn step(name: &str, ok: bool, detail: Value) -> Value {
    json!({
        "name": name,
        "ok": ok,
        "status": if ok { "ok" } else { "error" },
        "detail": detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_includes_structured_content() {
        let result = tool_success_result(
            json!({"steps": [], "stopReason": "one_verified_loop_complete", "message": "ok"}),
        );
        assert_eq!(result["isError"], false);
        assert!(result["structuredContent"]["steps"].is_array());
        assert_eq!(
            result["structuredContent"]["stopReason"],
            "one_verified_loop_complete"
        );
    }

    #[test]
    fn error_result_preserves_structured_content() {
        let result = tool_error_result(json!({
            "ok": false,
            "steps": [],
            "stopReason": "unity_maneuver_unavailable",
            "message": "blocked"
        }));
        assert_eq!(result["isError"], true);
        assert!(result["structuredContent"]["steps"].is_array());
        assert_eq!(
            result["structuredContent"]["stopReason"],
            "unity_maneuver_unavailable"
        );
    }
}
