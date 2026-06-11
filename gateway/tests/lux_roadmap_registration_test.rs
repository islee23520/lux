use lux::lux_roadmap::{
    roadmap_template_for_engine, save, RoadmapError, RoadmapPhaseStatus, RoadmapReality,
    CAPABILITY_GLOBAL_CLI, CAPABILITY_GODOT_BRIDGE_WORKFLOW, CAPABILITY_MCP_STDIO,
    CAPABILITY_THREE_JS_BRIDGE_WORKFLOW, CAPABILITY_UNITY_BRIDGE_WORKFLOW,
    GLOBAL_CLI_MCP_BRIDGE_PHASE,
};
use lux_project::EngineKind;
use serde_json::Value;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestTempDir {
    path: std::path::PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "lux-roadmap-registration-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("stale temp directory should be removed");
        }
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
fn default_roadmap_registers_global_cli_mcp_bridge_workflow() {
    let roadmap = RoadmapReality::default();

    let phase = roadmap
        .phases
        .iter()
        .find(|phase| phase.name == GLOBAL_CLI_MCP_BRIDGE_PHASE)
        .expect("global CLI MCP bridge workflow phase should be registered");

    assert_eq!(phase.status, RoadmapPhaseStatus::Planned);
    assert!(phase.evidence_path.is_none());
    for capability in [
        CAPABILITY_GLOBAL_CLI,
        CAPABILITY_MCP_STDIO,
        CAPABILITY_UNITY_BRIDGE_WORKFLOW,
    ] {
        assert!(
            roadmap
                .capabilities
                .iter()
                .any(|registered| registered == capability),
            "missing default roadmap capability {capability}"
        );
    }
}

#[test]
fn complete_global_cli_mcp_bridge_workflow_requires_evidence_path() {
    let temp = TestTempDir::new("complete-requires-evidence");
    let mut roadmap = RoadmapReality::default();
    let phase = roadmap
        .phases
        .iter_mut()
        .find(|phase| phase.name == GLOBAL_CLI_MCP_BRIDGE_PHASE)
        .expect("global CLI MCP bridge workflow phase should be registered");
    phase.status = RoadmapPhaseStatus::Complete;
    phase.evidence_path = None;

    let error = save(temp.path(), &roadmap)
        .expect_err("completed global CLI MCP bridge workflow needs evidence");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Invalid { .. }));
    assert!(roadmap_error
        .to_string()
        .contains("completed phase M7: Global CLI MCP Bridge Workflow"));
}

#[test]
fn unity_roadmap_template_selects_verified_bridge_workflow() {
    let roadmap = roadmap_template_for_engine(EngineKind::Unity);

    let phase = roadmap
        .phases
        .iter()
        .find(|phase| phase.name == GLOBAL_CLI_MCP_BRIDGE_PHASE)
        .expect("selected Unity roadmap template should include global CLI MCP phase");

    assert_eq!(phase.status, RoadmapPhaseStatus::Planned);
    assert!(roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_UNITY_BRIDGE_WORKFLOW));
    assert!(!roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_GODOT_BRIDGE_WORKFLOW));
    assert!(!roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_THREE_JS_BRIDGE_WORKFLOW));
}

#[test]
fn godot_roadmap_template_selects_partial_bridge_workflow() {
    let roadmap = roadmap_template_for_engine(EngineKind::Godot);

    let phase = roadmap
        .phases
        .iter()
        .find(|phase| phase.name == "M7: Godot Global CLI MCP Bridge Workflow")
        .expect("selected Godot roadmap template should include Godot-specific phase");

    assert_eq!(phase.status, RoadmapPhaseStatus::Partial);
    assert!(roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_GODOT_BRIDGE_WORKFLOW));
    assert!(!roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_UNITY_BRIDGE_WORKFLOW));
}

#[test]
fn three_js_roadmap_template_stays_planned_without_bridge_parity() {
    let roadmap = roadmap_template_for_engine(EngineKind::ThreeJs);

    let phase = roadmap
        .phases
        .iter()
        .find(|phase| phase.name == "M7: Three.js Global CLI MCP Bridge Workflow")
        .expect("selected Three.js roadmap template should include Three.js-specific phase");

    assert_eq!(phase.status, RoadmapPhaseStatus::Planned);
    assert!(roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_THREE_JS_BRIDGE_WORKFLOW));
    assert!(!roadmap
        .capabilities
        .iter()
        .any(|capability| capability == CAPABILITY_UNITY_BRIDGE_WORKFLOW));
}

#[test]
fn roadmap_init_cli_writes_selected_godot_template() {
    let temp = TestTempDir::new("cli-godot-template");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "roadmap",
            "--project-path",
            temp.path().to_str().expect("temp path should be UTF-8"),
            "init",
            "--engine",
            "godot",
        ])
        .output()
        .expect("run lux roadmap init");

    assert!(
        output.status.success(),
        "lux roadmap init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    let payload: Value =
        serde_json::from_str(&std::fs::read_to_string(roadmap_path).expect("roadmap JSON"))
            .expect("roadmap should parse");

    let phase = payload["phases"]
        .as_array()
        .expect("phases array")
        .iter()
        .find(|phase| phase["name"] == "M7: Godot Global CLI MCP Bridge Workflow")
        .expect("Godot template phase should be persisted");
    assert_eq!(phase["status"], "partial");
    assert!(payload["capabilities"]
        .as_array()
        .expect("capabilities array")
        .iter()
        .any(|capability| capability == CAPABILITY_GODOT_BRIDGE_WORKFLOW));
}

#[test]
fn roadmap_init_cli_refuses_existing_roadmap() {
    let temp = TestTempDir::new("cli-existing-roadmap");
    save(temp.path(), &RoadmapReality::default()).expect("seed roadmap");
    let original =
        std::fs::read_to_string(temp.path().join(".lux/roadmap.json")).expect("seeded roadmap");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "roadmap",
            "--project-path",
            temp.path().to_str().expect("temp path should be UTF-8"),
            "init",
            "--engine",
            "godot",
        ])
        .output()
        .expect("run lux roadmap init");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Lux roadmap already exists"));
    let after =
        std::fs::read_to_string(temp.path().join(".lux/roadmap.json")).expect("roadmap after");
    assert_eq!(after, original);
}

#[cfg(unix)]
#[test]
fn roadmap_init_cli_rejects_symlinked_lux_root() {
    let temp = TestTempDir::new("cli-symlinked-lux-root");
    let outside = std::env::temp_dir().join(format!(
        "lux-roadmap-registration-outside-root-{}",
        std::process::id()
    ));
    if outside.exists() {
        std::fs::remove_dir_all(&outside).expect("stale outside directory should be removed");
    }
    std::fs::create_dir(&outside).expect("outside directory should be created");
    std::os::unix::fs::symlink(&outside, temp.path().join(".lux")).expect("symlink .lux");

    let output = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "roadmap",
            "--project-path",
            temp.path().to_str().expect("temp path should be UTF-8"),
            "init",
            "--engine",
            "godot",
        ])
        .output()
        .expect("run lux roadmap init");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("roadmap directory must not be a symlink"));
    assert!(!outside.join("roadmap.json").exists());
    std::fs::remove_dir_all(outside).expect("outside directory cleanup");
}

#[cfg(unix)]
#[test]
fn roadmap_save_rejects_symlinked_roadmap_file() {
    let temp = TestTempDir::new("save-symlinked-roadmap-file");
    std::fs::create_dir(temp.path().join(".lux")).expect("create .lux");
    let outside = temp.path().join("outside-roadmap.json");
    std::fs::write(&outside, "outside-original").expect("outside target");
    std::os::unix::fs::symlink(&outside, temp.path().join(".lux/roadmap.json"))
        .expect("roadmap symlink");

    let error = save(temp.path(), &RoadmapReality::default()).expect_err("symlink must fail");

    assert!(error
        .to_string()
        .contains("roadmap file must not be a symlink"));
    assert_eq!(
        std::fs::read_to_string(outside).expect("outside content"),
        "outside-original"
    );
}

#[cfg(unix)]
#[test]
fn roadmap_save_rejects_hardlinked_temp_file() {
    let temp = TestTempDir::new("save-hardlinked-temp-file");
    std::fs::create_dir(temp.path().join(".lux")).expect("create .lux");
    let outside = temp.path().join("outside-temp.json");
    std::fs::write(&outside, "outside-original").expect("outside target");
    std::fs::hard_link(&outside, temp.path().join(".lux/roadmap.json.tmp"))
        .expect("roadmap temp hardlink");

    let error = save(temp.path(), &RoadmapReality::default()).expect_err("hardlink must fail");

    assert!(error
        .to_string()
        .contains("temporary file must not be hardlinked"));
    assert_eq!(
        std::fs::read_to_string(outside).expect("outside content"),
        "outside-original"
    );
}
