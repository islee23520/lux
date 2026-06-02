mod capability;
mod detection;

pub use capability::{
    detect_engine_capabilities, persist_engine_capabilities, persist_engine_status_snapshot,
    recommended_capability_blockers, CapabilityStatus, EngineCapability, EngineCapabilityBlocker,
    EngineCapabilityCatalog, EngineCapabilityInventory, EngineCapabilityRecord,
    EngineCapabilityStatus, EngineKind, ParseCapabilityError,
};
pub use detection::{
    detect_from_cwd, detect_from_path, detect_godot_project, detect_unity_project, DetectedPackage,
    GodotProjectDetection, ProjectInfo, UnityProjectDetection,
};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-project");
    }
}
