# LUX Core Package Split and Game Verification Ontology Worktree Plan

## TL;DR
> **Summary**: Split LUX into Rust core package layers first, then implement the game verification ontology suite as composable contracts instead of adding another heavy path inside `gateway`. This is a checked, high-density execution plan for parallel git worktrees; no implementation is complete or implied by this document.
> **Deliverables**:
> - A worktree-safe core crate split plan with one-way dependency ownership.
> - A game verification ontology contract that defines scene, coordinate, camera/UI, visual match, evidence, blocker, and completion semantics.
> - A scene AST and coordinate mapping contract that makes screenshot/vision evidence link back to engine facts.
> - An evidence/TDD harness plan that forbids pixel-only completion and fake Godot/Three.js parity.
> - AI context injection and dashboard projection tasks that remain read-only over canonical `.lux` state.
> **Effort**: XL
> **Parallel**: YES - full parallel mode, 12 worktrees, setup fanout plus 5 dependency-safe waves
> **Critical Path**: Task 2 -> Task 3 -> Task 4 -> Task 9 -> Task 12 -> Task 14 -> Task 17 -> Final Verification

## Context
### Original Request
The user asked to check `plans/lux-game-verification-ontology-suite-split.md` and increase its detail density into a parallel work plan using worktrees.

### Repo Findings
- Root `Cargo.toml` currently has one workspace member: `gateway`.
- `gateway/Cargo.toml` package name is `lux`; the plan must not casually rename the shipped CLI package.
- `gateway/src/lib.rs` exports many unrelated modules: spec, run, ticket, verification, bridge/protocol, project detection, hooks, events, server, and terminal surfaces.
- `gateway/src/server.rs` is the Axum route owner and includes remote/WebRTC routes that remain out of scope.
- `bridge/unity/AiBridgeEditor/Ast/` already contains Unity AST readers.
- `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs` already exposes `read_asset_ast`, `get_selection_ast`, and `get_scene_ast`.
- `Skills/skills/game-dev/SKILL.md` already states context-first, vision-supplemented verification.
- `docs/roadmap-reality-lock.md` marks the Game Context Adapter as a critical gap.
- `plans/lux-game-harness-overhaul.md` is a larger implementation plan; this plan is the prerequisite split and contract plan.

### Metis Review Incorporated
- Fixed duplicated task numbering.
- Canonical ontology/schema now belongs in `lux-verification-core`; `gateway/src` may only contain migration shims or API wiring.
- Added a worktree ownership matrix for shared collision files such as root `Cargo.toml`, `gateway/Cargo.toml`, `gateway/src/lib.rs`, `gateway/src/main.rs`, and docs.
- Added dependency order and merge order.
- Added `.lux` schema versioning and bridge protocol compatibility constraints.
- Replaced vague "later tests" with concrete task-level RED/GREEN and QA command expectations.
- Added a measurable "contract stability" definition.

## Work Objectives
### Core Objective
Create a decision-complete implementation plan for splitting LUX into core packages and then layering game verification ontology, scene AST, coordinate/camera/UI mapping, vision-to-AST matching, evidence gates, AI prompt injection, and dashboard projection on top.

### Deliverables
- Rust workspace split design and execution order.
- Worktree branch names, ownership boundaries, integration files, merge order, and rebase rules.
- Game verification ontology and schema boundaries.
- Unity AST and coordinate mapping contracts.
- Vision-to-AST matching contract.
- Evidence/TDD harness contract.
- AI context injection contract.
- UX/dashboard projection contract.
- Verification command matrix for each worktree and final integration.

### Definition of Done
- `plans/lux-game-verification-ontology-suite-split.md` contains no duplicated task IDs.
- Every task has references, acceptance criteria, QA scenarios, commit message, files, worktree owner, and dependency links.
- Worktree ownership explicitly prevents concurrent edits to shared files.
- The plan keeps WebRTC, remote Unity browser control, fake Godot/Three.js parity, and pixel-only completion evidence out of scope.
- The plan states that this session changes only a plan artifact and does not claim implementation completion.
- Planning-session verification is docs/search only.

### Must Have
- `.lux/` remains runtime SSoT.
- Gateway remains execution owner and side-effect shell.
- Core crates own stable, non-UI, non-server, non-CLI contracts.
- `gateway` package name remains `lux` until a separate explicit rename plan exists.
- Core crate dependency direction is one-way: `gateway/lux -> core crates`; core crates never depend on `gateway`.
- No canonical game verification schema lives only in `gateway/src`.
- All new runtime state must carry schema version and migration behavior.
- Unity is the first executable engine target; Godot/Three.js are capability-gated.

### Must NOT Have
- No WebRTC scope.
- No remote Unity browser control.
- No remote video streaming as verification dependency.
- No fake Godot/Three.js parity.
- No pixel-only completion evidence.
- No silent fallback to empty AST, empty coordinate map, empty evidence, or default success.
- No dashboard or adapter direct writes to canonical verification state.
- No broad implementation in this planning session.

## Core Package Target
| Package / crate | Owns | Must not own | Migrates from existing code |
| --- | --- | --- | --- |
| `lux-core` | Shared IDs, path wrappers, timestamps, atomic `.lux` IO helpers, redaction-safe primitives. | Axum, clap, engine process execution, network transport. | `gateway/src/lux_io.rs`, pure helpers from `cross_platform.rs`, shared primitives from `lux_events.rs`. |
| `lux-project` | Project detection and engine capability records for Unity/Godot/Three.js. | Running Unity/Godot/Node commands or serving HTTP. | `gateway/src/project.rs`, `gateway/src/project_godot.rs`, pure capability DTOs from `unity_hub.rs`. |
| `lux-spec-core` | `.lux/specs`, spec/domain/decision/preference models, ambiguity model, migration contracts. | Prompt text, Axum handlers, terminal execution. | `gateway/src/lux_spec.rs`, `lux_specs.rs`, `lux_spec_loop.rs`, `lux_ambiguity.rs`. |
| `lux-run-core` | Run state, tickets, task DAG, goals, evidence references, continuation state. | Engine execution side effects, git push, server routing. | `gateway/src/lux_run_state.rs`, `lux_ticket.rs`, `lux_task_dag.rs`, `lux_continuation_state.rs`, pure structs from `lux_run.rs`. |
| `lux-bridge-core` | Bridge protocol DTOs, command/result schemas, engine capability/blocker payloads. | TCP transport, Unity launch, uloop process execution. | `gateway/src/protocol.rs`, `bridge_types.rs`, pure DTOs from `lux_unity_maneuver.rs` and `uloop_*`. |
| `lux-verification-core` | Verification ontology, evidence classes, completion gates, scene AST schema, coordinate mapping schema, visual-match schema. | Screenshot capture, vision provider calls, dashboard rendering. | `gateway/src/lux_verification.rs`, pure `visual_regression.rs` DTOs, future ontology modules. |
| `lux-ai-core` | AI event/session/log models, prompt/context payload contracts, skill metadata contracts. | Spawning Codex/OpenCode/Claude, OpenCode plugin runtime, server hooks. | `gateway/src/ai_log.rs`, `lux_ai_session.rs`, pure prompt/context pieces from `lux_hooks.rs`, `skill_adapter/metadata.rs`. |
| `gateway` package `lux` | CLI, Axum HTTP/WS server, process/filesystem side effects, dashboard serving, adapter installation, engine command execution. | Canonical domain model definitions once moved to core crates. | Existing `gateway` crate after extractions. |

Dependency rule:

```text
gateway/lux
  -> lux-ai-core
  -> lux-verification-core
  -> lux-run-core
  -> lux-spec-core
  -> lux-project
  -> lux-bridge-core
  -> lux-core
```

Allowed cross edges:

- `lux-run-core -> lux-spec-core` for spec-derived tasks.
- `lux-verification-core -> lux-run-core` for ticket/evidence references.
- `lux-verification-core -> lux-project` for engine capability decisions.
- `lux-ai-core -> lux-verification-core` for prompt evidence requirements.

Forbidden edges:

- Any core crate -> `gateway`.
- `lux-core` -> any higher crate.
- `lux-project` -> process runners.
- `lux-verification-core` -> screenshot capture implementation.
- `lux-ai-core` -> OpenCode runtime APIs.

## Game Verification Layer Boundaries
| Layer | Owns | Does not own | Canonical home |
| --- | --- | --- | --- |
| Game Verification Ontology | Terms for scene, stage, actor, component, transform, camera, viewport, world/screen/UI coordinates, expected visual state, evidence class, blocker class, completion gate. | Engine execution, prompt runtime, dashboard rendering. | `lux-verification-core` docs/schema, `docs/adr/`, skill docs. |
| Scene AST Extraction | Engine-native hierarchy and object/component/property extraction. Unity first from existing bridge AST readers. | Vision interpretation, prompt injection, dashboard status. | `lux-bridge-core` DTOs plus Unity C# bridge AST files. |
| Coordinate/Camera/UI Mapping | World/local/screen/viewport/UI/input coordinate frames, camera projection, Canvas/RectTransform/anchor data, screenshot annotation frames. | Object ownership or OCR. | `lux-verification-core` schema plus Unity bridge DTOs. |
| Vision-to-AST Matching | Visual observation candidates linked to AST node IDs, component names, regions, and blocker reasons. | Completion from pixels alone. | `lux-verification-core`. |
| Evidence/TDD Harness | Automated test evidence, command evidence, engine manual QA evidence, screenshot references, blocker records, acceptance gates. | UX layout or autonomous scheduling. | `lux-verification-core` plus gateway execution wiring. |
| AI Prompt/Context Injection | Ontology, AST summary, coordinate map, visual mismatch, evidence requirements, and blockers for AI sessions. | Canonical `.lux` writes. | `lux-ai-core` payloads plus OpenCode adapter templates. |
| UX/Dashboard | Read-only projection of layer readiness, AST snapshot status, coordinate map status, visual match status, accepted/blocker evidence. | Verification authority. | Gateway API projection and React dashboard. |

## Worktree Operating Model
### Baseline Rule
Before creating any worktree, the integrator records current dirty state and never reverts user changes. Existing uncommitted files currently include docs/plan/skill changes; every worker must rebase onto the integrator branch and avoid unrelated edits.

### Worktree Naming
Use a shared parent outside the repo:

```bash
mkdir -p ../lux-worktrees
git worktree add ../lux-worktrees/wt-01-workspace -b plan/core-workspace-split
git worktree add ../lux-worktrees/wt-02-core -b plan/lux-core
git worktree add ../lux-worktrees/wt-03-project -b plan/lux-project
git worktree add ../lux-worktrees/wt-04-spec-run -b plan/spec-run-core
git worktree add ../lux-worktrees/wt-05-bridge-ast -b plan/bridge-ast-contract
git worktree add ../lux-worktrees/wt-06-verification -b plan/verification-ontology
git worktree add ../lux-worktrees/wt-07-ai-context -b plan/ai-context-injection
git worktree add ../lux-worktrees/wt-08-dashboard -b plan/dashboard-projection
git worktree add ../lux-worktrees/wt-09-docs-skills -b plan/docs-skills-contracts
git worktree add ../lux-worktrees/wt-10-integration -b plan/integration-gate
git worktree add ../lux-worktrees/wt-11-qa-ledger -b plan/qa-ledger
git worktree add ../lux-worktrees/wt-12-protocol-review -b plan/protocol-review
```

### Shared File Ownership
| Shared file | Exclusive owner | Other worktrees may edit? | Rule |
| --- | --- | --- | --- |
| Root `Cargo.toml` | WT-01 | No | All new crates registered only by WT-01. |
| `gateway/Cargo.toml` | WT-01 | No | Dependency additions flow through WT-01 integration patches. |
| `gateway/src/lib.rs` | WT-01 | No | Core module re-export changes only here. |
| `gateway/src/main.rs` | WT-10 | No | CLI wiring only after merged core APIs. |
| `gateway/src/server.rs` | WT-10 | No | API wiring only after merged core APIs. |
| `docs/adr/*` | WT-09 | Yes, with coordination | WT-09 owns final docs language. |
| `README.md`, `docs/usage.md`, `docs/roadmap-reality-lock.md` | WT-09 | No | Other worktrees add notes to evidence files, not docs. |
| `Skills/skills/*` | WT-09 | No | Skill wording centralized. |
| `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs` | WT-05 | No | Protocol compatibility centralized. |
| `gateway/ui-src/src/*` | WT-08 | No | UI projection centralized. |
| `adapters/opencode/*`, `gateway/src/templates/plugin/*` | WT-07 | No | Prompt injection centralized. |
| `evidence/worktree-*` | Owning WT | Yes, own subdir only | Each worker writes only its own evidence folder. |

### Full Parallel Dispatch Contract
Use this mode when the conductor wants maximum parallelism.

1. Create all 12 worktrees at once.
2. Dispatch read-only or design-only lanes immediately: WT-09, WT-11, WT-12.
3. Dispatch WT-01 workspace bootstrap immediately after Task 2 has enough ADR direction; do not wait for all read-only lanes to finish.
4. As soon as WT-01 merges, dispatch WT-02, WT-03, WT-04, WT-05, WT-06, and WT-07 in the same batch. They must not edit shared files; they submit integration notes for WT-10.
5. Dispatch WT-08 after WT-06 publishes stable payload names; WT-08 can begin UI type/interface prep earlier in read-only mode.
6. WT-10 runs continuously as integration conductor, applying only shared-file patches after each source worktree passes local verification.
7. WT-11 runs QA ledger/index checks in parallel with every wave and never changes source.
8. WT-12 reviews bridge protocol compatibility in parallel with WT-05 and reports blockers before WT-10 integration.

This is "full parallel" because all independent work starts as soon as its named dependency is satisfied. No task waits for another task merely because it is in the same subsystem.

### Merge Order
1. WT-09 ADR can merge first if it only documents current architecture.
2. WT-01 workspace bootstrap merges before Rust source extraction.
3. WT-02, WT-03, WT-04, WT-05, WT-06, and WT-07 run in parallel after WT-01.
4. WT-12 protocol review merges before WT-05 protocol-affecting changes are accepted.
5. WT-08 merges after WT-06 exposes stable payload names and WT-10 exposes or stubs read-only API projection.
6. WT-11 evidence ledger updates can merge after each task batch.
7. WT-10 integrates CLI/server/shared-file wiring and final verification last.

### Conflict Rules
- No worker edits another worktree's exclusive files.
- If a task needs a shared file, it writes an integration note under `evidence/worktree-notes/task-{N}.md`; WT-10 applies the shared-file change.
- Every worktree rebases on the integration branch before opening a PR/merge request.
- Every worktree records `git diff --name-only` in evidence before handoff.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all implementation verification is agent-executed when this plan is executed later.

- This planning session: docs/search checks only. No Rust/TypeScript/Unity/skill suites because only this markdown plan changes.
- Implementation later: TDD RED-GREEN-REFACTOR for every executable change.
- Rust crate split: targeted crate tests first, then `cd gateway && cargo test <target>`, then `cargo test --workspace`.
- Unity bridge: Unity EditMode tests for DTO/protocol changes.
- TypeScript plugin: `cd gateway/src/templates/plugin && npm test` or existing package test command.
- Dashboard: `cd gateway/ui-src && npx tsc --noEmit` plus focused hook/component tests.
- Skills: `bash Skills/tools/validate-skills.sh` after skill edits.
- Manual QA: only through supported surfaces; unsupported engine paths produce blocker evidence.
- Evidence path pattern: `evidence/worktree-{WT}/task-{N}-{slug}.{txt,json,md,png}`.

## Execution Strategy
### Parallel Execution Waves
Full parallel mode uses dispatch batches, not slow sequential phases:

Wave 0 - Setup and Read-Only Fanout: Task 1, Task 2, Task 19, Task 20
Wave 1 - Workspace Bootstrap: Task 3
Wave 2 - Core Extraction Fanout: Task 4, Task 5, Task 6, Task 7, Task 9
Wave 3 - Engine and Context Fanout: Task 8, Task 10, Task 11, Task 13
Wave 4 - Evidence Gate: Task 12
Wave 5 - Serial Integration Tail: Task 14 -> Task 15 -> Task 16 -> Task 17 -> Task 18

Wave 1 is intentionally narrow because `Cargo.toml`, `gateway/Cargo.toml`, and `gateway/src/lib.rs` are exclusive shared files. Every other wave is widened to the maximum dependency-safe set.

### Dependency Matrix
| Task | Worktree | Depends on | Blocks |
| --- | --- | --- | --- |
| 1. Contract inventory | WT-09 | none | 3, 6, 9 |
| 2. Workspace split ADR | WT-09 | 1 | 3, 4, 5 |
| 3. Workspace bootstrap | WT-01 | 2 | 4, 5, 6, 7, 8 |
| 4. `lux-core` extraction | WT-02 | 3 | integration |
| 5. `lux-project` extraction | WT-03 | 3 | 8, 11 |
| 6. `lux-spec-core` and `lux-run-core` contracts | WT-04 | 3 | 12 |
| 7. `lux-bridge-core` DTO boundary | WT-05 | 3 | 10 |
| 8. Engine capability blocker model | WT-03 | 5 | 11, 12 |
| 9. Verification ontology schema | WT-06 | 1, 3 | 10, 12, 13 |
| 10. Unity AST and coordinate mapping contract | WT-05 | 7, 9 | 12 |
| 11. Godot/Three.js capability guardrails | WT-03 | 8 | 12, 15 |
| 12. Evidence gate router | WT-06 | 8, 9, 10, 11 | 13, 14, 15 |
| 13. AI context injection payloads | WT-07 | 9 | 15 |
| 14. Gateway API/CLI integration | WT-10 | 12 | 15, 16, 17 |
| 15. Dashboard projection | WT-08 | 12, 13, 14 | 16, 17 |
| 16. Docs/skills final projection | WT-09 | 12, 14, 15 | 17 |
| 17. Cross-worktree integration gate | WT-10 | 14, 15, 16, 19, 20 | 18 |
| 18. Release-ready verification report | WT-10 | 17 | Final Verification |
| 19. QA ledger and evidence index scaffold | WT-11 | none | 17, 18 |
| 20. Bridge protocol compatibility review | WT-12 | none | 10, 17 |

## TODOs
- [ ] 1. Contract Inventory and Ownership Map

  **Worktree**: WT-09 `../lux-worktrees/wt-09-docs-skills`

  **What to do**: Build an inventory table before implementation. Classify current modules, docs, skills, bridge files, plugin templates, and dashboard files by target layer and exclusive worktree owner.

  **Must NOT do**: Do not edit Rust, C#, TypeScript, or dashboard source. Do not change root or gateway Cargo files.

  **Parallelization**: Can Parallel: YES | Wave 0 | Blocks: 3, 6, 9 | Blocked By: none

  **References**:
  - Root workspace: `Cargo.toml`
  - Gateway crate: `gateway/Cargo.toml`
  - Current exports: `gateway/src/lib.rs`
  - Game context docs: `README.md`, `docs/roadmap-reality-lock.md`
  - Existing AST bridge: `bridge/unity/AiBridgeEditor/Ast/*`

  **Acceptance Criteria**:
  - [ ] `evidence/worktree-09/task-1-inventory.md` lists every target package and layer owner.
  - [ ] Inventory labels each file as `exclusive`, `integration-owned`, or `read-only-reference`.
  - [ ] Inventory names all shared files that only WT-01 or WT-10 may edit.
  - [ ] `git diff --name-only` shows only evidence/docs/plan files.

  **QA Scenarios**:
  ```text
  Scenario: Inventory covers all target surfaces
    Tool: bash
    Steps: rg -n "lux_spec|lux_run|lux_ticket|lux_verification|CommandGetSceneAst|prompt-builder|VisualReportPanel|useWebRTC" gateway/src bridge/unity gateway/ui-src/src gateway/src/templates/plugin > evidence/worktree-09/task-1-surface-scan.txt
    Expected: evidence names gateway, bridge, plugin, dashboard, and WebRTC out-of-scope surfaces.
    Evidence: evidence/worktree-09/task-1-surface-scan.txt

  Scenario: No source mutation during inventory
    Tool: bash
    Steps: git diff --name-only
    Expected: output contains no Rust, C#, TS, or TSX source files.
    Evidence: evidence/worktree-09/task-1-diff-scope.txt
  ```

  **Commit**: YES | Message: `docs(plan): inventory lux split ownership` | Files: `evidence/worktree-09/*`, docs/plan artifact only

- [ ] 2. Workspace Split ADR

  **Worktree**: WT-09 `../lux-worktrees/wt-09-docs-skills`

  **What to do**: Add an ADR that locks the Rust core package split, dependency direction, package naming rule, and purity rule. Purity means no Axum, clap, network transport, process spawning, environment reads for behavior, or direct engine execution inside core crates. Core crates may perform typed `.lux` path calculations and atomic IO only where explicitly owned.

  **Must NOT do**: Do not rename the `gateway` package from `lux`.

  **Parallelization**: Can Parallel: YES | Wave 0 | Blocks: 3, 4, 5 | Blocked By: 1

  **References**:
  - Existing execution owner ADR: `docs/adr/ADR-001-Gateway-as-Execution-Owner.md`
  - Run-state SSoT ADR: `docs/adr/ADR-002-Active-Run-State-SSoT.md`
  - Domain separation ADR: `docs/adr/ADR-003-Domain-File-Separation.md`
  - Unity MCP loop ADR: `docs/adr/ADR-004-Unity-Game-Dev-MCP-Loop.md`

  **Acceptance Criteria**:
  - [ ] New ADR defines `lux-core`, `lux-project`, `lux-spec-core`, `lux-run-core`, `lux-bridge-core`, `lux-verification-core`, `lux-ai-core`, and gateway shell.
  - [ ] ADR states package `gateway/Cargo.toml` remains `name = "lux"` until a separate rename plan exists.
  - [ ] ADR includes allowed and forbidden dependency edges.
  - [ ] ADR defines the shared-file ownership rule from this plan.

  **QA Scenarios**:
  ```text
  Scenario: ADR locks package split and dependency direction
    Tool: bash
    Steps: rg -n "lux-core|lux-project|lux-spec-core|lux-run-core|lux-bridge-core|lux-verification-core|lux-ai-core|name = \"lux\"|forbidden" docs/adr
    Expected: all package names and package-name guardrail are present.
    Evidence: evidence/worktree-09/task-2-adr-scan.txt

  Scenario: ADR does not authorize WebRTC or remote Unity browser control
    Tool: bash
    Steps: '! rg -n "remote Unity browser control|WebRTC.*verification dependency|video streaming.*required" docs/adr'
    Expected: command exits 0.
    Evidence: evidence/worktree-09/task-2-guardrail-scan.txt
  ```

  **Commit**: YES | Message: `docs(adr): define lux core package boundaries` | Files: `docs/adr/ADR-005-Core-Package-Layer-Split.md`, evidence

- [ ] 3. Workspace Bootstrap

  **Worktree**: WT-01 `../lux-worktrees/wt-01-workspace`

  **What to do**: Add empty/minimal core crates to the workspace with compile-only placeholder modules and re-export strategy. Register crates in root `Cargo.toml`, add dependencies in `gateway/Cargo.toml`, and keep `gateway` package name as `lux`.

  **Must NOT do**: Do not move domain logic yet. Do not edit `gateway/src/main.rs` or `gateway/src/server.rs`.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: 4, 5, 6, 7, 8 | Blocked By: 2

  **References**:
  - Current workspace: `Cargo.toml`
  - Current gateway manifest: `gateway/Cargo.toml`
  - Current re-exports: `gateway/src/lib.rs`

  **Acceptance Criteria**:
  - [ ] Root workspace members include `crates/lux-core`, `crates/lux-project`, `crates/lux-spec-core`, `crates/lux-run-core`, `crates/lux-bridge-core`, `crates/lux-verification-core`, and `crates/lux-ai-core`.
  - [ ] `gateway/Cargo.toml` still has `name = "lux"`.
  - [ ] `cargo metadata --format-version=1 --no-deps` lists all new crates.
  - [ ] `cargo check --workspace` passes.

  **QA Scenarios**:
  ```text
  Scenario: Workspace metadata sees every core crate
    Tool: bash
    Steps: cargo metadata --format-version=1 --no-deps | tee evidence/worktree-01/task-3-metadata.json
    Expected: JSON contains all seven core package names and package name "lux".
    Evidence: evidence/worktree-01/task-3-metadata.json

  Scenario: Empty workspace compiles before logic moves
    Tool: bash
    Steps: cargo check --workspace 2>&1 | tee evidence/worktree-01/task-3-cargo-check.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-01/task-3-cargo-check.txt
  ```

  **Commit**: YES | Message: `build(workspace): add lux core crate skeletons` | Files: `Cargo.toml`, `gateway/Cargo.toml`, `crates/*`

- [ ] 4. `lux-core` Extraction

  **Worktree**: WT-02 `../lux-worktrees/wt-02-core`

  **What to do**: Move shared primitives into `lux-core`: atomic write helpers, path display helpers, timestamps, redaction-safe IDs, and simple `.lux` path constructors. Keep compatibility re-exports in `gateway` until all dependents move.

  **Must NOT do**: Do not move Axum, clap, network, process, Unity, or project detection logic.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: integration | Blocked By: 3

  **References**:
  - `gateway/src/lux_io.rs`
  - `gateway/src/cross_platform.rs`
  - `gateway/src/ai_log.rs` redaction helpers as reference only
  - `gateway/tests/redact_smoke.rs`, `gateway/tests/redact_patterns_smoke.rs`

  **Acceptance Criteria**:
  - [ ] RED test first proves a selected helper is unavailable from `lux_core`.
  - [ ] GREEN test imports and uses the helper from `lux_core`.
  - [ ] Gateway callers still compile through compatibility imports.
  - [ ] `cargo test -p lux-core` passes.

  **QA Scenarios**:
  ```text
  Scenario: lux-core exposes atomic/path primitives
    Tool: bash
    Steps: cargo test -p lux-core 2>&1 | tee evidence/worktree-02/task-4-lux-core-test.txt
    Expected: tests pass and include helper import assertions.
    Evidence: evidence/worktree-02/task-4-lux-core-test.txt

  Scenario: gateway still compiles after compatibility re-export
    Tool: bash
    Steps: cd gateway && cargo test redact_smoke 2>&1 | tee ../evidence/worktree-02/task-4-gateway-compat.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-02/task-4-gateway-compat.txt
  ```

  **Commit**: YES | Message: `refactor(core): extract shared lux primitives` | Files: `crates/lux-core/*`, selected gateway compatibility files, tests

- [ ] 5. `lux-project` Extraction

  **Worktree**: WT-03 `../lux-worktrees/wt-03-project`

  **What to do**: Move pure project detection and engine capability DTOs into `lux-project`. Detection may inspect files but must not run engine commands. Runtime command execution stays in gateway.

  **Must NOT do**: Do not claim Godot build/run/test or Three.js runtime support as verified.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8, 11 | Blocked By: 3

  **References**:
  - `gateway/src/project.rs`
  - `gateway/src/project_godot.rs`
  - `docs/godot-support.md`
  - `gateway/tests/lux_detection_test.rs`

  **Acceptance Criteria**:
  - [ ] RED test first proves engine capability DTO is unavailable from `lux_project`.
  - [ ] GREEN test proves Unity/Godot/Three.js capability records can represent `verified`, `partial`, `planned`, and `unsupported`.
  - [ ] Godot build remains unsupported until end-to-end evidence exists.
  - [ ] `cargo test -p lux-project` passes.

  **QA Scenarios**:
  ```text
  Scenario: Engine capability model distinguishes support levels
    Tool: bash
    Steps: cargo test -p lux-project capability_status_levels 2>&1 | tee evidence/worktree-03/task-5-capability-status.txt
    Expected: test passes for Unity verified, Godot partial, Three.js planned/unsupported cases.
    Evidence: evidence/worktree-03/task-5-capability-status.txt

  Scenario: Gateway detection behavior is preserved
    Tool: bash
    Steps: cd gateway && cargo test --test lux_detection_test 2>&1 | tee ../evidence/worktree-03/task-5-detection-compat.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-03/task-5-detection-compat.txt
  ```

  **Commit**: YES | Message: `refactor(project): extract engine capability models` | Files: `crates/lux-project/*`, `gateway/src/project*.rs`, tests

- [ ] 6. `lux-spec-core` and `lux-run-core` Contract Split

  **Worktree**: WT-04 `../lux-worktrees/wt-04-spec-run`

  **What to do**: Split pure spec/domain/ambiguity models into `lux-spec-core` and pure run/ticket/task/evidence-reference models into `lux-run-core`. Keep file-backed stores and server handlers in gateway unless they are pure and explicitly owned.

  **Must NOT do**: Do not move gateway-owned execution side effects, git push, process spawning, Axum handlers, or CLI parsing.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 12 | Blocked By: 3

  **References**:
  - `gateway/src/lux_spec.rs`
  - `gateway/src/lux_specs.rs`
  - `gateway/src/lux_spec_loop.rs`
  - `gateway/src/lux_ambiguity.rs`
  - `gateway/src/lux_run_state.rs`
  - `gateway/src/lux_ticket.rs`
  - `gateway/src/lux_task_dag.rs`
  - `gateway/tests/lux_spec_test.rs`
  - `gateway/tests/lux_ticket_test.rs`
  - `gateway/tests/lux_run_state_test.rs`

  **Acceptance Criteria**:
  - [ ] RED tests first prove selected spec/run types cannot be imported from target crates.
  - [ ] GREEN tests import selected spec/run/ticket types from target crates.
  - [ ] `.lux` schema versions remain unchanged unless a migration test is added.
  - [ ] `cd gateway && cargo test --test lux_spec_test --test lux_ticket_test --test lux_run_state_test` passes.

  **QA Scenarios**:
  ```text
  Scenario: Spec and run core models compile independently
    Tool: bash
    Steps: cargo test -p lux-spec-core && cargo test -p lux-run-core 2>&1 | tee evidence/worktree-04/task-6-core-tests.txt
    Expected: both crate test suites exit 0.
    Evidence: evidence/worktree-04/task-6-core-tests.txt

  Scenario: Existing gateway state tests are preserved
    Tool: bash
    Steps: cd gateway && cargo test --test lux_spec_test --test lux_ticket_test --test lux_run_state_test 2>&1 | tee ../evidence/worktree-04/task-6-gateway-state-tests.txt
    Expected: command exits 0 with no schema regressions.
    Evidence: evidence/worktree-04/task-6-gateway-state-tests.txt
  ```

  **Commit**: YES | Message: `refactor(state): extract spec and run core contracts` | Files: `crates/lux-spec-core/*`, `crates/lux-run-core/*`, selected gateway compatibility modules, tests

- [ ] 7. `lux-bridge-core` DTO Boundary

  **Worktree**: WT-05 `../lux-worktrees/wt-05-bridge-ast`

  **What to do**: Define Rust DTOs for bridge commands/results and Unity AST payloads in `lux-bridge-core`. This task does not change Unity C# behavior yet; it creates Rust-side schema compatibility and future protocol version fields.

  **Must NOT do**: Do not modify Unity bridge protocol in this task unless the change is version-compatible and covered by C# tests.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 10 | Blocked By: 3

  **References**:
  - `gateway/src/protocol.rs`
  - `gateway/src/bridge_types.rs`
  - `gateway/src/lux_unity_maneuver.rs`
  - `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs`
  - `bridge/unity/AiBridgeEditor/Ast/UnityAstNode.cs`

  **Acceptance Criteria**:
  - [ ] Rust DTOs can represent `read_asset_ast`, `get_selection_ast`, and `get_scene_ast` payloads.
  - [ ] DTOs include schema/protocol version fields.
  - [ ] Old bridge responses without new optional fields still parse if compatibility requires it.
  - [ ] `cargo test -p lux-bridge-core` passes.

  **QA Scenarios**:
  ```text
  Scenario: Rust bridge DTOs parse Unity AST payload fixtures
    Tool: bash
    Steps: cargo test -p lux-bridge-core ast_payload_round_trip 2>&1 | tee evidence/worktree-05/task-7-ast-dto.txt
    Expected: fixture payloads parse and serialize with schema version.
    Evidence: evidence/worktree-05/task-7-ast-dto.txt

  Scenario: Gateway Unity maneuver tests remain compatible
    Tool: bash
    Steps: cd gateway && cargo test lux_unity_maneuver 2>&1 | tee ../evidence/worktree-05/task-7-maneuver-compat.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-05/task-7-maneuver-compat.txt
  ```

  **Commit**: YES | Message: `refactor(bridge): extract bridge dto contracts` | Files: `crates/lux-bridge-core/*`, selected gateway bridge modules, tests

- [ ] 8. Engine Capability Blocker Model

  **Worktree**: WT-03 `../lux-worktrees/wt-03-project`

  **What to do**: Add explicit blocker payloads for unsupported, planned, missing-tool, and unverified-engine capabilities. Ensure Godot/Three.js planned paths cannot report completion success without verified evidence.

  **Must NOT do**: Do not add Godot/Three.js fake adapters or browser control.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 11, 12 | Blocked By: 5

  **References**:
  - `docs/godot-support.md`
  - `README.md` engine capability snapshot
  - `gateway/src/project_godot.rs`
  - `gateway/src/lux_ticket.rs` blocker fields

  **Acceptance Criteria**:
  - [ ] Capability blocker payload contains engine, capability, status, reason, evidence path, and recommended next supported action.
  - [ ] Unsupported engines create blocker evidence, not empty success.
  - [ ] Tests cover empty project, Unity-like project, Godot project, and Three.js marker project.

  **QA Scenarios**:
  ```text
  Scenario: Unsupported engine emits blocker model
    Tool: bash
    Steps: cargo test -p lux-project unsupported_engine_blocker_payload 2>&1 | tee evidence/worktree-03/task-8-blocker-payload.txt
    Expected: blocker includes engine, capability, reason, and recommended action.
    Evidence: evidence/worktree-03/task-8-blocker-payload.txt

  Scenario: Godot build remains unsupported without verification
    Tool: bash
    Steps: cd gateway && cargo test --test lux_detection_test godot 2>&1 | tee ../evidence/worktree-03/task-8-godot-guardrail.txt
    Expected: tests show detection/status but no build success claim.
    Evidence: evidence/worktree-03/task-8-godot-guardrail.txt
  ```

  **Commit**: YES | Message: `feat(project): model engine capability blockers` | Files: `crates/lux-project/*`, gateway project compatibility files, tests

- [ ] 9. Verification Ontology Schema

  **Worktree**: WT-06 `../lux-worktrees/wt-06-verification`

  **What to do**: Implement pure ontology schema in `lux-verification-core`: scene, stage, actor, component, transform, camera, viewport, coordinate frames, expected visual state, evidence class, blocker class, completion gate, and schema version.

  **Must NOT do**: Do not call bridge commands, screenshot capture, vision providers, Axum, or gateway process execution.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 10, 12, 13 | Blocked By: 1, 3

  **References**:
  - `gateway/src/lux_verification.rs`
  - `gateway/src/visual_regression.rs`
  - `Skills/skills/game-dev/SKILL.md`
  - `docs/roadmap-reality-lock.md`

  **Acceptance Criteria**:
  - [ ] RED test first proves ontology schema is missing from `lux-verification-core`.
  - [ ] GREEN test validates required ontology terms and schema version.
  - [ ] Completion gate schema rejects `pixel_only`.
  - [ ] Blocker schema supports missing AST, missing coordinate map, unsupported engine, and unverified visual match.

  **QA Scenarios**:
  ```text
  Scenario: Ontology rejects pixel-only completion gate
    Tool: bash
    Steps: cargo test -p lux-verification-core rejects_pixel_only_completion 2>&1 | tee evidence/worktree-06/task-9-pixel-only.txt
    Expected: test passes and assertion names pixel-only rejection.
    Evidence: evidence/worktree-06/task-9-pixel-only.txt

  Scenario: Ontology terms are complete
    Tool: bash
    Steps: cargo test -p lux-verification-core ontology_required_terms 2>&1 | tee evidence/worktree-06/task-9-required-terms.txt
    Expected: scene/stage/actor/component/transform/camera/viewport/coordinate/evidence/blocker terms exist.
    Evidence: evidence/worktree-06/task-9-required-terms.txt
  ```

  **Commit**: YES | Message: `feat(verification): add game ontology schema` | Files: `crates/lux-verification-core/*`, tests

- [ ] 10. Unity AST and Coordinate Mapping Contract

  **Worktree**: WT-05 `../lux-worktrees/wt-05-bridge-ast`

  **What to do**: Extend bridge/core contracts for scene AST node identity and coordinate mapping payloads. Unity C# changes must remain protocol-compatible and optional for older bridge installs.

  **Must NOT do**: Do not require screenshot annotations without declared coordinate frame. Do not break existing AST protocol commands.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 12 | Blocked By: 7, 9

  **References**:
  - `bridge/unity/AiBridgeEditor/Ast/UnityAstNode.cs`
  - `bridge/unity/AiBridgeEditor/Ast/UnityAstSerializer.cs`
  - `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs`
  - `Skills/skills/lux-unity/references/screenshots.md`
  - `Skills/skills/lux-unity/references/playmode-input.md`

  **Acceptance Criteria**:
  - [ ] Scene AST has stable node ID contract for visual matching.
  - [ ] Coordinate mapping distinguishes world, local, screen, viewport, UI Canvas, and input coordinates.
  - [ ] C# tests round-trip payloads through `JsonUtility`.
  - [ ] Rust DTO tests parse mapping payload fixtures.

  **QA Scenarios**:
  ```text
  Scenario: Unity AST node identity is stable in fixtures
    Tool: bash
    Steps: mkdir -p evidence/worktree-05 && cd gateway && cargo run -- run-tests --project-path ../bridge/unity --test-platform EditMode --test-results ../evidence/worktree-05/task-10-unity-editmode-results.xml --log-file ../evidence/worktree-05/task-10-unity-editmode.log 2>&1 | tee ../evidence/worktree-05/task-10-unity-ast-identity.txt
    Expected: command exits 0 and Unity EditMode results include AST/protocol tests for scene, selection, and asset AST payloads.
    Evidence: evidence/worktree-05/task-10-unity-ast-identity.txt

  Scenario: Coordinate mapping payload round-trips
    Tool: bash
    Steps: cargo test -p lux-bridge-core coordinate_mapping_round_trip 2>&1 | tee evidence/worktree-05/task-10-coordinate-roundtrip.txt
    Expected: world/local/screen/viewport/ui/input frames serialize and deserialize.
    Evidence: evidence/worktree-05/task-10-coordinate-roundtrip.txt
  ```

  **Commit**: YES | Message: `feat(bridge): define ast coordinate mapping contract` | Files: `crates/lux-bridge-core/*`, Unity bridge AST/protocol files if needed, tests

- [ ] 11. Godot and Three.js Capability Guardrails

  **Worktree**: WT-03 `../lux-worktrees/wt-03-project`

  **What to do**: Ensure capability models and docs keep Godot partial and Three.js planned unless real end-to-end verification exists.

  **Must NOT do**: Do not add fake Godot/Three.js parity or browser automation as a substitute for engine verification.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 12, 15 | Blocked By: 8

  **References**:
  - `docs/godot-support.md`
  - `README.md` engine capability snapshot
  - `docs/usage.md` engine support
  - `gateway/src/project_godot.rs`

  **Acceptance Criteria**:
  - [ ] Search checks show no docs claim Godot build/run/test verified.
  - [ ] Three.js remains planned unless an implementation plan supplies real verification.
  - [ ] Capability blockers include explicit evidence paths.

  **QA Scenarios**:
  ```text
  Scenario: Docs do not claim fake engine parity
    Tool: bash
    Steps: '! rg -n "Godot.*build.*verified|Three\\.js.*verified|Godot.*runtime parity|Three\\.js.*runtime parity" README.md docs plans'
    Expected: command exits 0.
    Evidence: evidence/worktree-03/task-11-fake-parity-scan.txt

  Scenario: Capability model produces blocker for planned engine surface
    Tool: bash
    Steps: cargo test -p lux-project planned_engine_requires_blocker_evidence 2>&1 | tee evidence/worktree-03/task-11-planned-blocker.txt
    Expected: planned support cannot produce completion success.
    Evidence: evidence/worktree-03/task-11-planned-blocker.txt
  ```

  **Commit**: YES | Message: `docs(project): preserve engine capability guardrails` | Files: `crates/lux-project/*`, docs if needed, tests

- [ ] 12. Evidence Gate Router

  **Worktree**: WT-06 `../lux-worktrees/wt-06-verification`

  **What to do**: Add layer-aware evidence gate types and router behavior in `lux-verification-core`, then wire gateway verification compatibility only through WT-10 if shared files are needed.

  **Must NOT do**: Do not make vision confidence sufficient for completion. Do not run engine commands inside core crate.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 13, 14, 15 | Blocked By: 8, 9, 10, 11

  **References**:
  - `gateway/src/lux_verification.rs`
  - `gateway/src/lux_ticket.rs`
  - `gateway/tests/lux_verification_test.rs`
  - `gateway/tests/lux_ticket_test.rs`

  **Acceptance Criteria**:
  - [ ] Evidence gate requires AST evidence for visual scene claims.
  - [ ] Evidence gate requires coordinate/camera/UI evidence for location claims.
  - [ ] Screenshot/vision evidence must reference AST node ID, coordinate region, or blocker reason.
  - [ ] `doc_only` remains valid for docs/contract changes only.

  **QA Scenarios**:
  ```text
  Scenario: Visual completion requires non-vision evidence
    Tool: bash
    Steps: cargo test -p lux-verification-core visual_completion_requires_ast_and_mapping 2>&1 | tee evidence/worktree-06/task-12-visual-gate.txt
    Expected: pixel-only or vision-only fixture is rejected.
    Evidence: evidence/worktree-06/task-12-visual-gate.txt

  Scenario: Doc-only route remains contract-only
    Tool: bash
    Steps: cargo test -p lux-verification-core doc_only_accepts_contract_docs_only 2>&1 | tee evidence/worktree-06/task-12-doc-only.txt
    Expected: doc_only accepts contract docs and rejects runtime completion claims.
    Evidence: evidence/worktree-06/task-12-doc-only.txt
  ```

  **Commit**: YES | Message: `feat(verification): add layer-aware evidence gates` | Files: `crates/lux-verification-core/*`, tests

- [ ] 13. AI Context Injection Payloads

  **Worktree**: WT-07 `../lux-worktrees/wt-07-ai-context`

  **What to do**: Define pure prompt/context payloads in `lux-ai-core` and update plugin prompt templates to include ontology, AST summary, coordinate mapping summary, evidence gate requirements, and blockers.

  **Must NOT do**: Do not let OpenCode adapter directly write canonical `.lux` verification state.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 15 | Blocked By: 9

  **References**:
  - `gateway/src/templates/plugin/prompt-builder.ts`
  - `gateway/src/templates/plugin/next-action-generator.ts`
  - `gateway/src/templates/plugin/continuation-injector.ts`
  - `adapters/opencode/lux-plugin.ts`
  - `gateway/src/lux_hooks.rs`

  **Acceptance Criteria**:
  - [ ] Prompt payload includes ontology and evidence gate requirements.
  - [ ] Prompt text forbids pixel-only completion and fake engine parity.
  - [ ] Adapter remains a requester/reporter, not canonical state owner.
  - [ ] Existing plugin prompt tests pass.

  **QA Scenarios**:
  ```text
  Scenario: Prompt builder includes game verification context
    Tool: bash
    Steps: cd gateway/src/templates/plugin && npm test -- prompt-builder 2>&1 | tee ../../../../evidence/worktree-07/task-13-prompt-builder.txt
    Expected: tests pass and assert ontology/evidence guardrails appear in generated prompt.
    Evidence: evidence/worktree-07/task-13-prompt-builder.txt

  Scenario: Adapter does not write canonical verification state
    Tool: bash
    Steps: rg -n "writeFile|appendFile|\\.lux/verification|evidence_gate" adapters/opencode gateway/src/templates/plugin > evidence/worktree-07/task-13-write-scan.txt
    Expected: any writes are logged/temporary/plugin-state only; canonical verification writes route through gateway API.
    Evidence: evidence/worktree-07/task-13-write-scan.txt
  ```

  **Commit**: YES | Message: `feat(ai): inject game verification context` | Files: `crates/lux-ai-core/*`, `gateway/src/templates/plugin/*`, `adapters/opencode/lux-plugin.ts`, tests

- [ ] 14. Gateway API and CLI Integration

  **Worktree**: WT-10 `../lux-worktrees/wt-10-integration`

  **What to do**: Wire merged core crate APIs into gateway CLI/server surfaces. This is the only task that edits `gateway/src/main.rs` and `gateway/src/server.rs`.

  **Must NOT do**: Do not introduce remote/WebRTC as a verification path. Do not change public CLI names unless tests and docs are updated.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 15, 16 | Blocked By: 12

  **References**:
  - `gateway/src/main.rs`
  - `gateway/src/server.rs`
  - `gateway/src/lux_mcp.rs`
  - `gateway/tests/gateway_cli_smoke.rs`
  - `gateway/tests/lux_verification_test.rs`
  - `gateway/src/AGENTS.md`

  **Acceptance Criteria**:
  - [ ] Gateway imports core crate APIs instead of duplicated canonical models where migrated.
  - [ ] API responses preserve existing shape unless versioned.
  - [ ] Server tests cover new projection routes if added.
  - [ ] CLI help remains stable.

  **QA Scenarios**:
  ```text
  Scenario: Gateway CLI smoke remains stable
    Tool: bash
    Steps: cd gateway && cargo test --test gateway_cli_smoke 2>&1 | tee ../evidence/worktree-10/task-14-cli-smoke.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-10/task-14-cli-smoke.txt

  Scenario: Verification API does not expose WebRTC dependency
    Tool: bash
    Steps: cd gateway && cargo test lux_verify 2>&1 | tee ../evidence/worktree-10/task-14-verify-api.txt
    Expected: verification tests pass and no route requires remote/WebRTC.
    Evidence: evidence/worktree-10/task-14-verify-api.txt
  ```

  **Commit**: YES | Message: `feat(gateway): wire core verification contracts` | Files: `gateway/src/main.rs`, `gateway/src/server.rs`, gateway compatibility modules, tests

- [ ] 15. Dashboard Projection

  **Worktree**: WT-08 `../lux-worktrees/wt-08-dashboard`

  **What to do**: Add read-only UI/API client projection for layer readiness, AST snapshot status, coordinate map status, visual match status, accepted evidence, missing evidence, and blocker evidence.

  **Must NOT do**: Do not make dashboard the verification authority. Do not use `useWebRTC` or remote session UI as a game verification dependency.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 16 | Blocked By: 12, 13, 14

  **References**:
  - `gateway/ui-src/src/lib/api.ts`
  - `gateway/ui-src/src/hooks/useDashboard.ts`
  - `gateway/ui-src/src/hooks/useSpec.ts`
  - `gateway/ui-src/src/hooks/useProgress.ts`
  - `gateway/ui-src/src/components/dashboard/VisualReportPanel.tsx`
  - `gateway/ui-src/src/components/dashboard/LoopStatusPanel.tsx`
  - `gateway/ui-src/src/hooks/useWebRTC.ts` as out-of-scope reference only

  **Acceptance Criteria**:
  - [ ] UI distinguishes missing evidence, blocker evidence, and accepted evidence.
  - [ ] No mock/fallback API data is added.
  - [ ] Dashboard projection is read-only over canonical `.lux` verification state.
  - [ ] TypeScript strict typecheck passes.

  **QA Scenarios**:
  ```text
  Scenario: Dashboard typecheck passes
    Tool: bash
    Steps: cd gateway/ui-src && npx tsc --noEmit 2>&1 | tee ../../evidence/worktree-08/task-15-tsc.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-08/task-15-tsc.txt

  Scenario: UI does not use WebRTC as verification dependency
    Tool: bash
    Steps: '! rg -n "useWebRTC\\(|RemoteViewer|remote.*verification|WebRTC.*verification" gateway/ui-src/src/components gateway/ui-src/src/hooks gateway/ui-src/src/lib'
    Expected: command exits 0.
    Evidence: evidence/worktree-08/task-15-webrtc-guardrail.txt
  ```

  **Commit**: YES | Message: `feat(ui): project game verification evidence` | Files: `gateway/ui-src/src/*`, tests

- [ ] 16. Docs and Skills Final Projection

  **Worktree**: WT-09 `../lux-worktrees/wt-09-docs-skills`

  **What to do**: Update public docs and bundled skills to describe the package split, ontology layers, worktree execution model, and verification guardrails.

  **Must NOT do**: Do not claim implementation is complete unless final verification evidence exists.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 17 | Blocked By: 12, 14, 15

  **References**:
  - `README.md`
  - `docs/usage.md`
  - `docs/roadmap-reality-lock.md`
  - `Skills/skills/game-dev/SKILL.md`
  - `Skills/skills/regression-suite/SKILL.md`
  - `Skills/skills/lux-unity/SKILL.md`

  **Acceptance Criteria**:
  - [ ] Docs explain core package split without overstating shipped state.
  - [ ] Skills require context-first, vision-supplemented evidence.
  - [ ] Docs preserve Godot/Three.js capability guardrails.
  - [ ] Skill validation passes.

  **QA Scenarios**:
  ```text
  Scenario: Docs describe split and guardrails
    Tool: bash
    Steps: rg -n "core package|Game Verification Ontology|pixel-only|Godot.*partial|Three\\.js.*planned|WebRTC.*out-of-scope" README.md docs Skills/skills > evidence/worktree-09/task-16-doc-scan.txt
    Expected: output contains split and guardrail references.
    Evidence: evidence/worktree-09/task-16-doc-scan.txt

  Scenario: Skills validate after wording updates
    Tool: bash
    Steps: bash Skills/tools/validate-skills.sh 2>&1 | tee evidence/worktree-09/task-16-skills-validate.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-09/task-16-skills-validate.txt
  ```

  **Commit**: YES | Message: `docs: project game verification layer split` | Files: docs, skill docs, evidence

- [ ] 17. Cross-Worktree Integration Gate

  **Worktree**: WT-10 `../lux-worktrees/wt-10-integration`

  **What to do**: Merge worktrees in the prescribed order, resolve conflicts only in owned integration files, and run full local verification.

  **Must NOT do**: Do not weaken or delete failing tests. Do not revert unrelated user changes.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 18 | Blocked By: 14, 15, 16, 19, 20

  **References**:
  - All task evidence paths.
  - Root `Cargo.toml`
  - `gateway/Cargo.toml`
  - `gateway/src/lib.rs`
  - `gateway/src/main.rs`
  - `gateway/src/server.rs`

  **Acceptance Criteria**:
  - [ ] `git worktree list` shows all worktrees accounted for before cleanup.
  - [ ] `cargo test --workspace` passes.
  - [ ] `cd gateway && cargo test` passes.
  - [ ] `cd gateway/ui-src && npx tsc --noEmit` passes.
  - [ ] `bash Skills/tools/validate-skills.sh` passes.
  - [ ] `git diff --name-only` contains only intended files.

  **QA Scenarios**:
  ```text
  Scenario: Full Rust workspace is green
    Tool: bash
    Steps: cargo test --workspace 2>&1 | tee evidence/worktree-10/task-17-cargo-workspace.txt
    Expected: command exits 0.
    Evidence: evidence/worktree-10/task-17-cargo-workspace.txt

  Scenario: All non-Rust gates are green
    Tool: bash
    Steps: cd gateway/ui-src && npx tsc --noEmit && cd ../.. && bash Skills/tools/validate-skills.sh 2>&1 | tee evidence/worktree-10/task-17-non-rust-gates.txt
    Expected: TypeScript and skill validation exit 0.
    Evidence: evidence/worktree-10/task-17-non-rust-gates.txt
  ```

  **Commit**: YES | Message: `chore: integrate game verification core split` | Files: integration-owned files, evidence

- [ ] 18. Release-Ready Verification Report

  **Worktree**: WT-10 `../lux-worktrees/wt-10-integration`

  **What to do**: Produce final evidence index and update planning/report docs with what was implemented, what remains planned, and what is explicitly blocked.

  **Must NOT do**: Do not claim autonomous game development or visual correctness verification is complete unless every required evidence gate exists and passes.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: Final Verification | Blocked By: 17

  **References**:
  - `docs/roadmap-reality-lock.md`
  - `plans/lux-game-harness-overhaul.md`
  - This plan file
  - All `evidence/worktree-*`

  **Acceptance Criteria**:
  - [ ] Evidence index names every task and command result.
  - [ ] Roadmap docs distinguish implemented, scaffolded, planned, and blocked surfaces.
  - [ ] Worktree cleanup instructions are recorded.
  - [ ] No stale claim says WebRTC, remote Unity browser control, fake engine parity, or pixel-only completion is allowed.

  **QA Scenarios**:
  ```text
  Scenario: Evidence report covers every task
    Tool: bash
    Steps: for n in $(seq 1 20); do rg -n "task-${n}" evidence/worktree-* >/dev/null || exit 1; done
    Expected: every task has at least one evidence reference.
    Evidence: evidence/worktree-10/task-18-evidence-index.txt

  Scenario: Guardrail scan stays clean
    Tool: bash
    Steps: '! rg -n "pixel-only.*complete|WebRTC.*verification dependency|remote Unity browser control|Godot.*verified build|Three\\.js.*verified runtime" README.md docs plans Skills/skills'
    Expected: command exits 0.
    Evidence: evidence/worktree-10/task-18-guardrail-scan.txt
  ```

  **Commit**: YES | Message: `docs: record game verification split evidence` | Files: docs, plans, evidence index

- [ ] 19. QA Ledger and Evidence Index Scaffold

  **Worktree**: WT-11 `../lux-worktrees/wt-11-qa-ledger`

  **What to do**: Create the evidence directory convention, task evidence checklist, and ledger/index template that every parallel worktree must fill. This task runs in parallel with all implementation work and never owns source code.

  **Must NOT do**: Do not run implementation tests for other workers. Do not mark another worker's criterion as passed. Do not edit Rust, C#, TypeScript, TSX, or skill source.

  **Parallelization**: Can Parallel: YES | Wave 0 | Blocks: 17, 18 | Blocked By: none

  **References**:
  - Evidence patterns in `plans/lux-game-harness-overhaul.md`
  - Verification commands in `AGENTS.md`
  - This plan's `Verification Strategy`

  **Acceptance Criteria**:
  - [ ] `evidence/worktree-11/evidence-index-template.md` defines required fields: task id, worktree, command, expected result, observed result, artifact path, cleanup receipt, and reviewer.
  - [ ] `evidence/worktree-11/task-checklist.md` lists Tasks 1-20 and their required evidence files.
  - [ ] The template rejects "tests pass" as standalone completion evidence.
  - [ ] No executable source files are changed.

  **QA Scenarios**:
  ```text
  Scenario: Evidence template covers every task
    Tool: bash
    Steps: for n in $(seq 1 20); do rg -n "Task ${n}|task-${n}" evidence/worktree-11/task-checklist.md >/dev/null || exit 1; done
    Expected: every task number appears in the checklist.
    Evidence: evidence/worktree-11/task-19-checklist-coverage.txt

  Scenario: Evidence template forbids test-only proof
    Tool: bash
    Steps: rg -n "tests pass.*not.*standalone|standalone completion evidence" evidence/worktree-11/evidence-index-template.md
    Expected: output contains the explicit rule.
    Evidence: evidence/worktree-11/task-19-test-only-guardrail.txt
  ```

  **Commit**: YES | Message: `docs(qa): scaffold worktree evidence ledger` | Files: `evidence/worktree-11/*`

- [ ] 20. Bridge Protocol Compatibility Review

  **Worktree**: WT-12 `../lux-worktrees/wt-12-protocol-review`

  **What to do**: Review Unity bridge protocol compatibility before WT-05 changes C# protocol DTOs. Produce a compatibility matrix for existing commands, optional fields, protocol versioning, and old installed bridge behavior.

  **Must NOT do**: Do not edit Unity bridge source. Do not add protocol fields. Do not change gateway bridge behavior.

  **Parallelization**: Can Parallel: YES | Wave 0 | Blocks: 10, 17 | Blocked By: none

  **References**:
  - `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs`
  - `bridge/unity/AiBridgeEditor/UnityAiBridgeTcpServer.cs`
  - `bridge/unity/AiBridgeEditor/Ast/UnityAstNode.cs`
  - `bridge/unity/AiBridgeTests/Editor/UnityAiBridgeProtocolTests.cs`
  - `bridge/unity/AiBridgeTests/Editor/UnityAiBridgeTcpServerTests.cs`

  **Acceptance Criteria**:
  - [ ] `evidence/worktree-12/protocol-compatibility-matrix.md` lists every AST and screenshot/context command relevant to this plan.
  - [ ] Matrix labels each proposed field as required, optional, additive, breaking, or unsupported.
  - [ ] Matrix defines how older installed bridges should fail visibly instead of silently returning empty AST/mapping data.
  - [ ] WT-05 uses this matrix before merging protocol-affecting changes.

  **QA Scenarios**:
  ```text
  Scenario: Protocol matrix covers AST and screenshot/context commands
    Tool: bash
    Steps: rg -n "read_asset_ast|get_selection_ast|get_scene_ast|capture_lux_screenshot|get_lux_context" evidence/worktree-12/protocol-compatibility-matrix.md
    Expected: every relevant command appears.
    Evidence: evidence/worktree-12/task-20-command-coverage.txt

  Scenario: Matrix classifies breaking vs additive changes
    Tool: bash
    Steps: rg -n "required|optional|additive|breaking|unsupported|older installed bridge" evidence/worktree-12/protocol-compatibility-matrix.md
    Expected: compatibility categories and old-bridge behavior are documented.
    Evidence: evidence/worktree-12/task-20-compat-categories.txt
  ```

  **Commit**: YES | Message: `docs(bridge): audit protocol compatibility for verification split` | Files: `evidence/worktree-12/*`

## Final Verification Wave
> Run after all implementation tasks. All checks are agent-executed. If this plan is only being edited as a document, run only the planning-session checks below.

- [ ] F1. Plan Compliance Audit
  - Command: `rg -n "Worktree|Acceptance Criteria|QA Scenarios|Commit|No WebRTC|No pixel-only|fake Godot|fake engine parity" plans/lux-game-verification-ontology-suite-split.md`
  - Pass: required sections and guardrails are present.

- [ ] F2. Dependency and Ownership Audit
  - Command: `rg -n "Shared File Ownership|Dependency Matrix|Merge Order|WT-01|WT-10" plans/lux-game-verification-ontology-suite-split.md`
  - Pass: worktree ownership and merge rules are explicit.

- [ ] F3. Source Mutation Audit for Planning Session
  - Command: `git diff --name-only`
  - Pass for this session: only the plan file and allowed draft artifacts changed by this turn.

- [ ] F4. Implementation Full Gate
  - Command: `cargo test --workspace && cd gateway && cargo test && cd ui-src && npx tsc --noEmit && cd ../.. && bash Skills/tools/validate-skills.sh`
  - Pass: all commands exit 0 after implementation starts. Not required for this document-only session.

## Commit Strategy
- One commit per worktree task after that task's local verification passes.
- Merge WT-01 before any code extraction worktree.
- Merge WT-10 last for gateway/server/CLI wiring.
- Use conventional commits listed in each task.
- Do not auto-commit in the planning session.

## Success Criteria
- The plan is decision-complete enough for parallel workers to start without choosing ownership or dependency order.
- Worktree names, shared-file owners, merge order, QA commands, and evidence paths are explicit.
- Every task has acceptance criteria and at least two QA scenarios.
- Guardrails are enforceable by search and tests.
- Implementation remains future work; this document does not claim it is complete.
