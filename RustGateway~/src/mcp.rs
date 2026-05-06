use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize { id: Value, params: InitializeParams },
    #[serde(rename = "tools/list")]
    ToolsList { id: Value },
    #[serde(rename = "tools/call")]
    ToolsCall { id: Value, params: ToolsCallParams },
    #[serde(rename = "resources/list")]
    ResourcesList { id: Value },
    #[serde(rename = "resources/read")]
    ResourcesRead {
        id: Value,
        params: ResourcesReadParams,
    },
    #[serde(untagged)]
    Unknown(Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: Value,
    #[serde(default)]
    pub client_info: Value,
}

#[derive(Debug, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Deserialize)]
pub struct ResourcesReadParams {
    pub uri: String,
}

pub struct McpServer {
    server_name: String,
    server_version: String,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
}

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: fn(&Value) -> anyhow::Result<Vec<McpContent>>,
}

pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub handler: fn() -> anyhow::Result<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl McpServer {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            server_name: name.to_string(),
            server_version: version.to_string(),
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }

    pub fn register_tool(&mut self, tool: McpTool) {
        self.tools.push(tool);
    }

    pub fn register_resource(&mut self, resource: McpResource) {
        self.resources.push(resource);
    }

    pub fn handle_request(&self, request: &str) -> Option<String> {
        let request = request.trim();
        if request.is_empty() {
            return None;
        }

        let value: Value = match serde_json::from_str(request) {
            Ok(value) => value,
            Err(_) => return Some(error_response(Value::Null, -32700, "Parse error")),
        };

        if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(error_response(
                id_or_null(&value),
                -32600,
                "Invalid Request",
            ));
        }

        let Some(method) = value.get("method").and_then(Value::as_str) else {
            return Some(error_response(
                id_or_null(&value),
                -32600,
                "Invalid Request",
            ));
        };
        if let Ok(typed_request) = serde_json::from_value::<McpRequest>(value.clone()) {
            observe_mcp_request(typed_request);
        }

        if method == "notifications/initialized" {
            return None;
        }

        let Some(id) = value.get("id").cloned() else {
            return None;
        };

        let result = match method {
            "initialize" => self.handle_initialize(&value),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => {
                self.handle_tools_call(value.get("params").cloned().unwrap_or(Value::Null))
            }
            "resources/list" => self.handle_resources_list(),
            "resources/read" => {
                self.handle_resources_read(value.get("params").cloned().unwrap_or(Value::Null))
            }
            _ => return Some(error_response(id, -32601, "Method not found")),
        };

        Some(match result {
            Ok(result) => success_response(id, result),
            Err((code, message)) => error_response(id, code, &message),
        })
    }

    fn handle_initialize(&self, value: &Value) -> Result<Value, (i32, String)> {
        let params = serde_json::from_value::<InitializeParams>(
            value.get("params").cloned().unwrap_or(Value::Null),
        )
        .map_err(|_| (-32602, "Invalid params".to_string()))?;
        let _ = (
            params.protocol_version,
            params.capabilities,
            params.client_info,
        );

        Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": self.server_name,
                "version": self.server_version
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value, (i32, String)> {
        let tools: Vec<Value> = self
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema
                })
            })
            .collect();

        Ok(json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, params: Value) -> Result<Value, (i32, String)> {
        let params: ToolsCallParams =
            serde_json::from_value(params).map_err(|_| (-32602, "Invalid params".to_string()))?;
        let Some(tool) = self.tools.iter().find(|tool| tool.name == params.name) else {
            return Err((-32602, format!("Unknown tool: {}", params.name)));
        };

        let content = (tool.handler)(&params.arguments)
            .map_err(|error| (-32603, format!("Tool handler failed: {error}")))?;
        Ok(json!({ "content": content }))
    }

    fn handle_resources_list(&self) -> Result<Value, (i32, String)> {
        let resources: Vec<Value> = self
            .resources
            .iter()
            .map(|resource| {
                json!({
                    "uri": resource.uri,
                    "name": resource.name,
                    "description": resource.description,
                    "mimeType": resource.mime_type
                })
            })
            .collect();

        Ok(json!({ "resources": resources }))
    }

    fn handle_resources_read(&self, params: Value) -> Result<Value, (i32, String)> {
        let params: ResourcesReadParams =
            serde_json::from_value(params).map_err(|_| (-32602, "Invalid params".to_string()))?;
        let Some(resource) = self
            .resources
            .iter()
            .find(|resource| resource.uri == params.uri)
        else {
            return Err((-32602, format!("Unknown resource: {}", params.uri)));
        };

        let text = (resource.handler)()
            .map_err(|error| (-32603, format!("Resource handler failed: {error}")))?;
        Ok(json!({
            "contents": [{
                "uri": resource.uri,
                "mimeType": resource.mime_type,
                "text": text
            }]
        }))
    }
}

pub fn create_lux_mcp_server() -> McpServer {
    let mut server = McpServer::new("lux-mcp", env!("CARGO_PKG_VERSION"));

    for tool in [
        (
            "unity_compile",
            "Compile Unity project",
            unity_compile_handler as fn(&Value) -> anyhow::Result<Vec<McpContent>>,
        ),
        ("unity_test", "Run Unity tests", unity_test_handler),
        (
            "unity_screenshot",
            "Capture Unity editor screenshot",
            unity_screenshot_handler,
        ),
        (
            "unity_hierarchy",
            "Read Unity hierarchy metadata",
            unity_hierarchy_handler,
        ),
        (
            "unity_dynamic_code",
            "Execute dynamic C# code in Unity",
            unity_dynamic_code_handler,
        ),
        (
            "unity_get_logs",
            "Read recent Unity console logs",
            unity_get_logs_handler,
        ),
        ("skill_list", "List LUX skills", skill_list_handler),
        (
            "skill_info",
            "Read LUX skill information",
            skill_info_handler,
        ),
        (
            "ai_log_recent",
            "Read recent AI action log entries",
            ai_log_recent_handler,
        ),
        (
            "lux_context",
            "Read current Unity context",
            lux_context_handler,
        ),
        (
            "selected_file_context",
            "Read selected file context",
            selected_file_context_handler,
        ),
        (
            "execute_shell",
            "Execute a shell command",
            execute_shell_handler,
        ),
        ("execute_git", "Execute a git command", execute_git_handler),
    ] {
        server.register_tool(McpTool {
            name: tool.0.to_string(),
            description: tool.1.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string" },
                    "cwd": { "type": "string" },
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "name": { "type": "string" },
                    "code": { "type": "string" },
                    "filePath": { "type": "string" }
                },
                "required": []
            }),
            handler: tool.2,
        });
    }

    server.register_resource(McpResource {
        uri: "unity://context".to_string(),
        name: "Unity Context".to_string(),
        description: "Current Unity editor context".to_string(),
        mime_type: "application/json".to_string(),
        handler: unity_context_resource,
    });
    server.register_resource(McpResource {
        uri: "unity://ai-log".to_string(),
        name: "AI Action Log".to_string(),
        description: "Recent LUX AI action log entries".to_string(),
        mime_type: "application/json".to_string(),
        handler: ai_log_resource,
    });
    server.register_resource(McpResource {
        uri: "unity://skills".to_string(),
        name: "LUX Skills".to_string(),
        description: "Installed LUX skill summary".to_string(),
        mime_type: "application/json".to_string(),
        handler: skills_resource,
    });

    server
}

pub fn run_stdio_server(server: McpServer) -> anyhow::Result<()> {
    eprintln!("LUX MCP stdio server started");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP request from stdin")?;
        if let Some(response) = server.handle_request(&line) {
            writeln!(stdout, "{response}").context("failed to write MCP response to stdout")?;
            stdout.flush().context("failed to flush MCP stdout")?;
        }
    }

    eprintln!("LUX MCP stdio server stopped");
    Ok(())
}

fn success_response(id: Value, result: Value) -> String {
    serialize_response(JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    })
}

fn error_response(id: Value, code: i32, message: &str) -> String {
    serialize_response(JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
        }),
    })
}

fn serialize_response(response: JsonRpcResponse) -> String {
    serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            r#"{{"jsonrpc":"2.0","id":null,"error":{{"code":-32603,"message":"serialization failed: {error}"}}}}"#
        )
    })
}

fn id_or_null(value: &Value) -> Value {
    value.get("id").cloned().unwrap_or(Value::Null)
}

fn observe_mcp_request(request: McpRequest) {
    match request {
        McpRequest::Initialize { id, params } => {
            let _ = (
                id,
                params.protocol_version,
                params.capabilities,
                params.client_info,
            );
        }
        McpRequest::ToolsList { id } | McpRequest::ResourcesList { id } => {
            let _ = id;
        }
        McpRequest::ToolsCall { id, params } => {
            let _ = (id, params.name, params.arguments);
        }
        McpRequest::ResourcesRead { id, params } => {
            let _ = (id, params.uri);
        }
        McpRequest::Unknown(value) => {
            let _ = value;
        }
    }
}

fn text_content(value: Value) -> anyhow::Result<Vec<McpContent>> {
    Ok(vec![McpContent {
        content_type: "text".to_string(),
        text: serde_json::to_string(&value)?,
    }])
}

fn project_path(arguments: &Value) -> anyhow::Result<PathBuf> {
    if let Some(path) = arguments.get("projectPath").and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    if let Ok(path) = std::env::var("LUX_PROJECT_PATH") {
        return Ok(PathBuf::from(path));
    }
    find_unity_project_root(std::env::current_dir()?)
        .context("Unity project not found. Pass projectPath or set LUX_PROJECT_PATH.")
}

fn cwd_path(arguments: &Value) -> anyhow::Result<PathBuf> {
    if let Some(path) = arguments.get("cwd").and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = arguments.get("projectPath").and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    Ok(std::env::current_dir()?)
}

fn find_unity_project_root(start: PathBuf) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start
    };

    loop {
        if current.join("Assets").is_dir() && current.join("ProjectSettings").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn read_json_file(path: &Path) -> anyhow::Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

fn run_lux_command(args: &[&str], arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let exe = std::env::current_exe().context("failed to resolve lux executable")?;
    let output = ProcessCommand::new(exe)
        .args(args)
        .current_dir(cwd_path(arguments)?)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to run lux {}", args.join(" ")))?;

    text_content(json!({
        "success": output.status.success(),
        "status": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr)
    }))
}

fn unity_compile_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    run_lux_command(
        &["compile", "--project-path", &project.to_string_lossy()],
        arguments,
    )
}

fn unity_test_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    run_lux_command(
        &["run-tests", "--project-path", &project.to_string_lossy()],
        arguments,
    )
}

fn unity_screenshot_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    run_lux_command(
        &[
            "unity",
            "screenshot",
            "--project-path",
            &project.to_string_lossy(),
        ],
        arguments,
    )
}

fn unity_hierarchy_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    run_lux_command(
        &[
            "unity",
            "get-hierarchy",
            "--project-path",
            &project.to_string_lossy(),
        ],
        arguments,
    )
}

fn unity_dynamic_code_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    let code = arguments
        .get("code")
        .and_then(Value::as_str)
        .context("code is required")?;
    run_lux_command(
        &[
            "unity",
            "execute-dynamic-code",
            "--project-path",
            &project.to_string_lossy(),
            "--code",
            code,
        ],
        arguments,
    )
}

fn unity_get_logs_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    run_lux_command(
        &[
            "unity",
            "get-logs",
            "--project-path",
            &project.to_string_lossy(),
        ],
        arguments,
    )
}

fn skill_list_handler(_: &Value) -> anyhow::Result<Vec<McpContent>> {
    text_content(json!({ "skills": discover_skill_summaries()? }))
}

fn skill_info_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .context("name is required")?;
    let skills = discover_skill_summaries()?;
    let Some(skill) = skills
        .iter()
        .find(|skill| skill.get("name").and_then(Value::as_str) == Some(name))
    else {
        return Err(anyhow!("skill not found: {name}"));
    };
    text_content(skill.clone())
}

fn ai_log_recent_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
    text_content(read_recent_ai_log(&project, limit)?)
}

fn lux_context_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let project = project_path(arguments)?;
    text_content(read_unity_context(&project)?)
}

fn selected_file_context_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let path = arguments
        .get("filePath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            let project = project_path(arguments).ok()?;
            Some(project.join("UserSettings/LuxSelectedFileContext.json"))
        })
        .context("filePath is required when selected file context is not exported")?;

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    text_content(json!({ "path": path, "text": text }))
}

fn execute_shell_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .context("command is required")?;
    let args = string_array(arguments.get("args"))?;
    let output = ProcessCommand::new(command)
        .args(args)
        .current_dir(cwd_path(arguments)?)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to execute {command}"))?;

    text_content(json!({
        "success": output.status.success(),
        "status": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr)
    }))
}

fn execute_git_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
    let args = string_array(arguments.get("args"))?;
    if args.is_empty() {
        return Err(anyhow!("args is required"));
    }
    let output = ProcessCommand::new("git")
        .args(args)
        .current_dir(cwd_path(arguments)?)
        .stdin(Stdio::null())
        .output()
        .context("failed to execute git")?;

    text_content(json!({
        "success": output.status.success(),
        "status": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr)
    }))
}

fn unity_context_resource() -> anyhow::Result<String> {
    Ok(serde_json::to_string(&read_unity_context(&project_path(
        &Value::Null,
    )?)?)?)
}

fn ai_log_resource() -> anyhow::Result<String> {
    Ok(serde_json::to_string(&read_recent_ai_log(
        &project_path(&Value::Null)?,
        50,
    )?)?)
}

fn skills_resource() -> anyhow::Result<String> {
    Ok(serde_json::to_string(&json!({
        "skills": discover_skill_summaries()?
    }))?)
}

fn read_unity_context(project: &Path) -> anyhow::Result<Value> {
    let bridge_settings_path = project.join("UserSettings/LuxBridgeSettings.json");
    if bridge_settings_path.exists() {
        return read_json_file(&bridge_settings_path);
    }

    let context_path = project.join("UserSettings/LuxUnityContext.json");
    if context_path.exists() {
        return read_json_file(&context_path);
    }

    let output =
        ProcessCommand::new(std::env::current_exe().context("failed to resolve lux executable")?)
            .args(["unity", "context", "--project-path"])
            .arg(project)
            .stdin(Stdio::null())
            .output()
            .context("failed to run lux unity context")?;

    if !output.status.success() {
        return Err(anyhow!(
            "lux unity context failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    serde_json::from_slice(&output.stdout).context("lux unity context output was not valid JSON")
}

fn read_recent_ai_log(project: &Path, limit: usize) -> anyhow::Result<Value> {
    let path = project.join(".lux/ai-action-log.jsonl");
    let fallback_path = project.join("UserSettings/LuxAiActionLog.jsonl");
    let path = if path.exists() { path } else { fallback_path };

    if !path.exists() {
        return Ok(json!({ "path": path, "count": 0, "entries": [] }));
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries: Vec<Value> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }

    Ok(json!({ "path": path, "count": entries.len(), "entries": entries }))
}

fn discover_skill_summaries() -> anyhow::Result<Vec<Value>> {
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.join("../Skills"));
        roots.push(current.join("Skills"));
        roots.push(current.join(".lux/skills"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join(".lux/skills"));
    }

    let mut skills = Vec::new();
    for root in roots.into_iter().filter(|root| root.is_dir()) {
        for entry in
            fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
        {
            let entry = entry?;
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let manifest_path = dir.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let manifest = read_json_file(&manifest_path).unwrap_or_else(|_| json!({}));
            skills.push(json!({
                "name": manifest.get("name").and_then(Value::as_str).unwrap_or_else(|| dir.file_name().and_then(|name| name.to_str()).unwrap_or("unknown")),
                "version": manifest.get("version").and_then(Value::as_str).unwrap_or("unknown"),
                "description": manifest.get("description").and_then(Value::as_str).unwrap_or(""),
                "path": dir
            }));
        }
    }
    skills.sort_by(|left, right| {
        left.get("name")
            .and_then(Value::as_str)
            .cmp(&right.get("name").and_then(Value::as_str))
    });
    skills.dedup_by(|left, right| left.get("name") == right.get("name"));
    Ok(skills)
}

fn string_array(value: Option<&Value>) -> anyhow::Result<Vec<String>> {
    match value {
        None => Ok(Vec::new()),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .context("args must contain strings only")
            })
            .collect(),
        Some(_) => Err(anyhow!("args must be an array")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_handler(arguments: &Value) -> anyhow::Result<Vec<McpContent>> {
        text_content(json!({ "success": true, "arguments": arguments }))
    }

    fn resource_handler() -> anyhow::Result<String> {
        Ok(json!({ "ok": true }).to_string())
    }

    fn test_server() -> McpServer {
        let mut server = McpServer::new("lux-mcp", "0.1.0");
        server.register_tool(McpTool {
            name: "unity_compile".to_string(),
            description: "Compile Unity project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": { "projectPath": { "type": "string" } },
                "required": []
            }),
            handler: ok_handler,
        });
        server.register_resource(McpResource {
            uri: "unity://context".to_string(),
            name: "Unity Context".to_string(),
            description: "Current Unity editor context".to_string(),
            mime_type: "application/json".to_string(),
            handler: resource_handler,
        });
        server
    }

    fn response_json(server: &McpServer, request: &str) -> Value {
        serde_json::from_str(&server.handle_request(request).expect("response expected"))
            .expect("valid JSON response")
    }

    #[test]
    fn initialize_returns_capabilities_with_tools_and_resources() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}"#,
        );

        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert!(response["result"]["capabilities"]["resources"].is_object());
    }

    #[test]
    fn tools_list_returns_all_registered_tools() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        );

        assert_eq!(response["result"]["tools"][0]["name"], "unity_compile");
    }

    #[test]
    fn tools_call_invokes_handler() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"unity_compile","arguments":{"projectPath":"/tmp/project"}}}"#,
        );

        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let content: Value = serde_json::from_str(text).unwrap();
        assert_eq!(content["success"], true);
        assert_eq!(content["arguments"]["projectPath"], "/tmp/project");
    }

    #[test]
    fn tools_call_unknown_tool_returns_error() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"missing_tool","arguments":{}}}"#,
        );

        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool"));
    }

    #[test]
    fn resources_list_returns_all_registered_resources() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":4,"method":"resources/list"}"#,
        );

        assert_eq!(response["result"]["resources"][0]["uri"], "unity://context");
    }

    #[test]
    fn resources_read_invokes_handler() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"unity://context"}}"#,
        );

        assert_eq!(response["result"]["contents"][0]["uri"], "unity://context");
        assert_eq!(response["result"]["contents"][0]["text"], r#"{"ok":true}"#);
    }

    #[test]
    fn resources_read_unknown_uri_returns_error() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":55,"method":"resources/read","params":{"uri":"unity://missing"}}"#,
        );

        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown resource"));
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let response = response_json(
            &test_server(),
            r#"{"jsonrpc":"2.0","id":6,"method":"missing/method"}"#,
        );

        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn invalid_json_returns_parse_error() {
        let response = response_json(&test_server(), "not json");

        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn notification_has_no_id_and_no_response() {
        let server = test_server();
        assert!(server
            .handle_request(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none());
        assert!(server
            .handle_request(r#"{"jsonrpc":"2.0","method":"tools/list"}"#)
            .is_none());
    }

    #[test]
    fn lux_server_registers_thirteen_tools_and_three_resources() {
        let response = response_json(
            &create_lux_mcp_server(),
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#,
        );
        assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 13);

        let response = response_json(
            &create_lux_mcp_server(),
            r#"{"jsonrpc":"2.0","id":8,"method":"resources/list"}"#,
        );
        assert_eq!(response["result"]["resources"].as_array().unwrap().len(), 3);
    }
}
