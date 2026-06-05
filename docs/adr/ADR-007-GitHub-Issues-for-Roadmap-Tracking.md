# ADR-007: GitHub Issues for Roadmap and Feature Tracking

## Status
Accepted (Phase 1 follow-up, 2026-06-05)

## Context
Lux has three different kinds of durable records that must not collapse into one
local ledger:

- Local agent/worktree decision records, such as `.omo/*/ledger.jsonl`.
- Target-project runtime state under `.lux/`, including specs, execution
  tickets, run state, evidence, and the minimal product roadmap status file.
- Repository-level roadmap and remaining feature tracking for Lux itself.

The roadmap and unaddressed product feature list need a collaborative remote
surface. A local ledger is not that surface because it is scoped to an agent run
or worktree and can be absent, stale, ignored, or unavailable to collaborators.

## Decision
Lux roadmap milestones and unaddressed repository/product features are tracked
in GitHub Issues.

`.ledger`-style files are only for local worktree decision recording. They must
not become the product backlog, milestone registry, or remote collaboration
surface.

`.lux/tickets/*.json` remains an execution queue for target-project or run
tasks. It is not a replacement for GitHub Issues and must not be used as the
canonical registry of repository roadmap work.

`.lux/roadmap.json` remains a minimal runtime status and feature-flag source for
Lux. It may name roadmap phases and gate runtime behavior, but GitHub Issues are
the collaboration surface for planning, acceptance criteria, owner discussion,
and remaining unaddressed features.

## Rules
- Register every Lux roadmap milestone that needs work as a GitHub Issue.
- Register every known unaddressed product feature or capability gap as a
  GitHub Issue.
- Use local ledger records only for decisions made in a specific local worktree
  or agent run.
- Do not store roadmap acceptance criteria only in a ledger.
- Do not close or advance roadmap issues solely because `.lux/roadmap.json` or a
  local ledger changed; require implementation and supported-surface evidence.
- Keep `.lux/tickets` focused on executable work units that a Lux run can
  dispatch or verify.

## Consequences
- Collaborators can inspect Lux roadmap and feature gaps without local agent
  state.
- Runtime state remains local-first and evidence-gated.
- Local worktree decisions stay auditable without becoming a second product
  backlog.
- Future CLI/API support should sync to GitHub Issues explicitly rather than
  writing roadmap or feature-tracking data only into local files.
