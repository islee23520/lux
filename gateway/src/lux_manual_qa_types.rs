use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualQaEngine {
    Unity,
    Godot,
    ThreeJs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualQaPhase {
    Compile,
    Test,
    DynamicCode,
    Screenshot,
    DevServer,
    BrowserScreenshot,
    GodotVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualQaCapabilityStatus {
    Supported,
    Unsupported,
    Blocker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualQaCapabilities {
    pub screenshot: ManualQaCapabilityStatus,
    pub video: ManualQaCapabilityStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualQaCommand {
    pub phase: ManualQaPhase,
    pub command: String,
}

impl ManualQaCommand {
    pub fn new(phase: ManualQaPhase, command: impl Into<String>) -> Self {
        Self {
            phase,
            command: command.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualQaEvidenceRequest {
    pub engine: ManualQaEngine,
    pub run_id: String,
    pub project_path: PathBuf,
    pub evidence_dir: PathBuf,
    pub commands: Vec<ManualQaCommand>,
    pub capabilities: ManualQaCapabilities,
    pub godot_cli: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualQaStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualQaEvidenceResult {
    pub status: ManualQaStatus,
    pub engine: ManualQaEngine,
    pub evidence_paths: Vec<String>,
    pub capabilities: ManualQaCapabilities,
}
