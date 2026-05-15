# Lux Spec Orchestrator

OpenCode plugin for spec-driven Unity game development orchestration.

## What It Does

- Monitors OpenCode session idle events
- Evaluates current project state against `.lux/spec.json`
- Injects continuation messages to keep development aligned with specs
- Auto-manages project glossary

## Installation

This plugin is auto-installed by `lux bridge install` into `.opencode/plugins/lux/`.

## Configuration

Default configuration:
- `maxContinuations`: 50 (per-session continuation limit)
- `specPath`: `.lux/spec.json`
- `glossaryPath`: `.lux/glossary.md`

> [!NOTE]
> Legacy `.lux/continuation-state.json` is deprecated. The plugin now uses the internal gateway state for tracking.

## How It Works

1. When an OpenCode session becomes idle, the plugin evaluates the spec
2. If ambiguity is high or work is incomplete, it injects a continuation prompt
3. The continuation counter prevents infinite loops (max 50 by default)
4. Glossary terms discovered during development are auto-appended

### Canonical Stop Reasons

The orchestrator stops continuation when:
- `max_continuations_reached`: The session hit the limit (default 50).
- `spec_satisfied`: Ambiguity score is below threshold and all requirements met.
- `manual_intervention`: User explicitly stopped the loop.
- `stagnation_detected`: No progress made across multiple continuations.
