# Lux Spec Orchestrator

OpenCode adapter for LUX's agent-neutral, spec-driven Unity game development orchestration. This adapter is one optional client integration; the canonical workflows live in `.agents/skills/` and `.lux/` so other compatible agents can consume the same project state through CLI/API/MCP surfaces.

## What It Does

- Monitors client session idle events from the OpenCode adapter
- Evaluates current project state against `.lux/spec.json`
- Injects continuation messages to keep development aligned with specs
- Auto-manages project glossary

## Installation

This optional adapter is installed by `lux bridge install --install-opencode-adapter` into `.opencode/plugins/lux/` for OpenCode users, or refreshed when a project already has an `.opencode/` adapter directory. Other agents should use the same LUX CLI/API and `.agents/skills/` surfaces without treating OpenCode as the default owner.

## Configuration

Default configuration:
- `maxContinuations`: 50 (per-session continuation limit)
- `specPath`: `.lux/spec.json`
- `glossaryPath`: `.lux/glossary.md`

> [!NOTE]
> Legacy `.lux/continuation-state.json` is deprecated. The plugin now uses the internal gateway state for tracking.

## How It Works

1. When a compatible client session becomes idle, the adapter evaluates the spec
2. If ambiguity is high or work is incomplete, it injects a continuation prompt
3. The continuation counter prevents infinite loops (max 50 by default)
4. Glossary terms discovered during development are auto-appended

### Canonical Stop Reasons

The orchestrator stops continuation when:
- `max_continuations_reached`: The session hit the limit (default 50).
- `spec_satisfied`: Ambiguity score is below threshold and all requirements met.
- `manual_intervention`: User explicitly stopped the loop.
- `stagnation_detected`: No progress made across multiple continuations.
