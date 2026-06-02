# Lux Spec Orchestrator

Legacy OpenCode template bundle for spec-driven Unity game development orchestration.

## What It Does

- Monitors OpenCode session idle events
- Evaluates current project state against canonical `.lux/specs/spec.json`
- Uses `.lux/spec.json` only as a legacy compatibility fallback
- Injects continuation messages to keep development aligned with specs
- Auto-manages project glossary

## Installation

This legacy template bundle is not the default runtime integration target. The server/MCP gateway is the supported control surface.

## Configuration

Default configuration:
- `maxContinuations`: 50 (per-session continuation limit)
- `specPath`: `.lux/specs/spec.json`
- `glossaryPath`: `.lux/glossary.md`

> [!NOTE]
> Legacy `.lux/continuation-state.json` is deprecated. The gateway owns active run lifecycle state in `.lux/run-state.json`; plugin reads/writes must go through gateway APIs rather than maintaining a second state file.

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
