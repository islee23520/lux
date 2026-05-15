use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use lux::{
    lux_roadmap::{RoadmapPhaseStatus, RoadmapReality},
    lux_run::{
        begin_milestone_push_approval, execute_milestone_push_with_runner, MilestonePushApproval,
        TransactionJournal, TransactionOperation, TransactionStatus, MILESTONE_PUSH_TRANSITION,
    },
    lux_run_recover::recover_pending_transactions,
    lux_run_state::{ApprovalGateType, RunState, RunStatus},
    lux_task_dag::{TaskDAG, TaskNode, TaskStatus},
    lux_ticket::{
        is_execution_grade, should_dispatch, validate_execution_grade, BlockerPolicy,
        DispatchPolicy, FileTicketStore, Ticket, TicketPriority, TicketStatus, TicketStore,
    },
    lux_verification::{create_blocker_tickets, CheckCategory, CheckResult, VerificationResult},
};

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-lifecycle-{name}-{}", uuid::Uuid::new_v4()));
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

#[test]
fn full_lifecycle_awaits_approval_then_pushes_and_completes() {
    let temp = TestTempDir::new("full");
    let evidence = write_t3_evidence(temp.path(), "full");
    let mut roadmap = RoadmapReality::default();
    lux::lux_roadmap::save(temp.path(), &roadmap).expect("roadmap should save");
    let mut state = run_state(temp.path(), "run-full");

    state
        .transition_to(RunStatus::Planning, "spec/roadmap selects milestone")
        .expect("planning transition");
    state.milestone_id = Some("M1".to_string());
    assert_eq!(state.status, RunStatus::Planning.to_string());

    let mut dag = TaskDAG::default();
    dag.add_node(TaskNode {
        id: "ticket-1".to_string(),
        spec_clause_id: "spec-1".to_string(),
        title: "Implement milestone ticket".to_string(),
        status: TaskStatus::Pending,
        dependencies: Vec::new(),
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });
    dag.nodes.get_mut("ticket-1").expect("ticket").status = TaskStatus::AwaitingEvidence;
    dag.mark_done("ticket-1", Some(evidence.display().to_string()));
    assert_eq!(dag.ready_nodes().len(), 0);

    begin_milestone_push_approval(&mut state, &evidence, Some("preview-sha".to_string()))
        .expect("approval should begin after evidence exists");
    assert_eq!(state.status, RunStatus::AwaitingApproval.to_string());
    assert_eq!(state.approval.gate.as_deref(), Some("ApproveDiff"));
    assert_eq!(
        state.approval.pending_transition.as_deref(),
        Some(MILESTONE_PUSH_TRANSITION)
    );

    let approval = MilestonePushApproval {
        project_path: temp.path().to_path_buf(),
        milestone_id: Some("M1".to_string()),
        evidence_path: evidence.strip_prefix(temp.path()).unwrap().to_path_buf(),
        git_sha: "abc123".to_string(),
    };
    execute_milestone_push_with_runner(&mut state, &mut roadmap, &approval, |_| Ok(()))
        .expect("approved milestone push should complete");

    assert_eq!(state.status, RunStatus::Completed.to_string());
    assert_eq!(state.stop_reason.as_deref(), Some("milestone_complete"));
    assert_eq!(roadmap.phases[0].status, RoadmapPhaseStatus::Pushed);
    assert_eq!(roadmap.phases[0].push_git_sha.as_deref(), Some("abc123"));

    let loaded = RunState::load(temp.path()).expect("run-state should be committed");
    assert_eq!(loaded.status, RunStatus::Completed.to_string());
    let loaded_roadmap = lux::lux_roadmap::load(temp.path()).expect("roadmap should load");
    assert_eq!(loaded_roadmap.phases[0].status, RoadmapPhaseStatus::Pushed);
}

#[test]
fn verification_failure_creates_blocker_and_blocks_push_without_evidence() {
    let temp = TestTempDir::new("verification-failure");
    lux::lux_roadmap::save(temp.path(), &RoadmapReality::default()).expect("roadmap should save");
    let mut state = run_state(temp.path(), "run-failure");
    state.current_ticket_id = Some("ticket-active".to_string());
    state.save(temp.path()).expect("state should save");
    seed_ticket(temp.path(), "ticket-active");

    let result = VerificationResult {
        passed: false,
        timestamp: Utc::now().to_rfc3339(),
        checks: vec![CheckResult {
            name: "T3 milestone: Unity Scene Smoke".to_string(),
            category: CheckCategory::UnityCompilable,
            passed: false,
            score: 0.0,
            message: "scene smoke failed".to_string(),
            details: None,
        }],
        overall_score: 0.0,
        blocker_ticket_ids: Vec::new(),
    };
    let blockers = create_blocker_tickets(&result, temp.path()).expect("blocker should be created");
    assert_eq!(blockers.len(), 1);
    let active = FileTicketStore::new(temp.path())
        .get("ticket-active")
        .expect("ticket read")
        .expect("ticket exists");
    assert_eq!(active.status, TicketStatus::Blocked);
    assert_eq!(active.blockers, blockers);

    state
        .transition_to(RunStatus::AwaitingApproval, "test")
        .unwrap();
    state.approval.gate = Some(ApprovalGateType::ApproveDiff.to_string());
    state.approval.pending_transition = Some(MILESTONE_PUSH_TRANSITION.to_string());
    let mut roadmap = RoadmapReality::default();
    let approval = MilestonePushApproval {
        project_path: temp.path().to_path_buf(),
        milestone_id: Some("M1".to_string()),
        evidence_path: PathBuf::from(".lux/verification/t3/missing/evidence.json"),
        git_sha: "abc123".to_string(),
    };
    let error = execute_milestone_push_with_runner(&mut state, &mut roadmap, &approval, |_| Ok(()))
        .expect_err("missing T3 evidence must block push");
    assert!(error.to_string().contains("requires T3 evidence"));
    assert_ne!(roadmap.phases[0].status, RoadmapPhaseStatus::Pushed);
    assert_ne!(state.status, RunStatus::Completed.to_string());
}

#[test]
fn transaction_recovery_commits_planned_lifecycle_writes_on_start() {
    let temp = TestTempDir::new("recovery");
    let evidence = write_t3_evidence(temp.path(), "recovery");
    let mut state = run_state(temp.path(), "run-recovery");
    begin_milestone_push_approval(&mut state, &evidence, Some("preview".to_string()))
        .expect("approval should begin");
    state.save(temp.path()).expect("awaiting state should save");

    let mut completed = state.clone();
    completed
        .transition_to(RunStatus::Completed, "milestone_complete")
        .unwrap();
    completed.stop_reason = Some("milestone_complete".to_string());
    completed.approval = Default::default();

    let mut roadmap = RoadmapReality::default();
    roadmap.phases[0].status = RoadmapPhaseStatus::Pushed;
    roadmap.phases[0].pushed_at = Some(Utc::now().to_rfc3339());
    roadmap.phases[0].push_git_sha = Some("recovered-sha".to_string());
    roadmap.phases[0].push_evidence_path = Some(evidence.display().to_string());
    lux::lux_roadmap::save(temp.path(), &RoadmapReality::default()).expect("roadmap should save");

    let journal = TransactionJournal::planned(
        "run-recovery",
        temp.path(),
        vec![
            TransactionOperation::WriteFile {
                path: RunState::path(temp.path()),
                content: serde_json::to_string_pretty(&completed).unwrap(),
                before_content: None,
            },
            TransactionOperation::WriteFile {
                path: lux::lux_roadmap::roadmap_file_path(temp.path()),
                content: serde_json::to_string_pretty(&roadmap).unwrap(),
                before_content: None,
            },
        ],
    )
    .expect("planned transaction should write journal");

    let recovered = recover_pending_transactions(temp.path()).expect("recovery should commit");
    assert_eq!(recovered.len(), 1);
    let journal_path = temp
        .path()
        .join(".lux/runs/run-recovery/transactions")
        .join(format!("{}.json", journal.id));
    let committed = TransactionJournal::load(&journal_path).expect("journal should load");
    assert_eq!(committed.status, TransactionStatus::Committed);
    assert_eq!(
        RunState::load(temp.path()).unwrap().status,
        RunStatus::Completed.to_string()
    );
    assert_eq!(
        lux::lux_roadmap::load(temp.path()).unwrap().phases[0].status,
        RoadmapPhaseStatus::Pushed
    );
}

fn run_state(project_path: &Path, run_id: &str) -> RunState {
    let mut state = RunState::idle(project_path).expect("idle state");
    state.run_id = run_id.to_string();
    state
}

fn write_t3_evidence(project_path: &Path, name: &str) -> PathBuf {
    let path = project_path
        .join(".lux")
        .join("verification")
        .join("t3")
        .join(name)
        .join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).expect("evidence dir should be created");
    let evidence_json = serde_json::json!({
        "base": {
            "passed": true,
            "timestamp": "2026-01-01T00:00:00Z",
            "checks": [],
            "overall_score": 1.0,
            "blocker_ticket_ids": []
        },
        "tier_results": [],
        "overall_tier": "T3Gate",
        "domain_tiers": {}
    });
    fs::write(&path, evidence_json.to_string()).expect("evidence should be written");
    path
}

fn seed_ticket(project_path: &Path, id: &str) {
    let now = Utc::now().to_rfc3339();
    FileTicketStore::new(project_path)
        .create(Ticket {
            id: id.to_string(),
            title: "Active ticket".to_string(),
            description: "Ticket under verification".to_string(),
            status: TicketStatus::InProgress,
            priority: TicketPriority::High,
            assignee: None,
            blockers: Vec::new(),
            tags: Vec::new(),
            spec_ref: None,
            created_at: now.clone(),
            updated_at: now,
            execution_objective: None,
            allowed_executor: None,
            dispatch_policy: None,
            verification_policy: None,
            command_allowlist: None,
            evidence_refs: None,
            blocker_policy: None,
            non_goals: None,
        })
        .expect("ticket should be seeded");
}

fn execution_grade_ticket(id: &str) -> Ticket {
    let now = Utc::now().to_rfc3339();
    Ticket {
        id: id.to_string(),
        title: "Execution grade ticket".to_string(),
        description: "A fully specified execution-grade ticket".to_string(),
        status: TicketStatus::ToDo,
        priority: TicketPriority::High,
        assignee: Some("opencode".to_string()),
        blockers: Vec::new(),
        tags: Vec::new(),
        spec_ref: None,
        created_at: now.clone(),
        updated_at: now,
        execution_objective: Some("Implement feature X".to_string()),
        allowed_executor: Some("opencode".to_string()),
        dispatch_policy: Some(DispatchPolicy::DispatchRequested),
        verification_policy: Some("unity_t3".to_string()),
        command_allowlist: None,
        evidence_refs: None,
        blocker_policy: Some(BlockerPolicy {
            max_depth: Some(3),
            max_attempts: Some(5),
        }),
        non_goals: Some(vec!["out of scope item".to_string()]),
    }
}

#[test]
fn ticket_execution_grade_validates() {
    let ticket = execution_grade_ticket("exec-001");
    assert!(is_execution_grade(&ticket));
    assert!(validate_execution_grade(&ticket).is_ok());
    assert!(should_dispatch(&ticket));
}

#[test]
fn ticket_plain_inprogress_no_dispatch() {
    let now = Utc::now().to_rfc3339();
    let ticket = Ticket {
        id: "plain-001".to_string(),
        title: "Plain ticket".to_string(),
        description: "No dispatch policy set".to_string(),
        status: TicketStatus::InProgress,
        priority: TicketPriority::Medium,
        assignee: None,
        blockers: Vec::new(),
        tags: Vec::new(),
        spec_ref: None,
        created_at: now.clone(),
        updated_at: now,
        execution_objective: None,
        allowed_executor: None,
        dispatch_policy: None,
        verification_policy: None,
        command_allowlist: None,
        evidence_refs: None,
        blocker_policy: None,
        non_goals: None,
    };
    assert!(!should_dispatch(&ticket));
    assert!(!is_execution_grade(&ticket));
}

#[test]
fn ticket_missing_objective_rejected() {
    let mut ticket = execution_grade_ticket("exec-002");
    ticket.execution_objective = None;
    assert!(validate_execution_grade(&ticket).is_err());
    assert!(!is_execution_grade(&ticket));
}

#[test]
fn ticket_command_allowlist_required_for_command_policy() {
    let mut ticket = execution_grade_ticket("exec-003");
    ticket.verification_policy = Some("command_suite".to_string());
    ticket.command_allowlist = Some(vec![]);
    let result = validate_execution_grade(&ticket);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("command_allowlist must be non-empty"));

    ticket.command_allowlist = None;
    assert!(validate_execution_grade(&ticket).is_err());

    ticket.command_allowlist = Some(vec!["cargo test".to_string()]);
    assert!(validate_execution_grade(&ticket).is_ok());
}
