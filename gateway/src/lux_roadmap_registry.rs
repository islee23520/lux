use std::collections::HashMap;

use crate::lux_roadmap::{RoadmapPhase, RoadmapPhaseStatus, REMOTE_WEBRTC_EXPERIMENTAL_FLAG};
use lux_project::EngineKind;

pub const GLOBAL_CLI_MCP_BRIDGE_PHASE: &str = "M7: Global CLI MCP Bridge Workflow";
pub const GODOT_GLOBAL_CLI_MCP_BRIDGE_PHASE: &str = "M7: Godot Global CLI MCP Bridge Workflow";
pub const THREE_JS_GLOBAL_CLI_MCP_BRIDGE_PHASE: &str =
    "M7: Three.js Global CLI MCP Bridge Workflow";
pub const CAPABILITY_GLOBAL_CLI: &str = "global_cli";
pub const CAPABILITY_MCP_STDIO: &str = "mcp_stdio";
pub const CAPABILITY_UNITY_BRIDGE_WORKFLOW: &str = "unity_bridge_workflow";
pub const CAPABILITY_GODOT_BRIDGE_WORKFLOW: &str = "godot_bridge_workflow_partial";
pub const CAPABILITY_THREE_JS_BRIDGE_WORKFLOW: &str = "three_js_bridge_workflow_planned";

pub fn default_phases() -> Vec<RoadmapPhase> {
    vec![
        planned_phase("M1: Canonical 9-Domain Schema & Defaults"),
        planned_phase("M2: Ambiguity Convergence & Socratic Loop"),
        planned_phase("M3: Execution-Grade Ticket Schema"),
        planned_phase("M4: Ticket-Driven OpenCode Hook Executor"),
        planned_phase("M5: Blocker Auto-Resolution Graph"),
        planned_phase(GLOBAL_CLI_MCP_BRIDGE_PHASE),
    ]
}

pub fn default_capabilities() -> Vec<String> {
    capabilities_for_engine(EngineKind::Unity)
}

pub fn default_experimental_flags() -> HashMap<String, bool> {
    HashMap::from([(REMOTE_WEBRTC_EXPERIMENTAL_FLAG.to_string(), false)])
}

pub fn phase_for_engine(engine: EngineKind) -> RoadmapPhase {
    match engine {
        EngineKind::Unity => planned_phase(GLOBAL_CLI_MCP_BRIDGE_PHASE),
        EngineKind::Godot => RoadmapPhase {
            name: GODOT_GLOBAL_CLI_MCP_BRIDGE_PHASE.to_string(),
            status: RoadmapPhaseStatus::Partial,
            evidence_path: Some("docs/godot-support.md".to_string()),
            pushed_at: None,
            push_git_sha: None,
            push_evidence_path: None,
        },
        EngineKind::ThreeJs => planned_phase(THREE_JS_GLOBAL_CLI_MCP_BRIDGE_PHASE),
    }
}

pub fn phases_for_engine(engine: EngineKind) -> Vec<RoadmapPhase> {
    let mut phases = default_phases();
    if let Some(phase) = phases
        .iter_mut()
        .find(|phase| phase.name == GLOBAL_CLI_MCP_BRIDGE_PHASE)
    {
        *phase = phase_for_engine(engine);
    }
    phases
}

pub fn capabilities_for_engine(engine: EngineKind) -> Vec<String> {
    let engine_capability = match engine {
        EngineKind::Unity => CAPABILITY_UNITY_BRIDGE_WORKFLOW,
        EngineKind::Godot => CAPABILITY_GODOT_BRIDGE_WORKFLOW,
        EngineKind::ThreeJs => CAPABILITY_THREE_JS_BRIDGE_WORKFLOW,
    };
    [
        CAPABILITY_GLOBAL_CLI,
        CAPABILITY_MCP_STDIO,
        engine_capability,
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn planned_phase(name: &str) -> RoadmapPhase {
    RoadmapPhase {
        name: name.to_string(),
        status: RoadmapPhaseStatus::Planned,
        evidence_path: None,
        pushed_at: None,
        push_git_sha: None,
        push_evidence_path: None,
    }
}
