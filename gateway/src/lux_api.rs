//! Phase 3 — Gateway Single-Writer Mutation API
//!
//! All agent mutations to `.lux/` state MUST go through these endpoints.
//! Implements the gateway-as-single-writer architecture.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};

use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    lux_lock::{acquire_lux_lock, DEFAULT_STALE_THRESHOLD_SECS},
    lux_run_state::{RunState, RunStatus},
    lux_ticket::{
        FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus, TicketStore,
    },
    server::GatewayState,
};

// ---------------------------------------------------------------------------
// In-memory bridge lease registry (Phase 6 will persist to .lux/)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeLease {
    pub lease_id: String,
    pub agent_id: String,
    pub project_root: String,
    pub acquired_at: String,
    pub expires_at: Option<String>,
}

static BRIDGE_LEASES: LazyLock<Arc<Mutex<HashMap<String, BridgeLease>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

// ---------------------------------------------------------------------------
// Error helpers (mirrors server.rs patterns)
// ---------------------------------------------------------------------------

fn bad_request(error: anyhow::Error) -> Response {
    (StatusCode::BAD_REQUEST, error.to_string()).into_response()
}

fn internal_error(error: anyhow::Error) -> Response {
    tracing::warn!(%error, "lux_api request failed");
    (StatusCode::INTERNAL_SERVER_ERROR, "lux_api request failed").into_response()
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "resource not found").into_response()
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStateResponse {
    pub schema_version: u32,
    pub seq: u64,
    pub run_id: String,
    pub status: String,
    pub goal_id: Option<String>,
    pub milestone_id: Option<String>,
    pub ticket_id: Option<String>,
    pub current_ticket_id: Option<String>,
    pub executor: Option<String>,
    pub verification_policy: Option<String>,
    pub last_error: Option<String>,
    pub pre_task_git_sha: Option<String>,
    pub team_run_id: Option<String>,
    pub updated_at: String,
}

impl From<RunState> for RunStateResponse {
    fn from(s: RunState) -> Self {
        Self {
            schema_version: s.schema_version,
            seq: s.seq,
            run_id: s.run_id,
            status: s.status,
            goal_id: s.goal_id,
            milestone_id: s.milestone_id,
            ticket_id: s.ticket_id,
            current_ticket_id: s.current_ticket_id,
            executor: s.executor.kind,
            verification_policy: s.verification_policy,
            last_error: s.last_error,
            pre_task_git_sha: s.pre_task_git_sha,
            team_run_id: s.team_run_id,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionRunStateRequest {
    pub expected_seq: Option<u64>,
    pub status: String,
    pub reason: Option<String>,
    pub ticket_id: Option<String>,
    pub current_ticket_id: Option<String>,
    pub goal_id: Option<String>,
    pub last_error: Option<String>,
    pub team_run_id: Option<String>,
    pub executor: Option<String>,
    pub verification_policy: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunRequest {
    pub goal_id: Option<String>,
    pub milestone_id: Option<String>,
    pub ticket_id: Option<String>,
    pub agent_id: Option<String>,
    pub executor: Option<String>,
    pub verification_policy: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunResponse {
    pub run_id: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketCreateRequest {
    pub title: String,
    pub description: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub tags: Option<Vec<String>>,
    pub spec_ref: Option<String>,
    pub blockers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketUpdateRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub tags: Option<Vec<String>>,
    pub spec_ref: Option<String>,
    pub blockers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquireBridgeLeaseRequest {
    pub agent_id: String,
    pub reason: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeLeaseResponse {
    pub lease_id: String,
    pub agent_id: String,
    pub acquired_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuxStateResponse {
    pub run_state: Option<RunStateResponse>,
    pub lock_held: bool,
    pub project_root: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: resolve project_root from state
// ---------------------------------------------------------------------------

fn resolve_project_root(state: &GatewayState) -> Result<PathBuf, Response> {
    state
        .config
        .project_root
        .clone()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "project_root not configured").into_response())
}

// ---------------------------------------------------------------------------
// Helper: parse TicketStatus from string
// ---------------------------------------------------------------------------

fn parse_ticket_status(s: &str) -> anyhow::Result<TicketStatus> {
    match s {
        "Backlog" => Ok(TicketStatus::Backlog),
        "Blocked" => Ok(TicketStatus::Blocked),
        "ToDo" => Ok(TicketStatus::ToDo),
        "InProgress" => Ok(TicketStatus::InProgress),
        "Done" => Ok(TicketStatus::Done),
        other => anyhow::bail!("unknown TicketStatus: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Helper: parse TicketPriority from string
// ---------------------------------------------------------------------------

fn parse_ticket_priority(s: &str) -> anyhow::Result<TicketPriority> {
    match s {
        "Critical" => Ok(TicketPriority::Critical),
        "High" => Ok(TicketPriority::High),
        "Medium" => Ok(TicketPriority::Medium),
        "Low" => Ok(TicketPriority::Low),
        other => anyhow::bail!("unknown TicketPriority: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Helper: parse RunStatus from string (no FromStr impl on RunStatus)
// ---------------------------------------------------------------------------

fn parse_run_status(s: &str) -> anyhow::Result<RunStatus> {
    match s {
        "Idle" => Ok(RunStatus::Idle),
        "Planning" => Ok(RunStatus::Planning),
        "planned" | "Planned" => Ok(RunStatus::Planned),
        "dispatch_ready" | "DispatchReady" => Ok(RunStatus::DispatchReady),
        "executing" | "Executing" => Ok(RunStatus::Executing),
        "AwaitingApproval" => Ok(RunStatus::AwaitingApproval),
        "ExecutingTicket" => Ok(RunStatus::ExecutingTicket),
        "Verifying" => Ok(RunStatus::Verifying),
        "AwaitingPlayStart" => Ok(RunStatus::AwaitingPlayStart),
        "AwaitingFeedback" => Ok(RunStatus::AwaitingFeedback),
        "Paused" => Ok(RunStatus::Paused),
        "Blocked" => Ok(RunStatus::Blocked),
        "retry_ready" | "RetryReady" => Ok(RunStatus::RetryReady),
        "resumed" | "Resumed" => Ok(RunStatus::Resumed),
        "Completed" => Ok(RunStatus::Completed),
        "Failed" => Ok(RunStatus::Failed),
        "Interrupted" => Ok(RunStatus::Interrupted),
        "Recovering" => Ok(RunStatus::Recovering),
        "Quarantined" => Ok(RunStatus::Quarantined),
        other => anyhow::bail!("unknown RunStatus: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Handlers — Run State
// ---------------------------------------------------------------------------

/// GET /api/lux/runs/state
/// Returns the current run-state.json.
async fn get_run_state(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
) -> Result<Json<RunStateResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let run_state = RunState::load(&project_root).map_err(internal_error)?;
    Ok(Json(RunStateResponse::from(run_state)))
}

/// POST /api/lux/runs/start
/// Starts a new run (transitions Idle → Planning).
async fn start_run(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<StartRunRequest>,
) -> Result<Json<StartRunResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");
    let agent_id = req.agent_id.as_deref().unwrap_or("gateway");
    let force = req.force.unwrap_or(false);

    let _guard = acquire_lux_lock(
        &lux_dir,
        agent_id,
        "start_run",
        DEFAULT_STALE_THRESHOLD_SECS,
        force,
    )
    .map_err(bad_request)?;

    let mut run_state = RunState::load(&project_root).map_err(internal_error)?;
    run_state
        .transition_to(RunStatus::Planning, "start_run")
        .map_err(bad_request)?;
    run_state.run_id = Uuid::new_v4().to_string();
    run_state.goal_id = req.goal_id;
    run_state.milestone_id = req.milestone_id;
    run_state.ticket_id = req.ticket_id;
    run_state.executor.kind = req.executor;
    run_state.verification_policy = req.verification_policy;
    run_state.save(&project_root).map_err(internal_error)?;

    Ok(Json(StartRunResponse {
        run_id: run_state.run_id,
        status: run_state.status,
        updated_at: run_state.updated_at,
    }))
}

/// POST /api/lux/runs/transition
/// Transitions run state to a new status.
async fn transition_run(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<TransitionRunStateRequest>,
) -> Result<Json<RunStateResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");
    let force = req.force.unwrap_or(false);

    let new_status = parse_run_status(&req.status).map_err(bad_request)?;

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        &format!("transition_run:{}", req.status),
        DEFAULT_STALE_THRESHOLD_SECS,
        force,
    )
    .map_err(bad_request)?;

    let reason = req.reason.as_deref().unwrap_or("api_transition");
    let apply_request = |run_state: &mut RunState| {
        if let Some(ticket_id) = req.ticket_id.clone() {
            run_state.ticket_id = Some(ticket_id);
        }
        if let Some(ticket_id) = req.current_ticket_id.clone() {
            run_state.current_ticket_id = Some(ticket_id);
        }
        if let Some(goal_id) = req.goal_id.clone() {
            run_state.goal_id = Some(goal_id);
        }
        if let Some(last_error) = req.last_error.clone() {
            run_state.last_error = Some(last_error);
        }
        if let Some(team_run_id) = req.team_run_id.clone() {
            run_state.team_run_id = Some(team_run_id);
        }
        if let Some(executor) = req.executor.clone() {
            run_state.executor.kind = Some(executor);
        }
        if let Some(policy) = req.verification_policy.clone() {
            run_state.verification_policy = Some(policy);
        }
    };

    let run_state = if let Some(expected_seq) = req.expected_seq {
        RunState::transition_with_seq_check(
            &project_root,
            expected_seq,
            new_status,
            reason,
            apply_request,
        )
        .map_err(|error| {
            if error.to_string().contains("stale seq") {
                bad_request(error)
            } else {
                internal_error(error)
            }
        })?
    } else {
        let mut run_state = RunState::load(&project_root).map_err(internal_error)?;
        run_state
            .transition_to(new_status, reason)
            .map_err(bad_request)?;
        apply_request(&mut run_state);
        run_state.save(&project_root).map_err(internal_error)?;
        run_state
    };
    Ok(Json(RunStateResponse::from(run_state)))
}

/// POST /api/lux/runs/stop
/// Stops the current run (transitions to Idle).
async fn stop_run(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
) -> Result<Json<RunStateResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "stop_run",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )
    .map_err(bad_request)?;

    let mut run_state = RunState::load(&project_root).map_err(internal_error)?;
    run_state
        .transition_to(RunStatus::Idle, "stop_run")
        .map_err(bad_request)?;
    run_state.current_ticket_id = None;
    run_state.save(&project_root).map_err(internal_error)?;
    Ok(Json(RunStateResponse::from(run_state)))
}

// ---------------------------------------------------------------------------
// Handlers — Tickets (Single-Writer)
// ---------------------------------------------------------------------------

/// GET /api/lux/runs/tickets
/// Lists tickets with optional filter.
async fn list_run_tickets(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
) -> Result<Json<Vec<Ticket>>, Response> {
    let project_root = resolve_project_root(&state)?;
    let store = FileTicketStore::new(&project_root);
    let tickets = store
        .list(TicketFilter::default())
        .map_err(internal_error)?;
    Ok(Json(tickets))
}

/// POST /api/lux/runs/tickets
/// Creates a new ticket.
async fn create_run_ticket(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<TicketCreateRequest>,
) -> Result<Json<Ticket>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "create_ticket",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )
    .map_err(bad_request)?;

    let priority = req
        .priority
        .as_deref()
        .map(parse_ticket_priority)
        .transpose()
        .map_err(bad_request)?
        .unwrap_or(TicketPriority::Medium);

    let now = Utc::now().to_rfc3339();
    let ticket = Ticket {
        id: Uuid::new_v4().to_string(),
        title: req.title,
        description: req.description,
        status: TicketStatus::Backlog,
        priority,
        assignee: req.assignee,
        blockers: req.blockers.unwrap_or_default(),
        tags: req.tags.unwrap_or_default(),
        spec_ref: req.spec_ref,
        created_at: now.clone(),
        updated_at: now,
        execution_objective: None,
        allowed_executor: None,
        dispatch_policy: None,
        verification_policy: None,
        command_allowlist: None,
        evidence_refs: None,
        blocker_policy: None,
        non_goals: None,
    };

    let store = FileTicketStore::new(&project_root);
    let created = store.create(ticket).map_err(internal_error)?;
    Ok(Json(created))
}

/// GET /api/lux/runs/tickets/:id
/// Gets a single ticket by ID.
async fn get_run_ticket(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Ticket>, Response> {
    let project_root = resolve_project_root(&state)?;
    let store = FileTicketStore::new(&project_root);
    let ticket = store
        .get(&id)
        .map_err(internal_error)?
        .ok_or_else(not_found)?;
    Ok(Json(ticket))
}

/// PUT /api/lux/runs/tickets/:id
/// Updates a ticket.
async fn update_run_ticket(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(req): Json<TicketUpdateRequest>,
) -> Result<Json<Ticket>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "update_ticket",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )
    .map_err(bad_request)?;

    let store = FileTicketStore::new(&project_root);
    let mut ticket = store
        .get(&id)
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    if let Some(title) = req.title {
        ticket.title = title;
    }
    if let Some(description) = req.description {
        ticket.description = description;
    }
    if let Some(status_str) = req.status {
        ticket.status = parse_ticket_status(&status_str).map_err(bad_request)?;
    }
    if let Some(priority_str) = req.priority {
        ticket.priority = parse_ticket_priority(&priority_str).map_err(bad_request)?;
    }
    if let Some(assignee) = req.assignee {
        ticket.assignee = Some(assignee);
    }
    if let Some(tags) = req.tags {
        ticket.tags = tags;
    }
    if let Some(spec_ref) = req.spec_ref {
        ticket.spec_ref = Some(spec_ref);
    }
    if let Some(blockers) = req.blockers {
        ticket.blockers = blockers;
    }
    ticket.updated_at = Utc::now().to_rfc3339();

    let updated = store.update(&id, ticket).map_err(bad_request)?;
    Ok(Json(updated))
}

/// PUT /api/lux/runs/tickets/:id/status
/// Updates only the status of a ticket.
async fn update_run_ticket_status(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(req): Json<TicketStatusRequest>,
) -> Result<Json<Ticket>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "update_ticket_status",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )
    .map_err(bad_request)?;

    let store = FileTicketStore::new(&project_root);
    let mut ticket = store
        .get(&id)
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    ticket.status = parse_ticket_status(&req.status).map_err(bad_request)?;
    ticket.updated_at = Utc::now().to_rfc3339();

    let updated = store.update(&id, ticket).map_err(bad_request)?;
    Ok(Json(updated))
}

/// DELETE /api/lux/runs/tickets/:id
/// Deletes a ticket.
async fn delete_run_ticket(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "delete_ticket",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )
    .map_err(bad_request)?;

    let store = FileTicketStore::new(&project_root);
    store.delete(&id).map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

// ---------------------------------------------------------------------------
// Handlers — Bridge Lease
// ---------------------------------------------------------------------------

/// POST /api/lux/runs/bridge-lease
/// Acquires an in-memory bridge lease for an agent.
async fn acquire_bridge_lease(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<AcquireBridgeLeaseRequest>,
) -> Result<Json<BridgeLeaseResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");
    let force = req.force.unwrap_or(false);

    let _guard = acquire_lux_lock(
        &lux_dir,
        &req.agent_id,
        req.reason.as_deref().unwrap_or("bridge_lease"),
        DEFAULT_STALE_THRESHOLD_SECS,
        force,
    )
    .map_err(bad_request)?;

    let lease_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let lease = BridgeLease {
        lease_id: lease_id.clone(),
        agent_id: req.agent_id.clone(),
        project_root: project_root.to_string_lossy().to_string(),
        acquired_at: now.clone(),
        expires_at: None,
    };

    let mut leases = BRIDGE_LEASES
        .lock()
        .map_err(|e| internal_error(anyhow::anyhow!("lease lock poisoned: {}", e)))?;
    leases.insert(lease_id.clone(), lease);

    Ok(Json(BridgeLeaseResponse {
        lease_id,
        agent_id: req.agent_id,
        acquired_at: now,
    }))
}

/// DELETE /api/lux/runs/bridge-lease/:lease_id
/// Releases a bridge lease.
async fn release_bridge_lease(
    _state: State<GatewayState>,
    _headers: HeaderMap,
    AxumPath(lease_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, Response> {
    let mut leases = BRIDGE_LEASES
        .lock()
        .map_err(|e| internal_error(anyhow::anyhow!("lease lock poisoned: {}", e)))?;

    if leases.remove(&lease_id).is_none() {
        return Err(not_found());
    }

    Ok(Json(serde_json::json!({ "released": lease_id })))
}

/// GET /api/lux/runs/bridge-lease
/// Lists all active bridge leases.
async fn list_bridge_leases(
    _state: State<GatewayState>,
    _headers: HeaderMap,
) -> Result<Json<Vec<BridgeLease>>, Response> {
    let leases = BRIDGE_LEASES
        .lock()
        .map_err(|e| internal_error(anyhow::anyhow!("lease lock poisoned: {}", e)))?;
    let list: Vec<BridgeLease> = leases.values().cloned().collect();
    Ok(Json(list))
}

// ---------------------------------------------------------------------------
// Request / Response types — Producer endpoints
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerRecord {
    pub id: String,
    pub run_id: String,
    pub submitted_at: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerPayload {
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerCreatedResponse {
    pub id: String,
    pub run_id: String,
    pub stored_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptEvidenceRequest {
    pub task_id: String,
    pub evidence_id: String,
    pub force: Option<bool>,
}

// ---------------------------------------------------------------------------
// Helpers — Producer endpoints
// ---------------------------------------------------------------------------

fn active_run_id(project_root: &std::path::Path) -> Result<String, Response> {
    let run_state = RunState::load(project_root).map_err(internal_error)?;
    if run_state.run_id.is_empty() {
        return Err((StatusCode::CONFLICT, "no active run").into_response());
    }
    Ok(run_state.run_id)
}

fn store_producer_record(
    project_root: &std::path::Path,
    run_id: &str,
    subdir: &str,
    payload: serde_json::Value,
) -> Result<ProducerRecord, Response> {
    let record_id = Uuid::new_v4().to_string();
    let submitted_at = Utc::now().to_rfc3339();
    let record = ProducerRecord {
        id: record_id.clone(),
        run_id: run_id.to_string(),
        submitted_at: submitted_at.clone(),
        payload,
    };
    let dir = project_root
        .join(".lux")
        .join("runs")
        .join(run_id)
        .join(subdir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| internal_error(anyhow::anyhow!("create dir: {e}")))?;
    let path = dir.join(format!("{record_id}.json"));
    crate::lux_io::atomic_write_json(&path, &record)
        .map_err(|e| internal_error(anyhow::anyhow!("write record: {e}")))?;
    Ok(record)
}

// ---------------------------------------------------------------------------
// Handlers — Producer endpoints
// ---------------------------------------------------------------------------

async fn post_proposal(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<ProducerPayload>,
) -> Result<(StatusCode, Json<ProducerCreatedResponse>), Response> {
    let project_root = resolve_project_root(&state)?;
    let run_id = active_run_id(&project_root)?;
    let record = store_producer_record(&project_root, &run_id, "proposals", req.payload)?;
    Ok((
        StatusCode::CREATED,
        Json(ProducerCreatedResponse {
            id: record.id,
            run_id: record.run_id,
            stored_at: record.submitted_at,
        }),
    ))
}

async fn post_evidence(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<ProducerPayload>,
) -> Result<(StatusCode, Json<ProducerCreatedResponse>), Response> {
    let project_root = resolve_project_root(&state)?;
    let run_id = active_run_id(&project_root)?;
    let record = store_producer_record(&project_root, &run_id, "evidence", req.payload)?;
    Ok((
        StatusCode::CREATED,
        Json(ProducerCreatedResponse {
            id: record.id,
            run_id: record.run_id,
            stored_at: record.submitted_at,
        }),
    ))
}

async fn post_blocker_resolution_request(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<ProducerPayload>,
) -> Result<(StatusCode, Json<ProducerCreatedResponse>), Response> {
    let project_root = resolve_project_root(&state)?;
    let run_id = active_run_id(&project_root)?;
    let record = store_producer_record(&project_root, &run_id, "blocker-requests", req.payload)?;
    Ok((
        StatusCode::CREATED,
        Json(ProducerCreatedResponse {
            id: record.id,
            run_id: record.run_id,
            stored_at: record.submitted_at,
        }),
    ))
}

async fn post_milestone_push_request(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<ProducerPayload>,
) -> Result<(StatusCode, Json<ProducerCreatedResponse>), Response> {
    let project_root = resolve_project_root(&state)?;
    let run_id = active_run_id(&project_root)?;
    let record = store_producer_record(&project_root, &run_id, "push-requests", req.payload)?;
    Ok((
        StatusCode::CREATED,
        Json(ProducerCreatedResponse {
            id: record.id,
            run_id: record.run_id,
            stored_at: record.submitted_at,
        }),
    ))
}

async fn accept_evidence(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
    Json(req): Json<AcceptEvidenceRequest>,
) -> Result<Json<serde_json::Value>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");
    let force = req.force.unwrap_or(false);

    let _guard = acquire_lux_lock(
        &lux_dir,
        "gateway",
        "accept_evidence",
        DEFAULT_STALE_THRESHOLD_SECS,
        force,
    )
    .map_err(bad_request)?;

    let manifest_path = {
        let run_state = RunState::load(&project_root).map_err(internal_error)?;
        if run_state.run_id.is_empty() {
            return Err((StatusCode::CONFLICT, "no active run").into_response());
        }
        project_root
            .join(".lux")
            .join("runs")
            .join(&run_state.run_id)
            .join("manifest.json")
    };

    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|e| internal_error(anyhow::anyhow!("read manifest: {e}")))?;
    let mut manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| internal_error(anyhow::anyhow!("parse manifest: {e}")))?;

    let task_id = &req.task_id;
    let evidence_id = &req.evidence_id;

    if let Some(nodes) = manifest.get_mut("nodes").and_then(|n| n.as_object_mut()) {
        if let Some(node) = nodes.get_mut(task_id) {
            let current = node.get("status").and_then(|s| s.as_str()).unwrap_or("");
            if current != "awaitingEvidence" && current != "inProgress" {
                return Err((
                    StatusCode::CONFLICT,
                    format!("task {task_id} is not awaiting evidence (status={current})"),
                )
                    .into_response());
            }
            node["status"] = serde_json::json!("done");
            node["evidencePath"] =
                serde_json::json!(format!(".lux/runs/{{run_id}}/evidence/{evidence_id}.json"));
        } else {
            return Err(
                (StatusCode::NOT_FOUND, format!("task {task_id} not found")).into_response()
            );
        }
    }

    crate::lux_io::atomic_write_json(&manifest_path, &manifest)
        .map_err(|e| internal_error(anyhow::anyhow!("write manifest: {e}")))?;

    Ok(Json(serde_json::json!({
        "taskId": task_id,
        "evidenceId": evidence_id,
        "accepted": true,
    })))
}

// ---------------------------------------------------------------------------
// Handlers — Lux State (composite)
// ---------------------------------------------------------------------------

/// GET /api/lux/runs/lux-state
/// Returns composite state: run-state + lock status.
async fn get_lux_state(
    State(state): State<GatewayState>,
    _headers: HeaderMap,
) -> Result<Json<LuxStateResponse>, Response> {
    let project_root = resolve_project_root(&state)?;
    let lux_dir = project_root.join(".lux");

    let run_state = RunState::load(&project_root)
        .ok()
        .map(RunStateResponse::from);

    let lock_held = crate::lux_lock::check_lux_lock(&lux_dir)
        .ok()
        .flatten()
        .is_some();

    Ok(Json(LuxStateResponse {
        run_state,
        lock_held,
        project_root: Some(project_root.to_string_lossy().to_string()),
    }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the lux_api sub-router. Mount at `/api/lux/runs` in server.rs.
pub fn build_lux_api_router() -> Router<GatewayState> {
    Router::new()
        // Run state
        .route("/state", get(get_run_state))
        .route("/start", post(start_run))
        .route("/transition", post(transition_run))
        .route("/stop", post(stop_run))
        // Tickets
        .route("/tickets", get(list_run_tickets).post(create_run_ticket))
        .route(
            "/tickets/:id",
            get(get_run_ticket)
                .put(update_run_ticket)
                .delete(delete_run_ticket),
        )
        .route("/tickets/:id/status", put(update_run_ticket_status))
        // Bridge leases
        .route(
            "/bridge-lease",
            get(list_bridge_leases).post(acquire_bridge_lease),
        )
        .route(
            "/bridge-lease/:lease_id",
            axum::routing::delete(release_bridge_lease),
        )
        // Producer endpoints (team-mode proposals, evidence, requests)
        .route("/proposals", post(post_proposal))
        .route("/evidence", post(post_evidence))
        .route(
            "/blocker-resolution-requests",
            post(post_blocker_resolution_request),
        )
        .route(
            "/milestone-push-requests",
            post(post_milestone_push_request),
        )
        .route("/evidence/accept", post(accept_evidence))
        // Composite state
        .route("/lux-state", get(get_lux_state))
}
