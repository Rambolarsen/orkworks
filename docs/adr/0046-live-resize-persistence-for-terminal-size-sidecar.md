# Live-resize persistence for the per-session `.terminal-size` sidecar

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-09-04

## Context

[ADR 0033](./0033-recorded-terminal-replay-size-sidecar.md) introduced
`events/<id>.terminal-size` and specified that it is written exactly once per
session run, at the moment a session enters a terminal status
(`killed` / `ended` / `error`). Mid-session resize events were deliberately
not persisted: replay is a dead-session artifact and only the last grid
matters.

Implementation diverged from that decision for a reason ADR 0033's model
missed: if the daemon exits and restarts mid-session,
`metadata::reconcile_orphaned_session` adopts the orphaned session but has no
in-memory runtime handle to read a size from and never reaches the
terminal-status transition itself. For every restarted-session orphan, the
write-once model meant the size file never existed, so historical replay
always degraded to fit-to-container — which misplaces any absolute-column
cursor addressing the original PTY width baked into the recorded bytes, the
exact garbled-replay failure ADR 0033 was written to prevent.

## Decision

- `update_runtime_size` (`runtime/session_runtime.rs`) persists the PTY size
  best-effort on every *changed* live resize, in addition to the
  authoritative write at the terminal-status transition. Only the newest
  `cols × rows` is kept — the file remains a snapshot of the last known
  grid, not a resize history.
- All writes serialize through `TERMINAL_SIZE_WRITE_LOCK`
  (`session_application.rs`), and `persist_terminal_size` takes an
  `authoritative` flag: the terminal-status transition writes through the
  `ending`/`ended` phases, while live-resize writes back off during those
  phases so a straggling resize cannot overwrite the authoritative grid.
- Unchanged sizes skip the persistence write entirely (`changed` check
  before spawning the blocking write task).
- Everything else from ADR 0033 is unchanged: the file format (`120x40`
  plain text), `MetadataStore`'s read/clear semantics (missing, malformed,
  or zero-valued content reads back as `None`), clearing on resume, the
  optional `cols`/`rows` fields on `GET /sessions/:id/terminal-output`, and
  the renderer's recorded-grid/fit-to-container split between
  `HistoricalTerminal` (dead sessions) and the WS-close in-place replay path
  (`terminalStore.ts`), which sizes the live terminal to the recorded grid
  before writing the replay when both dimensions are present.

## Consequences

- Daemons restarted mid-session still leave a usable last-known size for
  orphan reconciliation, so historical replay of an adopted orphan renders
  at a real PTY grid instead of always fit-to-container.
- A mid-session restart can surface a size from *before* the restart rather
  than the grid at death — but the last-known grid is strictly better than
  no grid, and the authoritative terminal-status write still wins whenever
  the transition is reached.
- Live resizes now incur a file write per changed size. The write is
  serialized, skips unchanged sizes, and runs on a blocking task off the
  async runtime; terminal replay at session end remains the authoritative
  value.
- ADR 0033's "written a single time per session run" claim is no longer
  true; the root `AGENTS.md` metadata-protocol description of this file
  (authoritative at terminal status, best-effort on every live resize,
  still absent for sessions that ended before the file existed) is the
  accurate statement going forward.
