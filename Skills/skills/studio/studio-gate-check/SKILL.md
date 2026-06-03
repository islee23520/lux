---
name: studio-gate-check
description: "Check whether a project is ready to advance to the next development phase by verifying artifacts, quality evidence, manual unknowns, and advisory reviewer feedback."
category: studio
source: chronos-adapted
source_path: .claude/skills/gate-check/SKILL.md
---

# Studio Gate Check

Use this skill to assess whether a project is ready to move from one development phase to the next.

This skill is prescriptive: it answers **"Are we ready to advance?"** rather than **"What phase are we in?"**

## Supported Stage Model

Use this generic stage sequence unless the repository defines its own equivalent:

1. Concept
2. Systems Design
3. Technical Setup
4. Pre-Production
5. Production
6. Polish
7. Release

If the project uses different names, map them explicitly before continuing.

## Phase 1 — Parse the Target Gate

If the user supplies a target phase, validate readiness for that transition.

If they do not, infer the current stage from available artifacts and confirm the inferred transition with `AskUserQuestion` before proceeding.

Do not skip that confirmation when the stage is inferred.

## Phase 2 — Define Gate Requirements

For the target transition, build a checklist of:

- required artifacts
- quality checks
- cross-reference checks
- manual checks that cannot be automated

Use repository evidence first. If the project already documents phase criteria, prefer that over generic assumptions.

### Default gate expectations

#### Concept → Systems Design

- concept brief exists
- pillars or design principles exist
- core loop and audience are defined

#### Systems Design → Technical Setup

- systems index exists
- MVP-level design docs exist
- design docs have been reviewed for internal consistency

#### Technical Setup → Pre-Production

- engine and technical baseline are chosen
- architecture docs and key decisions exist
- test framework exists
- at least minimal UX and accessibility intent exists

#### Pre-Production → Production

- playable prototype or vertical slice exists
- first sprint plan exists
- core design docs and architecture are coherent
- key playtest evidence exists

#### Production → Polish

- core gameplay path is playable end to end
- tests and QA evidence exist
- playtesting has identified and addressed major issues

#### Polish → Release

- features and content are complete
- regression checks pass
- release readiness artifacts exist

## Phase 3 — Run the Checks

For each requirement:

### Artifact checks

- verify file or directory existence
- verify content is meaningful, not just a placeholder
- verify counts or coverage when the gate implies more than one artifact

### Quality checks

- run tests when the gate depends on test health
- read review reports, architecture notes, or design evidence where required
- compare implementation against plans when the gate depends on coherence

### Cross-reference checks

- compare design docs with source structure
- compare sprint or milestone plans with actual work items
- compare architecture decisions with implementation patterns

## Phase 4 — Handle Unknowns Collaboratively

Some checks cannot be proven automatically.

Ask the user directly for items such as:

- whether the core loop has been playtested
- whether the experience is fun enough to move on
- whether informal testing exists when no report file is present
- whether performance evidence exists when no profiling output is available

Never silently convert unknowns into passes.

Mark them as **Manual Check Needed** until confirmed.

## Phase 5 — Optional Reviewer Panel

If the decision is high stakes, optionally spawn Task-based reviewers in parallel, such as:

- creative reviewer
- technical reviewer
- production reviewer
- visual or UX reviewer

Pass them:

- target phase
- artifact summary
- the most relevant context for their discipline

Treat reviewer outputs as advisory:

- any strong "not ready" signal should prevent a clean PASS
- concern-level feedback should raise the verdict floor to CONCERNS

## Phase 6 — Draft the Verdict

Use this structure:

```markdown
## Gate Check: [Current Phase] → [Target Phase]

### Required Artifacts
- [x] Present item
- [ ] Missing item

### Quality Checks
- [x] Passing item
- [ ] Failed item
- [?] Manual Check Needed

### Blockers
1. [Blocking issue]

### Recommendations
- [Priority action]
- [Optional improvement]

### Verdict
[PASS / CONCERNS / FAIL]
```

Verdict rules:

- **PASS** — required artifacts present and quality evidence is sufficient
- **CONCERNS** — advancement is possible but meaningful risks remain
- **FAIL** — critical blockers must be resolved first

## Phase 7 — Challenge the Verdict

Before finalizing, generate several challenge questions intended to disprove the draft verdict.

Examples:

- Which checks were directly verified versus inferred?
- Did any unknowns get treated too generously?
- Are any "concerns" actually blocking in practice?
- What is the single weakest piece of evidence behind this verdict?

Answer the questions by re-checking evidence or asking the user. Revise the verdict if needed.

Include a short **Chain-of-Verification** note in the final output.

## Phase 8 — Optional Stage Update

If the project tracks the current phase in a file and the verdict is PASS, ask before writing the new stage value.

Never update phase-tracking files without explicit user approval.

## Phase 9 — Close with Next-Step Choices

After the verdict, give 2 or 3 sensible next-step options using `AskUserQuestion` when appropriate.

Examples:

- proceed to the next planning or implementation workflow
- fill the most important missing artifact
- stop for this session

Tailor the options to the gate that just ran.

## Follow-Up Guidance

Base follow-up recommendations on what is missing.

Examples:

- missing concept work → brainstorm or concept review
- missing systems docs → system mapping and design docs
- missing architecture → architecture planning and decisions
- missing tests or QA → test setup, QA planning, or smoke checks
- missing prototype evidence → prototype and playtest workflows

Verdict: **COMPLETE** when the gate verdict, evidence, blockers, and next action are clear.
