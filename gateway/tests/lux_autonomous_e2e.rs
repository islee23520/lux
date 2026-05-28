use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use lux::{
    lux_loop::{create_executor_blocker, schedule_retry_ready},
    lux_run_state::{RunState, RunStatus},
    lux_ticket::{
        is_execution_grade, should_dispatch, validate_execution_grade, BlockerPolicy,
        DispatchPolicy, FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus,
        TicketStore,
    },
    lux_ticket_executor::{Executor, ExecutorOpts, ExecutorStatus, FakeExecutor, NoopSink},
    lux_verification::{route_verification, VerificationOpts, VerificationStatus},
};

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "lux-autonomous-e2e-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(path.join(".lux")).expect(".lux should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn execution_grade_ticket(id: &str) -> Ticket {
    let now = Utc::now().to_rfc3339();
    Ticket {
        id: id.to_string(),
        title: "Autonomous MVP ticket".to_string(),
        description: "End-to-end autonomous execution ticket for M6 pipeline".to_string(),
        status: TicketStatus::ToDo,
        priority: TicketPriority::High,
        assignee: Some("opencode".to_string()),
        blockers: Vec::new(),
        tags: Vec::new(),
        spec_ref: Some("design".to_string()),
        created_at: now.clone(),
        updated_at: now,
        execution_objective: Some("Implement the M6 autonomous pipeline feature".to_string()),
        allowed_executor: Some("opencode".to_string()),
        dispatch_policy: Some(DispatchPolicy::DispatchRequested),
        verification_policy: Some("command_suite:echo ok".to_string()),
        command_allowlist: Some(vec!["echo".to_string()]),
        evidence_refs: None,
        blocker_policy: Some(BlockerPolicy {
            max_depth: Some(3),
            max_attempts: Some(3),
        }),
        non_goals: Some(vec!["out of scope".to_string()]),
    }
}

fn executor_opts(project_path: &Path, run_id: &str, ticket_id: &str) -> ExecutorOpts {
    ExecutorOpts {
        run_id: run_id.to_string(),
        ticket_id: ticket_id.to_string(),
        working_dir: project_path.to_path_buf(),
        timeout_secs: 30,
    }
}

fn verification_opts(project_path: &Path, run_id: &str) -> VerificationOpts {
    VerificationOpts {
        run_id: run_id.to_string(),
        working_dir: project_path.to_path_buf(),
        evidence_dir: PathBuf::from(format!(".lux/evidence/autonomous/{run_id}")),
    }
}

fn run_state(project_path: &Path, run_id: &str) -> RunState {
    let mut state = RunState::idle(project_path).expect("idle state should construct");
    state.run_id = run_id.to_string();
    state
}

fn write_evidence(project_path: &Path, run_id: &str, content: &str) -> PathBuf {
    let dir = project_path
        .join(".lux")
        .join("evidence")
        .join("autonomous")
        .join(run_id);
    fs::create_dir_all(&dir).expect("evidence dir should be created");
    let path = dir.join("verify_1.txt");
    fs::write(&path, content).expect("evidence file should be written");
    path
}

#[test]
fn autonomous_mvp_happy_path() {
    let temp = TestTempDir::new("happy");
    let run_id = "run-autonomous-happy";
    let ticket = execution_grade_ticket("ticket-autonomous-happy");

    FileTicketStore::new(temp.path())
        .create(ticket.clone())
        .expect("execution-grade ticket should be created");

    assert!(
        is_execution_grade(&ticket),
        "ticket must be execution-grade"
    );
    assert!(
        validate_execution_grade(&ticket).is_ok(),
        "ticket must pass validation"
    );
    assert!(should_dispatch(&ticket), "ticket must be dispatchable");

    let mut state = run_state(temp.path(), run_id);
    state
        .transition_to(RunStatus::ExecutingTicket, "dispatch approved")
        .expect("transition to ExecutingTicket");
    state.current_ticket_id = Some(ticket.id.clone());
    state.ticket_id = Some(ticket.id.clone());
    state.save(temp.path()).expect("run-state should save");

    let opts = executor_opts(temp.path(), run_id, &ticket.id);
    let executor = FakeExecutor::success(run_id);
    let result = executor
        .execute(&ticket, &opts, &NoopSink)
        .expect("fake executor should succeed");

    assert_eq!(result.status, ExecutorStatus::Success);
    assert_eq!(result.exit_code, Some(0));
    assert!(
        !result.evidence_refs.is_empty(),
        "evidence_refs must be populated on success"
    );

    let v_opts = verification_opts(temp.path(), run_id);
    let v_result = route_verification(&ticket, &v_opts).expect("command_suite policy should route");

    assert_eq!(v_result.status, VerificationStatus::Passed);
    assert_eq!(v_result.policy_used, "command_suite:echo ok");
    assert!(
        !v_result.evidence_paths.is_empty(),
        "verification must produce evidence paths"
    );
    assert!(
        temp.path().join(&v_result.evidence_paths[0]).is_file(),
        "evidence file must exist on disk"
    );

    state
        .transition_to(RunStatus::Verifying, "executor succeeded")
        .expect("transition to Verifying");
    state
        .save(temp.path())
        .expect("run-state should save after verifying");

    state
        .transition_to(RunStatus::Completed, "verification passed")
        .expect("transition to Completed");
    state.stop_reason = Some("milestone_complete".to_string());
    state
        .save(temp.path())
        .expect("run-state should save after completion");

    let loaded = RunState::load(temp.path()).expect("run-state should load from disk");
    assert_eq!(loaded.status, RunStatus::Completed.to_string());
    assert_eq!(loaded.stop_reason.as_deref(), Some("milestone_complete"));
    assert_eq!(loaded.run_id, run_id);

    let blockers = FileTicketStore::new(temp.path())
        .list(TicketFilter {
            status: Some(TicketStatus::Blocked),
            ..TicketFilter::default()
        })
        .expect("blocker list should succeed");
    assert!(
        blockers.is_empty(),
        "happy path must produce zero blocker tickets"
    );

    let evidence_path = write_evidence(
        temp.path(),
        run_id,
        &format!(
            "run_id={run_id}\nstatus=completed\nevidence_refs={:?}\nverification_policy={}\n",
            result.evidence_refs, v_result.policy_used
        ),
    );
    assert!(evidence_path.is_file(), "evidence file must be written");
}

#[test]
fn autonomous_mvp_failure_path() {
    let temp = TestTempDir::new("failure");
    let run_id = "run-autonomous-failure";
    let ticket = execution_grade_ticket("ticket-autonomous-failure");

    FileTicketStore::new(temp.path())
        .create(ticket.clone())
        .expect("execution-grade ticket should be created");

    let mut state = run_state(temp.path(), run_id);
    state
        .transition_to(RunStatus::ExecutingTicket, "dispatch approved")
        .expect("transition to ExecutingTicket");
    state.current_ticket_id = Some(ticket.id.clone());
    state.ticket_id = Some(ticket.id.clone());
    state.save(temp.path()).expect("run-state should save");

    let opts = executor_opts(temp.path(), run_id, &ticket.id);
    let executor = FakeExecutor::failed(run_id, 1);
    let result = executor
        .execute(&ticket, &opts, &NoopSink)
        .expect("fake executor should return failure result without error");

    assert_eq!(result.status, ExecutorStatus::Failed);
    assert_eq!(result.exit_code, Some(1));
    assert!(
        !result.evidence_refs.is_empty(),
        "evidence_refs must be populated on failure"
    );

    let blocker_id =
        create_executor_blocker(temp.path(), run_id, &ticket.id, &result.status, &mut state)
            .expect("executor blocker should be created on first failure");

    assert!(
        !blocker_id.is_empty(),
        "blocker ticket id must be non-empty"
    );
    assert_eq!(
        state.status,
        RunStatus::Blocked.to_string(),
        "first failure must transition run-state to Blocked (not Quarantined)"
    );
    assert_eq!(
        state.consecutive_failures, 0,
        "consecutive_failures counter is managed by the loop, not create_executor_blocker"
    );

    let blockers = FileTicketStore::new(temp.path())
        .list(TicketFilter {
            status: Some(TicketStatus::Blocked),
            ..TicketFilter::default()
        })
        .expect("blocker list should succeed");
    assert_eq!(
        blockers.len(),
        1,
        "exactly one blocker ticket must exist after first failure"
    );
    assert!(
        blockers[0].tags.iter().any(|tag| tag == "executor"),
        "blocker ticket must carry the 'executor' tag"
    );

    let retry_state = schedule_retry_ready(temp.path(), run_id)
        .expect("blocked run should be schedulable for retry");
    assert_eq!(
        retry_state.status,
        RunStatus::RetryReady.to_string(),
        "schedule_retry_ready must yield RetryReady"
    );

    let evidence_path = write_evidence(
        temp.path(),
        run_id,
        &format!(
            "run_id={run_id}\nstatus=blocked\nblocker_id={blocker_id}\nexit_code={:?}\n",
            result.exit_code
        ),
    );
    assert!(
        evidence_path.is_file(),
        "failure evidence file must be written"
    );
}
