# Lux Unity Bridge

The Lux Unity Bridge is the Unity-side adapter for the Lux public beta. It installs C# Editor scripts into a Unity project so the local Lux Gateway can collect Editor context, run compile/test actions, read logs, and return evidence to AI assistants.

## Compatibility

| Surface | Public beta support |
|---|---|
| Unity | Unity 6000.0+ (Unity 6) |
| Host OS | macOS-first |
| Transport | Local TCP on 127.0.0.1 plus Gateway HTTP/WebSocket |
| Package name | `com.linalab.lux-bridge` |
| License | MIT |

Windows/Linux Editor support is not promised for the public beta. Unity-free flows can run documentation, Rust, and UI checks, but Bridge functionality requires a Unity project.

## Fresh Clone Setup

```bash
git clone --recurse-submodules https://github.com/islee23520/Lux.git
cd Lux
git submodule update --init --recursive
cargo install --path gateway
lux bridge install --project-path /path/to/your/unity-project
```

The bridge source is kept in the `bridge/` submodule (`https://github.com/islee23520/lux-bridge.git`). If `bridge/package.json` is missing after clone, run the submodule update command before installing.

## What `lux bridge install` Touches

- Copies Lux-owned Bridge Editor scripts into `Assets/Editor/LuxBridge/` in the target Unity project.
- If a legacy `Assets/Editor/AiBridgeEditor/` installation exists, migrates its contents to the new `LuxBridge/` layout and removes the old directory.
- Writes or updates Lux-owned bridge settings and discovery files.
- Initializes or uses the target project's `.lux/` directory as the runtime state root.
- Installs or reuses Lux workflow skills under `.agents/skills/` for AI-agent workflows.
- May install optional OpenCode plugin integration into `.opencode/plugins/` when the surrounding Lux flow requests it.

Run bridge install from a clean version-control state so file changes can be reviewed. Re-running bridge install should converge without duplicate destructive changes.

## Trust Boundary

The bridge communicates with the local Gateway only. The Gateway token is local session state and is sent as `x-lux-token` for WebSocket/API access. Project files and evidence remain local unless the user explicitly shares them through normal version-control or publishing workflows.

## Known Issues

- Bridge tests that require a live Unity Editor are not part of the default non-Unity CI gate.
- Remote/WebRTC is hidden experimental and not required for public beta readiness.
- Destructive uloop cleanup is not part of `lux bridge install`; use `lux unity sync-uloop --migrate-uloop=apply` only when the repair flow reports it is safe.

## Verification

```bash
cd gateway && cargo run -- bridge install --help
cd gateway && cargo run -- unity status --help
```

For a real Unity project, install the bridge, start `lux serve`, and verify `lux unity status --project-path /path/to/your/unity-project` reports an actionable result.
