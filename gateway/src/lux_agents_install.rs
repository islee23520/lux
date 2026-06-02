use anyhow::{Context, Result};
use clap::Parser;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Parser)]
pub struct AgentsInstallArgs {
    /// Project root containing .lux/ directory
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Overwrite existing skills
    #[arg(long)]
    pub force: bool,
    /// List skills that would be installed without installing
    #[arg(long)]
    pub list_only: bool,
    /// Install specific skill(s) by name (default: all)
    #[arg(long = "skill", num_args = 1..)]
    pub skill_names: Option<Vec<String>>,
}

pub const LUX_WORKFLOW_SKILLS: &[(&str, &str)] = &[
    ("lux-init", LUX_INIT_SKILL),
    ("lux-spec", LUX_SPEC_SKILL),
    ("lux-run", LUX_RUN_SKILL),
    ("lux-build", LUX_BUILD_SKILL),
    ("lux-verify", LUX_VERIFY_SKILL),
    ("lux-triage", LUX_TRIAGE_SKILL),
    ("lux-kanban", LUX_KANBAN_SKILL),
    ("lux-doctor", LUX_DOCTOR_SKILL),
    ("lux-status", LUX_STATUS_SKILL),
    ("lux-godot", LUX_GODOT_SKILL),
];

pub fn run_agents_install_command(args: AgentsInstallArgs) -> Result<()> {
    let project_path = match &args.project_path {
        Some(path) => path.clone(),
        None => std::env::current_dir().context("failed to resolve current directory")?,
    };
    let target_dir = project_path.join(".agents").join("skills");
    let skills = selected_skills(args.skill_names.as_deref())?;

    if args.list_only {
        eprintln!("Bundled Lux workflow skills for {}:", target_dir.display());
        for (name, _) in skills {
            eprintln!("  {name}");
        }
        return Ok(());
    }

    let mut installed = 0usize;
    let mut skipped = 0usize;

    for (name, content) in skills {
        let skill_dir = target_dir.join(name);
        let skill_file = skill_dir.join("SKILL.md");

        if skill_file.exists() && !args.force {
            eprintln!("Skipping {name}: already exists (use --force)");
            skipped += 1;
            continue;
        }

        fs::create_dir_all(&skill_dir)
            .with_context(|| format!("failed to create {}", skill_dir.display()))?;
        write_atomic(&skill_file, content)
            .with_context(|| format!("failed to write {}", skill_file.display()))?;
        eprintln!("Installed {name} -> {}", skill_file.display());
        installed += 1;
    }

    eprintln!(
        "Installed {installed} skills to {} ({skipped} skipped)",
        target_dir.display()
    );
    Ok(())
}

pub fn list_bundled_skills() -> Vec<&'static str> {
    LUX_WORKFLOW_SKILLS.iter().map(|(name, _)| *name).collect()
}

fn selected_skills(requested: Option<&[String]>) -> Result<Vec<(&'static str, &'static str)>> {
    match requested {
        None => Ok(LUX_WORKFLOW_SKILLS.to_vec()),
        Some(names) => {
            let mut selected = Vec::new();
            for requested_name in names {
                let skill = LUX_WORKFLOW_SKILLS
                    .iter()
                    .find(|(name, _)| name == requested_name)
                    .copied()
                    .with_context(|| {
                        format!("unknown bundled Lux workflow skill: {requested_name}")
                    })?;
                selected.push(skill);
            }
            Ok(selected)
        }
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no valid UTF-8 file name", path.display()))?;
    let tmp_path = parent.join(format!(".{file_name}.tmp"));

    let mut tmp_file = File::create(&tmp_path)
        .with_context(|| format!("failed to create temporary file {}", tmp_path.display()))?;
    tmp_file
        .write_all(content.as_bytes())
        .with_context(|| format!("failed to write temporary file {}", tmp_path.display()))?;
    tmp_file
        .sync_all()
        .with_context(|| format!("failed to sync temporary file {}", tmp_path.display()))?;
    drop(tmp_file);

    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to rename temporary file {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

const LUX_INIT_SKILL: &str = r#"# lux-init — .lux Workspace Initialization

## Purpose
Initialize or repair the Lux workspace state for a Unity project.

## When to Use
- First time an AI agent starts work in a Unity project that should use Lux.
- `.lux/` is missing, incomplete, or suspected to be corrupted.
- A team profile must be applied before automated Lux workflows start.
- A controlled reinitialization is needed with `--force`.

## Commands
| Command | Use |
| --- | --- |
| `lux init` | Create `.lux/`, write `spec.json`, and prepare server/MCP state. |
| `lux init --force` | Reinitialize generated Lux state without deleting project work. |
| `lux init --team-profile <name>` | Initialize using a named AI team profile. |
| `lux doctor` | Confirm initialization health after setup. |

## Examples
```bash
lux init
```
Expected: `.lux/spec.json` exists and the project can use Lux CLI/API/MCP surfaces.

```bash
lux init --team-profile small-team
```
Expected: workspace is initialized with the selected team defaults.

```bash
lux init --force && lux doctor
```
Expected: regenerated Lux metadata and a clean diagnostic report.

## Gotchas
- `lux init` is project-scoped; run it from the Unity project root or pass the correct project path through the caller.
- `--force` repairs Lux state but must not be treated as a request to delete worktrees, tickets, or user assets.
- Post-init success means `.lux/spec.json` and server/MCP state are present; verify before running automation.
"#;

const LUX_SPEC_SKILL: &str = r#"# lux-spec — Spec Management

## Purpose
Manage the game design and delivery specification that drives Lux automation.

## When to Use
- A project needs its gameplay, art, audio, UI, or testing intent captured.
- Before `lux run`, to ensure `spec.json` and domain notes are valid.
- When an AI agent needs to record assumptions, questions, or decisions.
- After scope changes that affect architecture, packages, levels, or verification.
- In CI or review flows that need spec validation.

## Commands
| Command | Use |
| --- | --- |
| `lux spec status` | Show current spec completion and validation state. |
| `lux spec edit <domain>` | Open `$EDITOR` for a domain markdown file. |
| `lux spec validate` | Validate schema version, required domains, and structure. |
| `lux spec edit dialectic` | Capture questions, decisions, and assumptions. |

## Examples
```bash
lux spec status
```
Expected: domain coverage for design, architecture, art-style, audio, narrative, levels, ui-ux, packages, and testing.

```bash
lux spec edit ui-ux
```
Expected: `$EDITOR` opens the UI/UX domain notes for precise edits.

```bash
lux spec validate
```
Expected: success only when `schema_version` and required domains are present.

## Gotchas
- Do not bypass `$EDITOR` by writing contradictory state outside `.lux/`; `.lux/` is the source of truth.
- The nine domains are required context, not optional decoration.
- Record unresolved topics in the dialectic section instead of hiding uncertainty in implementation notes.
"#;

const LUX_RUN_SKILL: &str = r#"# lux-run — Spec-Driven Automated Dev Run

## Purpose
Execute a Lux development run from project spec to verified completion.

## When to Use
- The workspace is initialized and has a valid Lux spec.
- A feature or fix should be decomposed into tickets and executed by AI agents.
- Existing run state needs controlled recovery after interruption.
- You need Lux to plan, execute, verify, and close work in one lifecycle.
- Task count or complexity suggests adaptive team composition.

## Commands
| Command | Use |
| --- | --- |
| `lux run` | Plan, execute, verify, and complete the next automated run. |
| `lux run --recover <id>` | Resume an interrupted run by run id. |
| `lux spec validate` | Required preflight before a new run. |
| `lux status` | Inspect run and bridge state while automation proceeds. |

## Examples
```bash
lux spec validate && lux run
```
Expected: lifecycle advances through planning, ticket execution, verification, and completion.

```bash
lux run --recover run-2026-05-14-001
```
Expected: Lux reloads run state and continues from the last safe step.

```bash
lux status
```
Expected: JSON shows run state such as `Idle`, `Planning`, `ExecutingTicket`, or `Verifying`.

## Gotchas
- Do not start a run with an invalid or stale spec; fix spec validation first.
- TaskDAG order matters: blocked tickets must not be executed before prerequisites.
- Recovery should use the recorded run id, not a guessed ticket id.
"#;

const LUX_BUILD_SKILL: &str = r#"# lux-build — Build Pipeline

## Purpose
Trigger and monitor the Unity WebGL build pipeline through Lux.

## When to Use
- A verified feature needs a distributable WebGL build.
- CI or release workflow must confirm the project builds outside the editor.
- Verification requires build artifacts after compile and bridge checks pass.
- Build status must be tracked through the Lux API or MCP surface.
- You need a reproducible build command for scripts.

## Commands
| Command | Use |
| --- | --- |
| `lux build` | Start the configured WebGL build. |
| `lux status` | Monitor build state and project connection details. |
| `lux verify` | Run verification before or after build as appropriate. |
| `lux doctor` | Diagnose build environment issues. |

## Examples
```bash
lux verify && lux build
```
Expected: build starts only after verification succeeds.

```bash
lux build
```
Expected: Lux records build progress and final status through its API.

```bash
lux status
```
Expected: JSON includes current project, bridge, and build-related state.

## Gotchas
- Unity compile errors must be resolved before requesting a WebGL build.
- Build status is asynchronous; poll status instead of assuming immediate completion.
- Treat failed builds as verification failures and triage the underlying Unity output.
"#;

const LUX_VERIFY_SKILL: &str = r#"# lux-verify — Full Verification Suite

## Purpose
Run Lux verification tiers that prove the Unity project is ready for the next workflow step.

## When to Use
- Before `lux run` completion or before `lux build`.
- After code, scene, package, or bridge changes.
- When CI needs a single verification entry point.
- After recovering from a crashed or interrupted run.
- When Unity behavior must be checked beyond static file edits.

## Commands
| Command | Use |
| --- | --- |
| `lux verify` | Run the standalone verification suite. |
| `lux run` | Includes verification in its lifecycle. |
| `lux status` | Inspect current verification/run state. |
| `lux triage` | Classify verification failures into actionable tickets. |

## Examples
```bash
lux verify
```
Expected: T1 compile, T2 bridge, and T3 batchmode results are reported.

```bash
lux run
```
Expected: verification occurs after ticket execution and before completion.

```bash
lux verify || lux triage
```
Expected: failed signals become classified events and tickets.

## Gotchas
- T1 compile checks script compilation; do not ignore warnings that block Unity compilation.
- T2 bridge checks Lux-to-Unity connectivity, not gameplay correctness by itself.
- T3 batchmode catches editor/runtime issues that may not appear in a quick file check.
"#;

const LUX_TRIAGE_SKILL: &str = r#"# lux-triage — Event Triage Pipeline

## Purpose
Turn raw Unity, AI, and Lux events into deduplicated actionable tickets.

## When to Use
- Verification, build, or run output contains errors that need classification.
- Unity console logs or AI logs are noisy and repetitive.
- Tickets should be created automatically from events.
- A project has accumulated unresolved event streams in `.lux/`.
- You need to reduce duplicate failure reports before planning work.

## Commands
| Command | Use |
| --- | --- |
| `lux triage` | Ingest events, classify them, deduplicate, and create tickets. |
| `lux kanban` | Review tickets created by triage. |
| `lux verify` | Produce fresh verification events before triage. |
| `lux status` | Confirm project and run state before event processing. |

## Examples
```bash
lux verify || lux triage
```
Expected: compile, bridge, or batchmode failures become categorized tickets.

```bash
lux triage && lux kanban
```
Expected: board shows new or updated tickets without duplicate spam.

```bash
lux triage
```
Expected: events are classified as domains such as compile-error, ai-log, or unity-console.

## Gotchas
- Deduplication uses Jaccard plus Levenshtein similarity with a 0.75 threshold; similar logs may merge.
- Triage should classify and ticket events, not silently discard confusing output.
- Review generated tickets before running broad automation from them.
"#;

const LUX_KANBAN_SKILL: &str = r#"# lux-kanban — Ticket/Kanban Management

## Purpose
Inspect and manage Lux tickets that coordinate automated and human work.

## When to Use
- You need to see the current board before selecting work.
- Triage has created tickets from events.
- A run is blocked and dependencies need inspection.
- Priority or lifecycle state must be checked before automation continues.
- CI or reporting needs a concise board snapshot.

## Commands
| Command | Use |
| --- | --- |
| `lux kanban` | Show board status and ticket distribution. |
| `lux triage` | Create or update tickets from classified events. |
| `lux run` | Execute tickets according to TaskDAG ordering. |
| `lux status` | Check whether a run is already active. |

## Examples
```bash
lux kanban
```
Expected: tickets grouped by Open, InProgress, Done, and Closed.

```bash
lux triage && lux kanban
```
Expected: newly classified failures appear as prioritized tickets.

```bash
lux status && lux kanban
```
Expected: current run state and board state can be compared safely.

## Gotchas
- Ticket lifecycle is Open → InProgress → Done → Closed; avoid skipping states without evidence.
- Priorities are Critical, High, Medium, and Low; Critical blockers should be resolved first.
- Respect blocker relationships or the TaskDAG may execute work in an unsafe order.
"#;

const LUX_DOCTOR_SKILL: &str = r#"# lux-doctor — Self-Diagnosis & Repair

## Purpose
Diagnose and optionally repair Lux workspace, Unity, bridge, and agent integration issues.

## When to Use
- Before starting significant work in an unfamiliar project.
- After crashes, interrupted runs, missing plugins, or strange status output.
- As a CI/CD gate before automated Lux workflows.
- When `.agents/skills/` appears incomplete.
- Before using `--fix` to let Lux propose safe repairs.

## Commands
| Command | Use |
| --- | --- |
| `lux doctor` | Run diagnostic checks and report failures. |
| `lux doctor --fix` | Auto-fix supported issues through `opencode -p`. |
| `lux status` | Compare live system state before or after diagnostics. |
| `lux init --force` | Repair initialization issues when doctor recommends it. |

## Examples
```bash
lux doctor
```
Expected: checks cover workspace, spec, run-state, unity-project, bridge, plugin, agents-skills, lux-binary, and integrity.

```bash
lux doctor --fix
```
Expected: supported failures are repaired through an observable OpenCode prompt flow.

```bash
lux doctor && lux verify
```
Expected: environment health is confirmed before full verification.

## Gotchas
- `--fix` is for supported repairs; do not assume it can resolve gameplay or design ambiguity.
- Doctor failures are signals to investigate, not reasons to invent fallback state outside `.lux/`.
- Run doctor after crashes before restarting automation to avoid compounding partial state.
"#;

const LUX_STATUS_SKILL: &str = r#"# lux-status — System Status

## Purpose
Read Lux server, project, bridge, run, and build state in script-friendly JSON.

## When to Use
- Before starting automation to ensure the correct project is active.
- During `lux run`, `lux build`, or verification to monitor progress.
- In scripts or CI that need machine-readable status.
- When diagnosing bridge connectivity or server lifecycle issues.
- Before deciding whether recovery or doctor is needed.

## Commands
| Command | Use |
| --- | --- |
| `lux status` | Print current Lux status as JSON. |
| `lux doctor` | Diagnose problems discovered from status. |
| `lux run --recover <id>` | Recover a run identified from status output. |
| `lux verify` | Validate the project after status looks healthy. |

## Examples
```bash
lux status
```
Expected: JSON describes server, project, bridge, run, and build state.

```bash
lux status | jq '.bridge.connected'
```
Expected: `true` when the Unity bridge is connected.

```bash
lux status | jq '.run.state'
```
Expected: state such as `Idle`, `Planning`, `ExecutingTicket`, or `Verifying`.

## Gotchas
- Status is observational; it should not mutate `.lux/` or repair state by itself.
- Always confirm the project path in JSON before acting on tickets or builds.
- In CI, parse explicit fields instead of scraping human text.
"#;

const LUX_GODOT_SKILL: &str = r#"---
name: lux-godot
description: Drive Godot projects through the Lux local harness with explicit capability checks.
---

# lux-godot — Godot Harness Workflow

## Purpose
Use Lux as a local-first AI harness for Godot projects without overclaiming unsupported build, run, test, scene, or capture behavior.

## When to Use
- A project contains `project.godot` and should be checked through Lux.
- An agent needs to install or verify the Godot bridge under `addons/lux_bridge/`.
- A Codex, Claude, OpenCode, or other `.agents`-aware client needs Godot-specific safety guidance.

## Workflow
1. Run `lux godot status --project-path <project>` and inspect both `gopeak.*` and `lux.*` fields.
2. If bridge files are missing, run `lux bridge install --project-path <project> --type godot`.
3. Treat `gopeak.available_commands` as external GoPeak visibility only.
4. Treat `lux.supported_commands` and `lux.unsupported_commands` as the Lux execution contract.
5. If a requested action is unsupported, report the explicit blocker and record evidence under `.lux/` or the active task artifact.

## Commands
| Command | Use |
| --- | --- |
| `lux godot status --project-path <project>` | Detect Godot 4, GoPeak visibility, and Lux-supported commands. |
| `lux bridge install --project-path <project> --type godot` | Install the managed Godot bridge files. |
| `lux godot build --project-path <project>` | Currently exits non-zero until GoPeak-backed build verification exists. |

## Gotchas
- Do not use `--engine godot` for bridge install; this plan uses `--type godot`.
- Do not infer Lux support from GoPeak manifest entries such as `project/build`.
- Do not write state outside the project `.lux/` evidence/spec/ticket paths.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_skills_include_lux_godot() {
        assert!(list_bundled_skills().contains(&"lux-godot"));
        let (_, content) = LUX_WORKFLOW_SKILLS
            .iter()
            .find(|(name, _)| *name == "lux-godot")
            .expect("lux-godot skill is bundled");
        assert!(content.contains("lux godot status"));
        assert!(content.contains("--type godot"));
        assert!(content.contains("lux.unsupported_commands"));
    }
}
