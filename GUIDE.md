This document is available in **English** | [한국어](GUIDE.ko.md) | [日本語](GUIDE.ja.md)

# LUX (Linalab Unity X) Developer Guide

LUX is a unified adapter and automation toolkit that connects the Unity Editor with AI coding tools (Claude Code, OpenAI Codex, OpenCode, etc.). This guide is intended for developers who want to understand and utilize the structure of LUX.

## 1. Introduction

LUX acts as a bridge, allowing external AI tools to control tasks within the Unity Editor. Beyond simple command forwarding, it provides a web-based control surface, a visual pipeline editor, and remote control capabilities using WebRTC to enhance the automation level of the Unity development environment.

## 2. Installation

### Unity Package Installation
1. Add the LUX package to your Unity project's `Packages/manifest.json`.
2. Ensure the `com.unity.webrtc` package is installed (required for remote streaming features).

### Rust CLI Installation
The LUX gateway and CLI tools are written in Rust.
```bash
cd Packages/com.linalab.lux/RustGateway~
cargo build --release
# Add the built executable to your PATH or run it directly.
./target/release/lux --help
```

## 3. Quick Start

How to start the LUX server and connect it to Unity in 5 minutes.

1. **Launch Unity Editor**: Open your project and navigate to `Window > Linalab > Lux Workbench`.
2. **Start Server**: Enter the following command in your terminal:
   ```bash
   # Standard execution (auto-shutdown after 30 minutes of idle time)
   lux serve --port 8080

   # Change idle timeout (0 = disabled)
   lux serve --port 8080 --idle-timeout 60
   ```
3. **Access Web UI**: Open `http://localhost:8080` in your browser.
4. **Verify Connection**: Check the server status in the `Tools > Linalab > Lux > Server Status` window. Green indicates connected, yellow means the server is not running, and red indicates an error.
5. **Server Lifecycle**: The server remains active as long as the Unity Editor is active. It will automatically shut down after 30 minutes of inactivity (adjustable via `--idle-timeout`).

## 4. Architecture

LUX consists of several core modules.

| Module | Description |
| :--- | :--- |
| **LuxEditor** | Main adapter. Includes the workbench window, automation gateway, and WebRTC producer. |
| **AiBridgeEditor** | TCP server and protocol handler for communication with AI tools. |
| **UnityGitEditor** | Supports Git status checks, staging, and branch management within Unity. |
| **CodexImage** | Node-based image generation pipeline engine. |
| **RustGateway** | Axum-based web server and CLI. Provides the web UI and API endpoints. |
| **Skills** | Core skill sets and reference documentation for Unity control. |

## 5. CLI Reference

You can manage the server and control Unity through the `lux` command-line tool.

| Command | Description |
| :--- | :--- |
| `lux serve` | Starts the web server and gateway. |
| `lux compile` | Executes Unity project compilation. |
| `lux test` | Runs PlayMode and EditMode tests. |
| `lux unity status` | Checks the Unity Editor connection status. |
| `lux unity screenshot` | Captures the current editor screen. |
| `lux unity logs` | Streams Unity console logs. |
| `lux unity dynamic-code` | Dynamically executes C# code within Unity. |
| `lux skill list` | Lists installed skills. |
| `lux skill install <name>` | Installs a new skill. |

## 6. Web UI

After starting the gateway server, you can use the following features via your browser:

- **AI Terminal (AITerminal)**: Switch between and use various AI tools like Claude and Codex.
- **Pipeline Editor (NodeEditor)**: A visual tool based on ReactFlow for designing image generation workflows.
- **Remote Viewer (RemoteViewer)**: View the Unity screen in real-time via WebRTC and send mouse/keyboard input.
- **Session Manager**: Manage currently active AI tool sessions and command history.

## 7. Skill System

Skills are units that define how an AI controls Unity.

- **Core Skills**: The `lux-unity` skill is included by default, supporting compilation, testing, log checking, and more.
- **Skill Management**:
  ```bash
  # Check skill information
  lux skill info lux-unity
  # Install external skills
  lux skill install my-custom-skill --source https://github.com/user/repo
  ```

## 8. API Reference

Key endpoints for integration with external tools.

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/health` | GET | Checks server status and protocol version. |
| `/api/health` | GET | Reports server uptime and status. |
| `/api/heartbeat` | POST | Periodically called by the Unity Editor to refresh the idle timer. Returns `{ "status": "alive", "uptime_seconds": N }`. |
| `/api/sessions` | GET/POST | Manages AI tool sessions. |
| `/api/graphs` | GET/POST | Saves and loads pipeline graphs. |
| `/api/tools/execute` | POST | Sends commands to specific AI tools. |
| `/api/remote/signaling` | POST | Exchanges WebRTC signaling data. |
| `/events` | WS | Real-time event streaming (WebSocket). |

## 9. Remote Access (WebRTC)

LUX streams the Unity screen to a web browser.

- **Configuration**: You can adjust resolution and frame rate in the Lux Workbench within the Unity Editor.
- **Network**: Accessing from outside the local network requires STUN/TURN server settings. Enter ICE server information in the gateway configuration file.

## 10. Development Guide

### Running Tests
- **Rust**: `cargo test` (includes unit tests and smoke tests)
- **C#**: Run `AiBridgeTests`, `LuxTests`, etc., in the Unity Test Runner.

### How to Contribute
1. When adding new features, check the gateway policy in the `LuxEditor` module first.
2. To modify the web UI, edit the React components in the `RustGateway~/ui-src` path.
3. After applying changes, always perform regression testing via `lux test`.

## 11. Troubleshooting

- **Connection Failed**: Check if the Unity Editor is running and if the AI Bridge TCP server is active.
- **Server Keeps Shutting Down**: Disable the idle timeout with `--idle-timeout 0`, or ensure the Server Status window is open in the Unity Editor (it sends a heartbeat every 60 seconds).
- **WebRTC Screen Not Appearing**: Check `com.unity.webrtc` package version compatibility and check for signaling errors in the browser's console logs.
- **Permission Error**: Check if an approval popup is visible in the Unity Editor when executing automation commands.
- **TypeScript Errors**: Check with `cd RustGateway~/ui-src && npx tsc --noEmit`. Since strict mode is enabled, type errors must be fixed.
