use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lux::lux_roadmap::{
    init_or_load, load, roadmap_file_path, save, RoadmapError, RoadmapPhase, RoadmapPhaseStatus,
    RoadmapReality, CAPABILITY_GLOBAL_CLI, CAPABILITY_MCP_STDIO, CAPABILITY_UNITY_BRIDGE_WORKFLOW,
    REMOTE_WEBRTC_EXPERIMENTAL_FLAG, ROADMAP_SCHEMA_VERSION,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new() -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("lux-roadmap-test-{}-{count}", std::process::id()));
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
fn lux_roadmap_create_default() {
    let roadmap = RoadmapReality::default();

    assert_eq!(roadmap.schema_version, ROADMAP_SCHEMA_VERSION);
    assert!(roadmap.authoritative);
    assert!(!roadmap.phases.is_empty());
    for capability in [
        CAPABILITY_GLOBAL_CLI,
        CAPABILITY_MCP_STDIO,
        CAPABILITY_UNITY_BRIDGE_WORKFLOW,
    ] {
        assert!(roadmap.capabilities.iter().any(|item| item == capability));
    }
    assert!(roadmap.evidence_refs.is_empty());
    assert_eq!(
        roadmap
            .experimental_flags
            .get(REMOTE_WEBRTC_EXPERIMENTAL_FLAG),
        Some(&false)
    );
    assert!(!roadmap.flag_enabled(REMOTE_WEBRTC_EXPERIMENTAL_FLAG));
    assert!(!roadmap.updated_at.is_empty());
    assert!(roadmap.validate().is_ok());
}

#[test]
fn lux_roadmap_init_or_load_creates_canonical_file() {
    let temp = TestTempDir::new();

    let roadmap = init_or_load(temp.path()).expect("roadmap should initialize explicitly");
    let roadmap_path = temp.path().join(".lux/roadmap.json");

    assert!(roadmap.authoritative);
    assert!(roadmap_path.is_file());
    assert_eq!(roadmap_file_path(temp.path()), roadmap_path);
}

#[test]
fn lux_roadmap_load_valid_file() {
    let temp = TestTempDir::new();
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    let roadmap = populated_roadmap();
    save(&roadmap_path, &roadmap).expect("roadmap should save");

    let loaded = load(temp.path()).expect("valid roadmap should load");

    assert_eq!(loaded, roadmap);
    assert!(loaded.flag_enabled("native_opencode"));
    assert!(!loaded.flag_enabled("missing_flag"));
}

#[test]
fn lux_roadmap_rejects_corrupt_json() {
    let temp = TestTempDir::new();
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    std::fs::create_dir_all(roadmap_path.parent().unwrap()).expect(".lux should be created");
    std::fs::write(&roadmap_path, "{not json").expect("corrupt fixture should be written");

    let error = load(temp.path()).expect_err("corrupt roadmap should fail");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Corrupt { .. }));
}

#[test]
fn lux_roadmap_rejects_missing_file_without_silent_default() {
    let temp = TestTempDir::new();

    let error = load(temp.path()).expect_err("missing roadmap should fail");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Missing { .. }));
}

#[test]
fn lux_roadmap_roundtrips_serialize_deserialize() {
    let roadmap = populated_roadmap();

    let json = serde_json::to_string_pretty(&roadmap).expect("serialize roadmap");
    let parsed: RoadmapReality = serde_json::from_str(&json).expect("deserialize roadmap");

    assert_eq!(parsed, roadmap);
    assert_eq!(parsed.phases[0].status, RoadmapPhaseStatus::InProgress);
}

#[test]
fn lux_roadmap_saves_pushed_phase_with_push_evidence() {
    let temp = TestTempDir::new();
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    let mut roadmap = populated_roadmap();
    roadmap.phases[0].status = RoadmapPhaseStatus::Pushed;
    roadmap.phases[0].pushed_at = Some("2026-05-15T00:00:00Z".to_string());
    roadmap.phases[0].push_git_sha = Some("0123456789abcdef".to_string());
    roadmap.phases[0].push_evidence_path =
        Some(".sisyphus/evidence/task-3-roadmap-pushed.txt".to_string());

    save(temp.path(), &roadmap).expect("pushed roadmap should save");
    let content = std::fs::read_to_string(&roadmap_path).expect("roadmap should be readable");
    let loaded = load(temp.path()).expect("pushed roadmap should load");

    assert!(content.contains("\"status\": \"pushed\""));
    assert!(content.contains("\"pushed_at\": \"2026-05-15T00:00:00Z\""));
    assert!(content.contains("\"push_git_sha\": \"0123456789abcdef\""));
    assert!(content
        .contains("\"push_evidence_path\": \".sisyphus/evidence/task-3-roadmap-pushed.txt\""));
    assert_eq!(loaded.phases[0].status, RoadmapPhaseStatus::Pushed);
}

#[test]
fn lux_roadmap_rejects_pushed_phase_missing_evidence_path() {
    let temp = TestTempDir::new();
    let mut roadmap = populated_roadmap();
    roadmap.phases[0].status = RoadmapPhaseStatus::Pushed;
    roadmap.phases[0].pushed_at = Some("2026-05-15T00:00:00Z".to_string());
    roadmap.phases[0].push_git_sha = Some("0123456789abcdef".to_string());
    roadmap.phases[0].push_evidence_path = None;

    let error = save(temp.path(), &roadmap).expect_err("pushed phase without evidence should fail");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Invalid { .. }));
    assert!(roadmap_error.to_string().contains("push_evidence_path"));
}

#[test]
fn lux_roadmap_save_preserves_existing_file_when_validation_fails() {
    let temp = TestTempDir::new();
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    let roadmap = populated_roadmap();
    save(temp.path(), &roadmap).expect("initial roadmap should save");
    let original = std::fs::read_to_string(&roadmap_path).expect("roadmap should be readable");

    let mut invalid = roadmap;
    invalid.phases[0].status = RoadmapPhaseStatus::Pushed;
    invalid.phases[0].pushed_at = Some("2026-05-15T00:00:00Z".to_string());
    invalid.phases[0].push_git_sha = Some("0123456789abcdef".to_string());
    invalid.phases[0].push_evidence_path = Some("   ".to_string());

    save(temp.path(), &invalid).expect_err("invalid pushed roadmap should not save");
    let after = std::fs::read_to_string(&roadmap_path).expect("roadmap should remain readable");

    assert_eq!(after, original);
    assert!(!roadmap_path.with_extension("json.tmp").exists());
}

#[test]
fn lux_roadmap_readme_projection_is_not_authoritative() {
    let temp = TestTempDir::new();
    std::fs::write(
        temp.path().join("README.md"),
        "# Roadmap\n\nThis projection says everything is complete.",
    )
    .expect("README projection should be written");

    let error = load(temp.path()).expect_err("README projection must not replace .lux roadmap");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Missing { .. }));
}

#[test]
fn lux_roadmap_experimental_flags_default_remote_webrtc_hidden_false() {
    let roadmap = RoadmapReality::default();

    assert_eq!(
        roadmap
            .experimental_flags
            .get(REMOTE_WEBRTC_EXPERIMENTAL_FLAG),
        Some(&false)
    );
    assert!(!roadmap.flag_enabled(REMOTE_WEBRTC_EXPERIMENTAL_FLAG));
    assert!(!roadmap.flag_enabled("unknown"));
}

#[test]
fn lux_roadmap_rejects_non_authoritative_file() {
    let temp = TestTempDir::new();
    let mut roadmap = RoadmapReality::default();
    roadmap.authoritative = false;

    let error = save(temp.path(), &roadmap).expect_err("non-authoritative roadmap should fail");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Invalid { .. }));
}

#[test]
fn lux_roadmap_m6_autonomous_appears_in_roadmap() {
    let temp = TestTempDir::new();
    let roadmap_path = temp.path().join(".lux/roadmap.json");
    let mut roadmap = RoadmapReality::default();
    roadmap.phases.push(RoadmapPhase {
        name: "M6: Autonomous — Spec-to-Ticket-to-Execution Pipeline".to_string(),
        status: RoadmapPhaseStatus::Planned,
        evidence_path: None,
        pushed_at: None,
        push_git_sha: None,
        push_evidence_path: None,
    });

    save(temp.path(), &roadmap).expect("roadmap with M6 should save");
    let loaded = load(temp.path()).expect("roadmap with M6 should load");

    let m6 = loaded
        .phases
        .iter()
        .find(|p| p.name.contains("M6") || p.name.contains("Autonomous"))
        .expect("M6/Autonomous phase must appear in roadmap");
    assert_eq!(m6.status, RoadmapPhaseStatus::Planned);
    assert!(m6.push_evidence_path.is_none());
    assert!(m6.pushed_at.is_none());
    assert!(m6.push_git_sha.is_none());

    let content = std::fs::read_to_string(&roadmap_path).expect("roadmap should be readable");
    assert!(content.contains("M6") || content.contains("Autonomous"));
}

#[test]
fn lux_roadmap_m6_cannot_be_pushed_without_evidence() {
    let temp = TestTempDir::new();
    let mut roadmap = RoadmapReality::default();
    roadmap.phases.push(RoadmapPhase {
        name: "M6: Autonomous — Spec-to-Ticket-to-Execution Pipeline".to_string(),
        status: RoadmapPhaseStatus::Pushed,
        evidence_path: None,
        pushed_at: Some("2026-05-15T00:00:00Z".to_string()),
        push_git_sha: Some("0123456789abcdef".to_string()),
        push_evidence_path: None,
    });

    let error = save(temp.path(), &roadmap)
        .expect_err("M6 pushed without push_evidence_path must be rejected");
    let roadmap_error = error
        .downcast_ref::<RoadmapError>()
        .expect("error should preserve roadmap error type");

    assert!(matches!(roadmap_error, RoadmapError::Invalid { .. }));
    assert!(roadmap_error.to_string().contains("push_evidence_path"));
}

fn populated_roadmap() -> RoadmapReality {
    let mut experimental_flags = HashMap::new();
    experimental_flags.insert("native_opencode".to_string(), true);
    RoadmapReality {
        schema_version: ROADMAP_SCHEMA_VERSION.to_string(),
        updated_at: "2026-05-13T00:00:00Z".to_string(),
        phases: vec![RoadmapPhase {
            name: "Roadmap reality lock".to_string(),
            status: RoadmapPhaseStatus::InProgress,
            evidence_path: Some("docs/roadmap-reality-lock.md".to_string()),
            pushed_at: None,
            push_git_sha: None,
            push_evidence_path: None,
        }],
        capabilities: vec!["gateway".to_string(), "mcp".to_string()],
        evidence_refs: vec!["docs/roadmap-reality-lock.md".to_string()],
        experimental_flags,
        authoritative: true,
    }
}
