mod ast;
mod coordinate;
mod protocol;

pub use ast::{
    UnityAstComponent, UnityAstNode, UnityAstProperty, UnityAstReadResult, UnityAstScene,
    UnityAstSelectionAstPayload,
};
pub use coordinate::{CoordinateMappingPayload, UnityCoordinateFrame, UnityCoordinateMapping};
pub use protocol::{
    BridgeProtocolRequest, BridgeProtocolResponse, BridgeRequestParams, BridgeResponsePayload,
};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const SCHEMA_VERSION: u32 = 1;
pub const PROTOCOL_VERSION: &str = "1";
pub const CMD_READ_ASSET_AST: &str = "read_asset_ast";
pub const CMD_GET_SELECTION_AST: &str = "get_selection_ast";
pub const CMD_GET_SCENE_AST: &str = "get_scene_ast";

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

fn default_protocol_version() -> String {
    PROTOCOL_VERSION.to_string()
}
