use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const TOKEN: &str = "lux-cli-smoke-token";

struct GatewayProcess {
    child: Child,
}

#[test]
fn cli_help_shows_correctly_on_all_platforms() {
    assert_command_help_contains(&["--help"], "Lux CLI");
    assert_command_help_contains(&["screenshot", "--help"], "--baseline");
    assert_command_help_contains(&["screenshot", "--help"], "--compare");
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn cli_server_starts_and_enforces_header_auth_and_origin_policy() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    assert_eq!(
        websocket_status(
            port,
            "/events?role=publisher&client_id=cli-smoke",
            &[("x-lux-token", TOKEN)],
        ),
        101
    );
    assert_eq!(
        websocket_status(port, "/events?token=lux-cli-smoke-token", &[]),
        101
    );
    assert_eq!(
        websocket_status(port, "/events?token=wrong-token", &[]),
        401
    );
    assert_eq!(
        websocket_status(
            port,
            "/events?role=publisher",
            &[
                ("x-lux-token", TOKEN),
                ("Origin", "http://localhost.evil.example"),
            ],
        ),
        403
    );
}

#[test]
fn rust_lux_cli_exposes_batch_mode_help_flags() {
    assert_command_help_contains(&["serve", "--help"], "--token");
    assert_command_help_contains(&["schema", "--help"], "Usage");
    assert_command_help_contains(&["compile", "--help"], "--project-path");
    assert_command_help_contains(&["run-tests", "--help"], "--test-platform");
    assert_command_help_contains(&["run-tests", "--help"], "--test-results");
    assert_command_help_contains(&["run-tests", "--help"], "--log-file");
    assert_command_help_contains(&["screenshot", "--help"], "--project-path");
    assert_command_help_contains(&["ai-log", "recent", "--help"], "--project-path");
    assert_command_help_contains(&["ai-log", "recent", "--help"], "--event-type");
    assert_command_help_contains(&["ai-log", "tail", "--help"], "--follow");
    assert_command_help_contains(&["ai-log", "context", "--help"], "--json");
    assert_command_help_contains(&["ai-log", "compact", "--help"], "--max-lines");
    assert_command_help_contains(&["ai-log", "work-step", "--help"], "--name");
    assert_command_help_contains(&["unity", "status", "--help"], "--project-path");
    assert_command_help_contains(&["unity", "context", "--help"], "--refresh");
    assert_command_help_contains(&["unity", "backend-status", "--help"], "--project-path");
    assert_command_help_contains(
        &["unity", "backend-list-commands", "--help"],
        "--project-path",
    );
    assert_command_help_contains(&["unity", "get-logs", "--help"], "--project-path");
    assert_command_help_contains(&["unity", "clear-console", "--help"], "--project-path");
    assert_command_help_contains(&["unity", "focus-window", "--help"], "--project-path");
    assert_command_help_contains(&["unity", "launch", "--help"], "--no-wait");
    assert_command_help_contains(&["unity", "scene-smoke", "--help"], "--object-count");
    assert_command_help_contains(&["unity", "scene-smoke", "--help"], "--batch");
    assert_command_help_contains(&["unity", "create-objects", "--help"], "--scene-path");
    assert_command_help_contains(&["unity", "create-objects", "--help"], "--object-count");
    assert_command_help_contains(&["unity", "find-game-objects", "--help"], "--search-mode");
    assert_command_help_contains(&["unity", "find-game-objects", "--help"], "--inline-limit");
    assert_command_help_contains(&["unity", "screenshot", "--help"], "--capture-mode");
    assert_command_help_contains(&["unity", "screenshot", "--help"], "--annotate-elements");
    assert_command_help_contains(&["unity", "screenshot", "--help"], "--elements-only");
    assert_command_help_contains(&["unity", "get-hierarchy", "--help"], "--root-path");
    assert_command_help_contains(&["unity", "get-hierarchy", "--help"], "--use-selection");
    assert_command_help_contains(&["unity", "control-play-mode", "--help"], "--action");
    assert_command_help_contains(&["unity", "control-play-mode", "--help"], "--wait");
    assert_command_help_contains(&["unity", "record-input", "--help"], "--action");
    assert_command_help_contains(&["unity", "replay-input", "--help"], "--file");
    assert_command_help_contains(&["unity", "execute-dynamic-code", "--help"], "--code");
    assert_command_help_contains(&["unity", "execute-dynamic-code", "--help"], "--file");
    assert_command_help_contains(&["unity", "simulate-mouse-ui", "--help"], "--action");
    assert_command_help_contains(&["unity", "simulate-mouse-ui", "--help"], "--x");
    assert_command_help_contains(&["unity", "simulate-mouse-ui", "--help"], "--y");
    assert_command_help_contains(&["unity", "simulate-keyboard", "--help"], "--action");
    assert_command_help_contains(&["unity", "simulate-keyboard", "--help"], "--key");
    assert_command_help_contains(&["unity", "simulate-mouse-input", "--help"], "--button");
    assert_command_help_contains(&["unity", "simulate-mouse-input", "--help"], "--delta-x");
    assert_command_help_contains(&["unity", "simulate-mouse-input", "--help"], "--scroll-y");
    assert_command_help_contains(&["autonomous", "--help"], "dispatch");
    assert_command_help_contains(&["autonomous", "dry-run", "--help"], "--project-path");
    assert_command_help_contains(&["autonomous", "dispatch", "--help"], "--seq");
    assert_command_help_contains(&["autonomous", "evidence", "--help"], "--run-id");
}

#[test]
fn autonomous_cli_dry_run_non_mutating() {
    let project_root = temp_lux_project("autonomous-dry-run");
    let run_state_path = project_root.join(".lux").join("run-state.json");

    let mtime_before = run_state_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok());

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "autonomous",
            "dry-run",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux autonomous dry-run");

    assert_command_success(&output, "lux autonomous dry-run");

    let mtime_after = run_state_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok());

    assert_eq!(
        mtime_before, mtime_after,
        "dry-run must not modify run-state.json"
    );
}

#[test]
fn ai_log_recent_json_filters_fixture_jsonl() {
    let project_root = create_ai_log_fixture_project("lux-ai-log-recent");
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "recent",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--limit",
            "2",
            "--json",
            "--actor",
            "codex",
            "--category",
            "ai-action-log",
            "--source",
            "cli",
            "--action",
            "edit",
            "--event-type",
            "append",
        ])
        .output()
        .expect("run lux ai-log recent");

    assert_command_success(&output, "lux ai-log recent");
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("recent JSON");
    assert_eq!(parsed["count"], 2);
    let entries = parsed["entries"].as_array().expect("entries array");
    assert_eq!(entries[0]["value"]["summary"], "second codex edit");
    assert_eq!(entries[1]["value"]["summary"], "third codex edit");
}

#[test]
fn ai_log_context_json_builds_continuation_context() {
    let project_root = create_ai_log_fixture_project("lux-ai-log-context");
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "context",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--limit",
            "2",
            "--json",
        ])
        .output()
        .expect("run lux ai-log context");

    assert_command_success(&output, "lux ai-log context");
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("context JSON");
    assert_eq!(parsed["count"], 2);
    let entries = parsed["entries"].as_array().expect("entries array");
    assert_eq!(entries[0]["summary"], "third codex edit");
    assert_eq!(entries[1]["summary"], "opencode review");
}

#[test]
fn ai_log_compact_json_atomically_keeps_tail_valid_lines() {
    let project_root = create_ai_log_fixture_project("lux-ai-log-compact");
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "compact",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--max-lines",
            "2",
            "--json",
            "--yes",
        ])
        .output()
        .expect("run lux ai-log compact");

    assert_command_success(&output, "lux ai-log compact");
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("compact JSON");
    assert_eq!(parsed["validBefore"], 4);
    assert_eq!(parsed["validAfter"], 2);
    assert_eq!(parsed["invalidDropped"], 1);
    assert_eq!(parsed["linesDropped"], 3);

    let compacted = fs::read_to_string(project_root.join(".lux/ai-action-log.jsonl"))
        .expect("read compacted log");
    assert!(compacted.contains("third codex edit"));
    assert!(compacted.contains("opencode review"));
    assert!(!compacted.contains("first codex edit"));
    assert!(!project_root.join(".lux/ai-action-log.jsonl.tmp").exists());
}

#[test]
fn ai_log_tail_follow_prints_snapshot_and_exits() {
    let project_root = create_ai_log_fixture_project("lux-ai-log-tail");
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "tail",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--limit",
            "1",
            "--json",
            "--follow",
        ])
        .output()
        .expect("run lux ai-log tail --follow");

    assert_command_success(&output, "lux ai-log tail --follow");
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("tail JSON");
    assert_eq!(parsed["follow"], true);
    assert_eq!(parsed["count"], 1);
    assert_eq!(parsed["entries"][0]["value"]["summary"], "opencode review");
}

#[test]
fn lux_ai_log_work_step_writes_and_reads_back() {
    let project_root = create_temp_dir("lux-ai-log-work-step").join("Project");
    fs::create_dir_all(&project_root).expect("create project root");

    let write = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "work-step",
            "--name",
            "compile",
            "--status",
            "completed",
            "--tool",
            "opencode",
            "--action",
            "compile",
            "--summary",
            "cargo build completed",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux ai-log work-step");
    assert_command_success(&write, "lux ai-log work-step");

    let recent = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "ai-log",
            "recent",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--limit",
            "1",
            "--json",
        ])
        .output()
        .expect("run lux ai-log recent after work-step");
    assert_command_success(&recent, "lux ai-log recent after work-step");

    let parsed: Value = serde_json::from_slice(&recent.stdout).expect("recent JSON");
    assert_eq!(parsed["count"], 1);
    let value = &parsed["entries"][0]["value"];
    assert_eq!(value["stepName"], "compile");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["tool"], "opencode");
    assert_eq!(value["action"], "compile");
    assert_eq!(value["summary"], "cargo build completed");
}

#[test]
fn skill_list_shows_lux_unity() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "list"])
        .output()
        .expect("run lux skill list");

    assert!(
        output.status.success(),
        "lux skill list failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("lux-unity"));
}

#[test]
fn skill_list_json_is_parseable() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "list", "--json"])
        .output()
        .expect("run lux skill list --json");

    assert!(
        output.status.success(),
        "lux skill list --json failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("skill list JSON");
    assert!(parsed
        .as_array()
        .expect("skill list array")
        .iter()
        .any(|skill| skill["manifest"]["name"] == "lux-unity"));
}

#[test]
fn skill_info_shows_lux_unity() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "info", "lux-unity"])
        .output()
        .expect("run lux skill info lux-unity");

    assert!(
        output.status.success(),
        "lux skill info failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Name:") || stdout.contains("Version:"));
}

#[test]
fn skill_info_json_is_parseable() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "info", "lux-unity", "--json"])
        .output()
        .expect("run lux skill info lux-unity --json");

    assert!(
        output.status.success(),
        "lux skill info --json failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("skill info JSON");
    assert_eq!(parsed["manifest"]["name"], "lux-unity");
}

#[test]
fn skill_info_nonexistent_exits_1() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "info", "nonexistent-skill-xyz"])
        .output()
        .expect("run lux skill info nonexistent-skill-xyz");

    assert!(!output.status.success());
}

#[test]
fn skill_help_shows_subcommands() {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "--help"])
        .output()
        .expect("run lux skill --help");

    assert!(
        output.status.success(),
        "lux skill --help failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"));
    assert!(stdout.contains("info"));
}

#[test]
fn skill_install_creates_skill_in_global_scope() {
    let home = create_temp_dir("lux-skill-install-home");
    let source = create_test_skill_source("smoke-install-skill");

    let install = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-install-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
        ])
        .env("HOME", &home)
        .output()
        .expect("run lux skill install");

    assert_command_success(&install, "lux skill install");

    let list = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "list"])
        .env("HOME", &home)
        .output()
        .expect("run lux skill list");

    assert_command_success(&list, "lux skill list");
    assert!(String::from_utf8_lossy(&list.stdout).contains("smoke-install-skill"));
}

#[test]
fn skill_install_project_adapt_writes_metadata_and_info_json_reads_it() {
    let home = create_temp_dir("lux-skill-adapt-home");
    let project = create_test_unity_project("lux-skill-adapt-project", true);
    let source = create_test_skill_source("smoke-adapt-skill");

    let install = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-adapt-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --project --adapt --json");

    assert_command_success(&install, "lux skill install --adapt");
    let install_json: Value = serde_json::from_slice(&install.stdout).expect("install JSON");
    assert_eq!(install_json["installed"], true);
    assert_eq!(install_json["adapted"], true);
    assert_eq!(
        install_json["adaptation_metadata"]["protocol"],
        "lux.skill.adaptation.v1"
    );

    let installed_dir = project.join(".agents/skills/smoke-adapt-skill");
    assert!(installed_dir.join("manifest.json").is_file());
    assert!(installed_dir.join("SKILL.md").is_file());
    assert!(installed_dir.join("references/usage.md").is_file());
    let adaptation_path = installed_dir.join("lux-adaptation.json");
    assert!(adaptation_path.is_file());

    let info = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "info", "smoke-adapt-skill", "--json"])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill info --json");

    assert_command_success(&info, "lux skill info adapted");
    let info_json: Value = serde_json::from_slice(&info.stdout).expect("info JSON");
    assert_eq!(info_json["manifest"]["name"], "smoke-adapt-skill");
    assert_eq!(
        info_json["adaptation_metadata"]["skill_name"],
        "smoke-adapt-skill"
    );
}

#[test]
fn skill_install_adapt_requires_project_scope() {
    let home = create_temp_dir("lux-skill-adapt-requires-project-home");
    let project = create_test_unity_project("lux-skill-adapt-requires-project", true);
    let source = create_test_skill_source("smoke-adapt-requires-project-skill");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-adapt-requires-project-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --adapt without --project");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("error JSON");
    assert_eq!(parsed["installed"], false);
    assert!(parsed["error"]
        .as_str()
        .expect("error string")
        .contains("--adapt requires --project"));
}

#[test]
fn skill_install_adapt_incompatible_project_exits_1() {
    let home = create_temp_dir("lux-skill-adapt-incompatible-home");
    let not_unity_project = create_temp_dir("lux-skill-adapt-incompatible-project");
    let source = create_test_skill_source("smoke-adapt-incompatible-skill");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-adapt-incompatible-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&not_unity_project)
        .env("HOME", &home)
        .output()
        .expect("run incompatible lux skill install --adapt");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("error JSON");
    assert_eq!(parsed["installed"], false);
    assert!(parsed["error"]
        .as_str()
        .expect("error string")
        .contains("Unity project root"));
}

#[test]
fn skill_install_refuses_core_removal() {
    let home = create_temp_dir("lux-skill-core-remove-home");
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "remove", "lux-unity"])
        .env("HOME", &home)
        .output()
        .expect("run lux skill remove lux-unity");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn skill_remove_deletes_installed_skill() {
    let home = create_temp_dir("lux-skill-remove-home");
    let source = create_test_skill_source("smoke-remove-skill");

    let install = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-remove-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
        ])
        .env("HOME", &home)
        .output()
        .expect("run lux skill install");
    assert_command_success(&install, "lux skill install");

    let remove = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "remove", "smoke-remove-skill", "--global"])
        .env("HOME", &home)
        .output()
        .expect("run lux skill remove");
    assert_command_success(&remove, "lux skill remove");

    let list = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(["skill", "list"])
        .env("HOME", &home)
        .output()
        .expect("run lux skill list");
    assert_command_success(&list, "lux skill list");
    assert!(!String::from_utf8_lossy(&list.stdout).contains("smoke-remove-skill"));
}

#[test]
fn skill_install_refuses_duplicate() {
    let home = create_temp_dir("lux-skill-duplicate-home");
    let source = create_test_skill_source("smoke-duplicate-skill");

    let first = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-duplicate-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
        ])
        .env("HOME", &home)
        .output()
        .expect("run first lux skill install");
    assert_command_success(&first, "first lux skill install");

    let second = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-duplicate-skill",
            "--source",
            source.to_str().expect("source path UTF-8"),
        ])
        .env("HOME", &home)
        .output()
        .expect("run second lux skill install");

    assert!(!second.status.success());
    assert_eq!(second.status.code(), Some(1));
}

#[test]
fn rust_lux_unity_status_reads_lux_bridge_settings_without_external_settings() {
    let temp_dir = create_temp_dir("lux-unity-status");
    let project_root = temp_dir.join("Project");
    let user_settings = project_root.join("UserSettings");
    fs::create_dir_all(&user_settings).expect("create UserSettings dir");
    fs::write(
        user_settings.join("LuxBridgeSettings.json"),
        r#"{
  "schema_version": 1,
  "protocol": "lux.unity.bridge.v1",
  "package_name": "com.linalab.lux",
  "package_version": "0.1.0",
  "project_root": "/tmp/lux-project",
  "rust_gateway_path": "/tmp/lux/RustGateway~",
  "unity_server_port": null,
  "generated_at_utc": "2026-04-30T00:00:00Z"
}
"#,
    )
    .expect("write LuxBridgeSettings.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "status",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity status");

    assert!(
        output.status.success(),
        "lux unity status failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let status: Value = serde_json::from_slice(&output.stdout).expect("status JSON");
    assert_eq!(status["protocol"], "lux.unity.bridge.v1");
    assert_eq!(status["package_name"], "com.linalab.lux");
}

#[test]
fn rust_lux_unity_backend_list_commands_reads_protocol_info() {
    let temp_dir = create_temp_dir("lux-unity-backend-list-commands");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "get_protocol_info");
        assert_eq!(request["token"], TOKEN);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "protocolInfo": {
                    "protocolVersion": "1",
                    "backendVersion": "0.1.0",
                    "commands": [
                        "clear_lux_console",
                        "create_lux_scene_objects",
                        "get_lux_console_logs",
                        "run_lux_scene_smoke"
                    ]
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "backend-list-commands",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity backend-list-commands");

    assert!(
        output.status.success(),
        "lux unity backend-list-commands failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["backendVersion"], "0.1.0");
    assert_eq!(
        json["commands"],
        serde_json::json!([
            "clear_lux_console",
            "create_lux_scene_objects",
            "get_lux_console_logs",
            "run_lux_scene_smoke"
        ])
    );
    assert_eq!(json["capturedAtUtc"], "2026-04-30T00:00:00.0000000Z");

    server.join().expect("join fake Unity TCP server");
}

#[test]
fn rust_lux_unity_context_reads_shared_context_file() {
    let temp_dir = create_temp_dir("lux-unity-context");
    let project_root = temp_dir.join("Project");
    let user_settings = project_root.join("UserSettings");
    fs::create_dir_all(&user_settings).expect("create UserSettings dir");
    fs::write(
        user_settings.join("LuxUnityContext.json"),
        r#"{
  "schema_version": 1,
  "protocol": "lux.unity.context.v1",
  "generated_at_utc": "2026-04-30T00:00:00Z",
  "project_root": "/tmp/lux-project",
  "unity_version": "6000.3.0f1",
  "is_playing": false,
  "is_paused": false,
  "is_compiling": false,
  "active_scene_name": "GamePlay",
  "active_scene_path": "Assets/_Main/Scenes/GamePlay.unity",
  "selected_object_name": "Player",
  "selected_object_type": "UnityEngine.GameObject",
  "selected_asset_path": "",
  "selected_game_object_path": "GamePlay/Player",
  "console": { "errors": 0, "warnings": 1, "logs": 2, "recent": [] }
}
"#,
    )
    .expect("write LuxUnityContext.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "context",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity context");

    assert!(
        output.status.success(),
        "lux unity context failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let context: Value = serde_json::from_slice(&output.stdout).expect("context JSON");
    assert_eq!(context["protocol"], "lux.unity.context.v1");
    assert_eq!(context["active_scene_name"], "GamePlay");
    assert_eq!(context["console"]["warnings"], 1);
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_get_logs_returns_console_entries() {
    let temp_dir = create_temp_dir("lux-unity-get-logs");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "get_lux_console_logs");
        assert_eq!(request["token"], TOKEN);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "consoleLogs": {
                    "totalCount": 3,
                    "displayedCount": 2,
                    "consoleLogs": [
                        {
                            "level": "Log",
                            "message": "hello",
                            "stackTrace": "",
                            "timestampUtc": "2026-04-30T00:00:00.0000000Z"
                        },
                        {
                            "level": "Warning",
                            "message": "careful",
                            "stackTrace": "trace",
                            "timestampUtc": null
                        }
                    ]
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "get-logs",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity get-logs");

    assert!(
        output.status.success(),
        "lux unity get-logs failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["capturedAtUtc"], "2026-04-30T00:00:00.0000000Z");
    assert_eq!(json["totalCount"], 3);
    assert_eq!(json["displayedCount"], 2);
    assert_eq!(json["consoleLogs"][0]["level"], "Log");
    assert_eq!(
        json["consoleLogs"][1]["timestampUtc"],
        serde_json::Value::Null
    );

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_clear_console_returns_before_and_after_counts() {
    let temp_dir = create_temp_dir("lux-unity-clear-console");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "clear_lux_console");
        assert_eq!(request["token"], TOKEN);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "consoleClearResult": {
                    "beforeCount": 7,
                    "afterCount": 0
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "clear-console",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity clear-console");

    assert!(
        output.status.success(),
        "lux unity clear-console failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["capturedAtUtc"], "2026-04-30T00:00:00.0000000Z");
    assert_eq!(json["beforeCount"], 7);
    assert_eq!(json["afterCount"], 0);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_focus_window_returns_focused_true() {
    let temp_dir = create_temp_dir("lux-unity-focus-window");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "focus_lux_window");
        assert_eq!(request["token"], TOKEN);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "focusWindowResult": {
                    "focused": true
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "focus-window",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity focus-window");

    assert!(
        output.status.success(),
        "lux unity focus-window failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["capturedAtUtc"], "2026-04-30T00:00:00.0000000Z");
    assert_eq!(json["focused"], true);

    server.join().expect("join fake Unity TCP server");
}

#[test]
fn rust_lux_unity_get_hierarchy_returns_hierarchy_metadata() {
    let temp_dir = create_temp_dir("lux-unity-get-hierarchy");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "get_lux_hierarchy");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["hierarchyAll"], true);
        assert_eq!(request["params"]["hierarchyRootPath"], Value::Null);
        assert_eq!(request["params"]["hierarchyUseSelection"], false);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "getHierarchyResult": {
                    "filePath": "/tmp/.lux/outputs/hierarchy/request.json",
                    "fileSizeBytes": 512,
                    "rootCount": 2,
                    "nodeCount": 5,
                    "activeScene": {
                        "name": "GamePlay",
                        "path": "Assets/_Main/Scenes/GamePlay.unity"
                    },
                    "filters": {
                        "all": true,
                        "rootPath": "",
                        "useSelection": false
                    }
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "get-hierarchy",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux unity get-hierarchy");

    assert!(
        output.status.success(),
        "lux unity get-hierarchy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["filePath"], "/tmp/.lux/outputs/hierarchy/request.json");
    assert_eq!(json["fileSizeBytes"], 512);
    assert_eq!(json["rootCount"], 2);
    assert_eq!(json["nodeCount"], 5);
    assert_eq!(json["activeScene"]["name"], "GamePlay");
    assert_eq!(json["filters"]["all"], true);
    assert_eq!(json["filters"]["useSelection"], false);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_screenshot_returns_file_metadata_and_annotations() {
    let temp_dir = create_temp_dir("lux-unity-screenshot");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "capture_lux_screenshot");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["screenshotCaptureMode"], "rendering");
        assert_eq!(request["params"]["screenshotAnnotateElements"], true);
        assert_eq!(request["params"]["screenshotElementsOnly"], true);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "screenshotResult": {
                    "filePath": "",
                    "fileSizeBytes": 0,
                    "mediaType": "image/png",
                    "captureMode": "rendering",
                    "annotated": true,
                    "elementsOnly": true,
                    "screenshotSaved": false,
                    "annotationCount": 0,
                    "annotatedElements": []
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "screenshot",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--capture-mode",
            "rendering",
            "--annotate-elements",
            "--elements-only",
        ])
        .output()
        .expect("run lux unity screenshot");

    assert!(
        output.status.success(),
        "lux unity screenshot failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["filePath"], "");
    assert_eq!(json["fileSizeBytes"], 0);
    assert_eq!(json["mediaType"], "image/png");
    assert_eq!(json["captureMode"], "rendering");
    assert_eq!(json["elementsOnly"], true);
    assert_eq!(json["screenshotSaved"], false);
    assert_eq!(json["annotationCount"], 0);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_control_play_mode_sends_action_and_waits_for_match() {
    let temp_dir = create_temp_dir("lux-unity-control-play-mode");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        for (expected_action, is_playing, transition_requested) in
            [("play", false, true), ("status", true, false)]
        {
            let (mut stream, _) = listener.accept().expect("accept client");
            let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut request_line = String::new();
            request_reader
                .read_line(&mut request_line)
                .expect("read request line");
            let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
            assert_eq!(request["command"], "control_lux_play_mode");
            assert_eq!(request["token"], TOKEN);
            assert_eq!(request["params"]["playModeAction"], expected_action);

            let response = serde_json::json!({
                "schemaVersion": 1,
                "requestId": request["requestId"],
                "ok": true,
                "payload": {
                    "playModeState": {
                        "action": expected_action,
                        "isPlaying": is_playing,
                        "isPaused": false,
                        "transitionRequested": transition_requested
                    }
                },
                "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
            });

            stream
                .write_all(format!("{}\n", response).as_bytes())
                .expect("write response");
        }
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "control-play-mode",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "play",
            "--wait",
        ])
        .output()
        .expect("run lux unity control-play-mode");

    assert!(
        output.status.success(),
        "lux unity control-play-mode failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["action"], "play");
    assert_eq!(json["isPlaying"], true);
    assert_eq!(json["isPaused"], false);
    assert_eq!(json["transitionRequested"], false);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_simulate_keyboard_sends_press_key() {
    let temp_dir = create_temp_dir("lux-unity-simulate-keyboard");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "simulate_lux_keyboard");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["inputAction"], "press");
        assert_eq!(request["params"]["inputKey"], "Space");
        assert_eq!(request["params"]["inputDurationMs"], 75);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "inputSimulationResult": {
                    "device": "keyboard",
                    "action": "press",
                    "key": "Space",
                    "button": "",
                    "deltaX": 0.0,
                    "deltaY": 0.0,
                    "scrollX": 0.0,
                    "scrollY": 0.0,
                    "heldKeys": ["Space"],
                    "heldButtons": [],
                    "queuedActions": 1
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "simulate-keyboard",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "press",
            "--key",
            "Space",
            "--duration-ms",
            "75",
        ])
        .output()
        .expect("run lux unity simulate-keyboard");

    assert!(
        output.status.success(),
        "lux unity simulate-keyboard failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["device"], "keyboard");
    assert_eq!(json["action"], "press");
    assert_eq!(json["key"], "Space");
    assert_eq!(json["heldKeys"][0], "Space");
    assert_eq!(json["queuedActions"], 1);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_simulate_mouse_ui_sends_eventsystem_coordinates() {
    let temp_dir = create_temp_dir("lux-unity-simulate-mouse-ui");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "simulate_lux_mouse_ui");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["mouseUiAction"], "click");
        assert_eq!(request["params"]["mouseUiX"], 320.5);
        assert_eq!(request["params"]["mouseUiY"], 144.25);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "mouseUiResult": {
                    "action": "click",
                    "x": 320.5,
                    "y": 144.25,
                    "success": true,
                    "targetName": "StartButton",
                    "targetPath": "Canvas/StartButton",
                    "raycastCount": 1,
                    "dragActive": false
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "simulate-mouse-ui",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "click",
            "--x",
            "320.5",
            "--y",
            "144.25",
        ])
        .output()
        .expect("run lux unity simulate-mouse-ui");

    assert!(
        output.status.success(),
        "lux unity simulate-mouse-ui failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["action"], "click");
    assert_eq!(json["x"], 320.5);
    assert_eq!(json["y"], 144.25);
    assert_eq!(json["success"], true);
    assert_eq!(json["targetName"], "StartButton");
    assert_eq!(json["raycastCount"], 1);
    assert_eq!(json["dragActive"], false);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_simulate_mouse_input_sends_smooth_delta() {
    let temp_dir = create_temp_dir("lux-unity-simulate-mouse-input");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "simulate_lux_mouse_input");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["inputAction"], "smooth-delta");
        assert_eq!(request["params"]["inputDeltaX"], 12.0);
        assert_eq!(request["params"]["inputDeltaY"], -4.0);
        assert_eq!(request["params"]["inputSteps"], 3);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "inputSimulationResult": {
                    "device": "mouse",
                    "action": "smooth-delta",
                    "key": "",
                    "button": "",
                    "deltaX": 12.0,
                    "deltaY": -4.0,
                    "scrollX": 0.0,
                    "scrollY": 0.0,
                    "heldKeys": [],
                    "heldButtons": [],
                    "queuedActions": 3
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "simulate-mouse-input",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "smooth-delta",
            "--delta-x",
            "12",
            "--delta-y=-4",
            "--steps",
            "3",
        ])
        .output()
        .expect("run lux unity simulate-mouse-input");

    assert!(
        output.status.success(),
        "lux unity simulate-mouse-input failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["device"], "mouse");
    assert_eq!(json["action"], "smooth-delta");
    assert_eq!(json["deltaX"], 12.0);
    assert_eq!(json["deltaY"], -4.0);
    assert_eq!(json["queuedActions"], 3);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_record_input_sends_start_action_and_prints_artifact() {
    let temp_dir = create_temp_dir("lux-unity-record-input");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "record_lux_input");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["inputAction"], "start");

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "inputRecordResult": {
                    "action": "start",
                    "active": true,
                    "frameCount": 0,
                    "filePath": "",
                    "fileSizeBytes": 0,
                    "mediaType": "application/vnd.linalab.lux.input-recording+json",
                    "message": "Input recording started."
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "record-input",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "start",
        ])
        .output()
        .expect("run lux unity record-input");

    assert!(
        output.status.success(),
        "lux unity record-input failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["action"], "start");
    assert_eq!(json["active"], true);
    assert_eq!(
        json["mediaType"],
        "application/vnd.linalab.lux.input-recording+json"
    );

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_replay_input_sends_file_and_prints_status() {
    let temp_dir = create_temp_dir("lux-unity-replay-input");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");
    let recording_path = project_root.join(".lux/outputs/input-recordings/sample.json");

    let expected_path = recording_path.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "replay_lux_input");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["inputAction"], "start");
        assert_eq!(
            request["params"]["inputFilePath"],
            expected_path.to_string_lossy().to_string()
        );

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "inputReplayResult": {
                    "action": "start",
                    "active": true,
                    "filePath": expected_path.to_string_lossy().to_string(),
                    "frameCount": 12,
                    "replayedFrameCount": 0,
                    "completed": false,
                    "message": "Input replay is running."
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "replay-input",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--action",
            "start",
            "--file",
            recording_path.to_str().expect("recording path UTF-8"),
        ])
        .output()
        .expect("run lux unity replay-input");

    assert!(
        output.status.success(),
        "lux unity replay-input failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["action"], "start");
    assert_eq!(json["active"], true);
    assert_eq!(json["frameCount"], 12);
    assert_eq!(json["replayedFrameCount"], 0);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_execute_dynamic_code_loads_file_and_prints_result() {
    let temp_dir = create_temp_dir("lux-unity-execute-dynamic-code");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");
    let code_path = temp_dir.join("snippet.csx");
    let code = "Debug.Log(\"hello dynamic\");";
    fs::write(&code_path, code).expect("write dynamic code file");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "execute_lux_dynamic_code");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["dynamicCode"], code);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "dynamicCodeResult": {
                    "success": true,
                    "action": "compile_and_execute",
                    "result": "",
                    "resultType": "void",
                    "message": "Dynamic code executed.",
                    "diagnostics": [],
                    "logs": [
                        {
                            "level": "Log",
                            "message": "hello dynamic",
                            "stackTrace": "",
                            "timestampUtc": "2026-04-30T00:00:00.0000000Z"
                        }
                    ],
                    "elapsedTimeMs": 12
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "execute-dynamic-code",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--file",
            code_path.to_str().expect("code path UTF-8"),
        ])
        .output()
        .expect("run lux unity execute-dynamic-code");

    assert!(
        output.status.success(),
        "lux unity execute-dynamic-code failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "compile_and_execute");
    assert_eq!(json["resultType"], "void");
    assert_eq!(json["logs"][0]["message"], "hello dynamic");
    assert_eq!(json["elapsedTimeMs"], 12);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_execute_dynamic_code_sends_inline_code_and_prints_result() {
    let temp_dir = create_temp_dir("lux-unity-execute-dynamic-code-inline");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");
    let code = "return 42;";

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let discovery = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "token": TOKEN,
    });
    fs::write(bridge_dir.join("server.json"), discovery.to_string()).expect("write discovery");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "execute_lux_dynamic_code");
        assert_eq!(request["token"], TOKEN);
        assert_eq!(request["params"]["dynamicCode"], code);
        assert_eq!(request["params"]["actor"], "lux-cli");

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "dynamicCodeResult": {
                    "success": true,
                    "action": "compile_and_execute",
                    "result": "42",
                    "resultType": "System.Int32",
                    "message": "Dynamic code executed.",
                    "diagnostics": [],
                    "logs": [],
                    "elapsedTimeMs": 7
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "execute-dynamic-code",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--code",
            code,
        ])
        .output()
        .expect("run lux unity execute-dynamic-code --code");

    assert!(
        output.status.success(),
        "lux unity execute-dynamic-code --code failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "compile_and_execute");
    assert_eq!(json["result"], "42");
    assert_eq!(json["resultType"], "System.Int32");
    assert_eq!(json["elapsedTimeMs"], 7);

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_launch_waits_for_bridge_readiness() {
    let temp_dir = create_temp_dir("lux-unity-launch-wait");
    let project_root = temp_dir.join("Project");
    let bridge_dir = project_root.join("Library/UnityAiBridge");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::create_dir_all(&bridge_dir).expect("create Unity AI Bridge dir");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let args_capture = temp_dir.join("captured-launch-args.txt");
    let discovery_path = bridge_dir.join("server.json");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake Unity TCP server");
    let port = listener.local_addr().expect("read port").port();
    let token = TOKEN;

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request_reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        request_reader
            .read_line(&mut request_line)
            .expect("read request line");
        let request: Value = serde_json::from_str(request_line.trim()).expect("request JSON");
        assert_eq!(request["command"], "ping");
        assert_eq!(request["token"], token);

        let response = serde_json::json!({
            "schemaVersion": 1,
            "requestId": request["requestId"],
            "ok": true,
            "payload": {
                "ping": {
                    "status": "ok"
                }
            },
            "capturedAtUtc": "2026-04-30T00:00:00.0000000Z"
        });

        stream
            .write_all(format!("{}\n", response).as_bytes())
            .expect("write response");
    });

    let fake_unity = temp_dir.join("fake-unity-launch-wait.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\nsleep 0.25\necho \"$@\" > '{}'\ncat > '{}' <<'JSON'\n{{\"host\":\"127.0.0.1\",\"port\":{},\"token\":\"{}\"}}\nJSON\nexit 0\n",
            args_capture.display(),
            discovery_path.display(),
            port,
            token
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "launch",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux unity launch");
    let elapsed = started.elapsed();

    assert!(
        output.status.success(),
        "lux unity launch failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed >= Duration::from_millis(200),
        "expected launch to wait for bridge readiness, elapsed={elapsed:?}"
    );
    wait_for_path(&args_capture);
    let captured_args = fs::read_to_string(&args_capture).expect("read captured args");
    assert!(
        captured_args.contains("-projectPath"),
        "expected -projectPath in args: {captured_args}"
    );

    server.join().expect("join fake Unity TCP server");
}

#[test]
#[cfg_attr(
    not(feature = "integration"),
    ignore = "requires uloop passthrough and an external uloop binary"
)]
fn rust_lux_unity_launch_no_wait_returns_immediately() {
    let temp_dir = create_temp_dir("lux-unity-launch-no-wait");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let args_capture = temp_dir.join("captured-launch-no-wait-args.txt");
    let fake_unity = temp_dir.join("fake-unity-launch-no-wait.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\nsleep 0.5\necho \"$@\" > '{}'\nexit 0\n",
            args_capture.display()
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let started = Instant::now();
    let status = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "launch",
            "--no-wait",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .status()
        .expect("run lux unity launch --no-wait");
    let elapsed = started.elapsed();

    assert!(status.success());
    assert!(
        elapsed < Duration::from_secs(1),
        "expected --no-wait to return immediately, elapsed={elapsed:?}"
    );
    wait_for_path(&args_capture);
    let captured_args = fs::read_to_string(&args_capture).expect("read captured args");
    assert!(
        captured_args.contains("-projectPath"),
        "expected -projectPath in args: {captured_args}"
    );
}

#[test]
fn rust_lux_unity_context_refresh_launches_unity_batch_mode() {
    let temp_dir = create_temp_dir("lux-unity-context-refresh");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::create_dir_all(project_root.join("UserSettings")).expect("create UserSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let args_capture = temp_dir.join("captured-context-refresh-args.txt");
    let context_path = project_root.join("UserSettings/LuxUnityContext.json");
    let fake_unity = temp_dir.join("fake-unity-context-refresh.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\necho \"$@\" > '{}'\ncat > '{}' <<'JSON'\n{{\"schema_version\":1,\"protocol\":\"lux.unity.context.v1\",\"active_scene_name\":\"RefreshScene\",\"console\":{{\"errors\":0,\"warnings\":0,\"logs\":0,\"recent\":[]}}}}\nJSON\nexit 0\n",
            args_capture.display(),
            context_path.display()
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "unity",
            "context",
            "--refresh",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux unity context --refresh");

    assert!(
        output.status.success(),
        "lux unity context --refresh failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    wait_for_path(&args_capture);
    let captured_args = fs::read_to_string(&args_capture).expect("read captured args");
    assert!(
        captured_args.contains("Linalab.Lux.Editor.LuxUnityContext.Refresh"),
        "expected context refresh executeMethod in args: {captured_args}"
    );

    let context: Value = serde_json::from_slice(&output.stdout).expect("context JSON");
    assert_eq!(context["protocol"], "lux.unity.context.v1");
    assert_eq!(context["active_scene_name"], "RefreshScene");
}

#[test]
fn rust_lux_compile_launches_unity_batch_mode() {
    let temp_dir = create_temp_dir("lux-compile-batch");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let compile_result_path = project_root.join("TestResults/CompileResult.json");
    let fake_unity = temp_dir.join("fake-unity-compile.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\nmkdir -p '{}/TestResults'\necho '{{\"ok\":true}}' > '{}/TestResults/CompileResult.json'\nexit 0\n",
            project_root.display(),
            project_root.display(),
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "compile",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux compile in batch mode");

    assert!(
        output.status.success(),
        "lux compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"ok\""),
        "expected ok in stdout, got: {stdout}"
    );
    assert!(
        compile_result_path.exists(),
        "CompileResult.json should exist"
    );
}

#[test]
fn rust_lux_compile_reports_failure_when_unity_exits_nonzero() {
    let temp_dir = create_temp_dir("lux-compile-fail");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let fake_unity = temp_dir.join("fake-unity-fail.sh");
    fs::write(&fake_unity, "#!/bin/sh\nexit 1\n").expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "compile",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux compile expecting failure");

    assert!(
        !output.status.success(),
        "lux compile should fail when Unity exits nonzero"
    );
}

#[test]
fn rust_lux_compile_removes_stale_result_before_failed_unity_launch() {
    let temp_dir = create_temp_dir("lux-compile-stale-fail");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::create_dir_all(project_root.join("TestResults")).expect("create TestResults");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");
    fs::write(
        project_root.join("TestResults/CompileResult.json"),
        "{\"ok\":true,\"message\":\"stale success\"}",
    )
    .expect("write stale compile result");

    let fake_unity = temp_dir.join("fake-unity-stale-fail.sh");
    fs::write(&fake_unity, "#!/bin/sh\nexit 1\n").expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "compile",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux compile expecting stale-result failure");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "lux compile should fail when Unity exits nonzero"
    );
    assert!(
        stdout.contains("\"ok\": false"),
        "expected truthful failure JSON, got: {stdout}"
    );
    assert!(
        !stdout.contains("stale success"),
        "stale CompileResult.json leaked into stdout: {stdout}"
    );
}

#[test]
fn rust_lux_run_tests_launches_unity_batch_mode() {
    let temp_dir = create_temp_dir("lux-run-tests-batch");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let args_capture = temp_dir.join("captured-args.txt");
    let fake_unity = temp_dir.join("fake-unity-tests.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\necho \"$@\" > '{}'\nexit 0\n",
            args_capture.display()
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "run-tests",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--test-platform",
            "EditMode",
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux run-tests in batch mode");

    assert!(
        output.status.success(),
        "lux run-tests failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    wait_for_path(&args_capture);
    let captured_args = fs::read_to_string(&args_capture).expect("read captured args");
    assert!(
        captured_args.contains("-runTests"),
        "expected -runTests in args: {captured_args}"
    );
    assert!(
        captured_args.contains("-batchmode"),
        "expected -batchmode in args: {captured_args}"
    );
    assert!(
        captured_args.contains("-testPlatform"),
        "expected -testPlatform in args: {captured_args}"
    );
    assert!(
        captured_args.contains("EditMode"),
        "expected EditMode in args: {captured_args}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"ok\""),
        "expected ok in stdout, got: {stdout}"
    );
    assert!(
        stdout.contains("EditMode"),
        "expected EditMode in stdout, got: {stdout}"
    );
}

#[test]
fn rust_lux_run_tests_with_playmode_platform() {
    let temp_dir = create_temp_dir("lux-run-tests-playmode");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.3.0f1\n",
    )
    .expect("write project version");

    let args_capture = temp_dir.join("captured-args-playmode.txt");
    let fake_unity = temp_dir.join("fake-unity-playmode.sh");
    fs::write(
        &fake_unity,
        format!(
            "#!/bin/sh\necho \"$@\" > '{}'\nexit 0\n",
            args_capture.display()
        ),
    )
    .expect("write fake Unity");
    make_executable(&fake_unity);

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "run-tests",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
            "--test-platform",
            "PlayMode",
        ])
        .env("LUX_UNITY_EDITOR", &fake_unity)
        .output()
        .expect("run lux run-tests PlayMode");

    assert!(output.status.success());
    wait_for_path(&args_capture);
    let captured_args = fs::read_to_string(&args_capture).expect("read captured args");
    assert!(
        captured_args.contains("PlayMode"),
        "expected PlayMode in args: {captured_args}"
    );
}

#[test]
fn rust_lux_auto_detects_unity_path_from_project_version() {
    let temp_dir = create_temp_dir("lux-auto-detect");
    let project_root = temp_dir.join("Project");
    fs::create_dir_all(project_root.join("Assets")).expect("create Assets");
    fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 99999.0.0f1\n",
    )
    .expect("write project version");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "compile",
            "--project-path",
            project_root.to_str().expect("project path UTF-8"),
        ])
        .output()
        .expect("run lux compile with non-existent Unity");

    assert!(!output.status.success(), "should fail when Unity not found");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("99999.0.0f1") || stderr.contains("LUX_UNITY_EDITOR"),
        "should mention version or LUX_UNITY_EDITOR env, got: {stderr}"
    );
}

fn assert_command_help_contains(args: &[&str], expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args(args)
        .output()
        .expect("run lux help command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output is UTF-8");
    assert!(
        stdout.contains(expected),
        "expected help output to contain {expected:?}, got:\n{stdout}"
    );
}

fn assert_command_success(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_ai_log_fixture_project(prefix: &str) -> std::path::PathBuf {
    let project_root = create_temp_dir(prefix).join("Project");
    let lux_directory = project_root.join(".lux");
    fs::create_dir_all(&lux_directory).expect("create .lux");
    fs::write(
        lux_directory.join("ai-action-log.jsonl"),
        concat!(
            "{\"timestampUtc\":\"2026-05-04T00:00:01Z\",\"actor\":\"codex\",\"category\":\"ai-action-log\",\"source\":\"cli\",\"action\":\"edit\",\"eventType\":\"append\",\"summary\":\"first codex edit\"}\n",
            "not-json\n",
            "{\"timestampUtc\":\"2026-05-04T00:00:02Z\",\"actor\":\"codex\",\"category\":\"ai-action-log\",\"source\":\"cli\",\"action\":\"edit\",\"eventType\":\"append\",\"summary\":\"second codex edit\"}\n",
            "{\"timestampUtc\":\"2026-05-04T00:00:03Z\",\"actor\":\"codex\",\"category\":\"ai-action-log\",\"source\":\"cli\",\"action\":\"edit\",\"eventType\":\"append\",\"summary\":\"third codex edit\"}\n",
            "{\"timestampUtc\":\"2026-05-04T00:00:04Z\",\"actor\":\"opencode\",\"category\":\"review\",\"source\":\"cli\",\"action\":\"qa\",\"eventType\":\"complete\",\"summary\":\"opencode review\"}\n",
        ),
    )
    .expect("write AI log fixture");
    project_root
}

fn create_test_skill_source(name: &str) -> std::path::PathBuf {
    let source = create_temp_dir(&format!("lux-skill-source-{name}"));
    fs::write(
        source.join("manifest.json"),
        format!(
            r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "description": "Smoke test skill",
  "displayName": "Smoke Test Skill",
  "luxVersion": "0.1.0",
  "author": {{ "name": "Lux Tests" }},
  "keywords": ["smoke"],
  "type": "test",
  "source": "{}"
}}"#,
            source.display()
        ),
    )
    .expect("write test skill manifest");
    fs::write(
        source.join("SKILL.md"),
        format!("# {name}\n\nSmoke test skill.\n"),
    )
    .expect("write test skill body");

    let references = source.join("references");
    fs::create_dir_all(&references).expect("create references dir");
    fs::write(references.join("usage.md"), "# Usage\n").expect("write reference");

    source
}

fn create_test_unity_project(prefix: &str, include_lux_package: bool) -> std::path::PathBuf {
    let project = create_temp_dir(prefix);
    fs::create_dir_all(project.join("Assets")).expect("create Assets dir");
    fs::create_dir_all(project.join("ProjectSettings")).expect("create ProjectSettings dir");
    fs::create_dir_all(project.join("Packages")).expect("create Packages dir");
    fs::write(
        project.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.20f1\nm_EditorVersionWithRevision: 2022.3.20f1\n",
    )
    .expect("write ProjectVersion.txt");
    let dependencies = if include_lux_package {
        r#""com.linalab.lux": "file:Packages/com.linalab.lux""#
    } else {
        r#""com.unity.modules.ai": "1.0.0""#
    };
    fs::write(
        project.join("Packages/manifest.json"),
        format!(
            r#"{{
  "dependencies": {{
    {dependencies}
  }}
}}
"#
        ),
    )
    .expect("write Packages/manifest.json");
    project
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path).expect("read metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("set executable");
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn reserve_local_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("reserve local port")
        .local_addr()
        .expect("read local addr")
        .port()
}

fn create_temp_dir(prefix: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        reserve_local_port()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    panic!("expected path to be created: {}", path.display());
}

fn start_gateway(port: u16) -> GatewayProcess {
    let child = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "serve",
            "--token",
            TOKEN,
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start lux test server");

    GatewayProcess { child }
}

fn wait_for_health(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if http_status(port, "/health") == Some(200) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }

    panic!("lux did not become healthy on port {port}");
}

fn http_status(port: u16, path: &str) -> Option<u16> {
    request_status(port, path, &[])
}

fn websocket_status(port: u16, path: &str, headers: &[(&str, &str)]) -> u16 {
    let mut websocket_headers = vec![
        ("Connection", "Upgrade"),
        ("Upgrade", "websocket"),
        ("Sec-WebSocket-Version", "13"),
        ("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ=="),
    ];
    websocket_headers.extend_from_slice(headers);

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = request_status(port, path, &websocket_headers) {
            return status;
        }
        thread::sleep(Duration::from_millis(50));
    }

    panic!("read WebSocket response status")
}

fn request_status(port: u16, path: &str, headers: &[(&str, &str)]) -> Option<u16> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set read timeout");

    let mut request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes()).ok()?;

    let mut response = Vec::new();
    let mut chunk = [0_u8; 512];
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(size) => {
                response.extend_from_slice(&chunk[..size]);
                if response.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return None,
        }
    }

    let response = std::str::from_utf8(&response).ok()?;
    response
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

// ---------------------------------------------------------------------------
// Phase 6 AC7: Project-adapting skill smoke tests
// ---------------------------------------------------------------------------

/// Helper: create a Unity project with URP and ProjectVersion.txt
fn create_test_unity_project_with_urp(prefix: &str) -> std::path::PathBuf {
    let project = create_temp_dir(prefix);
    fs::create_dir_all(project.join("Assets")).expect("create Assets dir");
    fs::create_dir_all(project.join("ProjectSettings")).expect("create ProjectSettings dir");
    fs::create_dir_all(project.join("Packages")).expect("create Packages dir");

    // ProjectVersion.txt for Unity version detection
    fs::write(
        project.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.20f1\nm_EditorVersionWithRevision: 2022.3.20f1\n",
    )
    .expect("write ProjectVersion.txt");

    // Packages/manifest.json with URP + LUX
    let manifest = serde_json::json!({
        "dependencies": {
            "com.unity.render-pipelines.universal": "14.0.8",
            "com.unity.modules.ui": "1.0.0",
            "com.linalab.lux": "file:Packages/com.linalab.lux",
            "com.unity.textmeshpro": "3.0.6"
        }
    });
    fs::write(
        project.join("Packages/manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write Packages/manifest.json");

    project
}

/// Helper: create a skill source with compatibility + slimming metadata
fn create_test_skill_source_with_compatibility(name: &str) -> std::path::PathBuf {
    let source = create_temp_dir(&format!("lux-skill-compat-{name}"));
    let manifest = serde_json::json!({
        "name": name,
        "version": "0.2.0",
        "description": "Compatibility test skill",
        "displayName": "Compat Test Skill",
        "luxVersion": "0.1.0",
        "author": { "name": "Lux Tests" },
        "keywords": ["compat"],
        "type": "test",
        "source": source.display().to_string(),
        "requiredPackages": ["com.unity.modules.ui"],
        "compatibleRenderPipelines": ["urp"],
        "contextSlimRules": {
            "maxReferences": 5,
            "maxSkillMdLines": 100,
            "excludeTags": ["hdrp-only", "advanced"]
        }
    });
    fs::write(
        source.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write compat skill manifest");

    // SKILL.md with known line count
    let skill_md = "# Compat Skill\n\nLine 3.\nLine 4.\nLine 5.\n";
    fs::write(source.join("SKILL.md"), skill_md).expect("write SKILL.md");

    // references with tagged files
    let references = source.join("references");
    fs::create_dir_all(&references).expect("create references dir");
    fs::write(references.join("usage.md"), "# Usage\n").expect("write usage ref");
    fs::write(references.join("getting-started.md"), "# Getting Started\n")
        .expect("write getting-started ref");
    fs::write(
        references.join("hdrp-only-post-processing.md"),
        "# HDRP Post FX\n",
    )
    .expect("write hdrp-only ref");
    fs::write(
        references.join("advanced-shaders.md"),
        "# Advanced Shaders\n",
    )
    .expect("write advanced ref");

    source
}

/// Helper: create a skill source that requires HDRP (incompatible with URP project)
fn create_test_skill_source_hdrp_only(name: &str) -> std::path::PathBuf {
    let source = create_temp_dir(&format!("lux-skill-hdrp-{name}"));
    let manifest = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "description": "HDRP-only test skill",
        "displayName": "HDRP Test Skill",
        "luxVersion": "0.1.0",
        "author": { "name": "Lux Tests" },
        "keywords": ["hdrp"],
        "type": "test",
        "source": source.display().to_string(),
        "compatibleRenderPipelines": ["hdrp"]
    });
    fs::write(
        source.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write hdrp skill manifest");
    fs::write(source.join("SKILL.md"), "# HDRP Skill\n\nHDRP only.\n").expect("write SKILL.md");
    let references = source.join("references");
    fs::create_dir_all(&references).expect("create references dir");
    fs::write(references.join("usage.md"), "# Usage\n").expect("write usage ref");
    source
}

/// Helper: create a skill source that requires a missing package
fn create_test_skill_source_missing_pkg(name: &str) -> std::path::PathBuf {
    let source = create_temp_dir(&format!("lux-skill-misspkg-{name}"));
    let manifest = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "description": "Missing package test skill",
        "displayName": "Missing Pkg Skill",
        "luxVersion": "0.1.0",
        "author": { "name": "Lux Tests" },
        "keywords": ["missing"],
        "type": "test",
        "source": source.display().to_string(),
        "requiredPackages": ["com.unity.does-not-exist", "com.unity.render-pipelines.universal"]
    });
    fs::write(
        source.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write missing-pkg skill manifest");
    fs::write(
        source.join("SKILL.md"),
        "# Missing Pkg Skill\n\nNeeds phantom package.\n",
    )
    .expect("write SKILL.md");
    let references = source.join("references");
    fs::create_dir_all(&references).expect("create references dir");
    fs::write(references.join("usage.md"), "# Usage\n").expect("write usage ref");
    source
}

#[test]
fn skill_install_adapt_detects_project_metadata() {
    let home = create_temp_dir("lux-adapt-meta-home");
    let project = create_test_unity_project_with_urp("lux-adapt-meta-project");
    let source = create_test_skill_source_with_compatibility("smoke-meta-skill");

    let install = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-meta-skill",
            "--source",
            source.to_str().expect("source path"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --adapt with URP project");

    assert_command_success(&install, "lux skill install --adapt metadata");
    let json: Value = serde_json::from_slice(&install.stdout).expect("install JSON");

    // Verify adaptation metadata
    assert_eq!(json["adapted"], true);

    // Verify project metadata detection
    let pm = &json["adaptation_metadata"]["project_metadata"];
    assert_eq!(
        pm["unity_version"].as_str().expect("unity version"),
        "2022.3.20f1"
    );
    assert_eq!(
        pm["render_pipeline"].as_str().expect("render pipeline"),
        "urp"
    );
    assert!(pm["has_lux_package"].as_bool().expect("has lux"));

    // Verify URP package was detected in installed_packages
    let installed = pm["installed_packages"]
        .as_array()
        .expect("installed packages");
    let pkg_names: Vec<&str> = installed.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        pkg_names.contains(&"com.unity.render-pipelines.universal"),
        "URP package should be detected: {pkg_names:?}"
    );
}

#[test]
fn skill_install_adapt_context_slimming_filters_references() {
    let home = create_temp_dir("lux-adapt-slim-home");
    let project = create_test_unity_project_with_urp("lux-adapt-slim-project");
    let source = create_test_skill_source_with_compatibility("smoke-slim-skill");

    let install = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-slim-skill",
            "--source",
            source.to_str().expect("source path"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --adapt slimming");

    assert_command_success(&install, "lux skill install --adapt slimming");
    let json: Value = serde_json::from_slice(&install.stdout).expect("install JSON");
    let slim = &json["adaptation_metadata"]["context_slimming"];

    // 4 references total: usage.md, getting-started.md, hdrp-only-post-processing.md, advanced-shaders.md
    assert_eq!(slim["totalReferences"], 4);

    // hdrp-only-post-processing.md and advanced-shaders.md should be excluded
    let excluded = slim["excludedReferences"]
        .as_array()
        .expect("excluded refs");
    let excluded_names: Vec<&str> = excluded.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        excluded_names.contains(&"hdrp-only-post-processing.md"),
        "hdrp-only reference should be excluded: {excluded_names:?}"
    );
    assert!(
        excluded_names.contains(&"advanced-shaders.md"),
        "advanced reference should be excluded: {excluded_names:?}"
    );

    // usage.md and getting-started.md should remain included
    let included = slim["includedReferences"]
        .as_array()
        .expect("included refs");
    let included_names: Vec<&str> = included.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        included_names.contains(&"usage.md"),
        "usage.md should be included: {included_names:?}"
    );
    assert!(
        included_names.contains(&"getting-started.md"),
        "getting-started.md should be included: {included_names:?}"
    );

    // slimmed_references = 2 (only usage + getting-started after filtering)
    assert_eq!(slim["slimmedReferences"], 2);
}

#[test]
fn skill_install_adapt_render_pipeline_incompatible_exits_1() {
    let home = create_temp_dir("lux-adapt-pipeline-home");
    let project = create_test_unity_project_with_urp("lux-adapt-pipeline-project");
    let source = create_test_skill_source_hdrp_only("smoke-hdrp-skill");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-hdrp-skill",
            "--source",
            source.to_str().expect("source path"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --adapt pipeline incompatible");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let parsed: Value = serde_json::from_slice(&output.stdout).expect("error JSON");
    assert_eq!(parsed["installed"], false);
    let error_msg = parsed["error"].as_str().expect("error string");
    assert!(
        error_msg.contains("incompatible"),
        "error should mention incompatibility: {error_msg}"
    );
    assert!(
        error_msg.contains("render pipeline") || error_msg.contains("hdrp"),
        "error should mention render pipeline mismatch: {error_msg}"
    );
}

#[test]
fn skill_install_adapt_missing_required_packages_exits_1() {
    let home = create_temp_dir("lux-adapt-misspkg-home");
    let project = create_test_unity_project_with_urp("lux-adapt-misspkg-project");
    let source = create_test_skill_source_missing_pkg("smoke-misspkg-skill");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "skill",
            "install",
            "smoke-misspkg-skill",
            "--source",
            source.to_str().expect("source path"),
            "--project",
            "--adapt",
            "--json",
        ])
        .current_dir(&project)
        .env("HOME", &home)
        .output()
        .expect("run lux skill install --adapt missing package");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let parsed: Value = serde_json::from_slice(&output.stdout).expect("error JSON");
    assert_eq!(parsed["installed"], false);
    let error_msg = parsed["error"].as_str().expect("error string");
    assert!(
        error_msg.contains("missing required packages"),
        "error should mention missing packages: {error_msg}"
    );
    assert!(
        error_msg.contains("com.unity.does-not-exist"),
        "error should name the missing package: {error_msg}"
    );
}

fn create_session_project(prefix: &str) -> std::path::PathBuf {
    let project_root = create_temp_dir(prefix);
    let assets = project_root.join("Assets");
    let settings = project_root.join("ProjectSettings");
    fs::create_dir_all(&assets).expect("create Assets");
    fs::create_dir_all(&settings).expect("create ProjectSettings");
    project_root
}

fn append_test_event(project_root: &Path, session_id: &str) {
    let sessions_dir = project_root.join(".lux").join("sessions");
    let session_file = sessions_dir.join(format!("{session_id}.jsonl"));
    let event = serde_json::json!({
        "recordType": "session_event",
        "event": {
            "timestampUtc": "2026-05-06T12:00:00Z",
            "eventType": "test-event",
            "category": "test",
            "source": "smoke",
            "summary": "smoke test event",
            "payload": {}
        }
    });
    fs::OpenOptions::new()
        .append(true)
        .open(&session_file)
        .and_then(|mut f| writeln!(f, "{}", event))
        .expect("append test event");
}

#[test]
fn session_help_shows_subcommands() {
    assert_command_help_contains(&["session", "--help"], "record");
    assert_command_help_contains(&["session", "--help"], "stop");
    assert_command_help_contains(&["session", "--help"], "replay");
    assert_command_help_contains(&["session", "--help"], "timeline");
    assert_command_help_contains(&["session", "--help"], "report");
}

#[test]
fn session_record_and_stop_cycle() {
    let project = create_session_project("lux-session-record-stop");

    let record_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "record",
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session record");
    assert_command_success(&record_output, "session record");

    let parsed: Value = serde_json::from_slice(&record_output.stdout).expect("record JSON");
    let session_id = parsed["sessionId"].as_str().expect("session id");
    assert!(!session_id.is_empty());

    let jsonl = project
        .join(".lux")
        .join("sessions")
        .join(format!("{session_id}.jsonl"));
    assert!(jsonl.exists(), "session JSONL should exist");

    let stop_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "stop",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session stop");
    assert_command_success(&stop_output, "session stop");

    let stop_parsed: Value = serde_json::from_slice(&stop_output.stdout).expect("stop JSON");
    assert_eq!(stop_parsed["ok"], true);
    assert_eq!(stop_parsed["eventCount"], 0);
}

#[test]
fn session_timeline_json_output() {
    let project = create_session_project("lux-session-timeline");

    let record_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "record",
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session record");
    assert_command_success(&record_output, "session record");

    let parsed: Value = serde_json::from_slice(&record_output.stdout).expect("record JSON");
    let session_id = parsed["sessionId"].as_str().expect("session id");

    append_test_event(&project, session_id);

    let stop_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "stop",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session stop");
    assert_command_success(&stop_output, "session stop");

    let timeline_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "timeline",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run lux session timeline");
    assert_command_success(&timeline_output, "session timeline");

    let timeline: Value = serde_json::from_slice(&timeline_output.stdout).expect("timeline JSON");
    assert_eq!(timeline["sessionId"].as_str(), Some(session_id));
    let events = timeline["events"].as_array().expect("events array");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["eventType"], "test-event");
}

#[test]
fn session_report_json_output() {
    let project = create_session_project("lux-session-report");

    let record_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "record",
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session record");
    assert_command_success(&record_output, "session record");

    let parsed: Value = serde_json::from_slice(&record_output.stdout).expect("record JSON");
    let session_id = parsed["sessionId"].as_str().expect("session id");

    append_test_event(&project, session_id);

    let stop_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "stop",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session stop");
    assert_command_success(&stop_output, "session stop");

    let report_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "report",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run lux session report");
    assert_command_success(&report_output, "session report");

    let report: Value = serde_json::from_slice(&report_output.stdout).expect("report JSON");
    assert_eq!(report["sessionId"].as_str(), Some(session_id));
    assert_eq!(report["totalEvents"], 1);
    assert_eq!(report["errorCount"], 0);
}

#[test]
fn session_full_replay_cycle() {
    let project = create_session_project("lux-session-replay");

    let record_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "record",
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session record");
    assert_command_success(&record_output, "session record");

    let parsed: Value = serde_json::from_slice(&record_output.stdout).expect("record JSON");
    let session_id = parsed["sessionId"].as_str().expect("session id");

    append_test_event(&project, session_id);

    let stop_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "stop",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
        ])
        .output()
        .expect("run lux session stop");
    assert_command_success(&stop_output, "session stop");

    let replay_output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "session",
            "replay",
            "--session-id",
            session_id,
            "--project-path",
            project.to_str().unwrap(),
            "--speed",
            "1000",
        ])
        .output()
        .expect("run lux session replay");
    assert_command_success(&replay_output, "session replay");

    let stdout = String::from_utf8(replay_output.stdout).expect("replay output");
    let json_start = stdout.find('{').expect("replay JSON start");
    let replay: Value = serde_json::from_str(&stdout[json_start..]).expect("replay JSON");
    assert_eq!(replay["totalEvents"], 1);
    assert_eq!(replay["replayedEvents"], 1);
    assert_eq!(replay["errors"].as_array().map(|a| a.len()), Some(0));
}

fn run_mcp_jsonl(project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "mcp",
            "--project-path",
            project.to_str().expect("project path"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn lux mcp");

    {
        let stdin = child.stdin.as_mut().expect("mcp stdin");
        for request in requests {
            writeln!(stdin, "{}", request).expect("write MCP request");
        }
    }

    let output = child.wait_with_output().expect("wait lux mcp");
    assert!(
        output.status.success(),
        "lux mcp failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("MCP response JSON"))
        .collect()
}

#[test]
fn mcp_lists_game_dev_loop_tools_with_structured_contract() {
    let project = create_session_project("lux-mcp-list-game-dev");
    let responses = run_mcp_jsonl(
        &project,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ],
    );
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "lux");
    let tools = responses[1]["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for expected in [
        "lux_bridge_install",
        "lux_bridge_diagnostics",
        "lux_game_spec_write",
        "lux_game_ticket_prepare",
        "lux_unity_maneuver",
        "lux_game_dev_loop_once",
    ] {
        assert!(
            names.contains(&expected),
            "missing {expected}; got {names:?}"
        );
    }
    let loop_tool = tools
        .iter()
        .find(|tool| tool["name"] == "lux_game_dev_loop_once")
        .expect("loop tool");
    assert_eq!(
        loop_tool["inputSchema"]["additionalProperties"],
        serde_json::json!(false)
    );
}

#[test]
fn mcp_loop_once_returns_steps_stop_reason_and_preserves_ping_after_blocker() {
    let project = create_session_project("lux-mcp-loop-once");
    let responses = run_mcp_jsonl(
        &project,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            serde_json::json!({
                "jsonrpc":"2.0",
                "id":2,
                "method":"tools/call",
                "params":{
                    "name":"lux_game_dev_loop_once",
                    "arguments":{
                        "project_path": project,
                        "objective":"Create one safe smoke-test game-dev change"
                    }
                }
            }),
            serde_json::json!({"jsonrpc":"2.0","id":3,"method":"ping","params":{}}),
        ],
    );

    let call = &responses[1]["result"];
    assert_eq!(call["isError"], true);
    let structured = &call["structuredContent"];
    assert_eq!(structured["protocol"], "lux.game_dev_loop_once.v1");
    assert_eq!(structured["stopReason"], "unity_bridge_unavailable");
    let steps = structured["steps"].as_array().expect("steps");
    assert!(
        steps
            .iter()
            .any(|step| step["name"] == "spec_write" && step["status"] == "ok"),
        "spec write step missing: {steps:?}"
    );
    assert!(
        steps
            .iter()
            .any(|step| step["name"] == "ticket_prepare" && step["status"] == "ok"),
        "ticket prepare step missing: {steps:?}"
    );
    assert!(
        steps
            .iter()
            .any(|step| step["name"] == "unity_maneuver" && step["status"] == "error"),
        "unity maneuver blocker step missing: {steps:?}"
    );
    assert!(project.join(".lux/spec.json").is_file());
    assert!(project.join(".lux/evidence").is_dir());
    assert_eq!(responses[2]["result"], serde_json::json!({}));
}

fn temp_lux_project(name: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("lux-smoke-{name}-{nanos}"));
    fs::create_dir_all(root.join(".lux")).unwrap();
    root
}

fn run_mcp_jsonl(project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "mcp",
            "--project-path",
            project.to_str().expect("project path UTF-8"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lux mcp");

    {
        let stdin = child.stdin.as_mut().expect("mcp stdin");
        for request in requests {
            writeln!(
                stdin,
                "{}",
                serde_json::to_string(request).expect("request JSON")
            )
            .expect("write MCP request");
        }
    }

    let output = child.wait_with_output().expect("wait for lux mcp");
    assert_command_success(&output, "lux mcp jsonl");
    String::from_utf8(output.stdout)
        .expect("mcp stdout UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("mcp response JSON"))
        .collect()
}

#[test]
fn mcp_stdio_initializes_and_lists_bridge_and_game_dev_tools_without_unity() {
    let project = create_test_unity_project("lux-mcp-list", false);
    let responses = run_mcp_jsonl(
        &project,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ],
    );

    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "lux");
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools array");
    let tool_names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<std::collections::HashSet<_>>();
    for expected in [
        "lux_bridge_install",
        "lux_bridge_diagnostics",
        "lux_game_spec_write",
        "lux_game_ticket_prepare",
        "lux_unity_maneuver",
        "lux_game_dev_loop_once",
    ] {
        assert!(tool_names.contains(expected), "missing MCP tool {expected}");
    }
    for tool in tools {
        assert!(tool["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
}

#[test]
fn mcp_spec_and_ticket_tools_are_idempotent_and_persist_lux_state() {
    let project = create_test_unity_project("lux-mcp-idempotent", false);
    let spec_args = json!({
        "project_name": "MCP Idempotent Game",
        "seed": {
            "game_title": "MCP Idempotent Game",
            "genre": "puzzle",
            "elevator_pitch": "One deterministic MCP loop smoke."
        }
    });
    let ticket_args = json!({
        "objective": "Create one safe deterministic MCP smoke ticket.",
        "verification_policy": "cargo_or_explicit_unavailable",
        "non_goals": ["destructive rewrites", "Unity Editor window UI"]
    });

    let responses = run_mcp_jsonl(
        &project,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lux_game_spec_write","arguments":spec_args.clone()}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lux_game_spec_write","arguments":spec_args}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"lux_game_ticket_prepare","arguments":ticket_args.clone()}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"lux_game_ticket_prepare","arguments":ticket_args}}),
        ],
    );

    assert_eq!(responses.len(), 4);
    let first_spec = &responses[0]["result"]["structuredContent"];
    let second_spec = &responses[1]["result"]["structuredContent"];
    assert_eq!(first_spec["ok"], true);
    assert_eq!(second_spec["ok"], true);
    assert_eq!(first_spec["changed"], true);
    assert_eq!(second_spec["changed"], false);
    assert_eq!(second_spec["projectName"], "MCP Idempotent Game");
    assert_eq!(second_spec["source"], "lux-mcp");

    let first_ticket = &responses[2]["result"]["structuredContent"];
    let second_ticket = &responses[3]["result"]["structuredContent"];
    assert_eq!(first_ticket["ok"], true);
    assert_eq!(second_ticket["ok"], true);
    assert_eq!(first_ticket["created"], true);
    assert_eq!(second_ticket["created"], false);
    assert_eq!(first_ticket["ticketId"], second_ticket["ticketId"]);

    let spec_path = project.join(".lux/spec.json");
    let spec: Value = serde_json::from_str(&fs::read_to_string(&spec_path).expect("spec file"))
        .expect("spec JSON");
    assert_eq!(spec["project_name"], "MCP Idempotent Game");
    assert_eq!(spec["source"], "lux-mcp");
    assert_eq!(spec["meta"]["genre"], "puzzle");

    let ticket_path = project.join(".lux/tickets/game-dev-loop-001.json");
    let ticket: Value =
        serde_json::from_str(&fs::read_to_string(&ticket_path).expect("ticket file"))
            .expect("ticket JSON");
    assert_eq!(ticket["id"], "game-dev-loop-001");
    assert_eq!(ticket["spec_ref"], ".lux/spec.json");
    assert_eq!(
        ticket["verification_policy"],
        "cargo_or_explicit_unavailable"
    );
    assert_eq!(ticket["dispatch_policy"], "manual");
}

#[test]
fn mcp_loop_once_failure_returns_structured_steps_evidence_and_keeps_ping_alive() {
    let project = create_test_unity_project("lux-mcp-loop-failure", false);
    let responses = run_mcp_jsonl(
        &project,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lux_game_dev_loop_once","arguments":{"project_name":"Loop Failure Game","objective":"Try one unavailable Unity maneuver."}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{}}),
        ],
    );

    assert_eq!(responses.len(), 2);
    let loop_result = &responses[0]["result"];
    assert_eq!(loop_result["isError"], true);
    let structured = &loop_result["structuredContent"];
    assert_eq!(structured["ok"], false);
    assert_eq!(structured["stopReason"], "unity_maneuver_unavailable");
    assert_eq!(structured["steps"].as_array().map(Vec::len), Some(2));
    assert_eq!(structured["steps"][0]["ok"], true);
    assert_eq!(structured["steps"][1]["ok"], true);
    assert!(structured["message"]
        .as_str()
        .is_some_and(|text| text.contains(".lux/evidence/game-dev-loop-001-maneuver.json")));
    assert_eq!(responses[1]["result"], json!({}));

    let evidence_path = project.join(".lux/evidence/game-dev-loop-001-maneuver.json");
    let evidence: Value = serde_json::from_str(
        &fs::read_to_string(&evidence_path).expect("unavailable maneuver evidence"),
    )
    .expect("evidence JSON");
    assert_eq!(evidence["ticketId"], "game-dev-loop-001");
    assert_eq!(evidence["status"], "blocked");
}

#[test]
fn mcp_tool_error_result_preserves_json_rpc_connection_for_following_ping() {
    let project = create_test_unity_project("lux-mcp-error", false);
    let responses = run_mcp_jsonl(
        &project,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lux_nope","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"ping","params":{}}),
        ],
    );

    assert_eq!(responses.len(), 3);
    let tool_result = &responses[1]["result"];
    assert_eq!(tool_result["isError"], true);
    assert!(tool_result["content"][0]["text"]
        .as_str()
        .expect("error text")
        .contains("Unknown tool"));
    assert_eq!(responses[2]["result"], json!({}));
}
