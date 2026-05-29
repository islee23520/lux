# Lux Skills Federation

A consolidated game development skills repository for AI coding assistants (OpenCode and Claude Code).

## Overview

This repo curates and federates game development skills from multiple sources into one reusable library. It follows the architectural patterns established by [Claude Code Game Studios](https://github.com/Donchitos/Claude-Code-Game-Studios) (CCGS), adapting them for the Linalab toolchain.

## What's Inside

### Skills (31 total)

| Category | Count | Description |
|----------|-------|-------------|
| Unity Design Patterns | 21 | Reference skills for Unity/C# design patterns (adapter, builder, command, observer, etc.) |
| Lux Workflow | 5 | Runtime skills for Unity AI tooling (game-dev, lux-unity, unity-cs-reference, core-invariants, ldp-decision-protocol) |
| Studio Management | 5 | Adapted from CCGS for OpenCode (brainstorm, code-review, sprint-plan, gate-check, help) |

### Additional Resources
- **Genre Seeds**: Action-RPG, Narrative, Roguelike game genre templates
- **Design Templates**: Narrative, Levels, Art-Style, Audio, Architecture, UI/UX, Testing docs
- **Federation Catalog**: Pointers to 100+ additional skills available on-demand from donor repos

## Quick Start

### For OpenCode Users
```bash
# Clone this repo
git clone https://github.com/islee23520/lux-skills-repos.git ~/workspace/linalab/lux-skills-repos

# Skills are ready in .claude/skills/ and .opencode/skills/
# Point your OpenCode config at this repo's skills
```

### For Claude Code Users
```bash
# Clone and symlink into your .claude/skills
git clone https://github.com/islee23520/lux-skills-repos.git
ln -s $(pwd)/lux-skills-repos/.claude/skills/* ~/.claude/skills/
```

## Architecture

This repo uses a **Federation** approach rather than a monorepo:
- **Canonical skills** are directly included (31 curated skills)
- **Federation pointers** reference additional skills in their source repos
- **Projection tools** generate Claude Code / OpenCode compatible structures

### Canonical Skill Format

Every skill follows the SKILL-SCHEMA.md specification:
- YAML frontmatter (name, description, category, source)
- Structured markdown body
- Under 500 lines per skill

## Sources

| Source | Type | Skills Used |
|--------|------|-------------|
| [Donchitos/Claude-Code-Game-Studios](https://github.com/Donchitos/Claude-Code-Game-Studios) | Reference | 5 adapted studio skills |
| unity-design-patterns-skills | Donor | 21 design pattern skills |
| Lux (Linalab Unity X) | Donor | 5 workflow skills |
| Chronos | Donor | Federation pointer |

## License

MIT License - See [LICENSE](./LICENSE)

## Acknowledgments

- [Donchitos/Claude-Code-Game-Studios](https://github.com/Donchitos/Claude-Code-Game-Studios) - Game studio AI skills template (49 agents, 73 skills, 12 hooks, 11 rules)
- [CatDarkGame/claude-skill-unity-urp](https://github.com/CatDarkGame/claude-skill-unity-urp) - Unity URP RenderGraph skill (available via federation)
