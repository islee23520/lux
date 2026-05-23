use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    lux_spec,
    lux_ticket::{
        DispatchPolicy, FileTicketStore, Ticket, TicketPriority, TicketStatus, TicketStore,
    },
};

const PROTOCOL_VERSION: &str = "2024-11-05";
const LOOP_TICKET_ID: &str = "game-dev-loop-001";

#[derive(Clone, Debug)]
struct McpTool {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

pub fn run_mcp_stdio(default_project_path: Option<PathBuf>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP stdin line")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_json_rpc_request(request, default_project_path.as_deref()),
            Err(error) => json_rpc_error(Value::Null, -32700, format!("Parse error: {error}")),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_json_rpc_request(request: Value, default_project_path: Option<&Path>) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    match method {
        "initialize" => json_rpc_result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "lux", "version": env!("CARGO_PKG_VERSION")}
            }),
        ),
        "ping" => json_rpc_result(id, json!({})),
        "tools/list" => json_rpc_result(id, json!({"tools": tool_definitions()})),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let result = call_tool(name, &arguments, default_project_path);
            json_rpc_result(id, result)
        }
        _ => json_rpc_error(id, -32601, format!("Method not found: {method}")),
    }
}

fn json_rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn json_rpc_error(id: Value, code: i64, message: String) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn tool_definitions() -> Vec<Value> {
    registry()
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect()
}

fn registry() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "lux_bridge_install",
            description: "Install or refresh the Lux Unity bridge files in a Unity project.",
            input_schema: project_schema(),
        },
        McpTool {
            name: "lux_bridge_diagnostics",
            description: "Report Lux Unity bridge discovery/connection diagnostics for a project.",
            input_schema: project_schema(),
        },
        McpTool {
            name: "lux_game_spec_write",
            description: "Initialize or update the canonical .lux game spec for a minimal game-development loop.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {"type": "string"},
                    "project_name": {"type": "string"},
                    "seed": {"type": "object"}
                },
                "additionalProperties": false
            }),
        },
        McpTool {
            name: "lux_game_ticket_prepare",
            description: "Create or select one safe first-loop ticket in .lux/tickets.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {"type": "string"},
                    "objective": {"type": "string"},
                    "verification_policy": {"type": "string"},
                    "non_goals": {"type": "array", "items": {"type": "string"}}
                },
                "additionalProperties": false
            }),
        },
        McpTool {
            name: "lux_unity_maneuver",
            description: "Attempt one safe Unity maneuver and record explicit structured success or failure.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {"type": "string"},
                    "ticket_id": {"type": "string"},
                    "operation": {"type": "string"},
                    "dry_run": {"type": "boolean"}
                },
                "additionalProperties": false
            }),
        },
        McpTool {
            name: "lux_game_dev_loop_once",
            description: "Run the first milestone game-development loop once, then stop with structured evidence.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {"type": "string"},
                    "project_name": {"type": "string"},
                    "objective": {"type": "string"},
                    "validation_policy": {"type": "string"},
                    "autonomy_policy": {"type": "string"}
                },
                "additionalProperties": false
            }),
        },
    ]
}

fn project_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"project_path": {"type": "string"}},
        "additionalProperties": false
    })
}

fn call_tool(name: &str, arguments: &Value, default_project_path: Option<&Path>) -> Value {
    if !registry().iter().any(|tool| tool.name == name) {
        return tool_error_result(
            name,
            anyhow!("Unknown tool '{name}'. Use tools/list to discover supported Lux tools."),
        );
    }

    let result = match name {
        "lux_bridge_install" => bridge_install(arguments, default_project_path),
        "lux_bridge_diagnostics" => bridge_diagnostics(arguments, default_project_path),
        "lux_game_spec_write" => game_spec_write(arguments, default_project_path),
        "lux_game_ticket_prepare" => game_ticket_prepare(arguments, default_project_path),
        "lux_unity_maneuver" => unity_maneuver(arguments, default_project_path),
        "lux_game_dev_loop_once" => game_dev_loop_once(arguments, default_project_path),
        _ => unreachable!(),
    };

    match result {
        Ok(structured) => tool_success_result(name, structured),
        Err(error) => tool_error_result(name, error),
    }
}

fn tool_success_result(name: &str, structured: Value) -> Value {
    json!({
        "content": [{"type": "text", "text": format!("{name} completed")}],
        "structuredContent": structured,
        "isError": false
    })
}

fn tool_error_result(name: &str, error: anyhow::Error) -> Value {
    json!({
        "content": [{"type": "text", "text": error.to_string()}],
        "structuredContent": {"tool": name, "ok": false, "message": error.to_string()},
        "isError": true
    })
}

fn project_path_from_args(
    arguments: &Value,
    default_project_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = arguments.get("project_path").and_then(Value::as_str) {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    if let Some(path) = default_project_path {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir().context("failed to resolve current directory")
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
    if let Some(project_name) = arguments.get("project_name").and_then(Value::as_str) {
        if !project_name.trim().is_empty() {
            spec.project_name = project_name.to_string();
            spec.source = "lux-mcp".to_string();
            lux_spec::lux_save(&project_path, &spec)?;
        }
    }
    let validation = spec.validate();
    Ok(json!({
        "ok": validation.is_ok(),
        "projectPath": project_path,
        "specPath": project_path.join(".lux/spec.json"),
        "validation": validation.err().unwrap_or_else(|| "valid".to_string()),
        "ambiguity": spec.overall_ambiguity
    }))
}

fn game_ticket_prepare(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    lux_spec::lux_init(&project_path)?;
    let store = FileTicketStore::new(&project_path);
    let objective = arguments
        .get("objective")
        .and_then(Value::as_str)
        .unwrap_or(
            "Make one safe, verifiable Unity project change for the first Lux game-dev loop.",
        );
    let verification_policy = arguments
        .get("verification_policy")
        .and_then(Value::as_str)
        .unwrap_or("compile_or_explicit_unavailable_evidence");
    let non_goals = arguments
        .get("non_goals")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "destructive rewrites".to_string(),
                "Unity Editor window UI changes".to_string(),
                "external service calls".to_string(),
            ]
        });

    let (ticket, created) = if let Some(existing) = store.get(LOOP_TICKET_ID)? {
        (existing, false)
    } else {
        let now = Utc::now().to_rfc3339();
        let ticket = Ticket {
            id: LOOP_TICKET_ID.to_string(),
            title: "First Lux game-dev MCP loop".to_string(),
            description: objective.to_string(),
            status: TicketStatus::ToDo,
            priority: TicketPriority::Medium,
            assignee: Some("lux-mcp".to_string()),
            blockers: Vec::new(),
            tags: vec!["lux-mcp".to_string(), "game-dev-loop".to_string()],
            spec_ref: Some(".lux/spec.json".to_string()),
            created_at: now.clone(),
            updated_at: now,
            execution_objective: Some(objective.to_string()),
            allowed_executor: Some("lux-mcp".to_string()),
            dispatch_policy: Some(DispatchPolicy::Manual),
            verification_policy: Some(verification_policy.to_string()),
            command_allowlist: Some(vec![
                "compile".to_string(),
                "run-tests".to_string(),
                "bridge".to_string(),
            ]),
            evidence_refs: Some(Vec::new()),
            blocker_policy: None,
            non_goals: Some(non_goals),
        };
        (store.create(ticket)?, true)
    };

    Ok(json!({
        "ok": true,
        "created": created,
        "ticketId": ticket.id,
        "ticketPath": project_path.join(".lux/tickets").join(format!("{}.json", ticket.id)),
        "objective": ticket.execution_objective,
        "verificationPolicy": ticket.verification_policy,
        "nonGoals": ticket.non_goals
    }))
}

fn unity_maneuver(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let project_path = project_path_from_args(arguments, default_project_path)?;
    let ticket_id = arguments
        .get("ticket_id")
        .and_then(Value::as_str)
        .unwrap_or(LOOP_TICKET_ID);
    let evidence_dir = project_path.join(".lux/evidence");
    fs::create_dir_all(&evidence_dir)
        .with_context(|| format!("failed to create {}", evidence_dir.display()))?;
    let evidence_path = evidence_dir.join(format!("{ticket_id}-maneuver.json"));
    let evidence = json!({
        "ticketId": ticket_id,
        "operation": arguments.get("operation").and_then(Value::as_str).unwrap_or("diagnostic_maneuver"),
        "status": "blocked",
        "reason": "Unity bridge execution is unavailable in this MCP smoke path; no silent fallback was performed.",
        "recordedAt": Utc::now().to_rfc3339()
    });
    fs::write(&evidence_path, serde_json::to_string_pretty(&evidence)?)
        .with_context(|| format!("failed to write {}", evidence_path.display()))?;

    anyhow::bail!(
        "Unity maneuver requires a running Unity bridge; recorded explicit unavailable evidence at {}",
        evidence_path.display()
    )
}

fn game_dev_loop_once(arguments: &Value, default_project_path: Option<&Path>) -> Result<Value> {
    let spec = game_spec_write(arguments, default_project_path)?;
    let ticket = game_ticket_prepare(arguments, default_project_path)?;
    let maneuver = unity_maneuver(arguments, default_project_path);
    match maneuver {
        Ok(maneuver) => Ok(json!({
            "ok": true,
            "stopReason": "one_verified_loop_complete",
            "steps": [spec, ticket, maneuver]
        })),
        Err(error) => Err(anyhow!(json!({
            "ok": false,
            "stopReason": "unity_maneuver_unavailable",
            "message": error.to_string(),
            "steps": [spec, ticket]
        })
        .to_string())),
    }
}
