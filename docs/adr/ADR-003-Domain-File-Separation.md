# ADR-003: Domain File Separation Under .lux/

## Status
Accepted (Phase 1, 2026-05-14)

Superseded in part by the current specs contract: `.lux/specs/spec.json` is the
canonical game spec path.
`.lux/spec.json` remains only as a compatibility mirror and legacy read fallback
for older projects and tests.

## Context
README claimed `.lux/roadmap.json` as universal SSoT. But roadmap, spec, tickets, and run-state were conflated:
- `lux goal` wanted roadmap.active_milestone_id for game milestones
- SpecProject already has roadmap.tickets
- RoadmapReality stored phases but individual gaps/tickets don't fit

## Decision
Canonical homes by domain:

| File | Domain | Owner | Writer |
|------|--------|-------|--------|
| `.lux/specs/spec.json` | Target project goals, game/design assumptions, requirements | User/AI | lux_spec save/load |
| `.lux/spec.json` | Compatibility mirror / legacy read fallback for older projects | Gateway | lux_spec compatibility write/read |
| `.lux/tickets/*.json` | Execution tasks derived from spec/goals | Gateway | Ticket CRUD |
| `.lux/run-state.json` | Active run lifecycle state | Gateway ONLY | State machine |
| `.lux/roadmap.json` | Minimal LUX runtime roadmap status and feature flags | lux_roadmap init/load |
| GitHub Issues | LUX implementation milestones and unaddressed product features | Repository maintainers | GitHub issue registration |
| `.ledger`-style local records | Worktree-local decision receipts only | Local agent/worktree | Agent tooling |

Key rules:
- M1-M5 are LUX implementation milestones, NOT user game milestones
- `lux goal` creates/updates spec goals + run-state pointers, NOT roadmap entries
- GitHub Issues track collaborator-visible Lux roadmap work, acceptance criteria, and remaining unaddressed product features
- `.lux/roadmap.json` stores minimal runtime phase status and feature gates, NOT the full product backlog
- Local ledger records are scoped to worktree decisions and must not replace GitHub Issues
- Schema tests must reject domain leakage (tickets inside roadmap, state inside spec)

## Consequences
- Resolves VV#1 (roadmap SSoT overloaded)
- Each file has one responsibility
- Prevents mega-roadmap that tries to be both product plan AND run database
