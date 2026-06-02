use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameContextEngine {
    Unity,
    Godot,
    ThreeJs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineObservationCapability {
    Supported,
    Unsupported,
    Blocker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameContextRefs {
    pub spec_ref: String,
    pub ticket_ref: Option<String>,
    pub run_evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSources {
    pub scene_hierarchy: String,
    pub selected_object: String,
    pub component_properties: String,
    pub transform: String,
    pub rect_transform: String,
    pub collider: String,
    pub camera_state: String,
    pub ui_coordinates: String,
    pub console_logs: String,
    pub compile_logs: String,
    pub playmode_state: String,
    pub input_trace: String,
    pub screenshot_refs: String,
    pub vision_annotations: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneHierarchyNode {
    pub path: String,
    pub active: bool,
    pub component_types: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentPropertySnapshot {
    pub game_object_path: String,
    pub component_type: String,
    pub property_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformSnapshot {
    pub game_object_path: String,
    pub position_xyz: [f64; 3],
    pub rotation_euler_xyz: [f64; 3],
    pub scale_xyz: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectTransformSnapshot {
    pub game_object_path: String,
    pub anchored_position_xy: [f64; 2],
    pub size_delta_xy: [f64; 2],
    pub anchor_min_xy: [f64; 2],
    pub anchor_max_xy: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColliderSnapshot {
    pub game_object_path: String,
    pub collider_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraState {
    pub name: String,
    pub screen_size_xy: [f64; 2],
    pub world_position_xyz: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiCoordinateState {
    pub element_path: String,
    pub screen_position_xy: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityBlocker {
    pub engine: GameContextEngine,
    pub reason: String,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameContextObservation {
    pub schema_version: u32,
    pub engine: GameContextEngine,
    pub capability_status: EngineObservationCapability,
    pub refs: GameContextRefs,
    pub sources: ObservationSources,
    pub scene_hierarchy: Vec<SceneHierarchyNode>,
    pub selected_object_path: Option<String>,
    pub components: Vec<ComponentPropertySnapshot>,
    pub transforms: Vec<TransformSnapshot>,
    pub rect_transforms: Vec<RectTransformSnapshot>,
    pub colliders: Vec<ColliderSnapshot>,
    pub camera_state: Option<CameraState>,
    pub ui_coordinates: Vec<UiCoordinateState>,
    pub console_logs: Vec<String>,
    pub compile_logs: Vec<String>,
    pub playmode_state: Option<String>,
    pub input_trace_refs: Vec<String>,
    pub screenshot_refs: Vec<String>,
    pub vision_annotations: Vec<String>,
    pub capability_blockers: Vec<CapabilityBlocker>,
}
