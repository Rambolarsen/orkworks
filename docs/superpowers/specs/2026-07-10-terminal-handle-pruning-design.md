# Terminal Handle Pruning Design

## Goal

Stop the desktop renderer from retaining stale xterm/WebSocket terminal handles after workspace switches or session-list churn.

## Scope

This change is limited to:

- `apps/desktop/src/terminalStore.ts`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/components/TerminalPanel.tsx`
- `docs/superpowers/specs/2026-07-10-terminal-handle-pruning-design.md`
- `docs/superpowers/plans/2026-07-10-terminal-handle-pruning.md`
- source-based regression tests in `apps/desktop/tests/`

## Problem

The renderer caches terminal handles in a module-level `Map<string, TerminalHandle>`.
That cache is intentional for active workspace session switching, because inactive
live terminals should stay warm while the user moves between sessions. The leak is
that the cache currently outlives the session set that justified those handles:

- switching workspaces resets session state but does not clear cached terminals
- a polled session list can drop ids, but cached handles for those ids are not pruned

Each stale handle retains an xterm instance, a wrapper element, a `ResizeObserver`,
and a `WebSocket`, so the leak compounds over time.

## Design

- Keep the existing behavior where switching between still-live sessions in the same
  workspace does not dispose inactive terminals.
- Gate terminal creation to live sessions only. Non-live selected sessions must not
  call `ensureTerminal`; they should render the existing terminal empty state instead
  of creating a new renderer terminal.
- Add a terminal-store helper with an explicit contract:
  `pruneTerminals(keepLiveSessionIds: ReadonlySet<string>)`.
  The helper only disposes cached ids that are absent from the keep set.
- Treat `memoryState === "live"` as the criterion for retaining a cached terminal.
  `App.tsx` is responsible for building `keepLiveSessionIds` from the current session
  list; remembered/resumable/unsupported sessions must not be included.
- Run pruning after session refreshes so handles for ids that disappeared or are no
  longer live are disposed promptly.
- Clear all cached terminals on workspace changes before resetting the session list,
  because terminal ids from the previous workspace must never survive into the next one.
- Guard session refresh application so stale results from the previous workspace cannot
  repopulate state or drive pruning after a workspace switch.
- Keep disposal idempotent. Repeated pruning or workspace resets must be safe.

## Non-Goals

- No change to terminal protocol, PTY lifetime, or backend session lifetime
- No change to the UX rule that inactive live terminals stay attached while switching sessions
- No attempt to reclaim live handles eagerly just because they are not the active session

## Verification

- Add a regression assertion that `App.tsx` clears all terminals during workspace switching.
- Add a regression assertion that `App.tsx` prunes terminals against the current live session ids after session refresh.
- Add a regression assertion that `App.tsx` guards `refreshSessions` with a workspace-switch
  generation/token before calling `pruneTerminals(...)` or `setSessions(...)`, and
  increments that token on workspace changes.
- Add a regression assertion that `TerminalPanel.tsx` only passes a session id to
  `CenterPanel` when `session.memoryState === "live"`.
- Add a source-based structural regression test for the terminal-store pruning helper, including:
  - disposing ids missing from the keep set
  - leaving retained live ids untouched
- Add a source-based regression test that `App.tsx` builds the keep set from
  `sessions.filter(s => s.memoryState === "live")`.
- Add a regression assertion that repeated `pruneTerminals(...)` and
  `disposeAllTerminals()` calls are safe and do not attempt to retain removed handles.
- Run the focused test files that cover the app shell, dockview/source structure, and terminal store behavior.
