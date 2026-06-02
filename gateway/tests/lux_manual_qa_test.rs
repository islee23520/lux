use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use lux::lux_manual_qa::{
    capture_manual_qa_evidence, ManualQaCapabilities, ManualQaCapabilityStatus, ManualQaCommand,
    ManualQaEngine, ManualQaEvidenceRequest, ManualQaPhase, ManualQaStatus,
};

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lux-manual-qa-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("temp project should be created");
        Self { path }
    }

    fn script(&self, name: &str, body: &str) -> String {
        let path = self.path.join(name);
        fs::write(&path, body).expect("script should be written");
        let mut permissions = fs::metadata(&path)
            .expect("script metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("script should be executable");
        path.to_string_lossy().into_owned()
    }

    fn opts(&self, engine: ManualQaEngine) -> ManualQaEvidenceRequest {
        ManualQaEvidenceRequest {
            engine,
            run_id: "h9-run".to_string(),
            project_path: self.path.clone(),
            evidence_dir: PathBuf::from(".lux/evidence/manual-qa/h9-run"),
            commands: Vec::new(),
            capabilities: ManualQaCapabilities {
                screenshot: ManualQaCapabilityStatus::Supported,
                video: ManualQaCapabilityStatus::Unsupported,
            },
            godot_cli: None,
        }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn unity_adapter_records_dynamic_code_and_screenshot_when_commands_succeed() {
    let project = TestProject::new("unity");
    let screenshot_path = project.path.join("unity-shot.png");
    let screenshot_script = project.script(
        "unity-screenshot.sh",
        &format!(
            "#!/bin/sh\nprintf png > {}\nprintf 'screenshot_path={}\\n'\n",
            screenshot_path.display(),
            screenshot_path.display()
        ),
    );
    let mut request = project.opts(ManualQaEngine::Unity);
    request.commands = vec![
        ManualQaCommand::new(
            ManualQaPhase::Compile,
            project.script("compile.sh", "#!/bin/sh\nprintf compile-ok\n"),
        ),
        ManualQaCommand::new(
            ManualQaPhase::Test,
            project.script("test.sh", "#!/bin/sh\nprintf test-ok\n"),
        ),
        ManualQaCommand::new(
            ManualQaPhase::DynamicCode,
            project.script("dynamic.sh", "#!/bin/sh\nprintf dynamic-code-ok\n"),
        ),
        ManualQaCommand::new(ManualQaPhase::Screenshot, screenshot_script),
    ];

    let result = capture_manual_qa_evidence(&request).expect("manual QA should capture");

    assert_eq!(result.status, ManualQaStatus::Passed);
    assert_eq!(result.evidence_paths.len(), 5);
    let evidence = read_all(&project.path, &result.evidence_paths);
    assert!(evidence.contains("\"phase\":\"dynamic_code\""));
    assert!(evidence.contains("\"screenshot_path\""));
    assert!(evidence.contains("\"video\":\"unsupported\""));
    copy_artifact(
        &project.path,
        &result.evidence_paths[3],
        "game-harness-task-9-unity-dynamic-code.json",
    );
    copy_artifact(
        &project.path,
        &result.evidence_paths[4],
        "game-harness-task-9-unity-screenshot.json",
    );
}

#[test]
fn screenshot_unsupported_records_blocker_without_requiring_video() {
    let project = TestProject::new("screenshot-blocker");
    let mut request = project.opts(ManualQaEngine::Unity);
    request.capabilities.screenshot = ManualQaCapabilityStatus::Blocker;
    request.commands = vec![ManualQaCommand::new(
        ManualQaPhase::DynamicCode,
        project.script("dynamic.sh", "#!/bin/sh\nprintf dynamic-code-ok\n"),
    )];

    let result = capture_manual_qa_evidence(&request).expect("blocker evidence should write");

    assert_eq!(result.status, ManualQaStatus::Blocked);
    let evidence = read_all(&project.path, &result.evidence_paths);
    assert!(evidence.contains("\"screenshot\":\"blocker\""));
    assert!(evidence.contains("screenshot unavailable"));
    assert!(evidence.contains("\"video\":\"unsupported\""));
    copy_artifact(
        &project.path,
        &result.evidence_paths[1],
        "game-harness-task-9-screenshot-blocker.json",
    );
    copy_artifact(
        &project.path,
        &result.evidence_paths[0],
        "game-harness-task-9-video-capability.json",
    );
}

#[test]
fn godot_adapter_records_blocker_when_cli_is_missing() {
    let project = TestProject::new("godot-missing");
    let mut request = project.opts(ManualQaEngine::Godot);
    request.godot_cli = Some("__lux_missing_godot_for_h9__".to_string());

    let result = capture_manual_qa_evidence(&request).expect("missing Godot should be captured");

    assert_eq!(result.status, ManualQaStatus::Blocked);
    let evidence = read_all(&project.path, &result.evidence_paths);
    assert!(evidence.contains("\"engine\":\"godot\""));
    assert!(evidence.contains("missing Godot CLI"));
}

#[test]
fn threejs_adapter_records_dev_server_and_browser_screenshot() {
    let project = TestProject::new("threejs");
    let screenshot_path = project.path.join("three-shot.png");
    let screenshot_script = project.script(
        "browser-shot.sh",
        &format!(
            "#!/bin/sh\nprintf png > {}\nprintf 'screenshot_path={}\\n'\n",
            screenshot_path.display(),
            screenshot_path.display()
        ),
    );
    let mut request = project.opts(ManualQaEngine::ThreeJs);
    request.commands = vec![
        ManualQaCommand::new(
            ManualQaPhase::DevServer,
            project.script("dev-server.sh", "#!/bin/sh\nprintf dev-server-ready\n"),
        ),
        ManualQaCommand::new(ManualQaPhase::BrowserScreenshot, screenshot_script),
    ];

    let result = capture_manual_qa_evidence(&request).expect("threejs evidence should capture");

    assert_eq!(result.status, ManualQaStatus::Passed);
    let evidence = read_all(&project.path, &result.evidence_paths);
    assert!(evidence.contains("\"phase\":\"dev_server\""));
    assert!(evidence.contains("\"phase\":\"browser_screenshot\""));
    assert!(evidence.contains("\"screenshot_path\""));
    copy_artifact(
        &project.path,
        &result.evidence_paths[1],
        "game-harness-task-9-threejs-dev-server.json",
    );
    copy_artifact(
        &project.path,
        &result.evidence_paths[2],
        "game-harness-task-9-threejs-browser-screenshot.json",
    );
}

fn read_all(project_path: &Path, evidence_paths: &[String]) -> String {
    evidence_paths
        .iter()
        .map(|path| fs::read_to_string(project_path.join(path)).expect("evidence should read"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn copy_artifact(project_path: &Path, evidence_path: &str, file_name: &str) {
    let Ok(copy_dir) = env::var("LUX_H9_EVIDENCE_COPY_DIR") else {
        return;
    };
    let copy_path = PathBuf::from(copy_dir).join(file_name);
    fs::create_dir_all(copy_path.parent().expect("copy path should have parent"))
        .expect("artifact copy directory should be created");
    fs::copy(project_path.join(evidence_path), copy_path).expect("artifact should be copied");
}
