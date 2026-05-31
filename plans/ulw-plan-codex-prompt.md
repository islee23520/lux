# ULW Plan Codex Prompt

Work in `/Users/ilseoblee/workspace/linalab/lux`.

We are not implementing a large feature yet. First produce a concrete LUX plan for game-development verification ontology and suite layer split.

Context to preserve:
- LLMs do not inherently understand game coordinate systems.
- Conventional TDD alone cannot prove visual correctness in game development.
- OMO-style development AI cannot be strengthened for games unless the definition of how to verify game outputs is injected beforehand as domain ontology.
- Users see problems visually; converting vision to text and then making the agent decode it loses meaning.
- A game scene should be treated like a page/base stage layer. Build a scene-structure AST and match it against vision/screenshot evidence so AI coding can be engineering-strengthened.
- LUX is currently too heavy as a codebase; split the suite into layers.

Plan target:
1. Define LUX Suite layers with clear boundaries:
   - Game Verification Ontology layer
   - Scene AST extraction layer
   - Coordinate/Camera/UI mapping layer
   - Vision-to-AST matching layer
   - Evidence/TDD harness layer
   - AI prompt/context injection layer
   - UX/dashboard layer
2. Identify exact repo files likely affected.
3. Create a bite-sized implementation plan under `plans/` that does NOT claim implementation is complete.
4. Preserve out-of-scope guardrails: no WebRTC, no remote Unity browser control, no fake Godot/Three.js parity, no pixel-only completion evidence.
5. Verification must include docs/search checks first, then later Rust tests/skill validation once implementation starts.

Deliverable for this Codex session:
- Inspect current repo docs/plans/skills enough to ground the plan.
- Draft or update a markdown plan file only.
- Do not commit.
- Do not run huge test suites unless you changed executable code.
