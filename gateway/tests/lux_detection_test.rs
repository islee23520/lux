use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use lux::lux_ambiguity::TargetedQuestion;
use lux::lux_spec;
use lux::lux_spec::{
    answer_direct, apply_detection_to_spec, lux_init, lux_init_interactive, lux_load, GlossarySpec,
    LuxInitInteractiveOptions, PackagesSpec, SpecProject, SpecQuestionIo, TestingSpec, UnitySpec,
};
use lux::project;
use lux::project::{detect_unity_project, DetectedPackage, UnityProjectDetection};
use lux::project_godot;
use lux_project::{recommended_capability_blockers, CapabilityStatus, EngineKind};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new() -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("lux-detection-test-{}-{count}", std::process::id()));
        std::fs::create_dir(&path).expect("temp directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent directory should be created");
    }
    std::fs::write(path, contents).expect("file should be written");
}

fn create_unity_project(temp: &TestTempDir) {
    write_file(
        temp.path(),
        "ProjectSettings/ProjectVersion.txt",
        "m_EditorVersion: 6000.0.0f1\n",
    );
}

struct FakeSpecQuestionIo {
    answers: Vec<String>,
    call_count: usize,
}

impl FakeSpecQuestionIo {
    fn new(answers: Vec<&str>) -> Self {
        Self {
            answers: answers.iter().map(|answer| answer.to_string()).collect(),
            call_count: 0,
        }
    }
}

impl SpecQuestionIo for FakeSpecQuestionIo {
    fn present_detection(&mut self, _detection: Option<&UnityProjectDetection>) -> Result<()> {
        Ok(())
    }

    fn ask(
        &mut self,
        _question: &TargetedQuestion,
        _iteration: u32,
        _max_iterations: u32,
    ) -> Result<Option<String>> {
        if self.call_count < self.answers.len() {
            let answer = self.answers[self.call_count].clone();
            self.call_count += 1;
            Ok(Some(answer))
        } else {
            Ok(None)
        }
    }

    fn report_progress(&mut self, _ambiguity_score: f64, _target_ambiguity: f64) -> Result<()> {
        Ok(())
    }
}

fn spec_question(phase: &str) -> TargetedQuestion {
    TargetedQuestion {
        domain: "spec".to_string(),
        phase: phase.to_string(),
        question: "test question".to_string(),
        options: Vec::new(),
        default_value: None,
        priority: 0.5,
    }
}

#[test]
fn answer_direct_rejects_empty_answer() {
    let mut spec = SpecProject::default();
    let error = answer_direct(&mut spec, &spec_question("unity.required_version"), "  ")
        .expect_err("empty answers should be rejected");

    assert!(error.to_string().contains("Answer cannot be empty"));
}

#[test]
fn lux_init_interactive_non_interactive() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    write_file(
        temp.path(),
        "ProjectSettings/ProjectSettings.asset",
        "productName: InteractiveGame\n",
    );
    write_file(
        temp.path(),
        "Packages/manifest.json",
        r#"{
  "dependencies": {
    "com.unity.render-pipelines.universal": "14.0.11",
    "com.unity.test-framework": "1.3.7"
  }
}"#,
    );
    let mut io = FakeSpecQuestionIo::new(vec!["6000.0.0f1"]);

    let lux_path = lux_init_interactive(
        temp.path(),
        &mut io,
        LuxInitInteractiveOptions {
            interactive: false,
            ..LuxInitInteractiveOptions::default()
        },
    )
    .expect("interactive init should succeed");
    let spec = lux_load(temp.path()).expect("spec should load");

    assert_eq!(lux_path, temp.path().join(".lux"));
    assert_eq!(io.call_count, 0);
    assert_eq!(
        spec.project_name,
        temp.path()
            .file_name()
            .expect("temp dir should have a name")
            .to_string_lossy()
    );
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.detected_version.as_deref()),
        Some("6000.0.0f1")
    );
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.render_pipeline.as_deref()),
        Some("urp")
    );
    assert_eq!(
        spec.testing
            .as_ref()
            .and_then(|testing| testing.framework.as_deref()),
        Some("Unity Test Framework")
    );
    assert!(spec
        .packages
        .as_ref()
        .is_some_and(|packages| packages.required.is_empty()));
    assert!(
        !temp.path().join(".lux/engines/capabilities.json").exists(),
        "non-interactive init should not persist engine capability snapshots"
    );
}

#[test]
fn lux_init_interactive_with_answers() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    let mut io = FakeSpecQuestionIo::new(vec![
        "6000.0.0f1",
        "android, ios",
        "EditMode smoke tests",
        "com.unity.addressables, com.unity.inputsystem",
    ]);

    lux_init_interactive(
        temp.path(),
        &mut io,
        LuxInitInteractiveOptions {
            interactive: true,
            target_ambiguity: 0.0,
            max_iterations: 4,
        },
    )
    .expect("interactive init should succeed");
    let spec = lux_load(temp.path()).expect("spec should load");

    assert_eq!(io.call_count, 4);
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.required_version.as_deref()),
        Some("6000.0.0f1")
    );
    assert_eq!(
        spec.targets
            .as_ref()
            .map(|targets| targets.platforms.clone()),
        Some(vec!["android".to_string(), "ios".to_string()])
    );
    assert_eq!(
        spec.testing
            .as_ref()
            .and_then(|testing| testing.strategy.as_deref()),
        Some("EditMode smoke tests")
    );
    assert_eq!(
        spec.packages.as_ref().map(|packages| packages
            .required
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>()),
        Some(vec!["com.unity.addressables", "com.unity.inputsystem"])
    );
}

#[test]
fn answer_direct_sets_unity_required_version() {
    let mut spec = SpecProject::default();

    answer_direct(
        &mut spec,
        &spec_question("unity.required_version"),
        "6000.0.0f1",
    )
    .expect("answer should apply");

    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.required_version.as_deref()),
        Some("6000.0.0f1")
    );
}

#[test]
fn answer_direct_parses_target_platforms() {
    let mut spec = SpecProject::default();

    answer_direct(
        &mut spec,
        &spec_question("targets.platforms"),
        "Android, IOS; android\nWebGL",
    )
    .expect("answer should apply");

    assert_eq!(
        spec.targets
            .as_ref()
            .map(|targets| targets.platforms.clone()),
        Some(vec![
            "android".to_string(),
            "ios".to_string(),
            "webgl".to_string(),
        ])
    );
}

#[test]
fn answer_direct_parses_required_packages() {
    let mut spec = SpecProject {
        packages: Some(PackagesSpec {
            required: vec![lux_spec::PackageEntry {
                name: "com.unity.inputsystem".to_string(),
                reason: Some("existing".to_string()),
                version: Some("1.7.0".to_string()),
                required_by_domain: Vec::new(),
            }],
            recommended: vec![],
            forbidden: vec![],
            detected: vec![],
        }),
        ..SpecProject::default()
    };

    answer_direct(
        &mut spec,
        &spec_question("packages.required"),
        "com.unity.textmeshpro, com.unity.inputsystem; com.unity.addressables",
    )
    .expect("answer should apply");

    let required = &spec
        .packages
        .as_ref()
        .expect("packages should exist")
        .required;
    assert_eq!(
        required
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "com.unity.inputsystem",
            "com.unity.textmeshpro",
            "com.unity.addressables",
        ]
    );
    assert_eq!(required[1].reason, None);
    assert_eq!(required[1].version, None);
}

#[test]
fn answer_direct_sets_testing_strategy() {
    let mut spec = SpecProject::default();

    answer_direct(
        &mut spec,
        &spec_question("testing.strategy"),
        "EditMode smoke tests plus PlayMode coverage gates",
    )
    .expect("answer should apply");

    assert_eq!(
        spec.testing
            .as_ref()
            .and_then(|testing| testing.strategy.as_deref()),
        Some("EditMode smoke tests plus PlayMode coverage gates")
    );
}

#[test]
fn answer_direct_skips_unsupported_domain() {
    let mut spec = SpecProject::default();
    let question = TargetedQuestion {
        domain: "design".to_string(),
        phase: "unity.required_version".to_string(),
        question: "test question".to_string(),
        options: Vec::new(),
        default_value: None,
        priority: 0.5,
    };

    answer_direct(&mut spec, &question, "6000.0.0f1").expect("unsupported domains should skip");

    assert!(spec.unity.is_none());
}

#[test]
fn detect_unity_project_non_unity_returns_none() {
    let temp = TestTempDir::new();

    assert!(detect_unity_project(temp.path())
        .expect("detection should succeed")
        .is_none());
}

#[test]
fn lux_init_does_not_write_engine_capabilities_for_plain_project() {
    let temp = TestTempDir::new();

    lux_init(temp.path()).expect("lux init should succeed");

    assert!(!temp.path().join(".lux/engines/capabilities.json").exists());
}

#[test]
fn godot_detection_reports_blockers_without_build_success() {
    let temp = TestTempDir::new();
    write_file(temp.path(), "project.godot", "config_version=5\n");

    let detection = project_godot::detect_godot_project(temp.path())
        .expect("Godot 4 project should be detected");
    let blockers = recommended_capability_blockers(Some(EngineKind::Godot));

    assert_eq!(detection.godot_version.as_deref(), Some("4.x"));
    assert!(blockers.iter().any(|blocker| {
        blocker.capability == "build"
            && blocker.status == CapabilityStatus::Unsupported
            && blocker.evidence_path.ends_with("godot-build.json")
            && blocker
                .recommended_next_supported_action
                .contains("lux godot status")
    }));
    assert!(!blockers
        .iter()
        .any(|blocker| blocker.status == CapabilityStatus::Verified));
}

#[test]
fn lux_godot_status_persists_engine_capabilities_json() {
    let temp = TestTempDir::new();
    let project_root = temp.path().join("GodotProject");
    std::fs::create_dir_all(&project_root).expect("create project root");
    write_file(&project_root, "project.godot", "config_version=5\n");
    write_file(
        &project_root,
        "package.json",
        r#"{
  "dependencies": {
    "three": "^0.179.0"
  }
}"#,
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "godot",
            "status",
            "--project-path",
            project_root.to_str().expect("project path should be UTF-8"),
        ])
        .output()
        .expect("run lux godot status");

    assert!(output.status.success());

    let capabilities_path = project_root.join(".lux/engines/capabilities.json");
    assert!(
        capabilities_path.is_file(),
        "expected persisted engine capability file at {}",
        capabilities_path.display()
    );

    let payload: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&capabilities_path).expect("read persisted capabilities json"),
    )
    .expect("parse persisted capabilities json");
    assert_eq!(payload["schema_version"], "1");
    assert_eq!(payload["engine"], "godot");
    assert_eq!(payload["status"], "limited");
    assert_eq!(
        payload["reason"],
        "Godot project markers found in project.godot"
    );
    assert_eq!(payload["unity"]["status"], "unsupported");
    assert_eq!(payload["godot"]["status"], "limited");
    assert_eq!(payload["three_js"]["status"], "limited");
    assert_eq!(
        payload["unity"]["reason"],
        "Unity project markers not detected in project root."
    );
    assert!(payload["godot"]["blocker_reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("GoPeak-backed")));
    assert!(payload["three_js"]["blocker_reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("verified LUX harness")));
}

#[test]
fn lux_init_persists_engine_capability_inventory() {
    let temp = TestTempDir::new();
    std::fs::create_dir_all(temp.path().join("ProjectSettings"))
        .expect("unity marker dir should be created");
    std::fs::write(
        temp.path().join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.0.0f1\n",
    )
    .expect("unity marker should be written");

    let catalog = lux_project::persist_engine_capabilities(temp.path(), EngineKind::Unity)
        .expect("engine capability detection should succeed");

    assert!(temp.path().join(".lux/engines/capabilities.json").is_file());
    assert_eq!(catalog.engine, EngineKind::Unity);
    assert_eq!(
        catalog.status,
        lux_project::EngineCapabilityStatus::Detected
    );
    assert!(catalog.reason.contains("Unity"));
    assert_eq!(
        catalog.unity.status,
        lux_project::EngineCapabilityStatus::Detected
    );
    assert_eq!(
        catalog.godot.status,
        lux_project::EngineCapabilityStatus::Unsupported
    );
    assert_eq!(
        catalog.three_js.status,
        lux_project::EngineCapabilityStatus::Unsupported
    );
    assert!(catalog.godot.blocker_reason.is_some());
    assert!(catalog.three_js.blocker_reason.is_some());
}

#[test]
fn lux_godot_status_writes_three_engine_capability_contract_for_empty_and_unity_like_projects() {
    let empty_temp = TestTempDir::new();
    let empty_project_root = empty_temp.path().join("EmptyProject");
    std::fs::create_dir_all(&empty_project_root).expect("create empty project root");

    let empty_output = std::process::Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "godot",
            "status",
            "--project-path",
            empty_project_root
                .to_str()
                .expect("project path should be UTF-8"),
        ])
        .output()
        .expect("run lux godot status for empty project");

    assert!(empty_output.status.success());

    let empty_capabilities_path = empty_project_root.join(".lux/engines/capabilities.json");
    assert!(
        empty_capabilities_path.is_file(),
        "expected persisted engine capability file at {}",
        empty_capabilities_path.display()
    );
    let empty_payload: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&empty_capabilities_path)
            .expect("read empty-project capabilities json"),
    )
    .expect("parse empty-project capabilities json");
    assert_eq!(
        empty_payload["engines"]
            .as_array()
            .map(|entries| entries.len()),
        Some(3)
    );
    assert_eq!(empty_payload["engines"][0]["engine"], "unity");
    assert_eq!(empty_payload["engines"][1]["engine"], "godot");
    assert_eq!(empty_payload["engines"][2]["engine"], "three_js");
    assert_eq!(empty_payload["engines"][0]["status"], "unsupported");
    assert_eq!(empty_payload["engines"][1]["status"], "unsupported");
    assert_eq!(empty_payload["engines"][2]["status"], "unsupported");

    let unity_temp = TestTempDir::new();
    let unity_project_root = unity_temp.path().join("UnityProject");
    std::fs::create_dir_all(&unity_project_root).expect("create unity project root");
    write_file(
        &unity_project_root,
        "ProjectSettings/ProjectVersion.txt",
        "m_EditorVersion: 6000.0.0f1\n",
    );

    let unity_output = std::process::Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "godot",
            "status",
            "--project-path",
            unity_project_root
                .to_str()
                .expect("project path should be UTF-8"),
        ])
        .output()
        .expect("run lux godot status for unity-like project");

    assert!(unity_output.status.success());

    let unity_capabilities_path = unity_project_root.join(".lux/engines/capabilities.json");
    assert!(
        unity_capabilities_path.is_file(),
        "expected persisted engine capability file at {}",
        unity_capabilities_path.display()
    );
    let unity_payload: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&unity_capabilities_path)
            .expect("read unity-like capabilities json"),
    )
    .expect("parse unity-like capabilities json");
    assert_eq!(
        unity_payload["engines"]
            .as_array()
            .map(|entries| entries.len()),
        Some(3)
    );
    assert_eq!(unity_payload["engines"][0]["engine"], "unity");
    assert_eq!(unity_payload["engines"][0]["status"], "detected");
    assert_eq!(unity_payload["engines"][1]["status"], "unsupported");
    assert_eq!(unity_payload["engines"][2]["status"], "unsupported");
}

#[test]
fn detect_unity_project_minimal() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);

    let detection = detect_unity_project(temp.path())
        .expect("detection should succeed")
        .expect("unity project should be detected");

    assert_eq!(detection.root, temp.path().to_path_buf());
    assert_eq!(
        detection.project_name,
        temp.path()
            .file_name()
            .expect("temp dir should have a name")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(detection.editor_version.as_deref(), Some("6000.0.0f1"));
    assert_eq!(detection.render_pipeline.as_deref(), Some("built-in"));
    assert_eq!(detection.scripting_backend, None);
    assert!(detection.target_platforms.is_empty());
    assert!(detection.packages.is_empty());
    assert!(!detection.test_framework_detected);
}

#[test]
fn detect_unity_project_with_manifest() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    write_file(
        temp.path(),
        "Packages/manifest.json",
        r#"{
  "dependencies": {
    "com.unity.modules.ui": "1.0.0",
    "com.unity.textmeshpro": "3.0.6"
  }
}"#,
    );

    let detection = detect_unity_project(temp.path())
        .expect("detection should succeed")
        .expect("unity project should be detected");

    assert_eq!(
        detection.packages,
        vec![
            DetectedPackage {
                name: "com.unity.modules.ui".to_string(),
                version: Some("1.0.0".to_string()),
            },
            DetectedPackage {
                name: "com.unity.textmeshpro".to_string(),
                version: Some("3.0.6".to_string()),
            },
        ]
    );
}

#[test]
fn detect_unity_project_render_pipeline_urp() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    write_file(
        temp.path(),
        "Packages/manifest.json",
        r#"{
  "dependencies": {
    "com.unity.render-pipelines.universal": "14.0.11"
  }
}"#,
    );

    let detection = detect_unity_project(temp.path())
        .expect("detection should succeed")
        .expect("unity project should be detected");

    assert_eq!(detection.render_pipeline.as_deref(), Some("urp"));
}

#[test]
fn detect_unity_project_render_pipeline_hdrp() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    write_file(
        temp.path(),
        "Packages/manifest.json",
        r#"{
  "dependencies": {
    "com.unity.render-pipelines.high-definition": "14.0.11"
  }
}"#,
    );

    let detection = detect_unity_project(temp.path())
        .expect("detection should succeed")
        .expect("unity project should be detected");

    assert_eq!(detection.render_pipeline.as_deref(), Some("hdrp"));
}

#[test]
fn detect_unity_project_test_framework() {
    let temp = TestTempDir::new();
    create_unity_project(&temp);
    write_file(
        temp.path(),
        "Packages/manifest.json",
        r#"{
  "dependencies": {
    "com.unity.test-framework": "1.3.7"
  }
}"#,
    );

    let detection = detect_unity_project(temp.path())
        .expect("detection should succeed")
        .expect("unity project should be detected");

    assert!(detection.test_framework_detected);
}

fn sample_detection() -> project::UnityProjectDetection {
    project::UnityProjectDetection {
        root: PathBuf::from("/tmp/sample-unity"),
        project_name: "SampleGame".to_string(),
        editor_version: Some("6000.0.0f1".to_string()),
        render_pipeline: Some("urp".to_string()),
        scripting_backend: Some("il2cpp".to_string()),
        target_platforms: vec!["Windows".to_string(), "macOS".to_string()],
        packages: vec![
            DetectedPackage {
                name: "com.unity.test-framework".to_string(),
                version: Some("1.3.7".to_string()),
            },
            DetectedPackage {
                name: "com.unity.textmeshpro".to_string(),
                version: Some("3.0.6".to_string()),
            },
        ],
        test_framework_detected: true,
    }
}

#[test]
fn apply_detection_fills_empty_spec() {
    let mut spec = SpecProject::default();
    let detection = sample_detection();

    apply_detection_to_spec(&mut spec, &detection);

    assert_eq!(spec.project_name, "SampleGame");
    assert_eq!(
        spec.unity,
        Some(UnitySpec {
            required_version: None,
            detected_version: Some("6000.0.0f1".to_string()),
            render_pipeline: Some("urp".to_string()),
            scripting_backend: Some("il2cpp".to_string()),
            ..UnitySpec::default()
        })
    );
    assert_eq!(
        spec.targets
            .as_ref()
            .map(|targets| targets.platforms.clone()),
        Some(vec!["Windows".to_string(), "macOS".to_string()])
    );
    assert_eq!(
        spec.packages,
        Some(PackagesSpec {
            required: vec![],
            recommended: vec![],
            forbidden: vec![],
            detected: vec![
                lux_spec::PackageEntry {
                    name: "com.unity.test-framework".to_string(),
                    reason: None,
                    version: Some("1.3.7".to_string()),
                    required_by_domain: Vec::new()
                },
                lux_spec::PackageEntry {
                    name: "com.unity.textmeshpro".to_string(),
                    reason: None,
                    version: Some("3.0.6".to_string()),
                    required_by_domain: Vec::new()
                },
            ],
        })
    );
    assert_eq!(
        spec.testing,
        Some(TestingSpec {
            framework: Some("Unity Test Framework".to_string()),
            strategy: None,
            coverage: false,
        })
    );
    assert_eq!(spec.glossary, Some(GlossarySpec::default()));
}

#[test]
fn apply_detection_preserves_user_fields() {
    let mut spec = SpecProject {
        version: "1.0.0".to_string(),
        schema_version: "2.0".to_string(),
        project_id: "id".to_string(),
        project_name: "UserName".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        source: "manual".to_string(),
        status: lux_spec::SpecStatus::Active,
        meta: lux_spec::ProjectMeta::default(),
        domains: lux_spec::SpecDomains::default(),
        dialectic: lux_spec::DialecticState::default(),
        roadmap: lux_spec::RoadmapSpec::default(),
        unity: Some(UnitySpec {
            required_version: Some("2022.3.0f1".to_string()),
            detected_version: None,
            render_pipeline: Some("built-in".to_string()),
            scripting_backend: Some("mono".to_string()),
            ..UnitySpec::default()
        }),
        targets: Some(lux_spec::TargetsSpec {
            platforms: vec!["Android".to_string()],
            min_sdk: std::collections::HashMap::new(),
            test_platform: Some("PlayMode".to_string()),
            target_platforms: Vec::new(),
        }),
        packages: Some(PackagesSpec {
            required: vec![lux_spec::PackageEntry {
                name: "com.company.required".to_string(),
                reason: Some("needed".to_string()),
                version: Some("1.0.0".to_string()),
                required_by_domain: Vec::new(),
            }],
            forbidden: vec![lux_spec::PackageEntry {
                name: "com.company.forbidden".to_string(),
                reason: None,
                version: None,
                required_by_domain: Vec::new(),
            }],
            recommended: vec![],
            detected: vec![],
        }),
        testing: Some(TestingSpec {
            framework: None,
            strategy: Some("existing".to_string()),
            coverage: true,
        }),
        glossary: Some(GlossarySpec {
            path: "custom/glossary.md".to_string(),
            last_updated: Some("2026-01-01T00:00:00Z".to_string()),
            term_count: 7,
        }),
        schell_evaluation: lux_spec::SchellEvaluation::default(),
        overall_ambiguity: 0.5,
    };
    let detection = sample_detection();

    apply_detection_to_spec(&mut spec, &detection);

    assert_eq!(spec.project_name, "UserName");
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.required_version.as_deref()),
        Some("2022.3.0f1")
    );
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.detected_version.as_deref()),
        Some("6000.0.0f1")
    );
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.render_pipeline.as_deref()),
        Some("built-in")
    );
    assert_eq!(
        spec.unity
            .as_ref()
            .and_then(|unity| unity.scripting_backend.as_deref()),
        Some("mono")
    );
    assert_eq!(
        spec.targets
            .as_ref()
            .map(|targets| targets.platforms.clone()),
        Some(vec!["Android".to_string()])
    );
    assert_eq!(
        spec.targets
            .as_ref()
            .and_then(|targets| targets.test_platform.as_deref()),
        Some("PlayMode")
    );
    assert_eq!(
        spec.packages
            .as_ref()
            .map(|packages| packages.required.clone().len()),
        Some(1)
    );
    assert_eq!(
        spec.packages
            .as_ref()
            .map(|packages| packages.forbidden.clone().len()),
        Some(1)
    );
    assert_eq!(
        spec.packages
            .as_ref()
            .map(|packages| packages.detected.clone().len()),
        Some(2)
    );
    assert_eq!(
        spec.testing
            .as_ref()
            .and_then(|testing| testing.framework.as_deref()),
        Some("Unity Test Framework")
    );
    assert_eq!(
        spec.testing
            .as_ref()
            .and_then(|testing| testing.strategy.as_deref()),
        Some("existing")
    );
    assert_eq!(
        spec.testing.as_ref().map(|testing| testing.coverage),
        Some(true)
    );
    assert_eq!(
        spec.glossary
            .as_ref()
            .map(|glossary| glossary.path.as_str()),
        Some("custom/glossary.md")
    );
}

#[test]
fn apply_detection_idempotent() {
    let mut spec = SpecProject::default();
    let detection = sample_detection();

    apply_detection_to_spec(&mut spec, &detection);
    let once = spec.clone();
    apply_detection_to_spec(&mut spec, &detection);

    assert_eq!(spec, once);
}
