# ADR-005: Core Package Layer Split

## Status
Accepted (LUX game verification split, 2026-06-01)

## Context
`gateway` currently owns CLI, Axum routes, MCP serving, engine side effects, bridge protocol DTOs, spec/run/ticket state, verification logic, AI session payloads, and adapter installation. The game verification ontology needs reusable contracts instead of another gateway-local subsystem.

The split preserves ADR-001's gateway execution ownership and ADR-002's `.lux/` runtime single source of truth. The package currently published from `gateway/Cargo.toml` remains `name = "lux"` until a separate rename plan exists.

## Decision
LUX will split stable contracts into one-way Rust core packages while keeping `gateway` as the side-effect shell.

| Package / crate | Owns | Must not own |
| --- | --- | --- |
| `lux-core` | Shared IDs, path wrappers, timestamps, atomic `.lux` IO helpers, redaction-safe primitives. | Axum, clap, engine process execution, network transport, environment-derived behavior. |
| `lux-project` | Project detection facts and engine capability records for Unity, Godot, and Three.js. | Running engine commands, serving HTTP, or launching tools. |
| `lux-spec-core` | `.lux/specs`, spec/domain/decision/preference models, ambiguity model, migration contracts. | Prompt text, Axum handlers, terminal execution. |
| `lux-run-core` | Run state, tickets, task DAG, goals, evidence references, continuation state. | Engine execution side effects, git push, server routing. |
| `lux-bridge-core` | Bridge protocol DTOs, command/result schemas, engine capability/blocker payloads. | TCP transport, Unity launch, uloop process execution. |
| `lux-verification-core` | Verification ontology, evidence classes, completion gates, scene AST schema, coordinate mapping schema, visual-match schema. | Screenshot capture, vision provider calls, server/API rendering. |
| `lux-ai-core` | AI event/session/log models, prompt/context payload contracts, skill metadata contracts. | Spawning Codex/OpenCode/Claude or owning server hooks. |
| `gateway` package `lux` | CLI, Axum HTTP/WS server, MCP server, process/filesystem side effects, adapter installation, engine command execution. | Canonical domain model definitions once moved to core crates. |

## Dependency Direction
Core dependencies are one-way and must not point back into `gateway`:

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
- `lux-verification-core -> lux-run-core` for ticket and evidence references.
- `lux-verification-core -> lux-project` for engine capability decisions.
- `lux-ai-core -> lux-verification-core` for prompt evidence requirements.

Forbidden edges:
- Any core crate -> `gateway`.
- `lux-core` -> any higher crate.
- `lux-project` -> process runners.
- `lux-verification-core` -> screenshot capture implementation.
- `lux-ai-core` -> OpenCode runtime APIs.

## Purity Rule
Core crates may define typed data, schema versions, migrations, pure calculations, and owned atomic `.lux` IO helpers where explicitly assigned. Core crates must not own Axum handlers, clap parsing, network transport, process spawning, environment reads that change behavior, or direct Unity/Godot/Three.js/browser/terminal execution.

## Shared File Ownership
| Shared file | Owner | Rule |
| --- | --- | --- |
| `Cargo.toml` | WT-01 | Workspace member registration only. |
| `gateway/Cargo.toml` | WT-01 | Dependency additions flow through workspace integration. |
| `gateway/src/lib.rs` | WT-01 | Module exports/re-exports only after crate boundaries exist. |
| `gateway/src/main.rs` | WT-10 | CLI wiring after merged core APIs. |
| `gateway/src/server.rs` | WT-10 | Axum/API wiring after merged core APIs. |
| `bridge/unity/AiBridgeEditor/UnityAiBridgeProtocol.cs` | WT-05 | Bridge protocol compatibility owner. |
| `gateway/src/templates/plugin/*` | WT-07 | AI context template payload owner. |
| `docs/adr/*`, `README.md`, `docs/usage.md`, `Skills/skills/*` | WT-09 | Final docs and skill wording owner. |

If a worker needs a shared file owned by another worktree, it writes an integration note under evidence instead of editing the shared file directly.

## Consequences
- Game verification ontology can become a reusable contract instead of gateway-local behavior.
- Gateway remains the only side-effect owner and HTTP/CLI execution shell.
- MCP/API clients project canonical state but do not write verification truth directly.
- Godot and Three.js support stays capability-gated until real surfaces produce evidence.
