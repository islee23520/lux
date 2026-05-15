use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::{Duration, Utc};
use lux::lux_ai_session::{
    apply_session_to_spec, create_session, process_message, save_session, SessionPhase,
    SessionStatus,
};
use lux::lux_ambiguity::calculate_ambiguity;
use lux::lux_build::{
    append_build_log, get_build_artifact_path, get_build_status, mark_build_running,
    mark_build_succeeded, start_build, BuildManager, BuildStatus, BuildTarget,
};
use lux::lux_event_log::{
    EventFilter, EventLogStore, FileEventLogStore, PlayEvent, PlayEventType, SessionMetadata,
};
use lux::lux_run_state::RunState;
use lux::lux_spec::{
    lux_init, lux_load, lux_save, AssessmentResult, DomainSpec, PhaseResult, PillarRating,
    PillarStatus, SchellEvaluation, SpecDomains, SpecProject, SpecStatus, TetradResult,
};
use lux::lux_ticket::{
    FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus, TicketStore,
};
use lux::lux_verification::{verify_all, CheckCategory, VerificationMode};
use serde_json::json;
use tempfile::TempDir;

struct E2eProject {
    tmp: TempDir,
}

impl E2eProject {
    fn new() -> Self {
        Self {
            tmp: tempfile::tempdir().expect("temp project should be created"),
        }
    }

    fn path(&self) -> &Path {
        self.tmp.path()
    }

    fn logs_path(&self) -> PathBuf {
        self.path().join(".lux/logs")
    }
}

#[test]
fn lux_e2e_phase_1_lux_init_creates_project_state() {
    let project = E2eProject::new();

    let lux_path = initialize_lux_project(project.path());

    assert_eq!(lux_path, project.path().join(".lux"));
    assert!(lux_path.is_dir());
    assert!(lux_path.join("spec.json").is_file());
    assert!(lux_path.join("domains/design.md").is_file());
    assert!(lux_path.join("tickets").is_dir());
    assert!(lux_path.join("logs").is_dir());
    assert!(lux_path.join("sessions").is_dir());
    assert!(lux_path.join("builds").is_dir());
}

#[test]
fn lux_e2e_phase_2_ai_session_generates_spec() {
    let project = initialized_project();

    let spec = run_mock_ai_spec_generation(project.path());
    let report = calculate_ambiguity(&spec);

    assert_eq!(spec.status, SpecStatus::Active);
    assert!(spec
        .domains
        .design
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .architecture
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .art_style
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .audio
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .narrative
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .levels
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec
        .domains
        .ui_ux
        .as_ref()
        .is_some_and(|domain| domain.defined));
    assert!(spec.overall_ambiguity < 0.5);
    assert_eq!(report.domain_scores.len(), 7);
    assert!(report.completion_ratio > 0.9);
}

#[test]
fn lux_e2e_phase_3_kanban_tickets_created_from_spec() {
    let project = initialized_project();
    let spec = run_mock_ai_spec_generation(project.path());

    let tickets = create_kanban_tickets_from_spec(project.path(), &spec);
    let listed = FileTicketStore::new(project.path())
        .list(TicketFilter::default())
        .expect("tickets should list");

    assert_eq!(tickets.len(), 7);
    assert_eq!(listed.len(), 7);
    assert!(listed
        .iter()
        .all(|ticket| ticket.status == TicketStatus::Backlog));
    assert!(listed
        .iter()
        .all(|ticket| ticket.tags.contains(&"ouroboros".to_string())));
    assert!(
        project
            .path()
            .join(".lux/tickets")
            .read_dir()
            .unwrap()
            .count()
            >= 7
    );
}

#[test]
fn lux_e2e_phase_4_webgl_build_is_tracked_and_servable() {
    let project = initialized_project();

    let (_manager, build_id, artifact_path) = run_mock_webgl_build(project.path());

    assert!(artifact_path.is_file());
    assert!(fs::read_to_string(&artifact_path)
        .expect("artifact should be readable")
        .contains("Lux E2E playable"));
    assert!(artifact_path.ends_with(Path::new(&format!(".lux/builds/{build_id}/index.html"))));
}

#[test]
fn lux_e2e_phase_5_play_events_are_received_and_stored() {
    let project = initialized_project();

    let events = record_play_events(project.path());

    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .any(|event| event.event_type == PlayEventType::LevelStart));
    assert!(events
        .iter()
        .any(|event| event.event_type == PlayEventType::Decision));
    assert!(events
        .iter()
        .any(|event| event.event_type == PlayEventType::LevelComplete));
    assert!(project.logs_path().join("play-session-e2e.jsonl").is_file());
}

#[test]
fn lux_e2e_phase_6_feedback_updates_spec() {
    let project = initialized_project();
    run_mock_ai_spec_generation(project.path());

    let updated = integrate_feedback_into_spec(project.path());

    let feedback_notes = updated
        .domains
        .design
        .as_ref()
        .and_then(|domain| domain.fields.get("feedback_notes"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(feedback_notes.contains("dash readability"));
    assert!(updated
        .schell_evaluation
        .phase5_assessment
        .recommendations
        .iter()
        .any(|item| item.contains("telegraph")));
    assert!(project
        .logs_path()
        .join("play-session-e2e.feedback.json")
        .is_file());
}

#[test]
fn lux_e2e_phase_7_verification_creates_blocker_tickets() {
    let project = initialized_project();
    let spec = run_mock_ai_spec_generation(project.path());
    create_kanban_tickets_from_spec(project.path(), &spec);
    run_mock_webgl_build(project.path());
    record_play_events(project.path());
    integrate_feedback_into_spec(project.path());

    let result = run_verification_with_blocker(project.path());

    assert!(!result.passed);
    assert!(result
        .checks
        .iter()
        .any(|check| check.category == CheckCategory::ImplementationExists && !check.passed));
    assert!(!result.blocker_ticket_ids.is_empty());
    let blockers = FileTicketStore::new(project.path())
        .list(TicketFilter {
            status: Some(TicketStatus::Blocked),
            ..TicketFilter::default()
        })
        .expect("blocker tickets should list");
    assert!(blockers
        .iter()
        .any(|ticket| ticket.title.contains("Implementation Exists")));
}

#[test]
fn lux_e2e_full_ouroboros_loop() {
    let project = E2eProject::new();

    let lux_path = initialize_lux_project(project.path());
    assert!(lux_path.join("spec.json").is_file());

    let spec = run_mock_ai_spec_generation(project.path());
    assert!(spec.overall_ambiguity < 0.5);

    let tickets = create_kanban_tickets_from_spec(project.path(), &spec);
    assert_eq!(tickets.len(), 7);

    let (manager, build_id, artifact_path) = run_mock_webgl_build(project.path());
    assert_eq!(
        get_build_status(&manager, &build_id).unwrap().status,
        BuildStatus::Succeeded
    );
    assert!(artifact_path.is_file());

    let events = record_play_events(project.path());
    assert_eq!(events.len(), 3);

    let updated_spec = integrate_feedback_into_spec(project.path());
    assert!(updated_spec
        .domains
        .design
        .as_ref()
        .and_then(|domain| domain.fields.get("feedback_notes"))
        .is_some());

    let verification = run_verification_with_blocker(project.path());
    assert!(!verification.passed);
    assert!(!verification.blocker_ticket_ids.is_empty());
}

fn initialized_project() -> E2eProject {
    let project = E2eProject::new();
    initialize_lux_project(project.path());
    project
}

fn initialize_lux_project(project_path: &Path) -> PathBuf {
    let spec_path = lux_init(project_path).expect("lux init should create project metadata");
    RunState::idle(project_path)
        .expect("idle run state should be created")
        .save(project_path)
        .expect("run state should save");
    spec_path
}

fn run_mock_ai_spec_generation(project_path: &Path) -> SpecProject {
    let mut session =
        create_session(project_path).expect("session should be created without external AI");
    for phase in [
        SessionPhase::Phase1Experience,
        SessionPhase::Phase2Tetrad,
        SessionPhase::Phase3CoreLoop,
        SessionPhase::Phase4Motivation,
        SessionPhase::Phase5Assessment,
    ] {
        answer_phase_with_mock_model(&mut session, phase);
    }
    assert_eq!(session.status, SessionStatus::Completed);
    save_session(&session).expect("mock session should persist");

    let mut spec = complete_spec(project_path);
    apply_session_to_spec(&session, &mut spec).expect("mock AI transcript should update spec");
    spec.status = SpecStatus::Active;
    spec.overall_ambiguity = calculate_ambiguity(&spec)
        .overall_score
        .min(spec.overall_ambiguity);
    lux_save(project_path, &spec).expect("generated spec should save");
    spec
}

fn answer_phase_with_mock_model(session: &mut lux::lux_ai_session::AiSession, phase: SessionPhase) {
    for index in 1..=3 {
        let answer = mock_ai_answer(&phase, index);
        let response = process_message(session, &answer).expect("mock response should process");
        if response.phase_complete {
            break;
        }
    }
}

fn mock_ai_answer(phase: &SessionPhase, index: usize) -> String {
    let phase_label = match phase {
        SessionPhase::Phase1Experience => "experience",
        SessionPhase::Phase2Tetrad => "mechanics story aesthetics technology",
        SessionPhase::Phase3CoreLoop => "core loop",
        SessionPhase::Phase4Motivation => "player motivation",
        SessionPhase::Phase5Assessment => "assessment strength risk recommendation",
        SessionPhase::Completed => "completed",
    };
    format!(
        "Mock AI synthesis {index}: the {phase_label} target is clear, testable, fun, and grounded in player feedback with readable choices."
    )
}

fn complete_spec(project_path: &Path) -> SpecProject {
    let now = Utc::now().to_rfc3339();
    SpecProject {
        version: "1.0.0".to_string(),
        schema_version: "2.0".to_string(),
        project_id: uuid::Uuid::new_v4().to_string(),
        project_name: "Lux E2E Harness".to_string(),
        created_at: now.clone(),
        updated_at: now,
        source: "mock-ai-session".to_string(),
        status: SpecStatus::Active,
        meta: lux::lux_spec::ProjectMeta::default(),
        domains: SpecDomains {
            design: Some(domain(
                project_path,
                "design",
                &[
                    "core_loop",
                    "genre",
                    "player_count",
                    "session_length",
                    "win_condition",
                ],
            )),
            architecture: Some(domain(
                project_path,
                "architecture",
                &["engine", "platform", "networking", "data_storage"],
            )),
            art_style: Some(domain(
                project_path,
                "art_style",
                &[
                    "visual_style",
                    "color_palette",
                    "resolution",
                    "animation_style",
                ],
            )),
            audio: Some(domain(
                project_path,
                "audio",
                &["music_style", "sfx_list", "ambient_sounds", "dynamic_audio"],
            )),
            narrative: Some(domain(
                project_path,
                "narrative",
                &[
                    "story_arc",
                    "characters",
                    "dialogue_system",
                    "world_building",
                ],
            )),
            levels: Some(domain(
                project_path,
                "levels",
                &["level_count", "difficulty_curve", "level_generation"],
            )),
            ui_ux: Some(domain(
                project_path,
                "ui_ux",
                &["hud_layout", "menu_flow", "accessibility", "input_mapping"],
            )),
            custom: HashMap::new(),
        },
        dialectic: lux::lux_spec::DialecticState::default(),
        roadmap: lux::lux_spec::RoadmapSpec::default(),
        unity: None,
        targets: None,
        packages: None,
        testing: None,
        glossary: None,
        schell_evaluation: SchellEvaluation {
            phase1_experience: phase("Experience Lens"),
            phase2_tetrad: TetradResult {
                mechanics: pillar("moment-to-moment dash timing"),
                story: pillar("arena ritual framing"),
                aesthetics: pillar("neon silhouettes and crisp contrast"),
                technology: pillar("Unity WebGL with local telemetry"),
                harmony_score: 0.9,
            },
            phase3_core_loop: phase("Core Loop Stress Test"),
            phase4_motivation: phase("Player Motivation"),
            phase5_assessment: AssessmentResult {
                status: PillarStatus::Strong,
                viability_score: 0.9,
                strengths: vec!["clear readable loop".to_string()],
                risks: vec!["dash readability can regress".to_string()],
                recommendations: vec!["test telegraph timing after play sessions".to_string()],
                summary: Some("ready for loop verification".to_string()),
            },
        },
        overall_ambiguity: 0.1,
    }
}

fn domain(project_path: &Path, name: &str, fields: &[&str]) -> DomainSpec {
    let content_path = project_path.join(format!(".lux/domains/{name}.md"));
    fs::write(&content_path, domain_markdown(name)).expect("domain markdown should be written");
    let values = fields
        .iter()
        .map(|field| ((*field).to_string(), json!(format!("{name} {field} value"))))
        .collect::<HashMap<_, _>>();

    let mut domain = DomainSpec::new(name, content_path.display().to_string(), 0.1);
    domain.fields = values;
    domain.last_evaluated = Some(Utc::now().to_rfc3339());
    domain.defined = true;
    domain
}

fn domain_markdown(name: &str) -> String {
    let keywords = match name {
        "design" => "genre mechanic loop player win",
        "architecture" => "engine platform network storage system",
        "art_style" => "visual color resolution animation style",
        "audio" => "music sfx ambient dynamic sound",
        "narrative" => "story character dialogue world arc",
        "levels" => "level difficulty procedural handcrafted curve",
        "ui_ux" => "hud menu accessibility input flow",
        _ => "custom",
    };
    format!(
        "# {name}\n\nThis domain captures {keywords}. The implementation notes are intentionally detailed so ambiguity analysis can parse enough markdown structure for the end-to-end harness."
    )
}

fn phase(name: &str) -> PhaseResult {
    PhaseResult {
        name: name.to_string(),
        status: PillarStatus::Strong,
        summary: Some(format!("{name} is grounded and testable")),
        score: 0.9,
        questions: Vec::new(),
    }
}

fn pillar(description: &str) -> PillarRating {
    PillarRating {
        status: PillarStatus::Strong,
        description: Some(description.to_string()),
        score: 0.9,
    }
}

fn create_kanban_tickets_from_spec(project_path: &Path, spec: &SpecProject) -> Vec<Ticket> {
    let store = FileTicketStore::new(project_path);
    let now = Utc::now().to_rfc3339();
    all_spec_domains(spec)
        .into_iter()
        .map(|(name, domain)| {
            let ticket = Ticket {
                id: uuid::Uuid::new_v4().to_string(),
                title: format!("Implement {name} spec"),
                description: format!("Implement and verify {}", domain.content_path),
                status: TicketStatus::Backlog,
                priority: TicketPriority::High,
                assignee: Some("lux-e2e".to_string()),
                blockers: Vec::new(),
                tags: vec!["ouroboros".to_string(), name.to_string()],
                spec_ref: Some(domain.content_path.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
                ..Default::default()
            };
            store.create(ticket).expect("ticket should be created")
        })
        .collect()
}

fn all_spec_domains(spec: &SpecProject) -> Vec<(&'static str, &DomainSpec)> {
    vec![
        ("design", spec.domains.design.as_ref().unwrap()),
        ("architecture", spec.domains.architecture.as_ref().unwrap()),
        ("art_style", spec.domains.art_style.as_ref().unwrap()),
        ("audio", spec.domains.audio.as_ref().unwrap()),
        ("narrative", spec.domains.narrative.as_ref().unwrap()),
        ("levels", spec.domains.levels.as_ref().unwrap()),
        ("ui_ux", spec.domains.ui_ux.as_ref().unwrap()),
    ]
}

fn run_mock_webgl_build(project_path: &Path) -> (BuildManager, String, PathBuf) {
    let mut manager = BuildManager::with_project_root(Some(project_path));
    let build_id = start_build(&mut manager, project_path, BuildTarget::WebGL)
        .expect("WebGL build job should be queued");
    mark_build_running(&mut manager, &build_id).expect("build should run");
    append_build_log(&mut manager, &build_id, "Mock WebGL artifact generated")
        .expect("build log should append");
    mark_build_succeeded(&mut manager, &build_id).expect("build should succeed");

    let artifact_path = get_build_artifact_path(&build_id, &manager.base_output_dir);
    fs::create_dir_all(artifact_path.parent().unwrap()).expect("artifact parent should exist");
    fs::write(&artifact_path, "<html><body>Lux E2E playable</body></html>")
        .expect("artifact should be written");
    fs::write(artifact_path.parent().unwrap().join("success.json"), "{}")
        .expect("success marker should be written");

    assert_eq!(
        get_build_status(&manager, &build_id).unwrap().status,
        BuildStatus::Succeeded
    );
    (manager, build_id, artifact_path)
}

fn record_play_events(project_path: &Path) -> Vec<PlayEvent> {
    let store = FileEventLogStore::new(project_path.join(".lux/logs"));
    store
        .create_session(SessionMetadata {
            session_id: "play-session-e2e".to_string(),
            started_at: Utc::now().to_rfc3339(),
            ended_at: None,
            duration_secs: None,
            event_count: 0,
            webgl_build_version: Some("mock-webgl".to_string()),
            player_id: Some("player-e2e".to_string()),
            metadata: HashMap::new(),
        })
        .expect("play session should be created");

    let events = vec![
        play_event(PlayEventType::LevelStart, 1, json!({"level": 1})),
        play_event(PlayEventType::Decision, 2, json!({"choice": "dash_left"})),
        play_event(
            PlayEventType::LevelComplete,
            3,
            json!({"duration_secs": 74}),
        ),
    ];
    for event in &events {
        store
            .append_event(event.clone())
            .expect("play event should append");
    }
    store
        .end_session("play-session-e2e")
        .expect("play session should end");
    store
        .query_events(EventFilter {
            session_id: Some("play-session-e2e".to_string()),
            ..EventFilter::default()
        })
        .expect("play events should query")
}

fn play_event(event_type: PlayEventType, sequence: u64, payload: serde_json::Value) -> PlayEvent {
    PlayEvent {
        session_id: "play-session-e2e".to_string(),
        timestamp: (Utc::now() + Duration::milliseconds(sequence as i64)).to_rfc3339(),
        event_type,
        payload,
        player_id: Some("player-e2e".to_string()),
        game_state: Some(json!({"hp": 3, "loop": "ouroboros"})),
        sequence,
    }
}

fn integrate_feedback_into_spec(project_path: &Path) -> SpecProject {
    let feedback_path = project_path.join(".lux/logs/play-session-e2e.feedback.json");
    fs::write(
        &feedback_path,
        serde_json::to_string_pretty(&json!({
            "session_id": "play-session-e2e",
            "issue": "dash readability",
            "recommendation": "telegraph dash hazards earlier"
        }))
        .unwrap(),
    )
    .expect("feedback should be written");

    let mut spec = lux_load(project_path).expect("spec should load for feedback integration");
    let design = spec
        .domains
        .design
        .as_mut()
        .expect("design domain should exist");
    design.fields.insert(
        "feedback_notes".to_string(),
        json!("Integrated play feedback: dash readability needs stronger telegraphing."),
    );
    spec.schell_evaluation
        .phase5_assessment
        .recommendations
        .push("telegraph dash hazards earlier".to_string());
    spec.updated_at = (Utc::now() + Duration::seconds(2)).to_rfc3339();
    lux_save(project_path, &spec).expect("feedback-updated spec should save");
    spec
}

fn run_verification_with_blocker(project_path: &Path) -> lux::lux_verification::VerificationResult {
    let missing = lux_load(project_path)
        .expect("spec should load before blocker verification")
        .domains
        .architecture
        .expect("architecture domain should exist")
        .content_path;
    let _ = fs::remove_file(missing);

    verify_all(project_path, VerificationMode::Cached)
        .expect("verification should complete and create blockers")
}
