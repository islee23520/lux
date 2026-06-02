use lux_bridge_core::{
    CoordinateMappingPayload, UnityAstNode, UnityCoordinateFrame, UnityCoordinateMapping,
};

#[test]
fn coordinate_mapping_round_trip() {
    let payload = CoordinateMappingPayload {
        node_id: "scene-root/player".to_string(),
        mappings: vec![
            UnityCoordinateMapping::new(UnityCoordinateFrame::World, "meters"),
            UnityCoordinateMapping::new(UnityCoordinateFrame::Local, "meters"),
            UnityCoordinateMapping::new(UnityCoordinateFrame::Screen, "pixels"),
            UnityCoordinateMapping::new(UnityCoordinateFrame::Viewport, "normalized"),
            UnityCoordinateMapping::new(UnityCoordinateFrame::UiCanvas, "canvas_units"),
            UnityCoordinateMapping::new(UnityCoordinateFrame::Input, "screen_pixels"),
        ],
    };

    let json = serde_json::to_string(&payload).expect("coordinate mapping should serialize");
    let loaded: CoordinateMappingPayload =
        serde_json::from_str(&json).expect("coordinate mapping should deserialize");

    let frames: Vec<UnityCoordinateFrame> = loaded
        .mappings
        .iter()
        .map(|mapping| mapping.frame)
        .collect();
    assert_eq!(
        frames,
        vec![
            UnityCoordinateFrame::World,
            UnityCoordinateFrame::Local,
            UnityCoordinateFrame::Screen,
            UnityCoordinateFrame::Viewport,
            UnityCoordinateFrame::UiCanvas,
            UnityCoordinateFrame::Input,
        ]
    );
}

#[test]
fn ast_node_identity_contract_round_trip() {
    let raw = serde_json::json!({
        "id": "g0",
        "stableId": "scene-root/player",
        "hierarchyPath": "/SceneRoot/Player",
        "name": "Player",
        "activeSelf": true,
        "layer": 0,
        "tag": "Player",
        "coordinateMappings": [
            {
                "frame": "world",
                "units": "meters",
                "origin": "unity_world",
                "x": 1.0,
                "y": 2.0,
                "z": 3.0
            },
            {
                "frame": "ui_canvas",
                "units": "canvas_units",
                "origin": "canvas_bottom_left",
                "x": 10.0,
                "y": 20.0,
                "z": 0.0
            }
        ],
        "components": [],
        "children": []
    });

    let node: UnityAstNode =
        serde_json::from_value(raw).expect("node with identity contract should parse");

    assert_eq!(node.id, "g0");
    assert_eq!(node.stable_id.as_deref(), Some("scene-root/player"));
    assert_eq!(node.hierarchy_path.as_deref(), Some("/SceneRoot/Player"));
    assert_eq!(node.coordinate_mappings.len(), 2);
    assert_eq!(
        node.coordinate_mappings[1].frame,
        UnityCoordinateFrame::UiCanvas
    );
}
