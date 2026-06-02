# Lux Skills

## Overview

Lux ships a tracked skill source tree for local AI workflows. The Gateway can expose manifest-backed built-in skills through CLI/API surfaces and can project installed workflow skills into target projects.

## What's Inside

### Skills Inventory

Current tree count: 46 total `SKILL.md` files under `Skills/skills`, including 20 manifest-backed built-in skills.

| Group | Count | Description |
|----------|-------|-------------|
| Manifest-backed built-in skills | 20 | Runtime and workflow skills with `manifest.json` plus `SKILL.md`; validated by `Skills/tools/validate-skills.sh`. |
| Reference-only SKILL.md docs | 26 | Unity pattern and studio reference skills kept as reusable documentation; not all are release manifests. |
| Total SKILL.md files | 46 | All skill documents currently tracked under `Skills/skills`. |

### Additional Resources
- **Design Templates**: Narrative, Levels, Art-Style, Audio, Architecture, UI/UX, Testing docs
- **Catalog Metadata**: Inventory files for skill discovery and validation

## Quick Start

### Validate Built-In Skills
```bash
SKILLS_ROOT=Skills/skills bash Skills/tools/validate-skills.sh
```

### Install Into A Target Project
```bash
cd gateway
cargo run -- agents-install --project-path /path/to/project
```

## Architecture

This repo uses a tracked-source plus projection model:
- **Manifest-backed built-in skills** are direct Gateway/CLI inventory entries.
- **Reference-only skills** remain useful `SKILL.md` documents without being forced into release manifests.
- **Projection tools** install selected skills into target-project agent directories.

### Canonical Skill Format

Manifest-backed built-in skills follow the SKILL-SCHEMA.md specification:
- YAML frontmatter (name, description, category, source)
- Structured markdown body
- Under 500 lines per skill

Reference-only skill documents may lack `manifest.json`; this cleanup intentionally does not force them into the manifest-backed release set.

## Sources

| Source | Type | Skills Used |
|--------|------|-------------|
| [Donchitos/Claude-Code-Game-Studios](https://github.com/Donchitos/Claude-Code-Game-Studios) | Reference | Studio reference skills |
| unity-design-patterns-skills | Donor | Unity design pattern reference skills |
| Lux (Linalab Unity X) | Local | Manifest-backed workflow skills |
| Chronos | Donor | Federation pointer |

## License

MIT License - See [LICENSE](./LICENSE)

## Acknowledgments

- [Donchitos/Claude-Code-Game-Studios](https://github.com/Donchitos/Claude-Code-Game-Studios) - Game studio AI skills template (49 agents, 73 skills, 12 hooks, 11 rules)
- [CatDarkGame/claude-skill-unity-urp](https://github.com/CatDarkGame/claude-skill-unity-urp) - Unity URP RenderGraph skill (available via federation)
