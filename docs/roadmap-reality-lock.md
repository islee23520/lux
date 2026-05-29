# Roadmap Reality Lock: Engineering Gap Matrix

This document serves as the authoritative engineering assessment for the **Roadmap Reality Lock** milestone. It reconciles the current repository state with the long-term target of autonomous Unity development.

## Current vs Target Gap Matrix

| Area | Current State | Target State | Gap | Severity | This Plan Action |
|---|---|---|---|---|---|
| Roadmap/status truth | `gateway/src/lux_roadmap.rs` exists with `RoadmapReality` and M1-M5 phase tracking; docs are projections that must follow this implementation. | One canonical source of roadmap truth. | Historical docs can still drift from gateway reality if not refreshed. | Critical | Keep `.lux/roadmap.json`/roadmap loader as the canonical status path and update projections from code evidence. |
| Domain schema | `gateway/src/lux_spec.rs:483` has `SpecDomains`; baseline found 7 built-ins + custom map. Templates contain 9 markdown files including packages/testing, but not canonical 9 built-in domains. | Exactly 9 canonical domains plus defaults. | Target schema not canonically represented. | High | Record as follow-on; do not implement full v3 domain migration here. Add roadmap gap entry. |
| Ambiguity semantics | `gateway/src/lux_spec.rs:160` stops when report <= target; `gateway/src/lux_loop.rs:19` has threshold 0.65; plugin evaluator uses separate logic. | Consistent ambiguity polarity and threshold across Rust/plugin/UI. | Stop/continue conditions may invert. | Critical | Add contract tests and code comments/API docs requiring low = clear, target <=0.02. |
| Socratic spec loop | `gateway/src/lux_spec_loop.rs` is implemented with proposal/approval flow and question/approve/reject/apply endpoints. | Autonomous Socratic convergence loop. | Human-gated proposal flow exists; autonomous convergence is not yet verified. | Medium | Treat as scaffolded implementation until autonomous convergence evidence exists. |
| Ticket system | `gateway/src/lux_ticket.rs`, `gateway/src/lux_ticket_executor.rs`, and `gateway/src/lux_triage.rs` exist; ticket store supports CRUD, filtering, status tracking, executor, and triage. | Execution-grade tickets with acceptance/evidence/milestone refs. | Core system is implemented; autonomous execution schema extension remains pending. | High | Extend schema/provenance only after convergence requirements are locked. |
| Milestone execution | `gateway/src/lux_roadmap.rs` exists; roadmap loading, status tracking, and feature flags are implemented. | Durable milestone graph and executor. | Roadmap status exists; full milestone executor remains follow-on. | High | Use existing roadmap loader/feature flags as the milestone truth foundation. |
| Verification/blockers | `gateway/src/lux_verification.rs` exists with blocker ticket creation, blocker checks, and blocker resolution request endpoint support. | Autonomous blocker resolution. | Blockers are tracked and resolution can be requested; autonomous resolution is not implemented. | High | Keep blocker autonomy as follow-on and require evidence before marking complete. |
| OpenCode prompt injection | `gateway/src/templates/plugin/` includes `continuation-injector.ts`, `prompt-builder.ts`, and `next-action-generator.ts`; `adapters/opencode/lux-plugin.ts` integrates prompt/context injection. | Ticket-driven OpenCode hook execution until milestone. | Prompt injection is scaffolded and template-backed; ticket-driven execution provenance is not complete. | High | Guardrail remains: prompt injection without ticket provenance is not completion evidence. |
| Direct FS / gateway SSoT | Plugin templates include loaders and clients that read/write local state; not all state is gateway-mediated. | Observable, validated SSoT path. | Some orchestration bypasses gateway validation/audit. | High | Inventory and document; do not refactor all plugin state access in this plan. |
| remote/WebRTC | `/api/remote/sessions` routes exist but are hidden experimental behind `experimental_flags.remote_webrtc=true`. | User/README out-of-scope says no public remote streaming. | Product direction is gated but must stay visibly experimental. | High | Keep disabled by default and exclude from public completion evidence. |
| Repo green baseline | `cargo build`, `cargo test`, `cd gateway/ui-src && npx tsc --noEmit`, `scripts/check-project-structure.sh`, and `Skills/tools/validate-skills.sh` pass after topology and skill metadata hardening. | Green baseline before roadmap automation. | Behavioral quality of individual skills still needs workflow-level QA beyond schema validation. | Critical | Keep baseline commands green and keep bundled skill schema validation passing before packaging bundled skills as release-ready. |

## Follow-on Milestones

The following milestones are sequenced after the **Roadmap Reality Lock** is established. Several now have scaffolding or partial implementation, but none should be treated as full autonomous Unity development until their success criteria are verified.

### M1: Canonical 9-Domain Schema & Defaults
- **Entry Criteria**: Roadmap Reality Lock complete; gap matrix established.
- **Description**: Migration to the full 9-domain specification engine (Architecture, UI/UX, Logic, Assets, Testing, Performance, Security, Deployment, Documentation) with built-in defaults and domain-specific validation rules.
- **Success Criteria**: Canonical domain v3 schema is the only accepted input for spec generation; all 9 domains have active validation hooks.
- **Status**: Partially implemented — `gateway/src/lux_spec.rs` has `SpecDomains`, and templates include 9 domain markdown files; canonical validation/default migration is not complete.

### M2: Ambiguity Convergence & Socratic Loop
- **Entry Criteria**: M1 complete; consistent ambiguity polarity (0.0 = clear) enforced.
- **Description**: Implementation of an autonomous Socratic question-answer loop that identifies spec gaps and drives ambiguity scores down to the target threshold.
- **Success Criteria**: Spec loop autonomously reaches ambiguity <= 0.02 without human intervention for standard feature requests.
- **Status**: Scaffolded — `gateway/src/lux_ambiguity.rs` and `gateway/src/lux_spec_loop.rs` exist; autonomous convergence to the target is not proven.

### M3: Execution-Grade Ticket Schema
- **Entry Criteria**: M2 complete; spec convergence proven.
- **Description**: Extension of the ticket system to include formal acceptance criteria, evidence references (screenshots, logs, test results), and explicit milestone/domain provenance.
- **Success Criteria**: Tickets contain all data required for an executor to verify completion without external context.
- **Status**: Partially implemented — `gateway/src/lux_ticket.rs` and `gateway/src/lux_ticket_executor.rs` exist with acceptance/evidence fields; execution-grade provenance remains incomplete.

### M4: Ticket-Driven OpenCode Hook Executor
- **Entry Criteria**: M3 complete; OpenCode continuation hooks scaffolded.
- **Description**: Integration of OpenCode prompt injection with the ticket/milestone graph. The executor drives the AI until the ticket's acceptance criteria are met and evidence is recorded.
- **Success Criteria**: Automatic OpenCode prompt injection is driven by ticket provenance until the milestone is reached.
- **Status**: Scaffolded — `gateway/src/lux_hooks.rs` and plugin templates exist; full ticket-driven OpenCode execution loop remains unverified.

### M5: Blocker Auto-Resolution Graph
- **Entry Criteria**: M4 complete; blocker detection active.
- **Description**: Autonomous detection, classification, and resolution orchestration of blockers. The system identifies circular dependencies or environment issues and attempts self-healing or alternative pathing.
- **Success Criteria**: Autonomous blocker resolution orchestration without human intervention where possible.
- **Status**: Planned — `gateway/src/lux_verification.rs` creates blockers, but autonomous resolution is not implemented.

### M6: Autonomous — Spec-to-Ticket-to-Execution Pipeline
- **Entry Criteria**: M5 complete; blocker auto-resolution proven.
- **Description**: Full autonomous pipeline from spec convergence through ticket generation to OpenCode execution and T3 Unity verification. The system drives itself from a locked spec to a pushed milestone without human intervention.
- **Success Criteria**: Spec → Ticket → OpenCode execution → T3 Unity verification completes autonomously; milestone is pushed only after T3 evidence is recorded.
- **Status**: Planned — `gateway/src/lux_run_state.rs` has M6 states, but the full autonomous pipeline is not implemented.

## Key Repository Facts

- `gateway/src/` contains 59 Rust source files.
- Gateway exposes 120+ API and WebSocket routes.
- Dashboard has 15 pages plus 2 redirects.
- `Skills/skills/` is the tracked source tree for bundled and federated skills.
- `adapters/opencode/lux-plugin.ts` is the verified OpenCode source adapter path.
- `bridge/` is registered as the Unity/Godot/Three.js bridge source area and is declared as a git submodule in `.gitmodules`.
- `seeds/` directory does not exist in the current codebase reality used for this lock.

## Repository Topology Lock

This repository is currently split into runtime state, source adapters, bridge sources, and skill sources. These ownership boundaries are part of the Roadmap Reality Lock and must be kept consistent across README, usage docs, installer code, and structure checks.

| Area | Current ownership | Current path | Notes |
|---|---|---|---|
| Runtime state | Lux project SSoT | `.lux/` | Gateway state, roadmap, tickets, run state, and evidence enter through defined write paths only. |
| Gateway control plane | Tracked source | `gateway/` | Rust CLI, Axum HTTP/WS server, dashboard API, and installer surfaces. |
| Source adapters | Tracked source | `adapters/` | OpenCode is verified at `adapters/opencode/lux-plugin.ts`; Claude, Codex, and Pi Agent adapters are scaffolded. |
| Skills source | Tracked source tree | `Skills/skills/` | `Skills/` is present as a normal tracked tree in this checkout; it must not be treated as a runtime SSoT. |
| Bridge source | Submodule-backed source area | `bridge/` | Fresh checkouts must make the bridge source availability explicit before bridge install can be considered verified. |
| Removed legacy seed artifacts | Not live source | `seeds/` absent | Deleted seed patches and manifests are not active source unless a supported surface proves they must be restored. |

## Default Decisions & Invariants

- **Canonical SSoT**: `.lux/roadmap.json` is the single source of truth for roadmap and status.
- **Ambiguity Polarity**: `0.0` = fully clear, `1.0` = maximally ambiguous.
- **Convergence Target**: Ambiguity must be `<= 0.02` for a spec to be considered locked.
- **Remote/WebRTC**: Classified as **hidden experimental**. Not part of the public roadmap or completion evidence; the only opt-in is `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`, and the default value is disabled.

## Trajectory Recommendation

The current repository trajectory should **Continue**, but only through a hardening gate before any new M1-M6 autonomy implementation starts.

| Axis | Recommendation | Rationale | Rollback rule |
|---|---|---|---|
| `Skills/skills/` | **Continue** as the tracked skill source tree. | The validator scans the real tracked path by default, and the tracked skill tree now passes schema validation. | Restore an old skill layout only if a supported install or validation surface proves the current path cannot satisfy it. |
| `adapters/opencode/` | **Continue** as source adapter ownership. | `lux init` and bridge install now converge on `.opencode/plugins/lux-plugin.ts` copied from `adapters/opencode/lux-plugin.ts`. | Reintroduce a legacy plugin bundle only with failing adapter-install evidence from a supported OpenCode surface. |
| `bridge/` | **Continue**, with explicit submodule initialization. | The Unity bridge source lives under `bridge/unity/` and installs to `Assets/Editor/LuxBridge/`, matching the documented Unity target layout. | Restore `Assets/Editor/AiBridgeEditor/` only if a supported Unity install surface requires that exact path. |
| Deleted `seeds/` | **Do not revert**. | The uloop manifest fallback now uses a live bundled asset under `gateway/assets/`. No active surface requires deleted seed files. | Restore a deleted seed artifact only when a supported command fails and the evidence names that artifact as required input. |
| Roadmap docs | **Split** docs from runtime truth. | README and usage docs are projections; `.lux/roadmap.json` and gateway loaders remain the SSoT. | Roll back a doc projection only when it contradicts gateway behavior or `.lux/` runtime truth. |
| M1-M6 autonomy | **Split** into future milestones. | Scaffolding exists, but autonomous spec convergence, ticket execution, blocker resolution, and T3 push completion are not proven. | Do not mark any M1-M6 milestone complete without fresh evidence from its supported runtime surface. |

Cleanup rollback policy: deleted artifacts are not restored speculatively. A restore requires a failing supported surface, an evidence file naming the missing artifact, and a scoped fix that preserves `.lux/` as runtime SSoT.

## Trajectory Hardening Release Gate

Publish this trajectory only when the hardening gate is green:

| Gate | Command or evidence | Required result |
|---|---|---|
| Repository topology | `bash scripts/check-project-structure.sh` / `evidence/task-8-structure.txt` | Passes against `gateway`, `bridge`, `adapters`, `Skills/skills`, `docs`, and `scripts`; removed roots stay absent. |
| Rust build | `(cd gateway && cargo build)` / `evidence/task-8-cargo-build.txt` | Exit 0. |
| Rust tests | `(cd gateway && cargo test)` / `evidence/task-8-cargo-test.txt` | Exit 0. |
| TypeScript typecheck | `(cd gateway/ui-src && npx tsc --noEmit)` / `evidence/task-8-tsc.txt` | Exit 0. |
| Skills validation evidence | `bash Skills/tools/validate-skills.sh` / `evidence/skills-gate-task-8-validate.txt` | Exit 0 with `PASS: summary - all skill checks passed`; this proves schema validity, not full behavioral completeness of every skill workflow. |
| Stale path scan | `evidence/task-8-stale-reference-scan.txt` | No active references to removed seed/plugin paths. |

The trajectory hardening gate is green, but M1-M6 implementation still requires a separate plan and fresh supported-surface evidence. This gate does not certify autonomous Unity development; it certifies that the current repository shape, installer paths, bridge layout, skill schema validation, and verification baseline are coherent enough to build on.

## Integrated Verification Evidence

Current verification evidence:

```bash
bash scripts/check-project-structure.sh        # PASS
cd gateway && cargo build                      # PASS
cd gateway && cargo test                       # PASS
cd gateway/ui-src && npx tsc --noEmit          # PASS
bash Skills/tools/validate-skills.sh           # PASS
```

---
*Note: This document is the canonical engineering assessment. README and other documentation are projections of this state.*
