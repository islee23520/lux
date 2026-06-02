---
description: Run the Lux spec-driven development loop
subtask: true
---

Start the Lux spec-driven development iteration loop. This evaluates the current project state against `.lux/specs/` and drives the next evidence-gated development step.

Workflow:

1. Read `.lux/specs/spec.json`, `.lux/specs/gdd.md`, domain specs, and decision/preference ledgers to understand the GDD SSoT
2. Evaluate current project state, Socratic ambiguity, engine capability routing, and active blockers
3. If ambiguity is above threshold, ask clarifying questions first
4. Determine the next goal from `.lux/specs`, tickets, and run-state evidence
5. Begin only one bounded development step for that goal
6. After the step, verify progress with command output, context snapshots, manual QA evidence, or explicit blocker evidence

Execute the evaluation:

```bash
lux spec validate
```

Then review `.lux/specs/` and the current project state. Identify the highest-priority incomplete step, but do not claim completion from intent alone. Unity is the primary verified engine surface; Godot and Three.js remain capability-routed and must record unsupported manual QA observations as blockers rather than fake parity.
