---
name: bug-report
description: "Lazy-load only when this workflow is explicitly needed. Use to turn observed failures into actionable reports. Capture summary, environment, exact steps, expected result, actual result, logs, suspected subsystem, severity, reproducibility, and minimal next diagnostic. Do not invent fallback data."
---

# Bug Report

Use to turn observed failures into actionable reports. Capture summary, environment, exact steps, expected result, actual result, logs, suspected subsystem, severity, reproducibility, and minimal next diagnostic. Do not invent fallback data.

## Passive Loading Rule

Do not preload this skill during startup, hot reload, or background indexing. Read it only when the user request directly matches this workflow.

## Minimal Procedure

1. Confirm the target result and affected LUX subsystem.
2. Read only the files or evidence needed for the decision.
3. Apply the checklist above without expanding scope.
4. Produce concise output with evidence, risks, and the next verified action.

## Output

- Result or verdict.
- Evidence used.
- Risks or blockers.
- Verification performed or the smallest remaining check.
