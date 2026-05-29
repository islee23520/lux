# Skills Release Metadata Gate Plan

## TL;DR
> **Summary**: The next goal is to make `Skills/skills` release-valid: `bash Skills/tools/validate-skills.sh` must exit 0 against the tracked skill tree, with no absolute-path skill symlinks and no oversized `SKILL.md` files. This should happen before any M1-M6 autonomy work or packaging claim.
> **Deliverables**:
> - A green skill validation gate for `Skills/skills`.
> - Normalized `category`/`source` metadata for legacy skills.
> - A standalone-safe `ldp-decision-protocol` skill file, not an absolute local symlink.
> - A split `unity-cs-reference` skill with large reference material moved under `references/`.
> - Evidence files proving validator, CLI skill discovery, and release docs are aligned.
> **Effort**: Medium
> **Parallel**: YES - 3 waves
> **Critical Path**: Task 1 -> Task 2 -> Task 4 -> Task 5 -> Task 8

## Context
### Original Request
User asked: `$omo:planing-prometheustic what should be next goal than do make next plan with current setp`.

### Interview Summary
No blocking interview is needed. The previous trajectory-hardening gate was complete enough to reveal the next bottleneck: skill validation was intentionally nonzero because existing skill metadata was invalid.

### Metis Review (gaps addressed)
A local Metis-style review found that the obvious next goal, "add missing frontmatter," is too narrow. The plan also addresses:
- `ldp-decision-protocol/SKILL.md` is currently a symlink to `/Users/ilseoblee/workspace/linalab/ldp/SKILL.md`, which is not standalone-safe.
- The symlink target declares `name: lina-decision-protocol`, causing a directory/name mismatch.
- `unity-cs-reference/SKILL.md` is 4999 lines, violating `Skills/SKILL-SCHEMA.md` max 500-line rule.
- Gateway CLI skill discovery must keep working after metadata and reference-file moves.
- Release docs must stop saying skill validation is explicitly nonzero once the gate is green.

## Work Objectives
### Core Objective
Make the tracked `Skills/skills` tree pass the canonical skill validator and remain consumable by the gateway CLI/API.

### Deliverables
- `Skills/tools/validate-skills.sh` exits 0 by default against `Skills/skills`.
- Every `Skills/skills/*/SKILL.md` has valid frontmatter: `name`, `description`, `category`, `source`.
- Every declared `name` matches its directory name.
- No `Skills/skills/*/SKILL.md` is an absolute symlink.
- No `SKILL.md` exceeds 500 lines; large reference content lives under `references/`.
- Gateway skill list/info smoke tests pass after the tree changes.
- Roadmap release gate docs reflect skill validation passing.

### Definition of Done
- `bash Skills/tools/validate-skills.sh` exits 0.
- `find Skills/skills -mindepth 2 -maxdepth 2 -name SKILL.md -type l -exec test ! -e {} \; -print` finds no broken symlink; a dedicated check proves no absolute SKILL symlink remains.
- `awk`/validator evidence proves no `SKILL.md` exceeds 500 lines.
- `(cd gateway && cargo test --test gateway_cli_smoke skill_)` exits 0.
- `bash scripts/check-project-structure.sh` exits 0.
- A stale failure-phrase scan across active docs and plans returns no release-gate claim that skill validation is still failing, except historical evidence files.

### Must Have
- Keep `Skills/skills` as the tracked source tree.
- Preserve existing skill names as public identifiers, except `ldp-decision-protocol` must match its directory.
- Preserve the Unity C# reference content by moving it to `references/`, not deleting it.
- Treat current `evidence/task-8-skills-validate.txt` as the RED baseline.

### Must NOT Have
- Do not start M1-M6 autonomy implementation.
- Do not make validator silently pass by weakening required fields or line limits.
- Do not keep absolute local symlinks in tracked skill source.
- Do not delete large reference content just to satisfy line count.
- Do not change gateway runtime semantics except tests/smokes needed to prove skill discovery still works.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: RED-GREEN using existing failing validator output as RED, plus targeted shell smokes and existing Rust CLI skill tests.
- QA policy: Every task has agent-executed scenarios with command evidence.
- Evidence: `evidence/skills-gate-task-{N}-{slug}.txt`.

## Execution Strategy
### Parallel Execution Waves
Wave 1: Task 1, Task 2
Wave 2: Task 3, Task 4, Task 5, Task 6
Wave 3: Task 7, Task 8, Task 9

### Dependency Matrix
| Task | Depends On | Blocks |
| --- | --- | --- |
| 1. RED baseline inventory | none | 2, 3, 4, 5, 8 |
| 2. Metadata taxonomy lock | 1 | 3, 4, 8 |
| 3. Legacy skill frontmatter normalization | 1, 2 | 8 |
| 4. LDP skill standalone repair | 1, 2 | 8 |
| 5. Unity C# reference split | 1, 2 | 8 |
| 6. Validator standalone-source hardening | 1 | 8 |
| 7. Gateway skill discovery regression check | 3, 4, 5, 6 | 8, 9 |
| 8. Green skill validation gate | 3, 4, 5, 6, 7 | 9 |
| 9. Release docs/evidence refresh | 8 | Final Verification |

## TODOs
- [x] 1. RED Baseline Inventory

  **What to do**: Capture the current validator failure state from `Skills/skills` before edits. Parse failing skill names, missing fields, name mismatches, symlink status, and line-count violations into a concise evidence file.
  **Must NOT do**: Do not edit any skill file in this task.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 2, 3, 4, 5, 8 | Blocked By: none

  **References**:
  - Validator: `Skills/tools/validate-skills.sh` - canonical gate.
  - Schema: `Skills/SKILL-SCHEMA.md` - required frontmatter and max line count.
  - Existing RED evidence: `evidence/task-8-skills-validate.txt`.

  **Acceptance Criteria**:
  - [ ] `bash Skills/tools/validate-skills.sh` is captured with nonzero exit before edits.
  - [ ] Evidence lists all failing categories: missing `category`, missing `source`, name mismatch, line-count violation, and symlink status.

  **QA Scenarios**:
  ```text
  Scenario: Current skill gate is red for known reasons
    Tool: bash
    Steps: bash Skills/tools/validate-skills.sh; rg -n '^FAIL:' evidence/skills-gate-task-1-red.txt
    Expected: command exits nonzero and failures match missing metadata/name/line-count issues.
    Evidence: evidence/skills-gate-task-1-red.txt

  Scenario: Inventory names absolute symlink risk
    Tool: bash
    Steps: find Skills/skills -mindepth 2 -maxdepth 2 -name SKILL.md -type l -ls
    Expected: ldp symlink is recorded before repair if still present.
    Evidence: evidence/skills-gate-task-1-symlink.txt
  ```

  **Commit**: NO | Message: n/a | Files: evidence only

- [x] 2. Metadata Taxonomy Lock

  **What to do**: Decide and document the exact `category` and `source` values to apply. Use `category: workflow` and `source: lux` for legacy LUX workflow skills; use `category: reference` and `source: lux` for `unity-cs-reference`; preserve existing `studio` and `reference/unity-design-patterns-skills` values for already-valid skills. If schema examples are too narrow, update `Skills/SKILL-SCHEMA.md` examples to include observed valid source values without weakening required fields.
  **Must NOT do**: Do not invent per-skill categories beyond `reference`, `workflow`, and `studio` unless the schema is first updated with evidence.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 3, 4, 8 | Blocked By: 1

  **References**:
  - Schema examples: `Skills/SKILL-SCHEMA.md`.
  - Valid studio pattern: `Skills/skills/studio-help/SKILL.md` frontmatter.
  - Valid Unity pattern: `Skills/skills/unity-pattern-command/SKILL.md` frontmatter.
  - Legacy workflow pattern: `Skills/skills/architecture-decision/SKILL.md` body and trigger language.

  **Acceptance Criteria**:
  - [ ] A mapping table exists in evidence or docs listing every edited skill and chosen `category`/`source`.
  - [ ] Schema docs remain stricter than implementation: required fields and 500-line cap stay required.

  **QA Scenarios**:
  ```text
  Scenario: Metadata mapping is explicit
    Tool: bash
    Steps: cat evidence/skills-gate-task-2-taxonomy.txt
    Expected: every failing legacy skill has one chosen category/source pair.
    Evidence: evidence/skills-gate-task-2-taxonomy.txt

  Scenario: Schema still enforces required fields
    Tool: bash
    Steps: rg -n 'name|description|category|source|Max 500' Skills/SKILL-SCHEMA.md Skills/tools/validate-skills.sh
    Expected: required fields and max line-count rule remain visible.
    Evidence: evidence/skills-gate-task-2-schema.txt
  ```

  **Commit**: YES | Message: `docs(skills): lock release metadata taxonomy` | Files: `Skills/SKILL-SCHEMA.md`, evidence

- [x] 3. Legacy Skill Frontmatter Normalization

  **What to do**: Add missing `category` and `source` fields to legacy regular-file skills that already have valid `name`/`description`: `architecture-decision`, `architecture-review`, `bug-report`, `bug-triage`, `changelog`, `code-review`, `core-invariants`, `game-dev`, `lux-unity`, `perf-profile`, `regression-suite`, `release-checklist`, `retrospective`, `security-audit`, `smoke-check`, `tech-debt`, `test-helpers`, and `test-setup`. Use the Task 2 taxonomy.
  **Must NOT do**: Do not rewrite skill bodies or descriptions in this task.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1, 2

  **References**:
  - Failing list: `evidence/task-8-skills-validate.txt`.
  - Existing frontmatter style: `Skills/skills/studio-code-review/SKILL.md`.

  **Acceptance Criteria**:
  - [ ] Each listed skill frontmatter contains `category: workflow` and `source: lux` unless Task 2 records a different explicit decision.
  - [ ] Names continue to match directory names.

  **QA Scenarios**:
  ```text
  Scenario: Legacy workflow skills have required metadata
    Tool: bash
    Steps: for each listed skill, sed -n '1,8p' Skills/skills/<skill>/SKILL.md
    Expected: frontmatter includes name, description, category, source before closing delimiter.
    Evidence: evidence/skills-gate-task-3-frontmatter.txt

  Scenario: No body rewrite occurred
    Tool: bash
    Steps: git diff -- Skills/skills/<legacy>/SKILL.md | rg -n '^[-+](category|source):|^[-+][^-+]' 
    Expected: diffs are limited to frontmatter metadata lines for this task.
    Evidence: evidence/skills-gate-task-3-diff-scope.txt
  ```

  **Commit**: YES | Message: `chore(skills): add release metadata to workflow skills` | Files: `Skills/skills/*/SKILL.md`

- [x] 4. LDP Skill Standalone Repair

  **What to do**: Replace `Skills/skills/ldp-decision-protocol/SKILL.md` absolute symlink with a standalone tracked file. The file must declare `name: ldp-decision-protocol`, keep a concise description of the Linalab decision workflow, and include `category: workflow`, `source: lux`. Preserve useful local LDP wording by copying only the minimal skill hub content required for this repository; do not depend on `/Users/ilseoblee/workspace/linalab/ldp/SKILL.md` at runtime.
  **Must NOT do**: Do not keep an absolute symlink or declare `name: lina-decision-protocol` under the `ldp-decision-protocol` directory.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1, 2

  **References**:
  - Current diff: `git diff -- Skills/skills/ldp-decision-protocol/SKILL.md` shows absolute symlink.
  - Schema name rule: `Skills/tools/validate-skills.sh` declared-name check.
  - Previous tracked file content appears in git diff deletion hunk; use it as the local fallback pattern.

  **Acceptance Criteria**:
  - [ ] `test -f Skills/skills/ldp-decision-protocol/SKILL.md && test ! -L Skills/skills/ldp-decision-protocol/SKILL.md` exits 0.
  - [ ] `sed -n '1,8p' Skills/skills/ldp-decision-protocol/SKILL.md` shows `name: ldp-decision-protocol`, `category: workflow`, `source: lux`.
  - [ ] No absolute `/Users/.../ldp/SKILL.md` path remains in tracked skill source.

  **QA Scenarios**:
  ```text
  Scenario: LDP skill is standalone
    Tool: bash
    Steps: test -f Skills/skills/ldp-decision-protocol/SKILL.md && test ! -L Skills/skills/ldp-decision-protocol/SKILL.md
    Expected: exits 0.
    Evidence: evidence/skills-gate-task-4-standalone.txt

  Scenario: LDP declared name matches directory
    Tool: bash
    Steps: bash Skills/tools/validate-skills.sh | rg -n 'ldp-decision-protocol'
    Expected: no name mismatch for ldp-decision-protocol.
    Evidence: evidence/skills-gate-task-4-name.txt
  ```

  **Commit**: YES | Message: `fix(skills): make ldp skill standalone` | Files: `Skills/skills/ldp-decision-protocol/SKILL.md`

- [x] 5. Unity C# Reference Split

  **What to do**: Reduce `Skills/skills/unity-cs-reference/SKILL.md` below 500 lines by moving the long reference catalog into `Skills/skills/unity-cs-reference/references/`. Keep `SKILL.md` as a concise loader with required frontmatter, purpose, when-to-use, and instructions for consulting the reference file(s). Preserve all existing reference content in the new reference file(s).
  **Must NOT do**: Do not delete Unity C# API/reference material to satisfy the line cap.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1, 2

  **References**:
  - Oversize failure: `evidence/task-8-skills-validate.txt` line-count failure for `unity-cs-reference`.
  - Schema reference folder rule: `Skills/SKILL-SCHEMA.md` says use `references/` for supplementary content.
  - Existing skill pattern with references: inspect any skill directory that already has `references/`, if present; otherwise follow schema.

  **Acceptance Criteria**:
  - [ ] `wc -l < Skills/skills/unity-cs-reference/SKILL.md` is <= 500.
  - [ ] `find Skills/skills/unity-cs-reference/references -type f` lists the moved reference material.
  - [ ] `rg -n 'Unity|C#|API|reference' Skills/skills/unity-cs-reference/SKILL.md Skills/skills/unity-cs-reference/references` confirms content is still discoverable.

  **QA Scenarios**:
  ```text
  Scenario: Unity C# skill passes line cap without content loss
    Tool: bash
    Steps: wc -l Skills/skills/unity-cs-reference/SKILL.md Skills/skills/unity-cs-reference/references/*
    Expected: SKILL.md <= 500 and reference files contain the moved body.
    Evidence: evidence/skills-gate-task-5-line-count.txt

  Scenario: Skill instructions route to references
    Tool: bash
    Steps: rg -n 'references/' Skills/skills/unity-cs-reference/SKILL.md
    Expected: SKILL.md tells agents where the large reference material lives.
    Evidence: evidence/skills-gate-task-5-routing.txt
  ```

  **Commit**: YES | Message: `refactor(skills): split unity csharp reference content` | Files: `Skills/skills/unity-cs-reference/SKILL.md`, `Skills/skills/unity-cs-reference/references/*`

- [x] 6. Validator Standalone-Source Hardening

  **What to do**: Add a validator check that rejects absolute `SKILL.md` symlinks under `Skills/skills`. Keep relative symlinks out of default scope unless a real use case exists; recommended default is to require regular files for `SKILL.md` because this repo is a standalone app source tree.
  **Must NOT do**: Do not weaken field checks or line-count checks to make validation pass.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 8 | Blocked By: 1

  **References**:
  - Validator script: `Skills/tools/validate-skills.sh`.
  - Standalone-app invariant: `AGENTS.md` says LUX is standalone and source ownership must be explicit.

  **Acceptance Criteria**:
  - [ ] Temporary skill root with an absolute `SKILL.md` symlink fails validation with an explicit message.
  - [ ] Normal tracked skill root still validates after Tasks 3-5.

  **QA Scenarios**:
  ```text
  Scenario: Absolute SKILL symlink is rejected
    Tool: bash
    Steps: create temp SKILLS_ROOT with a skill dir whose SKILL.md is an absolute symlink; run SKILLS_ROOT=<tmp> bash Skills/tools/validate-skills.sh
    Expected: exits nonzero with explicit symlink/standalone-source failure.
    Evidence: evidence/skills-gate-task-6-absolute-symlink-redgreen.txt

  Scenario: Existing regular skill files continue to validate normally
    Tool: bash
    Steps: SKILLS_ROOT=Skills/skills bash Skills/tools/validate-skills.sh
    Expected: after Tasks 3-5, exits 0.
    Evidence: evidence/skills-gate-task-6-normal-root.txt
  ```

  **Commit**: YES | Message: `test(skills): reject absolute skill symlinks` | Files: `Skills/tools/validate-skills.sh`, evidence

- [x] 7. Gateway Skill Discovery Regression Check

  **What to do**: Verify the gateway still discovers and reports skills after metadata normalization and reference splitting. Run existing gateway skill list/info tests and add a targeted smoke only if current tests do not cover `unity-cs-reference` and `ldp-decision-protocol` after the changes.
  **Must NOT do**: Do not change gateway discovery semantics unless a failing test proves it cannot read the valid skill tree.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 8, 9 | Blocked By: 3, 4, 5, 6

  **References**:
  - Existing tests: `gateway/tests/gateway_cli_smoke.rs` skill-related tests.
  - Gateway core skill path: `gateway/src/main.rs` `core_skills_dir()`.
  - Server skill path: `gateway/src/server.rs` skill scope roots.

  **Acceptance Criteria**:
  - [ ] `(cd gateway && cargo test --test gateway_cli_smoke skill_)` exits 0.
  - [ ] CLI smoke can list or show `ldp-decision-protocol` and `unity-cs-reference` without following broken absolute paths.

  **QA Scenarios**:
  ```text
  Scenario: Gateway skill tests remain green
    Tool: bash
    Steps: cd gateway && cargo test --test gateway_cli_smoke skill_
    Expected: all filtered skill tests pass.
    Evidence: evidence/skills-gate-task-7-gateway-skill-tests.txt

  Scenario: CLI sees repaired skills
    Tool: bash
    Steps: cd gateway && cargo run -- skill info ldp-decision-protocol --json; cargo run -- skill info unity-cs-reference --json
    Expected: both commands exit 0 and return parseable JSON with matching names.
    Evidence: evidence/skills-gate-task-7-cli-info.txt
  ```

  **Commit**: YES | Message: `test(gateway): verify repaired skill discovery` | Files: `gateway/tests/gateway_cli_smoke.rs` only if new coverage is needed

- [x] 8. Green Skill Validation Gate

  **What to do**: Run the canonical skill validation gate after all skill repairs. Capture full output and update the release baseline evidence.
  **Must NOT do**: Do not mark this task complete if validator exits nonzero for any reason.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 9 | Blocked By: 3, 4, 5, 6, 7

  **References**:
  - Validator: `Skills/tools/validate-skills.sh`.
  - Previous release gate doc: `docs/roadmap-reality-lock.md` skill validation section.

  **Acceptance Criteria**:
  - [ ] `bash Skills/tools/validate-skills.sh` exits 0.
  - [ ] Output includes `PASS: summary - all skill checks passed`.
  - [ ] No skill failure remains in `rg -n '^FAIL:' evidence/skills-gate-task-8-validate.txt`.

  **QA Scenarios**:
  ```text
  Scenario: Canonical skill validation is green
    Tool: bash
    Steps: bash Skills/tools/validate-skills.sh
    Expected: exits 0 and summary pass is printed.
    Evidence: evidence/skills-gate-task-8-validate.txt
  ```

  **Commit**: YES | Message: `chore(skills): green skill validation gate` | Files: `Skills/skills/**`, `Skills/tools/validate-skills.sh`, evidence

- [x] 9. Release Docs and Baseline Refresh

  **What to do**: Update `docs/roadmap-reality-lock.md` and any relevant plan/evidence summaries so the release gate says skill validation is green, not explicit nonzero. Keep historical evidence files untouched unless they are regenerated by the current task.
  **Must NOT do**: Do not overstate skill quality beyond schema validity; passing metadata validation does not prove every skill is behaviorally complete.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Final Verification | Blocked By: 8

  **References**:
  - Current doc statement: `docs/roadmap-reality-lock.md` release gate and integrated verification evidence.
  - Previous plan: `plans/lux-trajectory-hardening.md` should remain historical unless a current active claim is stale.

  **Acceptance Criteria**:
  - [ ] Active docs say `bash Skills/tools/validate-skills.sh` passes.
  - [ ] Docs still distinguish schema validity from autonomous M1-M6 completion.
  - [ ] `bash scripts/check-project-structure.sh` exits 0 after doc/skill changes.

  **QA Scenarios**:
  ```text
  Scenario: Release gate reflects green skill validation
    Tool: bash
    Steps: rg -n 'validate-skills|PASS|M1|M6' docs/roadmap-reality-lock.md README.md docs/usage.md
    Expected: docs show validation as pass and still keep autonomy as future/planned.
    Evidence: evidence/skills-gate-task-9-docs.txt

  Scenario: Project structure remains coherent
    Tool: bash
    Steps: bash scripts/check-project-structure.sh
    Expected: exits 0.
    Evidence: evidence/skills-gate-task-9-structure.txt

  Scenario: No stale validation failure claims remain in active docs
    Tool: bash
    Steps: scan docs, README.md, and plans for stale release-gate phrases that claim skill validation is still failing.
    Expected: no active release-gate claim says skill validation is still failing.
    Evidence: evidence/skills-gate-task-9-stale-docs.txt
  ```

  **Commit**: YES | Message: `docs(skills): refresh release validation gate` | Files: `docs/roadmap-reality-lock.md`, `README.md` or `docs/usage.md` only if active claims are stale

## Final Verification Wave
> ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
- [x] F1. Plan Compliance Audit: verify Tasks 1-9 acceptance criteria have evidence and no task is skipped.
- [x] F2. Code Quality Review: inspect skill frontmatter, symlink handling, validator strictness, and reference split for accidental content loss.
- [x] F3. Real Manual QA: run `lux skill list` and `lux skill info` through the CLI against the repaired tree and capture output.
- [x] F4. Scope Fidelity Check: confirm no M1-M6 autonomy implementation or gateway runtime feature work was added.

## Commit Strategy
Recommended commits:
1. `docs(skills): lock release metadata taxonomy`
2. `chore(skills): add release metadata to workflow skills`
3. `fix(skills): make ldp skill standalone`
4. `refactor(skills): split unity csharp reference content`
5. `test(skills): reject absolute skill symlinks`
6. `docs(skills): refresh release validation gate`

Do not commit automatically unless explicitly requested. If committing later, include `Plan: plans/skills-release-metadata-gate.md` in the final commit footer for this plan's execution branch.

## Success Criteria
- `bash Skills/tools/validate-skills.sh` exits 0.
- `bash scripts/check-project-structure.sh` exits 0.
- `(cd gateway && cargo test --test gateway_cli_smoke skill_)` exits 0.
- `lux skill info ldp-decision-protocol --json` and `lux skill info unity-cs-reference --json` return matching names.
- Active docs no longer describe skill validation as an unresolved metadata failure.
- No absolute local skill symlink remains under `Skills/skills`.
