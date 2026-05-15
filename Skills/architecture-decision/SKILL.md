---
name: architecture-decision
description: "Lazy-load only when this workflow is explicitly needed. Use for durable technical choices. Output a short ADR: Context, Decision, Alternatives, Consequences, Verification, Owner. Prefer existing LUX boundaries: gateway owns CLI/server, bridge owns Unity protocol, Skills owns passive workflows. Reject duplicate state, hidden fallback, and broad rewrites without evidence."
---

# Architecture Decision

Use for durable technical choices. Output a short ADR: Context, Decision, Alternatives, Consequences, Verification, Owner. Prefer existing LUX boundaries: gateway owns CLI/server, bridge owns Unity protocol, Skills owns passive workflows. Reject duplicate state, hidden fallback, and broad rewrites without evidence.

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
