use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{lux_io::atomic_write_json, lux_run::RunLifecycle, lux_task_dag::TaskStatus};

/// Metrics snapshot for an active or completed run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMetrics {
    pub run_id: String,
    pub recorded_at: String,

    pub started_at: Option<String>,
    pub planning_duration_secs: Option<u64>,
    pub execution_duration_secs: Option<u64>,
    pub total_wall_time_secs: u64,

    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub blocked_tasks: usize,
    pub pending_tasks: usize,

    pub agents_deployed: u32,
    pub agent_seconds: u64,
    pub max_parallel_agents: u32,

    pub verifications_passed: usize,
    pub verifications_failed: usize,
    pub merge_requests_created: usize,
    pub merge_requests_approved: usize,
    pub merge_requests_rejected: usize,

    pub recovery_count: usize,
    pub quarantine_events: usize,
}

impl RunMetrics {
    pub fn snapshot(lifecycle: &RunLifecycle) -> RunMetrics {
        let recorded_at = Utc::now();
        let started_at = lifecycle
            .active_agents
            .values()
            .filter_map(|agent| parse_rfc3339(&agent.started_at))
            .min()
            .map(|time| time.to_rfc3339());
        let total_wall_time_secs = started_at
            .as_deref()
            .and_then(parse_rfc3339)
            .map(|started| {
                recorded_at
                    .signed_duration_since(started)
                    .num_seconds()
                    .max(0) as u64
            })
            .unwrap_or(0);

        let completed_tasks = lifecycle
            .dag
            .nodes
            .values()
            .filter(|node| node.status == TaskStatus::Done)
            .count();
        let blocked_tasks = lifecycle
            .dag
            .nodes
            .values()
            .filter(|node| node.status == TaskStatus::Blocked)
            .count();
        let failed_tasks = blocked_tasks;
        let pending_tasks = lifecycle
            .dag
            .nodes
            .values()
            .filter(|node| node.status == TaskStatus::Pending)
            .count();

        let agent_seconds = lifecycle
            .active_agents
            .values()
            .filter_map(|agent| parse_rfc3339(&agent.started_at))
            .map(|started| {
                recorded_at
                    .signed_duration_since(started)
                    .num_seconds()
                    .max(0) as u64
            })
            .sum();

        RunMetrics {
            run_id: lifecycle.state.run_id.clone(),
            recorded_at: recorded_at.to_rfc3339(),
            started_at,
            planning_duration_secs: None,
            execution_duration_secs: Some(total_wall_time_secs),
            total_wall_time_secs,
            total_tasks: lifecycle.dag.nodes.len(),
            completed_tasks,
            failed_tasks,
            blocked_tasks,
            pending_tasks,
            agents_deployed: lifecycle.active_agents.len() as u32,
            agent_seconds,
            max_parallel_agents: lifecycle.active_agents.len() as u32,
            verifications_passed: 0,
            verifications_failed: 0,
            merge_requests_created: 0,
            merge_requests_approved: 0,
            merge_requests_rejected: 0,
            recovery_count: 0,
            quarantine_events: 0,
        }
    }

    pub fn save(&self, lux_dir: &Path) -> Result<()> {
        let path = lux_dir.join("runs").join(&self.run_id).join("metrics.json");
        atomic_write_json(&path, self)
    }

    pub fn load(lux_dir: &Path, run_id: &str) -> Result<Self> {
        let path = lux_dir.join("runs").join(run_id).join("metrics.json");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read metrics {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse metrics {}", path.display()))
    }
}

pub fn format_metrics_report(metrics: &RunMetrics) -> String {
    format!(
        "Run Metrics: {}\n\
         Recorded: {}\n\
         Tasks: {}/{} completed, {} failed, {} blocked, {} pending\n\
         Agents: {} deployed, {}s cumulative, {} max parallel\n\
         Verification: {} passed, {} failed\n\
         Merge Requests: {} created, {} approved, {} rejected\n\
         Recovery: {} recoveries, {} quarantines\n\
         Wall Time: {}s",
        metrics.run_id,
        metrics.recorded_at,
        metrics.completed_tasks,
        metrics.total_tasks,
        metrics.failed_tasks,
        metrics.blocked_tasks,
        metrics.pending_tasks,
        metrics.agents_deployed,
        metrics.agent_seconds,
        metrics.max_parallel_agents,
        metrics.verifications_passed,
        metrics.verifications_failed,
        metrics.merge_requests_created,
        metrics.merge_requests_approved,
        metrics.merge_requests_rejected,
        metrics.recovery_count,
        metrics.quarantine_events,
        metrics.total_wall_time_secs
    )
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}
