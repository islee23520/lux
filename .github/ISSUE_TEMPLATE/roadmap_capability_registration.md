---
name: Roadmap or capability registration
about: Register a Lux roadmap milestone, product feature gap, or capability maturity change
title: "[Roadmap]: "
labels: roadmap, needs-evidence
assignees: ""
---

## Issue Definition

Register this issue only when it describes one durable Lux repository concern:

- A roadmap milestone that still needs implementation, verification, or release evidence.
- A known product feature gap or capability gap.
- A capability maturity change for Unity, Godot, or Three.js.
- A repository projection mismatch where README, docs, skills, gateway, bridge, or `.lux/` runtime truth disagree.

Do not use this issue for local agent notes, one-off worktree decisions, target Unity project tasks, or `.lux/tickets` execution units.

## Project Context

- Area: `gateway` / `crates` / `bridge` / `Skills` / `docs` / `scripts` / `.lux runtime` / `release`
- Engine surface: `Unity` / `Godot` / `Three.js` / `engine-neutral`
- Capability maturity being claimed: `verified` / `partial` / `planned` / `experimental` / `not applicable`
- Related milestone: `Roadmap Reality Lock` / `M1` / `M2` / `M3` / `M4` / `M5` / `M6` / `none`
- Source of truth affected: `.lux/` / GitHub Issues / gateway API / bridge protocol / docs projection / skill projection

## Problem Statement

What is the issue, gap, contradiction, or missing capability?

Describe the current observable behavior and why it matters for Lux's local-first, evidence-gated game automation loop.

## Current Evidence

Attach or link evidence that proves the issue exists. Use supported surfaces whenever possible.

- Code or docs reference:
- Command output:
- Gateway API or MCP output:
- `.lux/` state or evidence path:
- Unity/Godot/Three.js bridge output:
- Screenshot or video, only if paired with engine context where possible:

Evidence condition:

- Claims about runtime state must cite `.lux/` state or a supported gateway/bridge/API surface.
- Claims about Unity support must include compile, status, run, test, screenshot, or bridge evidence as applicable.
- Claims about Godot or Three.js must state the current capability tier and must not imply Unity parity without evidence.
- Docs-only claims are not completion evidence unless the issue is strictly a documentation projection mismatch.
- Local ledger records are decision receipts only; they are not roadmap or completion evidence.

## Acceptance Criteria

This issue can close only when all applicable criteria are met:

- [ ] The implementation or documentation change is scoped to the correct owner boundary: `gateway`, `crates`, `bridge`, `Skills`, `docs`, `scripts`, or `.lux`.
- [ ] `.lux/` remains the runtime SSoT; no second runtime truth is introduced.
- [ ] GitHub Issues remain the collaborator-visible tracking surface for roadmap and unaddressed product work.
- [ ] Capability maturity is stated explicitly as verified, partial, planned, experimental, or not applicable.
- [ ] Planned or adapter-only behavior is not described as completed behavior.
- [ ] Acceptance criteria are backed by supported-surface evidence, not only by local notes.
- [ ] Any schema, API response, event, or bridge protocol change updates all affected consumers.
- [ ] Multi-step state changes are atomic or report explicit blocker evidence.
- [ ] Re-running the workflow, command, or install step converges without duplicate or corrupt state.
- [ ] Any fallback path is explicit, observable, and marked only with an allowed Lux marker when required.

## Verification Checklist

Run the checks that match the affected surface and attach the result:

- [ ] `bash scripts/test-all.sh --quick`
- [ ] `cargo build --workspace`
- [ ] `cargo test --workspace`
- [ ] `cd gateway && cargo run -- bridge install --help`
- [ ] `cd gateway && cargo run -- serve --help`
- [ ] `bash Skills/tools/validate-skills.sh`
- [ ] Unity bridge/status/compile/run evidence captured
- [ ] Godot support evidence captured with partial capability tier preserved
- [ ] Three.js evidence captured without upgrading planned maturity
- [ ] Documentation projection checked against `.lux/` or gateway truth

If a check is not applicable, state why.

## Completion Evidence Required

Before closing, attach the minimum evidence bundle:

- Changed files or PR link:
- Passing command outputs:
- Runtime evidence path under `.lux/`, when runtime behavior changed:
- Gateway/API/MCP response, when server behavior changed:
- Bridge or engine transcript, when engine behavior changed:
- Updated docs or skill projection, when public claims changed:
- Explicit blocker issue, when completion cannot be proven through a supported surface:

## Non-Goals

List what this issue intentionally does not cover, especially target Unity project work, GUI/dashboard/TUI work, remote streaming claims, or future autonomy claims beyond the named milestone.
