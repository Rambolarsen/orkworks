# Recorded terminal-replay grid via per-session `.terminal-size` sidecar

- Status: superseded by [0046](./0046-live-resize-persistence-for-terminal-size-sidecar.md)
- Deciders: Rambolarsen
- Date: 2026-08-01

## Context

Dead-session terminal replay used to re-wrap saved output to whatever width
the detail-panel happened to be at the moment of viewing, mid-word-splitting
long lines whenever that differed from the grid the PTY was actually recorded
at. [ADR 0024](./0024-bounded-terminal-replay-durable-summary-checkpoints.md)
bounded the raw replay bytes and added durable summary checkpoints, but did
not carry the recording grid, so a wide-terminal session viewed in a narrow
panel still garbled its wrap points.

Replay is a dead-session artifact, not live output, so the only grid that
matters is the one the PTY had when it last emitted. Mid-session resize
history is not needed for replay.

## Decision

> **Superseded 2026-09-04:** [ADR 0046](./0046-live-resize-persistence-for-terminal-size-sidecar.md)
> replaces the write-once model below — the size file is now persisted
> best-effort on every changed live resize in addition to the authoritative
> terminal-status write. The remaining bullets describe the original
> decision; the store format, HTTP fields, renderer behavior, and resume
> clearing described here are unchanged.

- Persist the PTY's last known `cols × rows` once, in
  `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size`, at the moment a
  session enters a terminal status (`killed` / `ended` / `error`). The file
  is plain text (`120x40`) and is written a single time per session run —
  live resize events are not persisted.
- `MetadataStore` exposes `write_terminal_size`, `read_terminal_size`, and
  `clear_terminal_size` next to the existing `.terminal` readers. Missing,
  malformed, or zero-valued content reads back as `None`, so untreated legacy
  sessions degrade to the same posture as absent data.
- `GET /sessions/:id/terminal-output` includes the recorded size, when
  present, as optional `cols` / `rows` fields in its JSON response. Both use
  `skip_serializing_if = "Option::is_none"`, so existing clients that ignore
  unknown fields keep working without changes.
- The recorded size is stored outside `SessionMetadata`. That struct is an
  exhaustive literal at ~30 call sites across `main.rs`,
  `session_view.rs`, `runtime/terminal_runtime.rs`, `runtime/peon_runtime.rs`,
  `http/session_handlers.rs`, and friends; the sidecar-file pattern already
  used by `.terminal` avoids editing all of them for two new fields.
- `resume_session` clears the sidecar before launching the resumed runtime:
  if the daemon exits before the resumed run reaches another terminal
  transition, startup reconciliation has no in-memory handle to overwrite the
  size, so the prior run's grid would otherwise be served for the new run's
  output and replay would wrap against a stale grid. Clearing makes the
  documented fit-to-container fallback apply instead.
- The renderer (`HistoricalTerminal.tsx`) constructs the xterm `Terminal`
  with the recorded `cols` / `rows` when present (no `FitAddon`-driven
  reflow), then on first xterm `onRender` caches its `.xterm-screen`
  `getBoundingClientRect()` as the recorded grid's natural pixel size, stamps
  that size onto the root `.xterm` element, and visually shrinks it with a CSS
  `transform: scale(...)` computed against the container's content box (not
  padding-inclusive `clientWidth` / `clientHeight`). `ResizeObserver`
  recomputes only the scale. If the first measurement is zero (panel hidden
  or not yet measured), the measurement is retried on later renders and
  resizes until both dimensions are positive — the cached size and the
  render-listener disposal both wait for a real measurement.
- Legacy and unknown-size sessions keep today's `FitAddon` fit-to-container
  behavior unchanged.

## Consequences

- Dead-session replay renders at the recorded grid; words no longer split
  mid-character when the panel is narrower than the recording.
- Legacy sessions (pre-sidecar, or sessions whose in-memory handle was
  already gone at the terminal-status transition, e.g. daemon crash before
  the resumed runtime reached another terminal transition) keep
  fit-to-container replay — the existing behavior, never a misrendered grid.
- One new durable artifact per session; it lives next to `.terminal` and is
  removed by the existing `delete_events` retention path. No new retention
  surface.
- HTTP `terminal-output` response gains two optional fields; older clients
  that tolerate unknown fields are unaffected.
- Mid-session size changes continue not to be persisted; only the size at
  the terminal-status transition matters for replay.
- The renderer depends on xterm's `.xterm-screen` inline pixel size being a
  true measurement of the recorded grid's natural size, independent of the
  container. If a future xterm version changes that internal sizing contract,
  the cache-and-scale approach needs re-validation.