# Roadmap Reality Lock: Engineering Gap Matrix

This document serves as the authoritative engineering assessment for the **Roadmap Reality Lock** milestone. It reconciles the current repository state with the long-term target of autonomous Unity development.

## Current vs Target Gap Matrix

| Area | Current State | Target State | Gap | Severity | This Plan Action |
|---|---|---|---|---|---|
| Roadmap/status truth | `README.md` claims A-E complete; `seeds/lux-roadmap-v2.seed.yaml` is more conservative; `gateway/README.md` still calls gateway Phase 1/prototype-like. | One canonical source of roadmap truth. | Multiple truths violate `.lux` SSoT and consistency. | Critical | Introduce `.lux/roadmap.json` schema/loader and update docs/seeds to match evidence. |
| Domain schema | `gateway/src/lux_spec.rs:483` has `SpecDomains`; baseline found 7 built-ins + custom map. Templates contain 9 markdown files including packages/testing, but not canonical 9 built-in domains. | Exactly 9 canonical domains plus defaults. | Target schema not canonically represented. | High | Record as follow-on; do not implement full v3 domain migration here. Add roadmap gap entry. |
| Ambiguity semantics | `gateway/src/lux_spec.rs:160` stops when report <= target; `gateway/src/lux_loop.rs:19` has threshold 0.65; plugin evaluator uses separate logic. | Consistent ambiguity polarity and threshold across Rust/plugin/UI. | Stop/continue conditions may invert. | Critical | Add contract tests and code comments/API docs requiring low = clear, target <=0.02. |
| Socratic spec loop | `gateway/src/lux_spec_loop.rs` exists with proposal/approval flow; max-question and approval-gated behavior remain. | Autonomous Socratic convergence loop. | Partial implementation only. | Medium | Document as follow-on; avoid expanding scope now. |
| Ticket system | `gateway/src/lux_ticket.rs:10` has `Ticket`; `gateway/src/lux_ticket.rs:25` has `TicketStatus`; plugin references ticket progress. | Execution-grade tickets with acceptance/evidence/milestone refs. | Canonical Rust ticket schema is not yet sufficient for autonomous execution. | High | Record required schema extension in roadmap; avoid building executor now. |
| Milestone execution | No `gateway/src/lux_roadmap*`; no tests matching `gateway/tests/lux_*roadmap*`. | Durable milestone graph and executor. | Missing. | High | Create roadmap reality schema now; leave executor to follow-on. |
| Verification/blockers | `gateway/src/lux_verification.rs` creates blocker tickets; `gateway/src/lux_ticket.rs:266` has blocker checks. | Autonomous blocker resolution. | Blockers are tracked, not resolved; missing graph validation/dedupe. | High | Record blocker autonomy as follow-on and ensure reality lock does not claim it complete. |
| OpenCode prompt injection | `gateway/src/templates/plugin/continuation-injector.ts`, `prompt-builder.ts`, `next-action-generator.ts`, `plugins/opencode/lux-plugin.ts` exist. | Ticket-driven OpenCode hook execution until milestone. | Continuation exists but is not milestone/ticket-executor driven. | High | Mark as scaffolded; add guardrail that prompt injection without ticket provenance is not completion evidence. |
| Direct FS / gateway SSoT | Plugin templates include loaders and clients that read/write local state; not all state is gateway-mediated. | Observable, validated SSoT path. | Some orchestration bypasses gateway validation/audit. | High | Inventory and document; do not refactor all plugin state access in this plan. |
| remote/WebRTC | `gateway/README.md` lists remote control as deferred; repo has remote/WebRTC UI/API traces. | User/README out-of-scope says no remote streaming. | Product direction drift. | High | Hide as experimental by default and update roadmap status accordingly. |
| Repo green baseline | Deep validator reported `cd gateway && cargo test` fails in 3 skill adaptation smoke tests. | Green baseline before roadmap automation. | Completion claims are not credible while tests fail. | Critical | Fix only the current cargo smoke failures; no unrelated refactor. |

## Follow-on Milestones (Not Implemented)

The following milestones are sequenced after the **Roadmap Reality Lock** is established. These milestones are **NOT completed by the Roadmap Reality Lock plan** and represent the path toward full autonomous Unity development.

### M1: Canonical 9-Domain Schema & Defaults
- **Entry Criteria**: Roadmap Reality Lock complete; gap matrix established.
- **Description**: Migration to the full 9-domain specification engine (Architecture, UI/UX, Logic, Assets, Testing, Performance, Security, Deployment, Documentation) with built-in defaults and domain-specific validation rules.
- **Success Criteria**: Canonical domain v3 schema is the only accepted input for spec generation; all 9 domains have active validation hooks.
- **Status**: NOT completed by Roadmap Reality Lock plan.

### M2: Ambiguity Convergence & Socratic Loop
- **Entry Criteria**: M1 complete; consistent ambiguity polarity (0.0 = clear) enforced.
- **Description**: Implementation of an autonomous Socratic question-answer loop that identifies spec gaps and drives ambiguity scores down to the target threshold.
- **Success Criteria**: Spec loop autonomously reaches ambiguity <= 0.02 without human intervention for standard feature requests.
- **Status**: NOT completed by Roadmap Reality Lock plan.

### M3: Execution-Grade Ticket Schema
- **Entry Criteria**: M2 complete; spec convergence proven.
- **Description**: Extension of the ticket system to include formal acceptance criteria, evidence references (screenshots, logs, test results), and explicit milestone/domain provenance.
- **Success Criteria**: Tickets contain all data required for an executor to verify completion without external context.
- **Status**: NOT completed by Roadmap Reality Lock plan.

### M4: Ticket-Driven OpenCode Hook Executor
- **Entry Criteria**: M3 complete; OpenCode continuation hooks scaffolded.
- **Description**: Integration of OpenCode prompt injection with the ticket/milestone graph. The executor drives the AI until the ticket's acceptance criteria are met and evidence is recorded.
- **Success Criteria**: Automatic OpenCode prompt injection is driven by ticket provenance until the milestone is reached.
- **Status**: NOT completed by Roadmap Reality Lock plan.

### M5: Blocker Auto-Resolution Graph
- **Entry Criteria**: M4 complete; blocker detection active.
- **Description**: Autonomous detection, classification, and resolution orchestration of blockers. The system identifies circular dependencies or environment issues and attempts self-healing or alternative pathing.
- **Success Criteria**: Autonomous blocker resolution orchestration without human intervention where possible.
- **Status**: NOT completed by Roadmap Reality Lock plan.

## Default Decisions & Invariants

- **Canonical SSoT**: `.lux/roadmap.json` is the single source of truth for roadmap and status.
- **Ambiguity Polarity**: `0.0` = fully clear, `1.0` = maximally ambiguous.
- **Convergence Target**: Ambiguity must be `<= 0.02` for a spec to be considered locked.
- **Remote/WebRTC**: Classified as **hidden experimental**. Not part of the public roadmap or completion evidence; the only opt-in is `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`, and the default value is disabled.

## Integrated Verification Evidence

**Note:** Evidence files from previous session unavailable. Verification criteria below represent REQUIRED gates — each must be satisfied before this phase can be considered complete:

- Cargo build must pass.
- Cargo test must pass.
- UI TypeScript check must pass.
- Policy check must pass.
- Full-stack rollup must pass.

---
*Note: This document is the canonical engineering assessment. README and other documentation are projections of this state.*
