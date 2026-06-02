# Godot Harness Support

LUX is a local-first server/MCP evidence-gated automation control plane for Unity, Three.js, and Godot game projects.
Unity remains the primary public-beta verified engine.
Godot is a first-class target category with explicit capability status so unsupported operations are not silently advertised as working.

Public maturity labels: Unity is verified, Godot is partial, and Three.js is planned. Godot's partial status covers detection, bridge install, status reporting, and workflow skill projection only.

## Godot Capability Matrix

| Capability | Godot status | Notes |
| --- | --- | --- |
| Project detection | verified | `project.godot` with `config_version=5` identifies Godot 4.x projects. |
| `.lux` init | planned | Godot-specific spec initialization is tracked separately from this bridge/status MVP. |
| Bridge install | verified | `lux bridge install --project-path <project> --type godot` installs managed files under `addons/lux_bridge/`. |
| Status | verified | `lux godot status --project-path <project>` reports detection plus separate GoPeak and LUX command capability fields. |
| Build/run/test | unsupported | `lux godot build` exits non-zero until GoPeak-backed build has automated verification. |
| Logs/events | planned | Requires a verified Godot/GoPeak evidence loop. |
| Scene/runtime inspection | planned | GoPeak command availability is reported separately from LUX-verified support. |
| Screenshot/capture | planned | Not part of this Godot MVP. |
| `.agents` workflow skill | verified | `lux agents-install` installs `lux-godot` into `.agents/skills/`. |

## Bridge Install

```bash
lux bridge install --project-path /path/to/godot-project --type godot
```

The project must be a Godot 4 project with `project.godot` containing `config_version=5`. There is no force bypass for Godot bridge validation in this plan.

Installed managed files:

- `addons/lux_bridge/plugin.cfg`
- `addons/lux_bridge/bridge.gd`

Re-running the command is idempotent when generated content is unchanged.

## Status

```bash
lux godot status --project-path /path/to/godot-project
```

The status output separates external GoPeak command visibility from LUX-verified command support:

- `gopeak.available_commands`
- `gopeak.missing_commands`
- `lux.supported_commands`
- `lux.unsupported_commands`

A GoPeak manifest entry such as `project/build` does not mean `lux godot build` is supported. `lux.unsupported_commands` includes `godot build` until end-to-end build verification exists.

## Build

```bash
lux godot build --project-path /path/to/godot-project
```

This command intentionally exits non-zero for now. It will only become supported after GoPeak-backed build behavior has automated verification.

## GoPeak Boundary

GoPeak commands reported in `lux godot status` are external adapter capabilities, NOT LUX-verified support. A `project/build` entry in the GoPeak manifest does not mean `lux godot build` is supported. LUX-verified commands require end-to-end harness verification within the LUX runtime.

## .agents Workflow

The `lux agents-install` command installs the `lux-godot` skill into `.agents/skills/`. This allows AI clients to follow the Godot harness workflow, providing the necessary context and instructions for interacting with Godot projects through LUX.
