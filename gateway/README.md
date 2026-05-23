# Lux Rust Gateway

The Lux Rust Gateway is the central orchestration layer for `com.linalab.lux`. It provides the CLI and HTTP/WebSocket API.

> [!NOTE]
> The canonical LUX implementation roadmap/status is maintained in `.lux/roadmap.json`. User/game requirements, tickets, and active run lifecycle live in their ADR-defined domain files (`.lux/spec.json`, `.lux/tickets/*.json`, and `.lux/run-state.json`). This document provides a high-level overview of the gateway's role and current capabilities.

## Run

```bash
cd gateway
LUX_GATEWAY_TOKEN=$(uuidgen) cargo run -- serve --host 127.0.0.1 --port 17340
```

## Install or Update

From a Unity Package Manager install of `com.linalab.lux`, open `Window >
Linalab > Lux Workbench` and click **Install Global Rust CLI**. You can also
run `Tools > Linalab > Lux > Rust CLI > Install or Update Global Tool`.
Both paths install or update the packaged Rust binary globally through Cargo.

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
Settings` or the Lux Workbench. It only uses Lux-owned settings files.

Environment variables are also supported:

```bash
LUX_GATEWAY_TOKEN=$(uuidgen) LUX_GATEWAY_PORT=17340 cargo run -- serve
```

The gateway intentionally has no built-in token default. Generate a local token
for each development session and pass the same value to Unity through the
`LUX_GATEWAY_TOKEN` environment variable before opening the Editor bridge.

## Status & Roadmap

The gateway implementation follows the project-wide roadmap:
- **Phase A (Core)**: ✅ Mostly complete.
- **Phase B (Events)**: ✅ Mostly complete.
- **Phase C (Dashboard)**: ⚠️ Partial.
- **Phase D/E (Skills/OpenCode)**: 🏗️ Scaffolded.

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
