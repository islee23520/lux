use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lux::lux_ambiguity::calculate_ambiguity;
use lux::lux_spec::{
    lux_init, lux_load, lux_save, DomainSpec, GlossarySpec, PackageEntry, PackagesSpec,
    SpecProject, TargetsSpec, TestingSpec, UnitySpec,
};
use serde_json::json;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new() -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lux-integration-test-{}-{count}",
            std::process::id()
        ));
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
fn spec_round_trip_with_new_fields() {
    let mut min_sdk = HashMap::new();
    min_sdk.insert("android".to_string(), "23".to_string());
    min_sdk.insert("ios".to_string(), "15.0".to_string());

    let mut spec = SpecProject::default();
    spec.project_id = "lux-round-trip".to_string();
    spec.project_name = "Lux Round Trip".to_string();
    spec.unity = Some(UnitySpec {
        required_version: Some("6000.0.25f1".to_string()),
        detected_version: Some("6000.0.30f1".to_string()),
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
        ..UnitySpec::default()
    });
    spec.targets = Some(TargetsSpec {
        platforms: vec!["android".to_string(), "ios".to_string()],
        min_sdk,
        test_platform: Some("android".to_string()),
        target_platforms: Vec::new(),
    });
    spec.packages = Some(PackagesSpec {
        required: vec![PackageEntry {
            name: "com.unity.inputsystem".to_string(),
            reason: Some("player controls".to_string()),
            version: Some("1.7.0".to_string()),
            required_by_domain: Vec::new(),
        }],
        forbidden: vec![PackageEntry {
            name: "com.unity.legacy".to_string(),
            reason: Some("avoid legacy APIs".to_string()),
            version: None,
            required_by_domain: Vec::new(),
        }],
        detected: vec![PackageEntry {
            name: "com.unity.textmeshpro".to_string(),
            reason: None,
            version: Some("3.0.9".to_string()),
            required_by_domain: Vec::new(),
        }],
        recommended: Vec::new(),
    });
    spec.testing = Some(TestingSpec {
        framework: Some("unity-test-runner".to_string()),
        strategy: Some("edit-mode and play-mode smoke tests".to_string()),
        coverage: true,
    });
    spec.glossary = Some(GlossarySpec {
        path: "glossary.md".to_string(),
        last_updated: Some("2026-05-11T00:00:00Z".to_string()),
        term_count: 12,
    });

    let serialized = serde_json::to_string_pretty(&spec).expect("spec should serialize");
    let parsed: SpecProject = serde_json::from_str(&serialized).expect("spec should deserialize");

    assert_eq!(parsed, spec);
    assert_eq!(
        parsed
            .targets
            .as_ref()
            .expect("targets should be present")
            .min_sdk["android"],
        "23"
    );
    assert!(
        parsed
            .testing
            .as_ref()
            .expect("testing should be present")
            .coverage
    );
}

#[test]
fn spec_backward_compatibility() {
    let old_json = json!({
        "version": "1.0.0",
        "project_id": "legacy-project",
        "project_name": "Legacy Project",
        "created_at": "2026-05-11T00:00:00Z",
        "updated_at": "2026-05-11T00:00:00Z",
        "source": "lux-init",
        "status": "Draft",
        "domains": {
            "design": null,
            "architecture": null,
            "art_style": null,
            "audio": null,
            "narrative": null,
            "levels": null,
            "ui_ux": null,
            "custom": {}
        },
        "schell_evaluation": {
            "phase1_experience": {"name": "Experience Lens", "status": "Missing", "summary": null, "score": 0.0, "questions": []},
            "phase2_tetrad": {
                "mechanics": {"status": "Missing", "description": null, "score": 0.0},
                "story": {"status": "Missing", "description": null, "score": 0.0},
                "aesthetics": {"status": "Missing", "description": null, "score": 0.0},
                "technology": {"status": "Missing", "description": null, "score": 0.0},
                "harmony_score": 0.0
            },
            "phase3_core_loop": {"name": "Core Loop Stress Test", "status": "Missing", "summary": null, "score": 0.0, "questions": []},
            "phase4_motivation": {"name": "Player Motivation", "status": "Missing", "summary": null, "score": 0.0, "questions": []},
            "phase5_assessment": {"status": "Missing", "viability_score": 0.0, "strengths": [], "risks": [], "recommendations": [], "summary": null}
        },
        "overall_ambiguity": 1.0
    });

    let spec: SpecProject = serde_json::from_value(old_json).expect("legacy spec should load");

    assert!(spec.unity.is_none());
    assert!(spec.targets.is_none());
    assert!(spec.packages.is_none());
    assert!(spec.testing.is_none());
    assert!(spec.glossary.is_none());
}

#[test]
fn spec_evaluation_with_target_ambiguity() {
    let mut spec = SpecProject::default();
    spec.domains.design = Some(DomainSpec::new("design", "design.md", 0.08));
    spec.domains.architecture = Some(DomainSpec::new("architecture", "architecture.md", 0.08));
    spec.domains.art_style = Some(DomainSpec::new("art_style", "art_style.md", 0.08));
    spec.domains.audio = Some(DomainSpec::new("audio", "audio.md", 0.08));
    spec.domains.narrative = Some(DomainSpec::new("narrative", "narrative.md", 0.08));
    spec.domains.levels = Some(DomainSpec::new("levels", "levels.md", 0.08));
    spec.domains.ui_ux = Some(DomainSpec::new("ui_ux", "ui_ux.md", 0.08));
    spec.unity = Some(UnitySpec {
        required_version: Some("6000.0".to_string()),
        detected_version: Some("6000.0".to_string()),
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
        ..UnitySpec::default()
    });
    spec.targets = Some(TargetsSpec {
        platforms: vec!["macos".to_string()],
        min_sdk: HashMap::new(),
        test_platform: Some("macos".to_string()),
        target_platforms: Vec::new(),
    });
    spec.packages = Some(PackagesSpec {
        required: vec![PackageEntry {
            name: "com.unity.inputsystem".to_string(),
            reason: Some("input".to_string()),
            version: None,
            required_by_domain: Vec::new(),
        }],
        recommended: Vec::new(),
        forbidden: Vec::new(),
        detected: Vec::new(),
    });
    spec.testing = Some(TestingSpec {
        framework: Some("unity-test-runner".to_string()),
        strategy: Some("smoke".to_string()),
        coverage: false,
    });
    spec.glossary = Some(GlossarySpec::default());

    let evaluation = calculate_ambiguity(&spec);
    let target_ambiguity = evaluation.overall_score;

    let strict_target_ambiguity = 0.02;
    let lenient_target_ambiguity = 0.90;

    assert!(
        target_ambiguity > strict_target_ambiguity,
        "ambiguity was {target_ambiguity}"
    );
    assert!(
        target_ambiguity <= lenient_target_ambiguity,
        "ambiguity was {target_ambiguity}"
    );
}

#[test]
fn glossary_spec_defaults() {
    let spec = GlossarySpec::default();

    assert_eq!(spec.path, "glossary.md");
}

#[test]
fn spec_lifecycle_init_evaluate_continue_with_new_fields() {
    let temp = TestTempDir::new();
    lux_init(temp.path()).expect("lux init should create spec directory");
    let mut spec = lux_load(temp.path()).expect("initialized spec should load");

    spec.unity = Some(UnitySpec {
        required_version: Some("6000.0".to_string()),
        detected_version: Some("6000.0".to_string()),
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
        ..UnitySpec::default()
    });
    spec.targets = Some(TargetsSpec {
        platforms: vec!["macos".to_string()],
        min_sdk: HashMap::new(),
        test_platform: Some("macos".to_string()),
        target_platforms: Vec::new(),
    });
    spec.packages = Some(PackagesSpec {
        required: vec![PackageEntry {
            name: "com.unity.inputsystem".to_string(),
            reason: Some("input".to_string()),
            version: None,
            required_by_domain: Vec::new(),
        }],
        recommended: Vec::new(),
        forbidden: Vec::new(),
        detected: Vec::new(),
    });
    spec.testing = Some(TestingSpec {
        framework: Some("unity-test-runner".to_string()),
        strategy: Some("play-mode smoke".to_string()),
        coverage: true,
    });
    spec.glossary = Some(GlossarySpec::default());
    lux_save(temp.path(), &spec).expect("updated spec should save");

    let reloaded = lux_load(temp.path()).expect("updated spec should reload");
    let evaluation = calculate_ambiguity(&reloaded);
    let should_continue = evaluation.overall_score < 0.98;

    assert_eq!(reloaded.unity, spec.unity);
    assert_eq!(reloaded.targets, spec.targets);
    assert_eq!(reloaded.packages, spec.packages);
    assert_eq!(reloaded.testing, spec.testing);
    assert_eq!(reloaded.glossary, spec.glossary);
    assert!(should_continue);
}
