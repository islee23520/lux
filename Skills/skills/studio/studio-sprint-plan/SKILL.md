---
name: studio-sprint-plan
description: "Create, update, or summarize a sprint plan from current milestones, prior velocity, ready work, risks, and available capacity, with an optional feasibility review."
category: studio
source: chronos-adapted
source_path: .claude/skills/sprint-plan/SKILL.md
---

# Studio Sprint Plan

Use this skill to generate a new sprint plan, update an existing one, or summarize sprint status.

Supported modes:

- `new`
- `update`
- `status`

If no mode is supplied, infer the simplest sensible default from context and state the choice.

## Phase 1 — Gather Planning Context

Read the minimum planning evidence needed:

- current milestone or release goal
- previous sprint plans if they exist
- in-progress sprint status if it exists
- design docs or task backlogs that appear implementation-ready
- current risk log, blockers, or dependency notes

Build a short planning picture of:

- what matters most now
- what carried over
- realistic team capacity
- what is ready versus still fuzzy

## Phase 2 — Generate the Sprint Plan or Status Report

### For `new`

Generate a sprint plan with these sections:

- Sprint title and dates
- Sprint Goal
- Capacity
- Must Have
- Should Have
- Nice to Have
- Carryover
- Risks
- External Dependencies
- Definition of Done

Use estimates, dependencies, and acceptance criteria for each task. Prefer a compact table when the content fits.

### For `update`

Read the existing sprint plan and current status, then:

- preserve work already completed or in progress
- carry forward unchanged items
- add newly ready work
- remove or downgrade work that no longer fits
- clearly note what changed from the prior version

### For `status`

Generate a short sprint status report containing:

- overall progress
- completed work
- in-progress work
- not-started work
- blocked work
- burndown or on-track assessment
- emerging risks

## Phase 3 — Optional Structured Status File

If the repository already uses a machine-readable sprint tracker, update or propose updating it after the markdown plan is ready.

If no such file exists, do not invent one unless the user explicitly asks. You may still suggest a simple YAML or JSON tracker as an optional improvement.

## Phase 4 — Feasibility Review

Before asking to write the final sprint plan, challenge the plan for realism.

Optionally spawn a Task-based production or planning reviewer with:

- proposed work list
- estimates
- team capacity
- carryover
- milestone constraints

Use the review to detect:

- overloaded scope
- hidden dependencies
- insufficient buffer
- tasks that should be demoted from Must Have

If the review returns concerns, revise the plan before asking for final write approval.

## Phase 5 — QA Readiness Check

Check whether the sprint has explicit QA expectations.

If a QA plan, test checklist, or acceptance-test artifact already exists, reference it.

If none exists, call it out explicitly:

- the sprint can still be planned
- but completion and sign-off will be ambiguous without QA expectations

Use `AskUserQuestion` to let the user choose whether to create the QA plan now or knowingly continue without it.

## Phase 6 — Write Approval

Present the draft first.

Then ask for approval before writing or updating files. If the user declines, keep the generated plan in the conversation and mark the workflow as blocked on approval.

If the user approves, write the sprint plan to the repository's sprint-planning location or the most obvious equivalent already present in the project.

## Recommended Plan Shape

```markdown
# Sprint [N] — [Start] to [End]

## Sprint Goal

## Capacity

## Must Have

## Should Have

## Nice to Have

## Carryover

## Risks

## External Dependencies

## Definition of Done
```

## Follow-Up Guidance

After the sprint plan is written or approved, recommend the next most logical actions, such as:

- confirm QA expectations
- validate story readiness
- start the first implementation item
- track sprint status mid-sprint
- run a scope check if the sprint expanded beyond the original milestone intent

Verdict: **COMPLETE** when the sprint plan or status report is produced and the next action is clear.
