---
name: test-helpers
description: "Lazy-load only when this workflow is explicitly needed. Use when adding or refactoring helper utilities for tests. Keep helpers small, deterministic, local to the test domain, and named by behavior. Avoid hiding assertions or creating broad fixtures that mask failures."
---

# Test Helpers

Use when adding or refactoring helper utilities for tests. Keep helpers small, deterministic, local to the test domain, and named by behavior. Avoid hiding assertions or creating broad fixtures that mask failures.

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
