use std::path::{Path, PathBuf};

use anyhow::Result;

pub use lux_project::{
    detect_from_cwd, detect_from_path, detect_unity_project, DetectedPackage, ProjectInfo,
    UnityProjectDetection,
};

use crate::lux_engines::{self, EngineCapabilityRecord};
use lux_project::EngineKind;

#[derive(Clone, Debug, PartialEq)]
pub struct EngineCapabilityInventory {
    pub path: PathBuf,
    pub engines: Vec<EngineCapabilityRecord>,
}

pub fn detect_engine_capabilities(project_root: &Path) -> Result<EngineCapabilityInventory> {
    let active_engine = active_engine_for_project(project_root);
    let snapshot = lux_engines::write_engine_capability_snapshot(project_root, active_engine)?;
    Ok(EngineCapabilityInventory {
        path: project_root.join(".lux/engines/capabilities.json"),
        engines: snapshot.engines,
    })
}

fn active_engine_for_project(project_root: &Path) -> EngineKind {
    if detect_unity_project(project_root).ok().flatten().is_some() {
        return EngineKind::Unity;
    }
    if lux_project::detect_godot_project(project_root).is_some() {
        return EngineKind::Godot;
    }
    if lux_engines::detect_three_js_project(project_root)
        .ok()
        .flatten()
        .is_some()
    {
        return EngineKind::ThreeJs;
    }
    EngineKind::Unity
}
