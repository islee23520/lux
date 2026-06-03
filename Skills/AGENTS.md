# Agent Guidelines

## Identity
You are an AI coding assistant with access to a curated library of game development skills.

## Available Skills

Skills are grouped by category under `Skills/skills/`. Each category owns an `AGENTS.md` routing file that defines when to load the skills inside it.

| Category | Route file | Use when |
| --- | --- | --- |
| `architecture/` | `Skills/skills/architecture/AGENTS.md` | Durable architecture, invariants, and high-risk decisions |
| `review/` | `Skills/skills/review/AGENTS.md` | Code review, studio review, security, and performance |
| `workflow/` | `Skills/skills/workflow/AGENTS.md` | LUX game-dev and Unity MCP/bridge operations |
| `unity/` | `Skills/skills/unity/AGENTS.md` | Unity API reference and Unity design patterns |
| `studio/` | `Skills/skills/studio/AGENTS.md` | Brainstorming, sprint planning, gates, and workflow help |
| `quality/` | `Skills/skills/quality/AGENTS.md` | Regression, smoke, test setup/helpers, and releases |
| `bugs/` | `Skills/skills/bugs/AGENTS.md` | Bug reports, triage, debt, retrospectives, changelog |

## Skill Activation
Do not treat `Skills/skills` as a flat dump. First select the category by intent, read that category's `AGENTS.md`, then load the narrowest matching skill.

## Constraints
- Always follow the patterns defined in the activated skill
- Use English for all generated content
- Validate code against Unity 6+ best practices
