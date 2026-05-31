# Lux Game Harness Overhaul Plan

## TL;DR
> **Summary**: Reframe LUX as a context-first game-development harness whose canonical flow is `.lux/specs` GDD/spec SSoT -> Socratic ambiguity minimization -> game context observation -> goal/ticket selection -> `lux run` execution -> TDD plus engine-specific manual QA evidence.
> **Deliverables**:
> - `.lux/specs/` canonical GDD/domain/decision model under `.lux` SSoT.
> - Baseline game-domain schema plus LLM-approved custom domains covering Unity, Godot, and Three.js without pretending equal maturity.
> - Game Context Adapter contract that exposes scene hierarchy, selected objects, components, coordinates, camera/UI state, logs, PlayMode state, screenshots, and optional vision as stable AI-readable evidence.
> - Socratic review loop that logs every user answer and compares it against prior decisions.
> - `lux run` goal pursuit contract that chooses the next task from spec ambiguity, tickets, and engine capability.
> - Verification router for TDD, command evidence, screenshots, and optional video per working engine surface.
> **Effort**: XL
> **Parallel**: YES - 5 waves
> **Critical Path**: Task 1 -> Task 2 -> Task 4 -> Task 6 -> Task 8 -> Task 10 -> Task 12

## Context
### Original Request
The user wants LUX to be an overhaul harness tool specifically for games. Initial planning/design should be decided by an LLM through Socratic review rounds that minimize ambiguity. Game development should continue from GDD/spec as SSoT under `.lux/specs`; decisions and user steps must be logged and compared so AI can infer what the human wants. `lux run` should pursue the current step toward the next goal, with automatic TDD and manual engine-specific tests. Unity should use uloop/dynamic-code/screenshot where working; Godot and Three.js should use their real working surfaces or explicit blocker evidence.

### Interview Summary
- Engine scope: all three declared engines: Unity, Godot, Three.js.
- SSoT: `.lux` remains the root SSoT; `.lux/specs` is the canonical GDD/spec sub-root.
- Verification: TDD plus manual tests must respect the active working game engine.
- User preference: evidence can include logs, screenshots, and optional video clips.

### Metis Review (gaps addressed)
- "All three engines" must not mean fake parity. The plan introduces an engine capability router and explicit unsupported/blocker evidence.
- `.lux/specs` must not silently conflict with existing `.lux/spec.json` and `.lux/domains`. The plan makes `.lux/specs` canonical and requires visible migration receipts.
- Socratic answers must not be transient chat. The plan persists answers, decisions, contradictions, and inferred preferences under `.lux/specs/decisions.jsonl`.
- `lux run` currently projects tasks; it must become evidence-gated, not completion-by-dispatch.
- Manual QA must use actual engine surfaces: Unity uloop/dynamic-code/screenshot, Godot CLI/project checks, Three.js dev-server/browser checks.
- Wave dependencies must be serial where evidence inventory is a prerequisite; the plan uses five waves so Task 1 completes before contract/design tasks execute.
- The baseline game-domain set must not block LLM-discovered domains; custom domains require Socratic decision ledger entries and migration-safe spec refs.
- Video capture is optional capability evidence, not a universal requirement; unsupported video must be recorded as capability false or blocker context.

## Work Objectives
### Core Objective
Make LUX's next architecture milestone a game-specific harness contract where game intent is captured in `.lux/specs`, ambiguity is reduced before execution, game/engine state is observed through stable context snapshots, and `lux run` executes only evidence-grade tasks against a detected working game engine.

### Deliverables
- `.lux/specs/gdd.md`, `.lux/specs/spec.json`, `.lux/specs/domains/*.md`, `.lux/specs/decisions.jsonl`, and `.lux/specs/preferences.json` contract.
- A canonical baseline game-domain set: `gdd`, `mechanics`, `controls`, `camera`, `levels`, `art-style`, `audio`, `narrative`, `ui-ux`, `technical-architecture`, `engine`, `testing`, `build-release`.
- A Game Context Adapter contract for scene hierarchy, selected object/component snapshots, Transform/RectTransform/Collider/Camera/UI coordinate state, console/compile logs, PlayMode/input trace, screenshots, and optional vision feedback.
- A rule that screenshot and vision feedback are supplemental evidence; text/JSON context snapshots are the first-class interface that lets AI agents reason about game state without hallucinating from pixels alone.
- LLM-approved custom domain support with decision-ledger provenance.
- Migration from existing `.lux/spec.json` and `.lux/domains/*.md` with explicit receipts.
- Socratic loop that records questions, answers, accepted decisions, rejected decisions, contradictions, and preference inferences.
- Engine capability router for Unity, Godot, and Three.js.
- `lux run` next-goal selection from spec ambiguity, user decisions, execution-grade tickets, and engine capability.
- Verification policy matrix for TDD and manual engine QA.
- Evidence model for command logs, screenshots, optional video, and blocker records.
- Current/next goal persistence under `.lux/goals/current.json` and run-linked goal history under `.lux/specs/decisions.jsonl`.
- CLI and docs that explain the game harness flow without overstating autonomous M1-M6 completion.

### Definition of Done
- `cd gateway && cargo test lux_game_harness` exits 0.
- `cd gateway && cargo test --test gateway_cli_smoke game_harness_` exits 0.
- `cd gateway && cargo test --test gateway_cli_smoke lux_run_` exits 0.
- `bash scripts/check-project-structure.sh` exits 0.
- `bash Skills/tools/validate-skills.sh` exits 0.
- A temp project smoke proves `lux init` creates `.lux/specs/*` and logs an initial decision.
- A temp project smoke proves `lux spec-loop start/answer/apply` appends to `.lux/specs/decisions.jsonl`.
- A temp project smoke proves `lux run --dry-run` writes a next-goal plan that references `.lux/specs`.
- Engine manual QA evidence exists for at least one working engine surface; unavailable engines produce explicit blocker evidence, not pass claims.

### Must Have
- `.lux/` remains the only runtime SSoT root.
- `.lux/specs/` is canonical for game GDD/spec documents.
- Game state observation is context-first: scene, component, coordinate, camera, UI, log, and PlayMode data must be representable as text/JSON evidence before vision-only review can mark work complete.
- Existing `.lux/spec.json` and `.lux/domains` migration is explicit and observable.
- No silent fallback from `.lux/specs` to stale `.lux/domains`.
- Every user answer and accepted/rejected decision is persisted.
- Every LLM-added custom domain and next-goal change is persisted with rationale.
- Ambiguity polarity remains `0.0 = clear`, `1.0 = ambiguous`.
- `lux run` cannot mark game work complete without evidence.

### Must NOT Have
- Do not implement full M1-M6 autonomy in this milestone.
- Do not claim Godot/Three.js runtime parity if only Unity is verified.
- Do not store canonical game requirements outside `.lux`.
- Do not make docs say screenshots/video are always available; they are capability-gated.
- Do not make vision feedback the only verification surface for game behavior; it must be linked back to engine context, object identity, coordinates, logs, or explicit blocker evidence.
- Do not use dry-run evidence as completion evidence for real execution.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD with Rust unit/CLI smoke tests before implementation changes; task acceptance requires RED/GREEN evidence.
- QA policy: Every task has agent-executed scenarios, including one happy path and one failure/blocker path.
- Evidence: `evidence/game-harness-task-{N}-{slug}.txt|json|png|mp4`.
- Manual engine QA:
  - Unity: `lux unity compile`, `lux unity run-tests`, `lux unity execute-dynamic-code`, `lux unity screenshot` through uloop or bridge when available.
  - Godot: detected Godot CLI/project validation and a minimal scene/build/browser/manual surface when available.
  - Three.js: package-script/dev-server/browser screenshot when available.
  - Missing engine tools: explicit blocker evidence under `.lux/evidence/<run>/blockers/*.json`.

## Execution Strategy
### Parallel Execution Waves
Wave 1: Task 1
Wave 2: Task 2, Task 3
Wave 3: Task 4, Task 5, Task 6, Task 7
Wave 4: Task 8, Task 9, Task 10, Task 11
Wave 5: Task 12, Task 13, Task 14

### Dependency Matrix
| Task | Depends On | Blocks |
| --- | --- | --- |
| 1. Current Surface Inventory | none | 2, 3, 4, 7, 12, 15 |
| 2. `.lux/specs` SSoT contract | 1 | 4, 5, 6, 8, 10 |
| 3. Engine capability taxonomy | 1 | 7, 9, 10, 12 |
| 4. Game domain schema and migration | 2 | 5, 6, 8, 10 |
| 5. Decision ledger and preference inference | 2, 4 | 6, 8, 10 |
| 6. Socratic ambiguity minimization contract | 4, 5 | 8, 10 |
| 7. Engine verification router | 3 | 9, 10, 12 |
| 8. Spec-loop CLI/API flow | 4, 5, 6 | 10, 11 |
| 9. Engine manual QA evidence adapters | 3, 7 | 10, 12 |
| 10. `lux run` next-goal execution contract | 5, 6, 7, 8, 9 | 11, 12 |
| 11. Dashboard/docs projection | 8, 10 | 13 |
| 12. End-to-end temp project harness | 7, 9, 10 | 13, 14 |
| 13. Release docs and guardrails | 11, 12 | 14 |
| 14. Baseline verification report | 12, 13 | Final Verification |
| 15. Game Context Adapter contract | 1, 2, 3 | 7, 9, 10, 12 |

## TODOs
- [ ] 1. Current Surface Inventory

  **What to do**: Capture the current code/docs surface for spec, spec-loop, run, ticket, verification, uloop, Godot, and Three.js paths. Store an inventory table that names what is implemented, scaffolded, unsupported, or stale.
  **Must NOT do**: Do not edit source or docs in this task.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 2, 3, 4, 7, 12 | Blocked By: none

  **References**:
  - Spec model: `gateway/src/lux_spec.rs`
  - Socratic loop: `gateway/src/lux_spec_loop.rs`
  - Run lifecycle: `gateway/src/lux_run.rs`
  - Run state: `gateway/src/lux_run_state.rs`
  - Verification router: `gateway/src/lux_verification.rs`
  - CLI routing: `gateway/src/main.rs`
  - Existing assessment: `docs/roadmap-reality-lock.md`

  **Acceptance Criteria**:
  - [ ] Evidence names every current `.lux/spec*` and `.lux/domain*` path used by code.
  - [ ] Evidence names engine-specific commands and their maturity: Unity, Godot, Three.js.
  - [ ] Evidence lists existing tests that should be extended.

  **QA Scenarios**:
  ```text
  Scenario: Inventory captures current game harness surfaces
    Tool: bash
    Steps: rg for spec/run/ticket/verification/uloop/godot/threejs surfaces and save summary
    Expected: evidence file contains paths and maturity labels for each subsystem.
    Evidence: evidence/game-harness-task-1-inventory.txt

  Scenario: No source mutation during inventory
    Tool: bash
    Steps: git diff --name-only before and after inventory
    Expected: only evidence files changed.
    Evidence: evidence/game-harness-task-1-no-mutation.txt
  ```

  **Commit**: NO | Message: n/a | Files: evidence only

- [ ] 2. `.lux/specs` SSoT Contract

  **What to do**: Define the `.lux/specs` runtime contract and write TDD tests first. Canonical files: `.lux/specs/gdd.md`, `.lux/specs/spec.json`, `.lux/specs/domains/*.md`, `.lux/specs/decisions.jsonl`, `.lux/specs/preferences.json`, `.lux/specs/migration.json`. Existing `.lux/spec.json` and `.lux/domains` must migrate into this layout with an explicit migration receipt.
  **Must NOT do**: Do not silently read stale `.lux/domains` as canonical after `.lux/specs` exists.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 4, 5, 6, 8, 10 | Blocked By: 1

  **References**:
  - Current load/save: `gateway/src/lux_spec.rs`
  - Atomic writes: `gateway/src/lux_io.rs`
  - Project SSoT invariant: `README.md` and `docs/roadmap-reality-lock.md`

  **Acceptance Criteria**:
  - [ ] RED test proves current `lux init` does not create `.lux/specs/gdd.md`.
  - [ ] GREEN test proves `lux init --project-path <tmp> --no-interactive` creates all canonical `.lux/specs` files.
  - [ ] Migration receipt records source paths and timestamp.

  **QA Scenarios**:
  ```text
  Scenario: Fresh project creates canonical game specs
    Tool: bash
    Steps: cd gateway && cargo run -- init --project-path <tmp> --no-interactive; find <tmp>/.lux/specs -maxdepth 3 -type f
    Expected: gdd.md, spec.json, domains, decisions.jsonl, preferences.json, migration.json exist.
    Evidence: evidence/game-harness-task-2-fresh-init.txt

  Scenario: Legacy domain path is migrated visibly
    Tool: bash
    Steps: create <tmp>/.lux/spec.json and <tmp>/.lux/domains/design.md, run init, inspect migration.json
    Expected: `.lux/specs/migration.json` records `.lux/spec.json` and `.lux/domains/design.md`; no silent fallback message.
    Evidence: evidence/game-harness-task-2-migration.txt
  ```

  **Commit**: YES | Message: `feat(spec): establish game specs ssot` | Files: `gateway/src/lux_spec.rs`, tests, docs

- [ ] 3. Engine Capability Taxonomy

  **What to do**: Define a persisted engine capability model that can express Unity, Godot, and Three.js as `detected`, `tool_available`, `manual_qa_supported`, `screenshot_supported`, `video_supported`, and `blocker_reason`. Detection must be explicit and evidence-backed.
  **Must NOT do**: Do not mark an engine as verified because a doc mentions it.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 7, 9, 10, 12 | Blocked By: 1

  **References**:
  - Unity detection: `gateway/src/project.rs`
  - Unity uloop runner: `gateway/src/uloop_runner.rs`
  - CLI engine docs: `README.md`, `docs/godot-support.md`
  - Project structure check: `scripts/check-project-structure.sh`

  **Acceptance Criteria**:
  - [ ] RED test proves unsupported engines currently lack blocker evidence.
  - [ ] Capability JSON is written under `.lux/engines/capabilities.json`.
  - [ ] Unity/Godot/Three.js each have a distinct status and reason.

  **QA Scenarios**:
  ```text
  Scenario: Non-engine temp project records unsupported blockers
    Tool: bash
    Steps: run engine capability detection in empty temp dir
    Expected: all engines are not detected and include explicit blocker reasons.
    Evidence: evidence/game-harness-task-3-empty-capabilities.json

  Scenario: Unity-like temp project records Unity detection path
    Tool: bash
    Steps: create minimal Unity markers and run detection
    Expected: Unity is detected; Godot/Three.js remain unsupported unless their markers exist.
    Evidence: evidence/game-harness-task-3-unity-capabilities.json
  ```

  **Commit**: YES | Message: `feat(engine): record game engine capabilities` | Files: `gateway/src/project.rs`, new engine module/tests

- [ ] 4. Game Domain Schema and Migration

  **What to do**: Replace the current 7-domain built-in assumption with a baseline game-harness domain set: `gdd`, `mechanics`, `controls`, `camera`, `levels`, `art-style`, `audio`, `narrative`, `ui-ux`, `technical-architecture`, `engine`, `testing`, `build-release`. Preserve old domain names through explicit migration aliases. Add custom-domain support where the LLM may propose additional domains only through Socratic approval and decision-ledger provenance.
  **Must NOT do**: Do not delete user-authored legacy domain content.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 5, 6, 8, 10 | Blocked By: 2

  **References**:
  - Current domains: `gateway/src/lux_spec.rs`
  - Existing templates: `Skills/docs/templates/*.md`
  - Domain status/progress: `gateway/src/server.rs` spec progress summary

  **Acceptance Criteria**:
  - [ ] Unit tests prove every canonical game domain is initialized.
  - [ ] Existing `design`, `architecture`, `art-style`, `audio`, `narrative`, `levels`, `ui-ux` migrate without content loss.
  - [ ] `packages` and `testing` stop being second-class only; they map into canonical game domains.
  - [ ] Custom domains can be added only with a decision-ledger entry containing rationale and source question.

  **QA Scenarios**:
  ```text
  Scenario: Domain schema includes game-specific domains
    Tool: bash
    Steps: cd gateway && cargo test game_domain_schema
    Expected: canonical domain set exactly matches the planned list.
    Evidence: evidence/game-harness-task-4-domain-tests.txt

  Scenario: Legacy content is preserved
    Tool: bash
    Steps: create old `.lux/domains/design.md`, migrate, rg unique marker under `.lux/specs/domains`
    Expected: marker is preserved in migrated domain content.
    Evidence: evidence/game-harness-task-4-migration-preserves-content.txt

  Scenario: LLM-approved custom domain is provenance-backed
    Tool: bash
    Steps: propose custom domain "economy" through spec-loop approval and inspect `.lux/specs/decisions.jsonl`
    Expected: domain exists only after approval and ledger records rationale/source question.
    Evidence: evidence/game-harness-task-4-custom-domain.txt
  ```

  **Commit**: YES | Message: `feat(spec): add game domain schema` | Files: `gateway/src/lux_spec.rs`, templates, tests

- [ ] 5. Decision Ledger and Preference Inference

  **What to do**: Add a decision ledger API that appends each user answer, accepted proposal, rejected proposal, override, and inferred preference to `.lux/specs/decisions.jsonl`. Add `preferences.json` derivation from repeated decisions with conflict detection.
  **Must NOT do**: Do not overwrite historical decisions; append only.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 6, 8, 10 | Blocked By: 2, 4

  **References**:
  - Current decision fields: `gateway/src/lux_spec.rs` `DialecticState`, `SpecDecision`, `SpecQuestion`
  - Current spec-loop persistence: `gateway/src/lux_spec_loop.rs`
  - Atomic writes: `gateway/src/lux_io.rs`

  **Acceptance Criteria**:
  - [ ] RED test proves current spec-loop answer does not append `.lux/specs/decisions.jsonl`.
  - [ ] GREEN test proves answer/approve/reject/apply append distinct ledger events.
  - [ ] Preference inference identifies at least one repeated user preference and one contradiction.

  **QA Scenarios**:
  ```text
  Scenario: User answers are durable
    Tool: bash
    Steps: run spec-loop start/answer/apply in temp project and tail `.lux/specs/decisions.jsonl`
    Expected: JSONL contains question_answered and proposal_applied events with timestamps.
    Evidence: evidence/game-harness-task-5-decision-ledger.txt

  Scenario: Contradictory answers are surfaced
    Tool: bash
    Steps: answer same preference domain with conflicting values and run preference derivation
    Expected: preferences.json includes conflict entry and ambiguity is not reduced silently.
    Evidence: evidence/game-harness-task-5-conflict.txt
  ```

  **Commit**: YES | Message: `feat(spec): persist decision ledger` | Files: `gateway/src/lux_spec_loop.rs`, new ledger module/tests

- [ ] 6. Socratic Ambiguity Minimization Contract

  **What to do**: Make Socratic review drive toward game-domain clarity. Questions must be generated from missing GDD/domain fields, contradiction ledger, engine capability blockers, and test strategy gaps. Ambiguity target remains `<= 0.02`.
  **Must NOT do**: Do not ask questions that can be answered from existing `.lux/specs` or engine detection.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 8, 10 | Blocked By: 4, 5

  **References**:
  - Ambiguity scorer: `gateway/src/lux_ambiguity.rs`
  - Spec-loop proposal flow: `gateway/src/lux_spec_loop.rs`
  - Existing roadmap gap: `docs/roadmap-reality-lock.md`

  **Acceptance Criteria**:
  - [ ] Tests prove missing mechanics/controls/camera/testing fields create targeted questions.
  - [ ] Tests prove answered questions reduce domain ambiguity.
  - [ ] Contradiction events prevent false ambiguity reduction.

  **QA Scenarios**:
  ```text
  Scenario: Mechanics uncertainty creates a Socratic question
    Tool: bash
    Steps: cd gateway && cargo test socratic_mechanics_question
    Expected: targeted question names mechanics domain and missing field.
    Evidence: evidence/game-harness-task-6-mechanics-question.txt

  Scenario: Contradiction blocks false clarity
    Tool: bash
    Steps: run ambiguity calculation on spec with conflicting preference decisions
    Expected: ambiguity remains above target and recommendation names contradiction.
    Evidence: evidence/game-harness-task-6-contradiction.txt
  ```

  **Commit**: YES | Message: `feat(spec): drive game socratic clarity` | Files: `gateway/src/lux_ambiguity.rs`, `gateway/src/lux_spec_loop.rs`, tests

- [ ] 7. Engine Verification Router

  **What to do**: Extend verification policy routing so each ticket selects one of: `unity_uloop`, `unity_t3`, `godot_cli`, `threejs_browser`, `command_suite`, or `engine_blocker`. Router output must include evidence paths and explicit unsupported reasons.
  **Must NOT do**: Do not let unsupported engines pass as `doc_only`.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 9, 10, 12 | Blocked By: 3

  **References**:
  - Existing verification router: `gateway/src/lux_verification.rs`
  - Ticket verification fields: `gateway/src/lux_ticket.rs`
  - Unity commands: `gateway/src/main.rs` `run_uloop_*`

  **Acceptance Criteria**:
  - [ ] RED tests prove unknown engine policy is unsupported.
  - [ ] GREEN tests prove each engine policy produces pass/fail/blocker evidence.
  - [ ] Unsupported engines write blocker tickets and evidence.

  **QA Scenarios**:
  ```text
  Scenario: Unity policy routes to uloop evidence when available
    Tool: bash
    Steps: run router unit test with mocked uloop command runner
    Expected: output contains compile/test/screenshot evidence paths.
    Evidence: evidence/game-harness-task-7-unity-router.txt

  Scenario: Missing Godot CLI creates blocker evidence
    Tool: bash
    Steps: run godot_cli policy with PATH stripped
    Expected: verification status is Unsupported and blocker JSON names missing CLI.
    Evidence: evidence/game-harness-task-7-godot-blocker.txt
  ```

  **Commit**: YES | Message: `feat(verify): route engine-specific QA` | Files: `gateway/src/lux_verification.rs`, tests

- [ ] 8. Spec-Loop CLI/API Flow

  **What to do**: Add or harden CLI/API commands for game spec-loop: start, answer, approve, reject, apply, status. All commands must use `.lux/specs` and append ledger events.
  **Must NOT do**: Do not require the dashboard to complete the flow.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 10, 11 | Blocked By: 4, 5, 6

  **References**:
  - Existing API routes: `gateway/src/server.rs` `/api/lux/spec-loop`
  - CLI spec command: `gateway/src/main.rs` `run_lux_spec_command`
  - Existing tests: `gateway/tests/gateway_cli_smoke.rs`

  **Acceptance Criteria**:
  - [ ] CLI smoke starts a spec-loop and prints run id.
  - [ ] CLI smoke answers a question and creates proposals.
  - [ ] Apply command modifies `.lux/specs/domains/*` and appends decisions.

  **QA Scenarios**:
  ```text
  Scenario: CLI Socratic round is complete
    Tool: bash
    Steps: lux spec-loop start; lux spec-loop answer; lux spec-loop approve; lux spec-loop apply
    Expected: status progresses and `.lux/specs/decisions.jsonl` has events.
    Evidence: evidence/game-harness-task-8-cli-round.txt

  Scenario: Empty answer is rejected
    Tool: bash
    Steps: lux spec-loop answer <id> <question-id> ""
    Expected: command exits nonzero with explicit empty-answer error.
    Evidence: evidence/game-harness-task-8-empty-answer.txt
  ```

  **Commit**: YES | Message: `feat(cli): expose game spec loop` | Files: `gateway/src/main.rs`, `gateway/src/server.rs`, tests

- [ ] 9. Engine Manual QA Evidence Adapters

  **What to do**: Implement evidence capture adapters for working engine surfaces. Unity must capture compile/test/dynamic-code/screenshot when uloop or bridge is available. Godot and Three.js must capture their real command/browser evidence if detected; otherwise write blocker evidence.
  **Must NOT do**: Do not invent screenshots or video files.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 10, 12 | Blocked By: 3, 7

  **References**:
  - Unity uloop command wrappers: `gateway/src/main.rs`
  - Bridge screenshot/dynamic code protocol: `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs`
  - Verification evidence writer: `gateway/src/lux_verification.rs`

  **Acceptance Criteria**:
  - [ ] Unity adapter can run mocked dynamic-code and screenshot commands in tests.
  - [ ] Three.js adapter can start a configured dev server and capture browser screenshot in smoke where project supports it.
  - [ ] Godot adapter produces explicit blocker when no Godot CLI is present.
  - [ ] Video capability is recorded as supported, unsupported, or blocker context without being required for pass.

  **QA Scenarios**:
  ```text
  Scenario: Unity manual QA evidence is captured
    Tool: bash
    Steps: run Unity adapter test with fake uloop binary returning JSON screenshot path
    Expected: evidence includes dynamic-code log and screenshot path.
    Evidence: evidence/game-harness-task-9-unity-manual-qa.txt

  Scenario: Missing screenshot capability is a blocker
    Tool: bash
    Steps: run engine adapter with screenshot_supported=false
    Expected: blocker evidence says screenshot unavailable and task is not marked complete.
    Evidence: evidence/game-harness-task-9-screenshot-blocker.json

  Scenario: Video capability is optional and explicit
    Tool: bash
    Steps: run engine adapter with video_supported=false
    Expected: evidence records video unsupported but does not fail a screenshot/log-only policy.
    Evidence: evidence/game-harness-task-9-video-capability.json
  ```

  **Commit**: YES | Message: `feat(verify): capture engine manual qa evidence` | Files: verification modules, tests

- [ ] 10. `lux run` Next-Goal Execution Contract

  **What to do**: Make `lux run` select the next goal from `.lux/specs` by this order: unresolved blocking contradiction, ambiguity above target, execution-grade ticket with satisfied dependencies, engine blocker resolution, then milestone task. Persist the selected current goal to `.lux/goals/current.json` and append a `goal_selected` event to `.lux/specs/decisions.jsonl`. A run may complete only after verification evidence is accepted.
  **Must NOT do**: Do not mark `AwaitingEvidence` as complete.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 11, 12 | Blocked By: 5, 6, 7, 8, 9

  **References**:
  - Current run lifecycle: `gateway/src/lux_run.rs`
  - Task DAG: `gateway/src/lux_task_dag.rs`
  - Run state: `gateway/src/lux_run_state.rs`
  - Ticket execution grade checks: `gateway/src/lux_ticket.rs`

  **Acceptance Criteria**:
  - [ ] RED test proves current run can project a task without engine QA evidence.
  - [ ] GREEN test proves run state remains `AwaitingEvidence` until evidence exists.
  - [ ] `lux run --dry-run` writes next-goal plan with source spec refs.
  - [ ] `lux run --dry-run` writes `.lux/goals/current.json` with goal id, source spec refs, selected engine, and rationale.
  - [ ] `lux run` writes evidence acceptance or blocker under `.lux/evidence`.

  **QA Scenarios**:
  ```text
  Scenario: Dry run chooses next goal from spec
    Tool: bash
    Steps: lux run --project-path <tmp> --dry-run "make next playable step"
    Expected: `.lux/runs/<id>/plan.json` references `.lux/specs` and selected next goal.
    Evidence: evidence/game-harness-task-10-dry-run.txt

  Scenario: Current goal is persisted
    Tool: bash
    Steps: run `lux run --dry-run` and inspect `.lux/goals/current.json` plus `.lux/specs/decisions.jsonl`
    Expected: current goal JSON exists and decision ledger has matching `goal_selected` event.
    Evidence: evidence/game-harness-task-10-current-goal.txt

  Scenario: Real run does not complete without evidence
    Tool: bash
    Steps: create ticket requiring engine QA but no engine capability; run lux run
    Expected: run status is Blocked or AwaitingEvidence with blocker evidence, not Completed.
    Evidence: evidence/game-harness-task-10-no-false-complete.txt
  ```

  **Commit**: YES | Message: `feat(run): pursue spec-backed game goals` | Files: `gateway/src/lux_run.rs`, `gateway/src/lux_task_dag.rs`, tests

- [ ] 11. Dashboard and Docs Projection

  **What to do**: Update dashboard/API projections and docs so users see `.lux/specs` as GDD SSoT, current ambiguity, decisions, engine capability, next goal, and evidence status. Keep docs clear that all engines are supported by capability routing, not equal verification maturity.
  **Must NOT do**: Do not build a marketing landing page or overstate autonomy.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 13 | Blocked By: 8, 10

  **References**:
  - Server spec progress: `gateway/src/server.rs`
  - UI source: `gateway/ui-src`
  - README current roadmap/spec sections: `README.md`
  - Usage docs: `docs/usage.md`

  **Acceptance Criteria**:
  - [ ] API returns spec path, ambiguity, decision count, engine capability summary, and current next goal.
  - [ ] UI typecheck passes.
  - [ ] Docs distinguish `.lux` SSoT from repo docs projections.

  **QA Scenarios**:
  ```text
  Scenario: Dashboard API projects game harness state
    Tool: curl
    Steps: start server against temp `.lux/specs`; curl relevant status endpoint
    Expected: JSON includes spec path, ambiguity, decisions, engine capabilities, next goal.
    Evidence: evidence/game-harness-task-11-api-status.json

  Scenario: UI remains type-safe
    Tool: bash
    Steps: cd gateway/ui-src && npx tsc --noEmit
    Expected: exit 0.
    Evidence: evidence/game-harness-task-11-tsc.txt
  ```

  **Commit**: YES | Message: `docs(ui): project game harness state` | Files: `gateway/src/server.rs`, `gateway/ui-src`, docs

- [ ] 12. End-to-End Temp Project Harness

  **What to do**: Build an automated temp project scenario that runs the whole loop: init `.lux/specs`, Socratic answer/apply, create next-goal ticket, run `lux run --dry-run`, run engine verification route, and capture evidence/blocker.
  **Must NOT do**: Do not depend on the user's personal Unity project.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 13, 14 | Blocked By: 7, 9, 10

  **References**:
  - CLI smoke tests: `gateway/tests/gateway_cli_smoke.rs`
  - Existing temp project helpers in tests
  - Engine capability router from Task 3

  **Acceptance Criteria**:
  - [ ] Smoke test runs with no real engine and records explicit engine blockers.
  - [ ] Smoke test runs with fake Unity uloop and records compile/test/screenshot evidence.
  - [ ] Run plan references `.lux/specs`, not stale `.lux/domains`.

  **QA Scenarios**:
  ```text
  Scenario: No-engine project produces blocker evidence
    Tool: bash
    Steps: cd gateway && cargo test --test gateway_cli_smoke game_harness_no_engine_blocks
    Expected: test passes and evidence contains unsupported engine blocker.
    Evidence: evidence/game-harness-task-12-no-engine.txt

  Scenario: Fake Unity surface produces manual QA evidence
    Tool: bash
    Steps: cd gateway && cargo test --test gateway_cli_smoke game_harness_fake_unity_evidence
    Expected: fake uloop compile/test/screenshot evidence is recorded and run remains evidence-gated.
    Evidence: evidence/game-harness-task-12-fake-unity.txt
  ```

  **Commit**: YES | Message: `test: add game harness e2e smoke` | Files: `gateway/tests/gateway_cli_smoke.rs`, fixtures

- [ ] 15. Game Context Adapter Contract

  **What to do**: Define the first-class observation contract that AI tools receive before acting on a game project. The contract must cover scene hierarchy, selected object identity, component/property snapshots, Transform/RectTransform/Collider data, camera and UI coordinate state, console/compile logs, PlayMode/input trace, screenshot references, and optional vision annotations. It must state how each observation links back to `.lux/specs`, tickets, run evidence, and engine capability status.
  **Must NOT do**: Do not make pixel-only vision feedback a completion gate without engine context. Do not introduce remote streaming, browser-based Unity control, or unverified Godot/Three.js parity.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 7, 9, 10, 12, 13 | Blocked By: 1, 2, 3

  **References**:
  - Unity context APIs: `gateway/src/uloop_runner.rs`, `gateway/src/bridge_types.rs`, `gateway/src/capture.rs`
  - Existing context command docs: `README.md`, `docs/usage.md`, `Skills/skills/lux-unity/SKILL.md`
  - Spec SSoT: `.lux/specs` contract from Tasks 2 and 4

  **Acceptance Criteria**:
  - [ ] Observation schema names every required context field and its evidence source.
  - [ ] Screenshot/vision evidence is represented as a supplement to text/JSON state, not a replacement.
  - [ ] Coordinate and UI/camera state include enough data for an AI agent to connect visual symptoms back to GameObjects or blocker evidence.
  - [ ] Unsupported engine observations produce explicit capability blockers.

  **QA Scenarios**:
  ```text
  Scenario: Context schema is text/JSON first
    Tool: bash
    Steps: rg -n "Game Context Adapter|scene hierarchy|RectTransform|coordinate|vision" README.md docs Skills gateway
    Expected: docs and/or schema describe text/JSON observations before screenshot/vision evidence.
    Evidence: evidence/game-harness-task-15-context-contract.txt

  Scenario: Vision is not standalone completion evidence
    Tool: bash
    Steps: rg -n "vision.*supplement|not.*vision.*only|pixel-only" README.md docs Skills gateway
    Expected: guardrail exists in public docs or skill contract.
    Evidence: evidence/game-harness-task-15-vision-guardrail.txt
  ```

  **Commit**: YES | Message: `docs: lock game context adapter contract` | Files: README/docs/skills and later schema tests

- [ ] 13. Release Docs and Guardrails

  **What to do**: Update README, usage docs, roadmap reality lock, and skill docs to state the new game harness contract: `.lux/specs` GDD SSoT, Socratic ambiguity, engine capability routing, evidence-gated `lux run`, and explicit engine maturity.
  **Must NOT do**: Do not claim completed full autonomous game development.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 14 | Blocked By: 11, 12, 15

  **References**:
  - `README.md`
  - `docs/usage.md`
  - `docs/roadmap-reality-lock.md`
  - `Skills/skills/game-dev/SKILL.md`
  - `Skills/docs/templates/commands/lux-run.md`

  **Acceptance Criteria**:
  - [ ] Docs say `.lux/specs` is canonical under `.lux`.
  - [ ] Docs list engine-specific manual QA surfaces and blocker behavior.
  - [ ] Stale docs do not say `.lux/spec.json` alone is the game SSoT.

  **QA Scenarios**:
  ```text
  Scenario: Docs reflect game harness contract
    Tool: bash
    Steps: rg -n '.lux/specs|GDD|Socratic|engine capability|evidence-gated|uloop' README.md docs Skills
    Expected: active docs describe the contract and engine-gated QA.
    Evidence: evidence/game-harness-task-13-docs.txt

  Scenario: No stale SSoT claim remains
    Tool: bash
    Steps: rg -n '.lux/spec.json.*single source|domains/.*canonical|all engines verified' README.md docs Skills
    Expected: no stale or overstated active claims.
    Evidence: evidence/game-harness-task-13-stale-docs.txt
  ```

  **Commit**: YES | Message: `docs: define lux game harness contract` | Files: docs and skill docs

- [ ] 14. Baseline Verification Report

  **What to do**: Run the final command matrix and write a concise evidence-backed report that separates implemented, verified, capability-gated, and planned surfaces.
  **Must NOT do**: Do not hide failing checks.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: Final Verification | Blocked By: 12, 13

  **References**:
  - Existing evidence pattern: `evidence/task-8-baseline-verification.txt`
  - Project structure check: `scripts/check-project-structure.sh`
  - Skill validator: `Skills/tools/validate-skills.sh`

  **Acceptance Criteria**:
  - [ ] Rust build and tests pass or any failure is explained with blocker evidence.
  - [ ] TypeScript typecheck passes.
  - [ ] Skill validator passes.
  - [ ] Report names actual working engine manual QA surface.

  **QA Scenarios**:
  ```text
  Scenario: Full baseline is evidence-backed
    Tool: bash
    Steps: run cargo build, cargo test, tsc, skill validation, structure check
    Expected: all command exits recorded.
    Evidence: evidence/game-harness-task-14-baseline.txt

  Scenario: Manual engine QA evidence is present
    Tool: bash
    Steps: find `.lux/evidence` and evidence/ for screenshots/logs/blockers
    Expected: at least one real engine or explicit blocker evidence exists.
    Evidence: evidence/game-harness-task-14-manual-evidence.txt
  ```

  **Commit**: YES | Message: `test: record game harness verification baseline` | Files: evidence and docs

## Final Verification Wave
> ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
- [ ] F1. Plan Compliance Audit: verify Tasks 1-15 acceptance criteria have evidence and no task is skipped.
- [ ] F2. Code Quality Review: inspect SSoT migration, no silent fallback, run-state evidence gating, and engine router strictness.
- [ ] F3. Real Manual QA: run one full CLI flow on a temp project and one working engine-specific manual QA surface or explicit blocker flow.
- [ ] F4. Scope Fidelity Check: confirm this milestone did not claim full M1-M6 autonomy or fake all-engine parity.

## Momus Review
- Local Momus-style review performed because no subagent spawn tool is exposed in this Codex session.
- Findings addressed:
  - Wave dependency conflict fixed by changing from 4 waves to 5 waves.
  - Fixed-domain overreach corrected by adding LLM-approved custom domains with ledger provenance.
  - Optional video evidence made explicit instead of implied as universal.
  - `lux run` next-goal persistence added via `.lux/goals/current.json` plus `goal_selected` ledger events.

## Commit Strategy
Recommended commits:
1. `feat(spec): establish game specs ssot`
2. `feat(engine): record game engine capabilities`
3. `feat(spec): add game domain schema`
4. `feat(spec): persist decision ledger`
5. `feat(spec): drive game socratic clarity`
6. `feat(verify): route engine-specific QA`
7. `feat(cli): expose game spec loop`
8. `feat(verify): capture engine manual qa evidence`
9. `feat(run): pursue spec-backed game goals`
10. `docs: lock game context adapter contract`
11. `docs: define lux game harness contract`
12. `test: record game harness verification baseline`

Do not commit automatically unless explicitly requested.

## Success Criteria
- `.lux/specs` is the canonical GDD/spec sub-root under `.lux`.
- User answers, decisions, contradictions, and inferred preferences are append-only and inspectable.
- Ambiguity-driven Socratic rounds reduce uncertainty before implementation.
- `lux run` chooses and pursues the next game goal from `.lux/specs`.
- TDD and manual engine QA are required before execution completion.
- Unity, Godot, and Three.js are capability-routed; unsupported surfaces generate blocker evidence.
- Active docs describe the game harness accurately without claiming unverified autonomy.
