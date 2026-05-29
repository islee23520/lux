---
name: regression-suite
description: "Lazy-load only when this workflow is explicitly needed. Use to design or run regression checks for changed behavior. Prefer targeted tests first, then typecheck/lint/build/smoke. Include command, expected signal, observed result, and remaining untested risk."
---

# Regression Suite

Use to design or run regression checks for changed behavior. Prefer targeted tests first, then typecheck/lint/build/smoke. Include command, expected signal, observed result, and remaining untested risk.

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
