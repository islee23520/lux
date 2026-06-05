use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Output, Stdio},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::lux_run_state::{RunState, RunStatus, StopReason};
use crate::lux_spec::{self, DomainSpec, PillarStatus, SpecProject};
use crate::lux_team_profile::{TeamProfile, VerificationTier};
use crate::lux_ticket::{
    create_or_update_blocker, stable_blocker_key, stable_blocker_ticket_id_from_key,
    FileTicketStore, Ticket, TicketPriority, TicketStore,
};

const VERIFICATION_BLOCKER_SPEC_REF: &str = ".lux/verification/latest.json";

pub const T3_COMPILE_TIMEOUT_SECS: u64 = 600;
pub const T3_SCENE_SMOKE_TIMEOUT_SECS: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockerConfig {
    pub max_blocker_attempts_per_ticket: u32,
}

impl Default for BlockerConfig {
    fn default() -> Self {
        Self {
            max_blocker_attempts_per_ticket: 3,
        }
    }
}

const T3_BUILD_TARGET: &str = "WebGL";
const T3_COMPILE_METHOD: &str = "Linalab.Lux.Editor.LuxBatchAutomation.Compile";
const T3_SCENE_SMOKE_METHOD: &str = "Linalab.Lux.Editor.LuxSceneSmoke.Run";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationOpts {
    pub run_id: String,
    pub working_dir: PathBuf,
    pub evidence_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Passed,
    Failed,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct T3UnityGateTimeouts {
    pub compile_secs: u64,
    pub scene_smoke_secs: u64,
}

impl Default for T3UnityGateTimeouts {
    fn default() -> Self {
        Self {
            compile_secs: T3_COMPILE_TIMEOUT_SECS,
            scene_smoke_secs: T3_SCENE_SMOKE_TIMEOUT_SECS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub timestamp: String,
    pub checks: Vec<CheckResult>,
    pub overall_score: f64,
    pub blocker_ticket_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutedVerificationResult {
    pub status: VerificationStatus,
    pub policy_used: String,
    pub evidence_paths: Vec<String>,
    pub passed: bool,
    pub timestamp: String,
    pub checks: Vec<CheckResult>,
    pub overall_score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub category: CheckCategory,
    pub passed: bool,
    pub score: f64,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckCategory {
    SpecCompleteness,
    ImplementationExists,
    UnityCompilable,
    WebGLPlayable,
    FeedbackIntegration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMode {
    Cached,
    Live,
}

pub fn route_verification(
    ticket: &Ticket,
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let policy = ticket
        .verification_policy
        .as_deref()
        .map(str::trim)
        .filter(|policy| !policy.is_empty())
        .context("verification_policy is required for execution-grade dispatch")?;

    if policy == "unity_t3" {
        return route_unity_t3(policy, opts);
    }
    if policy == "unity_uloop" {
        return route_unity_uloop(policy, ticket, opts);
    }
    if policy == "godot_cli" {
        return route_godot_cli(policy, ticket, opts);
    }
    if policy == "threejs_browser" {
        return route_threejs_browser(policy, ticket, opts);
    }
    if policy == "engine_blocker" {
        return route_engine_blocker(
            policy,
            opts,
            "engine verification blocker requested",
            json!({
                "mode": "router",
                "verification_basis": "engine_blocker",
                "policy": policy,
            }),
        );
    }
    if let Some(command_list) = policy.strip_prefix("command_suite:") {
        let commands = parse_policy_commands(command_list);
        if commands.is_empty() {
            bail!("command_suite verification requires at least one command");
        }
        return run_declared_commands(policy, &commands, opts);
    }
    if policy == "doc_only" {
        let commands = ticket.command_allowlist.clone().unwrap_or_default();
        if commands.is_empty() {
            bail!("doc_only verification requires at least one grep/schema validation command");
        }
        if !commands
            .iter()
            .any(|command| is_doc_validation_command(command))
        {
            bail!("doc_only verification requires at least one grep/schema validation command");
        }
        return run_declared_commands(policy, &commands, opts);
    }
    if policy == "live" {
        bail!("VerificationMode::Live is not supported in M6");
    }

    route_engine_blocker(
        policy,
        opts,
        &format!("Unsupported verification_policy: {policy}"),
        json!({
            "mode": "router",
            "verification_basis": "verification_policy_dispatch",
            "policy": policy,
        }),
    )
}

fn route_unity_t3(policy: &str, opts: &VerificationOpts) -> Result<RoutedVerificationResult> {
    let checks = run_t3_unity_gate(
        &opts.working_dir,
        "autonomous",
        &[check(
            "T3 autonomous: Router Prerequisite",
            CheckCategory::UnityCompilable,
            true,
            1.0,
            "Verification policy router selected Unity T3",
            Some(json!({
                "mode": "router",
                "verification_basis": "verification_policy_dispatch",
                "policy": policy,
            })),
        )],
    );
    let evidence_text = verification_checks_evidence(policy, &checks);
    let evidence_paths = vec![write_verification_evidence(opts, 1, &evidence_text)?];
    let status = verification_status_for_passed(checks.iter().all(|check| check.passed));
    Ok(policy_result(status, policy, evidence_paths, checks))
}

fn route_unity_uloop(
    policy: &str,
    ticket: &Ticket,
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let commands = ticket.command_allowlist.clone().unwrap_or_default();
    if commands.len() < 3 {
        return route_engine_blocker(
            policy,
            opts,
            "unity_uloop requires compile, run-tests, and screenshot commands",
            json!({
                "mode": "router",
                "verification_basis": "unity_uloop",
                "policy": policy,
                "required_evidence": ["compile", "run-tests", "screenshot"],
            }),
        );
    }
    run_labeled_commands(
        policy,
        &[
            ("compile", commands[0].clone()),
            ("test", commands[1].clone()),
            ("screenshot", commands[2].clone()),
        ],
        opts,
    )
}

fn route_godot_cli(
    policy: &str,
    ticket: &Ticket,
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let configured = ticket
        .command_allowlist
        .as_ref()
        .and_then(|commands| commands.first())
        .map_or("godot", String::as_str);
    if executable_in_path(configured).is_none() {
        return route_engine_blocker(
            policy,
            opts,
            &format!("missing CLI: {configured}"),
            json!({
                "mode": "router",
                "verification_basis": "godot_cli",
                "policy": policy,
                "required_binary": configured,
            }),
        );
    }
    run_labeled_commands(
        policy,
        &[("godot-version", format!("{configured} --version"))],
        opts,
    )
}

fn route_threejs_browser(
    policy: &str,
    ticket: &Ticket,
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let commands = ticket.command_allowlist.clone().unwrap_or_default();
    if commands.is_empty() {
        return route_engine_blocker(
            policy,
            opts,
            "threejs_browser requires an explicit browser QA command",
            json!({
                "mode": "router",
                "verification_basis": "threejs_browser",
                "policy": policy,
                "required_surface": "browser_qa_command",
            }),
        );
    }
    let labeled = commands
        .into_iter()
        .enumerate()
        .map(|(index, command)| (format!("browser-{}", index + 1), command))
        .collect::<Vec<_>>();
    let borrowed = labeled
        .iter()
        .map(|(label, command)| (label.as_str(), command.clone()))
        .collect::<Vec<_>>();
    run_labeled_commands(policy, &borrowed, opts)
}

fn route_engine_blocker(
    policy: &str,
    opts: &VerificationOpts,
    reason: &str,
    details: Value,
) -> Result<RoutedVerificationResult> {
    let check = check(
        "Engine Verification Router",
        CheckCategory::ImplementationExists,
        false,
        0.0,
        reason,
        Some(details.clone()),
    );
    let evidence = format!(
        "policy={policy}\nunsupported_reason={reason}\ndetails={}\n",
        serde_json::to_string(&details)?
    );
    let evidence_paths = vec![write_verification_evidence_named(
        opts,
        "engine-blocker",
        &evidence,
    )?];
    let store = FileTicketStore::new(&opts.working_dir);
    create_or_update_blocker(
        &store,
        "engine-verification",
        policy,
        Some(VERIFICATION_BLOCKER_SPEC_REF),
        format!("Engine verification blocker: {policy}"),
        format!("{reason}\n\nEvidence: {}", evidence_paths[0]),
        TicketPriority::High,
        vec![
            "verification".to_string(),
            "engine-blocker".to_string(),
            policy.to_string(),
        ],
    )?;
    Ok(policy_result(
        VerificationStatus::Unsupported,
        policy,
        evidence_paths,
        vec![check],
    ))
}

fn run_labeled_commands(
    policy: &str,
    commands: &[(&str, String)],
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let mut checks = Vec::new();
    let mut evidence_paths = Vec::new();

    for (index, (label, command_text)) in commands.iter().enumerate() {
        let command_number = index + 1;
        let argv = parse_command_argv(command_text)?;
        let output = ProcessCommand::new(&argv[0])
            .args(&argv[1..])
            .current_dir(&opts.working_dir)
            .stdin(Stdio::null())
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let passed = output.status.success();
                let evidence_text = format!(
                    "phase={}\n{}",
                    verification_phase_label(label),
                    command_evidence_text(
                        policy,
                        command_number,
                        command_text,
                        exit_code,
                        &stdout,
                        &stderr,
                        None,
                    )
                );
                evidence_paths.push(write_verification_evidence_named(
                    opts,
                    label,
                    &evidence_text,
                )?);
                checks.push(command_result_check(
                    command_number,
                    command_text,
                    passed,
                    exit_code,
                    None,
                ));
                if !passed {
                    return Ok(policy_result(
                        VerificationStatus::Failed,
                        policy,
                        evidence_paths,
                        checks,
                    ));
                }
            }
            Err(error) => {
                let evidence_text = format!(
                    "phase={}\n{}",
                    verification_phase_label(label),
                    command_evidence_text(
                        policy,
                        command_number,
                        command_text,
                        None,
                        "",
                        "",
                        Some(&error.to_string()),
                    )
                );
                evidence_paths.push(write_verification_evidence_named(
                    opts,
                    label,
                    &evidence_text,
                )?);
                checks.push(command_result_check(
                    command_number,
                    command_text,
                    false,
                    None,
                    Some(&error.to_string()),
                ));
                return Ok(policy_result(
                    VerificationStatus::Failed,
                    policy,
                    evidence_paths,
                    checks,
                ));
            }
        }
    }

    Ok(policy_result(
        VerificationStatus::Passed,
        policy,
        evidence_paths,
        checks,
    ))
}

fn run_declared_commands(
    policy: &str,
    commands: &[String],
    opts: &VerificationOpts,
) -> Result<RoutedVerificationResult> {
    let mut checks = Vec::new();
    let mut evidence_paths = Vec::new();

    for (index, command_text) in commands.iter().enumerate() {
        let command_number = index + 1;
        let argv = parse_command_argv(command_text)?;
        let output = ProcessCommand::new(&argv[0])
            .args(&argv[1..])
            .current_dir(&opts.working_dir)
            .stdin(Stdio::null())
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let passed = output.status.success();
                let evidence_text = command_evidence_text(
                    policy,
                    command_number,
                    command_text,
                    exit_code,
                    &stdout,
                    &stderr,
                    None,
                );
                evidence_paths.push(write_verification_evidence(
                    opts,
                    command_number,
                    &evidence_text,
                )?);
                checks.push(command_result_check(
                    command_number,
                    command_text,
                    passed,
                    exit_code,
                    None,
                ));
                if !passed {
                    return Ok(policy_result(
                        VerificationStatus::Failed,
                        policy,
                        evidence_paths,
                        checks,
                    ));
                }
            }
            Err(error) => {
                let evidence_text = command_evidence_text(
                    policy,
                    command_number,
                    command_text,
                    None,
                    "",
                    "",
                    Some(&error.to_string()),
                );
                evidence_paths.push(write_verification_evidence(
                    opts,
                    command_number,
                    &evidence_text,
                )?);
                checks.push(command_result_check(
                    command_number,
                    command_text,
                    false,
                    None,
                    Some(&error.to_string()),
                ));
                return Ok(policy_result(
                    VerificationStatus::Failed,
                    policy,
                    evidence_paths,
                    checks,
                ));
            }
        }
    }

    Ok(policy_result(
        VerificationStatus::Passed,
        policy,
        evidence_paths,
        checks,
    ))
}

fn parse_policy_commands(command_list: &str) -> Vec<String> {
    command_list
        .split(',')
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_command_argv(command_text: &str) -> Result<Vec<String>> {
    let argv = command_text
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if argv.is_empty() {
        bail!("verification command cannot be empty");
    }
    Ok(argv)
}

fn is_doc_validation_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower
        .split_whitespace()
        .next()
        .is_some_and(|binary| binary == "grep" || binary == "rg")
        || lower.contains("schema")
}

fn command_result_check(
    command_number: usize,
    command_text: &str,
    passed: bool,
    exit_code: Option<i32>,
    launch_error: Option<&str>,
) -> CheckResult {
    check(
        &format!("Verification Command {command_number}"),
        CheckCategory::ImplementationExists,
        passed,
        if passed { 1.0 } else { 0.0 },
        if passed {
            "Verification command completed successfully"
        } else if launch_error.is_some() {
            "Verification command failed to launch"
        } else {
            "Verification command exited non-zero"
        },
        Some(json!({
            "mode": "router",
            "verification_basis": "command_suite",
            "command": command_text,
            "exit_code": exit_code,
            "launch_error": launch_error,
        })),
    )
}

fn command_evidence_text(
    policy: &str,
    command_number: usize,
    command_text: &str,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    launch_error: Option<&str>,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("policy={policy}\n"));
    text.push_str(&format!("command_index={command_number}\n"));
    text.push_str(&format!("command={command_text}\n"));
    match exit_code {
        Some(code) => text.push_str(&format!("exit_code={code}\n")),
        None => text.push_str("exit_code=<unavailable>\n"),
    }
    if let Some(error) = launch_error {
        text.push_str(&format!("launch_error={error}\n"));
    }
    text.push_str("\nstdout:\n");
    text.push_str(stdout);
    text.push_str("\n\nstderr:\n");
    text.push_str(stderr);
    text
}

fn verification_checks_evidence(policy: &str, checks: &[CheckResult]) -> String {
    let mut text = String::new();
    text.push_str(&format!("policy={policy}\n"));
    for check in checks {
        text.push_str(&format!(
            "check={} passed={} score={:.2} message={}\n",
            check.name, check.passed, check.score, check.message
        ));
    }
    text
}

fn write_verification_evidence(
    opts: &VerificationOpts,
    command_number: usize,
    text: &str,
) -> Result<String> {
    write_verification_evidence_named(opts, &command_number.to_string(), text)
}

fn write_verification_evidence_named(
    opts: &VerificationOpts,
    label: &str,
    text: &str,
) -> Result<String> {
    let relative_dir = relative_evidence_dir(opts);
    let file_name = format!("verify_{}.txt", evidence_label(label));
    let relative_path = format!("{}/{}", path_to_slash(&relative_dir), file_name);
    crate::lux_io::write_evidence_file(&opts.working_dir, &relative_path, text, usize::MAX)
        .with_context(|| format!("failed to write verification evidence {relative_path}"))
}

fn evidence_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn verification_phase_label(label: &str) -> &str {
    match label {
        "compile" => "compile",
        "run-tests" => "test",
        "screenshot" => "screenshot",
        _ => label,
    }
}

fn executable_in_path(executable: &str) -> Option<PathBuf> {
    let executable_path = Path::new(executable);
    if executable_path.is_file() {
        return Some(executable_path.to_path_buf());
    }
    if executable_path.components().count() > 1 || executable_path.is_absolute() {
        return None;
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(executable))
            .find(|candidate| candidate.is_file())
    })
}

fn relative_evidence_dir(opts: &VerificationOpts) -> PathBuf {
    if opts.evidence_dir.as_os_str().is_empty() {
        return PathBuf::from(format!(".lux/evidence/autonomous/{}", opts.run_id));
    }
    if opts.evidence_dir.is_absolute() {
        opts.evidence_dir
            .strip_prefix(&opts.working_dir)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| PathBuf::from(format!(".lux/evidence/autonomous/{}", opts.run_id)))
    } else {
        opts.evidence_dir.clone()
    }
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn policy_result(
    status: VerificationStatus,
    policy_used: &str,
    evidence_paths: Vec<String>,
    checks: Vec<CheckResult>,
) -> RoutedVerificationResult {
    let passed = status == VerificationStatus::Passed;
    RoutedVerificationResult {
        status,
        policy_used: policy_used.to_string(),
        evidence_paths,
        passed,
        timestamp: Utc::now().to_rfc3339(),
        overall_score: weighted_average_score(&checks),
        checks,
    }
}

fn verification_status_for_passed(passed: bool) -> VerificationStatus {
    if passed {
        VerificationStatus::Passed
    } else {
        VerificationStatus::Failed
    }
}

/// Tier-aware verification result — extends VerificationResult with tier info.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TieredVerificationResult {
    pub base: VerificationResult,
    pub tier_results: Vec<TierResult>,
    pub overall_tier: VerificationTier,
    pub domain_tiers: HashMap<String, VerificationTier>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TierResult {
    pub domain: String,
    pub tier: VerificationTier,
    pub passed: bool,
    pub score: f64,
    pub checks: Vec<CheckResult>,
}

pub fn verify_all(project_path: &Path, mode: VerificationMode) -> Result<VerificationResult> {
    let timestamp = Utc::now().to_rfc3339();
    let spec = lux_spec::lux_load(project_path).ok();
    let checks = match mode {
        VerificationMode::Cached => vec![
            check_spec_completeness(spec.as_ref()),
            check_implementation_exists(project_path, spec.as_ref()),
            check_unity_compilable(project_path),
            check_webgl_playable(project_path),
            check_feedback_integration(project_path, spec.as_ref())?,
        ],
        VerificationMode::Live => live_verification_not_implemented_checks(),
    };
    let overall_score = weighted_average_score(&checks);
    let passed = checks.iter().all(|check| check.passed);
    let mut result = VerificationResult {
        passed,
        timestamp,
        checks,
        overall_score,
        blocker_ticket_ids: Vec::new(),
    };
    result.blocker_ticket_ids = create_blocker_tickets(&result, project_path)?;
    save_verification_result(&result, project_path)?;
    Ok(result)
}

pub fn verify_tiered(
    project_path: &Path,
    profile: &TeamProfile,
    mode: VerificationMode,
) -> Result<TieredVerificationResult> {
    let mut tier_results = Vec::new();
    let mut domain_tiers = HashMap::new();
    let mut all_checks = Vec::new();
    let domains = profile_verification_domains(profile);

    for domain in domains {
        let tier = profile.verification_tier_for_domain(&domain);
        let checks = match tier {
            VerificationTier::T1Always => verify_t1_domain(project_path, &domain)?,
            VerificationTier::T2Bridge => verify_t2_bridge(project_path, &domain)?,
            VerificationTier::T3Gate => verify_t3_gate(project_path, &domain)?,
        };
        let score = weighted_average_score(&checks);
        let passed = checks.iter().all(|check| check.passed);
        all_checks.extend(checks.clone());
        domain_tiers.insert(domain.clone(), tier.clone());
        tier_results.push(TierResult {
            domain,
            tier,
            passed,
            score,
            checks,
        });
    }

    if matches!(mode, VerificationMode::Live) && tier_results.is_empty() {
        eprintln!(
            "Tiered live verification found no configured domains; returning explicit empty result"
        );
    }

    let timestamp = Utc::now().to_rfc3339();
    let overall_score = weighted_average_score(&all_checks);
    let passed = tier_results.iter().all(|result| result.passed) && !tier_results.is_empty();
    let mut base = VerificationResult {
        passed,
        timestamp,
        checks: all_checks,
        overall_score,
        blocker_ticket_ids: Vec::new(),
    };
    base.blocker_ticket_ids = create_blocker_tickets(&base, project_path)?;

    Ok(TieredVerificationResult {
        base,
        tier_results,
        overall_tier: highest_configured_tier(&domain_tiers),
        domain_tiers,
    })
}

pub fn verify_t1_domain(project_path: &Path, domain: &str) -> Result<Vec<CheckResult>> {
    let spec = lux_spec::lux_load(project_path).ok();
    let mut checks = vec![
        check_spec_completeness(spec.as_ref()),
        check_implementation_exists(project_path, spec.as_ref()),
        check_unity_compilable(project_path),
    ];
    checks.push(check_static_syntax_awareness(project_path, domain)?);
    Ok(checks
        .into_iter()
        .map(|mut check| {
            check.name = format!("T1 {domain}: {}", check.name);
            check
        })
        .collect())
}

pub fn verify_t2_bridge(project_path: &Path, domain: &str) -> Result<Vec<CheckResult>> {
    let mut checks = verify_t1_domain(project_path, domain)?;
    match crate::try_ping_unity_bridge_backend(project_path, Duration::from_millis(750)) {
        Ok(ping) => checks.push(check(
            &format!("T2 {domain}: Unity Bridge Connectivity"),
            CheckCategory::UnityCompilable,
            true,
            1.0,
            "Unity bridge is connected and responsive",
            Some(json!({
                "mode": "bridge",
                "verification_basis": "unity_bridge_ping",
                "host": ping.host,
                "port": ping.port,
                "discovery_path": ping.discovery_path.display().to_string(),
            })),
        )),
        Err(error) => {
            eprintln!(
                "T2 verification degraded for domain {domain}: Unity bridge not connected: {error:#}"
            );
            checks.push(check(
                &format!("T2 {domain}: Unity Bridge Connectivity"),
                CheckCategory::UnityCompilable,
                false,
                0.0,
                "T2 bridge verification degraded: Unity bridge is not connected",
                Some(json!({
                    "mode": "bridge",
                    "verification_basis": "unity_bridge_ping",
                    "disposition": "degraded_explicit_bridge_unavailable",
                    "error": error.to_string(),
                })),
            ));
        }
    }
    checks.push(check_cached_unit_test_evidence(project_path, domain));
    Ok(checks)
}

pub fn verify_t3_gate(project_path: &Path, domain: &str) -> Result<Vec<CheckResult>> {
    let mut checks = verify_t2_bridge(project_path, domain)?;
    checks.extend(run_t3_unity_gate(project_path, domain, &checks));
    Ok(checks)
}

pub fn required_tier_for_action(action: &str) -> VerificationTier {
    match action.trim().to_ascii_lowercase().as_str() {
        "commit" => VerificationTier::T1Always,
        "push" => VerificationTier::T2Bridge,
        "milestone_push" => VerificationTier::T3Gate,
        "ship" | "release" => VerificationTier::T3Gate,
        _ => VerificationTier::T1Always,
    }
}

pub fn check_verification_gate(
    result: &TieredVerificationResult,
    required: VerificationTier,
) -> Result<bool> {
    if tier_rank(&result.overall_tier) < tier_rank(&required) {
        bail!(
            "Verification gate requires {:?}, but highest completed tier is {:?}",
            required,
            result.overall_tier
        );
    }

    let failed = result
        .tier_results
        .iter()
        .filter(|tier_result| tier_rank(&tier_result.tier) >= tier_rank(&required))
        .filter(|tier_result| !tier_result.passed)
        .map(|tier_result| format!("{} ({:?})", tier_result.domain, tier_result.tier))
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        bail!(
            "Verification gate {:?} failed for domain(s): {}",
            required,
            failed.join(", ")
        );
    }

    Ok(true)
}

fn live_verification_not_implemented_checks() -> Vec<CheckResult> {
    [
        ("Spec Completeness", CheckCategory::SpecCompleteness),
        ("Implementation Exists", CheckCategory::ImplementationExists),
        ("Unity Compilable", CheckCategory::UnityCompilable),
        ("WebGL Playable", CheckCategory::WebGLPlayable),
        ("Feedback Integration", CheckCategory::FeedbackIntegration),
    ]
    .into_iter()
    .map(|(name, category)| {
        check(
            name,
            category,
            false,
            0.0,
            &format!("Live verification not yet implemented for {name}"),
            Some(json!({ "mode": "live", "verification_basis": "not_implemented" })),
        )
    })
    .collect()
}

pub fn create_blocker_tickets(
    result: &VerificationResult,
    project_path: &Path,
) -> Result<Vec<String>> {
    let store = FileTicketStore::new(project_path);
    let failed_checks = result
        .checks
        .iter()
        .filter(|check| !check.passed)
        .collect::<Vec<_>>();

    if failed_checks.is_empty() {
        reset_blocker_generation_count(project_path)?;
        return Ok(Vec::new());
    }

    let mut run_state = load_or_initialize_run_state(project_path)?;
    let current_ticket_id = run_state.current_ticket_id.clone();
    let config = run_state.continuation_config.clone();
    let mut planned = Vec::new();
    let mut max_depth = run_state.blocker_depth;
    let mut generates_new_blocker = false;

    for check in failed_checks {
        let category = category_tag(&check.category);
        let stable_key =
            stable_blocker_key(category, &check.name, Some(VERIFICATION_BLOCKER_SPEC_REF));
        let stable_id = stable_blocker_ticket_id_from_key(&stable_key);
        let next_attempt = run_state
            .blocker_attempts
            .get(&stable_id)
            .copied()
            .unwrap_or(0)
            + 1;

        if next_attempt > config.max_blocker_attempts_per_ticket {
            quarantine_run_state(
                project_path,
                &mut run_state,
                StopReason::BlockerEscalationRequired,
                "verification blocker attempt limit exceeded",
            )?;
            bail!(StopReason::BlockerEscalationRequired.as_str());
        }

        let existing_stable_ticket = store.get(&stable_id)?;
        let existing_tagged_ticket = if existing_stable_ticket.is_none() {
            store.find_open_blocker_by_stable_key(&stable_key)?
        } else {
            None
        };
        let effective_blocker_id = existing_stable_ticket
            .as_ref()
            .or(existing_tagged_ticket.as_ref())
            .map(|ticket| ticket.id.clone())
            .unwrap_or_else(|| stable_id.clone());

        if existing_stable_ticket.is_none() && existing_tagged_ticket.is_none() {
            generates_new_blocker = true;
        }

        if let Some(blocked_ticket_id) = current_ticket_id.as_deref() {
            if store.blocker_dependency_would_cycle(blocked_ticket_id, &effective_blocker_id)? {
                quarantine_run_state(
                    project_path,
                    &mut run_state,
                    StopReason::BlockerCycleDetected,
                    "verification blocker cycle detected",
                )?;
                bail!(StopReason::BlockerCycleDetected.as_str());
            }
            let prospective_depth =
                store.prospective_blocker_depth(blocked_ticket_id, &effective_blocker_id)?;
            if prospective_depth > config.max_blocker_depth {
                quarantine_run_state(
                    project_path,
                    &mut run_state,
                    StopReason::BlockerEscalationRequired,
                    "verification blocker depth exceeded",
                )?;
                bail!(StopReason::BlockerEscalationRequired.as_str());
            }
            max_depth = max_depth.max(prospective_depth);
        }

        planned.push((check, category, stable_id, next_attempt));
    }

    if generates_new_blocker {
        let next_generation_count = run_state.consecutive_blocker_generations + 1;
        if next_generation_count > config.max_consecutive_blocker_generations {
            quarantine_run_state(
                project_path,
                &mut run_state,
                StopReason::BlockerEscalationRequired,
                "verification blocker generation limit exceeded",
            )?;
            bail!(StopReason::BlockerEscalationRequired.as_str());
        }
        run_state.consecutive_blocker_generations = next_generation_count;
    }

    let mut blocker_ids = Vec::new();
    for (check, category, stable_id, next_attempt) in planned {
        let upsert = create_or_update_blocker(
            &store,
            category,
            &check.name,
            Some(VERIFICATION_BLOCKER_SPEC_REF),
            format!("Verification failed: {}", check.name),
            format!("{}\n\nScore: {:.2}", check.message, check.score),
            blocker_priority(&check.category),
            vec!["verification".to_string(), category.to_string()],
        )?;

        if let Some(blocked_ticket_id) = current_ticket_id.as_deref() {
            store.add_blocker_dependency(blocked_ticket_id, &upsert.ticket.id)?;
        }

        run_state.blocker_attempts.insert(stable_id, next_attempt);
        blocker_ids.push(upsert.ticket.id);
    }

    run_state.blocker_depth = max_depth;
    touch_run_state(&mut run_state);
    run_state.save(project_path)?;

    Ok(blocker_ids)
}

fn load_or_initialize_run_state(project_path: &Path) -> Result<RunState> {
    if RunState::path(project_path).exists() {
        RunState::load(project_path)
    } else {
        bail!(
            "run-state.json not found at {}; run `lux init` first",
            RunState::path(project_path).display()
        )
    }
}

fn reset_blocker_generation_count(project_path: &Path) -> Result<()> {
    if !RunState::path(project_path).exists() {
        return Ok(());
    }
    let mut run_state = RunState::load(project_path)?;
    if run_state.consecutive_blocker_generations == 0 && run_state.blocker_depth == 0 {
        return Ok(());
    }
    run_state.consecutive_blocker_generations = 0;
    run_state.blocker_depth = 0;
    touch_run_state(&mut run_state);
    run_state.save(project_path)
}

pub fn quarantine_run_state(
    project_path: &Path,
    run_state: &mut RunState,
    reason: StopReason,
    note: &str,
) -> Result<()> {
    write_quarantine_evidence(project_path, run_state, reason, note)?;
    run_state.transition_to(RunStatus::Quarantined, reason.as_str())?;
    run_state.last_error = Some(reason.as_str().to_string());
    run_state.stop_reason = Some(reason.as_str().to_string());
    run_state.save(project_path)
}

fn write_quarantine_evidence(
    project_path: &Path,
    run_state: &RunState,
    reason: StopReason,
    note: &str,
) -> Result<()> {
    let run_id = if run_state.run_id.is_empty() {
        "unknown-run"
    } else {
        run_state.run_id.as_str()
    };
    let evidence_path = project_path
        .join(".lux")
        .join("evidence")
        .join("blockers")
        .join(format!("quarantine-{run_id}.json"));
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create evidence dir {}", parent.display()))?;
    }
    crate::lux_io::atomic_write_json(
        &evidence_path,
        &json!({
            "run_id": run_state.run_id,
            "ticket_id": run_state.ticket_id,
            "current_ticket_id": run_state.current_ticket_id,
            "reason": reason.as_str(),
            "note": note,
            "blocker_attempts": run_state.blocker_attempts,
            "written_at": Utc::now().to_rfc3339(),
        }),
    )
}

fn touch_run_state(run_state: &mut RunState) {
    run_state.updated_at = Utc::now().to_rfc3339();
}

pub fn get_latest_verification(project_path: &Path) -> Result<Option<VerificationResult>> {
    let path = verification_dir(project_path).join("latest.json");
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save_verification_result(result: &VerificationResult, project_path: &Path) -> Result<()> {
    let dir = verification_dir(project_path);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let content =
        serde_json::to_string_pretty(result).context("failed to serialize verification result")?;
    fs::write(dir.join("latest.json"), &content).context("failed to write latest verification")?;
    fs::write(dir.join(format!("{}.json", result.timestamp)), content)
        .context("failed to write timestamped verification")?;
    Ok(())
}

/// Cached spec scan: validates `.lux/spec.json` content already on disk.
/// This does not execute generation, linting, or external validation commands.
pub fn check_spec_completeness(spec: Option<&SpecProject>) -> CheckResult {
    let Some(spec) = spec else {
        return check(
            "Spec Completeness",
            CheckCategory::SpecCompleteness,
            false,
            0.0,
            "Spec file .lux/spec.json is missing or unreadable",
            Some(json!({
                "missing": ".lux/spec.json",
                "mode": "cached",
                "verification_basis": "spec_file_scan",
            })),
        );
    };
    let validation_error = spec.validate().err();
    let built_in = built_in_domains(spec);
    let custom = spec
        .domains
        .custom
        .iter()
        .map(|(key, domain)| (key.as_str(), domain));
    let domains = built_in
        .iter()
        .filter_map(|(name, domain)| domain.map(|domain| (*name, domain)))
        .chain(custom)
        .collect::<Vec<_>>();
    let required_missing = built_in
        .iter()
        .filter_map(|(name, domain)| match domain {
            Some(domain) if domain_has_required_fields(domain) => None,
            _ => Some(*name),
        })
        .collect::<Vec<_>>();
    let defined_missing = domains
        .iter()
        .filter_map(|(name, domain)| (!domain.defined).then_some(*name))
        .collect::<Vec<_>>();
    let schell_complete = schell_is_complete(spec);
    let low_ambiguity = domains
        .iter()
        .filter(|(_, domain)| domain.ambiguity_score < 0.5)
        .count();
    let score = ratio(low_ambiguity, domains.len());
    let passed = validation_error.is_none()
        && !domains.is_empty()
        && required_missing.is_empty()
        && defined_missing.is_empty()
        && schell_complete
        && (score - 1.0).abs() < f64::EPSILON;
    check(
        "Spec Completeness",
        CheckCategory::SpecCompleteness,
        passed,
        score,
        if passed {
            "Spec is complete and unambiguous"
        } else {
            "Spec is incomplete or ambiguous"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "spec_file_scan",
            "validation_error": validation_error,
            "domain_count": domains.len(),
            "required_missing": required_missing,
            "defined_missing": defined_missing,
            "schell_complete": schell_complete,
            "low_ambiguity_domains": low_ambiguity,
        })),
    )
}

/// Cached file scan: checks domain content paths and conventional Unity asset paths.
/// This does not compile, run tests, or inspect generated assemblies.
pub fn check_implementation_exists(project_path: &Path, spec: Option<&SpecProject>) -> CheckResult {
    let Some(spec) = spec else {
        return check(
            "Implementation Exists",
            CheckCategory::ImplementationExists,
            false,
            0.0,
            "Cannot inspect implementation without a spec",
            Some(json!({ "mode": "cached", "verification_basis": "implementation_file_scan" })),
        );
    };
    let domains = all_domains(spec);
    let evidence = domains
        .iter()
        .map(|(name, domain)| json!({ "domain": name, "exists": implementation_evidence_exists(project_path, name, domain) }))
        .collect::<Vec<_>>();
    let implemented = evidence
        .iter()
        .filter(|item| item.get("exists").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let score = ratio(implemented, domains.len());
    let passed = !domains.is_empty() && (score - 1.0).abs() < f64::EPSILON;
    check(
        "Implementation Exists",
        CheckCategory::ImplementationExists,
        passed,
        score,
        if passed {
            "Implementation evidence exists for all domains"
        } else {
            "Missing implementation evidence"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "implementation_file_scan",
            "domains": evidence,
        })),
    )
}

/// Cached marker scan: checks for prior build success marker files only.
/// This does not launch Unity or perform a fresh compilation.
pub fn check_unity_compilable(project_path: &Path) -> CheckResult {
    let latest = latest_build_dir(project_path);
    let passed = latest
        .as_ref()
        .is_some_and(|path| has_cached_successful_build_marker(path));
    check(
        "Unity Compilable",
        CheckCategory::UnityCompilable,
        passed,
        if passed { 1.0 } else { 0.0 },
        if passed {
            "Cached successful build marker exists"
        } else {
            "No cached successful build marker found"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "build_marker_scan",
            "latest_build": latest.map(|path| path.display().to_string()),
        })),
    )
}

/// Cached marker scan: checks whether the latest cached build has a non-empty `index.html`.
/// This does not start a browser, web server, or WebGL runtime.
pub fn check_webgl_playable(project_path: &Path) -> CheckResult {
    let latest = latest_build_dir(project_path);
    let index_path = latest.as_ref().map(|path| path.join("index.html"));
    let passed = index_path
        .as_ref()
        .is_some_and(|path| has_cached_webgl_playable_marker(path));
    check(
        "WebGL Playable",
        CheckCategory::WebGLPlayable,
        passed,
        if passed { 1.0 } else { 0.0 },
        if passed {
            "Cached WebGL playable marker exists"
        } else {
            "Cached WebGL playable marker is missing"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "webgl_index_marker_scan",
            "index_html": index_path.map(|path| path.display().to_string()),
        })),
    )
}

/// Cached file scan: compares feedback file mtimes against the spec `updated_at` timestamp.
/// This does not invoke review tooling or semantic diff analysis.
pub fn check_feedback_integration(
    project_path: &Path,
    spec: Option<&SpecProject>,
) -> Result<CheckResult> {
    let feedback = feedback_files(project_path)?;
    let Some(spec) = spec else {
        return Ok(check(
            "Feedback Integration",
            CheckCategory::FeedbackIntegration,
            feedback.is_empty(),
            if feedback.is_empty() { 1.0 } else { 0.0 },
            "Cannot compare feedback without a spec",
            Some(json!({
                "mode": "cached",
                "verification_basis": "feedback_file_mtime_scan",
                "feedback_count": feedback.len(),
            })),
        ));
    };
    if feedback.is_empty() {
        return Ok(check(
            "Feedback Integration",
            CheckCategory::FeedbackIntegration,
            true,
            1.0,
            "No feedback waiting for integration",
            Some(json!({
                "mode": "cached",
                "verification_basis": "feedback_file_mtime_scan",
                "feedback_count": 0,
            })),
        ));
    }
    let spec_updated_at = parse_time(&spec.updated_at);
    let integrated = feedback
        .iter()
        .filter(|path| {
            fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .map(DateTime::<Utc>::from)
                .zip(spec_updated_at)
                .is_some_and(|(feedback_time, spec_time)| spec_time > feedback_time)
        })
        .count();
    let score = ratio(integrated, feedback.len());
    let passed = (score - 1.0).abs() < f64::EPSILON;
    Ok(check(
        "Feedback Integration",
        CheckCategory::FeedbackIntegration,
        passed,
        score,
        if passed {
            "Feedback has been integrated into the spec"
        } else {
            "Feedback is newer than spec updates"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "feedback_file_mtime_scan",
            "feedback_count": feedback.len(),
            "integrated_count": integrated,
        })),
    ))
}

fn check(
    name: &str,
    category: CheckCategory,
    passed: bool,
    score: f64,
    message: &str,
    details: Option<Value>,
) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        category,
        passed,
        score: score.clamp(0.0, 1.0),
        message: message.to_string(),
        details,
    }
}

/// Computes the verification score as an equal-weight weighted average:
/// `sum(check.score * check_weight(check.category)) / sum(check_weight(check.category))`.
/// Every current category has weight `1.0`, making this equivalent to a simple average today.
pub fn weighted_average_score(checks: &[CheckResult]) -> f64 {
    if checks.is_empty() {
        0.0
    } else {
        let weighted_sum = checks
            .iter()
            .map(|check| check.score * check_weight(&check.category))
            .sum::<f64>();
        let weight_sum = checks
            .iter()
            .map(|check| check_weight(&check.category))
            .sum::<f64>();
        weighted_sum / weight_sum
    }
}

fn check_weight(_category: &CheckCategory) -> f64 {
    1.0
}

fn ratio(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

fn blocker_priority(category: &CheckCategory) -> TicketPriority {
    match category {
        CheckCategory::SpecCompleteness => TicketPriority::Critical,
        _ => TicketPriority::High,
    }
}

fn category_tag(category: &CheckCategory) -> &'static str {
    match category {
        CheckCategory::SpecCompleteness => "spec-completeness",
        CheckCategory::ImplementationExists => "implementation-exists",
        CheckCategory::UnityCompilable => "unity-compilable",
        CheckCategory::WebGLPlayable => "webgl-playable",
        CheckCategory::FeedbackIntegration => "feedback-integration",
    }
}

fn verification_dir(project_path: &Path) -> PathBuf {
    project_path.join(".lux/verification")
}

fn built_in_domains(spec: &SpecProject) -> [(&'static str, Option<&DomainSpec>); 13] {
    [
        ("gdd", spec.domains.gdd.as_ref()),
        ("mechanics", spec.domains.mechanics.as_ref()),
        ("controls", spec.domains.controls.as_ref()),
        ("camera", spec.domains.camera.as_ref()),
        ("art_style", spec.domains.art_style.as_ref()),
        ("audio", spec.domains.audio.as_ref()),
        ("narrative", spec.domains.narrative.as_ref()),
        ("levels", spec.domains.levels.as_ref()),
        (
            "technical_architecture",
            spec.domains.technical_architecture.as_ref(),
        ),
        ("engine", spec.domains.engine.as_ref()),
        ("testing", spec.domains.testing.as_ref()),
        ("build_release", spec.domains.build_release.as_ref()),
        ("ui_ux", spec.domains.ui_ux.as_ref()),
    ]
}

fn all_domains(spec: &SpecProject) -> Vec<(String, &DomainSpec)> {
    let mut domains = built_in_domains(spec)
        .into_iter()
        .filter_map(|(name, domain)| domain.map(|domain| (name.to_string(), domain)))
        .collect::<Vec<_>>();
    domains.extend(
        spec.domains
            .custom
            .iter()
            .map(|(name, domain)| (name.clone(), domain)),
    );
    domains
}

fn domain_has_required_fields(domain: &DomainSpec) -> bool {
    domain.defined
        && !domain.name.trim().is_empty()
        && !domain.content_path.trim().is_empty()
        && !domain.fields.is_empty()
}

fn schell_is_complete(spec: &SpecProject) -> bool {
    spec.schell_evaluation.phase1_experience.status != PillarStatus::Missing
        && spec.schell_evaluation.phase2_tetrad.mechanics.status != PillarStatus::Missing
        && spec.schell_evaluation.phase2_tetrad.story.status != PillarStatus::Missing
        && spec.schell_evaluation.phase2_tetrad.aesthetics.status != PillarStatus::Missing
        && spec.schell_evaluation.phase2_tetrad.technology.status != PillarStatus::Missing
        && spec.schell_evaluation.phase3_core_loop.status != PillarStatus::Missing
        && spec.schell_evaluation.phase4_motivation.status != PillarStatus::Missing
        && spec.schell_evaluation.phase5_assessment.status != PillarStatus::Missing
}

fn implementation_evidence_exists(project_path: &Path, name: &str, domain: &DomainSpec) -> bool {
    content_path_exists(project_path, &domain.content_path)
        || match name {
            "gdd" => {
                has_scene_file(&project_path.join("Assets/Scenes"))
                    || has_scene_file(&project_path.join("Assets"))
            }
            "mechanics" => has_any_path(
                project_path,
                &[
                    "Assets/Scripts",
                    "Assets/Gameplay",
                    "src/gameplay",
                    "Scripts/Gameplay",
                ],
            ),
            "controls" => has_any_path(
                project_path,
                &[
                    "Assets/InputSystem_Actions.inputactions",
                    "Assets/Input",
                    "Assets/Controls",
                    "src/input",
                ],
            ),
            "camera" => has_any_path(
                project_path,
                &[
                    "Assets/Cameras",
                    "Assets/Camera",
                    "Assets/Cinemachine",
                    "src/camera",
                ],
            ),
            "technical_architecture" => has_any_path(
                project_path,
                &["Assets/Scripts", "Assets", "src", "Scripts"],
            ),
            "art_style" => has_any_path(
                project_path,
                &["Assets/Art", "Assets/Materials", "Assets/Sprites"],
            ),
            "audio" => has_any_path(project_path, &["Assets/Audio", "Assets/Sounds"]),
            "narrative" => has_any_path(project_path, &["Assets/Dialogue", "Assets/Narrative"]),
            "levels" => has_any_path(project_path, &["Assets/Levels", "Assets/Scenes"]),
            "engine" => has_any_path(
                project_path,
                &[
                    "Packages/manifest.json",
                    "ProjectSettings",
                    "package.json",
                    "addons",
                ],
            ),
            "testing" => has_any_path(
                project_path,
                &[
                    "Assets/Tests",
                    "Tests",
                    "test",
                    "tests",
                    "Packages/packages-lock.json",
                ],
            ),
            "build_release" => has_any_path(
                project_path,
                &[".github/workflows", "builds", "Build", "Builds", "dist"],
            ),
            "ui_ux" => has_any_path(project_path, &["Assets/UI", "Assets/Prefabs/UI"]),
            _ => false,
        }
}

fn content_path_exists(project_path: &Path, content_path: &str) -> bool {
    let path = Path::new(content_path);
    if path.is_absolute() {
        path.exists()
    } else {
        project_path.join(path).exists()
    }
}

fn has_any_path(project_path: &Path, paths: &[&str]) -> bool {
    paths.iter().any(|path| project_path.join(path).exists())
}

fn has_scene_file(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            let path = entry.path();
            path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("unity")
        })
}

fn latest_build_dir(project_path: &Path) -> Option<PathBuf> {
    fs::read_dir(project_path.join(".lux/builds"))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let modified = entry.metadata().and_then(|meta| meta.modified()).ok()?;
            path.is_dir().then_some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn has_cached_successful_build_marker(path: &Path) -> bool {
    path.join("success.json").exists()
        || path.join("success.txt").exists()
        || path.join("build.json").exists()
        || has_cached_webgl_playable_marker(&path.join("index.html"))
}

fn has_cached_webgl_playable_marker(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

fn feedback_files(project_path: &Path) -> Result<Vec<PathBuf>> {
    let logs_dir = project_path.join(".lux/logs");
    if !logs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in
        fs::read_dir(&logs_dir).with_context(|| format!("failed to read {}", logs_dir.display()))?
    {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".feedback.json"))
        {
            files.push(path);
        }
    }
    Ok(files)
}

fn profile_verification_domains(profile: &TeamProfile) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut domains = profile
        .verification_tiers
        .iter()
        .filter_map(|entry| {
            let domain = entry.domain.trim();
            if domain.is_empty() || domain == "*" || !seen.insert(domain.to_string()) {
                None
            } else {
                Some(domain.to_string())
            }
        })
        .collect::<Vec<_>>();
    if domains.is_empty() {
        domains.push("all".to_string());
    }
    domains
}

fn highest_configured_tier(domain_tiers: &HashMap<String, VerificationTier>) -> VerificationTier {
    domain_tiers
        .values()
        .max_by_key(|tier| tier_rank(tier))
        .cloned()
        .unwrap_or(VerificationTier::T1Always)
}

fn tier_rank(tier: &VerificationTier) -> u8 {
    match tier {
        VerificationTier::T1Always => 1,
        VerificationTier::T2Bridge => 2,
        VerificationTier::T3Gate => 3,
    }
}

fn check_static_syntax_awareness(project_path: &Path, domain: &str) -> Result<CheckResult> {
    let roots = [project_path.join("Assets"), project_path.join("src")];
    let mut scanned = 0usize;
    let mut suspicious = Vec::new();
    for root in roots.iter().filter(|root| root.exists()) {
        scan_source_syntax(root, &mut scanned, &mut suspicious)?;
    }
    let passed = suspicious.is_empty();
    Ok(check(
        &format!("Static Syntax Awareness ({domain})"),
        CheckCategory::UnityCompilable,
        passed,
        if passed { 1.0 } else { 0.0 },
        if passed {
            "No obvious Rust/C# syntax imbalance found in static scan"
        } else {
            "Static scan found possible Rust/C# syntax imbalance"
        },
        Some(json!({
            "mode": "cached",
            "verification_basis": "rust_cs_static_syntax_scan",
            "scanned_files": scanned,
            "suspicious_files": suspicious,
        })),
    ))
}

fn scan_source_syntax(
    root: &Path,
    scanned: &mut usize,
    suspicious: &mut Vec<String>,
) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            scan_source_syntax(&path, scanned, suspicious)?;
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext != "cs" && ext != "rs" {
            continue;
        }
        *scanned += 1;
        let content = fs::read_to_string(&path).unwrap_or_default();
        if brace_balance(&content) != 0 {
            suspicious.push(path.display().to_string());
        }
    }
    Ok(())
}

fn brace_balance(content: &str) -> i32 {
    content.chars().fold(0, |balance, ch| match ch {
        '{' => balance + 1,
        '}' => balance - 1,
        _ => balance,
    })
}

fn check_cached_unit_test_evidence(project_path: &Path, domain: &str) -> CheckResult {
    let candidates = [
        project_path.join(".lux/test-results/latest.json"),
        project_path.join(".lux/test-results/latest.xml"),
        project_path.join("TestResults.xml"),
    ];
    let evidence = candidates
        .iter()
        .find(|path| path.exists())
        .map(|path| path.display().to_string());
    check(
        &format!("T2 {domain}: Unit Test Evidence"),
        CheckCategory::UnityCompilable,
        evidence.is_some(),
        if evidence.is_some() { 1.0 } else { 0.0 },
        if evidence.is_some() {
            "Cached unit test evidence exists"
        } else {
            "No cached unit test result evidence found"
        },
        Some(json!({
            "mode": "bridge",
            "verification_basis": "unit_test_result_marker_scan",
            "evidence": evidence,
        })),
    )
}

fn run_t3_unity_gate(
    project_path: &Path,
    domain: &str,
    t2_checks: &[CheckResult],
) -> Vec<CheckResult> {
    let target = match crate::resolve_unity_launch_target(project_path) {
        Ok(target) => target,
        Err(error) => {
            eprintln!("T3 verification unavailable for domain {domain}: {error:#}");
            return vec![unity_unavailable_check(domain, Some(error.to_string()))];
        }
    };
    run_t3_unity_gate_with_target(project_path, domain, t2_checks, &target)
}

pub fn run_t3_unity_gate_with_target(
    project_path: &Path,
    domain: &str,
    t2_checks: &[CheckResult],
    target: &crate::UnityLaunchTarget,
) -> Vec<CheckResult> {
    run_t3_unity_gate_with_target_and_timeouts(
        project_path,
        domain,
        t2_checks,
        target,
        T3UnityGateTimeouts::default(),
    )
}

pub fn run_t3_unity_gate_with_target_and_timeouts(
    project_path: &Path,
    domain: &str,
    t2_checks: &[CheckResult],
    target: &crate::UnityLaunchTarget,
    timeouts: T3UnityGateTimeouts,
) -> Vec<CheckResult> {
    if !t2_checks.iter().all(|check| check.passed) {
        return vec![check(
            &format!("T3 {domain}: T2 Prerequisites"),
            CheckCategory::UnityCompilable,
            false,
            0.0,
            "T3 gate blocked because one or more T2 bridge checks failed",
            Some(json!({
                "mode": "live",
                "verification_basis": "t2_prerequisite_gate",
                "failed_t2_checks": t2_checks
                    .iter()
                    .filter(|check| !check.passed)
                    .map(|check| check.name.clone())
                    .collect::<Vec<_>>(),
            })),
        )];
    }

    let executable = match validated_unity_executable(&target.executable) {
        Some(executable) => executable,
        None => return vec![unity_unavailable_check(domain, None)],
    };
    let evidence_dir = t3_evidence_dir(project_path, domain);
    if let Err(error) = fs::create_dir_all(&evidence_dir) {
        return vec![check(
            &format!("T3 {domain}: Evidence Directory"),
            CheckCategory::UnityCompilable,
            false,
            0.0,
            "T3 gate failed to create evidence directory",
            Some(json!({
                "mode": "live",
                "verification_basis": "t3_evidence_directory",
                "evidence_path": evidence_dir.display().to_string(),
                "error": error.to_string(),
            })),
        )];
    }

    let compile = run_t3_compile(
        project_path,
        domain,
        target,
        &executable,
        &evidence_dir,
        timeouts.compile_secs,
    );
    if !compile.passed {
        return vec![compile];
    }
    let scene_smoke = run_t3_scene_smoke(
        project_path,
        domain,
        target,
        &executable,
        &evidence_dir,
        timeouts.scene_smoke_secs,
    );
    vec![compile, scene_smoke]
}

fn run_t3_compile(
    project_path: &Path,
    domain: &str,
    target: &crate::UnityLaunchTarget,
    executable: &Path,
    evidence_dir: &Path,
    timeout_secs: u64,
) -> CheckResult {
    let build_path = evidence_dir.join("build");
    let output = run_command_with_timeout(
        ProcessCommand::new(executable)
            .args(&target.prefix_args)
            .arg("-batchmode")
            .arg("-projectPath")
            .arg(project_path)
            .arg("-executeMethod")
            .arg(T3_COMPILE_METHOD)
            .arg("-buildTarget")
            .arg(T3_BUILD_TARGET)
            .arg("-buildPath")
            .arg(&build_path),
        Duration::from_secs(timeout_secs),
    );
    command_check(
        domain,
        "Unity Batchmode Compile",
        "unity_batchmode_compile",
        "Unity batchmode compile completed successfully",
        "Unity batchmode compile failed",
        output,
        evidence_dir,
        timeout_secs,
        None,
    )
}

fn run_t3_scene_smoke(
    project_path: &Path,
    domain: &str,
    target: &crate::UnityLaunchTarget,
    executable: &Path,
    evidence_dir: &Path,
    timeout_secs: u64,
) -> CheckResult {
    let results_dir = project_path.join("TestResults");
    if let Err(error) = fs::create_dir_all(&results_dir) {
        return check(
            &format!("T3 {domain}: Unity Scene Smoke"),
            CheckCategory::UnityCompilable,
            false,
            0.0,
            "Unity scene smoke failed to create TestResults directory",
            Some(json!({
                "mode": "live",
                "verification_basis": "unity_scene_smoke",
                "evidence_path": evidence_dir.display().to_string(),
                "error": error.to_string(),
            })),
        );
    }
    let log_path = results_dir.join("LuxSceneSmoke.log");
    let output = run_command_with_timeout(
        ProcessCommand::new(executable)
            .args(&target.prefix_args)
            .arg("-batchmode")
            .arg("-nographics")
            .arg("-projectPath")
            .arg(project_path)
            .arg("-executeMethod")
            .arg(T3_SCENE_SMOKE_METHOD)
            .arg("-logFile")
            .arg(&log_path),
        Duration::from_secs(timeout_secs),
    );
    let log_text = fs::read_to_string(&log_path).unwrap_or_default();
    let stderr_or_log_error = output
        .as_ref()
        .ok()
        .and_then(|maybe| maybe.as_ref())
        .is_some_and(|out| contains_case_insensitive_error(&String::from_utf8_lossy(&out.stderr)))
        || contains_case_insensitive_error(&log_text);
    let scan_failure = stderr_or_log_error.then_some("Unity scene smoke stderr/log contains error");
    let mut result = command_check(
        domain,
        "Unity Scene Smoke",
        "unity_scene_smoke",
        "Unity scene smoke completed successfully",
        "Unity scene smoke failed",
        output,
        evidence_dir,
        timeout_secs,
        scan_failure,
    );
    if !log_text.is_empty() {
        let copied_log = evidence_dir.join("LuxSceneSmoke.log");
        let _ = fs::write(&copied_log, log_text);
        if let Some(details) = result.details.as_mut().and_then(Value::as_object_mut) {
            details.insert(
                "scene_smoke_log".to_string(),
                json!(copied_log.display().to_string()),
            );
        }
    }
    result
}

fn command_check(
    domain: &str,
    label: &str,
    basis: &str,
    success_message: &str,
    failure_message: &str,
    output: Result<Option<Output>>,
    evidence_dir: &Path,
    timeout_secs: u64,
    scan_failure: Option<&str>,
) -> CheckResult {
    let name = format!("T3 {domain}: {label}");
    match output {
        Ok(Some(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stdout_path =
                write_evidence_text(evidence_dir, &format!("{basis}.stdout.txt"), &stdout);
            let stderr_path =
                write_evidence_text(evidence_dir, &format!("{basis}.stderr.txt"), &stderr);
            let passed = output.status.success() && scan_failure.is_none();
            check(
                &name,
                CheckCategory::UnityCompilable,
                passed,
                if passed { 1.0 } else { 0.0 },
                if passed {
                    success_message
                } else {
                    scan_failure.unwrap_or(failure_message)
                },
                Some(json!({
                    "mode": "live",
                    "verification_basis": basis,
                    "status": output.status.code(),
                    "timeout_secs": timeout_secs,
                    "evidence_path": evidence_dir.display().to_string(),
                    "stdout_path": stdout_path.map(|path| path.display().to_string()),
                    "stderr_path": stderr_path.map(|path| path.display().to_string()),
                    "stderr": stderr.chars().take(4096).collect::<String>(),
                })),
            )
        }
        Ok(None) => check(
            &name,
            CheckCategory::UnityCompilable,
            false,
            0.0,
            &format!("{label} timed out after {timeout_secs} seconds"),
            Some(json!({
                "mode": "live",
                "verification_basis": basis,
                "disposition": "timeout",
                "timeout_secs": timeout_secs,
                "evidence_path": evidence_dir.display().to_string(),
            })),
        ),
        Err(error) => check(
            &name,
            CheckCategory::UnityCompilable,
            false,
            0.0,
            &format!("{label} failed to launch"),
            Some(json!({
                "mode": "live",
                "verification_basis": basis,
                "disposition": "explicit_launch_failure",
                "timeout_secs": timeout_secs,
                "evidence_path": evidence_dir.display().to_string(),
                "error": error.to_string(),
            })),
        ),
    }
}

fn run_command_with_timeout(
    command: &mut ProcessCommand,
    timeout: Duration,
) -> Result<Option<Output>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn Unity command")?;
    let start = std::time::Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map(Some).map_err(Into::into);
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn unity_unavailable_check(domain: &str, error: Option<String>) -> CheckResult {
    check(
        &format!("T3 {domain}: Unity Executable"),
        CheckCategory::UnityCompilable,
        false,
        0.0,
        "Unity executable unavailable; milestone push blocked",
        Some(json!({
            "mode": "live",
            "verification_basis": "unity_executable_discovery",
            "disposition": "hard_unity_unavailable",
            "error": error,
        })),
    )
}

fn validated_unity_executable(executable: &Path) -> Option<PathBuf> {
    if executable.as_os_str().is_empty() {
        return None;
    }
    if executable.is_file() {
        return Some(executable.to_path_buf());
    }
    if executable.components().count() > 1 || executable.is_absolute() {
        return None;
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(executable))
            .find(|candidate| candidate.is_file())
    })
}

fn t3_evidence_dir(project_path: &Path, domain: &str) -> PathBuf {
    project_path
        .join(".lux")
        .join("verification")
        .join("t3")
        .join(domain.replace(['/', '\\', ':'], "_"))
}

fn write_evidence_text(dir: &Path, name: &str, text: &str) -> Option<PathBuf> {
    let path = dir.join(name);
    fs::write(&path, text).ok().map(|_| path)
}

fn contains_case_insensitive_error(text: &str) -> bool {
    text.lines()
        .any(|line| line.to_ascii_lowercase().contains("error"))
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lux_ticket::{TicketFilter, TicketStatus, TicketStore};
    use chrono::Utc;
    use std::fs;

    struct TestProject {
        path: PathBuf,
    }

    impl TestProject {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("lux-verification-{name}-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(path.join(".lux")).expect(".lux directory should be created");
            Self { path }
        }

        fn opts(&self, run_id: &str) -> VerificationOpts {
            VerificationOpts {
                run_id: run_id.to_string(),
                working_dir: self.path.clone(),
                evidence_dir: PathBuf::from(format!(".lux/evidence/autonomous/{run_id}")),
            }
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn ticket_with_policy(policy: &str, commands: Vec<&str>) -> Ticket {
        let now = Utc::now().to_rfc3339();
        Ticket {
            id: format!("ticket-{}", uuid::Uuid::new_v4()),
            title: "Router test ticket".to_string(),
            description: "Ticket for verification router tests".to_string(),
            status: TicketStatus::ToDo,
            priority: TicketPriority::High,
            assignee: None,
            blockers: Vec::new(),
            tags: Vec::new(),
            spec_ref: Some("engine".to_string()),
            created_at: now.clone(),
            updated_at: now,
            execution_objective: Some("Verify engine router behavior".to_string()),
            allowed_executor: None,
            dispatch_policy: None,
            verification_policy: Some(policy.to_string()),
            command_allowlist: Some(commands.into_iter().map(ToOwned::to_owned).collect()),
            evidence_refs: None,
            blocker_policy: None,
            non_goals: None,
        }
    }

    #[test]
    fn route_verification_command_suite_writes_evidence() {
        let project = TestProject::new("command-suite");
        let ticket = ticket_with_policy("command_suite:printf ok", Vec::new());

        let result = route_verification(&ticket, &project.opts("run-command-suite"))
            .expect("command_suite should route");

        assert_eq!(result.status, VerificationStatus::Passed);
        assert_eq!(result.policy_used, "command_suite:printf ok");
        assert_eq!(result.evidence_paths.len(), 1);
        let evidence = project.path.join(&result.evidence_paths[0]);
        assert!(
            evidence.is_file(),
            "command_suite route should write evidence"
        );
        let text = fs::read_to_string(evidence).expect("evidence should be readable");
        assert!(text.contains("policy=command_suite:printf ok"));
        assert!(text.contains("command=printf ok"));
    }

    #[test]
    fn h7_unknown_engine_policy_is_unsupported_with_evidence() {
        let project = TestProject::new("unknown-policy");
        let ticket = ticket_with_policy("renpy_cli", Vec::new());

        let result = route_verification(&ticket, &project.opts("run-unknown-policy"))
            .expect("unsupported policy should route to blocker evidence");

        assert_eq!(result.status, VerificationStatus::Unsupported);
        assert_eq!(result.policy_used, "renpy_cli");
        assert!(!result.passed);
        assert_eq!(result.evidence_paths.len(), 1);
        let evidence = fs::read_to_string(project.path.join(&result.evidence_paths[0]))
            .expect("unsupported evidence should be readable");
        assert!(evidence.contains("unsupported_reason=Unsupported verification_policy: renpy_cli"));
        assert!(
            evidence.contains("\"policy\":\"renpy_cli\""),
            "unsupported evidence must name the policy"
        );
    }

    #[test]
    fn h7_unity_uloop_mock_writes_compile_test_screenshot_evidence() {
        let project = TestProject::new("unity-uloop");
        let ticket = ticket_with_policy(
            "unity_uloop",
            vec!["printf compile", "printf run-tests", "printf screenshot"],
        );

        let result = route_verification(&ticket, &project.opts("run-unity-uloop"))
            .expect("unity_uloop should route through mocked commands");

        assert_eq!(result.status, VerificationStatus::Passed);
        assert_eq!(result.policy_used, "unity_uloop");
        assert_eq!(result.evidence_paths.len(), 3);
        for path in &result.evidence_paths {
            assert!(project.path.join(path).is_file());
        }
        let all_evidence = result
            .evidence_paths
            .iter()
            .map(|path| fs::read_to_string(project.path.join(path)).expect("evidence readable"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all_evidence.contains("phase=compile"));
        assert!(all_evidence.contains("phase=test"));
        assert!(all_evidence.contains("phase=screenshot"));
    }

    #[test]
    fn h7_godot_cli_missing_path_creates_blocker_evidence() {
        let project = TestProject::new("godot-missing");
        let ticket = ticket_with_policy("godot_cli", vec!["__lux_missing_godot_cli__"]);

        let result = route_verification(&ticket, &project.opts("run-godot-missing"))
            .expect("missing Godot CLI should create blocker evidence");

        assert_eq!(result.status, VerificationStatus::Unsupported);
        assert_eq!(result.policy_used, "godot_cli");
        assert_eq!(result.evidence_paths.len(), 1);
        let evidence = fs::read_to_string(project.path.join(&result.evidence_paths[0]))
            .expect("blocker evidence should be readable");
        assert!(evidence.contains("missing CLI"));
        assert!(evidence.contains("\"policy\":\"godot_cli\""));

        let blockers = FileTicketStore::new(&project.path)
            .list(TicketFilter::default())
            .expect("blocker tickets should list");
        assert!(
            blockers
                .iter()
                .any(|ticket| ticket.title.contains("godot_cli")
                    && ticket.description.contains("missing CLI")),
            "missing Godot CLI must create a blocker ticket"
        );
    }
}
