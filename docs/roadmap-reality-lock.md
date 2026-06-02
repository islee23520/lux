# Roadmap Reality Lock: Engineering Gap Matrix

This document serves as the authoritative engineering assessment for the **Roadmap Reality Lock** milestone. It reconciles the current repository state with the long-term target of autonomous Unity development.

This file is a repository docs projection. Runtime roadmap, spec, run, ticket, and evidence truth remains under `.lux/`, with `.lux/specs/` serving as the GDD SSoT for game-domain intent. If this document and `.lux/` disagree, `.lux/` wins and the document must be refreshed from supported evidence.

## Current vs Target Gap Matrix

| Area | Current State | Target State | Gap | Severity | This Plan Action |
|---|---|---|---|---|---|
| Roadmap/status truth | `gateway/src/lux_roadmap.rs` exists with `RoadmapReality` and M1-M5 phase tracking; docs are projections that must follow this implementation. | One canonical source of roadmap truth. | Historical docs can still drift from gateway reality if not refreshed. | Critical | Keep `.lux/roadmap.json`/roadmap loader as the canonical status path and update projections from code evidence. |
| Domain schema | `gateway/src/lux_spec.rs:523` and `crates/lux-spec-core/src/domain.rs:1` now project the canonical game-domain set with migration aliases. Legacy docs/templates still referenced the older domain-path layout and the old `design`/`architecture` list. | Canonical game domains plus explicit aliases and migration receipts. | Docs and template projections can still lag runtime naming if not refreshed together. | High | Keep the runtime schema canonical and update projections whenever the alias map changes. |
| Ambiguity semantics | `gateway/src/lux_spec.rs:160` stops when report <= target; `gateway/src/lux_loop.rs:19` has threshold 0.65; gateway templates may still project separate evaluator guidance. | Consistent ambiguity polarity and threshold across Rust, MCP/API, and workflow-skill projections. | Stop/continue conditions may invert. | Critical | Add contract tests and code comments/API docs requiring low = clear, target <=0.02. |
| Socratic spec loop | `gateway/src/lux_spec_loop.rs` is implemented with proposal/approval flow and question/approve/reject/apply endpoints. | Autonomous Socratic convergence loop. | Human-gated proposal flow exists; autonomous convergence is not yet verified. | Medium | Treat as scaffolded implementation until autonomous convergence evidence exists. |
| Ticket system | `gateway/src/lux_ticket.rs`, `gateway/src/lux_ticket_executor.rs`, and `gateway/src/lux_triage.rs` exist; ticket store supports CRUD, filtering, status tracking, executor, and triage. | Execution-grade tickets with acceptance/evidence/milestone refs. | Core system is implemented; autonomous execution schema extension remains pending. | High | Extend schema/provenance only after convergence requirements are locked. |
| Milestone execution | `gateway/src/lux_roadmap.rs` exists; roadmap loading, status tracking, and feature flags are implemented. | Durable milestone graph and executor. | Roadmap status exists; full milestone executor remains follow-on. | High | Use existing roadmap loader/feature flags as the milestone truth foundation. |
| Verification/blockers | `gateway/src/lux_verification.rs` exists with blocker ticket creation, blocker checks, and blocker resolution request endpoint support. | Autonomous blocker resolution. | Blockers are tracked and resolution can be requested; autonomous resolution is not implemented. | High | Keep blocker autonomy as follow-on and require evidence before marking complete. |
| Game context observation | Unity context, hierarchy, logs, screenshots, uloop passthrough, and capture surfaces exist across gateway/bridge code and docs; the adapter contract is now locked as a text/JSON schema in `gateway/src/lux_game_context.rs`. | Adapter producers must populate scene, object, component, coordinate, camera/UI, log, PlayMode, screenshot, optional vision, `.lux/specs`, ticket, run-evidence, and capability status fields before execution is marked complete. | Pixel-only or command-only review can miss the link between a visual symptom and the GameObject/component/coordinate that caused it. | Critical | Keep engine adapters capability-routed; vision evidence is supplemental and must link back to engine context or blocker evidence. |
| Server projection | CLI/API/MCP fields expose ambiguity, decisions, capabilities, next goal, and evidence status from gateway/runtime state. | Evidence-gated server projections that never become a second source of truth. | API consumers can accidentally treat projections as stronger proof than the underlying evidence. | High | Keep projection payloads tied to `.lux/` evidence and supported endpoints. |
| Engine capability routing | Unity, Godot, and Three.js are listed separately by supported command maturity. | Per-engine capability routing without implying equal verification maturity. | Users may mistake partial or planned engines for Unity parity. | High | Keep capability rows explicit and require evidence before moving any engine command up a maturity tier. |
| Agent execution templates | `gateway/src/templates/plugin/` includes template assets, but the legacy `adapters/opencode/` source root is removed from the server/MCP-only repository shape. | Ticket-driven execution through supported gateway/MCP surfaces. | Agent execution is scaffolded and template-backed; ticket-driven execution provenance is not complete. | High | Guardrail remains: template-driven execution without ticket provenance is not completion evidence. |
| Direct FS / gateway SSoT | Gateway templates include loaders and clients that read/write local state; not all state is gateway-mediated. | Observable, validated SSoT path. | Some orchestration bypasses gateway validation/audit. | High | Inventory and document; do not refactor all template state access in this plan. |
| remote/WebRTC | `/api/remote/sessions` routes exist but are hidden experimental behind `experimental_flags.remote_webrtc=true`. | User/README out-of-scope says no user-facing remote streaming. | Product direction is gated but must stay visibly experimental. | High | Keep disabled by default and exclude from release evidence. |
| Repo green baseline | `cargo build`, `cargo test`, `scripts/check-project-structure.sh`, and `Skills/tools/validate-skills.sh` pass after topology and skill metadata hardening. | Green baseline before roadmap automation. | Behavioral quality of individual skills still needs workflow-level QA beyond schema validation. | Critical | Keep baseline commands green and keep bundled skill schema validation passing before packaging bundled skills as release-ready. |

## Follow-on Milestones

The following milestones are sequenced after the **Roadmap Reality Lock** is established. Several now have scaffolding or partial implementation, but none should be treated as full autonomous Unity development until their success criteria are verified.

### M1: Game Context Schema & Defaults
- **Entry Criteria**: Roadmap Reality Lock complete; gap matrix established.
- **Description**: Migration to a game-first schema that locks GDD/spec domains plus the AI observation units needed for actual game development: scene hierarchy, selected object/component snapshots, Transform/RectTransform/Collider values, camera/UI coordinate state, logs, PlayMode/input traces, screenshots, and optional vision annotations.
- **Success Criteria**: Game requirements and engine observations are represented as stable `.lux` text/JSON evidence; screenshot and vision evidence can supplement but not replace object/component/coordinate/log context.
- **Status**: Contract implemented — `gateway/src/lux_game_context.rs` defines the Game Context Adapter observation schema, while engine-specific producers remain capability-routed and must write blocker evidence when a surface cannot provide required observations.

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

### M4: Ticket-Driven Agent Executor
- **Entry Criteria**: M3 complete; agent execution templates and hooks scaffolded.
- **Description**: Integration of agent execution prompts with the ticket/milestone graph. The executor drives an AI client only through supported gateway/MCP surfaces until the ticket's acceptance criteria are met and evidence is recorded.
- **Success Criteria**: Agent execution is driven by ticket provenance until the milestone is reached.
- **Status**: Scaffolded — `gateway/src/lux_hooks.rs` and gateway templates exist; full ticket-driven agent execution loop remains unverified.

### M5: Blocker Auto-Resolution Graph
- **Entry Criteria**: M4 complete; blocker detection active.
- **Description**: Autonomous detection, classification, and resolution orchestration of blockers. The system identifies circular dependencies or environment issues and attempts self-healing or alternative pathing.
- **Success Criteria**: Autonomous blocker resolution orchestration without human intervention where possible.
- **Status**: Planned — `gateway/src/lux_verification.rs` creates blockers, but autonomous resolution is not implemented.

### M6: Autonomous — Spec-to-Ticket-to-Execution Pipeline
- **Entry Criteria**: M5 complete; blocker auto-resolution proven.
- **Description**: Full autonomous pipeline from spec convergence through ticket generation to agent execution and T3 Unity verification. The system drives itself from a locked spec to a pushed milestone without human intervention.
- **Success Criteria**: Spec → Ticket → agent execution → T3 Unity verification completes autonomously; milestone is pushed only after T3 evidence is recorded.
- **Status**: Planned — `gateway/src/lux_run_state.rs` has M6 states, but the full autonomous pipeline is not implemented.

## Key Repository Facts

- `gateway/src/` contains 59 Rust source files.
- Gateway exposes 120+ API and WebSocket routes.
- `Skills/skills/` is the tracked source tree for bundled and federated skills.
- `bridge/` is registered as the Unity/Godot/Three.js bridge source area and is declared as a git submodule in `.gitmodules`.
- `adapters/`, `gateway/ui-src/`, and `gateway/ui/` are removed roots and must not be treated as active source.
- `seeds/` directory does not exist in the current codebase reality used for this lock.

## Repository Topology Lock

This repository is currently split into runtime state, gateway/server source, bridge sources, and skill sources. These ownership boundaries are part of the Roadmap Reality Lock and must be kept consistent across README, usage docs, installer code, and structure checks.

| Area | Current ownership | Current path | Notes |
|---|---|---|---|
| Runtime state | Lux project SSoT | `.lux/` | Gateway state, roadmap, tickets, run state, and evidence enter through defined write paths only. |
| Gateway control plane | Tracked source | `gateway/` | Rust CLI, Axum HTTP/WS server, MCP server, and installer surfaces. |
| Skills source | Tracked source tree | `Skills/skills/` | `Skills/` is present as a normal tracked tree in this checkout; it must not be treated as a runtime SSoT. |
| Bridge source | Submodule-backed source area | `bridge/` | Fresh checkouts must make the bridge source availability explicit before bridge install can be considered verified. |
| Removed UI and adapter roots | Not live source | `gateway/ui-src/`, `gateway/ui/`, `adapters/` absent | Frontend, TUI, GUI, and legacy adapter roots are not active source in the server/MCP-only architecture. |
| Removed legacy seed artifacts | Not live source | `seeds/` absent | Deleted seed patches and manifests are not active source unless a supported surface proves they must be restored. |

## Default Decisions & Invariants

- **Canonical SSoT**: `.lux/roadmap.json` is the single source of truth for roadmap and status.
- **GDD SSoT**: `.lux/specs/` is the single source of truth for game-design domains; repository docs projection pages may summarize it but must not replace it.
- **Evidence-gated projection**: CLI, MCP/API, and docs status fields for ambiguity, decisions, capabilities, next goal, and evidence status must be backed by `.lux/` state or supported gateway endpoints.
- **Evidence-gated run completion**: `lux run` may orchestrate generated source changes only when completion links accepted artifacts such as command output, tests, logs, game-context snapshots, screenshots, uloop/manual QA transcripts, or explicit blocker reports.
- **Capability routing**: Engine support is routed by command and maturity; partial Godot and planned Three.js surfaces do not imply equal verification maturity with Unity.
- **Game Context Adapter**: AI-visible game state must be context-first. Scene, object, component, coordinate, camera, UI, log, and PlayMode data are first-class text/JSON evidence; screenshots and vision annotations are supplemental and must link back to engine context or explicit blocker evidence.
- **Game harness event schema**: Runtime plan, step, and iteration updates use `game_harness.plan.started`, `game_harness.plan.completed`, `game_harness.step.started`, `game_harness.step.completed`, `game_harness.step.failed`, `game_harness.iteration.started`, and `game_harness.iteration.completed`. These events are evidence-gated projections, not proof of production-ready autonomous completion by themselves.
- **Ambiguity Polarity**: `0.0` = fully clear, `1.0` = maximally ambiguous.
- **Convergence Target**: Ambiguity must be `<= 0.02` for a spec to be considered locked.
- **Remote/WebRTC**: Classified as **hidden experimental**. Not part of the public roadmap or completion evidence; the only opt-in is `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`, and the default value is disabled.

## Trajectory Recommendation

The current repository trajectory should **Continue**, but only through a hardening gate before any new M1-M6 autonomy implementation starts.

| Axis | Recommendation | Rationale | Rollback rule |
|---|---|---|---|
| `Skills/skills/` | **Continue** as the tracked skill source tree. | The validator scans the real tracked path by default, and the tracked skill tree now passes schema validation. | Restore an old skill layout only if a supported install or validation surface proves the current path cannot satisfy it. |
| `bridge/` | **Continue**, with explicit submodule initialization. | The Unity bridge source lives under `bridge/unity/` and installs to `Assets/Editor/LuxBridge/`, matching the documented Unity target layout. | Restore `Assets/Editor/AiBridgeEditor/` only if a supported Unity install surface requires that exact path. |
| Deleted `seeds/` | **Do not revert**. | The uloop manifest fallback now uses a live bundled asset under `gateway/assets/`. No active surface requires deleted seed files. | Restore a deleted seed artifact only when a supported command fails and the evidence names that artifact as required input. |
| Roadmap docs | **Split** docs from runtime truth. | README and usage docs are projections; `.lux/roadmap.json` and gateway loaders remain the SSoT. | Roll back a doc projection only when it contradicts gateway behavior or `.lux/` runtime truth. |
| M1-M6 autonomy | **Split** into future milestones. | Scaffolding exists, but autonomous spec convergence, ticket execution, blocker resolution, and T3 push completion are not proven. | Do not mark any M1-M6 milestone complete without fresh evidence from its supported runtime surface. |

Cleanup rollback policy: deleted artifacts are not restored speculatively. A restore requires a failing supported surface, an evidence file naming the missing artifact, and a scoped fix that preserves `.lux/` as runtime SSoT.

## Trajectory Hardening Release Gate

Publish this trajectory only when the hardening gate is green:

| Gate | Command or evidence | Required result |
|---|---|---|
| Repository topology | `bash scripts/check-project-structure.sh` / `evidence/task-8-structure.txt` | Passes against `gateway`, `bridge`, `Skills/skills`, `docs`, and `scripts`; removed roots stay absent. |
| Rust build | `(cd gateway && cargo build)` / `evidence/task-8-cargo-build.txt` | Exit 0. |
| Rust tests | `(cd gateway && cargo test)` / `evidence/task-8-cargo-test.txt` | Exit 0. |
| Skills validation evidence | `bash Skills/tools/validate-skills.sh` / `evidence/skills-gate-task-8-validate.txt` | Exit 0 with `PASS: summary - all skill checks passed`; this proves schema validity, not full behavioral completeness of every skill workflow. |
| Stale path scan | `evidence/task-8-stale-reference-scan.txt` | No active references to removed seed/plugin paths. |

## Release-Ready Verification Report

The release-ready verification report for the verification-ontology split is indexed in `evidence/worktree-10/task-18-evidence-index.txt`.
It classifies the repository into four evidence-bearing buckets:

- **Implemented**: V1-V17 are complete and backed by task evidence bundles.
- **Scaffolded**: V19 and V20 exist as pending support surfaces for the evidence ledger and protocol compatibility review.
- **Planned**: H1-H14 remain the game harness overhaul plan and are not release-ready yet.
- **Blocked**: Final Verification stays blocked until the remaining follow-on surfaces finish and the reviewer gate approves the full diff, evidence, and cleanup receipts.
- Cleanup instructions are recorded below and require tmux teardown plus removal of any temporary directories, fake npm roots, or scratch worktrees.

This report does not upgrade any unsupported capability claim. Remote-streaming verification, browser-mediated Unity control, fake engine parity, and image-only completion remain out of scope unless a supported surface proves otherwise.

The trajectory hardening gate is green, but M1-M6 implementation still requires a separate plan and fresh supported-surface evidence. This gate does not certify autonomous Unity development; it certifies that the current repository shape, installer paths, bridge layout, skill schema validation, and verification baseline are coherent enough to build on.

## Integrated Verification Evidence

Current verification evidence:

```bash
bash scripts/check-project-structure.sh        # PASS
cd gateway && cargo build                      # PASS
cd gateway && cargo test                       # PASS
bash Skills/tools/validate-skills.sh           # PASS
```

---
*Note: This document is the canonical engineering assessment. README and other documentation are projections of this state.*
