use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use lux::lux_events::{EventRouter, LuxEvent};
use lux::lux_loop::{ApprovalGate, LoopOrchestrator, LoopState};
use lux::lux_spec::{lux_init, lux_load, lux_save};
use serde_json::json;

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("lux-loop-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        lux_init(&path).unwrap();
        let mut spec = lux_load(&path).unwrap();
        spec.overall_ambiguity = 1.0;
        lux_save(&path, &spec).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn router_with_events(events: Arc<Mutex<Vec<(String, String)>>>) -> EventRouter {
    let mut router = EventRouter::new();
    router.register(
        "loop:state_change",
        Box::new(move |event| {
            if let LuxEvent::LoopStateChange {
                previous_state,
                current_state,
                ..
            } = event
            {
                events
                    .lock()
                    .unwrap()
                    .push((previous_state.clone(), current_state.clone()));
            }
        }),
    );
    router
}

#[test]
fn lux_loop_state_machine_requires_approval_between_steps() {
    let project = TestProject::new("state-machine");
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut orchestrator = LoopOrchestrator::with_max_iterations(
        project.path(),
        10,
        router_with_events(events.clone()),
    );

    let snapshot = orchestrator.start().unwrap();
    assert_eq!(snapshot.state, LoopState::Analyzing);
    assert_eq!(snapshot.pending_state, Some(LoopState::SpecRefining));
    assert_eq!(snapshot.approval_gate, Some(ApprovalGate::RefineSpec));
    assert!(snapshot.requires_user_approval);
    assert!(snapshot.last_verification.is_some());
    assert!(snapshot.last_ambiguity.is_some());

    let snapshot = orchestrator.approve_next().unwrap();
    assert_eq!(snapshot.state, LoopState::SpecRefining);
    assert_eq!(snapshot.pending_state, Some(LoopState::Building));
    assert!(snapshot.active_ai_session.is_some());

    let snapshot = orchestrator.approve_next().unwrap();
    assert_eq!(snapshot.state, LoopState::Building);
    assert_eq!(snapshot.pending_state, Some(LoopState::AwaitingPlay));
    assert!(snapshot.active_build_id.is_some());

    let snapshot = orchestrator.approve_next().unwrap();
    assert_eq!(snapshot.state, LoopState::AwaitingPlay);
    assert_eq!(snapshot.approval_gate, Some(ApprovalGate::StartPlay));
    assert_eq!(snapshot.pending_state, Some(LoopState::CollectingFeedback));

    let snapshot = orchestrator.approve_next().unwrap();
    assert_eq!(snapshot.state, LoopState::CollectingFeedback);
    assert!(!snapshot.requires_user_approval);

    let snapshot = orchestrator.record_play_started().unwrap();
    assert_eq!(snapshot.state, LoopState::CollectingFeedback);
    assert!(!snapshot.requires_user_approval);

    let snapshot = orchestrator
        .record_feedback(&json!({"fun": "needs polish"}))
        .unwrap();
    assert_eq!(snapshot.iteration, 1);
    assert_eq!(snapshot.feedback_count, 0);
    assert_eq!(
        snapshot.approval_gate,
        Some(ApprovalGate::CompleteIteration)
    );
    assert_eq!(snapshot.pending_state, Some(LoopState::Idle));

    let snapshot = orchestrator.approve_next().unwrap();
    assert_eq!(snapshot.state, LoopState::Idle);
    assert_eq!(snapshot.iteration, 1);
    assert_eq!(snapshot.feedback_count, 0);

    let recorded = events.lock().unwrap();
    assert!(recorded.contains(&("Idle".to_string(), "Analyzing".to_string())));
    assert!(recorded.contains(&("CollectingFeedback".to_string(), "Idle".to_string())));
}

#[test]
fn lux_loop_can_complete_two_sequential_iterations() {
    let project = TestProject::new("two-iterations");
    let mut orchestrator =
        LoopOrchestrator::with_max_iterations(project.path(), 2, EventRouter::new());

    let snapshot = orchestrator.start().unwrap();
    assert_eq!(snapshot.state, LoopState::Analyzing);

    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    let awaiting = orchestrator.approve_next().unwrap();
    assert_eq!(awaiting.state, LoopState::AwaitingPlay);
    assert_eq!(awaiting.pending_state, Some(LoopState::CollectingFeedback));

    let collecting = orchestrator.approve_next().unwrap();
    assert_eq!(collecting.state, LoopState::CollectingFeedback);
    let collecting = orchestrator.record_play_started().unwrap();
    assert_eq!(collecting.state, LoopState::CollectingFeedback);

    let completed = orchestrator
        .record_feedback(&json!({"iteration": 1, "notes": "tighten controls"}))
        .unwrap();
    assert_eq!(completed.iteration, 1);
    assert_eq!(
        completed.approval_gate,
        Some(ApprovalGate::CompleteIteration)
    );
    assert_eq!(completed.pending_state, Some(LoopState::Idle));

    let idle = orchestrator.approve_next().unwrap();
    assert_eq!(idle.state, LoopState::Idle);
    assert_eq!(idle.iteration, 1);

    orchestrator.start().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.record_play_started().unwrap();
    let done = orchestrator
        .record_feedback(&json!({"iteration": 2, "notes": "accepted"}))
        .unwrap();
    assert_eq!(done.iteration, 2);
    assert_eq!(done.state, LoopState::Idle);
    assert_eq!(done.pending_state, None);
    assert!(!done.requires_user_approval);
}

#[test]
fn lux_loop_pause_and_resume_restore_state() {
    let project = TestProject::new("pause-resume");
    let mut orchestrator = LoopOrchestrator::new(project.path(), EventRouter::new());
    orchestrator.start().unwrap();

    let paused = orchestrator.pause().unwrap();
    assert!(matches!(paused.state, LoopState::Paused(inner) if *inner == LoopState::Analyzing));

    let resumed = orchestrator.resume().unwrap();
    assert_eq!(resumed.state, LoopState::Analyzing);
    assert_eq!(resumed.pending_state, Some(LoopState::SpecRefining));
}

#[test]
fn lux_loop_max_iterations_guard_blocks_restart() {
    let project = TestProject::new("max-iterations");
    let mut orchestrator =
        LoopOrchestrator::with_max_iterations(project.path(), 1, EventRouter::new());

    orchestrator.start().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.approve_next().unwrap();
    orchestrator.record_play_started().unwrap();
    orchestrator
        .record_feedback(&json!({"issue": "short loop"}))
        .unwrap();
    let idle = orchestrator.snapshot();
    assert_eq!(idle.iteration, 1);
    assert_eq!(idle.state, LoopState::Idle);

    let error = orchestrator
        .start()
        .expect_err("max iteration guard should stop restart");
    assert!(error.to_string().contains("max iterations"));
}
