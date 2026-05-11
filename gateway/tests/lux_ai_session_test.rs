#[path = "../src/lux_ai_session.rs"]
mod lux_ai_session;
#[path = "../src/lux_spec.rs"]
mod lux_spec;

use std::fs;
use std::path::{Path, PathBuf};

use lux_ai_session::{
    advance_phase, apply_session_to_spec, create_session, evaluate_phase_completion, load_session,
    process_message, save_session, SessionPhase, SessionStatus, TurnRole,
};
use lux_spec::{lux_init, PillarStatus, SpecProject};

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-ai-session-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
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

fn substantive_answer(label: &str) -> String {
    format!(
        "The player should feel a clear {label} moment with readable feedback, meaningful choice, and a testable emotional payoff."
    )
}

fn complete_current_phase(session: &mut lux_ai_session::AiSession) {
    for index in 0..3 {
        let response =
            process_message(session, &substantive_answer(&format!("phase-{index}"))).unwrap();
        if response.phase_complete {
            break;
        }
    }
}

#[test]
fn test_create_session() {
    let project = TestProject::new("create");
    lux_init(project.path()).unwrap();

    let session = create_session(project.path()).unwrap();

    assert!(!session.session_id.is_empty());
    assert_eq!(session.project_path, project.path());
    assert_eq!(session.phase, SessionPhase::Phase1Experience);
    assert_eq!(session.turn_count, 0);
    assert_eq!(session.max_turns, 50);
    assert!(session.history.is_empty());
    assert_eq!(session.status, SessionStatus::Active);
}

#[test]
fn test_session_phase_progression() {
    let project = TestProject::new("progression");
    let mut session = create_session(project.path()).unwrap();

    complete_current_phase(&mut session);
    assert_eq!(session.phase, SessionPhase::Phase2Tetrad);

    complete_current_phase(&mut session);
    assert_eq!(session.phase, SessionPhase::Phase3CoreLoop);

    complete_current_phase(&mut session);
    assert_eq!(session.phase, SessionPhase::Phase4Motivation);

    complete_current_phase(&mut session);
    assert_eq!(session.phase, SessionPhase::Phase5Assessment);

    complete_current_phase(&mut session);
    assert_eq!(session.phase, SessionPhase::Completed);
    assert_eq!(session.status, SessionStatus::Completed);
}

#[test]
fn test_process_message_adds_turn() {
    let project = TestProject::new("adds-turn");
    let mut session = create_session(project.path()).unwrap();

    let response = process_message(
        &mut session,
        "Players feel clever when shadows reveal hidden paths.",
    )
    .unwrap();

    assert_eq!(session.turn_count, 2);
    assert_eq!(session.history.len(), 2);
    assert_eq!(session.history[0].role, TurnRole::User);
    assert_eq!(session.history[1].role, TurnRole::Ai);
    assert!(response.message.contains("Question"));
}

#[test]
fn test_max_turns_limit() {
    let project = TestProject::new("max-turns");
    let mut session = create_session(project.path()).unwrap();
    session.max_turns = 1;

    let response =
        process_message(&mut session, "This answer consumes the only allowed turn.").unwrap();

    assert_eq!(session.turn_count, 1);
    assert_eq!(session.phase, SessionPhase::Completed);
    assert_eq!(session.status, SessionStatus::Completed);
    assert!(response.message.contains("turn limit"));
}

#[test]
fn test_evaluate_phase_completion() {
    let project = TestProject::new("completion");
    let mut session = create_session(project.path()).unwrap();

    process_message(&mut session, &substantive_answer("first player experience")).unwrap();
    let incomplete = evaluate_phase_completion(&session).unwrap();
    assert!(!incomplete.complete);

    process_message(&mut session, &substantive_answer("second player fantasy")).unwrap();
    process_message(&mut session, &substantive_answer("third emotional moment")).unwrap();

    assert_eq!(session.phase, SessionPhase::Phase2Tetrad);
}

#[test]
fn test_save_load_session() {
    let project = TestProject::new("save-load");
    let mut session = create_session(project.path()).unwrap();
    process_message(
        &mut session,
        "The player should feel tense curiosity while tracing signal ghosts.",
    )
    .unwrap();

    save_session(&session).unwrap();
    let loaded = load_session(project.path(), &session.session_id).unwrap();

    assert_eq!(loaded.session_id, session.session_id);
    assert_eq!(loaded.project_path, session.project_path);
    assert_eq!(loaded.phase, session.phase);
    assert_eq!(loaded.turn_count, session.turn_count);
    assert_eq!(loaded.max_turns, session.max_turns);
    assert_eq!(loaded.history, session.history);
    assert_eq!(loaded.status, session.status);
}

#[test]
fn test_apply_session_to_spec() {
    let project = TestProject::new("apply-spec");
    let mut session = create_session(project.path()).unwrap();
    process_message(
        &mut session,
        "The player should feel clever and vulnerable while using light to expose hidden routes.",
    )
    .unwrap();
    process_message(
        &mut session,
        "Each moment should produce readable emotion through player choice and feedback.",
    )
    .unwrap();
    process_message(
        &mut session,
        "The experience fantasy is being a careful explorer whose action changes danger.",
    )
    .unwrap();

    let mut spec = SpecProject::default();
    apply_session_to_spec(&session, &mut spec).unwrap();

    assert_ne!(
        spec.schell_evaluation.phase1_experience.status,
        PillarStatus::Missing
    );
    assert!(spec
        .schell_evaluation
        .phase1_experience
        .summary
        .unwrap()
        .contains("clever"));
    assert!(spec.domains.custom.contains_key("experience"));
    assert!(spec.overall_ambiguity < 1.0);
}

#[test]
fn test_socratic_dialectic_flow() {
    let project = TestProject::new("dialectic");
    let mut session = create_session(project.path()).unwrap();

    let first = process_message(
        &mut session,
        "Players feel pressure as enemies react to every noisy movement.",
    )
    .unwrap();
    assert!(first.message.contains("Question"));

    let second = process_message(
        &mut session,
        "The player must choose between fast progress and safe silent routes.",
    )
    .unwrap();
    assert!(second.message.contains("Rebuttal"));

    let third = process_message(
        &mut session,
        "The emotional payoff is a visible escape moment after risky planning succeeds.",
    )
    .unwrap();
    assert!(third.message.contains("Synthesis"));
}

#[test]
fn test_manual_advance_requires_completion() {
    let project = TestProject::new("manual-advance");
    let mut session = create_session(project.path()).unwrap();

    let same = advance_phase(&mut session).unwrap();
    assert_eq!(same, SessionPhase::Phase1Experience);
}
