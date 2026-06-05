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
"#
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
fn ulw_check_removes_stale_temp_file_before_write() {
    let project = temp_path("lux-hooks-ulw-stale-temp");
    let hooks = project.join(".lux/hooks");
    fs::create_dir_all(&hooks).expect("create hooks dir");
    fs::write(hooks.join(".ulw-check.json.tmp"), "stale").expect("stale temp file");

    let output = run_hook(&project, "UserPromptSubmit", r#"{"prompt":"ulw"}"#);

    assert!(output.status.success());
    assert!(!hooks.join(".ulw-check.json.tmp").exists());
}

#[cfg(unix)]
#[test]
fn ulw_check_rejects_symlinked_temp_file_before_write() {
    let project = temp_path("lux-hooks-ulw-temp-symlink");
    let hooks = project.join(".lux/hooks");
    fs::create_dir_all(&hooks).expect("create hooks dir");
    let outside = temp_path("lux-hooks-outside-ulw-temp");
    fs::write(&outside, "outside-original").expect("outside target");
    std::os::unix::fs::symlink(&outside, hooks.join(".ulw-check.json.tmp"))
        .expect("symlink temp file");

    let output = run_hook(&project, "UserPromptSubmit", r#"{"prompt":"ulw"}"#);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("temporary file must not be a symlink"));
    assert_eq!(
        fs::read_to_string(outside).expect("outside content"),
        "outside-original"
    );
}

#[cfg(unix)]
#[test]
fn ulw_check_rejects_hardlinked_temp_file_before_write() {
    let project = temp_path("lux-hooks-ulw-temp-hardlink");
    let hooks = project.join(".lux/hooks");
    fs::create_dir_all(&hooks).expect("create hooks dir");
    let outside = temp_path("lux-hooks-outside-ulw-hardlink");
    fs::write(&outside, "outside-original").expect("outside target");
    fs::hard_link(&outside, hooks.join(".ulw-check.json.tmp")).expect("hardlink temp file");

    let output = run_hook(&project, "UserPromptSubmit", r#"{"prompt":"ulw"}"#);

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(outside).expect("outside content"),
        "outside-original"
    );
}

#[cfg(unix)]
#[test]
fn verification_evidence_rejects_hardlinked_evidence_file() {
    let project = temp_path("lux-hooks-hardlinked-evidence");
    fs::create_dir_all(project.join(".lux/evidence")).expect("create evidence dir");
    fs::write(
        project.join(".lux-agent.toml"),
        settings("verification_evidence"),
    )
    .expect("settings");
    let outside = temp_path("lux-hooks-outside-hardlinked-evidence");
    fs::write(&outside, "outside evidence").expect("outside evidence");
    fs::hard_link(&outside, project.join(".lux/evidence/manual.txt")).expect("hardlink evidence");

    let output = run_hook(
        &project,
        "LuxVerificationEvidence",
        r#"{"evidence_path":".lux/evidence/manual.txt"}"#,
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(
        report["gate_result"]["findings"][0]["message"],
        "evidence_path must not be hardlinked"
    );
}
