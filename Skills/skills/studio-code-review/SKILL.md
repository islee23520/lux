---
name: studio-code-review
description: "Review a file or small set of files for standards, architecture, SOLID, performance, testability, and implementation risks, with optional specialist subreviews."
category: studio
source: chronos-adapted
source_path: .claude/skills/code-review/SKILL.md
---

# Studio Code Review

Use this skill to perform a structured review of implementation quality on a target file, directory, or focused change set.

This skill is read-only unless the user explicitly asks for fixes in a separate step.

## Phase 1 — Load Target Context

Read the target file or files in full.

Also read any nearby coding standards or architecture guidance that exists in the repository, such as:

- `README`
- contributor guides
- architecture docs
- ADRs
- lint or test configuration

If no formal standards are present, infer conventions from neighboring code rather than inventing a style.

## Phase 2 — Identify Relevant Specialists

Determine whether this review would benefit from parallel Task-based specialists.

Examples:

- language or engine specialist for core runtime code
- UI specialist for presentation code
- shader or rendering specialist for graphics code
- QA or testability specialist for logic-heavy changes

Only spawn specialists that clearly fit the target. Avoid project-specific role names when they are not available.

## Phase 3 — ADR and Architecture Intent Check

Look for architecture references in:

- ADR files
- nearby design docs
- story or task files
- code comments or headers

If ADRs or design decisions are referenced, read the relevant parts and compare the implementation against them.

Classify deviations as:

- **Architectural Violation** — contradicts an explicit decision
- **Architecture Drift** — meaningfully diverges without clear justification
- **Minor Deviation** — small difference that does not alter the main design intent

If no references exist, note that the ADR compliance check was skipped.

## Phase 4 — Standards Compliance

Evaluate the implementation against repository conventions and these common checks:

- documentation on public APIs where the codebase expects it
- manageable method size and complexity
- dependency injection or clean dependency boundaries where appropriate
- configuration separated from hardcoded values where the codebase expects that
- interface use versus unnecessary concrete coupling

Do not treat every checklist item as universal law. Match the local codebase style.

## Phase 5 — Architecture and SOLID

Check:

### Architecture

- dependency direction is sensible
- layers are not inverted
- UI does not own core state unless that is the project pattern
- cross-system communication uses the project's preferred coordination pattern
- the code is consistent with established patterns nearby

### SOLID

- single responsibility
- open for extension where appropriate
- subtype substitutability
- interface segregation
- dependency inversion

Call out concrete examples with file and line references when possible.

## Phase 6 — Runtime and Operational Concerns

Check for practical risks relevant to the code under review:

- frame or time-step correctness if real-time logic exists
- unnecessary allocations in hot paths
- null or empty-state handling
- thread-safety or async safety where applicable
- cleanup and lifecycle correctness
- error handling and observability

Tailor this section to the technology involved rather than forcing game-only rules onto every review.

## Phase 7 — Parallel Specialist Reviews

If specialists are useful, spawn them in parallel via Task.

Possible specialist prompts:

- architecture specialist: review dependency boundaries and system fit
- language specialist: review idioms, correctness, and maintainability
- UI specialist: review state ownership, rendering concerns, and accessibility
- QA specialist: review testability, seams, hooks, and acceptance-criteria coverage

Collect all findings before issuing the final review.

## Phase 8 — Output Format

Use this structure:

```markdown
## Code Review: [File or System Name]

### Specialist Findings
[Summarize specialist input or say none were used]

### ADR / Architecture Intent
[Compliant / Drift / Violation / Not checked]

### Standards Compliance
[Key passes and failures]

### Architecture
[Clean / Minor Issues / Violations Found]

### SOLID
[Compliant / Issues Found]

### Runtime and Operational Concerns
[List relevant risks]

### Positive Observations
[Always include what is done well]

### Required Changes
[Must-fix items]

### Suggestions
[Non-blocking improvements]

### Verdict
[Approved / Approved with Suggestions / Changes Required]
```

## Phase 9 — Next-Step Guidance

- If approved: recommend the next workflow step such as merge, close the task, or continue implementation.
- If changes are required: list the smallest concrete changes needed before re-review.
- If an architectural violation exists: recommend updating the implementation or recording a fresh architecture decision before continuing.

Verdict: **COMPLETE** when the review is fully written and evidence-backed.
