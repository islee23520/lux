# Learnings

## [2026-05-15] Session Start

### Key Source Locations
- `gateway/src/lux_run_state.rs`: Canonical RunStatus (L13-32), RunState fields (L91-109), no-fallback load (L153-176), migration hook (L199-251)
- `gateway/src/lux_continuation_state.rs`: Legacy ContinuationStatus/ContinuationState (L9-31), save() to be removed (L33-69)
- `gateway/src/templates/plugin/continuation-state-client.ts`: Direct fs.writeFileSync — must be replaced (L38-69)
- `gateway/src/templates/plugin/continuation-injector.ts`: DEFAULT_MAX_CONTINUATIONS=10 (wrong), cooldown/in-flight guard (L26-35, L149-153)
- `gateway/src/templates/plugin/stop-evaluator.ts`: StopReason typo `max_continations`, DEFAULT_STOP_CONFIG.maxContinuations=50 (L4-31)
- `gateway/src/lux_roadmap.rs`: RoadmapPhaseStatus, RoadmapPhase, non-atomic save (L125-143)
- `gateway/src/lux_ticket.rs`: atomic write pattern, Ticket/TicketStatus, blocker fields (L11-23, L107-147)
- `gateway/src/lux_verification.rs`: verify_t3_gate, required_tier_for_action push→T2Bridge must→T3 (L211-224), verification failure→blocker (L278-307)
- `gateway/src/lux_team_profile.rs`: team profile schema, team size presets (L1-4, L14-24, L142-151, L166-212)
- `gateway/src/lux_run.rs`: execute_task, executor metadata, team-mode kind, dispatch projection (L251-310, L277-285, L297-307)
- `gateway/src/lux_loop.rs`: LoopState, ApprovalGate, DEFAULT_MAX_ITERATIONS=10 (L18-20, L141-175)
- `gateway/src/server.rs`: router with GET/PUT /continuation/state (L851-910), verify and spec-loop routes (L888-905)

### Deterministic Status Mapping (ContinuationStatus → RunStatus)
- `Complete` → `Completed`
- `Active` + inFlight=true → `ExecutingTicket`
- `Active` + current_ticket_id != null → `ExecutingTicket`
- `Active` (neither) → `Planning`
- `Stopped` + stop_reason in {all_complete, milestone_complete} → `Completed`
- `Stopped` (other) → `Interrupted`
- `Error` + stop_reason in {blocker_cycle_detected, blocker_escalation_required} → `Quarantined`
- `Error` (other) → `Failed`
- `Idle` → `Idle`
- `Recovering`, `AwaitingPlayStart` → native RunStatus only, do NOT collapse to legacy

### Canonical Stop Reason Spellings
- max_continuations_reached (NOT max_continations)
- max_iterations_reached
- stagnation_limit
- consecutive_failure_limit
- milestone_complete
- blocker_escalation_required
- blocker_cycle_detected

### Numeric Defaults
- max_continuations = 50 (canonical, stop-evaluator.ts), NOT 10 (continuation-injector.ts is wrong)
- max_blocker_depth = 3
- max_blocker_attempts_per_ticket = 3
- max_consecutive_blocker_generations = 2
- stagnation_limit = 3
- consecutive_failure_limit = 3
- DEFAULT_MAX_ITERATIONS = 10 (Rust loop)

### Critical Patterns
- Atomic write pattern: use temp-file + rename (see lux_ticket.rs L11-23)
- No silent fallback: missing/corrupt .lux/run-state.json must error, not return default Idle
- Single writer: Gateway only writes .lux/ state; no plugin or team-mode direct file writes
- Optimistic concurrency: API writes need expected_seq + expected_status; conflict on stale writes
- Dispatch ≠ Done: dispatch sets ExecutingTicket/Dispatched, Done requires accepted execution+verification evidence
- Transaction journal: multi-file ops → .lux/runs/<run_id>/transactions/*.json
- Blocker idempotency: derive stable key from check category/name/spec_ref; reuse existing open blocker

## [Task 1] Completed
- `RunState::load()` and `RunState::save()` now validate persisted `status` through `RunStatus::from_str()`; invalid legacy strings such as `Active` are rejected before disk write.
- Legacy continuation migration now deterministically maps `Complete`/`Active`/`Stopped`/`Error`/`Idle` into canonical `RunStatus`, preserves continuation counters, and only renames the legacy file after post-write reload validation succeeds.
- Canonical stop reason spellings and continuation numeric defaults live in `lux_run_state.rs` as typed Rust contract values for downstream tasks.
- `cargo test lux_run_state` and `cargo test lux_continuation_state` pass; targeted evidence was written under `.sisyphus/evidence/task-1-*.txt`.

## [Task 3] Completed roadmap pushed milestone schema
- Added `RoadmapPhaseStatus::Pushed` with explicit pushed evidence fields on `RoadmapPhase`.
- Pushed phases now require non-empty `pushed_at`, `push_git_sha`, and `push_evidence_path`; errors name the missing field.
- `RoadmapReality::save()` now writes `.lux/roadmap.json` through `*.json.tmp` plus `fs::rename` after validation, keeping `.lux/roadmap.json` as SSoT and README as projection only.
- Targeted roadmap tests pass with `cargo test --test lux_roadmap_test lux_roadmap`; broad `cargo test lux_roadmap` is blocked by pre-existing `lux_run_state_test` compile drift.

## [Task 6] Completed team-mode producer-only enforcement
- Added `TaskStatus::AwaitingEvidence` variant to `lux_task_dag.rs`; not counted as Done for dependency resolution.
- `execute_task` in `lux_run.rs` now sets `AwaitingEvidence` instead of `Done` — dispatch no longer marks nodes complete.
- Added 4 producer endpoints in `lux_api.rs`: `post_proposal`, `post_evidence`, `post_blocker_resolution_request`, `post_milestone_push_request`; all write to `.lux/runs/<id>/<subdir>/` only.
- Added `accept_evidence` endpoint — sole path to transition `AwaitingEvidence → Done`; requires operator invocation.
- Tests in `gateway/tests/lux_run_test.rs` rewritten to use real `LuxLockGuard` via `acquire_lux_lock(..., force: true)`; `TeamSizePreset::Solo` does not exist — use `Small`.
- `cargo build` clean; `cargo test --test lux_run_test` 3 passed, 0 failed.
- Evidence files written to `.sisyphus/evidence/task-6-*.txt`.

## [Task 4] Completed
- Added RunState blocker metadata SSoT fields: `blocker_attempts`, `consecutive_blocker_generations`, and `blocker_depth`; `start_run` resets them for fresh runs.
- Stable verification blocker identity is derived from `check_category + check_name + spec_ref`; `create_or_update_blocker` updates same-key blockers instead of creating duplicates.
- Verification blocker creation now checks max depth 3, max attempts 3, max consecutive new blocker generations 2, and quarantines via `RunState::save()` with `blocker_cycle_detected` or `blocker_escalation_required`.
- Ticket blocker relationships are checked for cycles before linking; `TaskDAG::topological_ids_checked()` rejects cycles explicitly instead of using the old fallback ordering.
- Verification evidence: `cargo test --test lux_run_state_test`, `cargo test --test lux_verification_test`, `cargo test --test lux_run_test`, `cargo test --test lux_ticket_test`, and `cargo build` passed in one chained run.

## [Task 5] Completed
- `required_tier_for_action("milestone_push")` now requires `VerificationTier::T3Gate`; general `push` remains T2.
- T3 gate now requires all T2 checks to pass before invoking Unity batchmode compile and scene smoke.
- Unity compile uses explicit `T3_COMPILE_TIMEOUT_SECS = 600`; scene smoke uses `T3_SCENE_SMOKE_TIMEOUT_SECS = 300`.
- Unity executable discovery treats empty or missing executable paths as a hard failure with `Unity executable unavailable; milestone push blocked`.
- Scene smoke fails on exit failure, timeout, or any case-insensitive `error` in stderr/log; logs and stdout/stderr evidence are recorded under `.lux/verification/t3/<domain>/`.
- Targeted verification passed: `cargo test --test lux_verification_test`; build passed: `cargo build`.

## [Task 2] Completed
- All direct `fs.writeFileSync` calls to `.lux/continuation-state.json` in the plugin replaced with async HTTP PUT to `PUT /api/lux/continuation/state`.
- `writeContinuationState(opts: ContinuationStateWriteOptions, state: ContinuationState): Promise<ContinuationWriteResult>` — opts carries `{ projectPath, gatewayUrl, expectedSeq }`.
- `ContinuationOrchestrator` tracks `lastKnownSeq: number = 0`; updates after each successful write via `result.seq`.
- Server handler acquires `continuation_write_lock` mutex, calls `RunState::update_with_seq_check`, returns HTTP 409 on seq conflict (error string contains "seq conflict").
- `RunState::update_with_seq_check(project_path, expected_seq, expected_status, patch)` validates seq, applies patch, increments seq, saves atomically.
- vitest: 343/343 passed (28 test files); `cargo test --test lux_run_state_test`: 17/17 passed (3 new seq-check tests added); `cargo build` clean; `npx tsc --noEmit` 0 errors.
- Evidence files: `.sisyphus/evidence/task-2-plugin-api-write.txt`, `task-2-server-canonical-write.txt`, `task-2-stale-seq-conflict.txt`.

## Task 9: Documentation Updates for Autonomous Milestone Continuation
- Updated README.md to clarify .lux/roadmap.json as the SSoT.
- Documented T3 Unity verification requirement for milestone push (batchmode compile + scene smoke).
- Documented deprecation of legacy .lux/continuation-state.json.
- Clarified that team-mode/hyperplan is producer-only and cannot write .lux/ state directly.
- Updated gateway/src/templates/plugin/README.md:
    - maxContinuations increased from 10 to 50.
    - Documented canonical stop reasons (max_continuations_reached, spec_satisfied, manual_intervention, stagnation_detected).
    - Documented legacy state deprecation.
- Verified that forbidden commands (ralph, start-work) and typos (max_continations) are not present in the updated documentation.

## [Task 7] Completed spec→ticket→verification→milestone lifecycle
- `RunStatus::AwaitingEvidence` now prevents `complete_run` from prematurely marking dispatch-only work complete; team-mode dispatch remains producer-only until evidence is accepted.
- Milestone push approval is explicit: `begin_milestone_push_approval` requires existing T3 evidence and sets `AwaitingApproval`, `ApproveDiff`, `pending_transition=milestone_push`, and `awaiting_since` while incrementing `seq`.
- Multi-file run-state + roadmap writes use `.lux/runs/<run_id>/transactions/<uuid>.json` journals with planned/committed/rolled_back status and operation pre-images for recovery.
- Gateway startup runs pending transaction recovery from `GatewayState::new`; planned journals are reapplied idempotently or rolled back on apply failure.
- Full lifecycle tests cover success, verification failure/blocker creation, and transaction recovery in `gateway/tests/lux_lifecycle_test.rs`.
- Existing ambiguity domain keys were normalized to spec field names (`art_style`, `ui_ux`) after full-suite tests exposed stale hyphenated keys.
- Verification passed: `cd gateway && cargo test`.

## Task 10 — Wave 4 End-to-End Evidence Bundle (2026-05-15)

### Verification Results
- `cargo test --workspace`: ALL PASS (0 failures). Previously noted pre-existing failure
  `test_ambiguity_schell_phases` now PASSES — no longer a blocker.
- `plugin npm run test:run`: 344/344 PASS (28 test files, 2.61s)
- `ui-src tsc --noEmit`: PASS (exit 0)
- `ui-src npm test`: 28/28 PASS (7 test files, vitest)
- `policy-scan.mjs --advisory-only`: exit 0 (Critical: 0, Warning: 48, Advisory: 154)
- `e2e-lux-sequential-smoke.sh --quick`: ALL 12 steps PASS

### Unity T3 Status
- Unity executables ARE present at `/Applications/Unity/Hub/Editor/6000.0.75f1/` and `6000.3.13f1/`
- T3 HARD FAIL: no target Unity project available → cannot run batch compile or scene smoke
- Milestone push remains BLOCKED until T3 is satisfied with a real Unity project

### AwaitingApproval Gate
- After T3 passes: RunStatus → AwaitingApproval with approval.gate=ApproveDiff,
  approval.pending_transition=milestone_push
- Remote push NOT executed until explicit user approval
- Upon approval: push → roadmap Pushed → RunStatus::Completed + stop_reason=milestone_complete

### Commands That Work
- `cargo test --workspace` (from gateway/) — all tests pass including ambiguity tests
- `bash scripts/e2e-lux-sequential-smoke.sh --quick` — 12-step sequential smoke, all pass
- `node scripts/policy-scan.mjs --advisory-only` — advisory-only exit 0
