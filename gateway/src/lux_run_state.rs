use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Schema version for run-state.json format migrations.
pub const RUN_STATE_SCHEMA_VERSION: u32 = 1;

/// Active run status values — exhaustive, no "other" variant.
/// These map to LoopState transitions but are persisted, not in-memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    Idle,
    Planning,
    AwaitingApproval,
    AwaitingEvidence,
    ExecutingTicket,
    Verifying,
    AwaitingPlayStart,
    AwaitingFeedback,
    Paused,
    Blocked,
    Completed,
    Failed,
    Interrupted,
    Recovering,
    Quarantined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    MaxContinuationsReached,
    MaxIterationsReached,
    StagnationLimit,
    ConsecutiveFailureLimit,
    MilestoneComplete,
    BlockerEscalationRequired,
    BlockerCycleDetected,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaxContinuationsReached => "max_continuations_reached",
            Self::MaxIterationsReached => "max_iterations_reached",
            Self::StagnationLimit => "stagnation_limit",
            Self::ConsecutiveFailureLimit => "consecutive_failure_limit",
            Self::MilestoneComplete => "milestone_complete",
            Self::BlockerEscalationRequired => "blocker_escalation_required",
            Self::BlockerCycleDetected => "blocker_cycle_detected",
        }
    }
}

impl std::str::FromStr for StopReason {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "max_continuations_reached" => Ok(Self::MaxContinuationsReached),
            "max_iterations_reached" => Ok(Self::MaxIterationsReached),
            "stagnation_limit" => Ok(Self::StagnationLimit),
            "consecutive_failure_limit" => Ok(Self::ConsecutiveFailureLimit),
            "milestone_complete" => Ok(Self::MilestoneComplete),
            "blocker_escalation_required" => Ok(Self::BlockerEscalationRequired),
            "blocker_cycle_detected" => Ok(Self::BlockerCycleDetected),
            other => Err(anyhow::anyhow!("unknown StopReason: {}", other)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationRunConfig {
    pub max_continuations: u32,
    pub max_blocker_depth: u32,
    pub max_blocker_attempts_per_ticket: u32,
    pub max_consecutive_blocker_generations: u32,
    pub stagnation_limit: u32,
    pub consecutive_failure_limit: u32,
}

impl Default for ContinuationRunConfig {
    fn default() -> Self {
        Self {
            max_continuations: 50,
            max_blocker_depth: 3,
            max_blocker_attempts_per_ticket: 3,
            max_consecutive_blocker_generations: 2,
            stagnation_limit: 3,
            consecutive_failure_limit: 3,
        }
    }
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "Idle",
            Self::Planning => "Planning",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::AwaitingEvidence => "AwaitingEvidence",
            Self::ExecutingTicket => "ExecutingTicket",
            Self::Verifying => "Verifying",
            Self::AwaitingPlayStart => "AwaitingPlayStart",
            Self::AwaitingFeedback => "AwaitingFeedback",
            Self::Paused => "Paused",
            Self::Blocked => "Blocked",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Interrupted => "Interrupted",
            Self::Recovering => "Recovering",
            Self::Quarantined => "Quarantined",
        })
    }
}

/// Approval gate type — what kind of human approval is pending?
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalGateType {
    ApprovePlan,
    ApproveDiff,
    HumanHelp,
}

impl fmt::Display for ApprovalGateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ApprovePlan => "ApprovePlan",
            Self::ApproveDiff => "ApproveDiff",
            Self::HumanHelp => "HumanHelp",
        })
    }
}

/// Executor adapter metadata — who/what is executing the current ticket?
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorInfo {
    pub kind: Option<String>,
    pub job_id: Option<String>,
    pub heartbeat_at: Option<String>,
}

/// Resume checkpoint data — enough to reconstruct previous state.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeCheckpoint {
    pub previous_status: Option<String>,
    pub checkpoint_data: Option<serde_json::Value>,
}

/// The canonical active-run state. Written ONLY by gateway.
/// Read by CLI, dashboard, plugin (via gateway API).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub schema_version: u32,
    pub seq: u64,
    pub run_id: String,
    pub status: String,
    pub goal_id: Option<String>,
    pub milestone_id: Option<String>,
    pub current_ticket_id: Option<String>,
    pub approval: ApprovalState,
    pub resume: ResumeData,
    pub executor: ExecutorInfo,
    pub last_error: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub continuation_count: u32,
    #[serde(default)]
    pub stagnation_count: u32,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default)]
    pub blocker_attempts: HashMap<String, u32>,
    #[serde(default)]
    pub consecutive_blocker_generations: u32,
    #[serde(default)]
    pub blocker_depth: u32,
    #[serde(default)]
    pub continuation_config: ContinuationRunConfig,
    pub pre_task_git_sha: Option<String>,
    pub team_run_id: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalState {
    pub gate: Option<String>,
    pub pending_transition: Option<String>,
    #[serde(default)]
    pub awaiting_since: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeData {
    pub previous_status: Option<String>,
    pub checkpoint: Option<String>,
}

impl RunState {
    /// Path to the canonical run-state file.
    pub fn path(project_path: &Path) -> PathBuf {
        project_path.join(".lux").join("run-state.json")
    }

    /// Create a fresh Idle run state.
    pub fn idle(_project_path: &Path) -> Result<Self> {
        Ok(Self {
            schema_version: RUN_STATE_SCHEMA_VERSION,
            seq: 0,
            run_id: String::new(),
            status: RunStatus::Idle.to_string(),
            goal_id: None,
            milestone_id: None,
            current_ticket_id: None,
            approval: ApprovalState::default(),
            resume: ResumeData::default(),
            executor: ExecutorInfo::default(),
            last_error: None,
            stop_reason: None,
            continuation_count: 0,
            stagnation_count: 0,
            consecutive_failures: 0,
            blocker_attempts: HashMap::new(),
            consecutive_blocker_generations: 0,
            blocker_depth: 0,
            continuation_config: ContinuationRunConfig::default(),
            pre_task_git_sha: None,
            team_run_id: None,
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Load from disk. Returns error if file missing (no silent fallback).
    pub fn load(project_path: &Path) -> Result<Self> {
        let path = Self::path(project_path);
        if !path.exists() {
            bail!(
                "run-state.json not found at {}. Run 'lux init' or start a run via gateway API.",
                path.display()
            );
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read run-state file {}", path.display()))?;
        let state: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse run-state file {}", path.display()))?;

        if state.schema_version > RUN_STATE_SCHEMA_VERSION {
            bail!(
                "run-state.json schema_version {} is newer than supported version {}. Upgrade LUX or migrate the file manually.",
                state.schema_version,
                RUN_STATE_SCHEMA_VERSION
            );
        }

        state.validate()?;

        Ok(state)
    }

    /// Atomically write to disk. Increments seq. Updates updated_at.
    pub fn save(&self, project_path: &Path) -> Result<()> {
        self.validate()?;

        let path = Self::path(project_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create run-state directory {}", parent.display())
            })?;
        }

        let previous_content = if path.exists() {
            Some(fs::read_to_string(&path).with_context(|| {
                format!("failed to read existing run-state file {}", path.display())
            })?)
        } else {
            None
        };

        crate::lux_io::atomic_write_json(&path, self)?;

        match Self::load(project_path) {
            Ok(_) => Ok(()),
            Err(error) => {
                match previous_content {
                    Some(content) => fs::write(&path, content).with_context(|| {
                        format!(
                            "failed to restore previous run-state file {}",
                            path.display()
                        )
                    })?,
                    None => {
                        if path.exists() {
                            fs::remove_file(&path).with_context(|| {
                                format!(
                                    "failed to remove invalid run-state file {}",
                                    path.display()
                                )
                            })?;
                        }
                    }
                }
                bail!("post-write run-state validation failed: {error}")
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.status.parse::<RunStatus>()?;
        Ok(())
    }

    /// Apply a partial continuation update with optimistic concurrency guard.
    ///
    /// Loads the current state from disk, validates that `expected_seq` matches
    /// the on-disk `seq`, applies the provided `patch` closure, increments `seq`,
    /// and atomically saves. Returns the saved state on success.
    ///
    /// Returns `Err` with a message containing "seq conflict" when `expected_seq`
    /// does not match the current on-disk `seq`.
    pub fn update_with_seq_check<F>(
        project_path: &Path,
        expected_seq: u64,
        expected_status: Option<&str>,
        patch: F,
    ) -> Result<Self>
    where
        F: FnOnce(&mut Self),
    {
        let mut state = Self::load(project_path)?;

        if state.seq != expected_seq {
            bail!(
                "seq conflict: expected {} but current seq is {}",
                expected_seq,
                state.seq
            );
        }

        if let Some(required_status) = expected_status {
            if state.status != required_status {
                bail!(
                    "status conflict: expected '{}' but current status is '{}'",
                    required_status,
                    state.status
                );
            }
        }

        patch(&mut state);
        state.seq += 1;
        state.updated_at = Utc::now().to_rfc3339();
        state.save(project_path)?;
        Ok(state)
    }

    /// Transition status. Returns new state with incremented seq and updated timestamp.
    pub fn transition_to(&mut self, new_status: RunStatus, _reason: &str) -> Result<()> {
        self.status = new_status.to_string();
        self.seq += 1;
        self.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    /// Migrate legacy continuation-state.json if present.
    /// Logs warning and attempts atomic migration. Does not delete old file.
    pub fn migrate_legacy_continuation_state(project_path: &Path) -> Result<bool> {
        let legacy_path = project_path.join(".lux").join("continuation-state.json");
        if !legacy_path.exists() {
            return Ok(false);
        }

        eprintln!(
            "⚠️  [lux] Legacy continuation-state.json detected. Migrating to run-state.json..."
        );

        let content = fs::read_to_string(&legacy_path).with_context(|| {
            format!(
                "failed to read legacy continuation state file {}",
                legacy_path.display()
            )
        })?;
        let legacy: serde_json::Value =
            serde_json::from_str(&content).context("failed to parse legacy continuation-state")?;

        let mut migrated = Self::idle(project_path)?;

        if let Some(obj) = legacy.as_object() {
            if let Some(ticket_id) = obj
                .get("current_ticket_id")
                .and_then(|value| value.as_str())
            {
                migrated.current_ticket_id = Some(ticket_id.to_string());
            }
            migrated.continuation_count = read_u32(obj, "continuation_count");
            migrated.stagnation_count = read_u32(obj, "stagnation_count");
            migrated.consecutive_failures = read_u32(obj, "consecutive_failures");

            if let Some(in_flight) = obj.get("inFlight").and_then(|value| value.as_bool()) {
                if in_flight {
                    migrated.executor.kind = Some("opencode".to_string());
                }
            }

            let status = obj
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("Idle");
            let stop_reason = obj.get("stop_reason").and_then(|value| value.as_str());
            let in_flight = obj
                .get("inFlight")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            migrated.status = map_legacy_continuation_status(
                status,
                in_flight,
                migrated.current_ticket_id.as_deref(),
                stop_reason,
            )?
            .to_string();
        }

        if let Err(error) = migrated.save(project_path) {
            let mut quarantined = Self::idle(project_path)?;
            quarantined.status = RunStatus::Quarantined.to_string();
            quarantined.last_error = Some(format!("legacy continuation migration failed: {error}"));
            quarantined.save(project_path)?;
            bail!("legacy continuation migration failed: {error}");
        }

        let deprecated_path = project_path
            .join(".lux")
            .join("continuation-state.json.deprecated");
        let _ = fs::rename(&legacy_path, &deprecated_path);

        eprintln!(
            "✅  [lux] Migration complete. Legacy state preserved at continuation-state.json.deprecated"
        );

        Ok(true)
    }
}

fn read_u32(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> u32 {
    obj.get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}

fn map_legacy_continuation_status(
    status: &str,
    in_flight: bool,
    current_ticket_id: Option<&str>,
    stop_reason: Option<&str>,
) -> Result<RunStatus> {
    match status {
        "Complete" => Ok(RunStatus::Completed),
        "Active" if in_flight || current_ticket_id.is_some() => Ok(RunStatus::ExecutingTicket),
        "Active" => Ok(RunStatus::Planning),
        "Stopped" if matches!(stop_reason, Some("all_complete" | "milestone_complete")) => {
            Ok(RunStatus::Completed)
        }
        "Stopped" => Ok(RunStatus::Interrupted),
        "Error"
            if matches!(
                stop_reason,
                Some("blocker_cycle_detected" | "blocker_escalation_required")
            ) =>
        {
            Ok(RunStatus::Quarantined)
        }
        "Error" => Ok(RunStatus::Failed),
        "Idle" => Ok(RunStatus::Idle),
        other => Err(anyhow::anyhow!(
            "unknown legacy ContinuationStatus: {}",
            other
        )),
    }
}

impl std::str::FromStr for RunStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Idle" => Ok(Self::Idle),
            "Planning" => Ok(Self::Planning),
            "AwaitingApproval" => Ok(Self::AwaitingApproval),
            "AwaitingEvidence" => Ok(Self::AwaitingEvidence),
            "ExecutingTicket" => Ok(Self::ExecutingTicket),
            "Verifying" => Ok(Self::Verifying),
            "AwaitingPlayStart" => Ok(Self::AwaitingPlayStart),
            "AwaitingFeedback" => Ok(Self::AwaitingFeedback),
            "Paused" => Ok(Self::Paused),
            "Blocked" => Ok(Self::Blocked),
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            "Interrupted" => Ok(Self::Interrupted),
            "Recovering" => Ok(Self::Recovering),
            "Quarantined" => Ok(Self::Quarantined),
            other => Err(anyhow::anyhow!("unknown RunStatus: {}", other)),
        }
    }
}

pub fn transition_run_state(
    project_path: &Path,
    expected: &RunStatus,
    next: &RunStatus,
) -> anyhow::Result<()> {
    let mut state = RunState::load(project_path)?;
    let current = state
        .status
        .parse::<RunStatus>()
        .map_err(|_| anyhow::anyhow!("invalid run status: {}", state.status))?;
    if current != *expected {
        bail!(
            "Run status transition mismatch: expected {:?}, found {:?}. Cannot transition to {:?}.",
            expected,
            current,
            next
        );
    }
    state.transition_to(next.clone(), &format!("{:?} -> {:?}", expected, next))?;
    state.save(project_path)?;
    Ok(())
}
