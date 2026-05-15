#[path = "../src/lux_ambiguity.rs"]
mod lux_ambiguity;
#[path = "../src/lux_roadmap.rs"]
mod lux_roadmap;
#[path = "../src/lux_spec.rs"]
mod lux_spec;
#[path = "../src/project.rs"]
mod project;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lux_spec::{
    lux_init, lux_load, lux_load_domain, lux_save, lux_save_domain, lux_update_domain_field,
    render_markdown_template, DomainSpec, GlossarySpec, PackageEntry, PackagesSpec, PillarStatus,
    SpecDomains, SpecProject, SpecStatus, TargetsSpec, TestingSpec, UnitySpec,
};
use serde_json::json;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new() -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("lux-spec-test-{}-{count}", std::process::id()));
        std::fs::create_dir(&path).expect("temp directory should be created");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_spec_schema_validates() {
    let spec = SpecProject::default();

    assert!(spec.validate().is_ok());
    assert_eq!(spec.status, SpecStatus::Draft);
}

#[test]
fn test_spec_schema_rejects_bad_version() {
    let mut spec = SpecProject::default();
    spec.schema_version = "9.0".to_string();
    spec.version = "9.0.0".to_string();

    let error = spec
        .validate()
        .expect_err("unsupported version should fail");
    assert!(error.contains("unsupported spec version"));
}

#[test]
fn test_spec_domains_default_empty() {
    let domains = SpecDomains::default();

    assert!(domains.design.is_none());
    assert!(domains.architecture.is_none());
    assert!(domains.art_style.is_none());
    assert!(domains.audio.is_none());
    assert!(domains.narrative.is_none());
    assert!(domains.levels.is_none());
    assert!(domains.ui_ux.is_none());
    assert!(domains.custom.is_empty());
}

#[test]
fn test_schell_evaluation_default_missing() {
    let spec = SpecProject::default();
    let evaluation = spec.schell_evaluation;

    assert_eq!(evaluation.phase1_experience.status, PillarStatus::Missing);
    assert_eq!(
        evaluation.phase2_tetrad.mechanics.status,
        PillarStatus::Missing
    );
    assert_eq!(evaluation.phase2_tetrad.story.status, PillarStatus::Missing);
    assert_eq!(
        evaluation.phase2_tetrad.aesthetics.status,
        PillarStatus::Missing
    );
    assert_eq!(
        evaluation.phase2_tetrad.technology.status,
        PillarStatus::Missing
    );
    assert_eq!(evaluation.phase3_core_loop.status, PillarStatus::Missing);
    assert_eq!(evaluation.phase4_motivation.status, PillarStatus::Missing);
    assert_eq!(evaluation.phase5_assessment.status, PillarStatus::Missing);
}

#[test]
fn test_domain_spec_ambiguity_range() {
    let low = DomainSpec::new("design", "design.md", -0.5);
    let high = DomainSpec::new("audio", "audio.md", 1.5);
    let exact = DomainSpec::new("levels", "levels.md", 0.42);

    assert_eq!(low.ambiguity_score, 0.0);
    assert_eq!(high.ambiguity_score, 1.0);
    assert_eq!(exact.ambiguity_score, 0.42);
}

#[test]
fn test_unity_spec_validates() {
    let spec = UnitySpec::default();
    assert!(spec.validate().is_ok());

    let mut bad_pipeline = UnitySpec::default();
    bad_pipeline.render_pipeline = Some("invalid".to_string());
    assert!(bad_pipeline.validate().is_err());

    let mut bad_backend = UnitySpec::default();
    bad_backend.scripting_backend = Some("invalid".to_string());
    assert!(bad_backend.validate().is_err());
}

#[test]
fn test_targets_spec_validates() {
    let spec = TargetsSpec::default();
    assert!(spec.validate().is_ok());
    assert!(spec.platforms.is_empty());
}

#[test]
fn test_packages_spec_validates() {
    let spec = PackagesSpec::default();
    assert!(spec.validate().is_ok());
    assert!(spec.required.is_empty());
    assert!(spec.forbidden.is_empty());
}

#[test]
fn test_package_entry_rejects_empty_name() {
    let mut entry = PackageEntry::default();
    assert!(entry.validate().is_err());
    entry.name = "com.unity.textmeshpro".to_string();
    assert!(entry.validate().is_ok());
}

#[test]
fn test_testing_spec_default() {
    let spec = TestingSpec::default();
    assert!(spec.framework.is_none());
    assert!(!spec.coverage);
}

#[test]
fn test_glossary_spec_default() {
    let spec = GlossarySpec::default();
    assert_eq!(spec.path, "glossary.md");
    assert_eq!(spec.term_count, 0);
}

#[test]
fn test_spec_new_fields_backward_compatible() {
    let old_json = r#"{"version":"1.0.0","project_id":"","project_name":"","created_at":"","updated_at":"","source":"lux-init","status":"Draft","domains":{"design":null,"architecture":null,"art_style":null,"audio":null,"narrative":null,"levels":null,"ui_ux":null,"custom":{}},"schell_evaluation":{"phase1_experience":{"name":"Experience Lens","status":"Missing","summary":null,"score":0.0,"questions":[]},"phase2_tetrad":{"mechanics":{"status":"Missing","description":null,"score":0.0},"story":{"status":"Missing","description":null,"score":0.0},"aesthetics":{"status":"Missing","description":null,"score":0.0},"technology":{"status":"Missing","description":null,"score":0.0},"harmony_score":0.0},"phase3_core_loop":{"name":"Core Loop Stress Test","status":"Missing","summary":null,"score":0.0,"questions":[]},"phase4_motivation":{"name":"Player Motivation","status":"Missing","summary":null,"score":0.0,"questions":[]},"phase5_assessment":{"status":"Missing","viability_score":0.0,"strengths":[],"risks":[],"recommendations":[],"summary":null}},"overall_ambiguity":1.0}"#;
    let spec: SpecProject = serde_json::from_str(old_json).expect("old spec should parse");
    assert!(spec.unity.is_none());
    assert!(spec.targets.is_none());
    assert!(spec.packages.is_none());
    assert!(spec.testing.is_none());
    assert!(spec.glossary.is_none());
}

#[test]
fn test_spec_new_fields_roundtrip() {
    let mut spec = SpecProject::default();
    spec.unity = Some(UnitySpec {
        required_version: Some("6000.0".to_string()),
        detected_version: None,
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
        ..UnitySpec::default()
    });
    spec.targets = Some(TargetsSpec {
        platforms: vec!["android".to_string(), "ios".to_string()],
        min_sdk: HashMap::new(),
        test_platform: Some("android".to_string()),
        target_platforms: Vec::new(),
    });

    let json = serde_json::to_string_pretty(&spec).expect("serialize");
    let parsed: SpecProject = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        parsed.unity.as_ref().unwrap().render_pipeline,
        Some("urp".to_string())
    );
    assert_eq!(parsed.targets.as_ref().unwrap().platforms.len(), 2);
}

#[test]
fn test_lux_init_creates_directory_structure() {
    let temp = TestTempDir::new();
    let lux_path = lux_init(temp.path()).expect("lux init should succeed");

    assert_eq!(lux_path, temp.path().join(".lux"));
    assert!(lux_path.join("spec.json").is_file());
    assert!(lux_path.join("roadmap.json").is_file());

    for directory in [
        "tickets", "logs", "backups", "sessions", "builds", "domains",
    ] {
        assert!(
            lux_path.join(directory).is_dir(),
            "{directory} should exist"
        );
    }

    for domain in [
        "design",
        "architecture",
        "art-style",
        "audio",
        "narrative",
        "levels",
        "ui-ux",
    ] {
        assert!(
            lux_path
                .join("domains")
                .join(format!("{domain}.md"))
                .is_file(),
            "{domain}.md should exist"
        );
    }

    let spec = lux_load(temp.path()).expect("spec should load");
    assert!(!spec.project_id.is_empty());
    assert_eq!(
        spec.project_name,
        temp.path().file_name().unwrap().to_string_lossy()
    );
}

#[test]
fn test_lux_init_is_idempotent() {
    let temp = TestTempDir::new();

    lux_init(temp.path()).expect("first init should succeed");

    assert_eq!(lux_init(temp.path()).unwrap(), temp.path().join(".lux"));
}

#[test]
fn test_lux_load_saves_roundtrip() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let mut spec = lux_load(temp.path()).expect("spec should load");
    spec.project_name = "Roundtrip".to_string();
    spec.domains.design = Some(DomainSpec::new("design", "design.md", 0.25));

    lux_save(temp.path(), &spec).expect("spec should save");
    let loaded = lux_load(temp.path()).expect("saved spec should load");

    assert_eq!(loaded.project_name, "Roundtrip");
    assert_eq!(loaded.project_id, spec.project_id);
    assert_eq!(loaded.domains.design.unwrap().ambiguity_score, 0.25);
}

#[test]
fn test_lux_save_creates_backup() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");
    let spec = lux_load(temp.path()).expect("spec should load");

    lux_save(temp.path(), &spec).expect("spec should save");

    let backups = std::fs::read_dir(temp.path().join(".lux/backups"))
        .expect("backups directory should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("backup entries should be readable");

    assert_eq!(backups.len(), 1);
    assert!(backups[0]
        .file_name()
        .to_string_lossy()
        .starts_with("spec-"));
}

#[test]
fn test_lux_domain_read_write() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    lux_save_domain(temp.path(), "design", "# Updated Design\n").expect("domain should save");
    let content = lux_load_domain(temp.path(), "design").expect("domain should load");

    assert_eq!(content, "# Updated Design\n");
}

#[test]
fn test_lux_update_domain_field() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let spec = lux_update_domain_field(temp.path(), "design", "core_loop", json!("jump collect"))
        .expect("domain field should update");
    let design = spec.domains.design.expect("design domain should exist");

    assert!(design.defined);
    assert_eq!(design.fields.get("core_loop"), Some(&json!("jump collect")));

    let loaded = lux_load(temp.path()).expect("updated spec should load");
    assert_eq!(
        loaded
            .domains
            .design
            .expect("design domain should persist")
            .fields
            .get("core_loop"),
        Some(&json!("jump collect"))
    );
}

#[test]
fn test_render_markdown_template() {
    let mut vars = HashMap::new();
    vars.insert("domain".to_string(), "Design".to_string());

    let rendered = render_markdown_template("design", &vars).expect("template should render");

    assert!(rendered.contains("# Design Spec"));
    assert!(!rendered.contains("{{domain}}"));
}
