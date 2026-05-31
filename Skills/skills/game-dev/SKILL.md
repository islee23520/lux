---
name: game-dev
description: "Lazy-load only when this workflow is explicitly needed. Use for compact game planning in LUX. Validate concept, core loop, mechanics, progression, technical feasibility, QA plan, ethics, and context-first engine observation. Check the player promise, target audience, prototype scope, Unity risks, scene/component/coordinate/UI/camera/log evidence, telemetry/privacy, accessibility, non-exploitative retention, and measurable playtest criteria."
category: workflow
source: lux
---

# Game Development

Use for compact game planning in LUX. Validate concept, core loop, mechanics, progression, technical feasibility, QA plan, ethics, and context-first engine observation. Check the player promise, target audience, prototype scope, Unity risks, scene/component/coordinate/UI/camera/log evidence, telemetry/privacy, accessibility, non-exploitative retention, and measurable playtest criteria.

## Game Context Rule

Treat LUX game development as **context-first, vision-supplemented**.

- First capture or request text/JSON evidence: `.lux/specs` intent, scene hierarchy, selected object identity, components/properties, Transform/RectTransform/Collider values, camera/UI coordinate state, console/compile logs, and PlayMode/input state.
- Use screenshots or vision feedback as supporting evidence, not as a standalone completion signal.
- When a visual symptom is mentioned, connect it back to a GameObject, component, coordinate value, camera/UI state, log entry, or explicit engine capability blocker.
- If the current engine surface cannot provide the needed observation, report a blocker instead of pretending the game state was verified.

## Passive Loading Rule

Do not preload this skill during startup, hot reload, or background indexing. Read it only when the user request directly matches this workflow.

## Minimal Procedure

1. Confirm the target result and affected LUX subsystem.
2. Read only the files or evidence needed for the decision.
3. Apply the checklist above without expanding scope.
4. Require context-first engine evidence before accepting screenshot/vision-only claims.
5. Produce concise output with evidence, risks, and the next verified action.

## Output

- Result or verdict.
- Evidence used.
- Risks or blockers.
- Verification performed or the smallest remaining check.
