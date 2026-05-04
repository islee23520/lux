# Contributing to LUX

Development, testing, and verification guide for LUX (Linalab Unity X).

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Unity | 6000.0+ | Editor with C# Roslyn analyzers |
| Rust | 1.75+ | `cargo`, `rustfmt`, `clippy` |
| Node.js | 18+ | For UI build and MCP helper |
| npm | 9+ | Package management for `ui-src/` |

## Repository Layout

```
com.linalab.lux/
├── LuxEditor/           # Unity Editor C# scripts (assembly: Linalab.Lux.Editor)
├── AiBridgeEditor/      # TCP server and protocol (assembly: Linalab.UnityAiBridge.Editor)
├── UnityGitEditor/      # Git integration (assembly: Linalab.UnityGit.Editor)
├── CodexImage/          # Image generation pipeline
├── RustGateway~/        # Rust CLI + Axum web server
│   ├── src/main.rs      # CLI entry point
│   ├── src/server.rs    # HTTP/WebSocket server
│   ├── src/protocol.rs  # Event schema
│   ├── src/ai_log.rs    # AI action log primitives
│   ├── tests/           # Smoke tests (44 tests)
│   └── ui-src/          # React 19 + TypeScript SPA
├── McpHelper~/          # Node.js MCP helper
├── Skills/lux-unity/    # Core AI skill
├── .lux/                # Project-local AI data root
│   ├── ROUTING.md       # Context routing manifest for AI tools
│   ├── context/         # Unity project context exports
│   ├── outputs/         # Artifact output directory
│   └── skills/          # Project-scoped installed skills
├── LuxEditorTests/      # LuxEditor NUnit tests
├── AiBridgeTests/       # AI Bridge NUnit tests
├── UnityGitTests/       # Git integration NUnit tests
├── CodexImage/Tests/    # CodexImage NUnit tests
├── LuxTests/            # Automation policy tests
└── scripts/             # Development and test scripts
```

## Local Development Setup

### 1. Clone and Open in Unity

```bash
git clone <repo-url>
# Open the parent Unity project in Unity Hub
# LUX is a UPM package under Packages/com.linalab.lux
```

### 2. Build the Rust CLI

```bash
cd RustGateway~
cargo build
```

The binary is at `target/debug/lux`. Install globally:

```bash
cargo install --path . --force --locked
lux --version
```

### 3. Set Up the Web UI

```bash
cd RustGateway~/ui-src
npm install
npm run dev      # Development server with HMR
npm run build    # Production build
```

### 4. Start the Gateway Server

```bash
cd RustGateway~
LUX_GATEWAY_TOKEN=$(uuidgen) cargo run -- serve --token $LUX_GATEWAY_TOKEN
# Or with project binding:
# LUX_GATEWAY_TOKEN=$(uuidgen) cargo run -- serve --token $LUX_GATEWAY_TOKEN --project-path /path/to/unity-project
```

## Testing

### Run All Automated Checks

```bash
./scripts/test-all.sh
```

This runs: Rust build, Rust tests, TypeScript strict check, CLI smoke tests, and protocol/module checks.

For a quick pass without the full test suite:

```bash
./scripts/test-all.sh --quick
```

### Individual Test Commands

#### Rust Tests

```bash
cd RustGateway~
cargo build                          # Compile
cargo test                           # Full Rust suite (unit + smoke tests)
cargo test ai_log                    # AI log primitives only
cargo test protocol                  # Protocol schema only
cargo test skill                     # Skill management tests only
cargo test rust_lux_cli_exposes      # CLI help flag tests
```

#### TypeScript / UI

```bash
cd RustGateway~/ui-src
npx tsc --noEmit                     # Strict type check (no test runner yet)
npm run build                        # Production build
```

#### C# / Unity Editor

C# tests require Unity Editor. Run via:

- `Window > General > Test Runner` in Unity Editor
- Or batch mode: `lux run-tests --project-path <unity-project-path>`

Test assemblies:
- `LuxEditorTests/Editor/LuxAiActionLogTests.cs` — AI action log core
- `LuxEditorTests/Editor/LuxAiActionLogBroadcaster*` — Broadcast queue
- `AiBridgeTests/Editor/` — TCP server, protocol, discovery
- `UnityGitTests/Editor/` — Git staging, branches, history
- `CodexImage/Tests/Editor/` — Pipeline, exporters, backends
- `LuxTests/Editor/` — Automation policy

### Feature-by-Feature Smoke Matrix

| Feature | CLI Verification | API Verification | Notes |
|---------|-----------------|------------------|-------|
| `lux ai-log recent` | `lux ai-log recent --limit 5 --json --project-path <path>` | `GET /api/ai-log?limit=5` | Requires `.lux/ai-action-log.jsonl` |
| `lux ai-log context` | `lux ai-log context --limit 10 --json --project-path <path>` | `GET /api/ai-log/context?limit=10` | Same log file |
| `lux ai-log compact` | `lux ai-log compact --max-lines 100 --project-path <path>` | N/A (CLI only) | Explicit only, no auto-compact |
| `lux ai-log tail` | `lux ai-log tail --limit 5 --project-path <path>` | N/A | Non-blocking, prints snapshot |
| `lux skill list` | `lux skill list --json` | N/A | Shows core + installed skills |
| `lux skill install --adapt` | `lux skill install <name> --source <path> --project --adapt --json` | N/A | Creates `.lux/skills/<name>/lux-adaptation.json` |
| `lux serve` | `lux serve --token test --port 17340 --project-path <path>` | `GET /health` | Background process |
| AI Timeline UI | Open `http://localhost:17340`, click "Timeline" tab | Same server | Requires running server with token |
| `.lux` path routing | Check `.lux/ROUTING.md` exists | N/A | Context routing for AI tools |

## Code Conventions

### C# (Editor Directories)

- Namespace: `UnityEditor`
- Assembly: `Linalab.Lux.Editor`
- All classes: `Lux` prefix
- Partial classes for large files
- Tests: NUnit `[Test]` in `*Tests/Editor/` directories
- No `as any`, `@ts-ignore` equivalents — use proper casts

### Rust (`RustGateway~/`)

- Axum 0.7, tokio 1, clap 4.5, anyhow, serde
- Error handling: `anyhow` for logic, `eprintln` for user output
- No `TODO`, `FIXME`, or `HACK` comments
- New endpoints require tests in `server.rs` or `gateway_cli_smoke.rs`
- Run `cargo fmt` before committing

### TypeScript (`RustGateway~/ui-src/`)

- React 19 with TypeScript strict mode
- Functional components and hooks only
- No mock or fallback data in API hooks
- State: `useState`, `useRef`, `useCallback`, `useEffect`

### Skills (`Skills/`)

- Core skills in `Skills/` are protected — do not remove
- Structure: `manifest.json`, `SKILL.md`, `references/`
- Project skills installed to `.lux/skills/`

## What Not To Commit

- `node_modules/` — installed via `npm install`
- `target/` — Rust build artifacts
- `Library/`, `Temp/` — Unity cache
- `dist/` — UI build output
- `.env`, `*.local.jsonc` — local secrets
- `UserSettings/` — Unity user settings (except `LuxBridgeSettings.json` schema)

## Architecture Notes

- **Core isolation**: `LuxAiActionLog` has no dependency on TCP bridge. Broadcast queue (`LuxAiActionLogBroadcaster`) bridges core → network.
- **Actor attribution**: Ambient scope + correlation ID + 2-second TTL propagation. Fallback: `actor=user`.
- **API security**: `/api/ai-log` endpoints require `x-lux-token` header. Project path is bound at `serve` startup, not per-request.
- **`.lux` directory**: Single AI data root. `ROUTING.md` tells AI tools which files to load for specific requests.
