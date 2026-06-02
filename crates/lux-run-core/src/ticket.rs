use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPolicy {
    Manual,
    DispatchRequested,
    AutoDispatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockerPolicy {
    pub max_depth: Option<u32>,
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub assignee: Option<String>,
    pub blockers: Vec<String>,
    pub tags: Vec<String>,
    pub spec_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_objective: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatch_policy: Option<DispatchPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_allowlist: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker_policy: Option<BlockerPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_goals: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TicketStatus {
    #[default]
    Backlog,
    Blocked,
    ToDo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TicketPriority {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TicketFilter {
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub has_blockers: Option<bool>,
    pub tag: Option<String>,
    pub spec_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerTicketUpsert {
    pub ticket: Ticket,
    pub stable_key: String,
    pub stable_id: String,
    pub created: bool,
}

pub trait TicketStore {
    fn create(&self, ticket: Ticket) -> Result<Ticket>;
    fn get(&self, id: &str) -> Result<Option<Ticket>>;
    fn update(&self, id: &str, ticket: Ticket) -> Result<Ticket>;
    fn list(&self, filter: TicketFilter) -> Result<Vec<Ticket>>;
    fn delete(&self, id: &str) -> Result<()>;
    fn check_blockers(&self, id: &str) -> Result<Vec<Ticket>>;
}

pub fn stable_blocker_key(
    check_category: &str,
    check_name: &str,
    spec_ref: Option<&str>,
) -> String {
    let canonical = format!(
        "category={}\nname={}\nspec_ref={}",
        check_category.trim(),
        check_name.trim(),
        spec_ref.unwrap_or("").trim()
    );
    let digest = Sha256::digest(canonical.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn stable_blocker_ticket_id(
    check_category: &str,
    check_name: &str,
    spec_ref: Option<&str>,
) -> String {
    let key = stable_blocker_key(check_category, check_name, spec_ref);
    stable_blocker_ticket_id_from_key(&key)
}

pub fn stable_blocker_ticket_id_from_key(stable_key: &str) -> String {
    let digest = stable_key.chars().take(32).collect::<String>();
    format!("blocker-{digest}")
}

pub fn blocker_stable_tag(stable_key: &str) -> String {
    format!("blocker-key:{stable_key}")
}

/// Returns true only if the ticket has all required execution-grade fields set
/// and the dispatch policy is not Manual.
pub fn is_execution_grade(ticket: &Ticket) -> bool {
    ticket.execution_objective.is_some()
        && ticket.allowed_executor.is_some()
        && ticket.verification_policy.is_some()
        && matches!(
            ticket.dispatch_policy,
            Some(DispatchPolicy::DispatchRequested) | Some(DispatchPolicy::AutoDispatch)
        )
}

/// Validates that a ticket is ready for execution dispatch.
/// Returns Err with a descriptive message if any required field is missing or invalid.
pub fn validate_execution_grade(ticket: &Ticket) -> Result<(), String> {
    if ticket.execution_objective.is_none() {
        return Err("execution_objective is required for execution-grade dispatch".to_string());
    }
    if ticket.allowed_executor.is_none() {
        return Err("allowed_executor is required for execution-grade dispatch".to_string());
    }
    if ticket.verification_policy.is_none() {
        return Err("verification_policy is required for execution-grade dispatch".to_string());
    }
    match &ticket.dispatch_policy {
        None | Some(DispatchPolicy::Manual) => {
            return Err(
                "dispatch_policy must be DispatchRequested or AutoDispatch for execution-grade dispatch"
                    .to_string(),
            );
        }
        Some(DispatchPolicy::DispatchRequested) | Some(DispatchPolicy::AutoDispatch) => {}
    }
    if ticket.verification_policy.as_deref() == Some("command_suite") {
        let allowlist_empty = ticket
            .command_allowlist
            .as_ref()
            .map_or(true, |v| v.is_empty());
        if allowlist_empty {
            return Err(
                "command_allowlist must be non-empty when verification_policy is command_suite"
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// Returns true ONLY if the ticket's dispatch_policy is DispatchRequested or AutoDispatch.
/// Moving a ticket to InProgress status alone NEVER triggers dispatch.
pub fn should_dispatch(ticket: &Ticket) -> bool {
    matches!(
        ticket.dispatch_policy,
        Some(DispatchPolicy::DispatchRequested) | Some(DispatchPolicy::AutoDispatch)
    )
}

impl Ticket {
    pub fn validate(&self) -> Result<()> {
        // Accept both UUID and slug-style IDs (e.g. "ticket-001") to support
        // spec-generated tickets that use human-readable identifiers.
        if uuid::Uuid::parse_str(&self.id).is_err() {
            let slug_ok = !self.id.is_empty()
                && self.id.len() <= 128
                && self
                    .id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
            anyhow::ensure!(
                slug_ok,
                "ticket id is neither a valid UUID nor a valid slug: {}",
                self.id
            );
        }
        chrono::DateTime::parse_from_rfc3339(&self.created_at)
            .with_context(|| format!("invalid created_at timestamp: {}", self.created_at))?;
        chrono::DateTime::parse_from_rfc3339(&self.updated_at)
            .with_context(|| format!("invalid updated_at timestamp: {}", self.updated_at))?;
        Ok(())
    }
}
