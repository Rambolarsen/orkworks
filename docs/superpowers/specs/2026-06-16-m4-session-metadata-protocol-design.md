# M4: Session Metadata Protocol — Design

## Overview

Implement the `.orkworks/` metadata protocol: directory structure, per-session JSON read/write, file watching, event logging, and a right sidebar that shows grouped session cards with metadata provenance.

## Architecture

### Rust Backend (orkworksd)

**New endpoints:**
- `POST /workspace` — set/switch workspace. Accepts `{ "path": "/path/to/repo" }`. Creates `.orkworks/` dirs, restarts sidecar cwd. Returns workspace path + basic git context (repo root, branch, dirty state).

**New modules:**
- `metadata.rs` — session JSON read/write, event log appending
- `watcher.rs` — `notify` crate file watcher on `.orkworks/sessions/`

**Changed modules:**
- `main.rs` — workspace state in AppState, new routes, session lifecycle hooks call metadata writer, file watcher pushes metadata changes to frontend via WebSocket

**`.orkworks/` directory creation:**
- Create `sessions/`, `events/`, `capacity/`, `skills/` subdirectories on workspace selection
- `capacity/` and `skills/` remain empty until later milestones

**Session JSON (`sessions/<id>.json`):**
```json
{
  "id": "uuid",
  "label": "Session abc12345",
  "workspace": "/path/to/repo",
  "task": "",
  "harness": "",
  "model": "",
  "cwd": "/path/to/repo",
  "status": "running",
  "phase": "implementation",
  "createdAt": "2026-06-16T08:00:00Z",
  "lastActivity": "2026-06-16T08:05:00Z",
  "metadataSource": "process",
  "metadataConfidence": 1.0
}
```

Fields `task`, `harness`, `model`, `phase` start empty — populated by later milestones (agent writes, Peon inference, harness config).

**Event log (`events/<id>.ndjson`):**
- Append JSON lines on lifecycle events: `session.created`, `session.status`, `session.killed`, `session.ended`
- Each line: `{"type":"...","timestamp":"ISO8601","status":"..."}`

**File watcher:**
- `notify` crate watches `.orkworks/sessions/` for file modifications
- On external change: reads updated JSON, pushes to frontend via a new WebSocket endpoint (`GET /sessions/:id/metadata` or extends existing session terminal WS with metadata messages)

**Metadata source priority (spec-defined):**
1. `user` — manual UI edit (future)
2. `agent` — agent-written session JSON (future)
3. `peon` — Peon inference (M6)
4. `backend_inference` — deterministic backend logic (M4: phase detection from terminal patterns)
5. `process` — process state only (M4 default)
6. `unknown` — unknown source

For M4: backend writes metadata with source `process`. No upward overrides yet.

### Electron Main

**New IPC handler:**
- `open-workspace` — shows OS native folder picker dialog, returns selected path

**Sidecar restart:**
- Kill current sidecar process
- Spawn new sidecar with `cwd` set to the selected workspace path
- Frontend reconnects to new port after health check

### Frontend (React)

**New component:** `RightSidebar.tsx` (replace stub) — grouped session cards with metadata

**Changed components:**
- `LeftSidebar.tsx` — workspace header at top: "Open Folder" button (no workspace) or workspace info + switch button (workspace active)
- `App.tsx` — new `workspace` state, workspace selection flow, pass session metadata to RightSidebar

**Workspace flow:**
1. App starts → no workspace → left sidebar shows "Open Folder"
2. Click → IPC calls Electron dialog → user picks folder
3. Path sent to `POST /workspace` → sidecar restarts in that path
4. `.orkworks/` created, health check reconnects, sessions enabled

**Right sidebar layout:**
- Scrollable list, two groups: **Working** (live sessions), **Done** (ended/killed)
- Each session card shows:
  - ⚠ indicator if status is `blocked`, `failed`, or `waiting_for_input`
  - Left border colored by status: green (running), yellow (blocked), red (failed), gray (done)
  - Label, phase, time since last activity
  - Metadata badge: source + confidence %
- Click card → activate session in center terminal
- Within each group, sorted by priority: waiting_for_input > blocked > failed > done > stale > working > idle
- Done sessions are dimmed (opacity)

## Session JSON Lifecycle

```
Session created     → write sessions/<id>.json (status: creating, source: process)
Terminal connected  → update status to "running", write event to ndjson
Status changes      → update json + append ndjson event
External JSON edit  → file watcher detects → read → push to frontend via WS
Session killed      → update status to "killed", append event
Session websocket closed → update status to "ended", append event
```

Higher-priority sources win: if an agent (priority 2) writes JSON with status "blocked", the backend (priority 4) won't overwrite it. The file watcher reads the agent-written value and surfaces it.

## Dependencies

- `notify` crate (Rust) — cross-platform file watching
- No new frontend dependencies

## Non-goals for M4

- Agent-written session JSON (that's the agent's responsibility, not OrkWorks code)
- Peon inference (M6)
- Capacity files (M8)
- Skills directory population (harness config, M7)
- User-editable session labels (separate follow-up)
- Full git context detection beyond repo root/branch/dirty (M5)
