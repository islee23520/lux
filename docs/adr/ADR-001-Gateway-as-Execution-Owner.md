# ADR-001: Gateway as Execution Control-Plane Owner

## Status
Accepted (Phase 1, 2026-05-14)

## Context
LUX has multiple surfaces that could own execution lifecycle:
- CLI (`main.rs`) — ephemeral, cannot own durable state
- Legacy OpenCode adapter — orchestration was split from gateway ownership and is not active source in the server/MCP-only repository shape
- Rust Gateway (`server.rs`) — long-running HTTP/WS process with 120+ API and WebSocket routes, health/heartbeat, and MCP/API surfaces

Previous codebase had split-brain: LoopOrchestrator in gateway memory, ContinuationOrchestrator in plugin memory + continuation-state.json, proposed run-state.json as third state location.

## Decision
The **Rust Gateway** is the sole owner of execution control-plane lifecycle.
- **Control clients**: CLI, MCP clients, API callers → read-only or request mutations
- **Executor adapters**: OpenCode Plugin, Unity Bridge → receive assigned work, report results back
- **Gateway responsibilities**: Run scheduling, ticket selection, approval gates, pause/resume, state persistence, verification
- **Plugin demoted to**: Executor adapter with explicit `executeTicket(ticket_id, run_id)` contract. No autonomous scheduling.

## Consequences
- CLI commands become wrappers around gateway APIs
- Plugin autonomy reduced: executes assigned tickets only
- All durable run state written by gateway only
- MCP/API clients become the interactive control surface
- Resolves contradictions: VV#2 (CLI surface), VV#5 (parallel impossible), VV#6 (ownership), VV#10 (split-brain)

## Alternatives Considered
- CLI-first: Rejected — CLI is ephemeral, cannot own pause/resume/crash recovery
- Plugin-first: Rejected — hard-codes OpenCode, single inFlight makes it unsuitable as generic owner
- Hybrid distributed: Rejected — split-brain is the current bug, not the solution
