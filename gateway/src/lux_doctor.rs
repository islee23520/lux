use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Parser)]
pub struct DoctorArgs {
    /// Project root containing .lux/ directory
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Auto-fix issues via opencode -p
    #[arg(long)]
    pub fix: bool,
    /// Show all checks including passing ones
    #[arg(long)]
    pub verbose: bool,
    /// Check Unity bridge connectivity (default: true)
    #[arg(long)]
    pub check_bridge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub message: String,
    pub fix_hint: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DoctorStatus {
    Ok,
    Warning,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub project_path: PathBuf,
    pub timestamp: String,
    pub total_checks: usize,
    pub passed: usize,
    pub warnings: usize,
    pub errors: usize,
    pub skipped: usize,
    pub checks: Vec<DoctorCheck>,
    pub overall: DoctorStatus,
}

#[derive(Debug, Deserialize)]
struct LockFile {
    pid: u64,
}

pub fn run_doctor_command(args: DoctorArgs) -> Result<()> {
    let project_path = args
        .project_path
        .clone()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    match run_diagnostics(&project_path, &args) {
        Ok(report) => {
            print_report(&report, args.verbose);
            if args.fix && (report.errors > 0 || report.warnings > 0) {
                match auto_fix_issues(&report, &project_path) {
                    Ok(results) => {
                        if !results.is_empty() {
                            eprintln!("\nAuto-fix results:");
                            for result in results {
                                eprintln!("- {result}");
                            }
                        }
                    }
                    Err(error) => eprintln!("\nAuto-fix failed: {error:#}"),
                }
            }
        }
        Err(error) => eprintln!("LUX Doctor failed to build report: {error:#}"),
    }

    Ok(())
}

fn run_diagnostics(project_path: &Path, args: &DoctorArgs) -> Result<DoctorReport> {
    let lux_dir = project_path.join(".lux");
    let is_unity_project = is_unity_project(project_path);
    let mut checks = Vec::new();

    checks.push(check_workspace(&lux_dir));
    checks.push(check_spec(&lux_dir));
    checks.push(check_run_state(&lux_dir));
    checks.push(check_unity_project(project_path, is_unity_project));
    checks.push(check_bridge(
        project_path,
        is_unity_project,
        args.check_bridge,
    ));
    checks.push(check_opencode_plugin(project_path));
    checks.push(check_agents_skills(project_path));
    checks.push(check_lux_binary());
    checks.push(check_lux_integrity(&lux_dir));

    let passed = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Ok)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Warning)
        .count();
    let errors = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Error)
        .count();
    let skipped = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Skipped)
        .count();
    let overall = if errors > 0 {
        DoctorStatus::Error
    } else if warnings > 0 {
        DoctorStatus::Warning
    } else {
        DoctorStatus::Ok
    };

    Ok(DoctorReport {
        project_path: project_path.to_path_buf(),
        timestamp: Utc::now().to_rfc3339(),
        total_checks: checks.len(),
        passed,
        warnings,
        errors,
        skipped,
        checks,
        overall,
    })
}

fn auto_fix_issues(report: &DoctorReport, project_path: &Path) -> Result<Vec<String>> {
    let mut results = Vec::new();
    for check in report
        .checks
        .iter()
        .filter(|check| check.auto_fixable && check.status != DoctorStatus::Ok)
    {
        let Some(prompt) = fix_prompt(check, project_path) else {
            continue;
        };
        let output = run_opencode_prompt(&prompt, project_path, Duration::from_secs(60))?;
        results.push(format!("{}: {}", check.name, output));
    }
    Ok(results)
}

fn check_workspace(lux_dir: &Path) -> DoctorCheck {
    if !lux_dir.exists() {
        return check(
            "workspace",
            DoctorStatus::Error,
            ".lux/ directory is missing",
            hint("Run: lux init --project-path <path>"),
            true,
        );
    }
    if !lux_dir.is_dir() {
        return check(
            "workspace",
            DoctorStatus::Error,
            ".lux exists but is not a directory",
            hint("Move the file and run: lux init --project-path <path>"),
            false,
        );
    }
    if !lux_dir.join("spec.json").is_file() {
        return check(
            "workspace",
            DoctorStatus::Warning,
            ".lux/ directory exists but spec.json is missing",
            hint("Run: lux init --project-path <path>"),
            true,
        );
    }
    check(
        "workspace",
        DoctorStatus::Ok,
        ".lux/ directory present and valid",
        None,
        false,
    )
}

fn check_spec(lux_dir: &Path) -> DoctorCheck {
    let spec_path = lux_dir.join("spec.json");
    let text = match fs::read_to_string(&spec_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return check(
                "spec",
                DoctorStatus::Error,
                "spec.json is missing",
                hint("Run: lux init --project-path <path>"),
                true,
            )
        }
        Err(error) => {
            return check(
                "spec",
                DoctorStatus::Error,
                format!("failed to read spec.json: {error}"),
                hint("Fix permissions or recreate .lux/spec.json"),
                false,
            )
        }
    };
    let value = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return check(
                "spec",
                DoctorStatus::Error,
                format!("spec.json contains invalid JSON: {error}"),
                hint("Repair JSON or run: lux init --force --project-path <path>"),
                true,
            )
        }
    };
    let Some(version) = value.get("version").and_then(Value::as_str) else {
        return check(
            "spec",
            DoctorStatus::Error,
            "spec.json is valid JSON but missing top-level version",
            hint("Run: lux spec validate, then repair the version field"),
            true,
        );
    };
    check(
        "spec",
        DoctorStatus::Ok,
        format!("spec.json valid (schema v{version})"),
        None,
        false,
    )
}

fn check_run_state(lux_dir: &Path) -> DoctorCheck {
    let run_state_path = lux_dir.join("run-state.json");
    let lock_path = lux_dir.join(".lock");
    let mut messages = Vec::new();
    let mut status = DoctorStatus::Ok;
    let mut fix_hint = None;
    let mut auto_fixable = false;

    if run_state_path.exists() {
        match fs::read_to_string(&run_state_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        {
            Some(_) => messages.push("run-state.json valid".to_string()),
            None => {
                status = DoctorStatus::Error;
                messages.push("run-state.json is corrupt".to_string());
                fix_hint = Some("Repair or remove .lux/run-state.json".to_string());
            }
        }
    }

    if lock_path.exists() {
        match read_lock_pid(&lock_path) {
            Ok(pid) if !pid_alive(pid) => {
                if status != DoctorStatus::Error {
                    status = DoctorStatus::Warning;
                }
                messages.push(format!("stale lock file detected (PID {pid} not running)"));
                fix_hint = Some("Run: rm .lux/.lock".to_string());
                auto_fixable = true;
            }
            Ok(pid) => messages.push(format!("lock held by live PID {pid}")),
            Err(error) => {
                if status != DoctorStatus::Error {
                    status = DoctorStatus::Warning;
                }
                messages.push(format!("lock file is unreadable: {error}"));
                fix_hint = Some("Inspect or remove .lux/.lock".to_string());
            }
        }
    }

    if !run_state_path.exists() && !lock_path.exists() {
        return check(
            "run-state",
            DoctorStatus::Skipped,
            "no run-state.json or .lock present",
            None,
            false,
        );
    }

    check(
        "run-state",
        status,
        messages.join("; "),
        fix_hint,
        auto_fixable,
    )
}

fn check_unity_project(project_path: &Path, _is_unity: bool) -> DoctorCheck {
    if _is_unity {
        check(
            "unity-project",
            DoctorStatus::Ok,
            "Unity ProjectSettings detected",
            None,
            false,
        )
    } else {
        check(
            "unity-project",
            DoctorStatus::Warning,
            "Unity project markers not found",
            hint("Run doctor from a Unity project root or pass --project-path"),
            false,
        )
    }
}

fn check_bridge(project_path: &Path, is_unity_project: bool, enabled: bool) -> DoctorCheck {
    if !enabled {
        return check(
            "bridge",
            DoctorStatus::Skipped,
            "bridge check disabled",
            None,
            false,
        );
    }
    if !is_unity_project {
        return check(
            "bridge",
            DoctorStatus::Skipped,
            "not a Unity project",
            None,
            false,
        );
    }
    let bridge_dir = project_path
        .join("Assets")
        .join("Editor")
        .join("AiBridgeEditor");
    if bridge_dir.is_dir() {
        check(
            "bridge",
            DoctorStatus::Ok,
            "Unity bridge installed in Assets/Editor/AiBridgeEditor",
            None,
            false,
        )
    } else {
        check(
            "bridge",
            DoctorStatus::Warning,
            "Unity bridge not installed in Assets/Editor/",
            Some(format!(
                "Run: lux bridge install --project-path {}",
                project_path.display()
            )),
            true,
        )
    }
}

fn check_opencode_plugin(project_path: &Path) -> DoctorCheck {
    let plugin_path = project_path
        .join(".opencode")
        .join("plugins")
        .join("lux-plugin.ts");
    if plugin_path.is_file() {
        check(
            "opencode-plugin",
            DoctorStatus::Ok,
            "Plugin installed at .opencode/plugins/lux-plugin.ts",
            None,
            false,
        )
    } else {
        check(
            "opencode-plugin",
            DoctorStatus::Warning,
            "OpenCode plugin missing; session toasts and Lux context injection are unavailable",
            hint("Run: lux init, or manually copy plugins/opencode/lux-plugin.ts"),
            true,
        )
    }
}

fn check_agents_skills(project_path: &Path) -> DoctorCheck {
    let skills_dir = project_path.join(".agents").join("skills");
    if !skills_dir.is_dir() {
        return check(
            "agents-skills",
            DoctorStatus::Warning,
            ".agents/skills/ directory missing",
            Some(format!(
                "Run: lux agents-install --project-path {}",
                project_path.display()
            )),
            true,
        );
    }
    let entries = fs::read_dir(&skills_dir)
        .map(|iter| {
            iter.filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_lux_skill = entries.iter().any(|name| !name.starts_with("uloop-"));
    if has_lux_skill {
        check(
            "agents-skills",
            DoctorStatus::Ok,
            "Lux workflow skills present",
            None,
            false,
        )
    } else {
        check(
            "agents-skills",
            DoctorStatus::Warning,
            "No lux workflow skills found (only uloop-* skills or empty directory)",
            Some(format!(
                "Run: lux agents-install --project-path {}",
                project_path.display()
            )),
            true,
        )
    }
}

fn check_lux_binary() -> DoctorCheck {
    match Command::new("lux").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            check(
                "lux-binary",
                DoctorStatus::Ok,
                format!("{} OK", version_or_default(&version)),
                None,
                false,
            )
        }
        Ok(output) => check(
            "lux-binary",
            DoctorStatus::Error,
            format!("lux --version failed with status {}", output.status),
            hint("Reinstall lux and verify it is on PATH"),
            false,
        ),
        Err(error) => check(
            "lux-binary",
            DoctorStatus::Error,
            format!("lux binary not found or not executable: {error}"),
            hint("Run: cargo install --path gateway --force"),
            false,
        ),
    }
}

fn check_lux_integrity(lux_dir: &Path) -> DoctorCheck {
    if !lux_dir.is_dir() {
        return check(
            ".lux integrity",
            DoctorStatus::Skipped,
            ".lux/ directory missing",
            None,
            false,
        );
    }
    let mut missing = Vec::new();
    if !lux_dir.join("spec.json").is_file() {
        missing.push("spec.json");
    }
    let optional = ["run-state.json", "roadmap.json"]
        .iter()
        .filter(|name| lux_dir.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        check(
            ".lux integrity",
            DoctorStatus::Ok,
            format!(
                "required files present; optional present: {}",
                optional.join(", ")
            ),
            None,
            false,
        )
    } else {
        check(
            ".lux integrity",
            DoctorStatus::Warning,
            format!("missing required file(s): {}", missing.join(", ")),
            hint("Run: lux init --project-path <path>"),
            true,
        )
    }
}

fn print_report(report: &DoctorReport, verbose: bool) {
    eprintln!("LUX Doctor Report");
    eprintln!("==================");
    eprintln!("Project: {}", report.project_path.display());
    eprintln!("Overall: {}", overall_text(report));
    eprintln!();
    for check in &report.checks {
        if verbose || check.status != DoctorStatus::Ok {
            eprintln!(
                "[{}] {:<16} {}",
                status_icon(&check.status),
                check.name,
                check.message
            );
        }
    }
    let fixes = report
        .checks
        .iter()
        .filter(|check| check.status != DoctorStatus::Ok)
        .filter_map(|check| check.fix_hint.as_ref().map(|hint| (&check.name, hint)));
    let fixes = fixes.collect::<Vec<_>>();
    if !fixes.is_empty() {
        eprintln!("\nSuggested fixes:");
        for (index, (name, hint)) in fixes.iter().enumerate() {
            eprintln!("{}. [{}] {}", index + 1, name, hint);
        }
        eprintln!("\nRun 'lux doctor --fix' to auto-fix actionable items.");
    }
}

fn run_opencode_prompt(prompt: &str, project_path: &Path, timeout: Duration) -> Result<String> {
    let mut child = Command::new("opencode")
        .args(["-p", prompt])
        .current_dir(project_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn opencode -p")?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().context("failed to poll opencode -p")? {
            return collect_child_output(child, status.success());
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok("timed out after 60s".to_string());
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn collect_child_output(mut child: std::process::Child, success: bool) -> Result<String> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut stream) = child.stdout.take() {
        stream
            .read_to_string(&mut stdout)
            .context("failed to read opencode stdout")?;
    }
    if let Some(mut stream) = child.stderr.take() {
        stream
            .read_to_string(&mut stderr)
            .context("failed to read opencode stderr")?;
    }
    let summary = if success { "completed" } else { "failed" };
    let detail = [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    if detail.is_empty() {
        Ok(summary.to_string())
    } else {
        Ok(format!("{summary}: {detail}"))
    }
}

fn fix_prompt(check: &DoctorCheck, project_path: &Path) -> Option<String> {
    let path = project_path.display();
    match check.name.as_str() {
        "workspace" | "spec" | ".lux integrity" => Some(format!(
            "The Lux project at {path} has a missing or invalid .lux workspace/spec. Run `lux init --project-path {path}` and preserve existing valid project files."
        )),
        "run-state" => Some(format!(
            "The Lux project at {path} has stale or corrupt run state: {}. If .lux/.lock belongs to a dead PID, remove it. Do not delete live state.",
            check.message
        )),
        "bridge" => Some(format!(
            "Install the Unity bridge for the Lux project at {path}. Run `lux bridge install --project-path {path}`."
        )),
        "opencode-plugin" => Some(format!(
            "Install the Lux OpenCode plugin for the project at {path}. Run `lux init --project-path {path}` again or copy plugins/opencode/lux-plugin.ts to .opencode/plugins/lux-plugin.ts."
        )),
        "agents-skills" => Some(format!(
            "Install Lux workflow skills for the project at {path}. Run `lux agents-install --project-path {path}`."
        )),
        _ => None,
    }
}

fn read_lock_pid(path: &Path) -> Result<u64> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let lock = serde_json::from_str::<LockFile>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(lock.pid)
}

fn pid_alive(pid: u64) -> bool {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if result == 0 {
            return true;
        }
        let errno = unsafe { *libc::__error() };
        errno != libc::ESRCH
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

fn is_unity_project(project_path: &Path) -> bool {
    project_path
        .join("ProjectSettings")
        .join("ProjectSettings.asset")
        .is_file()
        || project_path
            .join("ProjectSettings")
            .join("ProjectVersion.txt")
            .is_file()
}

fn check(
    name: &str,
    status: DoctorStatus,
    message: impl Into<String>,
    fix_hint: Option<String>,
    auto_fixable: bool,
) -> DoctorCheck {
    DoctorCheck {
        name: name.to_string(),
        status,
        message: message.into(),
        fix_hint,
        auto_fixable,
    }
}

fn hint(text: impl Into<String>) -> Option<String> {
    Some(text.into())
}

fn status_icon(status: &DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Ok => "✓",
        DoctorStatus::Warning => "⚠",
        DoctorStatus::Error => "✗",
        DoctorStatus::Skipped => "-",
    }
}

fn overall_text(report: &DoctorReport) -> String {
    match report.overall {
        DoctorStatus::Ok => format!("✓ {} checks passed", report.passed),
        DoctorStatus::Warning => {
            format!("⚠️  {} warnings, {} errors", report.warnings, report.errors)
        }
        DoctorStatus::Error => format!("✗ {} warnings, {} errors", report.warnings, report.errors),
        DoctorStatus::Skipped => format!("- {} skipped", report.skipped),
    }
}

fn version_or_default(version: &str) -> String {
    if version.is_empty() {
        "lux --version returned no text".to_string()
    } else {
        version.to_string()
    }
}
