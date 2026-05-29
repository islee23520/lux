---
name: lux-unity
description: "Lazy-load for LUX Unity bridge and MCP game-development operations: status, compile, tests, logs, launch, screenshots, hierarchy, dynamic code, record/replay, and one-loop MCP workflows."
---

# LUX Unity

Use this skill only when a request directly involves operating or inspecting a Unity project through LUX.

## Core Rules

- Treat `.lux/` as the project runtime source of truth for specs, tickets, run state, and evidence.
- Prefer gateway/bridge/MCP commands over direct Unity project edits.
- Keep operations explicit, observable, and reversible where possible.
- Never hide bridge, compile, test, MCP, or API failures behind fallback data.
- For game-development MCP requests, prefer the bounded one-loop contract: bridge install, diagnostics, spec write, ticket prepare, one Unity maneuver, validation/evidence update, then stop.
- Do not add or rely on Unity Editor window UI for the MCP game-development milestone.
- Read reference files only for the operation being performed.


## MCP Game-Development Tools

Use `lux mcp --project-path <unity-project>` when an AI client needs the stdio MCP surface. The game-development tools are:

- `lux_bridge_install` and `lux_bridge_diagnostics` for bridge setup and health.
- `lux_game_spec_write` for `.lux/spec.json` creation/import.
- `lux_game_ticket_prepare` for one safe `.lux/tickets/` first-loop ticket.
- `lux_unity_maneuver` for one safe Unity code/scene/settings/package/asset action with evidence.
- `lux_game_dev_loop_once` for the full installed-bridge loop, stopping after one verified loop or a specific structured failure.

Expected MCP reports include `structuredContent`, step/evidence references, and a clear `stopReason`. If Unity or bridge discovery is unavailable, report that blocker directly; do not substitute mock success unless the caller explicitly requested a test harness.

## Operation References

- `references/backend-status.md` for gateway and bridge status.
- `references/compile-tests.md` for compile and test flows.
- `references/dynamic-code.md` for guarded C# execution.
- `references/launch.md` for Unity launch behavior.
- `references/logs.md` for logs and event inspection.
- `references/object-hierarchy.md` for hierarchy/object inspection.
- `references/playmode-input.md` for play mode and input automation.
- `references/record-replay.md` for recording and replay.
- `references/screenshots.md` for screenshots and visual checks.

## Output

Report the command or API used, observed result, verification evidence, and any remaining blocker.
