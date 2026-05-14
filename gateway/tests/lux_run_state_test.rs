use std::{
    fs,
    path::{Path, PathBuf},
};

use lux::lux_run_state::{
    ApprovalGateType, ContinuationRunConfig, RunState, RunStatus, StopReason,
    RUN_STATE_SCHEMA_VERSION,
};
use serde_json::json;

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-run-state-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("temp directory should be created");
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

#[test]
fn test_run_state_idle_creates_valid_state() {
    let temp_dir = TestTempDir::new("idle");

    let state = RunState::idle(temp_dir.path()).expect("idle state should be created");

    assert_eq!(state.schema_version, RUN_STATE_SCHEMA_VERSION);
    assert_eq!(state.seq, 0);
    assert_eq!(state.run_id, "");
    assert_eq!(state.status, RunStatus::Idle.to_string());
    assert_eq!(state.goal_id, None);
    assert_eq!(state.milestone_id, None);
    assert_eq!(state.current_ticket_id, None);
    assert_eq!(state.approval.gate, None);
    assert_eq!(state.resume.checkpoint, None);
    assert_eq!(state.executor.kind, None);
    assert_eq!(state.last_error, None);
    chrono::DateTime::parse_from_rfc3339(&state.updated_at).expect("updated_at should be RFC3339");
}

#[test]
fn test_run_state_save_load_roundtrip() {
    let temp_dir = TestTempDir::new("roundtrip");
    let mut state = RunState::idle(temp_dir.path()).expect("idle state should be created");
    state.seq = 7;
    state.run_id = "run-123".to_string();
    state.status = RunStatus::AwaitingApproval.to_string();
    state.goal_id = Some("goal-1".to_string());
    state.milestone_id = Some("milestone-2".to_string());
    state.current_ticket_id = Some("ticket-3".to_string());
    state.approval.gate = Some(ApprovalGateType::ApproveDiff.to_string());
    state.approval.pending_transition = Some(RunStatus::ExecutingTicket.to_string());
    state.approval.created_at = Some("2026-05-14T00:00:00Z".to_string());
    state.resume.previous_status = Some(RunStatus::Planning.to_string());
    state.resume.checkpoint = Some("encoded-checkpoint".to_string());
    state.executor.kind = Some("opencode".to_string());
    state.executor.job_id = Some("job-4".to_string());
    state.executor.heartbeat_at = Some("2026-05-14T00:00:01Z".to_string());
    state.last_error = Some("blocked by compiler".to_string());
    state.updated_at = "2026-05-14T00:00:02Z".to_string();

    state.save(temp_dir.path()).expect("save should succeed");
    let loaded = RunState::load(temp_dir.path()).expect("load should succeed");

    assert_eq!(loaded, state);
}

#[test]
fn test_run_state_transition_increments_seq() {
    let temp_dir = TestTempDir::new("transition");
    let mut state = RunState::idle(temp_dir.path()).expect("idle state should be created");
    let initial_updated_at = state.updated_at.clone();

    state
        .transition_to(RunStatus::Planning, "start planning")
        .expect("transition should succeed");
    assert_eq!(state.status, RunStatus::Planning.to_string());
    assert_eq!(state.seq, 1);

    state
        .transition_to(RunStatus::Verifying, "verify work")
        .expect("transition should succeed");
    assert_eq!(state.status, RunStatus::Verifying.to_string());
    assert_eq!(state.seq, 2);
    assert!(state.updated_at >= initial_updated_at);
}

#[test]
fn test_run_state_rejects_future_schema() {
    let temp_dir = TestTempDir::new("future-schema");
    let path = RunState::path(temp_dir.path());
    fs::create_dir_all(path.parent().expect("run-state path should have parent"))
        .expect(".lux directory should be created");
    fs::write(
        &path,
        json!({
            "schemaVersion": RUN_STATE_SCHEMA_VERSION + 1,
            "seq": 0,
            "runId": "run-future",
            "status": "Idle",
            "goalId": null,
            "milestoneId": null,
            "currentTicketId": null,
            "approval": {},
            "resume": {},
            "executor": {},
            "lastError": null,
            "updatedAt": "2026-05-14T00:00:00Z"
        })
        .to_string(),
    )
    .expect("future schema file should be written");

    let error = RunState::load(temp_dir.path()).expect_err("future schema should fail");

    assert!(
        error.to_string().contains("newer than supported version"),
        "unexpected error: {error}"
    );
}

#[test]
fn test_run_state_missing_file_errors() {
    let temp_dir = TestTempDir::new("missing");

    let error = RunState::load(temp_dir.path()).expect_err("missing file should fail");

    assert!(
        error.to_string().contains("run-state.json not found"),
        "unexpected error: {error}"
    );
}

#[test]
fn test_migration_from_continuation_state() {
    let temp_dir = TestTempDir::new("migration");
    let lux_dir = temp_dir.path().join(".lux");
    fs::create_dir_all(&lux_dir).expect(".lux directory should be created");
    fs::write(
        lux_dir.join("continuation-state.json"),
        json!({
            "current_ticket_id": "ticket-123",
            "status": "Active",
            "inFlight": true
        })
        .to_string(),
    )
    .expect("legacy continuation state should be written");

    let migrated = RunState::migrate_legacy_continuation_state(temp_dir.path())
        .expect("migration should succeed");
    let state = RunState::load(temp_dir.path()).expect("migrated run-state should load");

    assert!(migrated);
    assert_eq!(state.current_ticket_id, Some("ticket-123".to_string()));
    assert_eq!(state.status, RunStatus::ExecutingTicket.to_string());
    assert_eq!(state.executor.kind, Some("opencode".to_string()));
    assert!(lux_dir.join("continuation-state.json.deprecated").exists());
    assert!(!lux_dir.join("continuation-state.json").exists());
}

#[test]
fn test_migration_noop_if_no_legacy() {
    let temp_dir = TestTempDir::new("migration-noop");

    let migrated = RunState::migrate_legacy_continuation_state(temp_dir.path())
        .expect("migration noop should succeed");

    assert!(!migrated);
    assert!(!RunState::path(temp_dir.path()).exists());
}

fn write_legacy_state(temp_dir: &TestTempDir, legacy: serde_json::Value) {
    let lux_dir = temp_dir.path().join(".lux");
    fs::create_dir_all(&lux_dir).expect(".lux directory should be created");
    fs::write(lux_dir.join("continuation-state.json"), legacy.to_string())
        .expect("legacy continuation state should be written");
}

fn migrate_and_load(temp_dir: &TestTempDir) -> RunState {
    assert!(RunState::migrate_legacy_continuation_state(temp_dir.path())
        .expect("migration should succeed"));
    RunState::load(temp_dir.path()).expect("migrated run-state should load")
}

#[test]
fn test_migration_uses_deterministic_continuation_status_mapping() {
    let cases = [
        (
            "complete",
            json!({ "status": "Complete" }),
            RunStatus::Completed,
        ),
        (
            "active-in-flight",
            json!({ "status": "Active", "inFlight": true }),
            RunStatus::ExecutingTicket,
        ),
        (
            "active-ticket",
            json!({ "status": "Active", "current_ticket_id": "ticket-7" }),
            RunStatus::ExecutingTicket,
        ),
        (
            "active-planning",
            json!({ "status": "Active" }),
            RunStatus::Planning,
        ),
        (
            "stopped-all-complete",
            json!({ "status": "Stopped", "stop_reason": "all_complete" }),
            RunStatus::Completed,
        ),
        (
            "stopped-milestone-complete",
            json!({ "status": "Stopped", "stop_reason": "milestone_complete" }),
            RunStatus::Completed,
        ),
        (
            "stopped-other",
            json!({ "status": "Stopped", "stop_reason": "user_abort" }),
            RunStatus::Interrupted,
        ),
        (
            "error-blocker-cycle",
            json!({ "status": "Error", "stop_reason": "blocker_cycle_detected" }),
            RunStatus::Quarantined,
        ),
        (
            "error-blocker-escalation",
            json!({ "status": "Error", "stop_reason": "blocker_escalation_required" }),
            RunStatus::Quarantined,
        ),
        (
            "error-other",
            json!({ "status": "Error", "stop_reason": "consecutive_failure_limit" }),
            RunStatus::Failed,
        ),
        ("idle", json!({ "status": "Idle" }), RunStatus::Idle),
    ];

    for (name, legacy, expected) in cases {
        let temp_dir = TestTempDir::new(name);
        write_legacy_state(&temp_dir, legacy);

        let state = migrate_and_load(&temp_dir);

        assert_eq!(state.status, expected.to_string(), "case {name}");
    }
}

#[test]
fn test_migration_preserves_continuation_counters() {
    let temp_dir = TestTempDir::new("migration-counters");
    write_legacy_state(
        &temp_dir,
        json!({
            "status": "Active",
            "continuation_count": 8,
            "stagnation_count": 2,
            "consecutive_failures": 1
        }),
    );

    let state = migrate_and_load(&temp_dir);

    assert_eq!(state.continuation_count, 8);
    assert_eq!(state.stagnation_count, 2);
    assert_eq!(state.consecutive_failures, 1);
}

#[test]
fn test_run_state_save_rejects_invalid_status_before_disk_write() {
    let temp_dir = TestTempDir::new("invalid-status-save");
    let mut state = RunState::idle(temp_dir.path()).expect("idle state should be created");
    state
        .save(temp_dir.path())
        .expect("initial save should succeed");
    let original = fs::read_to_string(RunState::path(temp_dir.path()))
        .expect("original run-state should exist");

    state.status = "Active".to_string();
    let error = state
        .save(temp_dir.path())
        .expect_err("legacy status should be rejected before write");

    assert!(
        error.to_string().contains("unknown RunStatus: Active"),
        "unexpected error: {error}"
    );
    let after =
        fs::read_to_string(RunState::path(temp_dir.path())).expect("run-state should still exist");
    assert_eq!(after, original);
}

#[test]
fn test_run_state_load_rejects_invalid_status() {
    let temp_dir = TestTempDir::new("invalid-status-load");
    let path = RunState::path(temp_dir.path());
    fs::create_dir_all(path.parent().expect("run-state path should have parent"))
        .expect(".lux directory should be created");
    fs::write(
        &path,
        json!({
            "schemaVersion": RUN_STATE_SCHEMA_VERSION,
            "seq": 0,
            "runId": "run-invalid",
            "status": "Active",
            "goalId": null,
            "milestoneId": null,
            "currentTicketId": null,
            "approval": {},
            "resume": {},
            "executor": {},
            "lastError": null,
            "preTaskGitSha": null,
            "teamRunId": null,
            "updatedAt": "2026-05-14T00:00:00Z"
        })
        .to_string(),
    )
    .expect("invalid run-state should be written");

    let error = RunState::load(temp_dir.path()).expect_err("invalid status should fail load");

    assert!(
        error.to_string().contains("unknown RunStatus: Active"),
        "unexpected error: {error}"
    );
}

#[test]
fn test_run_state_load_rejects_corrupt_json() {
    let temp_dir = TestTempDir::new("corrupt-json-load");
    let path = RunState::path(temp_dir.path());
    fs::create_dir_all(path.parent().expect("run-state path should have parent"))
        .expect(".lux directory should be created");
    fs::write(&path, "{not valid json").expect("corrupt run-state should be written");

    let error = RunState::load(temp_dir.path()).expect_err("corrupt state should fail load");

    assert!(
        error.to_string().contains("failed to parse run-state file"),
        "unexpected error: {error}"
    );
}

#[test]
fn test_recovering_and_awaiting_play_start_roundtrip_without_legacy_collapse() {
    for status in [RunStatus::Recovering, RunStatus::AwaitingPlayStart] {
        let temp_dir = TestTempDir::new(&format!("roundtrip-{status}"));
        let mut state = RunState::idle(temp_dir.path()).expect("idle state should be created");
        state.status = status.to_string();

        state.save(temp_dir.path()).expect("save should succeed");
        let loaded = RunState::load(temp_dir.path()).expect("load should succeed");

        assert_eq!(loaded.status, status.to_string());
    }
}

#[test]
fn test_canonical_stop_reasons_and_numeric_defaults() {
    assert_eq!(
        StopReason::MaxContinuationsReached.as_str(),
        "max_continuations_reached"
    );
    assert_eq!(
        StopReason::MaxIterationsReached.as_str(),
        "max_iterations_reached"
    );
    assert_eq!(StopReason::StagnationLimit.as_str(), "stagnation_limit");
    assert_eq!(
        StopReason::ConsecutiveFailureLimit.as_str(),
        "consecutive_failure_limit"
    );
    assert_eq!(StopReason::MilestoneComplete.as_str(), "milestone_complete");
    assert_eq!(
        StopReason::BlockerEscalationRequired.as_str(),
        "blocker_escalation_required"
    );
    assert_eq!(
        StopReason::BlockerCycleDetected.as_str(),
        "blocker_cycle_detected"
    );

    let defaults = ContinuationRunConfig::default();
    assert_eq!(defaults.max_continuations, 50);
    assert_eq!(defaults.max_blocker_depth, 3);
    assert_eq!(defaults.max_blocker_attempts_per_ticket, 3);
    assert_eq!(defaults.max_consecutive_blocker_generations, 2);
    assert_eq!(defaults.stagnation_limit, 3);
    assert_eq!(defaults.consecutive_failure_limit, 3);
}
