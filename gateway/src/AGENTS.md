# Gateway Rust Core

Axum 0.7 HTTP/WS server + CLI. Single-binary gateway for all LUX operations.

## STRUCTURE
```
gateway/src/
├── main.rs           # 4485L monolithic CLI: serve, compile, bridge install, smoke, context refresh
├── server.rs         # 3694L Axum routes, handlers, SPA fallback, WebSocket
├── session.rs        # Per-tool session tracking
├── protocol.rs       # TCP bridge protocol types
├── config.rs         # Gateway configuration
├── ai_log.rs         # AI interaction logging
├── project.rs        # Unity project detection
├── unity_hub.rs      # Unity Hub integration
├── unity_launch.rs   # Unity batch mode launcher
├── addon_auth.rs     # GitHub addon authentication
├── addon_routes.rs   # Addon marketplace routes
├── addon_store.rs    # Addon storage
├── cross_platform.rs # OS-specific utilities
├── visual_regression.rs # Visual diff utilities
├── lib.rs            # Library re-exports
└── skill_adapter/    # Skill loading/adaptation logic
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add CLI subcommand | `main.rs` | All subcommands in single file |
| Add API endpoint | `server.rs` | Axum router at top, handlers below |
| Modify session logic | `session.rs` | Per-tool session state |
| Change bridge protocol | `protocol.rs` | Request/response types |
| Bridge install logic | `main.rs:3587` | `install_bridge_files()` |
| Unity launch logic | `unity_launch.rs` | `resolve_unity_launch_target()` |
| Skill discovery | `skill_adapter/` | `discover_skill_summaries()` — scans Skills/, .lux/skills/ |

## KEY PATTERNS
- `main.rs` is intentionally monolithic — all CLI logic in one file
- `server.rs` follows Axum 0.7 patterns: `Router::new().route()`, `State` extraction
- Error handling: `anyhow` everywhere, `eprintln!` for user output
- Test pattern: integration tests in `gateway/tests/`, not in src/

## COMMANDS
```bash
cd gateway && cargo build && cargo test
cd gateway && cargo run -- serve --help
cd gateway && cargo run -- compile --project-path /path/to/project
```
