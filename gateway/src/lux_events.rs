use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum LuxEvent {
    #[serde(rename = "spec:update")]
    SpecUpdate { domain: String, changes: Value },

    /// Emitted while spec generation is in progress to report ambiguity and coverage across domains.
    #[serde(rename = "spec:progress")]
    SpecProgress {
        overall_ambiguity: f64,
        domain_ambiguities: HashMap<String, f64>,
        domains_defined: u32,
        domains_total: u32,
        requirements_by_status: HashMap<String, u32>,
    },

    #[serde(rename = "kanban:update")]
    KanbanUpdate {
        ticket_id: String,
        new_status: String,
    },

    /// Emitted when kanban progress changes so dashboards can reflect status counts and the active ticket.
    #[serde(rename = "kanban:progress")]
    KanbanProgress {
        by_status: HashMap<String, u32>,
        total: u32,
        active_count: u32,
        changed_ticket_id: Option<String>,
    },

    #[serde(rename = "terminal:output")]
    TerminalOutput { session_id: String, data: String },

    #[serde(rename = "terminal:input")]
    TerminalInput { session_id: String, data: String },

    #[serde(rename = "build:progress")]
    BuildProgress {
        build_id: String,
        progress: f64,
        message: String,
    },

    #[serde(rename = "build:complete")]
    BuildComplete {
        build_id: String,
        success: bool,
        artifact_path: Option<String>,
    },

    #[serde(rename = "play:event")]
    PlayEvent { session_id: String, event: Value },

    #[serde(rename = "play:feedback")]
    PlayFeedback { session_id: String, feedback: Value },

    #[serde(rename = "ai:message")]
    AiMessage {
        session_id: String,
        message: String,
        phase: u8,
    },

    #[serde(rename = "ai:request_input")]
    AiRequestInput {
        session_id: String,
        prompt: String,
        phase: u8,
    },

    #[serde(rename = "verification:result")]
    VerificationResult { passed: bool, details: Value },

    #[serde(rename = "loop:state_change")]
    LoopStateChange {
        previous_state: String,
        current_state: String,
        iteration: u32,
        max_iterations: u32,
        requires_user_approval: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LuxEventMessage {
    pub event: LuxEvent,
    pub timestamp: String,
    pub source: String,
}

pub struct EventRouter {
    handlers: HashMap<String, Vec<Box<dyn Fn(&LuxEvent) + Send + Sync>>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, event_type: &str, handler: Box<dyn Fn(&LuxEvent) + Send + Sync>) {
        self.handlers
            .entry(event_type.to_string())
            .or_default()
            .push(handler);
    }

    pub fn route(&self, event: &LuxEvent) {
        if let Some(handlers) = self.handlers.get(event.event_type()) {
            for handler in handlers {
                handler(event);
            }
        }
    }

    pub fn serialize(&self, event: &LuxEvent) -> Result<String> {
        Ok(serde_json::to_string(event)?)
    }

    pub fn deserialize(&self, raw: &str) -> Result<LuxEvent> {
        Ok(serde_json::from_str(raw)?)
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LuxEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            LuxEvent::SpecUpdate { .. } => "spec:update",
            LuxEvent::SpecProgress { .. } => "spec:progress",
            LuxEvent::KanbanUpdate { .. } => "kanban:update",
            LuxEvent::KanbanProgress { .. } => "kanban:progress",
            LuxEvent::TerminalOutput { .. } => "terminal:output",
            LuxEvent::TerminalInput { .. } => "terminal:input",
            LuxEvent::BuildProgress { .. } => "build:progress",
            LuxEvent::BuildComplete { .. } => "build:complete",
            LuxEvent::PlayEvent { .. } => "play:event",
            LuxEvent::PlayFeedback { .. } => "play:feedback",
            LuxEvent::AiMessage { .. } => "ai:message",
            LuxEvent::AiRequestInput { .. } => "ai:request_input",
            LuxEvent::VerificationResult { .. } => "verification:result",
            LuxEvent::LoopStateChange { .. } => "loop:state_change",
        }
    }
}
