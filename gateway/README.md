# Lux Rust Gateway

The Lux Rust Gateway is the central orchestration layer for `com.linalab.lux`.
LUX is a local-first server/MCP evidence-gated automation control plane for game projects, and the gateway is its side-effect owner: CLI, HTTP/WebSocket APIs, MCP stdio serving, bridge installation, engine command execution, and `.lux/` evidence writes.

> [!NOTE]
> The canonical LUX implementation roadmap/status is maintained in `.lux/roadmap.json`.
> User/game requirements live in canonical `.lux/specs/spec.json`.
> `.lux/spec.json` is a compatibility mirror / legacy read fallback.
> Tickets and active run lifecycle live in `.lux/tickets/*.json` and `.lux/run-state.json`.
> This document provides a high-level overview of the gateway's role and current capabilities.

## Run

```bash
cd gateway
LUX_GATEWAY_TOKEN=$(uuidgen) cargo run -- serve --host 127.0.0.1 --port 17340
```

## Install or Update

From a Unity Package Manager install of `com.linalab.lux`, run
`Tools > Linalab > Lux > Rust CLI > Install or Update Global Tool`. This installs
or updates the packaged Rust binary globally through Cargo.

From a terminal, run the same install/update command directly:

```bash
cargo install --path gateway --force --locked
lux --version
lux unity status --project-path /path/to/unity-project
```

For local package-cache paths, replace `gateway/`
with the actual path to this `gateway/` directory.

`lux unity status` reads `UserSettings/LuxBridgeSettings.json`, which is written
from Unity through `Tools > Linalab > Lux > Unity Bridge > Write Lux Bridge
Settings`. It only uses Lux-owned settings files.

Environment variables are also supported:

```bash
LUX_GATEWAY_TOKEN=$(uuidgen) LUX_GATEWAY_PORT=17340 cargo run -- serve
```

The gateway intentionally has no built-in token default. Generate a local token
for each development session and pass the same value to Unity through the
`LUX_GATEWAY_TOKEN` environment variable before opening the Editor bridge.

## Status & Roadmap

The gateway implementation follows the project-wide roadmap:
- **Phase A (Core)**: active Rust CLI/server foundation.
- **Phase B (Events)**: active event and evidence projection foundation.
- **Phase C (Server/MCP control plane)**: current repository surface.
- **Phase D/E (Skills and agent execution)**: scaffolded through gateway templates, MCP tools, installed workflow skills, and `.lux/` evidence; legacy adapter roots are not active source.

Autonomous spec-to-ticket execution is planned but not yet implemented.

## Endpoints

- `GET /health` returns gateway readiness and protocol version.
- `GET /schema` returns an example event envelope.
- `GET /events?role=<role>&client_id=<id>` upgrades to a WebSocket.

The local shared token should be passed as `x-lux-token` during the WebSocket
upgrade. The gateway also accepts a `token=` query parameter for local CLI and
smoke-test clients; prefer the header form for long-running integrations.

## Event Envelope

The prototype streams JSON envelopes:

```json
{
  "schema_version": 1,
  "event_id": "uuid-or-unity-guid",
  "category": "tool",
  "source": "unity-editor",
  "session_id": "unity-session",
  "captured_at_utc": "2026-04-30T00:00:00.0000000Z",
  "payload": { "kind": "demo" }
}
```

Supported Phase 1 categories are `playmode`, `scene`, `log`, `tool`, `input`,
`screenshot`, and `hierarchy`. Some categories are skeleton/demo events in this
slice; production fidelity is deliberately deferred.


## Stdio MCP Game-Development Surface

`lux mcp --project-path <unity-project>` exposes a JSON-RPC stdio MCP server for
AI clients that need a bounded Unity game-development loop. The MCP surface is
additive to the existing CLI/HTTP bridge APIs and uses `.lux/` as the canonical
state and evidence root; it does not introduce a second run database or a Unity
Editor window UI.

Existing bridge tools remain available:

- `lux_bridge_install` — install or refresh the Lux Unity bridge in the selected
  project. Re-runs should converge without duplicate corruption.
- `lux_bridge_diagnostics` — report bridge discovery/health information and
  surface missing bridge/server files as explicit tool failures.

The first game-development milestone adds these tools:

- `lux_game_spec_write` — write or import a minimal game/project spec through
  the Lux spec path, returning the canonical `.lux/specs/spec.json` path plus
  validation and detection summaries. `.lux/spec.json` remains a compatibility
  fallback for older project state.
- `lux_game_ticket_prepare` — create or select one safe first-loop ticket using
  the Lux ticket store, including objective, non-goals, allowlist, verification
  policy, and ticket path.
- `lux_unity_maneuver` — perform one safe code, scene, settings, package, or
  simple asset maneuver through existing bridge/uloop/capture surfaces, then
  record structured evidence under `.lux/` or return an explicit unavailable
  failure if Unity/bridge access is missing.
- `lux_game_dev_loop_once` — orchestrate bridge install, diagnostics, spec
  write/import, ticket preparation, one Unity maneuver, validation/evidence
  capture, and `.lux` state updates, then stop after one loop.

All tool calls must return MCP content plus structured output. Tool failures use
`isError: true` while keeping the JSON-RPC connection alive so a following
`ping` or tool call can still succeed. The loop-once tool reports every substep
in `structuredContent.steps[]` and ends with `stopReason` such as
`one_verified_loop_complete`, `unity_unavailable`, `bridge_unavailable`,
`validation_failed`, or another specific failure reason.

### One-loop contract and boundaries

The MCP game-development loop is intentionally bounded:

1. Install or refresh the bridge.
2. Run diagnostics.
3. Write/import a minimal spec into canonical `.lux/specs/spec.json`.
   Keep `.lux/spec.json` only as a compatibility fallback.
4. Create/select one ticket under `.lux/tickets/`.
5. Execute one safe maneuver through existing Lux/Unity surfaces.
6. Run compile/test/playmode or configured validation where available.
7. Store evidence references and update `.lux` ticket/run state.
8. Stop; multi-loop continuation is a later workflow concern.

Non-goals for this milestone:

- No new Unity Editor window or panel logic.
- No destructive rewrites, broad asset churn, or external-service actions by
  default.
- No silent dry-run success when Unity, bridge discovery, or validation is
  unavailable; return a structured failure with retained evidence instead.
- No alternative state root outside `.lux/`.

Team/Ultragoal implementation checkpoints should attach evidence for changed
files, focused MCP JSONL smoke tests, `.lux` spec/ticket/evidence samples, and
any explicit Unity-unavailable failure output before marking this milestone
complete.

## Unity Prototype Bridge

Unity can connect through the menu commands under `Tools > Linalab > Lux >
Rust Gateway Prototype` after the gateway is running. The default URL is
`ws://127.0.0.1:17340/events`. The bridge reads the token from
`LUX_GATEWAY_TOKEN` and sends it through the `x-lux-token` WebSocket upgrade
header.

## Verification

```bash
./Scripts/run-lux-rust-gateway-tests.sh
```

The integration smoke path starts the compiled Rust `lux serve` binary on an
ephemeral local port, verifies `/health`, and checks WebSocket authentication
and Origin rejection behavior. It does not require a separate Unity automation
server to be running. Capture-path integration tests provision their own local
Unity bridge discovery file (`Library/UnityAiBridge/server.json`) and TCP stub;
`lux unity status` still reads the Lux-owned `UserSettings/LuxBridgeSettings.json`
settings file.

For Unity-side compile verification, use the repo's direct Unity batchmode or
solution build entry points when needed. The Rust gateway smoke path above is
independent from separate Unity automation packages.

## Deferred

- Remote/WebRTC sessions are hidden experimental by default and gated by `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`.
- Production-grade remote control sessions.
- Per-client approval UI and role-based permissions.
- Full screenshot frame streaming and high-fidelity hierarchy diffs.
- Complete Lux-native Unity automation command surface.
