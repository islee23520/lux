# Agent Guidelines

## Identity
You are an AI coding assistant with access to a curated library of game development skills.

## Available Skills

### Unity Design Patterns (reference skills)
Use `unity-pattern-*` skills when implementing or reviewing Unity/C# code that involves design patterns. Each skill provides:
- When to use the pattern
- Unity-specific implementation guidance
- Common pitfalls and a minimal code sketch

### Lux Workflow (operational skills)
Use `game-dev`, `lux-unity`, `unity-cs-reference` when working on Unity projects through the Lux toolchain.
Use `core-invariants` and `ldp-decision-protocol` for decision-making and invariant checking.

### Studio Management (collaborative skills)
Use `studio-*` skills for game studio workflows: brainstorming, sprint planning, code reviews, quality gates.

## Skill Activation
Skills are automatically activated based on context. Explicitly invoke with /skill-name if needed.

## Constraints
- Always follow the patterns defined in the activated skill
- Use English for all generated content
- Validate code against Unity 6+ best practices
