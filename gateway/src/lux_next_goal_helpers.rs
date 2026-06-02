use std::path::Path;

use anyhow::Result;

use crate::{
    lux_spec::SpecProject,
    lux_ticket::{FileTicketStore, Ticket, TicketStatus, TicketStore},
};

pub(crate) fn ticket_goal(
    prefix: &str,
    rationale: &str,
    ticket: Ticket,
    fallback_engine: &str,
    requested_goal: Option<String>,
) -> crate::lux_next_goal_types::NextGoal {
    let selected_engine = ticket
        .verification_policy
        .as_deref()
        .and_then(engine_from_policy)
        .unwrap_or(fallback_engine)
        .to_string();
    crate::lux_next_goal_types::NextGoal {
        goal_id: format!("{prefix}:{}", ticket.id),
        title: ticket.title,
        rationale: rationale.to_string(),
        source_spec_refs: vec![ticket
            .spec_ref
            .unwrap_or_else(|| format!(".lux/tickets/{}.json", ticket.id))],
        selected_engine,
        requested_goal,
    }
}

pub(crate) fn blockers_resolved(store: &FileTicketStore, ticket: &Ticket) -> Result<bool> {
    for blocker_id in &ticket.blockers {
        match store.get(blocker_id)? {
            Some(blocker) if blocker.status == TicketStatus::Done => {}
            Some(_) | None => return Ok(false),
        }
    }
    Ok(true)
}

pub(crate) fn select_engine(project_path: &Path, spec: &SpecProject) -> Result<String> {
    let inventory = crate::project::detect_engine_capabilities(project_path)?;
    if let Some(engine) = inventory.engines.iter().find(|engine| engine.detected) {
        return Ok(engine_name(engine.engine).to_string());
    }
    let _ = spec;
    Ok("unity".to_string())
}

pub(crate) fn highest_ambiguous_domain(spec: &SpecProject) -> Option<(String, f64)> {
    spec.domains
        .built_in_domains()
        .into_iter()
        .flatten()
        .chain(spec.domains.custom.values())
        .filter(|domain| domain.ambiguity_score > 0.0)
        .max_by(|left, right| left.ambiguity_score.total_cmp(&right.ambiguity_score))
        .map(|domain| (domain.name.clone(), domain.ambiguity_score))
}

pub(crate) fn domain_ref(domain: &str) -> String {
    format!(".lux/specs/domains/{}.md", domain.replace('_', "-"))
}

pub(crate) fn goal_slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn engine_name(engine: lux_project::EngineKind) -> &'static str {
    match engine {
        lux_project::EngineKind::Unity => "unity",
        lux_project::EngineKind::Godot => "godot",
        lux_project::EngineKind::ThreeJs => "three_js",
    }
}

fn engine_from_policy(policy: &str) -> Option<&'static str> {
    if policy.starts_with("unity") {
        Some("unity")
    } else if policy.starts_with("godot") {
        Some("godot")
    } else if policy.starts_with("three") {
        Some("three_js")
    } else {
        None
    }
}
