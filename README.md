# LUX

**LUX** stands for **Linalab Unity X**.

LUX is a unified Unity Editor AI adapter and automation toolkit. It bridges AI
coding tools (Claude Code, OpenAI Codex, OpenCode), external terminals, Git
workflows, Codex Image generation, WebRTC remote access, and a web-based
control surface into one Editor-focused package.

The `lux` CLI is part of that workflow, but LUX is not just a Unity CLI
wrapper: it is the Unity-side integration layer for AI-assisted development,
local automation, validation, and project operations.

## Features

### AI Tool Integration

- **Multi-AI Terminal** — Switch between Claude Code, OpenAI Codex, and
  OpenCode in a web-based xterm terminal with per-tool session persistence
  and command history.
- **Skill Dispatch** — Invoke compile, test, screenshot, logs, playmode, and
  dynamic-code from any active AI tool through a unified skill panel.
- **Tool Execution API** — Server-side `/api/tools/execute` with WebSocket
  event broadcasting to connected clients.
- **AI Tool Dispatcher** — Unity-side C# bridge (`LuxAIToolDispatcher`) that
  routes tool commands to the gateway server.

### Visual Pipeline Editor

- **ReactFlow Node Editor** — Drag-and-drop graph with typed port connections.
- **6 Node Types** — UnityContext, OutputDirectory, PromptTemplate,
  CodexGeneration, Segmentation, MaskPostProcessing.
- **Undo/Redo** — 40-step history stack with copy/paste support.
- **Save/Load/Execute** — Pipeline workflows as JSON through `/api/graphs`,
  executable from the web UI.

### Remote Access & Streaming

- **WebRTC Streaming** — Unity camera capture with configurable resolution and
  frame rate via `com.unity.webrtc` (reflection-based backend detection).
- **Remote Control** — Mouse, keyboard, and touch input forwarding from
  browser to Unity Editor.
- **Signaling Relay** — Queue-based WebSocket message delivery for session
  negotiation.
- **Token Authentication** — All remote and signaling endpoints require
  `x-lux-token` header.
- **STUN/TURN Support** — Configurable ICE servers for local and remote
  networks.

### Unity Editor Integration

- **Lux Workbench** — Main control window for the LUX system.
- **AI Bridge TCP Server** — Protocol handler for external terminal/client
  connections with dynamic code execution, input recording/replay.
- **Unity Git** — Status, staging, history, branches, remotes, and submodules
  in Editor windows.
- **Codex Image** — Node-based image generation pipeline with multiple
  exporters (Unity 2D Animation, Spine draft rig, sprite sheets).
- **Automation Guardrails** — Command blacklist, audit log, and approval state.
- **Server Status Indicator** — Shows gateway server status in Editor with
  heartbeat keep-alive and uptime display.

### Rust Gateway & CLI

- **Web Server** — Axum-based HTTP/WebSocket gateway serving React SPA and
  REST API.
- **CLI Commands** — Compile, test, Unity control (screenshot, logs, hierarchy,
  dynamic-code, input simulation), and skill management.
- **Skill System** — Core bundled skills + installable external skills via
  `lux skill install/remove/update`.
- **Server Lifecycle** — Configurable idle timeout with graceful shutdown;
  Unity Editor sends periodic heartbeats to keep the server alive.

## Roadmap

### Phase 1 — Core Editor Adapter ✅

Local-first Unity Editor integration layer for AI-assisted development.

- macOS-first Unity Editor adapter with Lux Workbench (`LuxEditor/`).
- AI Bridge TCP server and protocol handler for external terminal/client
  connections (`AiBridgeEditor/`).
- Unity Git integration with status, staging, and history windows
  (`UnityGitEditor/`).
- Codex Image generation pipeline with node-based execution engine and
  multiple exporters (`CodexImage/`).
- Rust `lux` CLI: compile, test, unity status, and skill commands
  (`RustGateway~/`).
- Automation guardrails: command blacklist, audit log, and approval state.
- Skill Manager foundation: `lux skill list/info` CLI, core `lux-unity` skill
  (`Skills/`).
- MCP helper for AI tool integration (`McpHelper~/`).

### Phase 2 — Gateway Expansion & Web UI ✅

Extend the Rust gateway into a full web-accessible control surface.

- Rust gateway serves static SPA files from `/ui/*` route.
- Session management API (`/api/sessions`) and pipeline API
  (`/api/pipeline`).
- React + TypeScript SPA with ReactFlow node editor and AI tool terminal.
- Multi-scope skill discovery: core, project (`.lux/skills/`), global
  (`~/.lux/skills/`).

### Phase 3 — Visual Pipeline Editor ✅

Interactive graph-based editing for CodexImage pipelines.

- Drag-and-drop node graph with typed port connections via ReactFlow.
- Save/load pipeline workflows as JSON through `/api/graphs`.
- Node library browser with category grouping and search.
- Undo/redo (40-step stack) and copy/paste in the visual editor.
- Bridge between ReactFlow web UI and existing C# pipeline engine
  via `/api/graphs/:id/execute`.
- All 6 node types rendered and executable from the visual editor.

### Phase 4 — Remote Access & Streaming ✅

Real-time remote Unity control through WebRTC.

- Unity C# WebRTC producer with reflection-based `com.unity.webrtc`
  backend, camera capture, and configurable resolution/frame rate.
- Rust gateway signaling relay with queue-based message delivery for
  WebRTC session negotiation.
- React remote viewer with video streaming, mouse/keyboard/touch input,
  and AI command data channel.
- Token-based authentication for all remote session and signaling APIs.
- ICE server configuration with STUN/TURN support.

### Phase 5 — Multi-AI Platform ✅

Unified skill dispatch across multiple AI coding tools.

- Tool selector with Claude Code, OpenAI Codex, and OpenCode tabs.
- xterm-based AI terminal panel with tool switching and command history.
- Skill dispatch panel: compile, test, screenshot, logs, playmode,
  dynamic-code invocable from any active AI tool.
- Server-side tool execution API with skill dispatch payload broadcasting.
- Per-tool session tracking with command history.
- Sessions persist across tool switches without data loss.
- Skill install/remove/update CLI commands.

### Out of Scope

- iOS companion app / PWA (may become a separate package).
- Windows and Linux editor support (Phase 1 is macOS-first; cross-platform
  path handling is prepared but not tested).

## Unity Editor Menu Reference

### Window

| Menu Path | Description |
| :--- | :--- |
| `Window > Linalab > Lux Workbench` | Main LUX control window |
| `Window > Linalab > Lux > Unity Git` | Git status, staging, and diff |
| `Window > Linalab > Lux > Git History Graph` | Visual Git history graph |

### Tools

| Menu Path | Description |
| :--- | :--- |
| `Tools > Linalab > Lux > Server Status` | Gateway server status indicator |
| `Tools > Linalab > Lux > AI Bridge > Export Unity Context` | Export project context for AI tools |
| `Tools > Linalab > Lux > AI Bridge > Rebuild Command Registry` | Rebuild automation command registry |
| `Tools > Linalab > Lux > AI Tool Dispatcher > Connect` | Connect to tool dispatcher |
| `Tools > Linalab > Lux > AI Tool Dispatcher > Disconnect` | Disconnect tool dispatcher |
| `Tools > Linalab > Lux > AI Tool Dispatcher > Status` | Show dispatcher status |
| `Tools > Linalab > Lux > Unity Bridge > Write Lux Bridge Settings` | Write bridge configuration |
| `Tools > Linalab > Lux > WebRTC Remote > Start Streaming` | Start WebRTC video streaming |
| `Tools > Linalab > Lux > WebRTC Remote > Stop Streaming` | Stop WebRTC streaming |
| `Tools > Linalab > Lux > WebRTC Remote > Status` | Show WebRTC streaming status |
| `Tools > Linalab > Lux > Rust CLI > Install or Update Global Tool` | Install `lux` CLI globally |
| `Tools > Linalab > Lux > Rust CLI > Copy Terminal Install Command` | Copy install command to clipboard |
| `Tools > Linalab > Lux > Unity Context > Write Snapshot` | Write Unity context snapshot |
| `Tools > Linalab > Lux > Unity Context > Refresh Now` | Refresh context immediately |
| `Tools > Linalab > Lux > Toggle Auto-Compile Watch` | Toggle compile watcher |
| `Tools > Linalab > Lux > Batch > Compile (Dry Run)` | Dry-run batch compilation |
| `Tools > Linalab > Lux > Scene Smoke > Create 10 Objects And Test Player` | Scene smoke test |
| `Tools > Linalab > Lux > Pipeline Web Bridge > Connect` | Connect pipeline to web bridge |
| `Tools > Linalab > Lux > Pipeline Web Bridge > Disconnect` | Disconnect pipeline web bridge |
| `Tools > Linalab > Codex Image` | Open Codex Image window |
| `Tools > Linalab > Codex Image Pipeline` | Open visual pipeline editor |

## CLI Reference

### Server

```bash
lux serve --token <TOKEN> [--host 127.0.0.1] [--port 17340] [--idle-timeout 30]
```

### Unity Commands

```bash
lux unity status                    # Editor connection status
lux unity context                   # Read shared context file
lux unity backend-status            # Bridge backend status
lux unity backend-list-commands     # List available protocol commands
lux unity get-logs                  # Console log entries
lux unity clear-console             # Clear console and show counts
lux unity focus-window              # Focus hierarchy/game window
lux unity launch                    # Launch Unity editor
lux unity control-play-mode         # Play/pause/stop playmode
lux unity screenshot                # Capture editor screenshot
lux unity simulate-mouse-ui         # Send UI-system mouse event
lux unity simulate-keyboard         # Send key press
lux unity simulate-mouse-input      # Send smooth mouse delta
lux unity record-input              # Record input sequence
lux unity replay-input              # Replay recorded input
lux unity execute-dynamic-code      # Execute C# code in editor
lux unity get-hierarchy             # Hierarchy metadata
lux unity scene-smoke               # Scene smoke test
lux unity create-objects            # Create test objects
lux unity find-game-objects         # Find objects by name
```

### Skill Management

```bash
lux skill list                      # List installed skills
lux skill list --json               # JSON output
lux skill info <name>               # Show skill details
lux skill install <name> --source <path|url>  # Install skill
lux skill install <name> --source <path> --project  # Project scope
lux skill remove <name>             # Remove installed skill
lux skill update <name>             # Update skill from source
```

### Build & Test

```bash
lux compile [--project-path <path>] # Batch compile
lux run-tests [--project-path <path>]  # Run tests
lux run-tests --playmode-platform   # Playmode tests
```

## API Reference

### Health & Lifecycle

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| GET | `/health` | Server readiness and protocol version |
| GET | `/api/health` | Uptime and status report |
| POST | `/api/heartbeat` | Reset idle timer, returns `{ "status": "alive", "uptime_seconds": N }` |

### Sessions & Pipeline

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| GET/POST | `/api/sessions` | Session management |
| GET/POST/DELETE | `/api/pipeline` | Pipeline operations |

### Graphs (Visual Pipeline)

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| GET/POST | `/api/graphs` | List/create graphs |
| GET/PUT/DELETE | `/api/graphs/:id` | Graph CRUD |
| POST | `/api/graphs/:id/execute` | Execute a graph |

### Tools (Multi-AI)

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| GET | `/api/tools` | List available tools |
| GET/POST/DELETE | `/api/tools/sessions` | Tool session management |
| POST | `/api/tools/execute` | Execute tool command or skill |
| GET | `/api/tools/executions/:id` | Get execution status |

### Remote Access (WebRTC)

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| GET/POST/DELETE | `/api/remote/sessions` | Remote session management |
| GET/POST | `/api/remote/signaling/:session_id` | WebRTC signaling relay |

### Events & Config

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| WS | `/events` | Real-time event stream (WebSocket) |
| GET | `/api/webrtc-config` | STUN/TURN ICE server configuration |
| GET | `/api/node-types` | Static node type registry |
| GET | `/schema` | Example event envelope |

## Entry Points

- `Window > Linalab > Lux Workbench`
- `Window > Linalab > Lux > Unity Git`
- `Tools > Linalab > Lux > AI Bridge > Export Unity Context`

## Structure

```text
com.linalab.lux/
├── LuxEditor/           # Adapter workbench, automation gateway, execution policy
│   ├── LuxAutomationGateway.cs        # Automation coordinator (partial)
│   ├── LuxAutomation*Commands.cs      # Command groups (8 partial files)
│   ├── LuxWebRTCProducer.cs           # WebRTC coordinator (partial)
│   ├── LuxWebRTC*.cs                  # WebRTC subsystems (6 partial files)
│   ├── LuxAIToolDispatcher.cs         # AI tool execution bridge
│   ├── LuxServerStatusIndicator.cs    # Gateway status UI
│   ├── LuxWorkbenchWindow.cs          # Main workbench window
│   └── ...
├── AiBridgeEditor/      # AI Bridge TCP server and protocol handler
│   ├── UnityAiBridgeTcpServer.cs      # TCP server (1,898 lines)
│   └── UnityAiBridgeProtocol.cs       # Protocol handler (830 lines)
├── UnityGitEditor/      # Unity Git integration
├── CodexImage/          # Integrated Codex Image generation and pipeline tooling
│   ├── Editor/Pipeline/               # Pipeline engine and web bridge
│   ├── Editor/Backends/               # Codex CLI and segmentation backends
│   └── Editor/Exporters/              # Unity 2D, Spine, sprite sheet exporters
├── RustGateway~/        # Rust WebSocket/HTTP gateway and CLI
│   ├── src/main.rs                    # CLI entry (2,836 lines)
│   ├── src/server.rs                  # HTTP/WebSocket server (2,439 lines)
│   ├── src/protocol.rs                # Event schema
│   ├── tests/                         # 37 smoke tests
│   └── ui-src/                        # React 19 + TypeScript SPA (strict mode)
├── McpHelper~/          # Node.js MCP helper
├── Skills/lux-unity/    # Core AI skill (manifest + SKILL.md + 9 references)
├── LuxEditorTests/      # LuxEditor unit tests
├── AiBridgeTests/       # AI Bridge unit tests
├── UnityGitTests/       # Unity Git unit tests
├── CodexImage/Tests/    # CodexImage unit tests (11 files)
├── LuxTests/            # Automation policy tests
└── seeds/               # Seed specifications for skill provisioning
```

## Test Coverage

| Suite | Tests | Files |
| :--- | :--- | :--- |
| Rust unit tests | 24 | `server.rs` |
| Rust smoke tests | 37 | `gateway_cli_smoke.rs` |
| C# test files | 27 | Across `*Tests/Editor/` directories |

## Acknowledgments

LUX was heavily inspired by and references significant portions of
[**unity-cli-loop**](https://github.com/hatayama/unity-cli-loop) by
[hatayama](https://github.com/hatayama) (formerly uLoopMCP).

The AI Bridge module (`AiBridgeEditor/`), including the TCP server, protocol
handler, dynamic code execution, input recording/replay, and the associated
skill/reference structure, was derived from unity-cli-loop. We are grateful for
the foundational work that made this project possible.

## License

This project is licensed under the [MIT License](LICENSE).
