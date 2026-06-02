use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    lux_io::{append_jsonl, atomic_write_json},
    lux_next_goal_types::{CurrentGoal, NextGoal},
    lux_task_dag::{TaskDAG, TaskStatus},
};

pub fn persist_current_goal(
    project_path: &Path,
    run_id: &str,
    requested_goal: Option<String>,
    next_goal: &NextGoal,
) -> Result<()> {
    let selected_at = Utc::now().to_rfc3339();
    let current_goal = CurrentGoal {
        run_id: run_id.to_string(),
        goal_id: next_goal.goal_id.clone(),
        title: next_goal.title.clone(),
        rationale: next_goal.rationale.clone(),
        source_spec_refs: next_goal.source_spec_refs.clone(),
        selected_engine: next_goal.selected_engine.clone(),
        requested_goal,
        selected_at: selected_at.clone(),
    };
    atomic_write_json(&project_path.join(".lux/goals/current.json"), &current_goal)?;
    append_jsonl(
        &project_path.join(".lux/specs/decisions.jsonl"),
        &json!({
            "event": "goal_selected",
            "runId": current_goal.run_id,
            "goalId": current_goal.goal_id,
            "title": current_goal.title,
            "selectedEngine": current_goal.selected_engine,
            "requestedGoal": current_goal.requested_goal,
            "sourceSpecRefs": current_goal.source_spec_refs,
            "rationale": current_goal.rationale,
            "timestampUtc": selected_at,
        }),
    )
}

pub fn write_awaiting_evidence_blocker(
    project_path: &Path,
    run_id: &str,
    dag: &TaskDAG,
) -> Result<()> {
    let current_goal = read_current_goal(project_path)?;
    let awaiting_tasks = dag
        .nodes
        .values()
        .filter(|node| node.status == TaskStatus::AwaitingEvidence)
        .map(|node| {
            json!({
                "taskId": node.id,
                "title": node.title,
                "evidencePath": node.evidence_path,
            })
        })
        .collect::<Vec<_>>();
    atomic_write_json(
        &project_path
            .join(".lux/evidence/autonomous")
            .join(run_id)
            .join("awaiting-evidence-blocker.json"),
        &json!({
            "status": "awaiting_evidence",
            "runId": run_id,
            "currentGoal": current_goal,
            "awaitingTasks": awaiting_tasks,
            "reason": "run dispatched work and is awaiting execution plus verification evidence",
            "writtenAt": Utc::now().to_rfc3339(),
        }),
    )
}

fn read_current_goal(project_path: &Path) -> Result<Value> {
    let path = project_path.join(".lux/goals/current.json");
    if !path.is_file() {
        return Ok(json!(null));
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}
