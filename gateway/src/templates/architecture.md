# Architecture Spec

## Phase 1: Experience Lens
- Which technical choices directly affect player feel?
- What latency, loading, or reliability expectations matter most?

## Phase 2: Elemental Tetrad
- Mechanics: Which systems own the game rules?
- Story: Which data models represent narrative state?
- Aesthetics: Which rendering and asset pipelines are required?
- Technology: Which Unity packages, services, and platforms are fixed?

## Phase 3: Core Loop Stress Test
- Which runtime systems participate in one complete loop?
- Where can the loop fail under load or missing content?

## Phase 4: Player Motivation
- Which persistence, progression, or analytics systems support retention?
- How are saves, profiles, and unlocks represented?

## Phase 5: Honest Assessment
- What architectural risk is most likely to block production?
- What needs a spike before implementation expands?

## Domain Questions
- What are the major modules and ownership boundaries?
- What data must be authored, generated, loaded, or cached?
- What platform constraints shape the architecture?
