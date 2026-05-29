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
| Repo green baseline | `cargo build` passes; `cd gateway/ui-src && npx tsc --noEmit` passes; `cargo test` has 1 pre-existing failure in `capture_integration_session_stream_input_stop_and_health`. | Green baseline before roadmap automation. | One known pre-existing test failure remains. | Critical | Track the capture integration failure separately; do not treat roadmap docs as fixing baseline. |

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
- `Skills/` contains 20 skills.
- `bridge/` contains 37 C# bridge files.
- `seeds/` directory does not exist in the current codebase reality used for this lock.

## Default Decisions & Invariants

- **Canonical SSoT**: `.lux/roadmap.json` is the single source of truth for roadmap and status.
- **Ambiguity Polarity**: `0.0` = fully clear, `1.0` = maximally ambiguous.
- **Convergence Target**: Ambiguity must be `<= 0.02` for a spec to be considered locked.
- **Remote/WebRTC**: Classified as **hidden experimental**. Not part of the public roadmap or completion evidence; the only opt-in is `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`, and the default value is disabled.

## Integrated Verification Evidence

Current verification evidence:

```bash
cd gateway && cargo build     # PASS
cd gateway && cargo test      # 1 pre-existing failure (capture_integration test)
cd gateway/ui-src && npx tsc --noEmit  # PASS
```

---
*Note: This document is the canonical engineering assessment. README and other documentation are projections of this state.*
