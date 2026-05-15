use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    lux_bridge_lease::{expire_stale_leases, BridgeLease, LeaseStatus},
    lux_io::atomic_write_json,
    lux_metrics::RunMetrics,
    lux_run::{
        AgentSession, RunConfig, RunLifecycle, TaskProjection, TransactionJournal,
        TransactionStatus,
    },
    lux_run_state::{RunState, RunStatus},
    lux_task_dag::{TaskDAG, TaskStatus},
    lux_team_profile::TeamProfile,
    lux_worktree::{Worktree, WorktreeStatus},
};

/// Durable record of an active ticket execution.
/// Written to `.lux/runs/<run_id>/session.json` before the executor spawns.
/// Updated on each heartbeat tick. Used by `recover_stuck_executions` on restart.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSession {
    pub run_id: String,
    pub ticket_id: String,
    pub started_at: String,
    pub last_heartbeat_at: String,
    pub timeout_secs: u64,
    pub max_attempts: u32,
    pub attempt_number: u32,
}

impl ExecutionSession {
    pub fn path(project_path: &Path, run_id: &str) -> PathBuf {
        project_path
            .join(".lux")
            .join("runs")
            .join(run_id)
            .join("session.json")
    }

    pub fn begin(
        project_path: &Path,
        run_id: &str,
        ticket_id: &str,
        timeout_secs: u64,
        max_attempts: u32,
    ) -> Result<Self> {
        let now = Utc::now().to_rfc3339();
        let session = Self {
            run_id: run_id.to_string(),
            ticket_id: ticket_id.to_string(),
            started_at: now.clone(),
            last_heartbeat_at: now,
            timeout_secs,
            max_attempts,
            attempt_number: 0,
        };
        atomic_write_json(&Self::path(project_path, run_id), &session)?;
        Ok(session)
    }

    pub fn heartbeat(&mut self, project_path: &Path) -> Result<()> {
        self.last_heartbeat_at = Utc::now().to_rfc3339();
        atomic_write_json(&Self::path(project_path, &self.run_id), self)
    }

    pub fn load(project_path: &Path, run_id: &str) -> Result<Option<Self>> {
        let path = Self::path(project_path, run_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read session file {}", path.display()))?;
        let session: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse session file {}", path.display()))?;
        Ok(Some(session))
    }
}

/// Scan all run directories for sessions whose heartbeat has exceeded `timeout_secs`.
/// For each stuck session, transitions the run-state to `RetryReady` (if currently
/// `ExecutingTicket`) using `transition_with_seq_check` so stale-seq conflicts are
/// surfaced rather than silently overwritten.
///
/// Returns the list of run IDs that were recovered.
pub fn recover_stuck_executions(project_path: &Path) -> Result<Vec<String>> {
    let runs_dir = project_path.join(".lux").join("runs");
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut recovered = Vec::new();
    for entry in fs::read_dir(&runs_dir)
        .with_context(|| format!("failed to read runs dir {}", runs_dir.display()))?
    {
        let entry = entry?;
        let run_id = entry.file_name().to_string_lossy().to_string();
        let session = match ExecutionSession::load(project_path, &run_id)? {
            Some(s) => s,
            None => continue,
        };

        let last_beat = DateTime::parse_from_rfc3339(&session.last_heartbeat_at)
            .with_context(|| {
                format!(
                    "session {} has invalid last_heartbeat_at: {}",
                    run_id, session.last_heartbeat_at
                )
            })?
            .with_timezone(&Utc);

        let elapsed = Utc::now()
            .signed_duration_since(last_beat)
            .num_seconds()
            .max(0) as u64;

        if elapsed < session.timeout_secs {
            continue;
        }

        let state = match RunState::load(project_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if state.status != RunStatus::ExecutingTicket.to_string() {
            continue;
        }

        let target_status = if session.attempt_number < session.max_attempts {
            RunStatus::RetryReady
        } else {
            RunStatus::Blocked
        };
        let reason_label = if target_status == RunStatus::RetryReady {
            "recover_stuck_executions: heartbeat timeout"
        } else {
            "recover_stuck_executions: max attempts exceeded"
        };

        match RunState::transition_with_seq_check(
            project_path,
            state.seq,
            target_status,
            reason_label,
            |s| {
                s.last_error = Some(format!(
                    "execution timed out after {}s (ticket: {})",
                    session.timeout_secs, session.ticket_id
                ));
            },
        ) {
            Ok(_) => recovered.push(run_id),
            Err(e) => {
                eprintln!(
                    "[lux] recover_stuck_executions: seq conflict for run {run_id}, skipping: {e:#}"
                );
            }
        }
    }
    Ok(recovered)
}

pub fn recover_pending_transactions(project_path: &Path) -> Result<Vec<PathBuf>> {
    let runs_dir = project_path.join(".lux").join("runs");
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut recovered = Vec::new();
    for run_entry in fs::read_dir(&runs_dir)
        .with_context(|| format!("failed to read runs dir {}", runs_dir.display()))?
    {
        let run_entry = run_entry?;
        let transactions_dir = run_entry.path().join("transactions");
        if !transactions_dir.exists() {
            continue;
        }
        for transaction_entry in fs::read_dir(&transactions_dir).with_context(|| {
            format!(
                "failed to read transactions dir {}",
                transactions_dir.display()
            )
        })? {
            let transaction_path = transaction_entry?.path();
            if transaction_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let mut journal = TransactionJournal::load(&transaction_path)?;
            if journal.status != TransactionStatus::Planned {
                continue;
            }
            match journal.apply() {
                Ok(()) => journal.mark_committed(&transaction_path)?,
                Err(apply_error) => {
                    journal.rollback().with_context(|| {
                        format!(
                            "failed to roll back transaction {} after apply error: {apply_error:#}",
                            transaction_path.display()
                        )
                    })?;
                    journal.mark_rolled_back(&transaction_path)?;
                }
            }
            recovered.push(transaction_path);
        }
    }
    Ok(recovered)
}

/// Recovery plan — what actions to take for each quarantined/broken item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAction {
    pub target_type: RecoveryTarget,
    pub target_id: String,
    pub action: RecoveryActionType,
    pub reason: String,
    pub safe_to_auto_apply: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryTarget {
    Worktree,
    TaskNode,
    RunState,
    BridgeLease,
    FileChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryActionType {
    Quarantine,
    Restore,
    ResetToIdle,
    RetryTask,
    MergeWorktree,
    Discard,
    Skip,
}

/// Recovery result — what was actually done.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub run_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub actions_taken: Vec<RecoveryAction>,
    pub actions_skipped: Vec<RecoveryAction>,
    pub warnings: Vec<String>,
    pub metrics: RunMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPlan {
    pub run_id: String,
    pub previous_status: String,
    pub generated_at: String,
    pub actions: Vec<RecoveryAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub run_id: String,
    pub worktrees_clean: bool,
    pub state_consistent: bool,
    pub no_orphan_leases: bool,
    pub git_clean: bool,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunManifest {
    config: RunConfig,
    dag: TaskDAG,
    profile: TeamProfile,
    state: RunState,
    active_agents: std::collections::HashMap<String, AgentSession>,
    written_at: String,
}

pub fn assess_recovery(run_id: &str, project_path: &Path) -> Result<RecoveryPlan> {
    let lux_dir = project_path.join(".lux");
    let manifest = load_manifest(&lux_dir, run_id)?;
    let state = RunState::load(project_path)?;
    let previous_status = state.status.clone();
    let mut actions = Vec::new();
    let mut warnings = Vec::new();

    if state.run_id != run_id && !state.run_id.is_empty() {
        warnings.push(format!(
            "active run-state id {} differs from requested recovery id {}",
            state.run_id, run_id
        ));
    }

    if !matches_recoverable_status(&state.status) {
        warnings.push(format!(
            "run status {} is not an interrupted recovery status; plan will be conservative",
            state.status
        ));
    }

    if state.status == RunStatus::Quarantined.to_string() {
        actions.push(action(
            RecoveryTarget::RunState,
            run_id,
            RecoveryActionType::ResetToIdle,
            "quarantined run requires full reset to idle",
            true,
        ));
    } else if state.status != RunStatus::Recovering.to_string() {
        actions.push(action(
            RecoveryTarget::RunState,
            run_id,
            RecoveryActionType::Restore,
            "move interrupted run into recovering before cleanup",
            true,
        ));
    }

    for worktree in Worktree::list_all(&lux_dir)? {
        if worktree.status == WorktreeStatus::Active {
            actions.push(action(
                RecoveryTarget::Worktree,
                &worktree.id,
                RecoveryActionType::Quarantine,
                "active worktree was left orphaned by interrupted run",
                true,
            ));
        }
    }

    for node in manifest.dag.nodes.values() {
        if node.status == TaskStatus::InProgress {
            actions.push(action(
                RecoveryTarget::TaskNode,
                &node.id,
                RecoveryActionType::RetryTask,
                "task was in progress when run stopped",
                true,
            ));
        }
    }

    if stale_lease_count(&lux_dir)? > 0 {
        actions.push(action(
            RecoveryTarget::BridgeLease,
            "bridge-leases",
            RecoveryActionType::Restore,
            "stale bridge leases must be expired before resuming",
            true,
        ));
    }

    if !git_clean(project_path)? {
        actions.push(action(
            RecoveryTarget::FileChange,
            "project-worktree",
            RecoveryActionType::Skip,
            "uncommitted changes outside Lux worktrees require human review",
            false,
        ));
    }

    let plan = RecoveryPlan {
        run_id: run_id.to_string(),
        previous_status,
        generated_at: Utc::now().to_rfc3339(),
        actions,
        warnings,
    };
    atomic_write_json(&run_dir(&lux_dir, run_id).join("recovery-plan.json"), &plan)?;
    Ok(plan)
}

pub fn execute_recovery(
    run_id: &str,
    project_path: &Path,
    auto_apply_safe: bool,
) -> Result<RecoveryResult> {
    let lux_dir = project_path.join(".lux");
    let plan = assess_recovery(run_id, project_path)?;
    let mut manifest = load_manifest(&lux_dir, run_id)?;
    let previous_status = plan.previous_status.clone();
    let mut state = RunState::load(project_path)?;
    let mut actions_taken = Vec::new();
    let mut actions_skipped = Vec::new();
    let mut warnings = plan.warnings.clone();

    for planned in plan.actions {
        if !planned.safe_to_auto_apply && auto_apply_safe {
            actions_skipped.push(planned);
            continue;
        }
        let mut applied = planned.clone();
        match (&planned.target_type, &planned.action) {
            (RecoveryTarget::RunState, RecoveryActionType::Restore) => {
                if state.status != RunStatus::Recovering.to_string() {
                    state.transition_to(RunStatus::Recovering, "execute_recovery")?;
                    state.save(project_path)?;
                }
                applied.applied = true;
            }
            (RecoveryTarget::RunState, RecoveryActionType::ResetToIdle) => {
                state.transition_to(RunStatus::Idle, "execute_recovery_quarantine_reset")?;
                state.current_ticket_id = None;
                state.executor = Default::default();
                state.last_error = None;
                state.save(project_path)?;
                warnings.push("quarantined run was reset to idle".to_string());
                applied.applied = true;
            }
            (RecoveryTarget::Worktree, RecoveryActionType::Quarantine) => {
                let mut worktree = Worktree::load(&lux_dir, &planned.target_id)?;
                if worktree.status == WorktreeStatus::Active {
                    worktree.quarantine(&planned.reason)?;
                }
                applied.applied = true;
            }
            (RecoveryTarget::TaskNode, RecoveryActionType::RetryTask) => {
                if let Some(node) = manifest.dag.nodes.get_mut(&planned.target_id) {
                    if node.status == TaskStatus::InProgress {
                        node.status = TaskStatus::Pending;
                        node.assignee = None;
                        node.evidence_path = None;
                    }
                }
                applied.applied = true;
            }
            (RecoveryTarget::BridgeLease, RecoveryActionType::Restore) => {
                expire_stale_leases(&lux_dir)?;
                applied.applied = true;
            }
            _ => {
                actions_skipped.push(applied);
                continue;
            }
        }
        actions_taken.push(applied);
    }

    let tasks_remain = manifest
        .dag
        .nodes
        .values()
        .any(|node| node.status != TaskStatus::Done);
    if state.status != RunStatus::Idle.to_string() {
        let next = if tasks_remain {
            RunStatus::Planning
        } else {
            RunStatus::Idle
        };
        state.transition_to(next, "execute_recovery_complete")?;
        state.current_ticket_id = None;
        state.executor = Default::default();
        state.save(project_path)?;
    }

    manifest.state = state.clone();
    manifest.written_at = Utc::now().to_rfc3339();
    atomic_write_json(&run_dir(&lux_dir, run_id).join("manifest.json"), &manifest)?;
    write_projection(&lux_dir, run_id, &manifest.dag)?;

    let lifecycle = RunLifecycle::from_recovered_parts(
        RunConfig {
            project_path: project_path.to_path_buf(),
            ..manifest.config.clone()
        },
        manifest.dag.clone(),
        manifest.profile.clone(),
        state.clone(),
        manifest.active_agents.clone(),
        None,
    );
    let mut metrics = RunMetrics::snapshot(&lifecycle);
    metrics.recovery_count = previous_recovery_count(&lux_dir, run_id) + 1;
    metrics.quarantine_events = actions_taken
        .iter()
        .filter(|action| action.action == RecoveryActionType::Quarantine)
        .count();
    metrics.save(&lux_dir)?;

    let result = RecoveryResult {
        run_id: run_id.to_string(),
        previous_status,
        new_status: state.status.clone(),
        actions_taken,
        actions_skipped,
        warnings,
        metrics,
    };
    atomic_write_json(
        &run_dir(&lux_dir, run_id).join("recovery-result.json"),
        &result,
    )?;
    Ok(result)
}

pub fn validate_run_integrity(run_id: &str, project_path: &Path) -> Result<IntegrityReport> {
    let lux_dir = project_path.join(".lux");
    let mut issues = Vec::new();
    let active_worktrees = Worktree::list_all(&lux_dir)?
        .into_iter()
        .filter(|worktree| worktree.status == WorktreeStatus::Active)
        .map(|worktree| worktree.id)
        .collect::<Vec<_>>();
    if !active_worktrees.is_empty() {
        issues.push(format!(
            "active worktrees remain: {}",
            active_worktrees.join(", ")
        ));
    }
    let state = RunState::load(project_path)?;
    let state_consistent = state.run_id == run_id || state.run_id.is_empty();
    if !state_consistent {
        issues.push(format!(
            "run-state points at {} instead of {}",
            state.run_id, run_id
        ));
    }
    let orphan_leases = crate::lux_bridge_lease::list_active_leases(&lux_dir)?.len();
    if orphan_leases > 0 {
        issues.push(format!("{} active bridge lease(s) remain", orphan_leases));
    }
    let clean = git_clean(project_path)?;
    if !clean {
        issues.push("project git worktree has uncommitted changes".to_string());
    }
    let run_path = run_dir(&lux_dir, run_id);
    for required in [
        "manifest.json",
        "recovery-plan.json",
        "recovery-result.json",
    ] {
        if !run_path.join(required).exists() {
            issues.push(format!("missing run audit file {required}"));
        }
    }
    Ok(IntegrityReport {
        run_id: run_id.to_string(),
        worktrees_clean: active_worktrees.is_empty(),
        state_consistent,
        no_orphan_leases: orphan_leases == 0,
        git_clean: clean,
        issues,
    })
}

fn action(
    target_type: RecoveryTarget,
    target_id: &str,
    action: RecoveryActionType,
    reason: &str,
    safe_to_auto_apply: bool,
) -> RecoveryAction {
    RecoveryAction {
        target_type,
        target_id: target_id.to_string(),
        action,
        reason: reason.to_string(),
        safe_to_auto_apply,
        applied: false,
    }
}

fn matches_recoverable_status(status: &str) -> bool {
    matches!(
        status,
        "Interrupted" | "Failed" | "Quarantined" | "Recovering"
    )
}

fn load_manifest(lux_dir: &Path, run_id: &str) -> Result<RunManifest> {
    let path = run_dir(lux_dir, run_id).join("manifest.json");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read run manifest {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse run manifest {}", path.display()))
}

fn run_dir(lux_dir: &Path, run_id: &str) -> PathBuf {
    lux_dir.join("runs").join(run_id)
}

fn stale_lease_count(lux_dir: &Path) -> Result<usize> {
    let dir = lux_dir.join("bridge-leases");
    if !dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in
        fs::read_dir(&dir).with_context(|| format!("reading bridge lease dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("reading bridge lease {}", path.display()))?;
        let lease: BridgeLease = serde_json::from_str(&content)
            .with_context(|| format!("parsing bridge lease {}", path.display()))?;
        let expires_at = DateTime::parse_from_rfc3339(&lease.expires_at)
            .with_context(|| format!("bridge lease {} has invalid expires_at", lease.id))?
            .with_timezone(&Utc);
        if lease.status == LeaseStatus::Active && expires_at <= Utc::now() {
            count += 1;
        }
    }
    Ok(count)
}

fn git_clean(project_path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(["status", "--porcelain"])
        .output()
        .with_context(|| format!("failed to run git status in {}", project_path.display()))?;
    if !output.status.success() {
        return Ok(false);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn write_projection(lux_dir: &Path, run_id: &str, dag: &TaskDAG) -> Result<()> {
    let projected_at = Utc::now().to_rfc3339();
    let projection = dag
        .projection()
        .into_iter()
        .map(|node| TaskProjection {
            run_id: run_id.to_string(),
            node,
            projected_at: projected_at.clone(),
        })
        .collect::<Vec<_>>();
    atomic_write_json(
        &run_dir(lux_dir, run_id).join("task-projection.json"),
        &projection,
    )
}

fn previous_recovery_count(lux_dir: &Path, run_id: &str) -> usize {
    crate::lux_metrics::RunMetrics::load(lux_dir, run_id)
        .map(|metrics| metrics.recovery_count)
        .unwrap_or(0)
}
