# LUX Usage Guide

This document is a reference-oriented usage guide for LUX. It focuses on concrete interfaces and surfaces, the CLI, HTTP API, WebSocket events, specs, skills, runs, tickets, and verification. It is intentionally more detailed than the project overview.

Key references:

- Godot capability details: `docs/godot-support.md`
- Architectural decisions: `docs/adr/`

LUX is a local-first server/MCP evidence-gated automation control plane for game projects. Durable project state is owned under `.lux/`.

The repository is layered: `.lux/` owns runtime evidence and state, `gateway/` owns the Rust control plane, `bridge/` owns the engine adapter source, and `Skills/skills/` owns the bundled workflow docs. The docs below are a repository docs projection of that split without claiming autonomous verification is complete unless the supporting evidence exists.

Within `.lux/`, game design truth is split by domain. `.lux/specs/` is the GDD and game-spec single source of truth; README and docs pages are projections for people, not alternate stores for runtime decisions.

## 1. Quick Start

Minimal flow, install LUX, install the bridge, initialize `.lux/`, start the server.

```bash
# 0) Fresh clone: initialize the bridge submodule
git submodule update --init bridge

# 1) Install the LUX CLI
cargo install --path gateway

# 2) Install the bridge into your project
lux bridge install --project-path /path/to/project

# 3) Initialize .lux/ (run inside your project)
cd /path/to/project
lux init

# 4) Start the gateway server (HTTP and WebSocket)
lux serve
```

## 2. CLI Command Reference

Source of truth for the command tree is `gateway/src/main.rs`.

Conventions used below:

- Commands are shown as literal syntax.
- Options and flags are only shown where they are explicitly evidenced.
- Commands marked deprecated remain present for compatibility but prefer the listed replacement.

### Top-level command tree

```text
lux
  init
  install
  serve
  unity
  godot
  spec
  roadmap
  kanban
  triage
  verify
  run
  mcp
  skill
  ai-log
  hooks
  session
  addon
  config
  status
  schema
  completion
  self-update
  doctor
  agents-install
  autonomous

  build            (legacy)
  play             (legacy)
  compile          (legacy, deprecated)
  bridge           (legacy, deprecated)
  run-tests        (legacy, deprecated)
  screenshot       (legacy, deprecated)
```

### Project Setup

| Command | Purpose |
| --- | --- |
| `lux init` | Initialize `.lux/` state for the current project directory. |
| `lux install` | Install LUX-managed assets needed by the project workflow. |
| `lux bridge install --project-path <path>` | Install the engine bridge into a project. |

Bridge install for Godot must use `--type godot`:

```bash
lux bridge install --project-path /path/to/godot-project --type godot
```

### Server

| Command | Purpose |
| --- | --- |
| `lux serve` | Start the gateway server. |

### Unity Commands

All Unity commands are grouped under `lux unity ...`.

Note: the Unity command list in `gateway/src/main.rs` enumerates 27 subcommands in the current snapshot.

```text
lux unity
  status
  context
  backend-status
  backend-list-commands
  get-logs
  clear-console
  focus-window
  launch
  scene-smoke
  create-objects
  find-game-objects
  get-hierarchy
  control-play-mode
  screenshot
  simulate-mouse-ui
  simulate-keyboard
  simulate-mouse-input
  record-input
  replay-input
  execute-dynamic-code
  build
  play
  compile
  bridge
  run-tests
  visual-regression
  install-uloop
```

Syntax:

```bash
lux unity <subcommand>
```

Notes:

- `lux unity build`, `lux unity play`, `lux unity compile`, `lux unity bridge`, `lux unity run-tests` are Unity-scoped commands and are distinct from legacy top-level wrappers.
- `lux unity backend-list-commands` exists to surface backend command availability.

### Godot Commands

Godot is exposed as a top-level category with explicit capability status.

| Command | Status | Notes |
| --- | --- | --- |
| `lux godot status --project-path <path>` | verified | Detects Godot 4.x projects and reports capability fields. |
| `lux godot build --project-path <path>` | unsupported | Intentionally exits non-zero until GoPeak-backed verification exists. |

See `docs/godot-support.md`.

### Spec and Planning

| Command | Purpose |
| --- | --- |
| `lux spec edit` | Edit spec. |
| `lux spec validate` | Validate spec. |
| `lux roadmap status` | Show roadmap status. |
| `lux kanban` | Show kanban surfaces. |

### Skills

| Command | Purpose |
| --- | --- |
| `lux skill list` | List skills. |
| `lux skill info <name>` | Show skill details. |
| `lux skill install <name>` | Install a skill. |
| `lux skill remove <name>` | Remove a skill. |
| `lux skill update <name>` | Update a skill. |

### Sessions

| Command | Purpose |
| --- | --- |
| `lux session record` | Start recording a session. |
| `lux session stop` | Stop recording. |
| `lux session replay` | Replay a recorded session. |
| `lux session timeline` | Show a session timeline. |
| `lux session report` | Generate a session report. |

### AI Logs

| Command | Purpose |
| --- | --- |
| `lux ai-log recent` | Show recent AI log entries. |
| `lux ai-log tail` | Follow AI logs. |
| `lux ai-log context` | Show AI log context. |
| `lux ai-log compact` | Compact AI logs. |
| `lux ai-log work-step` | Work-step oriented log view. |

### Hooks

| Command | Purpose |
| --- | --- |
| `lux hooks install` | Install hooks. |
| `lux hooks status` | Show hook status. |
| `lux hooks run` | Run hooks. |

### Autonomous

| Command | Purpose |
| --- | --- |
| `lux autonomous status` | Show autonomous subsystem status. |
| `lux autonomous dry-run` | Preview an autonomous dispatch without applying. |
| `lux autonomous dispatch` | Dispatch an autonomous action. |
| `lux autonomous evidence` | View evidence produced by autonomous runs. |

Autonomous commands are evidence-gated surfaces. Their presence does not mean a spec-to-ticket-to-engine-verification loop is complete without accepted evidence in `.lux/`.

### Build and Verification

| Command | Purpose | Notes |
| --- | --- | --- |
| `lux verify` | Run verification. | Verified surface. |
| `lux build` | Build entrypoint. | legacy wrapper (deprecated for new automation, prefer Unity-scoped build flows). |
| `lux play` | Play entrypoint. | legacy wrapper (deprecated for new automation, prefer Unity-scoped play flows). |

### Run

| Command | Purpose |
| --- | --- |
| `lux run` | Run orchestration entrypoint. |

`lux run` is evidence-gated. When a run changes generated or project source, the terminal state must link accepted artifacts such as command output, tests, logs, game-context snapshots, screenshots, or manual QA records. If the required evidence cannot be produced, the run must end with an explicit blocker instead of a success projection.

For engine actions that require an operator loop, `lux run` may hand off to uloop or a manual QA session. Treat that handoff as part of the evidence path: record the command, tmux/session transcript, labels, screenshots or context captures, and final accepted/blocker status under `.lux/` or the run evidence path before claiming completion.

### MCP

| Command | Purpose |
| --- | --- |
| `lux mcp` | MCP integration entrypoint. |

### Config

| Command | Purpose |
| --- | --- |
| `lux config show` | Show current config. |
| `lux config set <key> <value>` | Set a config value. |
| `lux config get <key>` | Get a config value. |
| `lux config path` | Print config path. |
| `lux config edit` | Edit config. |

### Utility

| Command | Purpose |
| --- | --- |
| `lux status` | Show system and project status. |
| `lux schema` | Print schema examples. |
| `lux completion` | Shell completion. |
| `lux self-update` | Self update surface. |
| `lux doctor` | Diagnostics surface. |

## 3. Verification and Projection Notes

- `.lux/specs/spec.json` and `.lux/specs/domains/*.md` are the game GDD/spec SSoT. Repository docs are projections of that runtime state and can lag until refreshed.
- Verification evidence is read from `.lux/` and projected into the gateway CLI, HTTP/WebSocket API, and MCP surfaces.
- Server projections are read-only; they do not become the verification authority.
- Summary fields include ambiguity, decisions, capabilities, next goal, and evidence status. Each field is a projection from `.lux/` state or a supported gateway endpoint.
- Context-first game workflows use the Game Context Adapter contract: scene hierarchy, selected object identity, components/properties, Transform/RectTransform/Collider values, camera/UI coordinates, console/compile logs, PlayMode/input traces, screenshot refs, and optional vision annotations link back to `.lux/specs`, tickets, run evidence, and engine capability status.
- Screenshots and vision annotations are supporting evidence only; if an engine cannot provide required text/JSON observations, the adapter records an explicit capability blocker.
- Engine support uses capability routing rather than equal verification maturity: Godot remains partial and Three.js remains planned unless a supported verification surface proves otherwise.
- Remote/WebRTC stays hidden experimental and out of the public verification path.

### Legacy top-level commands (deprecated)

These commands are present for compatibility and should be considered deprecated.

| Deprecated command | Prefer |
| --- | --- |
| `lux compile` | `lux unity compile` |
| `lux run-tests` | `lux unity run-tests` |
| `lux screenshot` | `lux unity screenshot` |
| `lux bridge` | `lux unity bridge` or `lux bridge install` depending on intent |

## 3. API Endpoint Reference

The gateway exposes 120+ HTTP endpoints plus WebSocket event streams.

Endpoint list below is grouped by function. Method(s) are shown when explicitly evidenced.

### Health and Meta

- `GET /health`
- `GET /api/health`
- `POST /api/heartbeat`
- `GET /schema`

WebSocket roots:

- `GET /events` (WebSocket)
- `GET /api/events` (WebSocket)

### Project and Bridge

- `POST /api/project/detect`
- `POST /api/detect_project`
- `POST /api/bridge/install`
- `POST /api/compile`

### Unity Execution and Capture

Unity runs:

- `GET /api/unity/runs`
- `POST /api/unity/runs`
- `DELETE /api/unity/runs`

Unity capture sessions:

- `POST /api/unity/capture/sessions`
- `GET /api/unity/capture/sessions`
- `DELETE /api/unity/capture/sessions`

Unity launch and status:

- `POST /api/unity/launch`
- `GET /api/unity/status`
- `GET /api/unity/version`

### Sessions and Remote

Local sessions:

- CRUD `/api/sessions`

Remote sessions:

- CRUD `/api/remote/sessions`

Remote signaling:

- `GET /remote/signaling/:session_id`

### Tools

- `GET /api/tools`

Tool sessions:

- CRUD `/api/tools/sessions`

Tool execution:

- `POST /api/tools/execute`
- `GET /api/tools/executions/:execution_id`

### Pipeline and Graphs

Pipeline:

- `GET /api/pipeline`
- `POST /api/pipeline`
- `GET /api/pipeline/:run_id`

Graphs and execution:

- CRUD `/api/graphs`
- `POST /api/graphs/:graph_id/execute`

Node types:

- `GET /api/node-types`

### Skills

- `GET /api/skills`
- `GET /api/skills/:name/adaptation`

### Lux Management

Core management:

- `POST /api/lux/init`
- `GET /api/lux/experimental-flags`

Spec:

- `GET /api/lux/spec`
- `PUT /api/lux/spec`
- `GET /api/lux/spec/ambiguity`
- `POST /api/lux/spec/validate`
- `GET /api/lux/spec/:domain`
- `PUT /api/lux/spec/:domain`

Progress:

- `GET /api/lux/progress/summary`

Continuation:

- `GET /api/lux/continuation/state`
- `PUT /api/lux/continuation/state`

Kanban board:

- `GET /api/lux/kanban/board`

### Lux Runs

Run lifecycle:

- `GET /api/lux/runs/state`
- `GET /api/lux/runs/lux-state`
- `POST /api/lux/runs/start`
- `POST /api/lux/runs/transition`
- `POST /api/lux/runs/stop`

Run tickets (CRUD):

- `GET /api/lux/runs/tickets`
- `POST /api/lux/runs/tickets`
- `PUT /api/lux/runs/tickets`
- `DELETE /api/lux/runs/tickets`

Bridge lease:

- `GET /api/lux/runs/bridge-lease`
- `POST /api/lux/runs/bridge-lease`
- `DELETE /api/lux/runs/bridge-lease`

Proposals and evidence:

- `POST /api/lux/runs/proposals`
- `POST /api/lux/runs/evidence`
- `POST /api/lux/runs/evidence/accept`

Blocker and milestone push:

- `POST /api/lux/runs/blocker-resolution-requests`
- `POST /api/lux/runs/milestone-push-requests`

### Lux Build

- `POST /api/lux/build/start`
- `GET /api/lux/build/status/:build_id`
- `GET /api/lux/build/log/:build_id`
- `POST /api/lux/build/cancel/:build_id`
- `GET /api/lux/build/list`

### Lux Play

- `POST /api/lux/play/event`
- `POST /api/lux/play/events/batch`
- `POST /api/lux/play/session/start`
- `POST /api/lux/play/session/end`
- `GET /api/lux/play/sessions`
- `GET /api/lux/play/sessions/:id/events`
- `POST /api/lux/play/feedback`

### Lux Verify

- `POST /api/lux/verify/run`
- `GET /api/lux/verify/latest`

### Lux Loop

- `POST /api/lux/loop/start`
- `GET /api/lux/loop/status`
- `POST /api/lux/loop/pause`
- `POST /api/lux/loop/resume`
- `POST /api/lux/loop/approve`
- `POST /api/lux/loop/play-started`
- `POST /api/lux/loop/feedback`

### Lux Spec-Loop

- `POST /api/lux/spec-loop/start`
- `GET /api/lux/spec-loop/status`
- `POST /api/lux/spec-loop/answer`
- `POST /api/lux/spec-loop/approve`
- `POST /api/lux/spec-loop/reject`
- `POST /api/lux/spec-loop/apply`

### Lux Terminal

- `POST /api/lux/terminal/create`
- `POST /api/lux/terminal/:id/input`
- `GET /api/lux/terminal/:id/output`
- `DELETE /api/lux/terminal/:id`
- `GET /api/lux/terminal/list`

### Lux Kanban

Board:

- `GET /api/lux/kanban/board`

Tickets CRUD and status transition:

- `GET /api/lux/kanban/tickets`
- `POST /api/lux/kanban/tickets`
- `PUT /api/lux/kanban/tickets`
- `PUT /api/lux/kanban/tickets/:id/status`

### Addons

- `GET /api/addons/`
- `POST /api/addons/register`
- `DELETE /api/addons/:id`
- `POST /api/addons/auth/device`
- `POST /api/addons/auth/token`
- `GET /api/addons/auth/status`
- `POST /api/addons/auth/renew`
- `POST /api/addons/:id/verify`
- `GET /api/addons/:id/visibility`
- `POST /api/addons/discover`

### Experimental Flags

- `GET /api/lux/experimental-flags`

## 4. WebSocket Events

Event streams are exposed at:

- `GET /events`
- `GET /api/events`

All events use the same envelope:

```json
{
  "schema_version": 1,
  "event_id": "uuid",
  "category": "tool|playmode|scene|log|input|screenshot|hierarchy",
  "source": "unity-editor|cli",
  "session_id": "session-uuid",
  "captured_at_utc": "ISO 8601",
  "payload": {}
}
```

Categories:

- `tool`
- `playmode`
- `scene`
- `log`
- `input`
- `screenshot`
- `hierarchy`

### Game Harness Events

Game-harness orchestration events are emitted as evidence-gated runtime events. They are projections of `.lux/` state and must not be used to claim autonomous completion unless the referenced plan, step, iteration, and engine evidence are accepted.

| Event | Meaning |
| --- | --- |
| `game_harness.plan.started` | A spec-backed game-harness plan was selected from `.lux/specs` and current capability routing. |
| `game_harness.plan.completed` | The plan reached its evidence-gated terminal state with accepted evidence or explicit blockers. |
| `game_harness.step.started` | A single plan step began with a declared `.lux/specs` or ticket reference. |
| `game_harness.step.completed` | A step completed with linked command, test, context, screenshot, or blocker evidence. |
| `game_harness.step.failed` | A step failed and wrote explicit error or blocker evidence instead of silently falling back. |
| `game_harness.iteration.started` | A run iteration began, usually after ambiguity, decision, or engine capability review. |
| `game_harness.iteration.completed` | A run iteration ended after updating evidence status, next goal, or blocker state. |

Engine support for these events is adapter-dependent. Unity has the most mature verified surface; Godot and Three.js remain capability-routed and must record blockers for unsupported observations or commands.

Subscription model:

- Connect to the WebSocket endpoint.
- Read JSON event objects. Each message is one event envelope.

## 5. Spec System

LUX uses a game-domain spec model rooted under `.lux/specs/`. The spec is managed via CLI (`lux spec ...`) and API (`/api/lux/spec...`).

Treat `.lux/specs/` as the GDD SSoT. Repository docs projection text may summarize the domains, but changes to actual game intent must flow through the spec CLI/API and its `.lux/` write paths.

Domains (canonical game set):

| Domain | Identifier |
| --- | --- |
| GDD | `gdd` |
| Mechanics | `mechanics` |
| Controls | `controls` |
| Camera | `camera` |
| Art style | `art-style` |
| Audio | `audio` |
| Narrative | `narrative` |
| UI and UX | `ui-ux` |
| Technical architecture | `technical-architecture` |
| Engine | `engine` |
| Testing | `testing` |
| Build and release | `build-release` |

Ambiguity scoring and visibility:

- API: `GET /api/lux/spec/ambiguity`

Spec loop (Socratic refinement) surfaces:

- `POST /api/lux/spec-loop/start`
- `GET /api/lux/spec-loop/status`
- `POST /api/lux/spec-loop/answer`
- `POST /api/lux/spec-loop/approve`
- `POST /api/lux/spec-loop/reject`
- `POST /api/lux/spec-loop/apply`

## 7. Skill System

LUX ships curated skills from the tracked source tree at `Skills/skills`. The current tree contains 46 total `SKILL.md` files, including 20 manifest-backed built-in skills. Skills are exposed through the CLI and API, while runtime installation projects them into tool-specific skill directories.

Legacy adapter roots are not active source in the server/MCP-only repository shape. Runtime integration should use the CLI, HTTP/WS APIs, MCP server, and installed workflow skills. Remote/WebRTC remains hidden experimental and requires `.lux/roadmap.json` `experimental_flags.remote_webrtc=true`.

Built-in skill names:

- `architecture-decision`
- `architecture-review`
- `bug-report`
- `bug-triage`
- `changelog`
- `code-review`
- `core-invariants`
- `game-dev`
- `ldp-decision-protocol`
- `lux-unity`
- `perf-profile`
- `regression-suite`
- `release-checklist`
- `retrospective`
- `security-audit`
- `smoke-check`
- `tech-debt`
- `test-helpers`
- `test-setup`
- `unity-cs-reference`

Skill adaptation:

- API: `GET /api/skills/:name/adaptation`

## 8. Run and Ticket System

### Ownership and SSoT

Architectural decisions in `docs/adr/` define control-plane ownership:

- ADR-001: the Rust Gateway is the sole owner of execution lifecycle.
- ADR-002: `.lux/run-state.json` is the canonical durable state for the active run.
- ADR-003: `.lux/specs/spec.json`, `.lux/specs/domains/*.md`, `.lux/tickets/*.json`, `.lux/run-state.json`, and `.lux/roadmap.json` are separated by domain.

### Run states

Run states (from `lux_run_state.rs`):

```text
idle
planned
dispatch_ready
executing
verifying
blocked
retry_ready
resumed
Completed
Failed
Quarantined
```

Run lifecycle APIs:

- `GET /api/lux/runs/state`
- `POST /api/lux/runs/start`
- `POST /api/lux/runs/transition`
- `POST /api/lux/runs/stop`

### Ticket CRUD

Tickets are managed through:

- Kanban tickets: `/api/lux/kanban/tickets` and `/api/lux/kanban/tickets/:id/status`
- Run tickets: `/api/lux/runs/tickets`

## 9. Verification and Evidence

Verification runs:

- `POST /api/lux/verify/run`
- `GET /api/lux/verify/latest`

Evidence surfaces in the run lifecycle:

- `POST /api/lux/runs/evidence`
- `POST /api/lux/runs/evidence/accept`

Milestone push requests:

- `POST /api/lux/runs/milestone-push-requests`

Blocker resolution requests:

- `POST /api/lux/runs/blocker-resolution-requests`

## 10. Configuration

Configuration is managed through the `lux config` command group.

```bash
lux config show
lux config get <key>
lux config set <key> <value>
lux config path
lux config edit
```

Experimental flags are visible via:

- `GET /api/lux/experimental-flags`

## 11. Engine Support

Support status is intentionally explicit so clients do not assume planned features work.

Capability routing is per engine and per command. A verified Unity command, a partial Godot command, and a planned Three.js command do not have equal verification maturity.

| Engine | Support status | Notes |
| --- | --- | --- |
| Unity | verified | Primary public-beta verified engine. |
| Godot | partial | Detection and bridge install pass; build is unsupported. See `docs/godot-support.md`. |
| Three.js | planned | Runtime harness is absent in this repo snapshot. |

## 12. Troubleshooting

### Godot bridge install fails

Requirements from `docs/godot-support.md`:

- Project must be Godot 4.x.
- `project.godot` must contain `config_version=5`.
- There is no force bypass for Godot bridge validation.

Use:

```bash
lux bridge install --project-path /path/to/godot-project --type godot
```

### `lux godot build` exits non-zero

This is expected. The command is intentionally unsupported until GoPeak-backed build has automated verification.

```bash
lux godot build --project-path /path/to/godot-project
```

### Conflicting assumptions about run state

If you are integrating tooling, treat the Rust Gateway as the execution owner, and treat `.lux/run-state.json` as the active run single source of truth. See ADR-001 and ADR-002 in `docs/adr/`.
