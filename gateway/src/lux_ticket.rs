use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// Inline atomic write — avoids crate::lux_io dependency so lux_ticket_test.rs
// can include this file via #[path = "../src/lux_ticket.rs"] without the full module tree.
fn ticket_atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .context("failed to serialize value for atomic write")?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace file {}", path.display()))
}

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

#[derive(Debug, Clone)]
pub struct FileTicketStore {
    base_path: PathBuf,
}

impl FileTicketStore {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            base_path: project_root.as_ref().join(".lux/tickets"),
        }
    }

    fn ticket_path(&self, id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }

    fn ensure_base_path(&self) -> Result<()> {
        fs::create_dir_all(&self.base_path).context("failed to create ticket store directory")
    }

    fn read_ticket(path: &Path) -> Result<Ticket> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read ticket file {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse ticket file {}", path.display()))
    }

    fn write_ticket(path: &Path, ticket: &Ticket) -> Result<()> {
        ticket_atomic_write_json(path, ticket)
            .with_context(|| format!("failed to write ticket file {}", path.display()))
    }

    pub fn find_open_blocker_by_stable_key(&self, stable_key: &str) -> Result<Option<Ticket>> {
        let stable_tag = blocker_stable_tag(stable_key);
        let mut matches = self
            .list(TicketFilter::default())?
            .into_iter()
            .filter(|ticket| ticket.status != TicketStatus::Done)
            .filter(|ticket| ticket.tags.iter().any(|tag| tag == &stable_tag))
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        Ok(matches.into_iter().next())
    }

    pub fn blocker_dependency_would_cycle(
        &self,
        blocked_ticket_id: &str,
        blocker_ticket_id: &str,
    ) -> Result<bool> {
        if blocked_ticket_id == blocker_ticket_id {
            return Ok(true);
        }
        self.has_blocker_path(blocker_ticket_id, blocked_ticket_id, &mut HashSet::new())
    }

    pub fn prospective_blocker_depth(
        &self,
        blocked_ticket_id: &str,
        blocker_ticket_id: &str,
    ) -> Result<u32> {
        let blocked_ticket = self
            .get(blocked_ticket_id)?
            .ok_or_else(|| anyhow!("blocked ticket not found: {}", blocked_ticket_id))?;

        let mut blockers = blocked_ticket.blockers.clone();
        if !blockers.iter().any(|id| id == blocker_ticket_id) {
            blockers.push(blocker_ticket_id.to_string());
        }

        let mut max_depth = 0;
        for blocker_id in blockers {
            let depth = 1 + self.blocker_depth_from(&blocker_id, &mut HashSet::new())?;
            max_depth = max_depth.max(depth);
        }
        Ok(max_depth)
    }

    pub fn add_blocker_dependency(
        &self,
        blocked_ticket_id: &str,
        blocker_ticket_id: &str,
    ) -> Result<Ticket> {
        if self.blocker_dependency_would_cycle(blocked_ticket_id, blocker_ticket_id)? {
            bail!(
                "blocker cycle detected: adding {} as blocker for {} would create a cycle",
                blocker_ticket_id,
                blocked_ticket_id
            );
        }

        let mut blocked_ticket = self
            .get(blocked_ticket_id)?
            .ok_or_else(|| anyhow!("blocked ticket not found: {}", blocked_ticket_id))?;
        if !blocked_ticket
            .blockers
            .iter()
            .any(|id| id == blocker_ticket_id)
        {
            blocked_ticket.blockers.push(blocker_ticket_id.to_string());
        }
        blocked_ticket.status = TicketStatus::Blocked;
        blocked_ticket.updated_at = chrono::Utc::now().to_rfc3339();
        self.update(blocked_ticket_id, blocked_ticket)
    }

    fn has_blocker_path(
        &self,
        from_ticket_id: &str,
        target_ticket_id: &str,
        visited: &mut HashSet<String>,
    ) -> Result<bool> {
        if from_ticket_id == target_ticket_id {
            return Ok(true);
        }
        if !visited.insert(from_ticket_id.to_string()) {
            return Ok(false);
        }
        let Some(ticket) = self.get(from_ticket_id)? else {
            return Ok(false);
        };
        for blocker_id in ticket.blockers {
            if self.has_blocker_path(&blocker_id, target_ticket_id, visited)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn blocker_depth_from(&self, ticket_id: &str, visiting: &mut HashSet<String>) -> Result<u32> {
        if !visiting.insert(ticket_id.to_string()) {
            bail!(
                "blocker cycle detected while measuring depth at {}",
                ticket_id
            );
        }
        let Some(ticket) = self.get(ticket_id)? else {
            visiting.remove(ticket_id);
            return Ok(0);
        };
        let mut max_depth = 0;
        for blocker_id in ticket.blockers {
            let depth = 1 + self.blocker_depth_from(&blocker_id, visiting)?;
            max_depth = max_depth.max(depth);
        }
        visiting.remove(ticket_id);
        Ok(max_depth)
    }

    fn blockers_resolved(&self, ticket: &Ticket) -> Result<bool> {
        for blocker_id in &ticket.blockers {
            match self.get(blocker_id)? {
                Some(blocker) if blocker.status == TicketStatus::Done => continue,
                Some(_) | None => return Ok(false),
            }
        }
        Ok(true)
    }

    fn validate_transition(&self, existing: &Ticket, updated: &Ticket) -> Result<()> {
        if existing.status == updated.status {
            return Ok(());
        }

        match (&existing.status, &updated.status) {
            (TicketStatus::Backlog, TicketStatus::ToDo) => Ok(()),
            (TicketStatus::ToDo, TicketStatus::InProgress) => {
                if self.blockers_resolved(updated)? {
                    Ok(())
                } else {
                    bail!("transition denied: active blockers prevent moving to InProgress")
                }
            }
            (TicketStatus::InProgress, TicketStatus::Done) => Ok(()),
            (_, TicketStatus::Blocked) => Ok(()),
            (TicketStatus::Blocked, TicketStatus::ToDo) => {
                if self.blockers_resolved(updated)? {
                    Ok(())
                } else {
                    bail!("transition denied: blockers must be resolved before moving to ToDo")
                }
            }
            (TicketStatus::Done, TicketStatus::ToDo) => Ok(()),
            _ => bail!(
                "transition denied: {:?} -> {:?} is not allowed",
                existing.status,
                updated.status
            ),
        }
    }
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

pub fn create_or_update_blocker(
    store: &FileTicketStore,
    check_category: &str,
    check_name: &str,
    spec_ref: Option<&str>,
    title: String,
    description: String,
    priority: TicketPriority,
    tags: Vec<String>,
) -> Result<BlockerTicketUpsert> {
    let stable_key = stable_blocker_key(check_category, check_name, spec_ref);
    let stable_id = stable_blocker_ticket_id_from_key(&stable_key);
    let stable_tag = blocker_stable_tag(&stable_key);
    let now = chrono::Utc::now().to_rfc3339();

    let existing = match store.get(&stable_id)? {
        Some(ticket) => Some(ticket),
        None => store.find_open_blocker_by_stable_key(&stable_key)?,
    };

    if let Some(mut ticket) = existing {
        ticket.title = title;
        ticket.description = description;
        ticket.priority = priority;
        ticket.spec_ref = spec_ref.map(ToOwned::to_owned);
        ticket.tags = merge_tags(ticket.tags, tags, stable_tag);
        if ticket.status == TicketStatus::Done {
            ticket.status = TicketStatus::Blocked;
        }
        ticket.updated_at = now;
        let ticket_id = ticket.id.clone();
        let updated = store.update(&ticket_id, ticket)?;
        return Ok(BlockerTicketUpsert {
            ticket: updated,
            stable_key,
            stable_id,
            created: false,
        });
    }

    let ticket = Ticket {
        id: stable_id.clone(),
        title,
        description,
        status: TicketStatus::Blocked,
        priority,
        assignee: None,
        blockers: Vec::new(),
        tags: merge_tags(Vec::new(), tags, stable_tag),
        spec_ref: spec_ref.map(ToOwned::to_owned),
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
    let created_ticket = store.create(ticket)?;
    Ok(BlockerTicketUpsert {
        ticket: created_ticket,
        stable_key,
        stable_id,
        created: true,
    })
}

fn merge_tags(mut existing: Vec<String>, incoming: Vec<String>, stable_tag: String) -> Vec<String> {
    for tag in incoming.into_iter().chain(std::iter::once(stable_tag)) {
        if !existing.iter().any(|value| value == &tag) {
            existing.push(tag);
        }
    }
    existing
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

impl TicketStore for FileTicketStore {
    fn create(&self, ticket: Ticket) -> Result<Ticket> {
        ticket.validate()?;
        self.ensure_base_path()?;

        let path = self.ticket_path(&ticket.id);
        if path.exists() {
            bail!("ticket already exists: {}", ticket.id);
        }

        Self::write_ticket(&path, &ticket)?;
        Ok(ticket)
    }

    fn get(&self, id: &str) -> Result<Option<Ticket>> {
        let path = self.ticket_path(id);
        if !path.exists() {
            return Ok(None);
        }

        Ok(Some(Self::read_ticket(&path)?))
    }

    fn update(&self, id: &str, ticket: Ticket) -> Result<Ticket> {
        ticket.validate()?;
        self.ensure_base_path()?;

        let path = self.ticket_path(id);
        let existing = self
            .get(id)?
            .ok_or_else(|| anyhow!("ticket not found: {}", id))?;
        if ticket.id != id {
            bail!(
                "ticket id mismatch: path id {} does not match ticket id {}",
                id,
                ticket.id
            );
        }

        self.validate_transition(&existing, &ticket)?;
        Self::write_ticket(&path, &ticket)?;
        Ok(ticket)
    }

    fn list(&self, filter: TicketFilter) -> Result<Vec<Ticket>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut tickets = Vec::new();
        for entry in fs::read_dir(&self.base_path).with_context(|| {
            format!(
                "failed to read ticket directory {}",
                self.base_path.display()
            )
        })? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }

            let ticket = Self::read_ticket(&path)?;
            if filter
                .status
                .as_ref()
                .is_some_and(|status| ticket.status != *status)
            {
                continue;
            }
            if filter
                .priority
                .as_ref()
                .is_some_and(|priority| ticket.priority != *priority)
            {
                continue;
            }
            if filter
                .has_blockers
                .is_some_and(|has_blockers| has_blockers != !ticket.blockers.is_empty())
            {
                continue;
            }
            if filter
                .tag
                .as_ref()
                .is_some_and(|tag| !ticket.tags.iter().any(|value| value == tag))
            {
                continue;
            }
            if filter
                .spec_ref
                .as_ref()
                .is_some_and(|spec_ref| ticket.spec_ref.as_ref() != Some(spec_ref))
            {
                continue;
            }

            tickets.push(ticket);
        }

        Ok(tickets)
    }

    fn delete(&self, id: &str) -> Result<()> {
        let path = self.ticket_path(id);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to delete ticket file {}", path.display()))?;
        }
        Ok(())
    }

    fn check_blockers(&self, id: &str) -> Result<Vec<Ticket>> {
        let ticket = self
            .get(id)?
            .ok_or_else(|| anyhow!("ticket not found: {}", id))?;
        let mut blockers = Vec::new();
        for blocker_id in ticket.blockers {
            if let Some(blocker) = self.get(&blocker_id)? {
                blockers.push(blocker);
            }
        }
        Ok(blockers)
    }
}
