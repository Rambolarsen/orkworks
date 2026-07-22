# OrkWorks — Updated MVP Direction


Mission Control for AI Agents

## Naming and Terminology

Product name: **OrkWorks**.

Protocol directory: **`.orkworks/`** (under `~/.orkworks/`, see [ADR 0018](../docs/adr/0018-global-metadata-store.md)).

Low-cost metadata worker: **Peon**.

Use normal engineering terminology everywhere else: sessions, events, capacity, recommendations, workspaces, worktrees, harnesses, and models. Avoid expanding the fantasy theme into additional product terms. Peon is the single memorable product-specific worker name.


## Product Boundary

OrkWorks is a local-first observability and recommendation layer for AI coding sessions.

The app should help the user understand:

- what AI terminal sessions are currently running
- what each session is doing
- which sessions need user attention
- which sessions are blocked, failed, done, stale, or working
- which harnesses/models are capped, degraded, healthy, cheap, or expensive
- which harness/model is recommended for the next task
- what repo, branch, directory, or worktree each session is running in

OrkWorks should not own the user’s development workflow by default.

It should observe and recommend before it controls. It does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## Revised Product Principle

A bad version says:

> Here are twelve terminals.

A good version says:

> Two sessions need your input.  
> One session is done and ready for review.  
> Three are working.  
> One agent is running in a dirty shared workspace.  
> Claude is capped.  
> For your next implementation task, use OpenCode + DeepSeek.

The app should reduce cognitive load without taking over the repo workflow.

## Control Boundary

OrkWorks should own:

- terminal visibility
- session overview
- session metadata
- session status detection
- terminal activity monitoring
- capacity/cap tracking
- harness/model recommendations
- optional repo-local metadata files
- optional Peon metadata normalization

OrkWorks should not own by default:

- git workflow
- branch strategy
- worktree creation
- merging
- rebasing
- resetting
- stashing
- cleanup of branches/worktrees
- task decomposition
- automatic terminal input
- automatic command approval

Workflow actions may be added later as explicit opt-in conveniences, but they are not part of the initial MVP.

## Git and Worktree Context

Git worktree support should be context detection first, not workflow control.

OrkWorks should detect and display Git context for each terminal session:

- current working directory
- Git repository root
- current branch
- whether the directory is a Git worktree
- whether the working tree is dirty
- changed file count
- changed files where practical
- relationship between multiple sessions in the same repo
- whether multiple sessions are sharing the same working directory

OrkWorks may use this context for recommendations.

Example recommendations:

```text
This session is running in a separate worktree. Good isolation for parallel agent work.
```

```text
Three active sessions are running in the same dirty workspace. Consider using separate worktrees for new coding tasks.
```

```text
This looks like a review/debugging task. Running in the main workspace is probably fine.
```

```text
This looks like a risky implementation task. A separate worktree may be safer.
```

The user, repo skill, harness, or existing workflow decides whether to create a worktree.

OrkWorks should not create, delete, merge, rebase, reset, or clean up worktrees in the MVP.

## Repo Skills Boundary

Repo skills and OrkWorks may overlap, but they should have different responsibilities.

OrkWorks is the runtime observability layer.

Repo skills describe how agents should behave inside a repository.

The split should be:

```text
OrkWorks = observe sessions, detect state, show overview, recommend next action
Repo skills = tell agents how to work in this repo and how to report status
Peon = fallback metadata normalizer when agents do not report well
```

Repo skills may instruct agents to:

- update `.orkworks/sessions/<session-id>.json`
- append events to `.orkworks/events/<session-id>.ndjson`
- summarize current work
- report blockers
- report test status
- report files touched
- avoid merging unless explicitly asked
- follow the repo’s preferred branch/worktree workflow

Repo skills should not redefine the OrkWorks protocol.

OrkWorks should treat repo skills as instructions for agents, not as the source of application lifecycle logic.

## Metadata Source Priority

When multiple systems provide session metadata, OrkWorks should use explicit priority.

Priority order:

1. User/manual override
2. Explicit agent-written session JSON
3. Peon inference
4. Backend deterministic inference
5. Process state only

Session metadata should include source and confidence where useful:

```json
{
  "status": "waiting_for_input",
  "summary": "Needs decision about API compatibility.",
  "metadataSource": "agent",
  "metadataConfidence": "high"
}
```

Valid metadata sources:

- `user`
- `agent`
- `peon`
- `backend_inference`
- `process`
- `unknown`
- `debug` (debug-only temporary state injection; lower priority than normal runtime sources)

Peon must not overwrite higher-priority metadata unless the higher-priority metadata is stale or explicitly cleared.

### Deterministic harness-supplied signals

Alongside Peon's LLM-based inference, some harnesses expose deterministic, higher-confidence signals that OrkWorks can consume directly instead of inferring them from terminal output:

- **Attention state** (`waiting_for_input` and related statuses): a harness's own notification mechanism — e.g. Claude Code's `Notification` hook — can call `POST /sessions/:id/attention` on the sidecar. Writes use `metadataSource: "agent"` with `metadataConfidence: 1.0` and respect the same priority/staleness rule Peon already respects: they cannot overwrite fresh `user` or fresh `agent` metadata, but always outrank `peon`/`backend_inference`/`process`/`unknown`.
- **Plan handoff**: an attention report may include an optional repo-relative Markdown `planPath` identifying the plan that led to the current `needs_you` state. A JSON `null` clears a previously reported path; omission preserves it. The sidecar exposes only a boolean availability flag to the renderer, then validates the stored path before returning it to Electron for an explicit OS-level open action. The renderer never receives a filesystem path. The authenticated plan-open endpoint is available only to the Electron main process via a per-sidecar secret, and Electron revalidates the returned path immediately before opening it. See [ADR 0025](../docs/adr/0025-authenticated-session-plan-handoff.md).
- **Harness-native session ID**: a harness-specific mechanism (env var, hook JSON, structured JSONL event) reports the session's native ID via `POST /sessions/:id/harness-session`, tagged with a source string and confidence. This is the same generic capture endpoint used for OpenCode's `OPENCODE_SESSION_ID`, Claude Code's hook `session_id`, and Codex's `thread.started` JSONL event; see `skills/adding-harness/`.

These are opt-in per harness and never installed automatically. For Claude Code, `POST /workspace/attention-hook/install` (paired with `GET /workspace/attention-hook/status`) merges a single hook entry into the workspace's `.claude/settings.local.json` only after explicit, user-confirmed action in Settings — never `settings.json`, and never at session spawn time. See [ADR 0019](../docs/adr/0019-attention-signal-endpoint-opt-in-hook-install.md).

When no harness-specific signal source is registered or installed for a session, Peon's LLM-based inference remains the sole/fallback source, unchanged.

## Peon

The MVP should include Peon: a low-cost AI observer responsible for maintaining session metadata and improving observability.

Peon should help normalize messy terminal output into useful OrkWorks state.

Peon may infer:

- status
- phase
- summary
- next action
- whether user input is needed
- detected question
- suggested options
- blocker description
- failed command or failed test summary
- capacity/cap hints
- confidence

Peon may update:

- `.orkworks/sessions/<session-id>.json`
- `.orkworks/events/<session-id>.ndjson`
- `.orkworks/capacity/<capacity-id>.json`

Peon must not:

- type into terminals automatically
- approve commands
- decide merges
- delete files
- modify source code
- override user decisions
- treat inference as more authoritative than explicit agent/user metadata

The first MVP autonomy level for Peon is observer-only.

Later versions may add suggested terminal input, but it must be gated by explicit user approval.

## Updated MVP Scope

The first useful MVP should include:

### Must Have

#### Electron Desktop Shell

- Electron app shell
- React + TypeScript UI
- VS Code-like three-column layout
- left sidebar with workspaces/sessions
- center embedded terminal
- right sidebar with action overview, capacity, and recommendation panels
- Electron launches Rust backend sidecar
- frontend communicates with backend over localhost HTTP/WebSocket
- secure preload bridge
- `nodeIntegration: false`
- `contextIsolation: true`

#### App Settings and Hotkeys

- persist app-level settings in Electron user data
- use a versioned app settings object that can grow over time
- implement `hotkeys` as the first settings section
- support the currently implemented shortcuts only:
  - new session
  - sessions panel shortcut
  - detail panel shortcut
  - terminal panel shortcut
  - capacity panel shortcut
  - recommendations panel shortcut
  - reset layout shortcut
- default hotkeys must match the shipped accelerators
- build Electron menu accelerators from saved settings rather than hard-coded constants
- expose an in-app settings entry point in the desktop UI
- provide a settings modal with a Hotkeys section
- support edit, per-hotkey reset, restore defaults, cancel, and save
- validate invalid, duplicate, and required hotkey values before persisting changes
- prevent existing accelerators from firing while a replacement hotkey chord is being captured
- preserve the current Sessions panel shortcut behavior when that shortcut is customized
- saved hotkeys must survive app restart

#### Rust Backend

- Rust sidecar process
- Axum HTTP/WebSocket API
- dynamic localhost port
- health endpoint
- session registry
- PTY process manager using `portable-pty`
- terminal output streaming
- terminal input forwarding
- terminal resize support
- kill/archive session support

#### Terminal Sessions

- start terminal sessions through backend
- support multiple running sessions
- switch between sessions without killing processes
- preserve recent scrollback per session
- show active session metadata:
  - task
  - harness
  - model
  - workspace
  - working directory
  - status
  - phase
  - last activity

#### Session Metadata Protocol

- create `.orkworks/` structure under `~/.orkworks/workspaces/<path-hash>/` when enabled:
  - `sessions/`
  - `events/`
  - `capacity/`
  - `skills/`
- read/write `sessions/<session-id>.json`
- watch session JSON files for changes
- trust explicit agent-written session JSON
- infer state when JSON is missing or stale
- append basic event logs to `events/<session-id>.ndjson`

#### Git Context Detection

- detect whether session directory is inside a Git repo
- detect repo root
- detect branch name
- detect dirty/clean state
- detect changed file count
- detect whether directory is a worktree where practical
- show Git context in the UI
- include Git context in session metadata
- warn when multiple active sessions share the same dirty working directory
- recommend worktree isolation for suitable parallel coding tasks

#### Peon

- collect recent terminal output per session
- call cheap model with compact context
- require strict JSON response
- validate model response against schema
- update session JSON with inferred metadata
- append Peon notes to event log
- show confidence/source in UI
- never send terminal input automatically

#### Right Sidebar

The right sidebar should answer:

> What do I need to look at right now?

Groups:

- Needs You
- Blocked
- Failed
- Done
- Stale
- Working
- Idle
- Capacity
- Start Next Task

Sessions should be prioritized:

1. `waiting_for_input`
2. `blocked`
3. `failed`
4. `done`
5. `stale`
6. `working`
7. `idle`

#### Harness Configuration

- manual harness configuration
- generic terminal adapter
- ability to start configured commands such as:
  - OpenCode
  - Codex
  - Claude Code
  - Gemini CLI
  - Aider
- harness/model labels in UI
- initial prompt/instruction injection or display
- no smart harness-specific adapters required for v0.1

#### Capacity Tracking

- manual capacity state
- capacity JSON files
- status values:
  - `healthy`
  - `degraded`
  - `capped`
  - `unknown`
  - `disabled`
- cost tiers:
  - `local`
  - `low`
  - `medium`
  - `high`
  - `premium`
- output pattern detection for:
  - usage limit reached
  - rate limit
  - quota exceeded
- Peon-assisted capacity classification
- confidence/source fields for capacity status

#### Recommendation Engine

- rule-based recommendation engine
- uses:
  - task description
  - selected workspace
  - harness configuration
  - capacity status
  - cost tier
  - current active sessions
  - session summaries
  - Git context
- recommends a harness/model for the next task
- explains recommendation in plain language
- may recommend workflow context, such as:
  - use cheap model
  - save premium model for review
  - wait until capped model resets
  - consider a worktree for parallel implementation
  - avoid starting another agent in a dirty shared workspace

### Should Have

- workspaces
- Git worktree detection
- list known worktrees for a repo
- configurable output patterns
- configurable waiting/input patterns
- basic event timeline
- session archive
- Peon provider configuration
- local model option where available
- manual override for Peon-derived status
- OpenCode-specific adapter
- Codex-specific adapter
- Claude Code-specific adapter

### Could Have Later

- create worktree from OrkWorks
- archive worktree from OrkWorks
- cleanup worktree from OrkWorks
- branch management
- auto-suggest terminal input gated by user approval
- split terminal view
- notifications
- cost history
- token usage parsing
- provider API integration
- PR/diff integration
- multi-machine daemon
- VS Code extension client
- mobile dashboard
- shared team dashboard
- cloud sync

## Suggested Monorepo Structure

```text
orkworks/
├─ apps/
│  └─ desktop/
├─ crates/
│  └─ orkworksd/
├─ docs/
└─ examples/
```

The backend sidecar should be named `orkworksd`.

## Updated Milestones

### Milestone 1 — Shell and Backend

Goal: Electron can launch the Rust backend.

Deliverables:

- Electron app window
- Rust backend process
- dynamic localhost port
- health endpoint
- frontend can call backend
- logs visible in dev mode

### Milestone 2 — Embedded Terminal

Goal: Run one terminal session inside the app.

Deliverables:

- create PTY
- stream output to xterm.js
- send keyboard input to PTY
- resize terminal
- kill process

### Milestone 3 — Multiple Sessions

Goal: Run and switch between multiple terminals.

Deliverables:

- session registry
- session list
- switch active terminal
- preserve scrollback
- kill/archive sessions

### Milestone 4 — Session Metadata Protocol

Goal: Show meaningful session metadata.

Deliverables:

- create `sessions/<id>.json` under the global metadata root
- watch file changes
- reflect status in UI
- append basic event log
- show Needs You / Working / Done groups

### Milestone 5 — Git Context Detection

Goal: Show where each session is running and whether it is isolated.

Deliverables:

- detect Git repo root
- detect branch
- detect dirty/clean state
- detect changed file count
- detect worktree context where practical
- show Git context in session UI
- warn about multiple active sessions in the same dirty workspace
- use Git context in recommendations

### Milestone 6 — Peon

Goal: Use Peon to keep session metadata useful with low-cost observer-only inference.

Deliverables:

- collect recent output per session
- call cheap model with compact context
- require strict JSON response
- validate response against schema
- update session JSON with inferred metadata
- append Peon notes to event log
- show confidence/source in UI
- no automatic terminal input

### Milestone 7 — Harness Configuration

Goal: Start AI harnesses as configured terminal commands.

Deliverables:

- configurable harness commands
- start session dialog
- harness/model labels
- generic terminal adapter
- initial prompt/instruction support

### Milestone 8 — Capacity Panel

Goal: Track basic harness/model availability.

Deliverables:

- capacity files
- manual capacity override
- capped/healthy/unknown badges
- output pattern detection for caps
- Peon-assisted capacity classification
- confidence/source display

### Milestone 9 — Recommendation Engine

Goal: Recommend which harness/model to use for the next task.

Deliverables:

- simple task classifier
- rule-based scoring
- capacity-aware recommendation
- cost-aware recommendation
- Git-context-aware recommendation
- recommendation UI
- start recommended session

## Explicit MVP Non-Goals

The MVP is not:

- a new AI coding harness
- an IDE/editor replacement
- a full multi-agent planner
- a repo workflow manager
- a Git worktree manager
- an automatic merge system
- a cloud sync service
- a universal billing tracker
- a replacement for OpenCode, Claude Code, Codex CLI, Gemini CLI, or Aider
