# ADR-002: .lux/run-state.json as Active Run Single Source of Truth

## Status
Accepted (Phase 1, 2026-05-14)

## Context
State was split across 3 locations:
1. Server in-memory (LoopOrchestrator in Arc<RwLock<...>>)
2. Plugin `.lux/continuation-state.json` (current_ticket_id, inFlight, continuationCount)
3. Proposed `.lux/run-state.json`

Two+ files that must agree = eventual inconsistency. Pause on one layer doesn't affect others.

## Decision
`.lux/run-state.json` is the canonical durable state for active execution runs.

Rules:
- Gateway is the ONLY writer (atomic write + schema version + monotonic seq)
- Every persisted save increments `seq`; transitions that accept an expected seq MUST reject stale writers instead of overwriting newer state.
- All in-memory state is a derived cache from this file
- Plugin MUST NOT directly mutate this file or continuation-state.json for run state
- Plugin results go through gateway API or inbox (.lux/runs/<run_id>/inbox/*)
- Missing run-state.json after init = explicit error or initialization path; NO silent fallback
- One active run only for MVP (no parallel multi-run until state model proven)

M6 autonomous execution extends the active run lifecycle with these durable states:
`planned`, `dispatch_ready`, `ExecutingTicket` (displayed as "executing"), `Verifying`, `Blocked`, `RetryReady` (displayed as "retry_ready"), `Resumed` (displayed as "resumed"), `Completed`, `Failed`, and `Quarantined`. The legacy M5 states remain
valid for migration and existing gateway surfaces; M6 states add explicit dispatch,
executor, verification, retry/resume, and quarantine checkpoints without introducing
a second source of truth.

Minimal MVP shape:
```json
{
  "schema_version": 1,
  "seq": 0,
  "run_id": "",
  "status": "idle",
  "goal_id": null,
  "milestone_id": null,
  "ticket_id": null,
  "current_ticket_id": null,
  "approval": { "gate": null, "pending_transition": null, "created_at": null },
  "resume": { "previous_status": null, "checkpoint": null },
  "executor": { "kind": null, "job_id": null, "heartbeat_at": null },
  "verification_policy": null,
  "stop_reason": null,
  "continuation_count": 0,
  "blocker_attempts": 0,
  "continuation_config": null,
  "awaiting_since": null,
  "last_error": null,
  "updated_at": "",
  "planned_at": null,
  "dispatch_ready_at": null,
  "executing_at": null,
  "verifying_at": null,
  "blocked_at": null,
  "retry_ready_at": null,
  "resumed_at": null,
  "completed_at": null,
  "failed_at": null,
  "quarantined_at": null
}
```

Migration: On gateway startup, if .lux/continuation-state.json exists, log warning, attempt atomic migration to run-state.json, mark old file as deprecated. No dual-write period.

## Consequences
- Single durable truth for run lifecycle
- Pause/resume works across all surfaces (they all re-read run-state.json)
- Plugin continuation-state.json becomes legacy (migration path, then ignored)
- Resolves VV#10 (split-brain), enables VV#4 (StartPlay deadlock ground)
