use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use lux::lux_spec::{
    AssessmentResult, DomainSpec, PhaseResult, PillarRating, PillarStatus, SchellEvaluation,
    SpecDomains, SpecProject, TetradResult,
};
use lux::lux_ticket::{FileTicketStore, TicketFilter, TicketPriority, TicketStatus, TicketStore};
use lux::lux_verification::{
    create_blocker_tickets, get_latest_verification, save_verification_result, verify_all,
    CheckCategory, CheckResult, VerificationResult,
};
use serde_json::json;

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-verification-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(path.join(".lux/builds")).unwrap();
        fs::create_dir_all(path.join(".lux/logs")).unwrap();
        fs::create_dir_all(path.join(".lux/tickets")).unwrap();
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

fn domain(name: &str, ambiguity_score: f64) -> DomainSpec {
    let mut fields = HashMap::new();
    fields.insert("summary".to_string(), json!(format!("{name} summary")));
    DomainSpec {
        name: name.to_string(),
        content_path: format!(".lux/domains/{name}.md"),
        fields,
        ambiguity_score,
        last_evaluated: Some("2026-05-11T00:00:00Z".to_string()),
        defined: true,
    }
}

fn complete_spec() -> SpecProject {
    SpecProject {
        version: "1.0.0".to_string(),
        project_id: uuid::Uuid::new_v4().to_string(),
        project_name: "Verification Fixture".to_string(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
        updated_at: "2026-05-11T00:00:10Z".to_string(),
        source: "test".to_string(),
        status: lux::lux_spec::SpecStatus::Active,
        domains: SpecDomains {
            design: Some(domain("design", 0.1)),
            architecture: Some(domain("architecture", 0.1)),
            art_style: Some(domain("art_style", 0.1)),
            audio: Some(domain("audio", 0.1)),
            narrative: Some(domain("narrative", 0.1)),
            levels: Some(domain("levels", 0.1)),
            ui_ux: Some(domain("ui_ux", 0.1)),
            custom: HashMap::new(),
        },
        unity: None,
        targets: None,
        packages: None,
        testing: None,
        glossary: None,
        schell_evaluation: SchellEvaluation {
            phase1_experience: phase("Experience Lens"),
            phase2_tetrad: TetradResult {
                mechanics: pillar(),
                story: pillar(),
                aesthetics: pillar(),
                technology: pillar(),
                harmony_score: 0.9,
            },
            phase3_core_loop: phase("Core Loop Stress Test"),
            phase4_motivation: phase("Player Motivation"),
            phase5_assessment: AssessmentResult {
                status: PillarStatus::Strong,
                viability_score: 0.9,
                strengths: vec!["clear".to_string()],
                risks: Vec::new(),
                recommendations: vec!["ship".to_string()],
                summary: Some("ready".to_string()),
            },
        },
        overall_ambiguity: 0.1,
    }
}

fn phase(name: &str) -> PhaseResult {
    PhaseResult {
        name: name.to_string(),
        status: PillarStatus::Strong,
        summary: Some("evaluated".to_string()),
        score: 0.9,
        questions: Vec::new(),
    }
}

fn pillar() -> PillarRating {
    PillarRating {
        status: PillarStatus::Strong,
        description: Some("strong".to_string()),
        score: 0.9,
    }
}

fn save_spec(project: &TestProject, spec: &SpecProject) {
    fs::create_dir_all(project.path().join(".lux")).unwrap();
    fs::write(
        project.path().join(".lux/spec.json"),
        serde_json::to_string_pretty(spec).unwrap(),
    )
    .unwrap();
}

fn create_full_implementation(project: &TestProject) {
    for domain in [
        "design",
        "architecture",
        "art_style",
        "audio",
        "narrative",
        "levels",
        "ui_ux",
    ] {
        let path = project.path().join(format!(".lux/domains/{domain}.md"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, domain).unwrap();
    }
    let build = project.path().join(".lux/builds/20260511000000");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("success.json"), "{}").unwrap();
    fs::write(build.join("index.html"), "<html></html>").unwrap();
}

#[test]
fn test_verify_all_empty_project() {
    let project = TestProject::new("empty");

    let result = verify_all(project.path()).unwrap();

    assert!(!result.passed);
    assert_eq!(result.checks.len(), 5);
    assert!(result.overall_score < 0.5);
    assert!(result.blocker_ticket_ids.len() >= 3);
}

#[test]
fn test_verify_spec_completeness() {
    let project = TestProject::new("spec-completeness");
    let mut spec = complete_spec();
    spec.domains.design.as_mut().unwrap().ambiguity_score = 0.75;
    save_spec(&project, &spec);

    let result = verify_all(project.path()).unwrap();
    let check = result
        .checks
        .iter()
        .find(|check| check.category == CheckCategory::SpecCompleteness)
        .unwrap();

    assert!(!check.passed);
    assert!((check.score - (6.0 / 7.0)).abs() < 0.0001);
}

#[test]
fn test_verify_with_complete_spec() {
    let project = TestProject::new("complete");
    save_spec(&project, &complete_spec());
    create_full_implementation(&project);

    let result = verify_all(project.path()).unwrap();

    assert!(result.passed);
    assert_eq!(result.blocker_ticket_ids.len(), 0);
    assert!((result.overall_score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_create_blocker_tickets() {
    let project = TestProject::new("blockers");
    let result = VerificationResult {
        passed: false,
        timestamp: "2026-05-11T00:00:00Z".to_string(),
        checks: vec![
            CheckResult {
                name: "Spec Completeness".to_string(),
                category: CheckCategory::SpecCompleteness,
                passed: false,
                score: 0.0,
                message: "missing".to_string(),
                details: None,
            },
            CheckResult {
                name: "WebGL Playable".to_string(),
                category: CheckCategory::WebGLPlayable,
                passed: false,
                score: 0.0,
                message: "missing".to_string(),
                details: None,
            },
        ],
        overall_score: 0.0,
        blocker_ticket_ids: Vec::new(),
    };

    let ids = create_blocker_tickets(&result, project.path()).unwrap();
    let store = FileTicketStore::new(project.path());
    let tickets = store.list(TicketFilter::default()).unwrap();

    assert_eq!(ids.len(), 2);
    assert_eq!(tickets.len(), 2);
    assert!(tickets
        .iter()
        .all(|ticket| ticket.status == TicketStatus::Blocked));
    assert!(tickets
        .iter()
        .any(|ticket| ticket.priority == TicketPriority::Critical));
}

#[test]
fn test_save_load_verification() {
    let project = TestProject::new("persist");
    let result = VerificationResult {
        passed: true,
        timestamp: "2026-05-11T00:00:00Z".to_string(),
        checks: Vec::new(),
        overall_score: 1.0,
        blocker_ticket_ids: Vec::new(),
    };

    save_verification_result(&result, project.path()).unwrap();
    let loaded = get_latest_verification(project.path()).unwrap().unwrap();

    assert_eq!(loaded, result);
    assert!(project
        .path()
        .join(".lux/verification/2026-05-11T00:00:00Z.json")
        .exists());
}

#[test]
fn test_verification_scoring() {
    let project = TestProject::new("scoring");
    let mut spec = complete_spec();
    spec.domains.design.as_mut().unwrap().ambiguity_score = 0.9;
    spec.domains.audio.as_mut().unwrap().ambiguity_score = 0.8;
    save_spec(&project, &spec);

    let result = verify_all(project.path()).unwrap();
    let spec_score = result
        .checks
        .iter()
        .find(|check| check.category == CheckCategory::SpecCompleteness)
        .unwrap()
        .score;

    assert!((spec_score - (5.0 / 7.0)).abs() < 0.0001);
    assert!((0.0..=1.0).contains(&result.overall_score));
}
