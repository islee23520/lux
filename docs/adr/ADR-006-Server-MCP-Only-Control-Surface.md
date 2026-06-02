# ADR-006: Server/MCP-Only Control Surface

## Status
Accepted (Lux redefinition cleanup, 2026-06-02)

## Context
Lux previously carried multiple human-facing application UI concepts: a browser
control app, static `/ui` assets, GUI/TUI command examples, and legacy adapter
roots. Those surfaces conflicted with the current repository shape and with the
project invariant that `gateway/` owns the executable control plane while
runtime truth stays under `.lux/`.

The current product direction is not "no UI in target games" and not "Unity
only". Lux is a local-first automation and evidence loop for game projects:
Unity is the primary verified engine path, while Godot and Three.js remain
capability-tiered. Server/MCP-only describes the Lux control surface, not the
target engine capability model.

## Decision
Lux will expose its control surface through non-interactive local automation
interfaces:

- `lux serve` for HTTP/WebSocket gateway APIs.
- `lux mcp` for JSON-RPC stdio MCP clients.
- CLI commands that invoke gateway, bridge, spec, run, skill, and diagnostic
  workflows.
- Installed engine bridge adapters that report project and editor state back to
  the gateway.

Lux will not ship or restore a repository-owned browser control app, static
`/ui` application, GUI command, TUI shell, React/Vite app, or legacy adapter
source root as part of the active source hierarchy.

## Required Repository Shape
Active source roots:

- `gateway/`
- `crates/`
- `bridge/`
- `Skills/skills/`
- `docs/`
- `scripts/`

Removed roots must remain absent unless a future ADR supersedes this decision:

- `adapters/`
- `gateway/ui-src/`
- `gateway/ui/`
- `plugins/`
- `seeds/`
- `bridge-threejs/`

## Preserved Surfaces
This decision must not remove or weaken:

- `lux serve`
- `lux mcp`
- `/health`, `/api/health`, `/api/heartbeat`
- `/schema`, `/events`, `/api/events`
- existing `/api/*` route families
- Unity UI automation commands that operate on a target game or editor state
- game UI/UX spec domains under `.lux/specs`

## Consequences
- README, usage docs, roadmap projections, and agent instructions must describe
  Lux as a local-first server/MCP control plane.
- `scripts/check-project-structure.sh` must reject removed legacy roots.
- `scripts/test-all.sh` must verify the Rust workspace, structure guard, core
  crate boundary, CLI smoke, and policy scan.
- The prior `plans/server-mcp-only-ui-removal.md` plan is treated as a
  sub-workstream of the broader Lux redefinition cleanup, not a separate product
  direction.

## Verification
Required evidence for this decision:

```bash
bash scripts/check-project-structure.sh
cargo build --workspace
bash scripts/test-all.sh --quick
cd gateway && cargo test --test static_serving_smoke ui_route_is_not_served_when_ui_is_removed
cd gateway && cargo test --test gateway_cli_smoke gui_command_is_not_available
cd gateway && cargo test --test gateway_cli_smoke mcp_stdio_initializes_and_lists_bridge_and_game_dev_tools_without_unity
cd gateway && cargo test --test gateway_cli_smoke cli_server_starts_and_enforces_header_auth_and_origin_policy
```
