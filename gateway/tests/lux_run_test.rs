use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use lux::{
    lux_lock::{acquire_lux_lock, DEFAULT_STALE_THRESHOLD_SECS},
    lux_run::{execute_task, RunConfig, RunLifecycle},
    lux_run_recover::ExecutionSession,
    lux_run_state::{RunState, RunStatus},
    lux_task_dag::{TaskDAG, TaskNode, TaskStatus},
    lux_team_profile::{TeamProfile, TeamSizePreset},
};

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("lux-run-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(path.join(".lux")).expect("temp .lux dir should be created");
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

fn make_lifecycle_with_task(project_root: &Path, task_id: &str) -> RunLifecycle {
    let mut state = RunState::idle(project_root).expect("idle state");
    state.run_id = "test-run-001".to_string();
    state
        .transition_to(RunStatus::Planning, "test")
        .expect("transition to planning");
    state.save(project_root).expect("save state");

    let runs_dir = project_root
        .join(".lux")
        .join("runs")
        .join("test-run-001")
        .join("dispatch");
    fs::create_dir_all(&runs_dir).expect("create dispatch dir");

    let mut dag = TaskDAG::default();
    dag.add_node(TaskNode {
        id: task_id.to_string(),
        spec_clause_id: "test.clause".to_string(),
        title: "Test task".to_string(),
        status: TaskStatus::Pending,
        dependencies: vec![],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });

    let config = RunConfig {
        project_path: project_root.to_path_buf(),
        team_preset: TeamSizePreset::Small,
        dry_run: false,
        goal: None,
    };

    let lux_dir = project_root.join(".lux");
    let lock_guard = acquire_lux_lock(
        &lux_dir,
        "test-agent",
        "test",
        DEFAULT_STALE_THRESHOLD_SECS,
        true,
    )
    .expect("acquire lock for test");

    RunLifecycle::from_recovered_parts(
        config,
        dag,
        TeamProfile::default(),
        state,
        Default::default(),
        Some(lock_guard),
    )
}

#[test]
fn execute_task_sets_awaiting_evidence_not_done() {
    let temp = TestTempDir::new("dispatch-not-done");
    let task_id = "task-alpha";
    let mut lifecycle = make_lifecycle_with_task(temp.path(), task_id);

    execute_task(&mut lifecycle, task_id).expect("execute_task should succeed");

    let node = lifecycle.dag.nodes.get(task_id).expect("node should exist");

    assert_eq!(
        node.status,
        TaskStatus::AwaitingEvidence,
        "dispatch must set AwaitingEvidence, not Done"
    );
}

#[test]
fn execute_task_dispatch_file_written_atomically() {
    let temp = TestTempDir::new("dispatch-atomic");
    let task_id = "task-gamma";
    let mut lifecycle = make_lifecycle_with_task(temp.path(), task_id);

    execute_task(&mut lifecycle, task_id).expect("execute_task should succeed");

    let dispatch_path = temp
        .path()
        .join(".lux")
        .join("runs")
        .join("test-run-001")
        .join("dispatch")
        .join(format!("{task_id}.json"));

    assert!(
        dispatch_path.exists(),
        "dispatch file should be written at expected path"
    );

    let content = fs::read_to_string(&dispatch_path).expect("dispatch file should be readable");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("dispatch file should be valid JSON");
    assert!(
        parsed.is_object(),
        "dispatch file should contain a JSON object"
    );
}

#[test]
fn awaiting_evidence_node_does_not_unblock_dependents() {
    let mut dag = TaskDAG::default();
    dag.add_node(TaskNode {
        id: "parent".to_string(),
        spec_clause_id: "p".to_string(),
        title: "Parent".to_string(),
        status: TaskStatus::AwaitingEvidence,
        dependencies: vec![],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });
    dag.add_node(TaskNode {
        id: "child".to_string(),
        spec_clause_id: "c".to_string(),
        title: "Child".to_string(),
        status: TaskStatus::Pending,
        dependencies: vec!["parent".to_string()],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });
    dag.add_dependency("child", "parent");

    let ready = dag.ready_nodes();
    assert!(
        ready.iter().all(|n| n.id != "child"),
        "child must not be ready while parent is AwaitingEvidence (not Done)"
    );
}

#[test]
fn task_dag_rejects_explicit_cycle_without_fallback_ordering() {
    let mut dag = TaskDAG::default();
    dag.add_node(TaskNode {
        id: "a".to_string(),
        spec_clause_id: "a".to_string(),
        title: "A".to_string(),
        status: TaskStatus::Pending,
        dependencies: vec![],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });
    dag.add_node(TaskNode {
        id: "b".to_string(),
        spec_clause_id: "b".to_string(),
        title: "B".to_string(),
        status: TaskStatus::Pending,
        dependencies: vec![],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });

    dag.try_add_dependency("a", "b")
        .expect("first dependency should be accepted");
    let error = dag
        .try_add_dependency("b", "a")
        .expect_err("reverse dependency should be rejected");
    assert!(error.contains("dependency cycle detected"));

    dag.edges.push(("a".to_string(), "b".to_string()));
    dag.nodes
        .get_mut("b")
        .expect("node b should exist")
        .dependencies
        .push("a".to_string());
    let error = dag
        .topological_ids_checked()
        .expect_err("cycle must not fall back to arbitrary ordering");
    assert!(error.contains("dependency cycle detected"));
}

#[test]
fn execute_task_writes_session_and_heartbeat_updates_timestamp() {
    let temp = TestTempDir::new("session-heartbeat");
    let task_id = "task-heartbeat";
    let mut lifecycle = make_lifecycle_with_task(temp.path(), task_id);

    execute_task(&mut lifecycle, task_id).expect("execute_task should succeed");

    let run_id = &lifecycle.state.run_id;
    let mut session = ExecutionSession::load(temp.path(), run_id)
        .expect("load should succeed")
        .expect("session should be present after execute_task");

    assert_eq!(session.ticket_id, task_id);
    let before = session.last_heartbeat_at.clone();

    std::thread::sleep(std::time::Duration::from_millis(10));
    session
        .heartbeat(temp.path())
        .expect("heartbeat should succeed");

    let reloaded = ExecutionSession::load(temp.path(), run_id)
        .expect("reload should succeed")
        .expect("session should still be present");

    assert!(
        reloaded.last_heartbeat_at > before,
        "heartbeat must advance last_heartbeat_at"
    );
}

#[test]
fn execute_task_concurrency_guard_rejects_duplicate_dispatch() {
    let temp = TestTempDir::new("concurrency-guard");
    let task_id = "task-concurrent";
    let mut lifecycle = make_lifecycle_with_task(temp.path(), task_id);

    execute_task(&mut lifecycle, task_id).expect("first dispatch should succeed");

    let mut dag2 = TaskDAG::default();
    dag2.add_node(lux::lux_task_dag::TaskNode {
        id: task_id.to_string(),
        spec_clause_id: "test.clause".to_string(),
        title: "Test task".to_string(),
        status: TaskStatus::Pending,
        dependencies: vec![],
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    });
    let config2 = RunConfig {
        project_path: temp.path().to_path_buf(),
        team_preset: TeamSizePreset::Small,
        dry_run: false,
        goal: None,
    };
    let lux_dir = temp.path().join(".lux");
    let lock2 = lux::lux_lock::acquire_lux_lock(
        &lux_dir,
        "test-agent-2",
        "test",
        DEFAULT_STALE_THRESHOLD_SECS,
        true,
    )
    .expect("acquire second lock");
    let mut state2 = RunState::idle(temp.path()).expect("idle state");
    state2.run_id = lifecycle.state.run_id.clone();
    state2
        .transition_to(RunStatus::Planning, "test")
        .expect("transition");
    let mut lifecycle2 = RunLifecycle::from_recovered_parts(
        config2,
        dag2,
        lux::lux_team_profile::TeamProfile::default(),
        state2,
        Default::default(),
        Some(lock2),
    );

    let err = execute_task(&mut lifecycle2, task_id)
        .expect_err("duplicate dispatch must be rejected");
    assert!(
        err.to_string().contains("execution already in progress"),
        "error must mention concurrent execution: {err}"
    );
}
