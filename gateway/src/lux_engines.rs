use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{lux_io, project, project_godot};
use lux_project::{recommended_capability_blockers, EngineCapabilityBlocker, EngineKind};

const CAPABILITY_SNAPSHOT_SCHEMA_VERSION: &str = "1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineCapabilityStatus {
    Detected,
    Limited,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineCapabilityRecord {
    pub engine: EngineKind,
    pub detected: bool,
    pub tool_available: bool,
    pub manual_qa_supported: bool,
    pub screenshot_supported: bool,
    pub video_supported: bool,
    pub status: EngineCapabilityStatus,
    pub reason: String,
    pub blocker_reason: Option<String>,
    pub blockers: Vec<EngineCapabilityBlocker>,
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
            detected: true,
            tool_available,
            manual_qa_supported,
            screenshot_supported,
            video_supported,
            status: EngineCapabilityStatus::Detected,
            reason: reason.into(),
            blocker_reason: None,
            blockers: Vec::new(),
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
        blockers: Vec<EngineCapabilityBlocker>,
    ) -> Self {
        Self {
            engine,
            detected: true,
            tool_available,
            manual_qa_supported,
            screenshot_supported,
            video_supported,
            status: EngineCapabilityStatus::Limited,
            reason: reason.into(),
            blocker_reason: Some(blocker_reason.into()),
            blockers,
        }
    }

    fn unsupported(
        engine: EngineKind,
        reason: impl Into<String>,
        blocker_reason: impl Into<String>,
        blockers: Vec<EngineCapabilityBlocker>,
    ) -> Self {
        Self {
            engine,
            detected: false,
            tool_available: false,
            manual_qa_supported: false,
            screenshot_supported: false,
            video_supported: false,
            status: EngineCapabilityStatus::Unsupported,
            reason: reason.into(),
            blocker_reason: Some(blocker_reason.into()),
            blockers,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineCapabilitySnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub project_root: String,
    pub engine: EngineKind,
    pub detected: bool,
    pub tool_available: bool,
    pub manual_qa_supported: bool,
    pub screenshot_supported: bool,
    pub video_supported: bool,
    pub status: EngineCapabilityStatus,
    pub reason: String,
    pub blocker_reason: Option<String>,
    pub unity: EngineCapabilityRecord,
    pub godot: EngineCapabilityRecord,
    pub three_js: EngineCapabilityRecord,
    pub engines: Vec<EngineCapabilityRecord>,
}

pub fn write_engine_capability_snapshot(
    project_path: &Path,
    active_engine: EngineKind,
) -> Result<EngineCapabilitySnapshot> {
    let snapshot = collect_engine_capability_snapshot(project_path, active_engine)?;
    let snapshot_path = project_path.join(".lux/engines/capabilities.json");
    lux_io::atomic_write_json(&snapshot_path, &snapshot).with_context(|| {
        format!(
            "failed to write engine capability snapshot {}",
            snapshot_path.display()
        )
    })?;
    Ok(snapshot)
}

pub fn persist_engine_capabilities(
    project_path: &Path,
    active_engine: EngineKind,
) -> Result<EngineCapabilitySnapshot> {
    write_engine_capability_snapshot(project_path, active_engine)
}

pub fn collect_engine_capability_snapshot(
    project_path: &Path,
    active_engine: EngineKind,
) -> Result<EngineCapabilitySnapshot> {
    let generated_at = Utc::now().to_rfc3339();
    let project_root = project_path.display().to_string();

    let unity_detected = project::detect_unity_project(project_path)?.is_some();
    let godot_detected = project_godot::detect_godot_project(project_path).is_some();
    let three_detected = detect_three_js_project(project_path)?.is_some();

    let unity = if unity_detected {
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
            "Unity project markers not detected in project root.",
            format!(
                "Missing {}",
                project_path
                    .join("ProjectSettings")
                    .join("ProjectVersion.txt")
                    .display()
            ),
            Vec::new(),
        )
    };

    let godot_blockers = recommended_capability_blockers(Some(EngineKind::Godot));
    let godot = if godot_detected {
        EngineCapabilityRecord::limited(
            EngineKind::Godot,
            "Godot project markers found in project.godot",
            godot_blockers
                .first()
                .map(|blocker| blocker.reason.clone())
                .unwrap_or_else(|| {
                    "Godot build/run/test remain blocked until GoPeak-backed verification exists"
                        .to_string()
                }),
            false,
            false,
            false,
            false,
            godot_blockers,
        )
    } else {
        EngineCapabilityRecord::unsupported(
            EngineKind::Godot,
            "Godot project markers not detected in project root.",
            "Missing project.godot",
            godot_blockers,
        )
    };

    let three_js_blockers = recommended_capability_blockers(Some(EngineKind::ThreeJs));
    let three_js = if three_detected {
        EngineCapabilityRecord::limited(
            EngineKind::ThreeJs,
            "Three.js project markers found in package.json",
            three_js_blockers
                .first()
                .map(|blocker| blocker.reason.clone())
                .unwrap_or_else(|| "Three.js has no verified LUX harness yet".to_string()),
            false,
            false,
            false,
            false,
            three_js_blockers,
        )
    } else {
        EngineCapabilityRecord::unsupported(
            EngineKind::ThreeJs,
            "Three.js project markers not detected in project root.",
            "Missing package.json dependency on three",
            three_js_blockers,
        )
    };

    let active = match active_engine {
        EngineKind::Unity => &unity,
        EngineKind::Godot => &godot,
        EngineKind::ThreeJs => &three_js,
    };

    Ok(EngineCapabilitySnapshot {
        schema_version: CAPABILITY_SNAPSHOT_SCHEMA_VERSION.to_string(),
        generated_at,
        project_root,
        engine: active_engine,
        detected: active.detected,
        tool_available: active.tool_available,
        manual_qa_supported: active.manual_qa_supported,
        screenshot_supported: active.screenshot_supported,
        video_supported: active.video_supported,
        status: active.status,
        reason: active.reason.clone(),
        blocker_reason: active.blocker_reason.clone(),
        unity: unity.clone(),
        godot: godot.clone(),
        three_js: three_js.clone(),
        engines: vec![unity, godot, three_js],
    })
}

pub fn detect_three_js_project(project_root: &Path) -> Result<Option<String>> {
    let package_json = project_root.join("package.json");
    if !package_json.is_file() {
        return Ok(None);
    }

    let text = fs::read_to_string(&package_json)
        .with_context(|| format!("failed to read {}", package_json.display()))?;
    let manifest: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", package_json.display()))?;
    let dependency_sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];

    for section in dependency_sections {
        if manifest
            .get(section)
            .and_then(Value::as_object)
            .is_some_and(|dependencies| dependencies.contains_key("three"))
        {
            return Ok(Some(format!(
                "Three.js dependency detected in {}",
                package_json.display()
            )));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_engine_capability_snapshot, write_engine_capability_snapshot,
        EngineCapabilityStatus, CAPABILITY_SNAPSHOT_SCHEMA_VERSION,
    };
    use std::fs;

    #[test]
    fn snapshot_includes_three_engine_records() {
        let temp = tempfile::tempdir().expect("tempdir");

        let snapshot =
            collect_engine_capability_snapshot(temp.path(), lux_project::EngineKind::Godot)
                .expect("snapshot");

        assert_eq!(snapshot.schema_version, CAPABILITY_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(snapshot.engines.len(), 3);
        assert!(snapshot
            .engines
            .iter()
            .any(|engine| engine.engine == lux_project::EngineKind::Unity));
        assert!(snapshot
            .engines
            .iter()
            .any(|engine| engine.engine == lux_project::EngineKind::Godot));
        assert!(snapshot
            .engines
            .iter()
            .any(|engine| engine.engine == lux_project::EngineKind::ThreeJs));
    }

    #[test]
    fn snapshot_detects_unity_project_markers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_root = temp.path();
        fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
        fs::write(
            project_root.join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 6000.0.0f1\n",
        )
        .expect("write version");

        let snapshot =
            collect_engine_capability_snapshot(project_root, lux_project::EngineKind::Unity)
                .expect("snapshot");
        let unity = snapshot
            .engines
            .iter()
            .find(|engine| engine.engine == lux_project::EngineKind::Unity)
            .expect("unity record");

        assert!(unity.detected);
        assert!(unity.tool_available);
        assert!(unity.manual_qa_supported);
        assert!(unity.screenshot_supported);
        assert_eq!(unity.status, EngineCapabilityStatus::Detected);
        assert!(unity.blocker_reason.is_none());
    }

    #[test]
    fn snapshot_writes_capabilities_json() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_root = temp.path();
        fs::create_dir_all(project_root.join("ProjectSettings")).expect("create ProjectSettings");
        fs::write(
            project_root.join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 6000.0.0f1\n",
        )
        .expect("write version");

        let snapshot =
            write_engine_capability_snapshot(project_root, lux_project::EngineKind::Unity)
                .expect("snapshot");
        let capabilities_path = project_root.join(".lux/engines/capabilities.json");
        let payload: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&capabilities_path)
                .expect("persisted capabilities json should be readable"),
        )
        .expect("persisted capabilities json should parse");

        assert_eq!(snapshot.schema_version, CAPABILITY_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(payload["engine"], "unity");
        assert_eq!(payload["status"], "detected");
        assert!(payload["engines"].is_array());
        assert_eq!(payload["engines"][0]["engine"], "unity");
        assert_eq!(payload["engines"][0]["status"], "detected");
        assert_eq!(payload["engines"][1]["engine"], "godot");
        assert_eq!(payload["engines"][1]["status"], "unsupported");
        assert_eq!(payload["engines"][2]["engine"], "three_js");
        assert_eq!(payload["engines"][2]["status"], "unsupported");
    }
}
