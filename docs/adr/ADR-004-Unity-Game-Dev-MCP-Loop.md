# ADR-004: Unity Game-Development MCP Loop Shape

## Status
Accepted (Phase 1 follow-up, 2026-05-23)

## Context
Lux needs an MCP surface that can prove one installed-bridge Unity game-development
loop for AI clients: install or refresh the bridge, diagnose it, write/import a
minimal spec, prepare a safe ticket, perform one Unity maneuver, validate/capture
evidence, update `.lux`, and stop.

The repository already separates durable state under `.lux/`:

- `.lux/spec.json` for project/game requirements.
- `.lux/tickets/*.json` for execution tasks.
- `.lux/run-state.json` for active run lifecycle.
- `.lux/roadmap.json` for Lux product roadmap state.

The new MCP workflow must not reintroduce split-brain state, hide Unity failures,
or grow a Unity Editor window UI as part of this milestone.

## Decision
Implemented the game-development MCP milestone as **primitive tools plus one
bounded orchestrator**:

- `lux_game_spec_write` writes/imports the minimal spec through Lux spec helpers.
- `lux_game_ticket_prepare` creates or selects one safe first-loop ticket through
  the Lux ticket store.
- `lux_unity_maneuver` performs one safe maneuver using existing bridge, uloop,
  capture, and verification surfaces before any bridge protocol expansion.
- `lux_game_dev_loop_once` sequences the full first loop and then stops.

Existing `lux_bridge_install` and `lux_bridge_diagnostics` remain stable MCP
tools. Tool results include structured output; failures use `isError: true` but
preserve JSON-RPC connection health.

## Contract
`lux_game_dev_loop_once` must expose a step-by-step `structuredContent.steps[]`
trace and a `stopReason` value. A successful first milestone stops with
`one_verified_loop_complete`. Failure stop reasons must be specific, for example
`bridge_unavailable`, `unity_unavailable`, `validation_failed`, or
`state_write_failed`.

All durable spec, ticket, run, and evidence state belongs under `.lux/`. Evidence
for code, scene, settings, package, asset, compile/test, logs, screenshots, and
unavailable-environment failures must be referenced from the ticket/run output
when available.

## Non-goals
- No Unity Editor window or panel for this milestone.
- No destructive rewrites or broad project churn by default.
- No silent fallback data when Unity or bridge discovery is unavailable.
- No state root outside `.lux/`.
- No unbounded autonomous multi-loop execution; follow-up orchestration belongs
  to Team/Ultragoal or a later Lux run workflow.

## Consequences
- MCP clients get both debuggable primitives and a single demo-oriented loop
  entrypoint.
- Tests can cover tool listing, primitive idempotency, orchestrator failure
  resilience, and JSON-RPC connection health independently.
- Documentation and skills must route AI clients toward `.lux` state/evidence
  and explicit failure reporting.

## Verification Results
Team/Ultragoal evidence collected:

1. MCP list-tools evidence showing existing bridge tools and the four game-dev
   tools (Completed).
2. Focused JSONL smoke tests for spec write, ticket prepare, loop-once failure
   resilience, and ping-after-failure behavior were performed.
3. `.lux` spec/ticket/evidence samples or explicit unavailable-environment
   failure evidence were collected.
4. `cd gateway && cargo build && cargo test` results were verified.
