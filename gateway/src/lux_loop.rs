use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::lux_ai_session::{self, AiSession};
use crate::lux_ambiguity::{self, AmbiguityReport};
use crate::lux_build::{self, BuildManager, BuildTarget};
use crate::lux_events::{EventRouter, LuxEvent};
use crate::lux_spec::{self, SpecProject};
use crate::lux_ticket::{
    FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus, TicketStore,
};
use crate::lux_verification::{self, VerificationResult};

pub const DEFAULT_MAX_ITERATIONS: u32 = 10;
// Ambiguity polarity: 0.0 = fully clear, 1.0 = maximally ambiguous
pub const DEFAULT_AMBIGUITY_THRESHOLD: f64 = 0.02;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopState {
    Idle,
    Analyzing,
    SpecRefining,
    Building,
    AwaitingPlay,
    CollectingFeedback,
    Updating,
    Paused(Box<LoopState>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalGate {
    BeginAnalysis,
    RefineSpec,
    StartBuild,
    StartPlay,
    CollectFeedback,
    UpdateSpec,
    CompleteIteration,
    MilestonePush,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoopSnapshot {
    pub state: LoopState,
    pub project_path: PathBuf,
    pub iteration: u32,
    pub max_iterations: u32,
    pub requires_user_approval: bool,
    pub approval_gate: Option<ApprovalGate>,
    pub pending_state: Option<LoopState>,
    pub last_error: Option<String>,
    pub last_verification: Option<VerificationResult>,
    pub last_ambiguity: Option<AmbiguityReport>,
    pub active_ai_session: Option<AiSession>,
    pub active_build_id: Option<String>,
    pub feedback_count: usize,
}

pub struct LoopOrchestrator {
    state: LoopState,
    project_path: PathBuf,
    iteration: u32,
    max_iterations: u32,
    event_router: EventRouter,
    ambiguity_threshold: f64,
    approval_gate: Option<ApprovalGate>,
    pending_state: Option<LoopState>,
    last_error: Option<String>,
    last_verification: Option<VerificationResult>,
    last_ambiguity: Option<AmbiguityReport>,
    active_ai_session: Option<AiSession>,
    active_build_id: Option<String>,
    collected_feedback: Vec<Value>,
}

impl LoopOrchestrator {
    pub fn new(project_path: impl Into<PathBuf>, event_router: EventRouter) -> Self {
        Self::with_max_iterations(project_path, DEFAULT_MAX_ITERATIONS, event_router)
    }

    pub fn with_max_iterations(
        project_path: impl Into<PathBuf>,
        max_iterations: u32,
        event_router: EventRouter,
    ) -> Self {
        Self {
            state: LoopState::Idle,
            project_path: project_path.into(),
            iteration: 0,
            max_iterations: max_iterations.max(1),
            event_router,
            ambiguity_threshold: DEFAULT_AMBIGUITY_THRESHOLD,
            approval_gate: None,
            pending_state: None,
            last_error: None,
            last_verification: None,
            last_ambiguity: None,
            active_ai_session: None,
            active_build_id: None,
            collected_feedback: Vec::new(),
        }
    }

    pub fn register_handler(
        &mut self,
        event_type: &str,
        handler: Box<dyn Fn(&LuxEvent) + Send + Sync>,
    ) {
        self.event_router.register(event_type, handler);
    }

    pub fn state(&self) -> &LoopState {
        &self.state
    }

    pub fn snapshot(&self) -> LoopSnapshot {
        LoopSnapshot {
            state: self.state.clone(),
            project_path: self.project_path.clone(),
            iteration: self.iteration,
            max_iterations: self.max_iterations,
            requires_user_approval: self.requires_user_approval(),
            approval_gate: self.approval_gate.clone(),
            pending_state: self.pending_state.clone(),
            last_error: self.last_error.clone(),
            last_verification: self.last_verification.clone(),
            last_ambiguity: self.last_ambiguity.clone(),
            active_ai_session: self.active_ai_session.clone(),
            active_build_id: self.active_build_id.clone(),
            feedback_count: self.collected_feedback.len(),
        }
    }

    pub fn requires_user_approval(&self) -> bool {
        self.pending_state.is_some() || self.approval_gate.is_some()
    }

    pub fn start(&mut self) -> Result<LoopSnapshot> {
        match self.state {
            LoopState::Idle => {
                self.ensure_can_iterate()?;
                self.last_error = None;
                self.transition_to(LoopState::Analyzing);
                self.run_analysis()?;
                Ok(self.snapshot())
            }
            LoopState::Paused(_) => bail!("Lux loop is paused; resume before starting"),
            _ => bail!("Lux loop already started"),
        }
    }

    pub fn approve_next(&mut self) -> Result<LoopSnapshot> {
        if matches!(self.state, LoopState::Paused(_)) {
            bail!("Lux loop is paused; resume before approving the next step");
        }
        let next = self
            .pending_state
            .take()
            .ok_or_else(|| anyhow!("Lux loop has no pending approval gate"))?;
        self.approval_gate = None;
        self.transition_to(next.clone());
        match next {
            LoopState::SpecRefining => self.begin_spec_refinement()?,
            LoopState::Building => self.begin_build()?,
            LoopState::AwaitingPlay => self.await_play(),
            LoopState::CollectingFeedback => self.begin_feedback_collection(),
            LoopState::Updating => self.update_specs()?,
            LoopState::Idle => {}
            LoopState::Analyzing | LoopState::Paused(_) => {}
        }
        Ok(self.snapshot())
    }

    pub fn pause(&mut self) -> Result<LoopSnapshot> {
        if matches!(self.state, LoopState::Paused(_)) {
            return Ok(self.snapshot());
        }
        let previous = self.state.clone();
        self.transition_to(LoopState::Paused(Box::new(previous)));
        Ok(self.snapshot())
    }

    pub fn resume(&mut self) -> Result<LoopSnapshot> {
        let LoopState::Paused(previous) = self.state.clone() else {
            return Ok(self.snapshot());
        };
        self.transition_to(*previous);
        Ok(self.snapshot())
    }

    pub fn record_play_started(&mut self) -> Result<LoopSnapshot> {
        if !matches!(
            self.state,
            LoopState::AwaitingPlay | LoopState::CollectingFeedback
        ) {
            bail!("Lux loop is not awaiting play");
        }
        self.approval_gate = None;
        self.pending_state = None;
        if self.state != LoopState::CollectingFeedback {
            self.transition_to(LoopState::CollectingFeedback);
        }
        Ok(self.snapshot())
    }

    pub fn record_feedback(&mut self, feedback: &Value) -> Result<LoopSnapshot> {
        if self.state != LoopState::CollectingFeedback {
            bail!("Lux loop is not collecting feedback");
        }
        self.collected_feedback.push(feedback.clone());
        self.complete_iteration()?;
        Ok(self.snapshot())
    }

    pub fn request_milestone_push_approval(&mut self) -> LoopSnapshot {
        self.approval_gate = Some(ApprovalGate::MilestonePush);
        self.pending_state = Some(LoopState::Idle);
        self.snapshot()
    }

    fn run_analysis(&mut self) -> Result<()> {
        let verification = lux_verification::verify_all(
            &self.project_path,
            lux_verification::VerificationMode::Cached,
        )
        .with_context(|| format!("failed to verify {}", self.project_path.display()))?;
        let spec = lux_spec::lux_load(&self.project_path).with_context(|| {
            format!(
                "failed to load Lux spec from {}",
                self.project_path.display()
            )
        })?;
        let ambiguity = lux_ambiguity::calculate_ambiguity(&spec);
        let needs_refinement =
            ambiguity.overall_score > self.ambiguity_threshold || !verification.passed;
        self.last_verification = Some(verification);
        self.last_ambiguity = Some(ambiguity);
        if needs_refinement {
            self.request_approval(ApprovalGate::RefineSpec, LoopState::SpecRefining);
        } else {
            self.request_approval(ApprovalGate::StartBuild, LoopState::Building);
        }
        Ok(())
    }

    fn begin_spec_refinement(&mut self) -> Result<()> {
        let session = lux_ai_session::create_session(&self.project_path).with_context(|| {
            format!(
                "failed to create AI session for {}",
                self.project_path.display()
            )
        })?;
        self.active_ai_session = Some(session);
        self.generate_kanban_tickets()?;
        self.request_approval(ApprovalGate::StartBuild, LoopState::Building);
        Ok(())
    }

    fn begin_build(&mut self) -> Result<()> {
        self.ensure_can_iterate()?;
        let mut manager = BuildManager::with_project_root(Some(&self.project_path));
        let build_id =
            lux_build::start_build(&mut manager, &self.project_path, BuildTarget::WebGL)?;
        self.active_build_id = Some(build_id);
        self.request_approval(ApprovalGate::StartPlay, LoopState::AwaitingPlay);
        Ok(())
    }

    fn await_play(&mut self) {
        self.approval_gate = Some(ApprovalGate::StartPlay);
        self.pending_state = Some(LoopState::CollectingFeedback);
    }

    fn begin_feedback_collection(&mut self) {
        self.approval_gate = None;
        self.pending_state = None;
    }

    fn update_specs(&mut self) -> Result<()> {
        let mut spec = lux_spec::lux_load(&self.project_path).with_context(|| {
            format!(
                "failed to load Lux spec from {}",
                self.project_path.display()
            )
        })?;
        spec.updated_at = Utc::now().to_rfc3339();
        spec.source = "lux-loop".to_string();
        lux_spec::lux_save(&self.project_path, &spec)?;
        self.complete_iteration()?;
        Ok(())
    }

    fn complete_iteration(&mut self) -> Result<()> {
        self.iteration = self.iteration.saturating_add(1);
        self.ensure_can_iterate_or_idle()?;
        self.active_ai_session = None;
        self.active_build_id = None;
        self.collected_feedback.clear();
        self.approval_gate = None;
        self.pending_state = None;
        if self.iteration < self.max_iterations {
            self.request_approval(ApprovalGate::CompleteIteration, LoopState::Idle);
        } else {
            self.transition_to(LoopState::Idle);
        }
        Ok(())
    }

    fn generate_kanban_tickets(&self) -> Result<Vec<Ticket>> {
        let store = FileTicketStore::new(&self.project_path);
        let existing = store.list(TicketFilter::default())?;
        if existing
            .iter()
            .any(|ticket| ticket.tags.iter().any(|tag| tag == "lux-loop"))
        {
            return Ok(existing
                .into_iter()
                .filter(|ticket| ticket.tags.iter().any(|tag| tag == "lux-loop"))
                .collect());
        }
        let mut created = Vec::new();
        let now = Utc::now().to_rfc3339();
        // NOTE: These are placeholder tickets for MVP sequential validation.
        // Future replacement should derive tickets from spec analysis via ADR-003 domain separation.
        let tickets = [
            (
                "Refine Lux game specification",
                "Resolve ambiguity before the next build.",
                TicketPriority::High,
                Some(".lux/spec.json"),
            ),
            (
                "Build playable WebGL iteration",
                "Run the generated spec through the Lux build pipeline.",
                TicketPriority::High,
                None,
            ),
            (
                "Collect playtest feedback",
                "Capture player observations before updating the spec.",
                TicketPriority::Medium,
                Some(".lux/logs"),
            ),
        ];
        for (title, description, priority, spec_ref) in tickets {
            let ticket = Ticket {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                description: description.to_string(),
                status: TicketStatus::ToDo,
                priority,
                assignee: None,
                blockers: Vec::new(),
                tags: vec!["lux-loop".to_string()],
                spec_ref: spec_ref.map(str::to_string),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            created.push(store.create(ticket)?);
        }
        Ok(created)
    }

    fn ensure_can_iterate(&self) -> Result<()> {
        if self.iteration >= self.max_iterations {
            bail!("Lux loop reached max iterations ({})", self.max_iterations);
        }
        Ok(())
    }

    fn ensure_can_iterate_or_idle(&self) -> Result<()> {
        if self.iteration > self.max_iterations {
            bail!("Lux loop exceeded max iterations ({})", self.max_iterations);
        }
        Ok(())
    }

    fn request_approval(&mut self, gate: ApprovalGate, next_state: LoopState) {
        self.approval_gate = Some(gate);
        self.pending_state = Some(next_state);
    }

    fn transition_to(&mut self, next_state: LoopState) {
        let previous_state = self.state.clone();
        self.state = next_state.clone();
        let event = LuxEvent::LoopStateChange {
            previous_state: state_label(&previous_state).to_string(),
            current_state: state_label(&next_state).to_string(),
            iteration: self.iteration,
            max_iterations: self.max_iterations,
            requires_user_approval: self.requires_user_approval(),
        };
        self.event_router.route(&event);
    }
}

pub fn state_label(state: &LoopState) -> &'static str {
    match state {
        LoopState::Idle => "Idle",
        LoopState::Analyzing => "Analyzing",
        LoopState::SpecRefining => "SpecRefining",
        LoopState::Building => "Building",
        LoopState::AwaitingPlay => "AwaitingPlay",
        LoopState::CollectingFeedback => "CollectingFeedback",
        LoopState::Updating => "Updating",
        LoopState::Paused(_) => "Paused",
    }
}

pub fn load_or_init_spec(project_path: &Path) -> Result<SpecProject> {
    match lux_spec::lux_load(project_path) {
        Ok(spec) => Ok(spec),
        Err(_) => {
            lux_spec::lux_init(project_path)?;
            lux_spec::lux_load(project_path)
        }
    }
}

pub fn event_payload(event: &LuxEvent) -> Value {
    match event {
        LuxEvent::LoopStateChange {
            previous_state,
            current_state,
            iteration,
            max_iterations,
            requires_user_approval,
        } => json!({
            "type": "loop:state_change",
            "previousState": previous_state,
            "currentState": current_state,
            "iteration": iteration,
            "maxIterations": max_iterations,
            "requiresUserApproval": requires_user_approval,
        }),
        other => json!({ "type": other.event_type() }),
    }
}
