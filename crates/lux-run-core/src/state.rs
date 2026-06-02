use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::RunStatus;

/// Schema version for run-state.json format migrations.
pub const RUN_STATE_SCHEMA_VERSION: u32 = 1;

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
    #[serde(default)]
    pub ticket_id: Option<String>,
    pub current_ticket_id: Option<String>,
    pub approval: ApprovalState,
    pub resume: ResumeData,
    pub executor: ExecutorInfo,
    #[serde(default)]
    pub verification_policy: Option<String>,
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
    #[serde(default)]
    pub continuation_status: Option<String>,
    #[serde(default)]
    pub planned_at: Option<String>,
    #[serde(default)]
    pub dispatch_ready_at: Option<String>,
    #[serde(default)]
    pub executing_at: Option<String>,
    #[serde(default)]
    pub verifying_at: Option<String>,
    #[serde(default)]
    pub blocked_at: Option<String>,
    #[serde(default)]
    pub retry_ready_at: Option<String>,
    #[serde(default)]
    pub resumed_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub failed_at: Option<String>,
    #[serde(default)]
    pub quarantined_at: Option<String>,
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
    pub fn path(project_path: &Path) -> PathBuf {
        project_path.join(".lux").join("run-state.json")
    }

    pub fn idle() -> Result<Self> {
        Ok(Self {
            schema_version: RUN_STATE_SCHEMA_VERSION,
            seq: 0,
            run_id: String::new(),
            status: RunStatus::Idle.to_string(),
            goal_id: None,
            milestone_id: None,
            ticket_id: None,
            current_ticket_id: None,
            approval: ApprovalState::default(),
            resume: ResumeData::default(),
            executor: ExecutorInfo::default(),
            verification_policy: None,
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
            continuation_status: None,
            planned_at: None,
            dispatch_ready_at: None,
            executing_at: None,
            verifying_at: None,
            blocked_at: None,
            retry_ready_at: None,
            resumed_at: None,
            completed_at: None,
            failed_at: None,
            quarantined_at: None,
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.status.parse::<RunStatus>()?;
        Ok(())
    }

    pub fn transition_to(&mut self, new_status: RunStatus, _reason: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.status = new_status.to_string();
        self.updated_at = now.clone();
        self.record_transition_timestamp(&new_status, now);
        Ok(())
    }

    fn record_transition_timestamp(&mut self, status: &RunStatus, timestamp: String) {
        match status {
            RunStatus::Planning | RunStatus::Planned => self.planned_at = Some(timestamp),
            RunStatus::DispatchReady => self.dispatch_ready_at = Some(timestamp),
            RunStatus::Executing | RunStatus::ExecutingTicket => {
                self.executing_at = Some(timestamp)
            }
            RunStatus::Verifying => self.verifying_at = Some(timestamp),
            RunStatus::Blocked => self.blocked_at = Some(timestamp),
            RunStatus::RetryReady => self.retry_ready_at = Some(timestamp),
            RunStatus::Resumed | RunStatus::Recovering => self.resumed_at = Some(timestamp),
            RunStatus::Completed => self.completed_at = Some(timestamp),
            RunStatus::Failed => self.failed_at = Some(timestamp),
            RunStatus::Quarantined => self.quarantined_at = Some(timestamp),
            RunStatus::Idle
            | RunStatus::AwaitingApproval
            | RunStatus::AwaitingEvidence
            | RunStatus::AwaitingPlayStart
            | RunStatus::AwaitingFeedback
            | RunStatus::Paused
            | RunStatus::Interrupted => {}
        }
    }
}
