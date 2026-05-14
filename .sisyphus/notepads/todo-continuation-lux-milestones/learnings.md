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
