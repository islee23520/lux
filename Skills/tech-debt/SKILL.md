---
name: tech-debt
description: "Lazy-load only when this workflow is explicitly needed. Use to document or reduce debt. Describe symptom, cost, affected files, invariant risk, simplest cleanup, tests needed, and what not to change. Prefer deletion and reuse over new abstractions."
---

# Technical Debt

Use to document or reduce debt. Describe symptom, cost, affected files, invariant risk, simplest cleanup, tests needed, and what not to change. Prefer deletion and reuse over new abstractions.

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
