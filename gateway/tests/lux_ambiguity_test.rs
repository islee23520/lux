use std::fs;
use std::path::PathBuf;

use lux::lux_ambiguity::calculate_ambiguity;
use lux::lux_spec::{
    DomainSpec, GlossarySpec, PackageEntry, PackagesSpec, SpecProject, TargetsSpec, TestingSpec,
    UnitySpec,
};
use serde_json::json;

#[test]
fn test_ambiguity_empty_spec() {
    let spec = SpecProject::default();

    let report = calculate_ambiguity(&spec);

    assert_score_close(report.overall_score, 0.0);
    assert_score_close(report.completion_ratio, 0.0);
    assert_eq!(report.domain_scores.len(), 7);
    assert!(report.targeted_questions.len() >= 21);
}

#[test]
fn test_ambiguity_full_spec() {
    let workspace = TestWorkspace::new("full");
    let spec = full_spec(&workspace);

    let report = calculate_ambiguity(&spec);

    assert!(
        report.overall_score >= 0.8,
        "overall score was {}",
        report.overall_score
    );
    assert_score_close(report.completion_ratio, 1.0);
    assert!(report.targeted_questions.is_empty());
    for domain in report.domain_scores.values() {
        assert!(
            domain.composite_score >= 0.8,
            "{} was {}",
            domain.domain_name,
            domain.composite_score
        );
        assert!(
            domain.missing_fields.is_empty(),
            "{} missing fields: {:?}",
            domain.domain_name,
            domain.missing_fields
        );
    }
}

#[test]
fn test_ambiguity_partial_spec() {
    let workspace = TestWorkspace::new("partial");
    let mut spec = SpecProject::default();
    spec.domains.design = Some(domain(
        "design",
        workspace.markdown_path(
            "design",
            "# Design\n- Genre: arcade\n- Mechanic: movement\n",
        ),
        &["genre", "core_loop"],
    ));
    spec.domains.architecture = Some(domain(
        "architecture",
        workspace.missing_path("architecture"),
        &["engine"],
    ));

    let report = calculate_ambiguity(&spec);

    assert!(report.overall_score > 0.0);
    assert!(report.overall_score < 0.8);
    assert_score_close(report.completion_ratio, 2.0 / 7.0);
    assert!(
        report.domain_scores["design"].composite_score
            > report.domain_scores["audio"].composite_score
    );
}

#[test]
fn test_ambiguity_composite_formula() {
    let workspace = TestWorkspace::new("formula");
    let mut spec = SpecProject::default();
    spec.domains.design = Some(domain(
        "design",
        workspace.markdown_path("design", "# Design\n- loop\n- player\n"),
        &["genre", "core_loop", "player_count"],
    ));

    let report = calculate_ambiguity(&spec);
    let design = &report.domain_scores["design"];
    let expected = (0.40 * design.completion_ratio)
        + (0.35 * design.ai_eval_score)
        + (0.25 * design.ast_parsability);

    assert_score_close(design.composite_score, expected);
}

#[test]
fn test_ambiguity_targeted_questions() {
    let spec = SpecProject::default();

    let report = calculate_ambiguity(&spec);

    let design_questions = report
        .targeted_questions
        .iter()
        .filter(|question| question.domain == "design")
        .collect::<Vec<_>>();

    assert_eq!(design_questions.len(), 3);
    assert!(design_questions
        .iter()
        .any(|question| question.question == "What is the core game loop?"));
    assert!(design_questions
        .iter()
        .all(|question| question.phase == "phase3_core_loop"));
    assert!(design_questions
        .iter()
        .all(|question| question.priority > 0.0));
}

#[test]
fn test_ambiguity_schell_phases() {
    let workspace = TestWorkspace::new("schell");
    let spec = full_spec(&workspace);

    let report = calculate_ambiguity(&spec);
    let tetrad_expected = average(&[
        report.domain_scores["design"].composite_score,
        report.domain_scores["narrative"].composite_score,
        report.domain_scores["art_style"].composite_score,
        report.domain_scores["architecture"].composite_score,
    ]);
    let core_loop_expected = average(&[
        report.domain_scores["design"].composite_score,
        report.domain_scores["levels"].composite_score,
        report.domain_scores["ui_ux"].composite_score,
    ]);

    assert_eq!(report.schell_phase_scores.len(), 5);
    assert_score_close(report.schell_phase_scores["phase2_tetrad"], tetrad_expected);
    assert_score_close(
        report.schell_phase_scores["phase3_core_loop"],
        core_loop_expected,
    );
}

#[test]
fn test_ambiguity_score_clamped() {
    let workspace = TestWorkspace::new("clamped");
    let mut spec = full_spec(&workspace);
    spec.domains
        .design
        .as_mut()
        .unwrap()
        .fields
        .insert("extra".to_string(), json!("overflow"));

    let report = calculate_ambiguity(&spec);

    assert_clamped(report.overall_score);
    assert_clamped(report.completion_ratio);
    for domain in report.domain_scores.values() {
        assert_clamped(domain.completion_ratio);
        assert_clamped(domain.ai_eval_score);
        assert_clamped(domain.ast_parsability);
        assert_clamped(domain.composite_score);
    }
    for score in report.schell_phase_scores.values() {
        assert_clamped(*score);
    }
    for question in &report.targeted_questions {
        assert_clamped(question.priority);
    }
}

fn full_spec(workspace: &TestWorkspace) -> SpecProject {
    let mut spec = SpecProject::default();
    spec.unity = Some(UnitySpec {
        required_version: Some("6000.0.0f1".to_string()),
        detected_version: Some("6000.0.0f1".to_string()),
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
    });
    spec.targets = Some(TargetsSpec {
        platforms: vec!["windows".to_string(), "macos".to_string()],
        min_sdk: Default::default(),
        test_platform: Some("windows".to_string()),
    });
    spec.packages = Some(PackagesSpec {
        required: vec![PackageEntry {
            name: "com.unity.inputsystem".to_string(),
            reason: Some("input handling".to_string()),
            version: Some("1.7.0".to_string()),
        }],
        forbidden: vec![],
        detected: vec![],
    });
    spec.testing = Some(TestingSpec {
        framework: Some("nunit".to_string()),
        strategy: Some("playmode and editmode smoke coverage".to_string()),
        coverage: true,
    });
    spec.glossary = Some(GlossarySpec {
        path: workspace.markdown_path(
            "glossary",
            "# Glossary\n- Term: Canonical meaning for project planning.\n",
        ).to_string_lossy().to_string(),
        last_updated: Some("2026-05-11T00:00:00Z".to_string()),
        term_count: 1,
    });
    spec.domains.design = Some(domain(
        "design",
        workspace.markdown_path(
            "design",
            full_markdown("Design", &["genre", "mechanic", "loop", "player", "win"]),
        ),
        &[
            "core_loop",
            "genre",
            "player_count",
            "session_length",
            "win_condition",
        ],
    ));
    spec.domains.architecture = Some(domain(
        "architecture",
        workspace.markdown_path(
            "architecture",
            full_markdown(
                "Architecture",
                &["engine", "platform", "network", "storage", "system"],
            ),
        ),
        &["engine", "platform", "networking", "data_storage"],
    ));
    spec.domains.art_style = Some(domain(
        "art_style",
        workspace.markdown_path(
            "art_style",
            full_markdown(
                "Art Style",
                &["visual", "color", "resolution", "animation", "style"],
            ),
        ),
        &[
            "visual_style",
            "color_palette",
            "resolution",
            "animation_style",
        ],
    ));
    spec.domains.audio = Some(domain(
        "audio",
        workspace.markdown_path(
            "audio",
            full_markdown("Audio", &["music", "sfx", "ambient", "dynamic", "sound"]),
        ),
        &["music_style", "sfx_list", "ambient_sounds", "dynamic_audio"],
    ));
    spec.domains.narrative = Some(domain(
        "narrative",
        workspace.markdown_path(
            "narrative",
            full_markdown(
                "Narrative",
                &["story", "character", "dialogue", "world", "arc"],
            ),
        ),
        &[
            "story_arc",
            "characters",
            "dialogue_system",
            "world_building",
        ],
    ));
    spec.domains.levels = Some(domain(
        "levels",
        workspace.markdown_path(
            "levels",
            full_markdown(
                "Levels",
                &["level", "difficulty", "procedural", "handcrafted", "curve"],
            ),
        ),
        &["level_count", "difficulty_curve", "level_generation"],
    ));
    spec.domains.ui_ux = Some(domain(
        "ui_ux",
        workspace.markdown_path(
            "ui_ux",
            full_markdown("UI UX", &["hud", "menu", "accessibility", "input", "flow"]),
        ),
        &["hud_layout", "menu_flow", "accessibility", "input_mapping"],
    ));
    spec
}

fn domain(name: &str, content_path: PathBuf, fields: &[&str]) -> DomainSpec {
    let mut domain = DomainSpec::new(name, content_path.to_string_lossy(), 0.0);
    domain.defined = true;
    for field in fields {
        domain
            .fields
            .insert((*field).to_string(), json!(format!("{field} value")));
    }
    domain
}

fn full_markdown(title: &str, keywords: &[&str]) -> String {
    let bullets = keywords
        .iter()
        .map(|keyword| {
            format!("- {keyword}: detailed concrete requirement with acceptance criteria.")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("# {title}\n## Scope\n{bullets}\n## Constraints\n- Every choice is explicit and testable for implementation readiness.\n")
}

fn average(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn assert_score_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 0.000_001,
        "left={left}, right={right}"
    );
}

fn assert_clamped(score: f64) {
    assert!(
        (0.0..=1.0).contains(&score),
        "score {score} outside 0.0..=1.0"
    );
}

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "lux_ambiguity_{name}_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn markdown_path(&self, name: &str, content: impl AsRef<str>) -> PathBuf {
        let path = self.root.join(format!("{name}.md"));
        fs::write(&path, content.as_ref()).unwrap();
        path
    }

    fn missing_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.md"))
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
