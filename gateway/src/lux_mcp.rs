use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    lux_spec,
    lux_ticket::{
        DispatchPolicy, FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus,
        TicketStore,
    },
    project,
};

const TOOL_BRIDGE_INSTALL: &str = "lux_bridge_install";
const TOOL_BRIDGE_DIAGNOSTICS: &str = "lux_bridge_diagnostics";
const TOOL_SPEC_WRITE: &str = "lux_game_spec_write";
const TOOL_TICKET_PREPARE: &str = "lux_game_ticket_prepare";
const TOOL_UNITY_MANEUVER: &str = "lux_unity_maneuver";
const TOOL_LOOP_ONCE: &str = "lux_game_dev_loop_once";

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
                "objective": { "type": "string" },
                "spec_seed": { "type": "object" },
                "validation_policy": { "type": "string" },
                "autonomy_policy": { "type": "string" },
                "ticket_id": { "type": "string" }
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

    let outcome = match name {
        TOOL_BRIDGE_INSTALL => bridge_install(default_project, arguments),
        TOOL_BRIDGE_DIAGNOSTICS => bridge_diagnostics(default_project, arguments),
        TOOL_SPEC_WRITE => game_spec_write(default_project, arguments),
        TOOL_TICKET_PREPARE => game_ticket_prepare(default_project, arguments),
        TOOL_UNITY_MANEUVER => unity_maneuver(default_project, arguments),
        TOOL_LOOP_ONCE => {
            let structured = game_dev_loop_once(default_project, arguments);
            return Ok(match structured {
                Ok(value) if value.get("verified").and_then(Value::as_bool) == Some(false) => {
                    tool_error_result(value)
                }
                Ok(value) => tool_success_result(value),
                Err(error) => tool_error_result(json!({"tool": name, "error": error.to_string()})),
            });
        }
        _ => {
            return Ok(tool_error_result(
                json!({"tool": name, "error": format!("unknown tool: {name}")}),
            ))
        }
    };

    Ok(match outcome {
        Ok(value) => tool_success_result(value),
        Err(error) => tool_error_result(json!({"tool": name, "error": error.to_string()})),
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

fn tool_error_result(name: &str, error: anyhow::Error) -> Value {
    let message = error.to_string();
    let structured = serde_json::from_str::<Value>(&message).unwrap_or_else(|_| {
        json!({"tool": name, "ok": false, "message": message})
    });
    json!({
        "content": [{"type": "text", "text": message}],
        "structuredContent": structured,
        "isError": true
    })
}

fn resolve_project_path(default_project: &Path, args: &Value) -> PathBuf {
    args.get("project_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_project.to_path_buf())
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
        "projectPath": project_path,
        "installedMarker": marker,
        "note": "MCP bridge install path is idempotent and project-local."
    }))
}

fn bridge_diagnostics(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    if !discovery_path.is_file() {
        anyhow::bail!(
            "Unity bridge discovery file not found: {}. Open the project in Unity after installing the Lux bridge.",
            discovery_path.display()
        );
    }
    let discovery: Value = serde_json::from_str(
        &fs::read_to_string(&discovery_path)
            .with_context(|| format!("failed to read {}", discovery_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", discovery_path.display()))?;

    Ok(json!({
        "ok": true,
        "projectPath": project_path,
        "discoveryPath": discovery_path,
        "discovery": discovery
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

    if let Some(seed) = arguments.get("seed").and_then(Value::as_object) {
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
        "source": spec.source
    }))
}

fn game_ticket_prepare(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    lux_spec::lux_init(&project_path)?;
    Ok(json!({
        "protocol": "lux.mcp.bridge_install.v1",
        "projectPath": project_path,
        "bridgeInstalled": false,
        "message": "Lux .lux workspace initialized; bridge file refresh is handled by lux bridge install in full CLI mode"
    }))
}

fn bridge_diagnostics(default_project: &Path, args: &Value) -> Result<Value> {
    let project_path = resolve_project_path(default_project, args);
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    let available = discovery_path.is_file();
    Ok(json!({
        "protocol": "lux.mcp.bridge_diagnostics.v1",
        "projectPath": project_path,
        "available": available,
        "discoveryPath": discovery_path,
        "message": if available { "Unity bridge discovery file is present" } else { "Unity bridge discovery file is missing" }
    }))
}

fn game_spec_write(default_project: &Path, args: &Value) -> Result<Value> {
    let project_path = resolve_project_path(default_project, args);
    let lux_path = lux_spec::lux_init(&project_path)?;
    let mut spec = lux_spec::lux_load(&project_path)?;
    if let Ok(Some(detection)) = project::detect_unity_project(&project_path) {
        lux_spec::apply_detection_to_spec(&mut spec, &detection);
    }
    if let Some(objective) = args.get("objective").and_then(Value::as_str) {
        if !objective.trim().is_empty() {
            spec.source = "lux-mcp-game-spec-write".to_string();
            spec.meta.elevator_pitch = Some(objective.trim().to_string());
        }
    }
    spec.updated_at = Utc::now().to_rfc3339();
    spec.validate()
        .map_err(|error| anyhow!("spec validation failed: {error}"))?;
    lux_spec::lux_save(&project_path, &spec)?;
    let spec_path = project_path.join(".lux/spec.json");
    Ok(json!({
        "protocol": "lux.game_spec_write.v1",
        "projectPath": project_path,
        "luxPath": lux_path,
        "specPath": spec_path,
        "projectName": spec.project_name,
        "status": "written",
        "message": "Lux game spec written"
    }))
}

fn game_ticket_prepare(default_project: &Path, args: &Value) -> Result<Value> {
    let project_path = resolve_project_path(default_project, args);
    lux_spec::lux_init(&project_path)?;
    let objective = args
        .get("objective")
        .and_then(Value::as_str)
        .unwrap_or("Perform one safe Lux Unity game-development loop")
        .trim()
        .to_string();
    let store = FileTicketStore::new(&project_path);
    let existing = store
        .list(TicketFilter {
            tag: Some("lux-game-dev-loop-once".to_string()),
            ..TicketFilter::default()
        })?
        .into_iter()
        .filter(|ticket| ticket.status != TicketStatus::Done)
        .min_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
    let (ticket, created) = match existing {
        Some(ticket) => (ticket, false),
        None => {
            let now = Utc::now().to_rfc3339();
            let ticket = Ticket {
                id: format!("lux-loop-once-{}", Uuid::new_v4().simple()),
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
                allowed_executor: Some("lux_game_dev_loop_once".to_string()),
                dispatch_policy: Some(DispatchPolicy::Manual),
                verification_policy: Some(
                    "compile_test_playmode_or_explicit_unavailable".to_string(),
                ),
                command_allowlist: Some(vec![
                    "lux bridge install".to_string(),
                    "lux unity compile".to_string(),
                    "lux unity run-tests".to_string(),
                    "lux unity screenshot".to_string(),
                ]),
                evidence_refs: Some(Vec::new()),
                blocker_policy: None,
                non_goals: Some(vec![
                    "destructive rewrites".to_string(),
                    "external service changes".to_string(),
                    "Unity Editor window UI".to_string(),
                ]),
            };
            (store.create(ticket)?, true)
        }
    };
    Ok(json!({
        "protocol": "lux.game_ticket_prepare.v1",
        "projectPath": project_path,
        "ticketId": ticket.id,
        "ticketPath": format!(".lux/tickets/{}.json", ticket.id),
        "created": created,
        "status": format!("{:?}", ticket.status),
        "objective": objective,
        "message": "Lux game-dev ticket prepared"
    }))
}

fn unity_maneuver(default_project: &Path, args: &Value) -> Result<Value> {
    let project_path = resolve_project_path(default_project, args);
    let ticket_id = args.get("ticket_id").and_then(Value::as_str);
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    fs::create_dir_all(project_path.join(".lux/evidence"))?;
    let evidence_rel = format!(
        ".lux/evidence/loop-once-{}.json",
        Utc::now().format("%Y%m%dT%H%M%S%3fZ")
    );
    let evidence_path = project_path.join(&evidence_rel);
    let available = discovery_path.is_file();
    let status = if available { "ready" } else { "unavailable" };
    let reason = if available {
        "unity_bridge_available"
    } else {
        "unity_bridge_discovery_missing"
    };
    let evidence = json!({
        "protocol": "lux.unity_maneuver.evidence.v1",
        "ticketId": ticket_id,
        "status": status,
        "reason": reason,
        "discoveryPath": discovery_path,
        "recordedAt": Utc::now().to_rfc3339(),
    });
    fs::write(&evidence_path, serde_json::to_string_pretty(&evidence)?)?;
    if let Some(ticket_id) = ticket_id {
        append_ticket_evidence(&project_path, ticket_id, &evidence_rel)?;
    }
    if !available {
        return Err(anyhow!(
            "Unity bridge discovery file missing at {}; open Unity or run lux bridge install/diagnostics first",
            discovery_path.display()
        ));
    }
    Ok(json!({
        "protocol": "lux.unity_maneuver.v1",
        "projectPath": project_path,
        "ticketId": ticket_id,
        "evidenceRefs": [evidence_rel],
        "status": status,
        "message": "Unity maneuver readiness evidence recorded"
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

fn game_dev_loop_once(default_project: &Path, args: &Value) -> Result<Value> {
    let project_path = resolve_project_path(default_project, args);
    let mut steps = Vec::new();
    let mut stop_reason = "one_verified_loop_complete";

    let bridge = bridge_install(&project_path, &json!({"project_path": project_path}))?;
    steps.push(step("bridge_install", "ok", bridge));

    let diagnostics = bridge_diagnostics(&project_path, &json!({"project_path": project_path}))?;
    steps.push(step("bridge_diagnostics", "ok", diagnostics));

    let spec = game_spec_write(&project_path, args)?;
    steps.push(step("spec_write", "ok", spec));

    let ticket = game_ticket_prepare(&project_path, args)?;
    let ticket_id = ticket
        .get("ticketId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    steps.push(step("ticket_prepare", "ok", ticket));

    let maneuver_args = merge_args(
        args,
        json!({"project_path": project_path, "ticket_id": ticket_id}),
    );
    match unity_maneuver(&project_path, &maneuver_args) {
        Ok(value) => steps.push(step("unity_maneuver", "ok", value)),
        Err(error) => {
            stop_reason = "unity_bridge_unavailable";
            steps.push(step(
                "unity_maneuver",
                "error",
                json!({"error": error.to_string(), "ticketId": ticket_id}),
            ));
        }
    }

    let ok = stop_reason == "one_verified_loop_complete";
    let structured = json!({
        "protocol": "lux.game_dev_loop_once.v1",
        "projectPath": project_path,
        "ticketId": ticket_id,
        "steps": steps,
        "stopReason": stop_reason,
        "verified": ok,
        "message": if ok { "One verified Lux game-dev loop completed" } else { "Lux game-dev loop stopped with explicit blocker" }
    });

    Ok(structured)
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

fn step(name: &str, status: &str, detail: Value) -> Value {
    json!({
        "name": name,
        "status": status,
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
}
