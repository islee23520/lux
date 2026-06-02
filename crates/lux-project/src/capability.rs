use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Unity,
    Godot,
    ThreeJs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Verified,
    Partial,
    Planned,
    Unsupported,
}

impl CapabilityStatus {
    pub const fn blocks_completion(self) -> bool {
        !matches!(self, Self::Verified | Self::Partial)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineCapabilityStatus {
    Detected,
    Limited,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseCapabilityError {
    kind: &'static str,
    value: String,
}

impl ParseCapabilityError {
    fn new(kind: &'static str, value: &str) -> Self {
        Self {
            kind,
            value: value.to_string(),
        }
    }
}

impl fmt::Display for ParseCapabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown {} '{}'", self.kind, self.value)
    }
}

impl Error for ParseCapabilityError {}

impl FromStr for EngineKind {
    type Err = ParseCapabilityError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "unity" => Ok(Self::Unity),
            "godot" => Ok(Self::Godot),
            "three_js" | "threejs" => Ok(Self::ThreeJs),
            _ => Err(ParseCapabilityError::new("engine", value)),
        }
    }
}

impl FromStr for CapabilityStatus {
    type Err = ParseCapabilityError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "verified" => Ok(Self::Verified),
            "partial" => Ok(Self::Partial),
            "planned" => Ok(Self::Planned),
            "unsupported" => Ok(Self::Unsupported),
            _ => Err(ParseCapabilityError::new("capability status", value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineCapability {
    pub engine: EngineKind,
    pub capability: String,
    pub status: CapabilityStatus,
    pub reason: String,
}

impl EngineCapability {
    pub fn new(
        engine: EngineKind,
        capability: impl Into<String>,
        status: CapabilityStatus,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            engine,
            capability: capability.into(),
            status,
            reason: reason.into(),
        }
    }

    pub const fn blocks_completion(&self) -> bool {
        self.status.blocks_completion()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineCapabilityBlocker {
    pub engine: EngineKind,
    pub capability: String,
    pub status: CapabilityStatus,
    pub reason: String,
    pub evidence_path: String,
    pub recommended_next_supported_action: String,
}

impl EngineCapabilityBlocker {
    pub fn new(
        engine: EngineKind,
        capability: impl Into<String>,
        status: CapabilityStatus,
        reason: impl Into<String>,
        evidence_path: impl Into<String>,
        recommended_next_supported_action: impl Into<String>,
    ) -> Self {
        Self {
            engine,
            capability: capability.into(),
            status,
            reason: reason.into(),
            evidence_path: evidence_path.into(),
            recommended_next_supported_action: recommended_next_supported_action.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineCapabilityRecord {
    pub engine: EngineKind,
    pub status: EngineCapabilityStatus,
    pub reason: String,
    pub detected: bool,
    pub tool_available: bool,
    pub manual_qa_supported: bool,
    pub screenshot_supported: bool,
    pub video_supported: bool,
    pub blocker_reason: Option<String>,
}

impl EngineCapabilityRecord {
    fn detected(
        engine: EngineKind,
        reason: impl Into<String>,
        tool_available: bool,
        manual_qa_supported: bool,
        screenshot_supported: bool,
        video_supported: bool,
    ) -> Self {
        Self {
            engine,
            status: EngineCapabilityStatus::Detected,
            reason: reason.into(),
            detected: true,
            tool_available,
            manual_qa_supported,
            screenshot_supported,
            video_supported,
            blocker_reason: None,
        }
    }

    fn limited(
        engine: EngineKind,
        reason: impl Into<String>,
        blocker_reason: impl Into<String>,
        tool_available: bool,
        manual_qa_supported: bool,
        screenshot_supported: bool,
        video_supported: bool,
    ) -> Self {
        Self {
            engine,
            status: EngineCapabilityStatus::Limited,
            reason: reason.into(),
            detected: true,
            tool_available,
            manual_qa_supported,
            screenshot_supported,
            video_supported,
            blocker_reason: Some(blocker_reason.into()),
        }
    }

    fn unsupported(
        engine: EngineKind,
        reason: impl Into<String>,
        blocker_reason: impl Into<String>,
    ) -> Self {
        Self {
            engine,
            status: EngineCapabilityStatus::Unsupported,
            reason: reason.into(),
            detected: false,
            tool_available: false,
            manual_qa_supported: false,
            screenshot_supported: false,
            video_supported: false,
            blocker_reason: Some(blocker_reason.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineCapabilityCatalog {
    pub schema_version: u32,
    pub engine: EngineKind,
    pub status: EngineCapabilityStatus,
    pub reason: String,
    pub unity: EngineCapabilityRecord,
    pub godot: EngineCapabilityRecord,
    #[serde(rename = "three_js")]
    pub three_js: EngineCapabilityRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineCapabilityInventory {
    pub path: PathBuf,
    pub engines: Vec<EngineCapabilityRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct EngineCapabilitySnapshot {
    pub engine: EngineKind,
    pub status: &'static str,
    pub capabilities: Vec<EngineCapability>,
    pub engines: Vec<EngineCapabilityRecord>,
}

pub const ENGINE_CAPABILITY_SCHEMA_VERSION: u32 = 1;

pub fn recommended_capability_blockers(engine: Option<EngineKind>) -> Vec<EngineCapabilityBlocker> {
    match engine {
        Some(EngineKind::Godot) => vec![
            EngineCapabilityBlocker::new(
                EngineKind::Godot,
                "build",
                CapabilityStatus::Unsupported,
                "Godot build is blocked until GoPeak-backed build has automated verification",
                ".lux/evidence/capability-blockers/godot-build.json",
                "Use `lux godot status` and `lux bridge install --type godot`; do not claim build completion.",
            ),
            EngineCapabilityBlocker::new(
                EngineKind::Godot,
                "run/test/scene-inspect/screenshot",
                CapabilityStatus::Planned,
                "Godot runtime evidence commands require a verified Godot/GoPeak evidence loop",
                ".lux/evidence/capability-blockers/godot-runtime.json",
                "Collect project detection and bridge-install evidence only.",
            ),
        ],
        Some(EngineKind::ThreeJs) => vec![EngineCapabilityBlocker::new(
            EngineKind::ThreeJs,
            "runtime",
            CapabilityStatus::Planned,
            "Three.js runtime support has no verified LUX harness yet",
            ".lux/evidence/capability-blockers/threejs-runtime.json",
            "Use Unity verified paths or record an explicit unsupported-engine blocker.",
        )],
        Some(EngineKind::Unity) | None => Vec::new(),
    }
}

pub fn persist_engine_capabilities(
    project_root: &Path,
    active_engine: EngineKind,
) -> Result<EngineCapabilityCatalog> {
    let catalog = build_catalog(project_root, active_engine)?;
    write_json(&capabilities_path(project_root), &catalog)?;
    Ok(catalog)
}

pub fn detect_engine_capabilities(project_root: &Path) -> Result<EngineCapabilityInventory> {
    let catalog = build_catalog(project_root, EngineKind::Unity)?;
    let engines = vec![
        catalog.unity.clone(),
        catalog.godot.clone(),
        catalog.three_js.clone(),
    ];
    write_json(
        &capabilities_path(project_root),
        &serde_json::json!({
            "schema_version": ENGINE_CAPABILITY_SCHEMA_VERSION,
            "engines": engines,
        }),
    )?;
    Ok(EngineCapabilityInventory {
        path: capabilities_path(project_root),
        engines,
    })
}

pub fn persist_engine_status_snapshot(project_root: &Path, engine: EngineKind) -> Result<()> {
    let catalog = build_catalog(project_root, engine)?;
    let active = match engine {
        EngineKind::Unity => &catalog.unity,
        EngineKind::Godot => &catalog.godot,
        EngineKind::ThreeJs => &catalog.three_js,
    };
    let snapshot = EngineCapabilitySnapshot {
        engine,
        status: if active.detected {
            "supported"
        } else {
            "unsupported"
        },
        capabilities: snapshot_capabilities(engine, active.detected),
        engines: vec![catalog.unity, catalog.godot, catalog.three_js],
    };
    write_json(&capabilities_path(project_root), &snapshot)
}

fn snapshot_capabilities(engine: EngineKind, detected: bool) -> Vec<EngineCapability> {
    match (engine, detected) {
        (EngineKind::Godot, true) => vec![
            EngineCapability::new(
                EngineKind::Godot,
                "project_detection",
                CapabilityStatus::Partial,
                "Godot project markers are detected, but build/runtime evidence remain gated",
            ),
            EngineCapability::new(
                EngineKind::Godot,
                "build",
                CapabilityStatus::Unsupported,
                "Godot build is blocked until GoPeak-backed build has automated verification",
            ),
            EngineCapability::new(
                EngineKind::Godot,
                "runtime",
                CapabilityStatus::Planned,
                "Godot runtime evidence commands require a verified Godot/GoPeak loop",
            ),
        ],
        (EngineKind::Godot, false) => vec![EngineCapability::new(
            EngineKind::Godot,
            "project_detection",
            CapabilityStatus::Unsupported,
            "Godot 4 project markers were not found at the requested path",
        )],
        (EngineKind::Unity, true) => vec![EngineCapability::new(
            EngineKind::Unity,
            "project_detection",
            CapabilityStatus::Verified,
            "Unity project markers are detected locally",
        )],
        (EngineKind::Unity, false) => vec![EngineCapability::new(
            EngineKind::Unity,
            "project_detection",
            CapabilityStatus::Unsupported,
            "Unity project markers were not found at the requested path",
        )],
        (EngineKind::ThreeJs, true) => vec![EngineCapability::new(
            EngineKind::ThreeJs,
            "runtime",
            CapabilityStatus::Planned,
            "Three.js runtime support has no verified LUX harness yet",
        )],
        (EngineKind::ThreeJs, false) => vec![EngineCapability::new(
            EngineKind::ThreeJs,
            "runtime",
            CapabilityStatus::Unsupported,
            "Three.js project markers were not found at the requested path",
        )],
    }
}

fn build_catalog(
    project_root: &Path,
    active_engine: EngineKind,
) -> Result<EngineCapabilityCatalog> {
    let unity = if project_root
        .join("ProjectSettings/ProjectVersion.txt")
        .is_file()
    {
        EngineCapabilityRecord::detected(
            EngineKind::Unity,
            "Unity markers found in ProjectSettings/ProjectVersion.txt",
            true,
            true,
            true,
            false,
        )
    } else {
        EngineCapabilityRecord::unsupported(
            EngineKind::Unity,
            "Unity markers not found",
            format!(
                "Missing {}",
                project_root
                    .join("ProjectSettings/ProjectVersion.txt")
                    .display()
            ),
        )
    };
    let godot = if project_root.join("project.godot").is_file() {
        EngineCapabilityRecord::limited(
            EngineKind::Godot,
            "Godot project markers found in project.godot",
            "Godot build/run/test remain blocked until GoPeak-backed verification exists",
            true,
            true,
            false,
            false,
        )
    } else {
        EngineCapabilityRecord::unsupported(
            EngineKind::Godot,
            "Godot project markers not found",
            format!("Missing {}", project_root.join("project.godot").display()),
        )
    };
    let three_js = if detect_three_js_project(project_root)? {
        EngineCapabilityRecord::limited(
            EngineKind::ThreeJs,
            "Three.js project markers found in package.json",
            "Three.js has no verified LUX harness yet",
            false,
            false,
            false,
            false,
        )
    } else {
        EngineCapabilityRecord::unsupported(
            EngineKind::ThreeJs,
            "Three.js project markers not found",
            "Missing package.json dependency on three",
        )
    };
    let active = match active_engine {
        EngineKind::Unity => &unity,
        EngineKind::Godot => &godot,
        EngineKind::ThreeJs => &three_js,
    };
    Ok(EngineCapabilityCatalog {
        schema_version: ENGINE_CAPABILITY_SCHEMA_VERSION,
        engine: active_engine,
        status: active.status,
        reason: active.reason.clone(),
        unity,
        godot,
        three_js,
    })
}

fn detect_three_js_project(project_root: &Path) -> Result<bool> {
    let package_json = project_root.join("package.json");
    if !package_json.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(&package_json)
        .with_context(|| format!("failed to read {}", package_json.display()))?;
    let manifest: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", package_json.display()))?;
    Ok([
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .into_iter()
    .any(|section| {
        manifest
            .get(section)
            .and_then(Value::as_object)
            .is_some_and(|dependencies| dependencies.contains_key("three"))
    }))
}

fn capabilities_path(project_root: &Path) -> PathBuf {
    project_root.join(".lux/engines/capabilities.json")
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .context("failed to serialize engine capability payload")?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace file {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{
        detect_engine_capabilities, persist_engine_capabilities, recommended_capability_blockers,
        CapabilityStatus, EngineCapability, EngineCapabilityBlocker, EngineCapabilityStatus,
        EngineKind, ENGINE_CAPABILITY_SCHEMA_VERSION,
    };
    use std::{fs, str::FromStr};

    #[test]
    fn capability_status_levels() {
        let capabilities = [
            EngineCapability::new(
                EngineKind::Unity,
                "project detection",
                CapabilityStatus::Verified,
                "Unity project markers are detected locally",
            ),
            EngineCapability::new(
                EngineKind::Godot,
                "project detection",
                CapabilityStatus::Partial,
                "Godot project markers are detected, but build remains unsupported",
            ),
            EngineCapability::new(
                EngineKind::ThreeJs,
                "runtime",
                CapabilityStatus::Planned,
                "Three.js runtime support has no verified harness yet",
            ),
            EngineCapability::new(
                EngineKind::Godot,
                "build",
                CapabilityStatus::Unsupported,
                "Godot build is blocked until end-to-end verification exists",
            ),
        ];

        assert_eq!(capabilities[0].status, CapabilityStatus::Verified);
        assert_eq!(capabilities[1].status, CapabilityStatus::Partial);
        assert_eq!(capabilities[2].status, CapabilityStatus::Planned);
        assert_eq!(capabilities[3].status, CapabilityStatus::Unsupported);
        assert!(capabilities[3].blocks_completion());
    }

    #[test]
    fn invalid_capability_status_is_rejected() {
        let error = CapabilityStatus::from_str("certified").expect_err("status should be rejected");
        assert_eq!(error.to_string(), "unknown capability status 'certified'");
    }

    #[test]
    fn unsupported_engine_blocker_payload() {
        let blocker = EngineCapabilityBlocker::new(
            EngineKind::Godot,
            "build",
            CapabilityStatus::Unsupported,
            "Godot build is blocked until end-to-end verification exists",
            ".lux/evidence/capability-blockers/godot-build.json",
            "Use `lux godot status` and bridge install until build verification lands",
        );
        assert_eq!(blocker.engine, EngineKind::Godot);
        assert_eq!(blocker.capability, "build");
        assert_eq!(blocker.status, CapabilityStatus::Unsupported);
        assert!(blocker.reason.contains("end-to-end verification"));
        assert_eq!(
            blocker.evidence_path,
            ".lux/evidence/capability-blockers/godot-build.json"
        );
        assert!(blocker
            .recommended_next_supported_action
            .contains("lux godot status"));
    }

    #[test]
    fn capability_blockers_cover_empty_unity_godot_and_three_markers() {
        assert!(recommended_capability_blockers(None).is_empty());
        assert!(recommended_capability_blockers(Some(EngineKind::Unity)).is_empty());
        let godot = recommended_capability_blockers(Some(EngineKind::Godot));
        assert!(godot.iter().any(|blocker| blocker.capability == "build"
            && blocker.status == CapabilityStatus::Unsupported));
        let three = recommended_capability_blockers(Some(EngineKind::ThreeJs));
        assert!(three.iter().any(|blocker| blocker.capability == "runtime"
            && blocker.status == CapabilityStatus::Planned));
    }

    #[test]
    fn persist_engine_capabilities_writes_all_three_engines() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let project_root = temp.path();
        fs::create_dir_all(project_root.join("ProjectSettings")).expect("create unity marker dir");
        fs::write(
            project_root.join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 6000.0.0f1\n",
        )
        .expect("write unity marker");
        fs::write(
            project_root.join("package.json"),
            "{\n  \"dependencies\": {\n    \"three\": \"^0.179.0\"\n  }\n}\n",
        )
        .expect("write three.js marker");
        fs::write(project_root.join("project.godot"), "config_version=5\n")
            .expect("write godot marker");

        let catalog = persist_engine_capabilities(project_root, EngineKind::Godot)
            .expect("capabilities should persist");
        let payload: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(project_root.join(".lux/engines/capabilities.json"))
                .expect("persisted capabilities json should be readable"),
        )
        .expect("persisted capabilities json should parse");

        assert_eq!(catalog.schema_version, ENGINE_CAPABILITY_SCHEMA_VERSION);
        assert_eq!(payload["engine"], "godot");
        assert_eq!(payload["status"], "limited");
        assert_eq!(payload["unity"]["status"], "detected");
        assert_eq!(payload["godot"]["status"], "limited");
        assert_eq!(payload["three_js"]["status"], "limited");
        assert_eq!(
            payload["unity"]["reason"],
            "Unity markers found in ProjectSettings/ProjectVersion.txt"
        );
        assert!(payload["godot"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("project.godot")));
        assert!(payload["three_js"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Three.js project markers found")));
    }

    #[test]
    fn detect_engine_capabilities_writes_inventory_file() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        fs::create_dir_all(temp.path().join("ProjectSettings")).expect("create unity marker dir");
        fs::write(
            temp.path().join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 6000.0.0f1\n",
        )
        .expect("write unity marker");

        let inventory = detect_engine_capabilities(temp.path()).expect("inventory should persist");
        let payload: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&inventory.path).expect("persisted inventory should be readable"),
        )
        .expect("persisted inventory should parse");

        assert_eq!(
            payload["engines"].as_array().map(std::vec::Vec::len),
            Some(3)
        );
        assert!(inventory
            .engines
            .iter()
            .any(|engine| engine.engine == EngineKind::Unity
                && engine.status == EngineCapabilityStatus::Detected));
    }
}
