use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lux::lux_ambiguity::TargetedQuestion;
use lux::lux_spec::{
    answer_direct, lux_init, lux_load, lux_load_domain, lux_save, lux_save_domain,
    lux_update_domain_field, render_markdown_template, DomainSpec, GlossarySpec, PackageEntry,
    PackagesSpec, PillarStatus, SpecDecision, SpecDomains, SpecProject, SpecStatus, TargetsSpec,
    TestingSpec, UnitySpec,
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

    assert!(domains.gdd.is_none());
    assert!(domains.mechanics.is_none());
    assert!(domains.controls.is_none());
    assert!(domains.camera.is_none());
    assert!(domains.art_style.is_none());
    assert!(domains.audio.is_none());
    assert!(domains.narrative.is_none());
    assert!(domains.levels.is_none());
    assert!(domains.technical_architecture.is_none());
    assert!(domains.engine.is_none());
    assert!(domains.testing.is_none());
    assert!(domains.build_release.is_none());
    assert!(domains.design.is_none());
    assert!(domains.architecture.is_none());
    assert!(domains.ui_ux.is_none());
    assert!(domains.custom.is_empty());
}

#[test]
fn test_spec_domains_legacy_aliases_map_to_canonical_game_domains() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    lux_update_domain_field(
        temp.path(),
        "design",
        "core_loop",
        json!("route, collect, refine"),
    )
    .expect("design alias should update gdd");
    lux_update_domain_field(
        temp.path(),
        "architecture",
        "engine_contract",
        json!("server-client bridge"),
    )
    .expect("architecture alias should update technical architecture");

    let loaded = lux_load(temp.path()).expect("updated spec should load");
    assert!(
        loaded
            .domains
            .gdd
            .as_ref()
            .expect("gdd should exist")
            .defined
    );
    assert_eq!(
        loaded
            .domains
            .gdd
            .as_ref()
            .expect("gdd should exist")
            .fields
            .get("core_loop"),
        Some(&json!("route, collect, refine"))
    );
    assert!(
        loaded
            .domains
            .technical_architecture
            .as_ref()
            .expect("technical architecture should exist")
            .defined
    );
    assert_eq!(
        loaded
            .domains
            .technical_architecture
            .as_ref()
            .expect("technical architecture should exist")
            .fields
            .get("engine_contract"),
        Some(&json!("server-client bridge"))
    );
}

#[test]
fn test_custom_domain_roundtrips_through_validate_and_serialization() {
    let mut spec = SpecProject::default();
    let mut custom = DomainSpec::new("boss-fights", "boss-fights.md", 0.27);
    custom.defined = true;
    custom
        .fields
        .insert("scope".to_string(), json!("prototype"));
    spec.domains
        .custom
        .insert("boss-fights".to_string(), custom);

    spec.validate().expect("custom domain should validate");

    let json = serde_json::to_string_pretty(&spec).expect("spec should serialize");
    let roundtrip: SpecProject = serde_json::from_str(&json).expect("spec should deserialize");

    let custom = roundtrip
        .domains
        .custom
        .get("boss-fights")
        .expect("custom domain should round-trip");
    assert!(custom.defined);
    assert_eq!(custom.name, "boss-fights");
    assert_eq!(custom.content_path, "boss-fights.md");
    assert_eq!(custom.fields.get("scope"), Some(&json!("prototype")));
}

#[test]
fn test_custom_domain_without_provenance_is_rejected() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let error = lux_update_domain_field(temp.path(), "economy", "scope", json!("trade loops"))
        .expect_err("custom domain without decision provenance should be rejected");
    assert!(
        error.to_string().contains("decision"),
        "unexpected error: {error}"
    );

    let mut spec = lux_load(temp.path()).expect("spec should load");
    spec.dialectic.decisions.push(SpecDecision {
        id: "decision-1".to_string(),
        domain: Some("economy".to_string()),
        text: "Add economy domain".to_string(),
        rationale: Some("The project needs a trade and economy specification.".to_string()),
        source_question: Some("Should this game include a trade/economy domain?".to_string()),
        created_at: Some("2026-06-01T00:00:00Z".to_string()),
    });
    lux_save(temp.path(), &spec).expect("spec should save");

    lux_update_domain_field(temp.path(), "economy", "scope", json!("trade loops"))
        .expect("custom domain with decision provenance should be accepted");
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
    let low = DomainSpec::new("mechanics", "mechanics.md", -0.5);
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
fn test_legacy_game_domain_aliases_migrate_into_canonical_domains() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux workspace should initialize");

    let legacy_spec = r#"{
        "version":"1.0.0",
        "schema_version":"2.0",
        "project_id":"legacy-game",
        "project_name":"Legacy Game",
        "created_at":"2026-05-11T00:00:00Z",
        "updated_at":"2026-05-11T00:00:00Z",
        "source":"lux-init",
        "status":"Draft",
        "domains":{
            "design":{
                "name":"design",
                "content_path":"design.md",
                "fields":{"summary":"legacy design"},
                "ambiguity_score":0.4,
                "last_evaluated":null,
                "defined":true,
                "kind":"Experience",
                "status":"Defined",
                "goals":["legacy loop"],
                "non_goals":[],
                "requirements":[{"id":"design-req","text":"legacy design requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "architecture":{
                "name":"architecture",
                "content_path":"architecture.md",
                "fields":{"summary":"legacy architecture"},
                "ambiguity_score":0.2,
                "last_evaluated":null,
                "defined":true,
                "kind":"Technology",
                "status":"Defined",
                "goals":["legacy tech"],
                "non_goals":[],
                "requirements":[{"id":"architecture-req","text":"legacy architecture requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "art_style":null,
            "audio":null,
            "narrative":null,
            "levels":null,
            "ui_ux":null,
            "custom":{}
        },
        "schell_evaluation":{
            "phase1_experience":{"name":"Experience Lens","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase2_tetrad":{
                "mechanics":{"status":"Missing","description":null,"score":0.0},
                "story":{"status":"Missing","description":null,"score":0.0},
                "aesthetics":{"status":"Missing","description":null,"score":0.0},
                "technology":{"status":"Missing","description":null,"score":0.0},
                "harmony_score":0.0
            },
            "phase3_core_loop":{"name":"Core Loop Stress Test","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase4_motivation":{"name":"Player Motivation","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase5_assessment":{"status":"Missing","viability_score":0.0,"strengths":[],"risks":[],"recommendations":[],"summary":null}
        },
        "overall_ambiguity":1.0
    }"#;
    std::fs::write(temp.path().join(".lux/specs/spec.json"), legacy_spec)
        .expect("legacy spec should be written");

    let spec = lux_load(temp.path()).expect("legacy spec should migrate");

    let gdd = spec.domains.gdd.expect("gdd should be migrated");
    assert_eq!(gdd.name, "gdd");
    assert_eq!(gdd.content_path, "gdd.md");
    assert_eq!(gdd.fields.get("summary"), Some(&json!("legacy design")));
    assert!(gdd
        .requirements
        .iter()
        .any(|requirement| requirement.id == "design-req"));

    let technical_architecture = spec
        .domains
        .technical_architecture
        .expect("technical architecture should be migrated");
    assert_eq!(technical_architecture.name, "technical-architecture");
    assert_eq!(
        technical_architecture.content_path,
        "technical-architecture.md"
    );
    assert_eq!(
        technical_architecture.fields.get("summary"),
        Some(&json!("legacy architecture"))
    );
    assert!(technical_architecture
        .requirements
        .iter()
        .any(|requirement| requirement.id == "architecture-req"));

    let migrated_json = std::fs::read_to_string(temp.path().join(".lux/specs/spec.json"))
        .expect("migrated spec should be written back");
    assert!(migrated_json.contains("\"gdd\""));
    assert!(migrated_json.contains("\"technical_architecture\""));
    assert!(!migrated_json.contains("\"design\""));
    assert!(!migrated_json.contains("\"architecture\""));
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
    assert!(lux_path.join("specs/spec.json").is_file());
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
        "gdd",
        "mechanics",
        "controls",
        "camera",
        "art-style",
        "audio",
        "narrative",
        "levels",
        "technical-architecture",
        "engine",
        "testing",
        "build-release",
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
fn test_lux_init_creates_canonical_specs_contract() {
    let temp = TestTempDir::new();
    let lux_path = lux_init(temp.path()).expect("lux init should succeed");
    let specs_root = lux_path.join("specs");

    assert!(specs_root.is_dir());
    assert!(specs_root.join("gdd.md").is_file());
    assert!(specs_root.join("spec.json").is_file());
    assert!(specs_root.join("domains").is_dir());
    for domain in [
        "mechanics",
        "controls",
        "camera",
        "art-style",
        "audio",
        "narrative",
        "levels",
        "technical-architecture",
        "engine",
        "testing",
        "build-release",
        "ui-ux",
    ] {
        assert!(
            specs_root
                .join("domains")
                .join(format!("{domain}.md"))
                .is_file(),
            "{domain}.md should exist"
        );
    }
    assert!(specs_root.join("decisions.jsonl").is_file());
    assert!(specs_root.join("preferences.json").is_file());
    assert!(specs_root.join("migration.json").is_file());
}

#[test]
fn test_lux_init_creates_canonical_game_domain_set() {
    let temp = TestTempDir::new();
    let lux_path = lux_init(temp.path()).expect("lux init should succeed");
    let domains_root = lux_path.join("specs/domains");

    for domain in [
        "gdd",
        "mechanics",
        "controls",
        "camera",
        "levels",
        "art-style",
        "audio",
        "narrative",
        "ui-ux",
        "technical-architecture",
        "engine",
        "testing",
        "build-release",
    ] {
        assert!(
            domains_root.join(format!("{domain}.md")).is_file(),
            "expected canonical game domain file for {domain}"
        );
    }

    let mut discovered_domains = std::fs::read_dir(&domains_root)
        .expect("domains root should be readable")
        .map(|entry| {
            entry
                .expect("domain entry should be readable")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    discovered_domains.sort();

    assert_eq!(
        discovered_domains,
        vec![
            "art-style.md",
            "audio.md",
            "build-release.md",
            "camera.md",
            "controls.md",
            "engine.md",
            "gdd.md",
            "levels.md",
            "mechanics.md",
            "narrative.md",
            "technical-architecture.md",
            "testing.md",
            "ui-ux.md",
        ]
    );
}

#[test]
fn game_domain_schema_initializes_canonical_game_domain_set() {
    let temp = TestTempDir::new();
    let lux_path = lux_init(temp.path()).expect("lux init should succeed");
    let domains_root = lux_path.join("specs/domains");

    for domain in [
        "gdd",
        "mechanics",
        "controls",
        "camera",
        "levels",
        "art-style",
        "audio",
        "narrative",
        "ui-ux",
        "technical-architecture",
        "engine",
        "testing",
        "build-release",
    ] {
        assert!(
            domains_root.join(format!("{domain}.md")).is_file(),
            "expected canonical game domain file for {domain}"
        );
    }
}

#[test]
fn test_lux_init_preserves_multiple_legacy_domains() {
    let temp = TestTempDir::new();
    let legacy_domains = temp.path().join(".lux/domains");
    std::fs::create_dir_all(&legacy_domains).expect("legacy domains directory should exist");

    let fixtures = [
        (
            "boss-fights.md",
            "boss-fights.md",
            "# Boss Fights\n\nLegacy boss fight notes.\n",
        ),
        (
            "economy.md",
            "economy.md",
            "# Economy\n\nLegacy economy notes.\n",
        ),
        (
            "navigation.md",
            "navigation.md",
            "# Navigation\n\nLegacy navigation notes with YAML-like: value.\n",
        ),
        (
            "packages.md",
            "engine.md",
            "# Packages\n\nLegacy packages notes.\n",
        ),
    ];

    for (name, _, content) in &fixtures {
        std::fs::write(legacy_domains.join(name), content)
            .expect("legacy domain fixture should be written");
    }

    lux_init(temp.path()).expect("lux init should succeed");

    for (name, migrated_name, content) in fixtures {
        let migrated =
            std::fs::read_to_string(temp.path().join(".lux/specs/domains").join(migrated_name))
                .expect("migrated domain should exist");
        assert_eq!(migrated, content, "{name} should preserve exact content");
    }
}

#[test]
fn test_lux_load_prefers_canonical_specs_root_when_both_exist() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let mut legacy_spec = lux_load(temp.path()).expect("spec should load");
    legacy_spec.project_name = "LegacySpec".to_string();
    lux_save(temp.path(), &legacy_spec).expect("legacy save should succeed");

    let canonical_spec_path = temp.path().join(".lux/specs/spec.json");
    let canonical_spec = SpecProject {
        project_name: "CanonicalSpec".to_string(),
        ..legacy_spec.clone()
    };
    std::fs::write(
        &canonical_spec_path,
        serde_json::to_string_pretty(&canonical_spec).expect("serialize canonical spec"),
    )
    .expect("write canonical spec");

    let loaded = lux_load(temp.path()).expect("canonical spec should load");
    assert_eq!(loaded.project_name, "CanonicalSpec");
}

#[test]
fn test_lux_init_is_idempotent() {
    let temp = TestTempDir::new();

    lux_init(temp.path()).expect("first init should succeed");

    assert_eq!(lux_init(temp.path()).unwrap(), temp.path().join(".lux"));
}

#[test]
fn test_legacy_domain_names_migrate_to_canonical_game_domains() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    lux_update_domain_field(
        temp.path(),
        "design",
        "core_loop",
        json!("route, collect, refine"),
    )
    .expect("design should migrate into gdd");
    lux_update_domain_field(
        temp.path(),
        "architecture",
        "engine_contract",
        json!("server-client bridge"),
    )
    .expect("architecture should migrate into technical architecture");

    let loaded = lux_load(temp.path()).expect("updated spec should load");

    assert_eq!(
        loaded
            .domains
            .gdd
            .as_ref()
            .and_then(|domain| domain.fields.get("core_loop")),
        Some(&json!("route, collect, refine"))
    );
    assert!(loaded
        .domains
        .technical_architecture
        .as_ref()
        .is_some_and(
            |domain| domain.fields.get("engine_contract") == Some(&json!("server-client bridge"))
        ));
    assert!(loaded.domains.custom.get("design").is_none());
    assert!(loaded.domains.custom.get("architecture").is_none());
}

#[test]
fn test_legacy_game_domain_alias_migration_preserves_domain_content() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let legacy_spec = r#"{
        "version":"1.0.0",
        "schema_version":"2.0",
        "project_id":"legacy-content-game",
        "project_name":"Legacy Content Game",
        "created_at":"2026-05-11T00:00:00Z",
        "updated_at":"2026-05-11T00:00:00Z",
        "source":"lux-init",
        "status":"Draft",
        "domains":{
            "design":{
                "name":"design",
                "content_path":"design.md",
                "fields":{"summary":"legacy design"},
                "ambiguity_score":0.4,
                "last_evaluated":null,
                "defined":true,
                "kind":"Experience",
                "status":"Defined",
                "goals":["legacy loop"],
                "non_goals":[],
                "requirements":[{"id":"design-req","text":"legacy design requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "architecture":{
                "name":"architecture",
                "content_path":"architecture.md",
                "fields":{"summary":"legacy architecture"},
                "ambiguity_score":0.2,
                "last_evaluated":null,
                "defined":true,
                "kind":"Technology",
                "status":"Defined",
                "goals":["legacy tech"],
                "non_goals":[],
                "requirements":[{"id":"architecture-req","text":"legacy architecture requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "art_style":{
                "name":"art_style",
                "content_path":"art-style.md",
                "fields":{"summary":"legacy art"},
                "ambiguity_score":0.3,
                "last_evaluated":null,
                "defined":true,
                "kind":"Content",
                "status":"Defined",
                "goals":["legacy visuals"],
                "non_goals":[],
                "requirements":[{"id":"art-req","text":"legacy art requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "ui_ux":{
                "name":"ui_ux",
                "content_path":"ui-ux.md",
                "fields":{"summary":"legacy ui"},
                "ambiguity_score":0.1,
                "last_evaluated":null,
                "defined":true,
                "kind":"Experience",
                "status":"Defined",
                "goals":["legacy ui"],
                "non_goals":[],
                "requirements":[{"id":"ui-req","text":"legacy ui requirement","priority":"Medium","status":"Proposed","acceptance_criteria":[],"rationale":null,"source_question":null,"depends_on":[],"conflicts_with":[],"confidence":null}],
                "dependencies":[],
                "decisions":[],
                "open_questions":[],
                "glossary_terms":[],
                "tests":[],
                "examples":[]
            },
            "audio":null,
            "narrative":null,
            "levels":null,
            "technical_architecture":null,
            "engine":null,
            "testing":null,
            "build_release":null,
            "custom":{}
        },
        "schell_evaluation":{
            "phase1_experience":{"name":"Experience Lens","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase2_tetrad":{
                "mechanics":{"status":"Missing","description":null,"score":0.0},
                "story":{"status":"Missing","description":null,"score":0.0},
                "aesthetics":{"status":"Missing","description":null,"score":0.0},
                "technology":{"status":"Missing","description":null,"score":0.0},
                "harmony_score":0.0
            },
            "phase3_core_loop":{"name":"Core Loop Stress Test","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase4_motivation":{"name":"Player Motivation","status":"Missing","summary":null,"score":0.0,"questions":[]},
            "phase5_assessment":{"status":"Missing","viability_score":0.0,"strengths":[],"risks":[],"recommendations":[],"summary":null}
        },
        "overall_ambiguity":1.0
    }"#;
    std::fs::write(temp.path().join(".lux/specs/spec.json"), legacy_spec)
        .expect("legacy spec should be written");

    let spec = lux_load(temp.path()).expect("legacy spec should migrate");

    let gdd = spec.domains.gdd.expect("gdd should be migrated");
    assert_eq!(gdd.fields.get("summary"), Some(&json!("legacy design")));
    assert!(gdd
        .requirements
        .iter()
        .any(|requirement| requirement.id == "design-req"));

    let technical_architecture = spec
        .domains
        .technical_architecture
        .expect("technical architecture should be migrated");
    assert_eq!(
        technical_architecture.fields.get("summary"),
        Some(&json!("legacy architecture"))
    );
    assert!(technical_architecture
        .requirements
        .iter()
        .any(|requirement| requirement.id == "architecture-req"));

    let art_style = spec
        .domains
        .art_style
        .expect("art style should remain canonical");
    assert_eq!(art_style.fields.get("summary"), Some(&json!("legacy art")));
    assert!(art_style
        .requirements
        .iter()
        .any(|requirement| requirement.id == "art-req"));

    let ui_ux = spec.domains.ui_ux.expect("ui ux should remain canonical");
    assert_eq!(ui_ux.fields.get("summary"), Some(&json!("legacy ui")));
    assert!(ui_ux
        .requirements
        .iter()
        .any(|requirement| requirement.id == "ui-req"));
}

#[test]
fn test_packages_and_testing_promotion_updates_spec_fields() {
    let question_packages = TargetedQuestion {
        domain: "spec".to_string(),
        phase: "packages.required".to_string(),
        question: "Which packages are required?".to_string(),
        priority: 1.0,
        default_value: None,
        options: vec![],
    };
    let question_testing = TargetedQuestion {
        domain: "spec".to_string(),
        phase: "testing.strategy".to_string(),
        question: "What is the testing strategy?".to_string(),
        priority: 1.0,
        default_value: None,
        options: vec![],
    };

    let mut spec = SpecProject::default();
    answer_direct(
        &mut spec,
        &question_packages,
        "com.unity.textmeshpro, com.unity.timeline",
    )
    .expect("packages.required answer should be applied");
    answer_direct(
        &mut spec,
        &question_testing,
        "unit tests plus playmode smoke",
    )
    .expect("testing.strategy answer should be applied");

    let packages = spec.packages.expect("packages should be promoted");
    assert_eq!(
        packages
            .required
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>(),
        vec!["com.unity.textmeshpro", "com.unity.timeline"]
    );

    let testing = spec.testing.expect("testing should be promoted");
    assert_eq!(
        testing.strategy.as_deref(),
        Some("unit tests plus playmode smoke")
    );
}

#[test]
fn test_lux_load_saves_roundtrip() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let mut spec = lux_load(temp.path()).expect("spec should load");
    spec.project_name = "Roundtrip".to_string();
    spec.domains.mechanics = Some(DomainSpec::new("mechanics", "mechanics.md", 0.25));

    lux_save(temp.path(), &spec).expect("spec should save");
    let loaded = lux_load(temp.path()).expect("saved spec should load");

    assert_eq!(loaded.project_name, "Roundtrip");
    assert_eq!(loaded.project_id, spec.project_id);
    assert_eq!(loaded.domains.mechanics.unwrap().ambiguity_score, 0.25);
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

    lux_save_domain(temp.path(), "mechanics", "# Updated Mechanics\n").expect("domain should save");
    let content = lux_load_domain(temp.path(), "mechanics").expect("domain should load");

    assert_eq!(content, "# Updated Mechanics\n");
    assert_eq!(
        std::fs::read_to_string(temp.path().join(".lux/specs/domains/mechanics.md"))
            .expect("canonical domain should be written"),
        "# Updated Mechanics\n"
    );
}

#[test]
fn test_lux_load_domain_prefers_canonical_specs_path() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    std::fs::write(
        temp.path().join(".lux/specs/domains/mechanics.md"),
        "# Canonical Mechanics\n",
    )
    .expect("canonical domain should be writable");
    std::fs::write(
        temp.path().join(".lux/domains/mechanics.md"),
        "# Stale Legacy Mechanics\n",
    )
    .expect("legacy domain should be writable");

    let content = lux_load_domain(temp.path(), "mechanics").expect("domain should load");

    assert_eq!(content, "# Canonical Mechanics\n");
}

#[test]
fn test_lux_update_domain_field() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should succeed");

    let spec =
        lux_update_domain_field(temp.path(), "mechanics", "core_loop", json!("jump collect"))
            .expect("domain field should update");
    let mechanics = spec
        .domains
        .mechanics
        .expect("mechanics domain should exist");

    assert!(mechanics.defined);
    assert_eq!(
        mechanics.fields.get("core_loop"),
        Some(&json!("jump collect"))
    );

    let loaded = lux_load(temp.path()).expect("updated spec should load");
    assert_eq!(
        loaded
            .domains
            .mechanics
            .expect("mechanics domain should persist")
            .fields
            .get("core_loop"),
        Some(&json!("jump collect"))
    );
}

#[test]
fn test_render_markdown_template() {
    let mut vars = HashMap::new();
    vars.insert("domain".to_string(), "Design".to_string());

    let rendered = render_markdown_template("mechanics", &vars).expect("template should render");

    assert!(rendered.contains("# Mechanics Spec"));
    assert!(!rendered.contains("{{domain}}"));
}
