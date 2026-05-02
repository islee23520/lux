This document is available in **English** | [한국어](AGENTS.ko.md) | [日本語](AGENTS.ja.md)

# LUX (Linalab Unity X) Agent Guide

LUX is a unified Unity Editor AI adapter and automation toolkit. It is an independent Unity package. It is not a standalone application.

## Codebase Structure

| Path | Description | Assembly / Tech |
| :--- | :--- | :--- |
| `LuxEditor/` | Unity Editor C# scripts | `Linalab.LuxEditor` |
| `AiBridgeEditor/` | TCP server and protocol | `Linalab.UnityAiBridge.Editor` |
| `UnityGitEditor/` | Git integration | `Linalab.UnityGit.Editor` |
| `CodexImage/` | Image generation pipeline | C# Editor scripts |
| `RustGateway~/` | Rust CLI and Web Server | Axum 0.7, React 19 |
| `McpHelper~/` | Node.js MCP helper | Node.js |
| `Skills/lux-unity/` | Core AI skills | Manifest + SKILL.md |
| `*Tests/` | C# and Rust test suites | NUnit / Cargo |

## Key Conventions

### Rust (`RustGateway~/`)
- Use Axum 0.7, tokio 1, clap 4.5, anyhow, and serde.
- Error handling: Use `anyhow` for logic and `eprintln` for user output.
- No `TODO`, `FIXME`, or `HACK` comments.
- New endpoints must have tests in `server.rs` or `gateway_cli_smoke.rs`.
- Server lifecycle: idle timeout with graceful shutdown (`--idle-timeout`), heartbeat (`POST /api/heartbeat`), health (`GET /api/health`).

### TypeScript (`RustGateway~/ui-src/`)
- React 19 with TypeScript strict mode.
- Use functional components and hooks.
- No mock or fallback data in API hooks.
- State: `useState`, `useRef`, `useCallback`, `useEffect`.

### C# (Editor Directories)
- Namespace: `UnityEditor`. Assembly: `Linalab.LuxEditor`.
- All classes must have the `Lux` prefix.
- Use partial classes for large files to group logic.
- Large C# files use partial classes (e.g., LuxAutomationGateway split into ~10 files, LuxWebRTCProducer into ~7).
- Tests: Use NUnit `[Test]` in `*Tests/Editor/` directories.

### Skills
- Core skills are in `Skills/`. They cannot be removed.
- Structure: `manifest.json`, `SKILL.md`, and `references/`.

## Anti-Patterns (DO NOT)
- Do not remove the `Lux` prefix from C# class names.
- Do not add mock or fallback data to API hooks.
- Do not disable TypeScript strict mode.
- Do not remove core skill protection in the CLI.
- Do not commit without running `cargo test`.
- Do not edit test files just to make tests pass.
- Do not treat the host project (neon-glitch) as part of LUX.

## Verification Commands

### Rust
```bash
cd RustGateway~ && cargo build && cargo test
```

### TypeScript
```bash
cd RustGateway~/ui-src && npx tsc --noEmit
```

### CLI Help
```bash
cd RustGateway~ && cargo run -- skill install --help
cd RustGateway~ && cargo run -- serve --help
```

### C#
Verify using LSP diagnostics. No CLI build command is available.
