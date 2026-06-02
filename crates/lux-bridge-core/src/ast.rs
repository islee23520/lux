use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::coordinate::UnityCoordinateMapping;
use crate::{default_protocol_version, default_schema_version};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstReadResult {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    pub asset_path: String,
    pub asset_type: String,
    pub ast: UnityAstNode,
    pub file_size_bytes: i64,
    pub ast_node_count: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstSelectionAstPayload {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    pub selection_count: i32,
    pub selections: Vec<UnityAstNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstScene {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    pub captured_at_utc: String,
    pub scene_name: String,
    pub scene_path: String,
    pub root_count: i32,
    pub total_game_objects: i32,
    pub total_components: i32,
    pub roots: Vec<UnityAstNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy_path: Option<String>,
    pub name: String,
    pub active_self: bool,
    pub layer: i32,
    pub tag: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coordinate_mappings: Vec<UnityCoordinateMapping>,
    pub components: Vec<UnityAstComponent>,
    pub children: Vec<UnityAstNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstComponent {
    #[serde(rename = "type")]
    pub component_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_guid: Option<String>,
    pub enabled: bool,
    pub properties: Vec<UnityAstProperty>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityAstProperty {
    pub key: String,
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub int_value: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub float_value: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bool_value: Option<bool>,
    #[serde(rename = "v3_x", default, skip_serializing_if = "Option::is_none")]
    pub v3_x: Option<f32>,
    #[serde(rename = "v3_y", default, skip_serializing_if = "Option::is_none")]
    pub v3_y: Option<f32>,
    #[serde(rename = "v3_z", default, skip_serializing_if = "Option::is_none")]
    pub v3_z: Option<f32>,
    #[serde(rename = "q_x", default, skip_serializing_if = "Option::is_none")]
    pub q_x: Option<f32>,
    #[serde(rename = "q_qy", default, skip_serializing_if = "Option::is_none")]
    pub q_qy: Option<f32>,
    #[serde(rename = "q_z", default, skip_serializing_if = "Option::is_none")]
    pub q_z: Option<f32>,
    #[serde(rename = "q_w", default, skip_serializing_if = "Option::is_none")]
    pub q_w: Option<f32>,
    #[serde(rename = "c_r", default, skip_serializing_if = "Option::is_none")]
    pub c_r: Option<u8>,
    #[serde(rename = "c_g", default, skip_serializing_if = "Option::is_none")]
    pub c_g: Option<u8>,
    #[serde(rename = "c_b", default, skip_serializing_if = "Option::is_none")]
    pub c_b: Option<u8>,
    #[serde(rename = "c_a", default, skip_serializing_if = "Option::is_none")]
    pub c_a: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_value: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}
