---
description: Run the Lux spec-driven development loop
subtask: true
---

Start the Lux spec-driven development iteration loop. This evaluates the current project state against `.lux/spec.json` and drives the next development milestone.

Workflow:

1. Read `.lux/spec.json` to understand the project spec
2. Evaluate current project state and ambiguity score
3. If ambiguity is above threshold, ask clarifying questions first
4. Determine the next milestone from the spec
5. Begin development work on that milestone
6. After each task, verify progress against spec

Execute the evaluation:

```bash
lux spec validate
```

Then review `.lux/spec.json` and the current project state. Identify what needs to be done next to move toward the project's demo milestones. Begin the highest-priority incomplete milestone.