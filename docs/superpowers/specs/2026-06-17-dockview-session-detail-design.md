# Dockview Session Awareness — Design

> **Issue:** #10 Dockview session detail and attention prioritization
> **Date:** 2026-06-17

## Goal

Replace the fixed three-column flexbox layout with dockview-powered draggable panels. Make the session tab/list itself the "what needs attention" surface by sorting and styling sessions by priority, then show explanatory details for the selected session in a dedicated detail panel docked below the session list.

Issue #10 was pivoted from a right-sidebar grouped overview into this dockview session-awareness design. The original "What do I need to look at right now?" intent is preserved by baking priority, status, and urgency into each session item and terminal tab instead of adding a separate grouped dashboard panel.

## Architecture

Replace `react-resizable-panels` with `dockview-react`. Five panels registered in a dockview grid, all draggable/repositionable by the user.

Session state stays in `App.tsx` (single owner). Panels communicate through shared `activeSessionId` prop and callbacks — no cross-panel direct coupling.

Attention state is derived from `observedStatus ?? status` using the existing priority order:

1. `waiting_for_input`
2. `blocked`
3. `failed`
4. `done`
5. `stale`
6. `working`
7. `idle`

Lifecycle `status` (`creating`, `running`, `ended`, `killed`, `error`) remains separate from observer `observedStatus`. Lifecycle status controls terminal liveness and kill/archive behavior; observer status controls attention sorting, badges, and detail text.

### Default layout

```
+------------------+---------------------------+----------------+
| Sessions (top)   | Terminal (tabbed)         | Capacity       |
|                  | [session 1] [session 2]   | (placeholder)  |
|                  |                           |                |
|                  | $ ls -la                  | Recs           |
+------------------+                           | (placeholder)  |
| Detail (bottom)  |                           |                |
| Summary, harness |                           |                |
| git, status...   |                           |                |
+------------------+---------------------------+----------------+
```

## Components

### Modified

| Component | Change |
|---|---|
| `App.tsx` | Remove flexbox layout, remove `react-resizable-panels` imports. State (sessions, activeSessionId, workspace) stays. Renders `<DockviewApp>`. |
| `LeftSidebar.tsx` | Strip outer `<aside>`, keep session list + WorkspaceHeader. Session list becomes the primary attention dashboard: priority-sorted, status-styled, and clickable. Receives `onSelectSession` callback. |
| `TerminalTabs.tsx` / `CenterPanel.tsx` | Adaptive: each terminal session tab gets its own `CenterPanel` + WebSocket. dockview handles terminal tabs natively, with attention styling on each session tab. |
| `RightSidebar.tsx` | Strip outer `<aside>`, keep detail content. Becomes the detail panel. |
| `App.css` | Replace three-column CSS with dockview dark theme. |

### New

| Component | Purpose |
|---|---|
| `DockviewApp.tsx` | Initialize dockview grid, register 5 panels, set default layout, wire panel props. |
| `CapacityPanel.tsx` | Placeholder — "Capacity tracking coming in M8" |
| `RecommendationsPanel.tsx` | Placeholder — "Recommendations coming in M9" |

### Removed

- `SessionView.tsx` — unused alternative
- `react-resizable-panels` dependency

### Unchanged

`WorkspaceHeader.tsx`, `RightSidebarHelpers.ts`, `api.ts`, terminal theme/size/launch utils.

## Panel definitions

### Session List (existing, adapted)
- Workspace header (repo path, branch, dirty indicator)
- Session items are the attention overview: status dot/border, label, CWD, harness/model badge, last activity, kill button
- Items are sorted by attention priority: waiting_for_input, blocked, failed, done, stale, working, idle
- Visual treatment is applied directly to each item:
  - `waiting_for_input`: urgent/high-emphasis
  - `blocked`: warning
  - `failed`: error
  - `done`: success/complete
  - `stale` and `idle`: muted
  - `working`: neutral active indicator
- Click selects session → updates activeSessionId, focuses terminal tab
- No separate grouped "Needs You / Blocked / Failed" dashboard panel is added in this pivot

### Session Detail (new, core deliverable)
Explains the selected session. It does not duplicate the full priority dashboard; it gives the user the context behind the selected session's tab/list state.

Shows for active session, sections vertically stacked:

| Section | Data | Source |
|---|---|---|
| Task summary | `summary` | agent/peon |
| Harness + model | `harness` + `model` | peon (already inferred) |
| Context usage | placeholder for #21 | — |
| Git context | `branch`, `dirty`, `changedFiles`, `isWorktree`, workspace path | backend |
| Lifecycle status | `status` | process/backend |
| Observed status | `observedStatus` w/ color badge | agent/peon/backend inference |
| Blocker (conditional) | `blockerDescription`, `failedCommand`, `failedTest` | agent/peon |
| Activity | `peonLastInference`, session created_at | backend |
| Source | `metadataSource` + `metadataConfidence` badge | backend |

Empty state: "Select a session to see details"

### Terminal (existing, adapted)
- Tab bar: one tab per active session (not killed/ended)
- Each tab has its own `CenterPanel` + WebSocket
- Tab label: session label + attention status dot/badge
- Tab visual treatment mirrors session list priority so attention is visible even when the left panel is narrow or moved
- Close tab = kill session
- Session list click = focus existing tab or create new one

### Capacity (placeholder, M8)
- Title: "Capacity"
- Empty state: "Capacity tracking coming in M8"

### Recommendations (placeholder, M9)
- Title: "Start Next Task"
- Empty state: "Recommendations coming in M9"

## Data flow

```
App.tsx (state owner)
  │
  ├─ sessions: SessionInfo[]          ← poll GET /sessions every 2s
  ├─ activeSessionId: string | null   ← set by any panel
  ├─ workspace: WorkspaceInfo | null
  │
  └─ DockviewApp
       │
       ├─ SessionList panel        ← reads sessions, calls onSelectSession(id)
       ├─ Detail panel             ← reads sessions, finds active, renders detail
       ├─ Terminal panel           ← reads sessions, creates/removes tabs
       │     └─ CenterPanel × N    ← each owns WebSocket to /sessions/:id/terminal
       ├─ Capacity panel           ← placeholder, no data flow
       └─ Recommendations panel    ← placeholder, no data flow
```

- Single source of truth: `App.tsx` state
- Session list → terminal: `onSelectSession(id)` sets `activeSessionId`
- Terminal panel syncs tabs to sessions list
- Session attention sorting/styling is derived from each session's `observedStatus ?? status`
- Kill: `onKillSession(id)` → `DELETE /sessions/:id` → poll picks up change
- New session: `onCreateSession()` → `POST /sessions` → poll picks up change
- 2-second polling kept; WebSocket push deferred to #22

## Error handling

- dockview init failure: fallback message, terminal functional in minimal single-panel
- No sessions: "No active sessions. Click + to start one."
- No active session in detail: "Select a session to see details"
- Terminal WebSocket disconnect: existing CenterPanel reconnect/handling stays
- Poll failure: keep last-known state, subtle error indicator in status bar
- Placeholder panels: static empty states, never error

## Testing

- Panel registration: dockview grid renders all 5 panels with correct IDs
- Detail rendering: all sections shown/hidden based on data presence
- Attention sorting: sessions order by waiting_for_input, blocked, failed, done, stale, working, idle
- Attention styling: session list items and terminal tabs render distinct urgent/warning/error/success/muted/working states
- Empty states: each panel renders correctly with no data
- Session select → terminal focus: click in session list opens terminal tab
- Terminal tabs sync: adding/removing sessions creates/removes tabs
- Lifecycle vs observed status: ended/killed sessions do not create live terminal tabs even if observedStatus is high priority
- Existing tests (API, terminal launch, kill session, workspace) kept as-is

Run via: `node --experimental-strip-types --test`

## Deferred

| Item | Issue |
|---|---|
| Context usage % | #21 |
| WebSocket push for session list | #22 |
| Capacity panel content | M8 / #8 |
| Recommendations panel content | M9 / #9 |
| Harness/model badges from config | M7 / #7 |
