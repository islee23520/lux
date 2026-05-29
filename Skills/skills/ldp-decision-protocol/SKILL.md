---
name: ldp-decision-protocol
description: "Lazy-load only when this workflow is explicitly needed. Use for high-risk product, automation, or ethical decisions. Run: clarify intent; quantify evidence; check user/data/IP/fairness risks; define stop and rollback conditions; record approval. Verdicts: PASS, REVIEW, or REJECT."
category: workflow
source: lux
---

# LDP Decision Protocol

Use for high-risk product, automation, or ethical decisions. Run: clarify intent; quantify evidence; check user, data, IP, and fairness risks; define stop and rollback conditions; record approval. Verdicts: PASS, REVIEW, or REJECT.

## Passive Loading Rule

Do not preload this skill during startup, hot reload, or background indexing. Read it only when the user request directly matches this workflow.

## Minimal Procedure

1. Confirm the target result and affected LUX subsystem.
2. Read only the files or evidence needed for the decision.
3. Apply the decision checklist without expanding scope.
4. Produce concise output with evidence, risks, and the next verified action.

## Decision Checklist

- Intent: state the user-visible decision and who is affected.
- Evidence: separate measured facts from assumptions.
- Ethics: check privacy, consent, IP, fairness, exploitative incentives, and destructive automation risk.
- Stop rule: define rollback, abort, or escalation conditions.
- Approval: return PASS, REVIEW, or REJECT with the smallest verified next action.

## Output

- Verdict.
- Evidence used.
- Risks or blockers.
- Verification performed or the smallest remaining check.
