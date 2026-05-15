# ADR-003: Domain File Separation Under .lux/

## Status
Accepted (Phase 1, 2026-05-14)

## Context
README claimed `.lux/roadmap.json` as universal SSoT. But roadmap, spec, tickets, and run-state were conflated:
- `lux goal` wanted roadmap.active_milestone_id for game milestones
- SpecProject already has roadmap.tickets
- RoadmapReality stored phases but individual gaps/tickets don't fit

## Decision
Canonical homes by domain:

| File | Domain | Owner | Writer |
|------|--------|-------|--------|
| `.lux/spec.json` | Target project goals, game/design assumptions, requirements | User/AI | lux_spec save/load |
| `.lux/tickets/*.json` | Execution tasks derived from spec/goals | Gateway | Ticket CRUD |
| `.lux/run-state.json` | Active run lifecycle state | Gateway ONLY | State machine |
| `.lux/roadmap.json` | LUX implementation milestones (M1-M5) ONLY | lux_roadmap init/load |

Key rules:
- M1-M5 are LUX implementation milestones, NOT user game milestones
- `lux goal` creates/updates spec goals + run-state pointers, NOT roadmap entries
- Roadmap stores LUX product progress (phase C=85%, D=65%, etc.), NOT game features
- Schema tests must reject domain leakage (tickets inside roadmap, state inside spec)

## Consequences
- Resolves VV#1 (roadmap SSoT overloaded)
- Each file has one responsibility
- Prevents mega-roadmap that tries to be both product plan AND run database
