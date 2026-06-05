use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

fn settings(enabled_hook: &str) -> String {
    format!(
        r#"
version = 1

[hooks]
{enabled_hook} = true

[policy]
forbidden_markers = ["{}"]
allow_markers = ["lux-allow-failover"]
"#,
        ["TO", "DO"].concat()
    )
}

fn run_hook(project: &Path, event: &str, stdin_json: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "hooks",
            "run",
            "--project-path",
            &project.display().to_string(),
            "--event",
            event,
            "--json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lux hooks run");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_json.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("hook output")
}

#[test]
fn unknown_hook_event_returns_unsupported_not_success() {
    let project = temp_path("lux-hooks-unknown-event");
    fs::create_dir_all(&project).expect("create project");

    let output = run_hook(&project, "MadeUpEvent", "{}");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(report["gate_result"]["status"], "unsupported");
    assert_eq!(report["source"], "unsupported-hook");
    let log = fs::read_to_string(project.join(".lux/hooks/events.jsonl")).expect("event log");
    assert!(log.contains("\"status\":\"unsupported\""));
    assert!(!log.contains("\"source\":\"lux-project-hook\""));
}

#[test]
fn verification_evidence_requires_existing_lux_evidence_path() {
    let project = temp_path("lux-hooks-verification-evidence");
    fs::create_dir_all(project.join(".lux/evidence")).expect("create evidence dir");
    fs::write(
        project.join(".lux-agent.toml"),
        settings("verification_evidence"),
    )
    .expect("settings");

    let missing = run_hook(&project, "LuxVerificationEvidence", "{}");

    assert!(!missing.status.success());
    let missing_stdout = String::from_utf8(missing.stdout).expect("missing stdout");
    let missing_report: Value = serde_json::from_str(&missing_stdout).expect("missing json");
    assert_eq!(missing_report["gate_result"]["status"], "failed");
    assert_eq!(
        missing_report["gate_result"]["findings"][0]["marker"],
        "evidence_path"
    );

    fs::write(project.join(".lux/evidence/manual.txt"), "manual qa pass").expect("evidence");
    let passed = run_hook(
        &project,
        "LuxVerificationEvidence",
        r#"{"evidence_path":".lux/evidence/manual.txt"}"#,
    );

    assert!(passed.status.success());
    let passed_stdout = String::from_utf8(passed.stdout).expect("passed stdout");
    let passed_report: Value = serde_json::from_str(&passed_stdout).expect("passed json");
    assert_eq!(passed_report["gate_result"]["status"], "passed");
}

#[cfg(unix)]
#[test]
fn verification_evidence_rejects_symlink_escape() {
    let project = temp_path("lux-hooks-verification-symlink");
    fs::create_dir_all(project.join(".lux/evidence")).expect("create evidence dir");
    fs::write(
        project.join(".lux-agent.toml"),
        settings("verification_evidence"),
    )
    .expect("settings");
    let outside = temp_path("lux-hooks-outside-evidence");
    fs::write(&outside, "external evidence").expect("outside evidence");
    std::os::unix::fs::symlink(&outside, project.join(".lux/evidence/link.txt")).expect("symlink");

    let output = run_hook(
        &project,
        "LuxVerificationEvidence",
        r#"{"evidence_path":".lux/evidence/link.txt"}"#,
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(report["gate_result"]["status"], "failed");
    assert_eq!(
        report["gate_result"]["findings"][0]["message"],
        "evidence_path must stay under .lux/evidence"
    );
}

#[test]
fn policy_scan_excludes_generated_build_and_vendor_paths() {
    let project = temp_path("lux-hooks-generated-paths");
    fs::create_dir_all(project.join("Library")).expect("library dir");
    fs::create_dir_all(project.join("Builds")).expect("builds dir");
    fs::create_dir_all(project.join("vendor")).expect("vendor dir");
    fs::write(
        project.join(".lux-agent.toml"),
        settings("post_edit_policy"),
    )
    .expect("settings");
    for path in ["Library/cache.txt", "Builds/output.txt", "vendor/code.txt"] {
        fs::write(
            project.join(path),
            format!(
                "generated {} marker should be ignored",
                ["TO", "DO"].concat()
            ),
        )
        .expect("generated file");
    }

    let output = run_hook(&project, "LuxPostEditPolicy", "{}");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(report["gate_result"]["status"], "passed");
    assert_eq!(report["gate_result"]["findings"], Value::Array(Vec::new()));
}

#[test]
fn policy_findings_do_not_persist_source_line_snippets() {
    let project = temp_path("lux-hooks-redacted-snippet");
    fs::create_dir_all(project.join("src")).expect("source dir");
    fs::write(
        project.join(".lux-agent.toml"),
        settings("post_edit_policy"),
    )
    .expect("settings");
    fs::write(
        project.join("src/main.rs"),
        format!(
            "let token = \"secret-token-123\"; // {} marker\n",
            ["TO", "DO"].concat()
        ),
    )
    .expect("source");

    let output = run_hook(&project, "LuxPostEditPolicy", "{}");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(
        report["gate_result"]["findings"][0]["snippet"],
        "[redacted]"
    );
    assert!(!stdout.contains("secret-token-123"));
    let log = fs::read_to_string(project.join(".lux/hooks/events.jsonl")).expect("event log");
    assert!(!log.contains("secret-token-123"));
}
