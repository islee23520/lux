use std::{
    collections::HashMap,
    fs,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    lux_events::{EventRouter, LuxEvent},
    lux_io::atomic_write_json,
    lux_lock::{acquire_lux_lock, LuxLockGuard, DEFAULT_STALE_THRESHOLD_SECS},
    lux_metrics::RunMetrics,
    lux_next_goal::{
        persist_current_goal, select_next_goal, write_awaiting_evidence_blocker, NextGoal,
    },
    lux_roadmap::{RoadmapPhaseStatus, RoadmapReality},
    lux_run_recover::ExecutionSession,
    lux_run_state::{ApprovalGateType, StopReason},
    lux_run_state::{RunState, RunStatus},
    lux_task_dag::{TaskDAG, TaskNodeProjection, TaskStatus},
    lux_team_profile::{RoleMapping, TeamProfile, TeamSizePreset},
    lux_verification::{
        check_verification_gate, required_tier_for_action, TieredVerificationResult,
    },
};

pub const MILESTONE_PUSH_TRANSITION: &str = "milestone_push";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Planned,
    Committed,
    RolledBack,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionOperation {
    WriteFile {
        path: PathBuf,
        content: String,
        #[serde(default)]
        before_content: Option<String>,
    },
    RenameFile {
        from: PathBuf,
        to: PathBuf,
        #[serde(default)]
        before_from_content: Option<String>,
        #[serde(default)]
        before_to_content: Option<String>,
    },
    DeleteFile {
        path: PathBuf,
        #[serde(default)]
        before_content: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionJournal {
    pub id: String,
    pub created_at: String,
    pub status: TransactionStatus,
    pub operations: Vec<TransactionOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestonePushApproval {
    pub project_path: PathBuf,
    pub milestone_id: Option<String>,
    pub evidence_path: PathBuf,
    pub git_sha: String,
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Goal text or spec reference to automate
    pub goal: Option<String>,
    /// Team size preset for adaptive team-mode composition
    #[arg(long = "team-size", value_enum, default_value_t = TeamSizePreset::Medium)]
    pub team_size: TeamSizePreset,
    /// Plan and project the run without dispatching execution tasks
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Recover a previously persisted run id from .lux/runs/<id>
    #[arg(long)]
    pub recover: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    pub project_path: PathBuf,
    pub team_preset: TeamSizePreset,
    pub dry_run: bool,
    pub goal: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub role: String,
    pub category: String,
    pub skills: Vec<String>,
    pub task_id: Option<String>,
    pub status: String,
    pub started_at: String,
}

pub struct RunLifecycle {
    pub config: RunConfig,
    pub dag: TaskDAG,
    pub profile: TeamProfile,
    pub state: RunState,
    pub active_agents: HashMap<String, AgentSession>,
    pub event_router: EventRouter,
    lock_guard: Option<LuxLockGuard>,
}

impl TransactionJournal {
    pub fn planned(
        run_id: &str,
        project_path: &Path,
        operations: Vec<TransactionOperation>,
    ) -> Result<Self> {
        let mut journal = Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            status: TransactionStatus::Planned,
            operations,
        };
        journal.capture_before_state()?;
        let path = journal_path(project_path, run_id, &journal.id);
        atomic_write_json(&path, &journal)?;
        Ok(journal)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read transaction journal {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse transaction journal {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        atomic_write_json(path, self)
    }

    pub fn apply(&self) -> Result<()> {
        for operation in &self.operations {
            apply_transaction_operation(operation)?;
        }
        Ok(())
    }

    pub fn rollback(&self) -> Result<()> {
        for operation in self.operations.iter().rev() {
            rollback_transaction_operation(operation)?;
        }
        Ok(())
    }

    pub fn mark_committed(&mut self, path: &Path) -> Result<()> {
        self.status = TransactionStatus::Committed;
        self.save(path)
    }

    pub fn mark_rolled_back(&mut self, path: &Path) -> Result<()> {
        self.status = TransactionStatus::RolledBack;
        self.save(path)
    }

    fn capture_before_state(&mut self) -> Result<()> {
        for operation in &mut self.operations {
            match operation {
                TransactionOperation::WriteFile {
                    path,
                    before_content,
                    ..
                }
                | TransactionOperation::DeleteFile {
                    path,
                    before_content,
                } => {
                    *before_content = read_optional_file(path)?;
                }
                TransactionOperation::RenameFile {
                    from,
                    to,
                    before_from_content,
                    before_to_content,
                } => {
                    *before_from_content = read_optional_file(from)?;
                    *before_to_content = read_optional_file(to)?;
                }
            }
        }
        Ok(())
    }
}

impl RunLifecycle {
    pub fn from_recovered_parts(
        config: RunConfig,
        dag: TaskDAG,
        profile: TeamProfile,
        state: RunState,
        active_agents: HashMap<String, AgentSession>,
        lock_guard: Option<LuxLockGuard>,
    ) -> Self {
        Self {
            config,
            dag,
            profile,
            state,
            active_agents,
            event_router: EventRouter::new(),
            lock_guard,
        }
    }

    fn ensure_lock_held(&self) -> Result<()> {
        if self.lock_guard.is_none() {
            bail!("lux run lock is not held for run {}", self.state.run_id);
        }
        Ok(())
    }

    fn save_metrics_best_effort(&self) {
        let metrics = RunMetrics::snapshot(self);
        if let Err(err) = metrics.save(&self.config.project_path.join(".lux")) {
            eprintln!("[lux-run] failed to write metrics snapshot: {err:#}");
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProjection {
    pub run_id: String,
    pub node: TaskNodeProjection,
    pub projected_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunManifest {
    config: RunConfig,
    dag: TaskDAG,
    profile: TeamProfile,
    state: RunState,
    active_agents: HashMap<String, AgentSession>,
    written_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunPlan {
    run_id: String,
    goal: Option<String>,
    next_goal: NextGoal,
    ambiguity_score: f64,
    task_count: usize,
    ready_count: usize,
    roles: Vec<RoleMapping>,
    generated_at: String,
}

pub fn run_command(args: &RunArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let mut lifecycle = if let Some(run_id) = &args.recover {
        let result = crate::lux_run_recover::execute_recovery(run_id, &project_root, true)?;
        eprintln!(
            "[lux-run] recovered run {} from {} to {}",
            result.run_id, result.previous_status, result.new_status
        );
        recover_run(&project_root, run_id)?
    } else {
        start_run(args)?
    };

    let run_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        plan_phase(&mut lifecycle)?;
        let projection = project_tasks(&lifecycle)?;
        eprintln!(
            "[lux-run] projected {} task(s) for run {}",
            projection.len(),
            lifecycle.state.run_id
        );

        if !lifecycle.config.dry_run {
            let ready = lifecycle
                .dag
                .ready_nodes()
                .into_iter()
                .map(|node| node.id)
                .collect::<Vec<_>>();
            for task_id in ready.iter().take(4) {
                execute_task(&mut lifecycle, task_id)?;
            }
        }

        complete_run(&mut lifecycle)
    }));

    match run_result {
        Ok(result) => result,
        Err(payload) => {
            lifecycle.save_metrics_best_effort();
            resume_unwind(payload);
        }
    }
}

pub fn start_run(args: &RunArgs) -> Result<RunLifecycle> {
    let project_path = resolve_project_root(&args.project_path)?;
    let lux_dir = project_path.join(".lux");
    let lock_guard = acquire_lux_lock(
        &lux_dir,
        "lux-run",
        "active run",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )?;

    let profile = TeamProfile::load(&lux_dir)?;
    let spec = crate::lux_spec::lux_load(&project_path)?;
    let dag = TaskDAG::from_spec(&spec);
    let mut state = RunState::load(&project_path)?;
    state.transition_to(RunStatus::Planning, "starting")?;
    state.run_id = Uuid::new_v4().to_string();
    state.goal_id = args.goal.clone();
    state.current_ticket_id = None;
    state.last_error = None;
    state.blocker_attempts.clear();
    state.consecutive_blocker_generations = 0;
    state.blocker_depth = 0;
    state.save(&project_path)?;

    let config = RunConfig {
        project_path: project_path.clone(),
        team_preset: args.team_size.clone(),
        dry_run: args.dry_run,
        goal: args.goal.clone(),
    };

    let lifecycle = RunLifecycle {
        config,
        dag,
        profile,
        state,
        active_agents: HashMap::new(),
        event_router: EventRouter::new(),
        lock_guard: Some(lock_guard),
    };
    save_manifest(&lifecycle)?;
    eprintln!("[lux-run] started run {}", lifecycle.state.run_id);
    Ok(lifecycle)
}

pub fn plan_phase(lifecycle: &mut RunLifecycle) -> Result<()> {
    lifecycle.ensure_lock_held()?;
    lifecycle
        .state
        .transition_to(RunStatus::Planning, "plan_phase")?;
    lifecycle.state.save(&lifecycle.config.project_path)?;

    let spec = crate::lux_spec::lux_load(&lifecycle.config.project_path)?;
    let ambiguity = crate::lux_ambiguity::calculate_ambiguity(&spec);
    let ready_count = lifecycle.dag.ready_nodes().len();
    let roles = adaptive_team_composition(lifecycle);
    let next_goal = select_next_goal(
        &lifecycle.config.project_path,
        &spec,
        &ambiguity,
        &lifecycle.dag,
        lifecycle.config.goal.clone(),
    )?;
    persist_current_goal(
        &lifecycle.config.project_path,
        &lifecycle.state.run_id,
        lifecycle.config.goal.clone(),
        &next_goal,
    )?;
    let plan = RunPlan {
        run_id: lifecycle.state.run_id.clone(),
        goal: lifecycle.config.goal.clone(),
        next_goal,
        ambiguity_score: ambiguity.overall_score,
        task_count: lifecycle.dag.nodes.len(),
        ready_count,
        roles,
        generated_at: Utc::now().to_rfc3339(),
    };

    let plan_path = run_dir(lifecycle).join("plan.json");
    atomic_write_json(&plan_path, &plan)?;
    save_manifest(lifecycle)?;
    Ok(())
}

pub fn execute_task(lifecycle: &mut RunLifecycle, task_id: &str) -> Result<()> {
    lifecycle.ensure_lock_held()?;

    let project_path = &lifecycle.config.project_path;
    let run_id = &lifecycle.state.run_id;
    if let Some(existing) = ExecutionSession::load(project_path, run_id)? {
        bail!(
            "execution already in progress for run {} (ticket {}); \
             call recover_stuck_executions first if the session is stale",
            run_id,
            existing.ticket_id
        );
    }

    let ready = lifecycle
        .dag
        .ready_nodes()
        .into_iter()
        .any(|node| node.id == task_id);
    if !ready {
        lifecycle.dag.mark_blocked(
            task_id,
            Some("Task is not ready; dependencies remain open".to_string()),
        );
        save_manifest(lifecycle)?;
        bail!("task is not ready: {task_id}");
    }

    let roles = adaptive_team_composition(lifecycle);
    let Some(role) = roles.first().cloned() else {
        bail!("team profile produced no roles for preset");
    };

    if let Some(node) = lifecycle.dag.nodes.get_mut(task_id) {
        node.status = TaskStatus::InProgress;
        node.assignee = Some(role.role.clone());
    }

    lifecycle.state.begin_task_execution(
        &lifecycle.config.project_path,
        task_id,
        "team-mode",
        &format!("{}:{task_id}", lifecycle.state.run_id),
        current_git_sha(&lifecycle.config.project_path),
    )?;

    ExecutionSession::begin(
        &lifecycle.config.project_path,
        &lifecycle.state.run_id,
        task_id,
        300,
        3,
    )?;

    let session = AgentSession {
        role: role.role.clone(),
        category: role.category.clone(),
        skills: role.skills.clone(),
        task_id: Some(task_id.to_string()),
        status: "projected".to_string(),
        started_at: Utc::now().to_rfc3339(),
    };
    lifecycle.active_agents.insert(role.role.clone(), session);

    let dispatch_path = run_dir(lifecycle)
        .join("dispatch")
        .join(format!("{task_id}.json"));
    atomic_write_json(&dispatch_path, &lifecycle.active_agents[&role.role])?;

    if let Some(node) = lifecycle.dag.nodes.get_mut(task_id) {
        node.status = crate::lux_task_dag::TaskStatus::AwaitingEvidence;
        node.evidence_path = Some(format!(
            ".lux/runs/{}/dispatch/{task_id}.json",
            lifecycle.state.run_id
        ));
    }

    lifecycle
        .event_router
        .route(&LuxEvent::AutonomousDispatchRequested {
            run_id: lifecycle.state.run_id.clone(),
            ticket_id: task_id.to_string(),
        });

    save_manifest(lifecycle)?;
    lifecycle.save_metrics_best_effort();
    eprintln!("[lux-run] projected task {task_id} to role {}", role.role);
    Ok(())
}

pub fn adaptive_team_composition(lifecycle: &RunLifecycle) -> Vec<RoleMapping> {
    let domain = current_domain(lifecycle).unwrap_or_else(|| "gameplay".to_string());
    let skill_bindings = lifecycle
        .profile
        .skill_bindings
        .iter()
        .find(|binding| binding.domain == domain)
        .map(|binding| binding.skills.clone())
        .unwrap_or_default();

    let mut roles = lifecycle
        .profile
        .roles_for_preset(&lifecycle.config.team_preset)
        .into_iter()
        .map(|role| {
            let mut role = role.clone();
            for skill in &skill_bindings {
                if !role.skills.contains(skill) {
                    role.skills.push(skill.clone());
                }
            }
            role
        })
        .collect::<Vec<_>>();

    if !skill_bindings.is_empty() {
        roles.sort_by_key(|role| {
            let matches = role
                .skills
                .iter()
                .filter(|skill| skill_bindings.contains(*skill))
                .count();
            std::cmp::Reverse(matches)
        });
    }

    roles.truncate(4);
    roles
}

pub fn project_tasks(lifecycle: &RunLifecycle) -> Result<Vec<TaskProjection>> {
    let state = RunState::load(&lifecycle.config.project_path)?;
    let projected_at = Utc::now().to_rfc3339();
    let projection = lifecycle
        .dag
        .projection()
        .into_iter()
        .map(|node| TaskProjection {
            run_id: state.run_id.clone(),
            node,
            projected_at: projected_at.clone(),
        })
        .collect::<Vec<_>>();
    let path = run_dir(lifecycle).join("task-projection.json");
    atomic_write_json(&path, &projection)?;
    Ok(projection)
}

pub fn complete_run(lifecycle: &mut RunLifecycle) -> Result<()> {
    lifecycle.ensure_lock_held()?;
    let failed = lifecycle
        .dag
        .nodes
        .values()
        .any(|node| node.status == TaskStatus::Blocked);
    let awaiting_evidence = lifecycle
        .dag
        .nodes
        .values()
        .any(|node| node.status == TaskStatus::AwaitingEvidence);
    let next = if failed {
        RunStatus::Failed
    } else if awaiting_evidence {
        write_awaiting_evidence_blocker(
            &lifecycle.config.project_path,
            &lifecycle.state.run_id,
            &lifecycle.dag,
        )?;
        RunStatus::AwaitingEvidence
    } else {
        RunStatus::Completed
    };
    lifecycle.state.transition_to(next, "complete_run")?;
    lifecycle.state.current_ticket_id = None;
    lifecycle.state.executor.heartbeat_at = Some(Utc::now().to_rfc3339());
    lifecycle.state.save(&lifecycle.config.project_path)?;

    RunMetrics::snapshot(lifecycle).save(&lifecycle.config.project_path.join(".lux"))?;
    save_manifest(lifecycle)?;
    lifecycle.lock_guard = None;
    eprintln!(
        "[lux-run] completed run with status {}",
        lifecycle.state.status
    );
    Ok(())
}

pub fn begin_milestone_push_approval(
    run_state: &mut RunState,
    evidence_path: &Path,
    git_sha_preview: Option<String>,
) -> Result<()> {
    if !evidence_path.exists() {
        bail!(
            "milestone push approval requires existing T3 evidence at {}",
            evidence_path.display()
        );
    }
    let evidence_content = fs::read_to_string(evidence_path)
        .with_context(|| format!("failed to read T3 evidence at {}", evidence_path.display()))?;
    let tiered_result: TieredVerificationResult = serde_json::from_str(&evidence_content)
        .with_context(|| format!("failed to parse T3 evidence at {}", evidence_path.display()))?;
    let required = required_tier_for_action("milestone_push");
    check_verification_gate(&tiered_result, required)?;
    let awaiting_since = Utc::now().to_rfc3339();
    run_state.transition_to(RunStatus::AwaitingApproval, "begin_milestone_push_approval")?;
    run_state.approval.gate = Some(ApprovalGateType::ApproveDiff.to_string());
    run_state.approval.pending_transition = Some(MILESTONE_PUSH_TRANSITION.to_string());
    run_state.approval.awaiting_since = Some(awaiting_since.clone());
    run_state.approval.created_at = Some(awaiting_since);
    run_state.resume.checkpoint = Some(
        serde_json::json!({
            "evidencePath": evidence_path.display().to_string(),
            "gitShaPreview": git_sha_preview,
        })
        .to_string(),
    );
    Ok(())
}

pub fn execute_milestone_push(
    run_state: &mut RunState,
    roadmap: &mut RoadmapReality,
    approval: &MilestonePushApproval,
) -> Result<()> {
    execute_milestone_push_with_runner(run_state, roadmap, approval, run_git_push)
}

pub fn execute_milestone_push_with_runner<F>(
    run_state: &mut RunState,
    roadmap: &mut RoadmapReality,
    approval: &MilestonePushApproval,
    push_runner: F,
) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    validate_milestone_push_approval(run_state, approval)?;
    push_runner(&approval.project_path)?;

    let mut next_roadmap = roadmap.clone();
    mark_roadmap_phase_pushed(&mut next_roadmap, run_state, approval)?;

    let mut next_state = run_state.clone();
    next_state.transition_to(RunStatus::Completed, StopReason::MilestoneComplete.as_str())?;
    next_state.current_ticket_id = None;
    next_state.approval = Default::default();
    next_state.stop_reason = Some(StopReason::MilestoneComplete.as_str().to_string());
    next_state.last_error = None;
    next_state.executor.heartbeat_at = Some(Utc::now().to_rfc3339());

    let run_id = if next_state.run_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        next_state.run_id.clone()
    };
    let run_state_path = RunState::path(&approval.project_path);
    let roadmap_path = crate::lux_roadmap::roadmap_file_path(&approval.project_path);
    let operations = vec![
        TransactionOperation::WriteFile {
            path: run_state_path,
            content: serde_json::to_string_pretty(&next_state)
                .context("failed to serialize completed run-state")?,
            before_content: None,
        },
        TransactionOperation::WriteFile {
            path: roadmap_path,
            content: serde_json::to_string_pretty(&next_roadmap)
                .context("failed to serialize pushed roadmap")?,
            before_content: None,
        },
    ];

    let mut journal = TransactionJournal::planned(&run_id, &approval.project_path, operations)?;
    let path = journal_path(&approval.project_path, &run_id, &journal.id);
    journal.apply()?;
    journal.mark_committed(&path)?;

    *run_state = next_state;
    *roadmap = next_roadmap;
    Ok(())
}

fn validate_milestone_push_approval(
    run_state: &RunState,
    approval: &MilestonePushApproval,
) -> Result<()> {
    if run_state.status != RunStatus::AwaitingApproval.to_string() {
        bail!(
            "milestone push requires AwaitingApproval status, found {}",
            run_state.status
        );
    }
    if run_state.approval.gate.as_deref() != Some("ApproveDiff") {
        bail!("milestone push requires approval.gate=ApproveDiff");
    }
    if run_state.approval.pending_transition.as_deref() != Some(MILESTONE_PUSH_TRANSITION) {
        bail!("milestone push requires approval.pending_transition=milestone_push");
    }
    let evidence_path = resolve_project_path(&approval.project_path, &approval.evidence_path);
    if !evidence_path.exists() {
        bail!(
            "milestone push requires T3 evidence before roadmap push: {}",
            evidence_path.display()
        );
    }
    Ok(())
}

fn mark_roadmap_phase_pushed(
    roadmap: &mut RoadmapReality,
    run_state: &RunState,
    approval: &MilestonePushApproval,
) -> Result<()> {
    let phase = if let Some(milestone_id) = approval
        .milestone_id
        .as_ref()
        .or(run_state.milestone_id.as_ref())
    {
        roadmap
            .phases
            .iter_mut()
            .find(|phase| phase.name == *milestone_id || phase.name.starts_with(milestone_id))
            .with_context(|| format!("roadmap milestone not found: {milestone_id}"))?
    } else {
        roadmap
            .phases
            .iter_mut()
            .find(|phase| phase.status != RoadmapPhaseStatus::Pushed)
            .context("no roadmap phase available for milestone push")?
    };

    phase.status = RoadmapPhaseStatus::Pushed;
    phase.pushed_at = Some(Utc::now().to_rfc3339());
    phase.push_git_sha = Some(approval.git_sha.clone());
    phase.push_evidence_path = Some(approval.evidence_path.display().to_string());
    roadmap.updated_at = Utc::now().to_rfc3339();
    roadmap.validate()?;
    Ok(())
}

fn run_git_push(project_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .arg("push")
        .output()
        .with_context(|| format!("failed to run git push in {}", project_path.display()))?;
    if !output.status.success() {
        bail!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

pub fn recover_run(project_path: &Path, run_id: &str) -> Result<RunLifecycle> {
    let lux_dir = project_path.join(".lux");
    let lock_guard = acquire_lux_lock(
        &lux_dir,
        "lux-run",
        "recover run",
        DEFAULT_STALE_THRESHOLD_SECS,
        false,
    )?;
    let manifest_path = lux_dir.join("runs").join(run_id).join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read run manifest {}", manifest_path.display()))?;
    let manifest: RunManifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse run manifest {}", manifest_path.display()))?;

    let quarantine_dir = lux_dir.join("runs").join(run_id).join("quarantined");
    if quarantine_dir.exists() {
        bail!(
            "run {} has quarantined worktrees at {}; inspect before recovery",
            run_id,
            quarantine_dir.display()
        );
    }

    let mut lifecycle = RunLifecycle {
        config: RunConfig {
            project_path: project_path.to_path_buf(),
            ..manifest.config
        },
        dag: manifest.dag,
        profile: manifest.profile,
        state: manifest.state,
        active_agents: manifest.active_agents,
        event_router: EventRouter::new(),
        lock_guard: Some(lock_guard),
    };
    lifecycle
        .state
        .transition_to(RunStatus::Recovering, "recover_run")?;
    lifecycle.state.save(project_path)?;
    let _ = project_tasks(&lifecycle)?;
    save_manifest(&lifecycle)?;
    eprintln!("[lux-run] recovered run {run_id}");
    Ok(lifecycle)
}

fn current_git_sha(project_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|sha| !sha.is_empty())
}

fn save_manifest(lifecycle: &RunLifecycle) -> Result<()> {
    let manifest = RunManifest {
        config: lifecycle.config.clone(),
        dag: lifecycle.dag.clone(),
        profile: lifecycle.profile.clone(),
        state: lifecycle.state.clone(),
        active_agents: lifecycle.active_agents.clone(),
        written_at: Utc::now().to_rfc3339(),
    };
    atomic_write_json(&run_dir(lifecycle).join("manifest.json"), &manifest)
}

fn run_dir(lifecycle: &RunLifecycle) -> PathBuf {
    lifecycle
        .config
        .project_path
        .join(".lux")
        .join("runs")
        .join(&lifecycle.state.run_id)
}

pub fn journal_path(project_path: &Path, run_id: &str, transaction_id: &str) -> PathBuf {
    project_path
        .join(".lux")
        .join("runs")
        .join(run_id)
        .join("transactions")
        .join(format!("{transaction_id}.json"))
}

fn apply_transaction_operation(operation: &TransactionOperation) -> Result<()> {
    match operation {
        TransactionOperation::WriteFile { path, content, .. } => atomic_write_text(path, content),
        TransactionOperation::RenameFile { from, to, .. } => {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            if from.exists() {
                fs::rename(from, to).with_context(|| {
                    format!("failed to rename {} to {}", from.display(), to.display())
                })?;
            }
            Ok(())
        }
        TransactionOperation::DeleteFile { path, .. } => {
            if path.exists() {
                fs::remove_file(path)
                    .with_context(|| format!("failed to delete {}", path.display()))?;
            }
            Ok(())
        }
    }
}

fn rollback_transaction_operation(operation: &TransactionOperation) -> Result<()> {
    match operation {
        TransactionOperation::WriteFile {
            path,
            before_content,
            ..
        }
        | TransactionOperation::DeleteFile {
            path,
            before_content,
        } => restore_optional_file(path, before_content.as_deref()),
        TransactionOperation::RenameFile {
            from,
            to,
            before_from_content,
            before_to_content,
        } => {
            restore_optional_file(from, before_from_content.as_deref())?;
            restore_optional_file(to, before_to_content.as_deref())
        }
    }
}

fn read_optional_file(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(path)
        .map(Some)
        .with_context(|| format!("failed to read {}", path.display()))
}

fn restore_optional_file(path: &Path, content: Option<&str>) -> Result<()> {
    match content {
        Some(content) => atomic_write_text(path, content),
        None => {
            if path.exists() {
                fs::remove_file(path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
            Ok(())
        }
    }
}

fn atomic_write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))
}

fn resolve_project_path(project_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_path.join(path)
    }
}

fn current_domain(lifecycle: &RunLifecycle) -> Option<String> {
    lifecycle
        .dag
        .ready_nodes()
        .first()
        .and_then(|node| node.id.strip_prefix("task-"))
        .map(|rest| match rest {
            rest if rest.starts_with("technical-architecture-") => {
                "technical-architecture".to_string()
            }
            rest if rest.starts_with("build-release-") => "build-release".to_string(),
            rest if rest.starts_with("art-style-") => "art-style".to_string(),
            rest if rest.starts_with("ui-ux-") => "ui-ux".to_string(),
            rest if rest.starts_with("roadmap-") => "gdd".to_string(),
            rest => rest
                .split_once('-')
                .map(|(domain, _)| domain)
                .unwrap_or(rest)
                .to_string(),
        })
}

fn resolve_project_root(project_path: &Option<PathBuf>) -> Result<PathBuf> {
    match project_path {
        Some(path) => Ok(path.clone()),
        None => std::env::current_dir().context("failed to resolve current working directory"),
    }
}
