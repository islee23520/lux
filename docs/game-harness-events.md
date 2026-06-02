# Game Harness Events

This file is a repository docs projection of the game-harness event surface. Runtime truth remains under `.lux/`, and `.lux/specs` remains the GDD SSoT for game intent.

Game-harness events are evidence-gated. They report plan, step, and iteration transitions, but they do not prove production-ready autonomous completion without linked command output, context snapshots, screenshots, accepted evidence, or explicit blockers.

| Event | Evidence expectation |
| --- | --- |
| `game_harness.plan.started` | Selected from `.lux/specs`, decisions, next goal, and engine capability routing. |
| `game_harness.plan.completed` | Completed only with accepted evidence or explicit blocker status. |
| `game_harness.step.started` | Names the current step and its spec, ticket, or goal reference. |
| `game_harness.step.completed` | Links to evidence such as tests, logs, context snapshots, screenshots, or blocker records. |
| `game_harness.step.failed` | Records an observable failure or capability blocker. |
| `game_harness.iteration.started` | Starts a bounded run iteration after ambiguity, decisions, and capabilities are reviewed. |
| `game_harness.iteration.completed` | Updates next goal and evidence status before another iteration can begin. |

Engine support is adapter-dependent. Unity is the primary verified path; Godot and Three.js use capability routing and must expose unsupported observations as blockers rather than equal verification maturity.
