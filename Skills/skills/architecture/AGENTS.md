# Architecture Skill Routing

Use this category when the request is about durable technical direction, boundary drift, or high-risk decisions.

## Route By Intent

- `architecture-decision`: use for ADR-style choices that need context, alternatives, consequences, owner, and verification.
- `architecture-review`: use for reviewing an existing design or implementation against LUX boundaries and invariants.
- `core-invariants`: use whenever `.lux` SSoT, subsystem ownership, atomicity, idempotency, consistency, or no-silent-fallback may be at risk.
- `ldp-decision-protocol`: use for product, automation, privacy, fairness, IP, or ethics decisions that need PASS/REVIEW/REJECT framing.

## Do Not Use For

- Routine code review. Use `../review/AGENTS.md`.
- Unity command operation. Use `../workflow/lux-unity`.
