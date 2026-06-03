---
name: studio-help
description: "Quickly orient the user in the project workflow, identify the most likely current phase, and recommend the single best next action with concise supporting context."
category: studio
source: chronos-adapted
source_path: .claude/skills/help/SKILL.md
---

# Studio Help

Use this skill when the user asks what to do next, says they are stuck, or needs a lightweight workflow orientation.

This skill is read-only. It should be fast, practical, and short.

## Step 1 — Read Available Workflow Context

Look for any repository-level workflow definitions, progress trackers, stage files, sprint files, or active-session notes.

If a formal workflow catalog exists, use it.

If not, infer the workflow from the repository structure and existing planning artifacts.

## Step 2 — Identify Installed Skills and Useful Commands

Scan installed skill files and collect the most relevant ones for the user's likely current phase.

If some skills are not part of the primary workflow, they can still be listed as optional tools.

Limit optional suggestions to the most relevant items.

## Step 3 — Determine the Current Phase

Check for a phase-tracking file first if one exists.

If not, infer the most advanced likely phase from artifacts such as:

- concept documents
- systems design docs
- architecture docs
- sprint plans or stories
- source code volume
- QA or release artifacts

Prefer the most advanced artifact set that is actually present.

## Step 4 — Read Active Session Context

If the repository contains an active session or progress note, read it to identify:

- what was worked on most recently
- what is currently blocked
- what the user may have just finished

Use this to personalize the answer.

## Step 5 — Identify Completion and the Next Required Step

Determine:

1. the last clearly completed step
2. the current blocker or first missing required step
3. optional things that can be done now without blocking progress
4. the next one or two required steps after that

If the user's message mentions that they just finished something, use that as extra evidence when the repository signals are ambiguous.

## Step 6 — Ask About Unverifiable Manual Steps

If an important step cannot be confirmed from files alone, ask the user directly instead of guessing.

Examples:

- whether a review already happened informally
- whether a prototype was playtested
- whether a milestone decision was already made off-record

## Step 7 — Present a Short Orientation

Use a compact structure like this:

```markdown
## Where You Are: [Phase]

**In progress:** [recent work, if known]

### Done
- [confirmed completed step]

### Next up (required)
**[step name]** — [why it is next]
Command or workflow: `[name]`

### Also available
- [optional step] → `[name]`

### Coming up after that
- [future step]
```

Formatting rules:

- recommend one primary next action
- keep optional suggestions secondary
- keep the whole answer lightweight, not a full audit

## Step 8 — Gate Warning When Relevant

If the project appears close to a phase transition, mention that a formal gate or readiness check is the next milestone.

Do not mention this if the project is still clearly far from a gate.

## Step 9 — Escalation Paths

If the user sounds confused or overwhelmed, include one or two deeper follow-up options such as:

- a full stage-detection or audit workflow
- a formal gate check
- a concept, design, or sprint-planning workflow depending on context

Keep this section minimal and only show it when it is genuinely useful.

## Collaborative Rules

- Never auto-run the next skill.
- Recommend the next step clearly and let the user choose.
- Match the user's tone.
- If they sound stressed, give one concrete action rather than a long menu.

Verdict: **COMPLETE** when the user can see exactly what to do next.
