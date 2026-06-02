use lux_bridge_core::{
    BridgeProtocolRequest, BridgeProtocolResponse, BridgeResponsePayload, UnityAstNode,
    UnityAstReadResult, UnityAstScene, UnityAstSelectionAstPayload, CMD_GET_SCENE_AST,
    CMD_GET_SELECTION_AST, CMD_READ_ASSET_AST, CRATE_NAME, PROTOCOL_VERSION, SCHEMA_VERSION,
};

#[test]
fn crate_name_matches_package_when_bootstrapped() {
    assert_eq!(CRATE_NAME, "lux-bridge-core");
}

#[test]
fn ast_payload_round_trip() {
    let request = BridgeProtocolRequest::read_asset_ast("req-1", "token-1", "Assets/Prefab.prefab");
    assert_eq!(request.schema_version, SCHEMA_VERSION);
    assert_eq!(request.command, CMD_READ_ASSET_AST);
    assert_eq!(
        BridgeProtocolRequest::get_selection_ast("req-2", "token-1").command,
        CMD_GET_SELECTION_AST
    );
    assert_eq!(
        BridgeProtocolRequest::get_scene_ast("req-3", "token-1").command,
        CMD_GET_SCENE_AST
    );

    let node = UnityAstNode {
        id: "node-1".to_string(),
        stable_id: None,
        hierarchy_path: None,
        name: "Player".to_string(),
        active_self: true,
        layer: 0,
        tag: "Player".to_string(),
        coordinate_mappings: Vec::new(),
        components: Vec::new(),
        children: Vec::new(),
    };
    let response = BridgeProtocolResponse::ok(
        "req-1",
        BridgeResponsePayload {
            asset_ast: Some(UnityAstReadResult {
                schema_version: SCHEMA_VERSION,
                protocol_version: PROTOCOL_VERSION.to_string(),
                asset_path: "Assets/Prefab.prefab".to_string(),
                asset_type: "Prefab".to_string(),
                ast: node,
                file_size_bytes: 128,
                ast_node_count: 1,
            }),
            selection_ast: Some(UnityAstSelectionAstPayload {
                schema_version: SCHEMA_VERSION,
                protocol_version: PROTOCOL_VERSION.to_string(),
                selection_count: 0,
                selections: Vec::new(),
            }),
            scene_ast: Some(UnityAstScene {
                schema_version: SCHEMA_VERSION,
                protocol_version: PROTOCOL_VERSION.to_string(),
                captured_at_utc: "2026-06-01T00:00:00Z".to_string(),
                scene_name: "SampleScene".to_string(),
                scene_path: "Assets/SampleScene.unity".to_string(),
                root_count: 0,
                total_game_objects: 0,
                total_components: 0,
                roots: Vec::new(),
            }),
        },
    );

    assert!(response.ok);
    assert_eq!(CMD_GET_SELECTION_AST, "get_selection_ast");
    assert_eq!(CMD_GET_SCENE_AST, "get_scene_ast");
}

#[test]
fn old_ast_response_without_payload_versions_parses() {
    let raw = serde_json::json!({
        "schemaVersion": 1,
        "requestId": "req-legacy",
        "ok": true,
        "payload": {
            "assetAst": {
                "assetPath": "Assets/Prefab.prefab",
                "assetType": "Prefab",
                "ast": {
                    "id": "node-1",
                    "name": "Player",
                    "activeSelf": true,
                    "layer": 0,
                    "tag": "Player",
                    "components": [],
                    "children": []
                },
                "fileSizeBytes": 128,
                "astNodeCount": 1
            }
        },
        "capturedAtUtc": "2026-06-01T00:00:00Z"
    });

    let response: BridgeProtocolResponse =
        serde_json::from_value(raw).expect("legacy bridge AST response should parse");
    let asset = response
        .payload
        .and_then(|payload| payload.asset_ast)
        .expect("asset AST should be present");

    assert_eq!(asset.schema_version, SCHEMA_VERSION);
    assert_eq!(asset.protocol_version, PROTOCOL_VERSION);
    assert_eq!(asset.ast.name, "Player");
    assert!(asset.ast.stable_id.is_none());
    assert!(asset.ast.coordinate_mappings.is_empty());
}
