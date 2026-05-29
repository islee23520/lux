# How to Use LUX

LUX is a local-first multi-engine AI harness and evidence loop for Unity, Three.js, and Godot projects. Unity is the primary public-beta verified engine; Three.js and Godot use explicit capability status so AI clients can avoid unsupported actions.

## Godot Quick Checks

```bash
lux godot status --project-path /path/to/godot-project
lux bridge install --project-path /path/to/godot-project --type godot
```

Do not use `--engine godot` for bridge install in this plan. The bridge install selector is `--type godot`.

`lux godot build` is currently unavailable and exits non-zero until GoPeak-backed build support is verified.

## Agent Workflow Skills

Run:

```bash
lux agents-install --project-path /path/to/project
```

This installs `lux-godot` alongside other Lux workflow skills under `.agents/skills/` so Codex, Claude, OpenCode, and other `.agents`-aware clients can follow the same Godot harness workflow.
