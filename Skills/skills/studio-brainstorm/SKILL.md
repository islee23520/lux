---
name: studio-brainstorm
description: "Guided game concept ideation from vague ideas to a structured concept brief, using collaborative questioning, concept generation, loop design, and scope validation."
category: studio
source: chronos-adapted
source_path: .claude/skills/brainstorm/SKILL.md
---

# Studio Brainstorm

Use this skill to turn a rough genre hint, theme hint, or blank slate into a structured game concept through collaborative ideation. Keep the human in the loop at every major decision point.

## Operating Rules

- Adapt to the user's hint if they provide one. If they provide nothing or say `open`, start from first principles.
- Do not silently invent the whole concept. Ask targeted questions and use `AskUserQuestion` at decision points.
- Keep the core workflow intact: discovery → concept generation → loop design → pillars → audience validation → feasibility → concept brief.
- Replace project-specific assumptions with generic references such as "the project concept document" or "the project design folder".
- If the workspace already contains a concept brief or pillar document, read it first and resume instead of restarting.

## Phase 1 — Creative Discovery

Understand the person before the project.

Ask conversationally about:

- Emotional anchors: memorable play moments, fantasies, power fantasies, or experiences they have been chasing.
- Taste profile: games they return to, genres they love or avoid, and why.
- Intent: whether they want challenge, relaxation, story, expression, mastery, discovery, or social play.
- Constraints: timeline, team size, experience level, platform ambitions, and appetite for scope.

Use `AskUserQuestion` for constrained choices, but leave open-text room for specific game names and nuanced preferences.

Then synthesize the answers into a short **Creative Brief** that captures:

- the emotional promise
- the taste profile
- the practical constraints
- the kind of project this can realistically become

Read the brief back and confirm it matches the user's intent.

## Phase 2 — Concept Generation

Generate **3 distinct concepts** from the brief. Each concept should come from a different ideation lens:

1. **Verb-first** — define the central player action first.
2. **Mashup** — combine two unexpected ingredients.
3. **Experience-first** — start with the desired player feeling and work backward.

For each concept, provide:

- Working Title
- Elevator Pitch
- Core Verb
- Core Fantasy
- Unique Hook
- Primary Player Emotion or Aesthetic
- Estimated Scope
- Why It Could Work
- Biggest Risk

After presenting all 3, use `AskUserQuestion` with a plain option list:

- Concept 1 — [Title]
- Concept 2 — [Title]
- Concept 3 — [Title]
- Combine elements across concepts
- Generate fresh directions

If the user combines concepts or asks for new directions, iterate before moving on.

## Phase 3 — Core Loop Design

For the selected concept, collaboratively define the loop at four levels.

### 30-second loop

Ask about:

- the feel of the primary action
- the most important design dimension for this concept
- what makes the action intrinsically satisfying

Generate choices dynamically from the chosen concept rather than using canned options. Always include a free-text escape hatch.

### 5-minute loop

Define:

- the short-cycle structure
- the "one more run" or "one more turn" hook
- the meaningful decisions the player makes at this scale

### Session loop

Define:

- what a full play session looks like
- where natural stopping points happen
- what thought or tension pulls the player back between sessions

### Progression loop

Define:

- how the player grows
- whether growth is power, knowledge, options, mastery, story, or expression
- what the long-term goal is

Also analyze motivation through autonomy, competence, and relatedness.

## Phase 4 — Pillars and Boundaries

Define **3 to 5 pillars** and **at least 3 anti-pillars**.

For each pillar include:

- a short name
- a one-sentence definition
- a design test that helps choose between competing options

For each anti-pillar include:

- what the project will explicitly not do
- why that protects the concept

Use `AskUserQuestion` to confirm whether the pillar set feels right. If the user wants revisions, revise and confirm again until they explicitly lock the set.

If deeper review would help, optionally spawn parallel Task-based reviewers such as:

- a creative reviewer for concept clarity and pillar strength
- a visual or art-oriented reviewer for possible visual identity directions

When using Task-based reviewers:

- pass only the concept summary, pillars, anti-pillars, and known platform/context
- avoid project-specific gate names or private docs
- present the reviewer findings together and let the user decide what to adopt

If a visual direction is selected, store it as a **Visual Identity Anchor** with:

- direction name
- one-line rule
- 2 to 3 supporting principles
- a short color or mood philosophy

## Phase 5 — Audience Validation

Validate who the game is for and who it is not for.

Cover:

- primary player type
- secondary appeal
- clear non-audience
- comparable games or audience patterns in the market

Use lightweight frameworks such as Bartle or broader motivation models only as thinking tools, not rigid labels.

## Phase 6 — Scope and Feasibility

Ground the idea in reality.

Ask about:

- target platform
- prior engine familiarity or engine preference
- art style and production cost
- content expectations
- MVP definition
- technical, design, and market risks
- scope tiers: full vision versus minimum shippable version

If the user has no engine preference, do not force a recommendation. Suggest a dedicated engine-selection workflow later.

If the concept carries technical uncertainty, optionally spawn a Task-based technical reviewer with:

- core loop summary
- target platform
- engine preference or undecided state
- identified risks

If the overall sprint or milestone scope seems unrealistic, optionally spawn a Task-based production reviewer to challenge scope and timeline assumptions.

Treat reviewer outputs as advisory. Surface risks clearly and revise with the user if needed.

## Phase 7 — Concept Brief Generation

Generate a structured concept brief from the conversation. Write it only after explicit user approval.

The brief should include at minimum:

- core identity
- elevator pitch
- core fantasy and hook
- loop breakdown
- player motivation summary
- pillars and anti-pillars
- target audience
- platform and engine context
- MVP definition
- scope tiers
- major risks
- visual identity anchor if one was chosen

Use `AskUserQuestion` for approval:

- Yes — write it
- Not yet — revise a section first

If the user requests revisions, revise the requested section, show what changed, and ask again.

## Phase 8 — Suggested Next Steps

After the concept brief is approved or written, suggest the next planning steps in order, tailored to the project:

1. choose or confirm the engine and technical baseline
2. define visual identity or art direction
3. review the concept for completeness
4. map the game into systems
5. write per-system design docs
6. define technical architecture
7. record major architecture decisions
8. run a readiness or gate check before production commitment
9. prototype the riskiest part
10. playtest and validate the hypothesis
11. plan the first sprint

## Output Contract

End with a concise summary containing:

- chosen concept
- elevator pitch
- pillars
- primary audience
- platform or engine context
- biggest risk
- whether a concept brief was written

Verdict: **COMPLETE** when the concept is clear and the next step is identified.
