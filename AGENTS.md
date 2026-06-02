# LUX Local Automation Toolkit Agent Guide

LUX is a local-first AI automation toolkit for game projects. It operates as an independent server/MCP control plane that communicates with engine projects through installed bridge adapters and records runtime truth under `.lux/`.

Unity is the primary verified engine path. Godot and Three.js support must be described through explicit capability maturity tiers.
Planned or adapter-only features must not be presented as completed behavior.

## Codebase Structure

| Path | Description | Tech |
| :--- | :--- | :--- |
| `gateway/` | Rust CLI, Axum HTTP/WS server, and MCP-facing APIs | Rust, Axum 0.7 |
| `crates/` | Shared Rust core packages extracted from gateway responsibilities | Rust |
| `bridge/` | Engine bridge adapter files for auto-installation | C# Editor scripts, engine-specific adapters |
| `Skills/` | Core AI skills and references | Manifest + SKILL.md |
| `docs/` | Project documentation | Markdown |
| `scripts/` | Utility shell scripts | Bash/Zsh |

## Key Conventions

### Rust (`gateway/`)
- Use Axum 0.7, tokio 1, clap 4.5, anyhow, and serde.
- Error handling: Use `anyhow` for logic and `eprintln` for user output.
- No `TODO`, `FIXME`, or `HACK` comments.
- New endpoints must have tests in `server.rs` or `gateway_cli_smoke.rs`.
- Server lifecycle: idle timeout with graceful shutdown (`--idle-timeout`), heartbeat (`POST /api/heartbeat`), health (`GET /api/health`).

### Unity Bridge (`bridge/`)
- Contains the C# source for the Unity `AI Bridge` TCP server and protocol.
- These files are automatically installed into target Unity projects via `lux bridge install`.
- Maintain compatibility with Unity 6000.0+ (Unity 6).

### Skills
- Core skills are located in `Skills/`.
- Structure: `manifest.json`, `SKILL.md`, and `references/`.

## Verification Commands

### Rust
```bash
cargo build --workspace
cargo test --workspace
```

### CLI Help
```bash
cd gateway && cargo run -- bridge install --help
cd gateway && cargo run -- serve --help
```

## Core Invariants

Adapted from [alex-core-invariants](https://github.com/islee23520/alex-core-invariants). These six invariants govern every subsystem: gateway, bridge, and skills.

### `.lux` is the Single Source of Truth

The `.lux/` directory is the canonical state root for every Lux runtime. No other location may shadow or duplicate its data.

- If `.lux/` and another source disagree, `.lux/` is the live truth.
- Self-heal from `.lux/` when drift is detected — never from stale caches, indexes, or environment variables.
- External state (Unity project context, AI tool sessions, event logs) enters `.lux/` through defined write paths only.

### The Six Invariants

| # | Invariant | Principle | Lux-specific guidance |
|---|-----------|-----------|----------------------|
| 1 | **SSoT** | Two truths stay two truths. Pick one. | `.lux/` is the canonical owner. Gateway state, bridge connection info, session data — all live under `.lux/`. |
| 2 | **SoC / SRP** | Mixed responsibility survives every refactor. | `gateway/` owns server+CLI. `bridge/` owns Unity protocol. `Skills/` owns AI workflows. Cross-boundary writes require explicit interfaces. |
| 3 | **Consistency** | Contradictions compound. | Event log schemas, API response shapes, and bridge protocol messages must stay in sync. Schema changes must propagate to all consumers before merge. |
| 4 | **Atomicity** | Half-written state is undeclared truth. | Bridge commands must complete fully or roll back. Multi-step API operations must be transactional. Never expose partial state through server APIs. |
| 5 | **Idempotency** | Retries must converge, not corrupt. | `lux bridge install` must be safe to re-run. Heartbeat and status endpoints must return the same result for repeated identical requests. Event deduplication must exist at the log level. |
| 6 | **No Silent Fallback** | Silent fallback kills the core. | Never catch errors and return empty/default data. Never fall back to a legacy path without logging. Explicit failover (e.g., health check degradation) is allowed only if observable and does not alter canonical truth. |

### Enforcement

- `scripts/test-all.sh` — runs Rust, CLI smoke, structure, and policy checks.
- `scripts/test-all.sh --quick` — skips the full Cargo test suite but still runs smoke, structure, and policy checks.
- Violations found during code review must be resolved before merge.

### Allow Markers

In rare cases where a pattern is intentional, add a comment marker:
- `// lux-allow-failover` — explicit, observable failover.
- `// lux-allow-legacy` — documented transition path with sunset date.
- `// lux-allow-dual-write` — temporary migration with removal tracked in an issue.

## Anti-Patterns (DO NOT)
- Do not include Unity Editor window logic (Workbench, CodexImage) in this repo.
- Do not add GUI, dashboard, TUI, or frontend app code to this repo.
- Do not include TODO/FIXME/HACK comments.
- Do not treat the target Unity project as part of this repository.
