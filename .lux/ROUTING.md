# LUX Context Routing

Read this file first when loading project context. It tells you which files to load for specific requests.

Do NOT load all files at once. Route to only the file needed for the current request.

## Files

| Path | Purpose | When to Load | Approximate Size |
|------|---------|--------------|------------------|
| `.lux/context/unity-context.json` | Unity project metadata (version, platform, active scene, selection, packages, assemblies) | Session start, or user asks about project setup | 2-5 KB |
| `.lux/ai-action-log.jsonl` | Timestamped AI action log entries (actor, category, source, target, severity) | User asks about recent actions, workflow history, or interrupted work | Grows over time |
| `.lux/outputs/<feature>/<request>.json` | Artifact metadata (screenshots, hierarchy data, input recordings) | User asks about a specific artifact | Varies |
| `.lux/skills/<name>/manifest.json` | Skill manifest with name, version, description | Session start (names only, not full content) | <1 KB each |
| `.lux/skills/<name>/SKILL.md` | Full skill instructions | User invokes or asks about a specific skill | 5-50 KB |

## Injection Rules

1. **Session start**: Read `ROUTING.md` + skill names from `.lux/skills/*/manifest.json`.
2. **Project setup question**: Read `.lux/context/unity-context.json`.
3. **Workflow or history question**: Run `lux ai-log recent --limit 20 --json`.
4. **Artifact question**: Read the specific `.lux/outputs/<feature>/` file.
5. **Skill question**: Read `.lux/skills/<name>/SKILL.md`.
6. **Never**: Do not load the entire `.lux/ai-action-log.jsonl` file into context.

## CLI Commands for Large Files

Use these instead of reading raw files:

```bash
lux ai-log recent --limit <N>        # Recent log entries
lux ai-log recent --limit <N> --json # JSON output
lux ai-log context --limit <N>       # Context window extraction
lux ai-log tail --limit <N>           # Tail latest entries
lux skill list                        # List installed skills
lux skill info <name>                 # Skill details
```

## Auto-Refresh

| File | Trigger |
|------|---------|
| `.lux/context/unity-context.json` | `Tools > Linalab > Lux > Unity Context > Refresh Now` in Unity Editor |
| `.lux/ai-action-log.jsonl` | Automatic on Unity Editor events (selection, playmode, scene, undo, tool dispatch) |
