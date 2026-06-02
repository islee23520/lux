use serde::{Deserialize, Serialize};

use crate::ast::{UnityAstReadResult, UnityAstScene, UnityAstSelectionAstPayload};
use crate::{
    default_schema_version, CMD_GET_SCENE_AST, CMD_GET_SELECTION_AST, CMD_READ_ASSET_AST,
    SCHEMA_VERSION,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeProtocolRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub request_id: String,
    pub command: String,
    pub token: String,
    #[serde(rename = "params", default, skip_serializing_if = "Option::is_none")]
    pub params: Option<BridgeRequestParams>,
}

impl BridgeProtocolRequest {
    pub fn read_asset_ast(
        request_id: impl Into<String>,
        token: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: request_id.into(),
            command: CMD_READ_ASSET_AST.to_string(),
            token: token.into(),
            params: Some(BridgeRequestParams {
                asset_path: Some(asset_path.into()),
                ..BridgeRequestParams::default()
            }),
        }
    }

    pub fn get_selection_ast(request_id: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: request_id.into(),
            command: CMD_GET_SELECTION_AST.to_string(),
            token: token.into(),
            params: Some(BridgeRequestParams::default()),
        }
    }

    pub fn get_scene_ast(request_id: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: request_id.into(),
            command: CMD_GET_SCENE_AST.to_string(),
            token: token.into(),
            params: Some(BridgeRequestParams::default()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeRequestParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_depth: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_include_components: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_root_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_options: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeProtocolResponse {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub request_id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<BridgeResponsePayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at_utc: Option<String>,
}

impl BridgeProtocolResponse {
    pub fn ok(request_id: impl Into<String>, payload: BridgeResponsePayload) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: request_id.into(),
            ok: true,
            error_code: None,
            error_message: None,
            payload: Some(payload),
            captured_at_utc: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResponsePayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_ast: Option<UnityAstReadResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_ast: Option<UnityAstSelectionAstPayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_ast: Option<UnityAstScene>,
}
