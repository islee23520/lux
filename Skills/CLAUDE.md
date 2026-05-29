# Lux Skills Federation

This repository contains curated game development skills for AI coding assistants.

## Skill Categories
- `unity-pattern-*`: Unity/C# design pattern reference skills (21)
- `game-dev`, `lux-unity`, `unity-cs-reference`, `core-invariants`, `ldp-decision-protocol`: Lux workflow skills (5)
- `studio-*`: Adapted game studio management skills from CCGS (5)

## Usage
Skills in `.claude/skills/` are automatically available to Claude Code.
For OpenCode, run `tools/sync-opencode.sh` to mirror skills.

## Adding Skills
1. Create a new directory under `.claude/skills/<name>/`
2. Add a `SKILL.md` following the schema in `SKILL-SCHEMA.md`
3. Run `tools/validate-skills.sh` to verify
4. Update `catalogs/skill-inventory.yaml`

## Federation
Skills not in this repo can be found in donor repos listed in `catalogs/skill-inventory.yaml`.
