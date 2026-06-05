use std::{collections::HashMap, fs, process::Command};

use anyhow::{bail, Context, Result};

use crate::lux_roadmap::{self, RoadmapPhaseStatus};
pub use crate::lux_roadmap_issue_register_types::{
    ExistingIssue, IssueRegisterAction, IssueRegisterItem, IssueRegisterPlan, IssueRegisterRequest,
};

pub fn register_roadmap_issues(request: IssueRegisterRequest) -> Result<IssueRegisterPlan> {
    validate_repo(&request.repo)?;

    let roadmap = lux_roadmap::RoadmapReality::init_or_load(&request.project_root)?;
    let existing = load_existing_issues(&request)?;
    let existing_by_title = existing
        .iter()
        .cloned()
        .map(|issue| (normalize_title(&issue.title), issue))
        .collect::<HashMap<_, _>>();
    let existing_by_milestone = existing
        .into_iter()
        .filter_map(|issue| milestone_key(&issue.title).map(|key| (key, issue)))
        .collect::<HashMap<_, _>>();

    let mut items = Vec::new();
    for phase in roadmap.phases {
        if matches!(
            phase.status,
            RoadmapPhaseStatus::Complete | RoadmapPhaseStatus::Pushed
        ) {
            continue;
        }
        let title = format!("Roadmap: {}", phase.name);
        let existing_issue =
            find_existing_issue(&existing_by_title, &existing_by_milestone, &title);
        let created_url = create_if_missing(&request, &title, &existing_issue, None)?;
        let action = issue_action(
            request.dry_run,
            existing_issue.as_ref(),
            created_url.as_ref(),
        );
        items.push(IssueRegisterItem {
            body: format!(
                "## Source\n- .lux/roadmap.json phase `{}` currently has status `{:?}`.\n\n## Tracking rule\nGitHub Issues are the collaborator-visible tracking surface for Lux roadmap work. `.lux/roadmap.json` remains runtime status/feature-gate state, and local ledger files remain worktree decision receipts only.",
                phase.name, phase.status
            ),
            title,
            labels: default_labels(),
            action,
            existing_issue,
            created_url,
        });
    }

    for (title, body) in known_gap_candidates() {
        let existing_issue = find_existing_issue(&existing_by_title, &existing_by_milestone, title);
        let created_url = create_if_missing(&request, title, &existing_issue, Some(body))?;
        let action = issue_action(
            request.dry_run,
            existing_issue.as_ref(),
            created_url.as_ref(),
        );
        items.push(IssueRegisterItem {
            title: title.to_string(),
            body: body.to_string(),
            labels: default_labels(),
            action,
            existing_issue,
            created_url,
        });
    }

    let planned_count = items
        .iter()
        .filter(|item| item.action == IssueRegisterAction::WouldCreate)
        .count();
    let existing_count = items
        .iter()
        .filter(|item| item.action == IssueRegisterAction::Exists)
        .count();
    let created_count = items
        .iter()
        .filter(|item| item.action == IssueRegisterAction::Created)
        .count();

    Ok(IssueRegisterPlan {
        repo: request.repo,
        dry_run: request.dry_run,
        planned_count,
        existing_count,
        created_count,
        items,
    })
}

fn validate_repo(repo: &str) -> Result<()> {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        bail!("GitHub repo must be owner/name");
    }
    Ok(())
}

fn load_existing_issues(request: &IssueRegisterRequest) -> Result<Vec<ExistingIssue>> {
    if let Some(path) = request.existing_issues_json.as_deref() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        return serde_json::from_str(&content)
            .with_context(|| format!("failed to parse existing GitHub issues {}", path.display()));
    }
    if request.dry_run {
        return Ok(Vec::new());
    }

    let output = Command::new("gh")
        .args([
            "issue",
            "list",
            "--repo",
            &request.repo,
            "--state",
            "all",
            "--limit",
            "200",
            "--json",
            "number,title,state,url",
        ])
        .output()
        .context(
            "failed to run gh issue list; authenticate GitHub CLI before non-dry-run registration",
        )?;
    if !output.status.success() {
        bail!(
            "gh issue list failed for {}; stderr: {}",
            request.repo,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    serde_json::from_slice(&output.stdout)
        .context("failed to parse gh issue list JSON; no local ledger fallback was written")
}

fn normalize_title(title: &str) -> String {
    title
        .trim()
        .strip_prefix("Roadmap: ")
        .unwrap_or(title.trim())
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty() && *part != "and")
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn milestone_key(title: &str) -> Option<String> {
    let title = title
        .trim()
        .strip_prefix("Roadmap: ")
        .unwrap_or(title.trim());
    let (prefix, _) = title.split_once(':')?;
    let mut chars = prefix.chars();
    let marker = chars.next()?;
    let number = chars.as_str().parse::<u16>();
    match (marker, number) {
        ('M' | 'm', Ok(number)) => Some(format!("M{number}")),
        _ => None,
    }
}

fn roadmap_milestone_key(title: &str) -> Option<String> {
    let title = title.trim().strip_prefix("Roadmap: ")?;
    milestone_key(title)
}

fn find_existing_issue(
    existing_by_title: &HashMap<String, ExistingIssue>,
    existing_by_milestone: &HashMap<String, ExistingIssue>,
    title: &str,
) -> Option<ExistingIssue> {
    existing_by_title
        .get(&normalize_title(title))
        .or_else(|| {
            roadmap_milestone_key(title)
                .as_ref()
                .and_then(|key| existing_by_milestone.get(key))
        })
        .cloned()
}

fn default_labels() -> Vec<String> {
    ["roadmap", "enhancement", "unaddressed-feature"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn create_if_missing(
    request: &IssueRegisterRequest,
    title: &str,
    existing_issue: &Option<ExistingIssue>,
    body: Option<&str>,
) -> Result<Option<String>> {
    if request.dry_run || existing_issue.is_some() {
        return Ok(None);
    }
    let body = body.unwrap_or("Registered from Lux .lux/roadmap.json runtime status.");
    let mut args = vec![
        "issue".to_string(),
        "create".to_string(),
        "--repo".to_string(),
        request.repo.clone(),
        "--title".to_string(),
        title.to_string(),
        "--body".to_string(),
        body.to_string(),
    ];
    for label in default_labels() {
        args.push("--label".to_string());
        args.push(label);
    }
    let output = Command::new("gh")
        .args(&args)
        .output()
        .with_context(|| format!("failed to run gh issue create for {title}"))?;
    if !output.status.success() {
        bail!(
            "gh issue create failed for {title}; stderr: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

fn issue_action(
    dry_run: bool,
    existing_issue: Option<&ExistingIssue>,
    created_url: Option<&String>,
) -> IssueRegisterAction {
    if existing_issue.is_some() {
        IssueRegisterAction::Exists
    } else if dry_run {
        IssueRegisterAction::WouldCreate
    } else if created_url.is_some() {
        IssueRegisterAction::Created
    } else {
        IssueRegisterAction::WouldCreate
    }
}

fn known_gap_candidates() -> &'static [(&'static str, &'static str)] {
    &[
        ("Godot: finish evidence-backed runtime support beyond partial tier", "Godot is partial until runtime support produces supported evidence or explicit blockers."),
        ("M6: Autonomous spec-to-ticket-to-execution pipeline", "M6 remains planned until spec convergence, ticket generation, execution, and T3 Unity verification complete autonomously with evidence."),
        ("Three.js: build and verify runtime harness before promoting from planned", "Three.js remains planned until a supported runtime harness is verified."),
        ("Bundled workflow skills: add behavioral QA beyond schema validation", "Skill schema validation is not behavioral readiness."),
        ("Roadmap projection drift: keep .lux roadmap, docs, CLI, API, and MCP aligned", "Roadmap projections must not drift from runtime status or GitHub issue tracking."),
        ("Gateway-mediated SSoT: audit template and runtime state writes", "State-changing paths need classification instead of silent local bypasses."),
        ("Remote/WebRTC: keep hidden experimental and define evidence gate", "Remote/WebRTC must remain hidden experimental unless the feature flag and evidence gate allow it."),
    ]
}
