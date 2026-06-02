use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnityCoordinateFrame {
    World,
    Local,
    Screen,
    Viewport,
    UiCanvas,
    Input,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityCoordinateMapping {
    pub frame: UnityCoordinateFrame,
    pub units: String,
    pub origin: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl UnityCoordinateMapping {
    pub fn new(frame: UnityCoordinateFrame, units: impl Into<String>) -> Self {
        Self {
            frame,
            units: units.into(),
            origin: default_origin_for(frame).to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateMappingPayload {
    pub node_id: String,
    pub mappings: Vec<UnityCoordinateMapping>,
}

const fn default_origin_for(frame: UnityCoordinateFrame) -> &'static str {
    match frame {
        UnityCoordinateFrame::World => "unity_world",
        UnityCoordinateFrame::Local => "parent_local",
        UnityCoordinateFrame::Screen => "screen_bottom_left",
        UnityCoordinateFrame::Viewport => "viewport_bottom_left",
        UnityCoordinateFrame::UiCanvas => "canvas_bottom_left",
        UnityCoordinateFrame::Input => "input_screen",
    }
}
