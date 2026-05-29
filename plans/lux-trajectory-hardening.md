# Lux Trajectory Hardening Plan

## TL;DR
> **Summary**: The repository has moved from a Unity-only adapter toward a local-first multi-engine AI harness with gateway-owned execution, source adapters, federated skills, and split bridge ownership. The next plan is not to advance M1-M6 autonomy yet; it is to make this new shape internally consistent, packageable, and verifiable.
> **Deliverables**:
> - A cleaned trajectory statement and reality-locked docs.
> - Stale `seeds/`, `plugins/`, `Skills`, and adapter path references removed or redirected.
> - Tests/smokes for OpenCode adapter installation, Unity bridge installation, skills validation, uloop manifest fallback, project-structure sanity, and submodule/package hygiene.
> - A verified baseline report naming the one known pre-existing Rust test failure if it still exists.
> **Effort**: Medium
> **Parallel**: YES - 3 waves
> **Critical Path**: Task 1 -> Task 5 -> Task 6 -> Final Verification

## Context
### Original Request
User asked: "지금까지 작업 한 내용들이 어떤 trajectory 로 가고 있는지 수정 사항이나 설계 부족한 다음 계획은 무엇인지 잡아줄래 ?"

### Interview Summary
No blocking user interview was needed. Repo exploration provided enough facts to choose defaults:
- Brownfield architecture-level planning.
- Tests-after strategy because Rust/TypeScript test infrastructure already exists.
- Scope is stabilization of current committed trajectory, not execution of future autonomous roadmap milestones.

### Metis Review (gaps addressed)
Metis review found gaps and broadened the plan from docs/path cleanup to installation topology hardening. The final plan handles these execution risks:
- `README.md` still projects deleted `seeds/` and old `Skills` layout.
- `gateway/src/uloop_sync.rs` still depends on a deleted bundled manifest path.
- `Skills/tools/validate-skills.sh` points at `Skills/.claude/skills`, but tracked skills live at `Skills/skills`.
- `.gitmodules` says `Skills` is a submodule, while `git ls-tree HEAD Skills` shows a normal tracked tree.
- `bridge` is a gitlink submodule and may be uninitialized in fresh checkouts.
- OpenCode source adapter moved to `adapters/opencode`, but install coverage is not directly tested.
- `scripts/check-project-structure.sh` still requires deleted `seeds`.
- `lux init` and `lux bridge install` currently imply different OpenCode plugin targets: `.opencode/plugins/lux-plugin.ts` versus `.opencode/plugins/lux/`.
- `bridge/unity/README.md` promises `Assets/Editor/LuxBridge/`, while gateway bridge install still looks for `bridge/AiBridgeEditor` and installs `Assets/Editor/AiBridgeEditor/`.

## Work Objectives
### Core Objective
Make the current repository trajectory coherent and release-safe before starting new autonomous roadmap work.

### Deliverables
- Updated docs that describe the actual repository shape.
- Code/tests that no longer reference deleted source paths.
- Explicit repository ownership decision for `Skills/` and `bridge/`.
- Verification evidence under `evidence/`.

### Definition of Done
- `git status --short --branch` shows only intentional plan/execution changes.
- `rg -n "plugins/opencode|seeds/uloop-manifest|bridge-threejs|Skills/.claude/skills|YAML 시드 데이터" README.md docs gateway Skills adapters` returns no stale production/doc references except intentional migration notes.
- `cd gateway && cargo build` exits 0.
- `cd gateway && cargo test` exits 0, or only the already documented `capture_integration_session_stream_input_stop_and_health` failure remains and is recorded in evidence.
- `cd gateway/ui-src && npx tsc --noEmit` exits 0.
- Adapter install smoke proves `lux init` or the install helper copies from `adapters/opencode/lux-plugin.ts`.
- Bridge install smoke proves `lux bridge install` copies from the actual bridge source layout into the documented Unity target layout.
- Skills validation proves the tracked skill tree is checked from its real path.
- `scripts/check-project-structure.sh` exits 0 against the new repository topology.

### Must Have
- Gateway remains the execution control-plane owner per `docs/adr/ADR-001-Gateway-as-Execution-Owner.md:15`.
- `.lux/` remains runtime SSoT; source assets under `adapters/`, `Skills/`, and `bridge/` must not become shadow runtime truth.
- Hidden experimental WebRTC remains opt-in only through `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`.

### Must NOT Have
- No new M1-M6 autonomy implementation in this stabilization pass.
- No silent fallback to deleted seed/plugin paths.
- No mock API data in UI hooks.
- No weakening or deleting failing tests.
- No untracked dependency on an initialized submodule without a visible checkout check.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: tests-after with existing Rust unit/integration tests, TypeScript typecheck, project-structure script, shell smokes, and targeted path scans.
- QA policy: Every task has happy-path and failure/edge checks.
- Evidence: `evidence/task-{N}-{slug}.txt` or `.json`.

## Execution Strategy
### Parallel Execution Waves
Wave 1: Task 1, Task 2, Task 3
Wave 2: Task 4, Task 5, Task 6, Task 7
Wave 3: Task 8, Task 9, Task 10

### Dependency Matrix
| Task | Depends On | Blocks |
| --- | --- | --- |
| 1. Reality inventory | none | 2, 4, 8 |
| 2. Docs projection repair | 1 | 8 |
| 3. Skills validation repair | none | 8 |
| 4. Uloop manifest path hardening | 1 | 8 |
| 5. OpenCode install topology decision | 1 | 6, 8 |
| 6. Unity bridge install topology repair | 1, 5 | 8 |
| 7. Submodule/package hygiene | 1 | 8 |
| 8. Project structure and baseline verification report | 2, 3, 4, 5, 6, 7 | 9, 10 |
| 9. Trajectory recommendation artifact | 8 | 10 |
| 10. Release-readiness gate | 8, 9 | Final Verification |

## TODOs
- [x] 1. Reality Inventory Lock

  **What to do**: Capture a machine-readable inventory of repository shape after the 5 ahead commits: tracked directories, submodules, removed legacy directories, source adapter paths, skill paths, and known verification status. Store the summary in `docs/roadmap-reality-lock.md` or a small adjacent section if it is currently stale.
  **Must NOT do**: Do not reintroduce deleted `seeds/`, `plugins/`, or `bridge-threejs/dist` content.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 2, 4, 8 | Blocked By: none

  **References**:
  - Pattern: `docs/roadmap-reality-lock.md:1` - existing engineering gap matrix.
  - Evidence: `git diff --stat origin/main..HEAD` currently shows 266 files changed, 8,982 insertions, 38,351 deletions.
  - Fact: `docs/roadmap-reality-lock.md:68` says `seeds/` does not exist.

  **Acceptance Criteria**:
  - [ ] `docs/roadmap-reality-lock.md` names `adapters/`, `Skills/skills`, and `bridge` ownership accurately.
  - [ ] `rg -n "seeds/ directory does not exist|adapters/opencode|Skills/skills" docs/roadmap-reality-lock.md` finds the updated inventory.

  **QA Scenarios**:
  ```text
  Scenario: Current repository shape is documented
    Tool: bash
    Steps: git ls-tree HEAD bridge Skills && git submodule status && rg -n "adapters/opencode|Skills/skills|seeds/" docs/roadmap-reality-lock.md
    Expected: docs mention actual paths; submodule/tree status is not contradicted.
    Evidence: evidence/task-1-reality-inventory.txt

  Scenario: Deleted legacy paths are not treated as live source
    Tool: bash
    Steps: test ! -d seeds && test ! -d plugins && test ! -d bridge-threejs
    Expected: command exits 0 and docs do not claim these are active directories.
    Evidence: evidence/task-1-legacy-paths.txt
  ```

  **Commit**: YES | Message: `docs(roadmap): lock current repository trajectory` | Files: `docs/roadmap-reality-lock.md`

- [x] 2. README and Usage Projection Repair

  **What to do**: Update `README.md` and `docs/usage.md` so they project the actual repo shape: `adapters/` source adapters, `Skills/skills/` skill content, no live `seeds/` directory, and hidden experimental WebRTC only.
  **Must NOT do**: Do not describe planned Three.js/Godot capabilities as verified unless docs already have supporting maturity status.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 8 | Blocked By: 1

  **References**:
  - Stale path: `README.md:474` lists deleted `seeds/`.
  - Stale layout: `README.md:460` still shows old flat skill directories.
  - Guardrail: `README.md:519` keeps WebRTC experimental and gated.
  - Maturity pattern: `docs/roadmap-reality-lock.md:18` and `docs/godot-support.md`.

  **Acceptance Criteria**:
  - [ ] `rg -n "YAML 시드 데이터|120\\+ 항목|Skills/.claude/skills|plugins/opencode|bridge-threejs" README.md docs/usage.md` returns no stale active-path claims.
  - [ ] `rg -n "adapters/opencode|Skills/skills|experimental_flags.remote_webrtc" README.md docs/usage.md` returns expected updated references.

  **QA Scenarios**:
  ```text
  Scenario: Public docs match current source layout
    Tool: bash
    Steps: rg -n "adapters/opencode|Skills/skills" README.md docs/usage.md
    Expected: both files describe actual source paths.
    Evidence: evidence/task-2-docs-layout.txt

  Scenario: Deleted paths are not advertised
    Tool: bash
    Steps: '! rg -n "YAML 시드 데이터|plugins/opencode|bridge-threejs" README.md docs/usage.md'
    Expected: command exits 0.
    Evidence: evidence/task-2-stale-paths.txt
  ```

  **Commit**: YES | Message: `docs: align projections with repository reality` | Files: `README.md`, `docs/usage.md`

- [x] 3. Skills Validation Path Repair

  **What to do**: Make `Skills/tools/validate-skills.sh` validate the tracked skill source under `Skills/skills`, or explicitly support both `Skills/skills` and generated `.claude/skills` with observable output. Use the current tracked layout as the default.
  **Must NOT do**: Do not silently pass when the skills root is missing.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 8 | Blocked By: none

  **References**:
  - Current mismatch: `Skills/tools/validate-skills.sh:7` sets `SKILLS_ROOT="$REPO_ROOT/.claude/skills"`.
  - Current tracked tree: `Skills/skills/lux-unity/SKILL.md` and sibling skill directories.
  - Schema docs: `Skills/SKILL-SCHEMA.md`.

  **Acceptance Criteria**:
  - [ ] `bash Skills/tools/validate-skills.sh` validates `Skills/skills` by default and exits nonzero on real schema failures.
  - [ ] Missing-root behavior remains explicit and nonzero.

  **QA Scenarios**:
  ```text
  Scenario: Validator checks tracked skills
    Tool: bash
    Steps: bash Skills/tools/validate-skills.sh
    Expected: command scans Skills/skills and reports pass/fail per real skill files.
    Evidence: evidence/task-3-skills-validate.txt

  Scenario: Validator fails on invalid root
    Tool: bash
    Steps: SKILLS_ROOT=/tmp/lux-missing-skills bash Skills/tools/validate-skills.sh
    Expected: exits nonzero with explicit missing-root message.
    Evidence: evidence/task-3-skills-missing-root.txt
  ```

  **Commit**: YES | Message: `fix(skills): validate tracked skill source tree` | Files: `Skills/tools/validate-skills.sh`

- [x] 4. Uloop Manifest Fallback Hardening

  **What to do**: Remove dependency on deleted `seeds/uloop-manifest.json`. Choose one concrete replacement: either move the bundled manifest into a live path such as `gateway/assets/uloop-manifest.json`, or remove bundled fallback and make remote unavailability an observable warning with no false local fallback. Recommended default: add a live bundled manifest under `gateway/assets/` and update `BUNDLED_MANIFEST_PATH`.
  **Must NOT do**: Do not leave a fallback path that always fails after the cleanup commit.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1

  **References**:
  - Broken URL/path: `gateway/src/uloop_sync.rs:21` and `gateway/src/uloop_sync.rs:24`.
  - Fallback read site: `gateway/src/uloop_sync.rs:209`.
  - No silent fallback invariant from `AGENTS.md`.

  **Acceptance Criteria**:
  - [ ] `rg -n "seeds/uloop-manifest" gateway/src` returns no matches.
  - [ ] A test or smoke proves the bundled fallback path exists and parses when remote fetch fails.
  - [ ] Fallback warning remains observable.

  **QA Scenarios**:
  ```text
  Scenario: Bundled manifest path is live
    Tool: bash
    Steps: rg -n "BUNDLED_MANIFEST_PATH" gateway/src/uloop_sync.rs && test -f gateway/assets/uloop-manifest.json
    Expected: path points to an existing tracked file.
    Evidence: evidence/task-4-uloop-path.txt

  Scenario: Deleted seed fallback is gone
    Tool: bash
    Steps: '! rg -n "seeds/uloop-manifest" gateway/src README.md docs'
    Expected: command exits 0.
    Evidence: evidence/task-4-no-seed-fallback.txt
  ```

  **Commit**: YES | Message: `fix(gateway): restore uloop manifest fallback path` | Files: `gateway/src/uloop_sync.rs`, `gateway/assets/uloop-manifest.json`, tests if present

- [x] 5. OpenCode Install Topology Decision

  **What to do**: Choose one OpenCode runtime target and make all installer/docs/tests agree. Recommended default: `lux init` owns the verified source adapter install at `.opencode/plugins/lux-plugin.ts`, while the older `lux bridge install` template bundle is either migrated to the same target or explicitly marked legacy and removed from the default bridge install path. Add targeted coverage proving the chosen install surface uses `adapters/opencode/lux-plugin.ts`.
  **Must NOT do**: Do not keep two default OpenCode plugin targets that both claim to be current.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 6, 8 | Blocked By: 1

  **References**:
  - Install implementation: `gateway/src/main.rs:2607`.
  - Legacy template install implementation: `gateway/src/main.rs:5545`.
  - Legacy template README: `gateway/src/templates/plugin/README.md:14`.
  - Doctor hints: `gateway/src/lux_doctor.rs:401` and `gateway/src/lux_doctor.rs:615`.
  - Source adapter: `adapters/opencode/lux-plugin.ts`.

  **Acceptance Criteria**:
  - [ ] Exactly one default OpenCode install target is documented and tested.
  - [ ] Test or smoke verifies the chosen runtime plugin target is copied from `adapters/opencode/lux-plugin.ts`, or the legacy template bundle is explicitly labeled non-default with a migration note.
  - [ ] `rg -n "plugins/opencode" gateway README.md docs adapters` returns no active install instructions.
  - [ ] Failure when source adapter is missing is explicit in stderr and does not claim success.

  **QA Scenarios**:
  ```text
  Scenario: Init installs OpenCode adapter from new source path
    Tool: bash
    Steps: cd gateway && cargo run -- init --project-path /tmp/lux-adapter-smoke --no-interactive
    Expected: chosen runtime plugin target exists and matches the documented source adapter content.
    Evidence: evidence/task-5-opencode-install.txt

  Scenario: Dual runtime targets are not both current
    Tool: bash
    Steps: rg -n ".opencode/plugins/lux-plugin.ts|.opencode/plugins/lux/" gateway README.md docs adapters
    Expected: output shows one default target and labels any other path legacy/non-default.
    Evidence: evidence/task-5-opencode-targets.txt
  ```

  **Commit**: YES | Message: `test(gateway): cover OpenCode adapter install path` | Files: `gateway/src/main.rs`, `gateway/tests/*` if needed

- [x] 6. Unity Bridge Install Topology Repair

  **What to do**: Align bridge source and target layouts. Recommended default: source from `bridge/unity/AiBridgeEditor` and `bridge/unity/LuxBridgeSettings.cs`; install into the documented `Assets/Editor/LuxBridge/` target, with idempotent migration/removal for legacy `Assets/Editor/AiBridgeEditor/`. Add a temp Unity-project-shaped smoke that runs `lux bridge install --project-path` and asserts exact copied files.
  **Must NOT do**: Do not silently skip missing bridge sources and still report a successful bridge install.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1, 5

  **References**:
  - Current source lookup: `gateway/src/main.rs:5505`.
  - Current source dirs/files: `gateway/src/main.rs:5510`.
  - Current target: `gateway/src/main.rs:5515`.
  - Documented target: `bridge/unity/README.md:31`.
  - Actual bridge files: `bridge/unity/AiBridgeEditor/UnityAiBridge.cs`, `bridge/unity/LuxBridgeSettings.cs`.

  **Acceptance Criteria**:
  - [ ] `lux bridge install --project-path <temp-unity-project>` copies bridge scripts from `bridge/unity/` into the documented target layout.
  - [ ] Re-running the command converges without duplicate files and without deleting non-Lux project files.
  - [ ] Missing bridge source produces a nonzero or explicitly degraded result; no silent success.
  - [ ] OpenCode plugin installation behavior during `bridge install` matches Task 5's topology decision.

  **QA Scenarios**:
  ```text
  Scenario: Bridge install copies from actual source layout
    Tool: bash
    Steps: create /tmp/lux-unity-smoke with Assets/; cd gateway && cargo run -- bridge install --project-path /tmp/lux-unity-smoke
    Expected: documented bridge target exists and contains UnityAiBridge.cs plus LuxBridgeSettings.cs; no root-level bridge/AiBridgeEditor dependency is required.
    Evidence: evidence/task-6-bridge-install.txt

  Scenario: Bridge install is idempotent
    Tool: bash
    Steps: run the same bridge install command twice and compare file list/checksums under Assets/Editor
    Expected: second run exits 0 and produces no duplicate bridge tree.
    Evidence: evidence/task-6-bridge-idempotent.txt
  ```

  **Commit**: YES | Message: `fix(gateway): align Unity bridge install layout` | Files: `gateway/src/main.rs`, `gateway/tests/*`, `bridge/unity/README.md`

- [x] 7. Submodule and Package Hygiene

  **What to do**: Resolve repository ownership contradictions. Decision for this stabilization pass: keep `Skills/` as a tracked source tree, not a broken submodule, because current HEAD already tracks its files and standalone source checkouts must work without recursive submodule setup. Remove the `Skills` entry from `.gitmodules` or convert the tree fully only if packaging tests prove recursive submodules are enforced. Keep `bridge` as a real submodule and document/check its initialization requirement.
  **Must NOT do**: Do not leave `.gitmodules` claiming `Skills` is a submodule while `git ls-tree HEAD Skills` is a normal tree.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1

  **References**:
  - Contradiction: `.gitmodules:4` declares `Skills`.
  - Current tree fact: `git ls-tree HEAD Skills` returns `040000 tree`.
  - Real submodule fact: `git ls-tree HEAD bridge` returns `160000 commit`.
  - Bridge source docs: `bridge/unity/README.md`.

  **Acceptance Criteria**:
  - [ ] `git config -f .gitmodules --get-regexp '^submodule\\.Skills\\.'` returns no entry if `Skills` remains tracked.
  - [ ] `git submodule status bridge` is documented and checked in setup docs.
  - [ ] `README.md` or `docs/usage.md` tells fresh checkout users whether `git submodule update --init bridge` is required.

  **QA Scenarios**:
  ```text
  Scenario: Skills ownership is not contradictory
    Tool: bash
    Steps: git ls-tree HEAD Skills && '! git config -f .gitmodules --get-regexp "^submodule\\.Skills\\."'
    Expected: Skills is a normal tree and .gitmodules has no Skills entry.
    Evidence: evidence/task-7-skills-ownership.txt

  Scenario: Bridge submodule requirement is observable
    Tool: bash
    Steps: git ls-tree HEAD bridge && git submodule status bridge
    Expected: bridge remains a gitlink and docs/setup mention initialization or packaged fallback.
    Evidence: evidence/task-7-bridge-submodule.txt
  ```

  **Commit**: YES | Message: `chore(repo): clarify source tree and submodule ownership` | Files: `.gitmodules`, `README.md`, `docs/usage.md` as needed

- [x] 8. Project Structure and Baseline Verification Report

  **What to do**: Run the full local verification surface after Tasks 1-7. Store concise outputs under `evidence/` and update `docs/roadmap-reality-lock.md` only if results changed. Include `scripts/check-project-structure.sh` in the command matrix and update it to match the chosen topology before running.
  **Must NOT do**: Do not mark roadmap automation complete because builds pass.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 9, 10 | Blocked By: 2, 3, 4, 5, 6, 7

  **References**:
  - Existing verification claim: `docs/roadmap-reality-lock.md:81`.
  - Required commands from `AGENTS.md`: `cd gateway && cargo build && cargo test`; `cd gateway/ui-src && npx tsc --noEmit`.
  - Structure check stale root: `scripts/check-project-structure.sh:13`.

  **Acceptance Criteria**:
  - [ ] `bash scripts/check-project-structure.sh` exits 0 against the chosen topology.
  - [ ] `cd gateway && cargo build` exits 0.
  - [ ] `cd gateway && cargo test` exits 0, or the only failure is documented as pre-existing with test name and reason.
  - [ ] `cd gateway/ui-src && npx tsc --noEmit` exits 0.
  - [ ] `bash Skills/tools/validate-skills.sh` has evidence output.

  **QA Scenarios**:
  ```text
  Scenario: Rust and TypeScript baseline is known
    Tool: bash
    Steps: bash scripts/check-project-structure.sh; (cd gateway && cargo build); (cd gateway && cargo test); (cd gateway/ui-src && npx tsc --noEmit)
    Expected: commands pass or only the named pre-existing Rust failure remains.
    Evidence: evidence/task-8-baseline-verification.txt

  Scenario: Policy/path scan catches stale references
    Tool: bash
    Steps: rg -n "plugins/opencode|seeds/uloop-manifest|bridge-threejs|Skills/.claude/skills|YAML 시드 데이터" README.md docs gateway Skills adapters
    Expected: no stale active references.
    Evidence: evidence/task-8-stale-reference-scan.txt
  ```

  **Commit**: YES | Message: `test: record trajectory hardening baseline` | Files: `docs/roadmap-reality-lock.md`, `evidence/*`

- [x] 9. Trajectory Recommendation Artifact

  **What to do**: Add a concise section to `docs/roadmap-reality-lock.md` that gives an explicit continue/revert/split recommendation for each major axis: `Skills`, `adapters`, `bridge`, deleted `seeds`, roadmap docs, and M1-M6 autonomy. Recommended default: continue the trajectory, but split it into a hardening gate before autonomy work; do not revert the cleanup unless a deleted artifact is proven required by a supported surface.
  **Must NOT do**: Do not leave "trajectory" as an implicit narrative only in git history.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 10 | Blocked By: 8

  **References**:
  - Latest trajectory commits from `git log --oneline -n 5`.
  - Reality lock doc: `docs/roadmap-reality-lock.md:1`.
  - Public roadmap boundary: `README.md:505`.

  **Acceptance Criteria**:
  - [ ] `docs/roadmap-reality-lock.md` contains explicit "Continue", "Split", or "Revert" recommendation per axis.
  - [ ] Recommendation says M1-M6 implementation waits until the hardening gate is green.
  - [ ] Deleted artifacts have a rollback rule: restore only if a supported test/surface needs them.

  **QA Scenarios**:
  ```text
  Scenario: Trajectory decision is explicit
    Tool: bash
    Steps: rg -n "Continue|Split|Revert|Skills|adapters|bridge|seeds|M1|M6" docs/roadmap-reality-lock.md
    Expected: every major axis has an actionable recommendation.
    Evidence: evidence/task-9-trajectory-recommendation.txt

  Scenario: Cleanup rollback policy is evidence-based
    Tool: bash
    Steps: rg -n "restore|rollback|supported surface|evidence" docs/roadmap-reality-lock.md
    Expected: deleted artifact recovery is tied to failing supported surfaces, not speculation.
    Evidence: evidence/task-9-rollback-policy.txt
  ```

  **Commit**: YES | Message: `docs: state trajectory recommendation and rollback policy` | Files: `docs/roadmap-reality-lock.md`

- [x] 10. Release-Readiness Gate

  **What to do**: Add or update one release checklist/gate document that says the current trajectory is safe to publish only after Tasks 1-9 pass. The gate must explicitly say M1-M6 autonomy is still future work.
  **Must NOT do**: Do not add marketing language that overstates verified multi-engine automation.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Final Verification | Blocked By: 8, 9

  **References**:
  - Roadmap future states: `docs/roadmap-reality-lock.md:25`.
  - Public capability table: `README.md` Engine Capability Snapshot.
  - ADR owner rule: `docs/adr/ADR-001-Gateway-as-Execution-Owner.md:15`.

  **Acceptance Criteria**:
  - [ ] Checklist references exact commands and evidence files from Task 8.
  - [ ] Checklist states "do not start M1-M6 implementation until trajectory hardening gate is green".
  - [ ] Public docs continue to separate verified, scaffolded, planned, and experimental capabilities.

  **QA Scenarios**:
  ```text
  Scenario: Release gate is actionable
    Tool: bash
    Steps: rg -n "cargo build|cargo test|npx tsc --noEmit|validate-skills|M1|M6" docs README.md
    Expected: release gate names commands and autonomy boundary.
    Evidence: evidence/task-10-release-gate.txt

  Scenario: Capabilities are not overstated
    Tool: bash
    Steps: rg -n "verified|scaffolded|planned|experimental" README.md docs/roadmap-reality-lock.md docs/godot-support.md
    Expected: maturity labels remain visible in public docs.
    Evidence: evidence/task-10-capability-labels.txt
  ```

  **Commit**: YES | Message: `docs: add trajectory hardening release gate` | Files: `docs/roadmap-reality-lock.md`, `README.md` or `docs/usage.md`

## Final Verification Wave
> ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
- [x] F1. Plan Compliance Audit: verify every task acceptance criterion has evidence.
- [x] F2. Code Quality Review: inspect changed Rust, shell, and docs for stale paths, silent fallback, and SSoT violations.
- [x] F3. Real Manual QA: run CLI smoke for adapter install against a temp project and verify generated files on disk.
- [x] F4. Scope Fidelity Check: confirm no M1-M6 autonomy implementation was added.

## Commit Strategy
- Prefer 4 atomic commits: docs reality lock, path/fallback fixes, test/smoke coverage, release gate.
- Do not squash unrelated changes from other agents.
- Do not push until Task 8 and Final Verification pass or known pre-existing failures are named.

## Success Criteria
- The repository tells one coherent story: gateway-owned local-first harness, `adapters/` source adapters, tracked `Skills/skills`, bridge submodule, `.lux/` runtime SSoT.
- No production code or public docs reference deleted seed/plugin/dist paths as active source.
- Adapter and skill installation/validation have executable coverage.
- The current baseline is evidence-backed, including any pre-existing failure.
