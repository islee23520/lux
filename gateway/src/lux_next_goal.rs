use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    lux_ambiguity::AmbiguityReport,
    lux_next_goal_helpers::{
        blockers_resolved, domain_ref, goal_slug, highest_ambiguous_domain, select_engine,
        ticket_goal,
    },
    lux_spec::SpecProject,
    lux_task_dag::TaskDAG,
    lux_ticket::{is_execution_grade, FileTicketStore, TicketStatus, TicketStore},
};

const TARGET_AMBIGUITY: f64 = 0.02;

pub use crate::lux_next_goal_evidence::{persist_current_goal, write_awaiting_evidence_blocker};
pub use crate::lux_next_goal_types::NextGoal;

pub fn select_next_goal(
    project_path: &Path,
    spec: &SpecProject,
    ambiguity: &AmbiguityReport,
    dag: &TaskDAG,
    requested_goal: Option<String>,
) -> Result<NextGoal> {
    let selected_engine = select_engine(project_path, spec)?;
    if let Some(goal) = contradiction_goal(project_path, &selected_engine, requested_goal.clone())?
    {
        return Ok(goal);
    }
    if let Some(goal) = ambiguity_goal(spec, ambiguity, &selected_engine, requested_goal.clone()) {
        return Ok(goal);
    }
    if let Some(goal) =
        execution_grade_ticket_goal(project_path, &selected_engine, requested_goal.clone())?
    {
        return Ok(goal);
    }
    if let Some(goal) = engine_blocker_goal(project_path, &selected_engine, requested_goal.clone())?
    {
        return Ok(goal);
    }
    Ok(milestone_goal(dag, &selected_engine, requested_goal))
}

fn contradiction_goal(
    project_path: &Path,
    selected_engine: &str,
    requested_goal: Option<String>,
) -> Result<Option<NextGoal>> {
    let preferences_path = project_path.join(".lux/specs/preferences.json");
    if !preferences_path.is_file() {
        return Ok(None);
    }
    let preferences: Value = serde_json::from_str(&fs::read_to_string(&preferences_path)?)
        .with_context(|| format!("failed to parse {}", preferences_path.display()))?;
    let Some(conflict) = preferences
        .get("conflicts")
        .and_then(Value::as_array)
        .and_then(|conflicts| conflicts.first())
    else {
        return Ok(None);
    };
    let domain = conflict
        .get("domain")
        .and_then(Value::as_str)
        .unwrap_or("spec");
    Ok(Some(NextGoal {
        goal_id: format!("contradiction:{}", goal_slug(domain)),
        title: format!("Resolve blocking {domain} contradiction"),
        rationale: "Selected first because .lux/specs/preferences.json records an unresolved blocking contradiction.".to_string(),
        source_spec_refs: vec![
            ".lux/specs/preferences.json".to_string(),
            ".lux/specs/decisions.jsonl".to_string(),
        ],
        selected_engine: selected_engine.to_string(),
        requested_goal,
    }))
}

fn ambiguity_goal(
    spec: &SpecProject,
    ambiguity: &AmbiguityReport,
    selected_engine: &str,
    requested_goal: Option<String>,
) -> Option<NextGoal> {
    if ambiguity.overall_score <= TARGET_AMBIGUITY {
        return None;
    }
    let question = ambiguity
        .targeted_questions
        .iter()
        .max_by(|left, right| left.priority.total_cmp(&right.priority));
    let (domain, reason) = match question {
        Some(question) => (
            question.domain.clone(),
            format!(
                "ambiguity remains in {}: {}",
                question.domain, question.question
            ),
        ),
        None => highest_ambiguous_domain(spec).map(|(domain, score)| {
            (
                domain.clone(),
                format!("{domain} has ambiguity score {score:.2}"),
            )
        })?,
    };
    Some(NextGoal {
        goal_id: format!("ambiguity:{}", goal_slug(&domain)),
        title: format!("Resolve {domain} spec ambiguity"),
        rationale: format!("Selected from .lux/specs because {reason}"),
        source_spec_refs: vec![".lux/specs/spec.json".to_string(), domain_ref(&domain)],
        selected_engine: selected_engine.to_string(),
        requested_goal,
    })
}

fn execution_grade_ticket_goal(
    project_path: &Path,
    selected_engine: &str,
    requested_goal: Option<String>,
) -> Result<Option<NextGoal>> {
    let store = FileTicketStore::new(project_path);
    let mut tickets = store.list(Default::default())?;
    tickets.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then(left.id.cmp(&right.id))
    });
    for ticket in tickets {
        if ticket.status == TicketStatus::Done || !is_execution_grade(&ticket) {
            continue;
        }
        if blockers_resolved(&store, &ticket)? {
            return Ok(Some(ticket_goal(
                "execution-ticket",
                "Selected execution-grade ticket with satisfied dependencies from .lux/tickets.",
                ticket,
                selected_engine,
                requested_goal,
            )));
        }
    }
    Ok(None)
}

fn engine_blocker_goal(
    project_path: &Path,
    selected_engine: &str,
    requested_goal: Option<String>,
) -> Result<Option<NextGoal>> {
    let store = FileTicketStore::new(project_path);
    let mut blockers = store
        .list(Default::default())?
        .into_iter()
        .filter(|ticket| ticket.status != TicketStatus::Done)
        .filter(|ticket| ticket.tags.iter().any(|tag| tag == "engine-blocker"))
        .collect::<Vec<_>>();
    blockers.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then(left.id.cmp(&right.id))
    });
    Ok(blockers.into_iter().next().map(|ticket| {
        ticket_goal(
            "engine-blocker",
            "Selected engine blocker resolution before milestone work.",
            ticket,
            selected_engine,
            requested_goal,
        )
    }))
}

fn milestone_goal(
    dag: &TaskDAG,
    selected_engine: &str,
    requested_goal: Option<String>,
) -> NextGoal {
    let node = dag.ready_nodes().into_iter().next();
    let (goal_id, title, spec_ref) = node
        .map(|node| (node.id, node.title, node.spec_clause_id))
        .unwrap_or_else(|| {
            (
                "task-spec-review".to_string(),
                "Review spec for next executable goal".to_string(),
                "spec-review".to_string(),
            )
        });
    NextGoal {
        goal_id,
        title,
        rationale: "Selected milestone task after contradictions, ambiguity, executable tickets, and engine blockers were clear.".to_string(),
        source_spec_refs: vec![".lux/specs/spec.json".to_string(), spec_ref],
        selected_engine: selected_engine.to_string(),
        requested_goal,
    }
}
