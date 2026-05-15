use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    lux_ambiguity::{self, AmbiguityReport, TargetedQuestion},
    lux_spec,
    lux_ticket::{FileTicketStore, Ticket, TicketPriority, TicketStatus, TicketStore},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecLoopState {
    Analyzing,
    AskingQuestions,
    DraftingProposal,
    AwaitingApproval,
    ApplyingApprovedChanges,
    DerivingRoadmap,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecProposalKind {
    DomainDocPatch,
    TicketDraft,
    RoadmapPatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecProposalStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecLoopQuestion {
    pub id: String,
    pub domain: String,
    pub phase: String,
    pub question: String,
    pub priority: f64,
    pub answer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecProposal {
    pub id: String,
    pub run_id: String,
    pub kind: SpecProposalKind,
    pub status: SpecProposalStatus,
    pub summary: String,
    pub rationale: String,
    pub domain: Option<String>,
    pub spec_refs: Vec<String>,
    pub changes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecLoopRun {
    pub id: String,
    pub project_path: PathBuf,
    pub state: SpecLoopState,
    pub iteration: u32,
    pub max_iterations: u32,
    pub ambiguity: AmbiguityReport,
    pub questions: Vec<SpecLoopQuestion>,
    pub proposals: Vec<SpecProposal>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn start(project_path: &Path, max_iterations: Option<u32>) -> Result<SpecLoopRun> {
    let spec = lux_spec::lux_load_or_init(project_path)?;
    let ambiguity = lux_ambiguity::calculate_ambiguity(&spec);
    let questions = build_questions(&ambiguity);
    let now = Utc::now().to_rfc3339();
    let mut run = SpecLoopRun {
        id: Uuid::new_v4().to_string(),
        project_path: project_path.to_path_buf(),
        state: if questions.is_empty() {
            SpecLoopState::AwaitingApproval
        } else {
            SpecLoopState::AskingQuestions
        },
        iteration: 0,
        max_iterations: max_iterations.unwrap_or(10).max(1),
        ambiguity,
        questions,
        proposals: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };
    if run.questions.is_empty() {
        run.proposals
            .push(roadmap_proposal(&run.id, &run.ambiguity));
    }
    save_run(project_path, &run)?;
    save_proposals(project_path, &run.proposals)?;
    Ok(run)
}

pub fn latest(project_path: &Path) -> Result<Option<SpecLoopRun>> {
    let dir = runs_dir(project_path);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        runs.push(load_run_path(&entry.path())?);
    }
    runs.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(runs.into_iter().next())
}

pub fn load(project_path: &Path, run_id: &str) -> Result<SpecLoopRun> {
    load_run_path(&run_path(project_path, run_id))
}

pub fn answer(
    project_path: &Path,
    run_id: &str,
    question_id: &str,
    answer: &str,
) -> Result<SpecLoopRun> {
    let mut run = load(project_path, run_id)?;
    let Some(question) = run
        .questions
        .iter_mut()
        .find(|question| question.id == question_id)
    else {
        bail!("spec-loop question not found: {question_id}");
    };
    let answer = answer.trim();
    if answer.is_empty() {
        bail!("answer cannot be empty");
    }
    question.answer = Some(answer.to_string());
    run.state = SpecLoopState::DraftingProposal;
    run.proposals
        .extend(proposals_for_answer(&run.id, question));
    let answered_question = question.clone();
    run.proposals
        .push(roadmap_proposal(&run.id, &run.ambiguity));
    dedupe_proposals(&mut run.proposals);
    run.state = SpecLoopState::AwaitingApproval;
    touch_run(project_path, &mut run)?;
    save_proposals(project_path, &run.proposals)?;
    if let Ok(mut spec) = lux_spec::lux_load(project_path) {
        spec.dialectic.questions.push(lux_spec::SpecQuestion {
            id: answered_question.id,
            domain: Some(answered_question.domain),
            text: answered_question.question,
            answer: answered_question.answer,
            status: Some("Answered".to_string()),
            created_at: None,
            answered_at: Some(Utc::now().to_rfc3339()),
        });
        let _ = lux_spec::lux_save(project_path, &spec);
    }
    Ok(run)
}

pub fn approve(project_path: &Path, run_id: &str, proposal_id: &str) -> Result<SpecLoopRun> {
    update_proposal_status(
        project_path,
        run_id,
        proposal_id,
        SpecProposalStatus::Approved,
    )
}

pub fn reject(project_path: &Path, run_id: &str, proposal_id: &str) -> Result<SpecLoopRun> {
    update_proposal_status(
        project_path,
        run_id,
        proposal_id,
        SpecProposalStatus::Rejected,
    )
}

pub fn apply_approved(project_path: &Path, run_id: &str) -> Result<SpecLoopRun> {
    let mut run = load(project_path, run_id)?;
    run.state = SpecLoopState::ApplyingApprovedChanges;
    for index in 0..run.proposals.len() {
        if run.proposals[index].status != SpecProposalStatus::Approved {
            continue;
        }
        apply_proposal(project_path, &run.proposals[index])?;
        run.proposals[index].status = SpecProposalStatus::Applied;
        run.proposals[index].updated_at = Utc::now().to_rfc3339();
    }
    run.state = SpecLoopState::DerivingRoadmap;
    run.iteration = run.iteration.saturating_add(1);
    run.state = if run.iteration >= run.max_iterations {
        SpecLoopState::Complete
    } else {
        SpecLoopState::Analyzing
    };
    touch_run(project_path, &mut run)?;
    save_proposals(project_path, &run.proposals)?;
    if let Ok(mut spec) = lux_spec::lux_load(project_path) {
        for proposal in &run.proposals {
            if proposal.status == SpecProposalStatus::Applied {
                spec.dialectic.decisions.push(lux_spec::SpecDecision {
                    id: proposal.id.clone(),
                    domain: proposal.domain.clone(),
                    text: proposal.summary.clone(),
                    rationale: Some(proposal.rationale.clone()),
                    source_question: None,
                    created_at: Some(Utc::now().to_rfc3339()),
                });
            }
        }
        let _ = lux_spec::lux_save(project_path, &spec);
    }
    Ok(run)
}

fn build_questions(ambiguity: &AmbiguityReport) -> Vec<SpecLoopQuestion> {
    let mut questions: Vec<_> = ambiguity
        .targeted_questions
        .iter()
        .take(8)
        .map(question_from_target)
        .collect();
    if questions.is_empty() {
        questions.push(SpecLoopQuestion {
            id: Uuid::new_v4().to_string(),
            domain: "design".to_string(),
            phase: "Roadmap".to_string(),
            question: "Which player-facing uncertainty should the next iteration resolve first?"
                .to_string(),
            priority: 0.5,
            answer: None,
        });
    }
    questions
}

fn question_from_target(target: &TargetedQuestion) -> SpecLoopQuestion {
    SpecLoopQuestion {
        id: Uuid::new_v4().to_string(),
        domain: target.domain.clone(),
        phase: target.phase.clone(),
        question: target.question.clone(),
        priority: target.priority,
        answer: None,
    }
}

fn proposals_for_answer(run_id: &str, question: &SpecLoopQuestion) -> Vec<SpecProposal> {
    let now = Utc::now().to_rfc3339();
    let answer = question.answer.clone().unwrap_or_default();
    vec![
        SpecProposal {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.to_string(),
            kind: SpecProposalKind::DomainDocPatch,
            status: SpecProposalStatus::Pending,
            summary: format!("Add clarification to {} spec", question.domain),
            rationale: "User answer reduces ambiguity and should be preserved in the domain markdown before the next roadmap derivation.".to_string(),
            domain: Some(question.domain.clone()),
            spec_refs: vec![format!(".lux/domains/{}.md", question.domain)],
            changes: vec![format!(
                "## Recursive Clarifications\n\n### {}\nQuestion: {}\nAnswer: {}\n",
                question.phase, question.question, answer
            )],
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        SpecProposal {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.to_string(),
            kind: SpecProposalKind::TicketDraft,
            status: SpecProposalStatus::Pending,
            summary: format!("Organize {} implementation references", question.domain),
            rationale: "The recursive loop needs explicit tools, assets, and references before it can safely turn the clarified spec into work.".to_string(),
            domain: Some(question.domain.clone()),
            spec_refs: vec![format!(".lux/domains/{}.md", question.domain)],
            changes: vec![format!(
                "Create roadmap ticket for {}: collect required tools, Unity assets, reference docs, and constraints related to '{}'.",
                question.domain, question.question
            )],
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

fn roadmap_proposal(run_id: &str, ambiguity: &AmbiguityReport) -> SpecProposal {
    let now = Utc::now().to_rfc3339();
    let priority = ambiguity
        .targeted_questions
        .first()
        .map(|question| question.domain.clone())
        .unwrap_or_else(|| "design".to_string());
    SpecProposal {
        id: Uuid::new_v4().to_string(),
        run_id: run_id.to_string(),
        kind: SpecProposalKind::RoadmapPatch,
        status: SpecProposalStatus::Pending,
        summary: "Derive next roadmap slice from current ambiguity".to_string(),
        rationale: "Roadmap order should follow the highest remaining spec ambiguity rather than unrelated logs or tooling tasks.".to_string(),
        domain: Some(priority.clone()),
        spec_refs: vec![".lux/spec.json".to_string(), format!(".lux/domains/{priority}.md")],
        changes: vec![format!(
            "Prioritize {priority} until ambiguity drops below {:.0}%.",
            ambiguity.overall_score * 100.0
        )],
        created_at: now.clone(),
        updated_at: now,
    }
}

fn update_proposal_status(
    project_path: &Path,
    run_id: &str,
    proposal_id: &str,
    status: SpecProposalStatus,
) -> Result<SpecLoopRun> {
    let mut run = load(project_path, run_id)?;
    let Some(proposal) = run
        .proposals
        .iter_mut()
        .find(|proposal| proposal.id == proposal_id)
    else {
        bail!("spec-loop proposal not found: {proposal_id}");
    };
    proposal.status = status;
    proposal.updated_at = Utc::now().to_rfc3339();
    touch_run(project_path, &mut run)?;
    save_proposals(project_path, &run.proposals)?;
    Ok(run)
}

fn apply_proposal(project_path: &Path, proposal: &SpecProposal) -> Result<()> {
    match proposal.kind {
        SpecProposalKind::DomainDocPatch => {
            let Some(domain) = proposal.domain.as_ref() else {
                bail!("domain doc patch proposal missing domain");
            };
            let existing = lux_spec::lux_load_domain(project_path, domain).unwrap_or_default();
            let mut next = existing.trim_end().to_string();
            next.push_str("\n\n");
            next.push_str(&proposal.changes.join("\n"));
            next.push('\n');
            lux_spec::lux_save_domain(project_path, domain, &next)?;
        }
        SpecProposalKind::TicketDraft | SpecProposalKind::RoadmapPatch => {
            let store = FileTicketStore::new(project_path);
            let now = Utc::now().to_rfc3339();
            store.create(Ticket {
                id: Uuid::new_v4().to_string(),
                title: proposal.summary.clone(),
                description: format!("{}\n\n{}", proposal.rationale, proposal.changes.join("\n")),
                status: TicketStatus::ToDo,
                priority: TicketPriority::High,
                assignee: None,
                blockers: Vec::new(),
                tags: vec!["spec-loop".to_string(), "roadmap".to_string()],
                spec_ref: proposal.spec_refs.first().cloned(),
                created_at: now.clone(),
                updated_at: now,
            })?;
        }
    }
    Ok(())
}

fn dedupe_proposals(proposals: &mut Vec<SpecProposal>) {
    let mut seen = Vec::<(SpecProposalKind, String)>::new();
    proposals.retain(|proposal| {
        let key = (proposal.kind.clone(), proposal.summary.clone());
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

fn runs_dir(project_path: &Path) -> PathBuf {
    project_path.join(".lux/sessions/spec-loop")
}

fn proposals_dir(project_path: &Path) -> PathBuf {
    project_path.join(".lux/proposals")
}

fn run_path(project_path: &Path, run_id: &str) -> PathBuf {
    runs_dir(project_path).join(format!("{run_id}.json"))
}

fn load_run_path(path: &Path) -> Result<SpecLoopRun> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn touch_run(project_path: &Path, run: &mut SpecLoopRun) -> Result<()> {
    run.updated_at = Utc::now().to_rfc3339();
    save_run(project_path, run)
}

fn save_run(project_path: &Path, run: &SpecLoopRun) -> Result<()> {
    let dir = runs_dir(project_path);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = run_path(project_path, &run.id);
    let json = serde_json::to_string_pretty(run).context("failed to serialize spec-loop run")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))
}

fn save_proposals(project_path: &Path, proposals: &[SpecProposal]) -> Result<()> {
    let dir = proposals_dir(project_path);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    for proposal in proposals {
        let path = dir.join(format!("{}.json", proposal.id));
        let json =
            serde_json::to_string_pretty(proposal).context("failed to serialize spec proposal")?;
        fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn answers_create_pending_proposals_without_mutating_domain_doc() {
        let dir = tempdir().unwrap();
        lux_spec::lux_init(dir.path()).unwrap();
        let before = lux_spec::lux_load_domain(dir.path(), "design").unwrap();
        let run = start(dir.path(), Some(3)).unwrap();
        let question_id = run.questions[0].id.clone();

        let answered = answer(
            dir.path(),
            &run.id,
            &question_id,
            "The loop is scout, dash, collect, upgrade.",
        )
        .unwrap();

        assert_eq!(
            lux_spec::lux_load_domain(dir.path(), "design").unwrap(),
            before
        );
        assert!(answered
            .proposals
            .iter()
            .any(|proposal| proposal.status == SpecProposalStatus::Pending));
    }

    #[test]
    fn approved_domain_doc_patch_applies_only_after_apply() {
        let dir = tempdir().unwrap();
        lux_spec::lux_init(dir.path()).unwrap();
        let run = start(dir.path(), Some(3)).unwrap();
        let question_id = run.questions[0].id.clone();
        let answered = answer(
            dir.path(),
            &run.id,
            &question_id,
            "The first decision is route choice.",
        )
        .unwrap();
        let proposal_id = answered
            .proposals
            .iter()
            .find(|proposal| proposal.kind == SpecProposalKind::DomainDocPatch)
            .unwrap()
            .id
            .clone();

        approve(dir.path(), &run.id, &proposal_id).unwrap();
        apply_approved(dir.path(), &run.id).unwrap();

        let design = lux_spec::lux_load_domain(dir.path(), "design").unwrap();
        assert!(design.contains("The first decision is route choice."));
    }
}
