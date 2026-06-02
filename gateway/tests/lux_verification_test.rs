use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use lux::lux_run_state::{RunState, RunStatus, StopReason};
use lux::lux_spec::{lux_init, lux_load, lux_save, DomainSpec, PillarStatus, SpecProject};
use lux::lux_team_profile::VerificationTier;
use lux::lux_ticket::{
    stable_blocker_ticket_id, FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus,
    TicketStore,
};
use lux::lux_verification::{
    check_feedback_integration, check_implementation_exists, check_spec_completeness,
    check_unity_compilable, check_webgl_playable, create_blocker_tickets, required_tier_for_action,
    route_verification, run_t3_unity_gate_with_target, run_t3_unity_gate_with_target_and_timeouts,
    verify_all, weighted_average_score, CheckCategory, CheckResult, T3UnityGateTimeouts,
    VerificationMode, VerificationOpts, VerificationResult, VerificationStatus,
    T3_COMPILE_TIMEOUT_SECS, T3_SCENE_SMOKE_TIMEOUT_SECS,
};
use lux::UnityLaunchTarget;
use serde_json::{json, Value};

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-verification-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("temp directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn init_with_complete_spec(&self) -> SpecProject {
        lux_init(&self.path).expect("lux workspace should initialize");
        RunState::idle(&self.path)
            .expect("idle run state should construct")
            .save(&self.path)
            .expect("run-state.json should save");
        let mut spec = lux_load(&self.path).expect("spec should load");
        make_spec_complete(&mut spec);
        lux_save(&self.path, &spec).expect("complete spec should save");
        spec
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn make_domain(name: &str) -> DomainSpec {
    let mut domain = DomainSpec::new(name, format!(".lux/domains/{name}.md"), 0.0);
    domain.defined = true;
    domain
        .fields
        .insert("summary".to_string(), json!("complete"));
    domain
}

fn make_spec_complete(spec: &mut SpecProject) {
    spec.domains.gdd = Some(make_domain("gdd"));
    spec.domains.mechanics = Some(make_domain("mechanics"));
    spec.domains.controls = Some(make_domain("controls"));
    spec.domains.camera = Some(make_domain("camera"));
    spec.domains.art_style = Some(make_domain("art_style"));
    spec.domains.audio = Some(make_domain("audio"));
    spec.domains.narrative = Some(make_domain("narrative"));
    spec.domains.levels = Some(make_domain("levels"));
    spec.domains.technical_architecture = Some(make_domain("technical_architecture"));
    spec.domains.engine = Some(make_domain("engine"));
    spec.domains.testing = Some(make_domain("testing"));
    spec.domains.build_release = Some(make_domain("build_release"));
    spec.domains.ui_ux = Some(make_domain("ui_ux"));
    spec.schell_evaluation.phase1_experience.status = PillarStatus::Strong;
    spec.schell_evaluation.phase2_tetrad.mechanics.status = PillarStatus::Strong;
    spec.schell_evaluation.phase2_tetrad.story.status = PillarStatus::Strong;
    spec.schell_evaluation.phase2_tetrad.aesthetics.status = PillarStatus::Strong;
    spec.schell_evaluation.phase2_tetrad.technology.status = PillarStatus::Strong;
    spec.schell_evaluation.phase3_core_loop.status = PillarStatus::Strong;
    spec.schell_evaluation.phase4_motivation.status = PillarStatus::Strong;
    spec.schell_evaluation.phase5_assessment.status = PillarStatus::Strong;
    spec.overall_ambiguity = 0.0;
}

fn create_domain_files(project_path: &Path) {
    for name in [
        "gdd",
        "mechanics",
        "controls",
        "camera",
        "art_style",
        "audio",
        "narrative",
        "levels",
        "technical_architecture",
        "engine",
        "testing",
        "build_release",
        "ui_ux",
    ] {
        let path = project_path.join(format!(".lux/domains/{name}.md"));
        fs::create_dir_all(path.parent().expect("domain file should have parent"))
            .expect("domain directory should be created");
        fs::write(path, format!("# {name}\n")).expect("domain file should be written");
    }
}

fn create_build(project_path: &Path, name: &str, success_marker: bool, webgl_marker: bool) {
    let build_dir = project_path.join(".lux/builds").join(name);
    fs::create_dir_all(&build_dir).expect("build directory should be created");
    if success_marker {
        fs::write(build_dir.join("success.json"), "{}").expect("success marker should be written");
    }
    if webgl_marker {
        fs::write(build_dir.join("index.html"), "<html></html>")
            .expect("webgl marker should be written");
    }
}

fn score_check(name: &str, category: CheckCategory, score: f64) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        category,
        passed: (score - 1.0).abs() < f64::EPSILON,
        score,
        message: String::new(),
        details: None,
    }
}

fn failing_result(name: &str, category: CheckCategory) -> VerificationResult {
    VerificationResult {
        passed: false,
        timestamp: "2026-05-15T00:00:00Z".to_string(),
        checks: vec![CheckResult {
            name: name.to_string(),
            category,
            passed: false,
            score: 0.0,
            message: "verification evidence missing".to_string(),
            details: None,
        }],
        overall_score: 0.0,
        blocker_ticket_ids: Vec::new(),
    }
}

fn test_ticket(id: &str, status: TicketStatus, blockers: Vec<String>) -> Ticket {
    Ticket {
        id: id.to_string(),
        title: format!("Ticket {id}"),
        description: "test ticket".to_string(),
        status,
        priority: TicketPriority::High,
        assignee: None,
        blockers,
        tags: Vec::new(),
        spec_ref: None,
        created_at: "2026-05-15T00:00:00Z".to_string(),
        updated_at: "2026-05-15T00:00:00Z".to_string(),
        execution_objective: None,
        allowed_executor: None,
        dispatch_policy: None,
        verification_policy: None,
        command_allowlist: None,
        evidence_refs: None,
        blocker_policy: None,
        non_goals: None,
    }
}

fn save_run_state(project_path: &Path, current_ticket_id: Option<&str>) {
    let mut run_state = RunState::idle(project_path).expect("idle run state should be created");
    run_state.status = RunStatus::Verifying.to_string();
    run_state.current_ticket_id = current_ticket_id.map(ToOwned::to_owned);
    run_state.save(project_path).expect("run state should save");
}

fn passed_t2_checks() -> Vec<CheckResult> {
    vec![CheckResult {
        name: "T2 testing: mocked bridge pass".to_string(),
        category: CheckCategory::UnityCompilable,
        passed: true,
        score: 1.0,
        message: "mocked T2 pass".to_string(),
        details: Some(json!({ "verification_basis": "test_mock" })),
    }]
}

fn fake_unity(project: &TestProject, mode: &str) -> PathBuf {
    let path = project.path().join(format!("fake-unity-{mode}.sh"));
    let script = format!(
        r#"#!/bin/sh
method=""
log=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -executeMethod) shift; method="$1" ;;
    -logFile) shift; log="$1" ;;
  esac
  shift
done
case "$method" in
  Linalab.Lux.Editor.LuxBatchAutomation.Compile)
    if [ "{mode}" = "compile-timeout" ]; then sleep 2; fi
    printf 'compile ok\n'
    exit 0
    ;;
  Linalab.Lux.Editor.LuxSceneSmoke.Run)
    if [ "{mode}" = "scene-error" ]; then
      printf 'scene stderr Error marker\n' >&2
      [ -n "$log" ] && printf 'clean line\nERROR from smoke log\n' > "$log"
      exit 0
    fi
    [ -n "$log" ] && printf 'scene smoke clean\n' > "$log"
    printf 'scene ok\n'
    exit 0
    ;;
esac
exit 7
"#
    );
    fs::write(&path, script).expect("fake unity script should be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }
    path
}

fn unity_target(executable: PathBuf) -> UnityLaunchTarget {
    UnityLaunchTarget {
        executable,
        prefix_args: Vec::new(),
    }
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let previous = env::var_os(key);
        unsafe {
            env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.as_ref() {
            unsafe {
                env::set_var(self.key, previous);
            }
        } else {
            unsafe {
                env::remove_var(self.key);
            }
        }
    }
}

fn verification_opts(project: &TestProject, run_id: &str) -> VerificationOpts {
    VerificationOpts {
        run_id: run_id.to_string(),
        working_dir: project.path().to_path_buf(),
        evidence_dir: PathBuf::from(format!(".lux/evidence/autonomous/{run_id}")),
    }
}

fn verification_ticket(policy: Option<&str>) -> Ticket {
    let mut ticket = test_ticket("ticket-verify", TicketStatus::InProgress, Vec::new());
    ticket.verification_policy = policy.map(ToOwned::to_owned);
    ticket
}

fn detail<'a>(check: &'a CheckResult, key: &str) -> &'a Value {
    check
        .details
        .as_ref()
        .and_then(|details| details.get(key))
        .expect("check detail should exist")
}

#[test]
fn verification_policy_unity_selects_t3_path() {
    let project = TestProject::new("policy-unity-t3");
    let executable = fake_unity(&project, "pass");
    let _guard = EnvVarGuard::set("LUX_UNITY_EDITOR", &executable);
    let ticket = verification_ticket(Some("unity_t3"));
    let opts = verification_opts(&project, "run-unity-t3");

    let result = route_verification(&ticket, &opts).expect("unity_t3 policy should route");

    assert_eq!(result.status, VerificationStatus::Passed);
    assert_eq!(result.policy_used, "unity_t3");
    assert!(result
        .checks
        .iter()
        .any(|check| check.name.contains("Unity Batchmode Compile")));
    assert!(result
        .checks
        .iter()
        .any(|check| check.name.contains("Unity Scene Smoke")));
    assert_eq!(
        result.evidence_paths,
        vec![".lux/evidence/autonomous/run-unity-t3/verify_1.txt"]
    );
    assert!(project.path().join(&result.evidence_paths[0]).is_file());
}

#[test]
fn verification_policy_command_suite_runs_declared() {
    let project = TestProject::new("policy-command-suite");
    let ticket = verification_ticket(Some("command_suite:echo ok"));
    let opts = verification_opts(&project, "run-command-suite");

    let result = route_verification(&ticket, &opts).expect("command suite should route");

    assert_eq!(result.status, VerificationStatus::Passed);
    assert_eq!(result.policy_used, "command_suite:echo ok");
    assert_eq!(
        result.evidence_paths,
        vec![".lux/evidence/autonomous/run-command-suite/verify_1.txt"]
    );
    let evidence = fs::read_to_string(project.path().join(&result.evidence_paths[0]))
        .expect("command evidence should be readable");
    assert!(evidence.contains("command=echo ok"));
    assert!(evidence.contains("exit_code=0"));
    assert!(evidence.contains("ok"));
}

#[test]
fn verification_policy_doc_only_requires_command() {
    let project = TestProject::new("policy-doc-only");
    let ticket = verification_ticket(Some("doc_only"));
    let opts = verification_opts(&project, "run-doc-only");

    let error =
        route_verification(&ticket, &opts).expect_err("doc_only without command should fail");

    assert!(error
        .to_string()
        .contains("doc_only verification requires at least one grep/schema validation command"));
}

#[test]
fn verification_live_unsupported() {
    let project = TestProject::new("policy-live");
    let ticket = verification_ticket(Some("live"));
    let opts = verification_opts(&project, "run-live");

    let error = route_verification(&ticket, &opts).expect_err("live policy should be unsupported");

    assert!(error
        .to_string()
        .contains("VerificationMode::Live is not supported in M6"));
    assert!(!project
        .path()
        .join(".lux/evidence/autonomous/run-live")
        .exists());
}

#[test]
fn verification_missing_policy_blocks_dispatch() {
    let project = TestProject::new("policy-missing");
    let ticket = verification_ticket(None);
    let opts = verification_opts(&project, "run-missing");

    let error =
        route_verification(&ticket, &opts).expect_err("missing policy should block dispatch");

    assert!(error
        .to_string()
        .contains("verification_policy is required for execution-grade dispatch"));
}

#[test]
fn lux_verification_check_spec_completeness_passes_with_valid_spec() {
    let project = TestProject::new("valid-spec");
    let spec = project.init_with_complete_spec();

    let check = check_spec_completeness(Some(&spec));

    assert!(check.passed);
    assert_eq!(check.score, 1.0);
    assert_eq!(detail(&check, "verification_basis"), "spec_file_scan");
}

#[test]
fn lux_verification_check_spec_completeness_fails_with_missing_fields() {
    let project = TestProject::new("missing-fields");
    let mut spec = project.init_with_complete_spec();
    spec.domains.gdd.as_mut().unwrap().fields.clear();

    let check = check_spec_completeness(Some(&spec));

    assert!(!check.passed);
    assert_eq!(check.score, 1.0);
    assert!(detail(&check, "required_missing")
        .as_array()
        .unwrap()
        .contains(&json!("gdd")));
}

#[test]
fn lux_verification_check_spec_completeness_fails_with_empty_domains() {
    let project = TestProject::new("empty-domains");
    lux_init(project.path()).expect("lux workspace should initialize");
    let spec = lux_load(project.path()).expect("default spec should load");

    let check = check_spec_completeness(Some(&spec));

    assert!(!check.passed);
    assert_eq!(check.score, 0.0);
    assert_eq!(detail(&check, "domain_count"), 0);
}

#[test]
fn lux_verification_check_implementation_exists_passes_with_files_present() {
    let project = TestProject::new("implementation-present");
    let spec = project.init_with_complete_spec();
    create_domain_files(project.path());

    let check = check_implementation_exists(project.path(), Some(&spec));

    assert!(check.passed);
    assert_eq!(check.score, 1.0);
    assert_eq!(
        detail(&check, "verification_basis"),
        "implementation_file_scan"
    );
}

#[test]
fn lux_verification_check_implementation_exists_fails_with_missing_files() {
    let project = TestProject::new("implementation-missing");
    let spec = project.init_with_complete_spec();
    fs::remove_dir_all(project.path().join(".lux/domains"))
        .expect("domain files should be removed");

    let check = check_implementation_exists(project.path(), Some(&spec));

    assert!(!check.passed);
    assert_eq!(check.score, 0.0);
    assert!(check.message.contains("Missing implementation evidence"));
    assert_eq!(detail(&check, "domains").as_array().unwrap().len(), 13);
}

#[test]
fn lux_verification_check_unity_compilable_uses_cached_build_markers() {
    let project = TestProject::new("unity-markers");

    let missing = check_unity_compilable(project.path());
    assert!(!missing.passed);

    create_build(project.path(), "build-1", true, false);
    let present = check_unity_compilable(project.path());
    assert!(present.passed);
    assert_eq!(detail(&present, "verification_basis"), "build_marker_scan");
}

#[test]
fn lux_verification_check_webgl_playable_uses_cached_index_marker() {
    let project = TestProject::new("webgl-markers");
    create_build(project.path(), "build-1", true, false);

    let missing = check_webgl_playable(project.path());
    assert!(!missing.passed);

    fs::write(
        project.path().join(".lux/builds/build-1/index.html"),
        "<html></html>",
    )
    .expect("index marker should be written");
    let present = check_webgl_playable(project.path());
    assert!(present.passed);
    assert_eq!(
        detail(&present, "verification_basis"),
        "webgl_index_marker_scan"
    );
}

#[test]
fn lux_verification_check_feedback_integration_passes_without_feedback_files() {
    let project = TestProject::new("feedback-none");
    let spec = project.init_with_complete_spec();

    let check =
        check_feedback_integration(project.path(), Some(&spec)).expect("feedback check should run");

    assert!(check.passed);
    assert_eq!(check.score, 1.0);
    assert_eq!(detail(&check, "feedback_count"), 0);
}

#[test]
fn lux_verification_check_feedback_integration_fails_with_pending_feedback_files() {
    let project = TestProject::new("feedback-pending");
    let mut spec = project.init_with_complete_spec();
    spec.updated_at = "2000-01-01T00:00:00Z".to_string();
    let logs_dir = project.path().join(".lux/logs");
    fs::create_dir_all(&logs_dir).expect("logs directory should be created");
    fs::write(logs_dir.join("playtest.feedback.json"), "{}").expect("feedback should be written");

    let check =
        check_feedback_integration(project.path(), Some(&spec)).expect("feedback check should run");

    assert!(!check.passed);
    assert_eq!(check.score, 0.0);
    assert_eq!(detail(&check, "feedback_count"), 1);
}

#[test]
fn lux_verification_verify_all_passes_when_all_cached_evidence_exists() {
    let project = TestProject::new("verify-all-pass");
    project.init_with_complete_spec();
    create_domain_files(project.path());
    create_build(project.path(), "build-1", true, true);

    let result =
        verify_all(project.path(), VerificationMode::Cached).expect("verification should complete");

    assert!(result.passed);
    assert_eq!(result.overall_score, 1.0);
    assert!(result.blocker_ticket_ids.is_empty());
}

#[test]
fn lux_verification_verify_all_reports_mixed_cached_results() {
    let project = TestProject::new("verify-all-mixed");
    project.init_with_complete_spec();
    create_domain_files(project.path());

    let result =
        verify_all(project.path(), VerificationMode::Cached).expect("verification should complete");

    assert!(!result.passed);
    assert_eq!(result.overall_score, 0.6);
    assert_eq!(result.checks.iter().filter(|check| check.passed).count(), 3);
    assert_eq!(result.blocker_ticket_ids.len(), 2);
}

#[test]
fn lux_verification_verify_all_reports_all_fail_when_no_cached_evidence_exists() {
    let project = TestProject::new("verify-all-fail");
    lux_init(project.path()).expect("lux workspace should initialize");
    RunState::idle(project.path())
        .expect("idle run state should construct")
        .save(project.path())
        .expect("run-state.json should save");
    let logs_dir = project.path().join(".lux/logs");
    fs::create_dir_all(&logs_dir).expect("logs directory should be created");
    fs::write(logs_dir.join("playtest.feedback.json"), "{}").expect("feedback should be written");

    let result =
        verify_all(project.path(), VerificationMode::Cached).expect("verification should complete");

    assert!(!result.passed);
    assert_eq!(result.overall_score, 0.0);
    assert!(result.checks.iter().all(|check| !check.passed));
}

#[test]
fn lux_verification_score_calculation_uses_documented_equal_weights() {
    let checks = vec![
        score_check("Spec", CheckCategory::SpecCompleteness, 1.0),
        score_check("Implementation", CheckCategory::ImplementationExists, 0.5),
        score_check("Unity", CheckCategory::UnityCompilable, 0.0),
        score_check("WebGL", CheckCategory::WebGLPlayable, 1.0),
    ];

    let score = weighted_average_score(&checks);

    assert_eq!(score, 0.625);
}

#[test]
fn lux_verification_milestone_push_requires_t3_gate() {
    assert_eq!(
        required_tier_for_action("milestone_push"),
        VerificationTier::T3Gate
    );
    assert_eq!(required_tier_for_action("push"), VerificationTier::T2Bridge);
}

#[test]
fn lux_verification_t3_timeout_constants_are_explicit() {
    assert_eq!(T3_COMPILE_TIMEOUT_SECS, 600);
    assert_eq!(T3_SCENE_SMOKE_TIMEOUT_SECS, 300);
    assert_eq!(T3UnityGateTimeouts::default().compile_secs, 600);
    assert_eq!(T3UnityGateTimeouts::default().scene_smoke_secs, 300);
}

#[test]
fn lux_verification_t3_unity_unavailable_hard_fails() {
    let project = TestProject::new("t3-unity-unavailable");

    let checks = run_t3_unity_gate_with_target(
        project.path(),
        "testing",
        &passed_t2_checks(),
        &unity_target(PathBuf::new()),
    );

    assert_eq!(checks.len(), 1);
    assert!(!checks[0].passed);
    assert_eq!(
        checks[0].message,
        "Unity executable unavailable; milestone push blocked"
    );
    assert_eq!(detail(&checks[0], "disposition"), "hard_unity_unavailable");
}

#[test]
fn lux_verification_t3_scene_smoke_fails_on_case_insensitive_error_log() {
    let project = TestProject::new("t3-scene-error");
    let executable = fake_unity(&project, "scene-error");

    let checks = run_t3_unity_gate_with_target(
        project.path(),
        "testing",
        &passed_t2_checks(),
        &unity_target(executable),
    );

    assert_eq!(checks.len(), 2);
    assert!(checks[0].passed);
    assert!(!checks[1].passed);
    assert_eq!(
        checks[1].message,
        "Unity scene smoke stderr/log contains error"
    );
    assert_eq!(
        detail(&checks[1], "verification_basis"),
        "unity_scene_smoke"
    );
}

#[test]
fn lux_verification_t3_compile_timeout_hard_fails() {
    let project = TestProject::new("t3-compile-timeout");
    let executable = fake_unity(&project, "compile-timeout");

    let checks = run_t3_unity_gate_with_target_and_timeouts(
        project.path(),
        "testing",
        &passed_t2_checks(),
        &unity_target(executable),
        T3UnityGateTimeouts {
            compile_secs: 1,
            scene_smoke_secs: 1,
        },
    );

    assert_eq!(checks.len(), 1);
    assert!(!checks[0].passed);
    assert_eq!(detail(&checks[0], "disposition"), "timeout");
    assert_eq!(detail(&checks[0], "timeout_secs"), 1);
}

#[test]
fn lux_verification_t3_passes_with_t2_compile_scene_and_records_evidence() {
    let project = TestProject::new("t3-pass");
    let executable = fake_unity(&project, "pass");

    let checks = run_t3_unity_gate_with_target(
        project.path(),
        "testing",
        &passed_t2_checks(),
        &unity_target(executable),
    );

    assert_eq!(checks.len(), 2);
    assert!(checks.iter().all(|check| check.passed));
    let evidence_path = detail(&checks[1], "evidence_path")
        .as_str()
        .expect("evidence path should be a string");
    assert!(Path::new(evidence_path).is_dir());
    assert!(Path::new(evidence_path).join("LuxSceneSmoke.log").is_file());
}

#[test]
fn lux_verification_creates_blocker_tickets_on_failure() {
    let project = TestProject::new("blocker-tickets");
    project.init_with_complete_spec();

    let result =
        verify_all(project.path(), VerificationMode::Cached).expect("verification should complete");

    assert!(!result.blocker_ticket_ids.is_empty());
    for id in &result.blocker_ticket_ids {
        assert!(project
            .path()
            .join(".lux/tickets")
            .join(format!("{id}.json"))
            .exists());
    }
}

#[test]
fn lux_verification_live_mode_reports_not_implemented_without_cached_fallback() {
    let project = TestProject::new("live-mode");
    project.init_with_complete_spec();
    create_domain_files(project.path());
    create_build(project.path(), "build-1", true, true);

    let result = verify_all(project.path(), VerificationMode::Live)
        .expect("live verification should produce explicit unsupported checks");

    assert!(!result.passed);
    assert_eq!(result.overall_score, 0.0);
    assert!(result.checks.iter().all(|check| check
        .message
        .starts_with("Live verification not yet implemented for")));
}

#[test]
fn lux_verification_quarantines_blocker_cycle() {
    let project = TestProject::new("blocker-cycle");
    let store = FileTicketStore::new(project.path());
    let blocker_id = stable_blocker_ticket_id(
        "unity-compilable",
        "Cycle Check",
        Some(".lux/verification/latest.json"),
    );
    store
        .create(test_ticket(
            &blocker_id,
            TicketStatus::Blocked,
            vec!["ticket-b".to_string()],
        ))
        .expect("blocker ticket should be created");
    store
        .create(test_ticket("ticket-b", TicketStatus::ToDo, Vec::new()))
        .expect("blocked ticket should be created");
    save_run_state(project.path(), Some("ticket-b"));

    let error = create_blocker_tickets(
        &failing_result("Cycle Check", CheckCategory::UnityCompilable),
        project.path(),
    )
    .expect_err("cycle should quarantine run");

    assert!(error
        .to_string()
        .contains(StopReason::BlockerCycleDetected.as_str()));
    let run_state = RunState::load(project.path()).expect("run state should load");
    assert_eq!(run_state.status, RunStatus::Quarantined.to_string());
    assert_eq!(
        run_state.last_error.as_deref(),
        Some(StopReason::BlockerCycleDetected.as_str())
    );
}

#[test]
fn lux_verification_quarantines_after_blocker_attempt_limit() {
    let project = TestProject::new("blocker-attempts");
    save_run_state(project.path(), None);
    let result = failing_result("Attempt Check", CheckCategory::UnityCompilable);

    for _ in 0..3 {
        create_blocker_tickets(&result, project.path()).expect("attempt should stay in bounds");
    }
    let error = create_blocker_tickets(&result, project.path())
        .expect_err("fourth attempt should quarantine run");

    assert!(error
        .to_string()
        .contains(StopReason::BlockerEscalationRequired.as_str()));
    let run_state = RunState::load(project.path()).expect("run state should load");
    assert_eq!(run_state.status, RunStatus::Quarantined.to_string());
    assert_eq!(
        run_state.last_error.as_deref(),
        Some(StopReason::BlockerEscalationRequired.as_str())
    );
}

#[test]
fn lux_verification_repeated_same_failure_converges_to_one_blocker() {
    let project = TestProject::new("blocker-idempotency");
    save_run_state(project.path(), None);
    let result = failing_result("Stable Check", CheckCategory::WebGLPlayable);

    let first = create_blocker_tickets(&result, project.path()).expect("first blocker creation");
    let second = create_blocker_tickets(&result, project.path()).expect("second blocker update");
    let third = create_blocker_tickets(&result, project.path()).expect("third blocker update");

    assert_eq!(first, second);
    assert_eq!(second, third);
    let tickets = FileTicketStore::new(project.path())
        .list(TicketFilter::default())
        .expect("tickets should list");
    assert_eq!(tickets.len(), 1);
}

#[test]
fn lux_verification_allows_blocked_ticket_after_blocker_done() {
    let project = TestProject::new("blocker-resolution");
    let store = FileTicketStore::new(project.path());
    store
        .create(test_ticket("ticket-a", TicketStatus::ToDo, Vec::new()))
        .expect("target ticket should be created");
    save_run_state(project.path(), Some("ticket-a"));

    let blocker_ids = create_blocker_tickets(
        &failing_result("Resolution Check", CheckCategory::ImplementationExists),
        project.path(),
    )
    .expect("blocker should be created");
    assert_eq!(blocker_ids.len(), 1);

    let target = store
        .get("ticket-a")
        .expect("target ticket read")
        .expect("target ticket exists");
    assert_eq!(target.status, TicketStatus::Blocked);
    assert_eq!(target.blockers, blocker_ids);

    let mut blocked_target = target.clone();
    blocked_target.status = TicketStatus::ToDo;
    assert!(store.update("ticket-a", blocked_target).is_err());

    let blocker_id = &target.blockers[0];
    let mut blocker = store
        .get(blocker_id)
        .expect("blocker read")
        .expect("blocker exists");
    blocker.status = TicketStatus::ToDo;
    let blocker = store
        .update(blocker_id, blocker)
        .expect("blocker should move to ToDo");
    let mut blocker = blocker;
    blocker.status = TicketStatus::InProgress;
    let blocker = store
        .update(blocker_id, blocker)
        .expect("blocker should move to InProgress");
    let mut blocker = blocker;
    blocker.status = TicketStatus::Done;
    store
        .update(blocker_id, blocker)
        .expect("blocker should move to Done");

    let mut target = store
        .get("ticket-a")
        .expect("target read")
        .expect("target exists");
    target.status = TicketStatus::ToDo;
    store
        .update("ticket-a", target)
        .expect("target should proceed after blocker is Done");

    let run_state = RunState::load(project.path()).expect("run state should load");
    assert!(run_state
        .blocker_attempts
        .values()
        .all(|attempt| *attempt <= 3));
    assert!(run_state.blocker_depth <= 3);
}

#[test]
fn test_create_blocker_tickets_errors_when_run_state_missing() {
    let project = TestProject::new("no-run-state");
    lux_init(project.path()).expect("lux_init should succeed");
    RunState::idle(project.path())
        .expect("idle run state should construct")
        .save(project.path())
        .expect("run-state.json should save");

    let result = lux::lux_verification::verify_all(project.path(), VerificationMode::Cached);
    let verification = result.expect("verify_all should return a result");
    fs::remove_file(project.path().join(".lux/run-state.json"))
        .expect("run-state.json should be removable");
    let failed_result = VerificationResult {
        checks: vec![CheckResult {
            name: "dummy".to_string(),
            category: CheckCategory::SpecCompleteness,
            passed: false,
            score: 0.0,
            message: "forced failure".to_string(),
            details: None,
        }],
        ..verification
    };
    let err = create_blocker_tickets(&failed_result, project.path())
        .expect_err("create_blocker_tickets must error when run-state.json is missing");
    assert!(
        err.to_string().contains("run-state.json not found"),
        "unexpected error: {err}"
    );
}
