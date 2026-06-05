use lux::lux_hooks::{
    codex_hook_status, install_codex_hook_bridge, run_hook_bridge, HooksInstallArgs, HooksRunArgs,
    HooksStatusArgs,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ))
}

fn forbidden_markers_toml() -> String {
    format!(
        "forbidden_markers = [\"{}\", \"{}\", \"{}\"]",
        ["TO", "DO"].concat(),
        ["FIX", "ME"].concat(),
        ["HA", "CK"].concat()
    )
}

#[test]
fn install_codex_hook_bridge_is_idempotent_for_project() {
    let project = temp_path("lux-hooks-unit-project");
    let hooks_path = temp_path("lux-hooks-unit-codex").join("hooks.json");
    fs::create_dir_all(&project).expect("create project");

    let first = install_codex_hook_bridge(&HooksInstallArgs {
        project_path: Some(project.clone()),
        hooks_path: Some(hooks_path.clone()),
        dry_run: false,
        force: false,
        json: true,
    })
    .expect("first install");
    assert!(first.changed);

    let second = install_codex_hook_bridge(&HooksInstallArgs {
        project_path: Some(project.clone()),
        hooks_path: Some(hooks_path.clone()),
        dry_run: false,
        force: false,
        json: true,
    })
    .expect("second install");
    assert!(!second.changed);

    let status = codex_hook_status(&HooksStatusArgs {
        project_path: Some(project),
        hooks_path: Some(hooks_path),
        json: true,
    })
    .expect("hook status");
    assert!(status.events.iter().all(|event| event.installed));
}

#[test]
fn missing_settings_reported_as_not_configured() {
    let project = temp_path("lux-hooks-missing-settings");
    let hooks_path = temp_path("lux-hooks-missing-settings-codex").join("hooks.json");
    fs::create_dir_all(&project).expect("create project");

    let status = codex_hook_status(&HooksStatusArgs {
        project_path: Some(project),
        hooks_path: Some(hooks_path),
        json: true,
    })
    .expect("hook status");

    assert_eq!(status.project_settings.status, "not_configured");
    assert!(status.project_settings.path.is_none());
    assert!(status.loaded_rule_paths.is_empty());
}

#[test]
fn post_edit_policy_without_settings_is_not_configured_not_enforced() {
    let project = temp_path("lux-hooks-policy-missing-settings");
    fs::create_dir_all(project.join("src")).expect("create project");
    fs::write(
        project.join("src").join("main.rs"),
        format!(
            "fn main() {{ /* {} would be forbidden when configured */ }}\n",
            ["TO", "DO"].concat()
        ),
    )
    .expect("source");

    let report = run_hook_bridge(&HooksRunArgs {
        event: "LuxPostEditPolicy".to_string(),
        project_path: Some(project.clone()),
        json: true,
    })
    .expect("policy without settings should not enforce");

    assert_eq!(report.project_settings.status, "not_configured");
    assert_eq!(report.gate_result.status, "not_configured");
    assert!(report.gate_result.findings.is_empty());
    let log = fs::read_to_string(project.join(".lux/hooks/events.jsonl")).expect("event log");
    assert!(log.contains("\"source\":\"lux-project-hook\""));
    assert!(log.contains("\"status\":\"not_configured\""));
}

#[test]
fn status_reports_project_settings_and_agents_rule_paths() {
    let project_root = temp_path("lux-hooks-settings-root");
    let project = project_root.join("game");
    let hooks_path = temp_path("lux-hooks-settings-codex").join("hooks.json");
    fs::create_dir_all(&project).expect("create project");
    fs::write(project_root.join("AGENTS.md"), "# root rules\n").expect("root rules");
    fs::write(project.join("AGENTS.md"), "# game rules\n").expect("game rules");
    let settings = format!(
        r#"
version = 1

[hooks]
pre_work_rule_load = true
post_edit_policy = true
verification_evidence = true

[policy]
{}
allow_markers = ["lux-allow-failover", "lux-allow-legacy", "lux-allow-dual-write"]
"#,
        forbidden_markers_toml()
    );
    fs::write(project.join(".lux-agent.toml"), settings).expect("settings");

    let status = codex_hook_status(&HooksStatusArgs {
        project_path: Some(project.clone()),
        hooks_path: Some(hooks_path),
        json: true,
    })
    .expect("hook status");

    assert_eq!(status.project_settings.status, "configured");
    assert_eq!(
        status.project_settings.path,
        Some(project.join(".lux-agent.toml"))
    );
    assert_eq!(status.project_settings.version, Some(1));
    assert_eq!(
        status.loaded_rule_paths,
        vec![project_root.join("AGENTS.md"), project.join("AGENTS.md")]
    );
    assert!(status
        .lux_events
        .iter()
        .any(|event| event == "LuxPostEditPolicy"));
}

#[test]
fn post_edit_policy_fails_on_forbidden_marker_and_allow_marker_without_evidence() {
    let project = temp_path("lux-hooks-policy-project");
    fs::create_dir_all(project.join("src")).expect("create project");
    let settings = format!(
        r#"
version = 1

[hooks]
post_edit_policy = true

[policy]
{}
allow_markers = ["lux-allow-failover"]
"#,
        forbidden_markers_toml()
    );
    fs::write(project.join(".lux-agent.toml"), settings).expect("settings");
    fs::write(
        project.join("src").join("main.rs"),
        format!(
            "fn main() {{ /* {} forbidden */ }}\n// lux-allow-failover\n",
            ["TO", "DO"].concat()
        ),
    )
    .expect("source");

    let error = run_hook_bridge(&HooksRunArgs {
        event: "LuxPostEditPolicy".to_string(),
        project_path: Some(project),
        json: true,
    })
    .expect_err("policy should fail");
    let message = error.to_string();
    assert!(message.contains("policy gate failed"));
    assert!(message.contains(&["TO", "DO"].concat()));
    assert!(message.contains("lux-allow-failover"));
    assert!(!message.contains(".lux-agent.toml"));
}

#[test]
fn run_records_project_settings_and_gate_result_in_lux_event_log() {
    let project = temp_path("lux-hooks-run-settings");
    fs::create_dir_all(&project).expect("create project");
    let settings = format!(
        r#"
version = 1

[hooks]
pre_work_rule_load = true

[policy]
{}
"#,
        forbidden_markers_toml()
    );
    fs::write(project.join(".lux-agent.toml"), settings).expect("settings");

    let report = run_hook_bridge(&HooksRunArgs {
        event: "UserPromptSubmit".to_string(),
        project_path: Some(project.clone()),
        json: true,
    })
    .expect("hook run");

    assert_eq!(report.project_settings.status, "configured");
    assert_eq!(report.gate_result.status, "passed");
    let log = fs::read_to_string(project.join(".lux/hooks/events.jsonl")).expect("event log");
    assert!(log.contains("\"project_settings\""));
    assert!(log.contains("\"gate_result\""));
}

#[test]
fn lux_project_events_are_recorded_as_lux_project_hooks() {
    let project = temp_path("lux-hooks-lux-event-source");
    fs::create_dir_all(&project).expect("create project");
    let settings = format!(
        r#"
version = 1

[hooks]
pre_work_rule_load = true

[policy]
{}
"#,
        forbidden_markers_toml()
    );
    fs::write(project.join(".lux-agent.toml"), settings).expect("settings");

    let report = run_hook_bridge(&HooksRunArgs {
        event: "LuxPreWorkRuleLoad".to_string(),
        project_path: Some(project.clone()),
        json: true,
    })
    .expect("lux project hook run");

    assert_eq!(report.gate_result.status, "passed");
    let log = fs::read_to_string(project.join(".lux/hooks/events.jsonl")).expect("event log");
    assert!(log.contains("\"event\":\"LuxPreWorkRuleLoad\""));
    assert!(log.contains("\"source\":\"lux-project-hook\""));
    assert!(!log.contains("\"source\":\"codex-native-hook\""));
}
