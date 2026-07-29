# Session Output Recency Design

Date: 2026-07-29
Status: approved

## Goal

Make a live session's “last active” time reflect the latest real PTY output,
even when Peon sees no change in the session's inferred situation.

## Problem

`lastActivityAt` currently records meaningful situation changes only. This
correctly prevents spinner or TUI redraw output from making an idle-looking
summary appear permanently fresh, but it also makes a continuously running
session appear inactive for hours.

## Design

Add a persisted `lastOutputAt` timestamp to session metadata and the session
API. Update it whenever the sidecar receives non-empty PTY output for a live
session. It represents observed terminal recency only; it does not change
Peon's summary, attention, event checkpoints, or metadata-source priority.

The session list's “last active” label, recency ordering, and day grouping will
prefer `lastOutputAt`, then retain the current fallback order of
`lastActivityAt`, `peonLastInference`, and creation time. The detail panel
continues to use `lastActivityAt` because its task-history refresh follows
meaningful summary checkpoints, not raw output.

`lastOutputAt` is written to the existing session JSON so it survives a
sidecar/app restart. Older session files omit the field and continue to use
the existing fallbacks.

## Boundaries

- No change to terminal input, Peon inference scheduling, event-log format, or
  summary-checkpoint behavior.
- No additional dependencies or IPC boundary imports; the Electron and
  renderer copies of the API contract remain independently owned.
- Terminal output remains bounded by the existing replay limits.

## Verification

- Backend regression tests prove that receiving PTY output persists and
  exposes `lastOutputAt` without changing `lastActivityAt`.
- Frontend tests prove list labels, ordering, and grouping prefer
  `lastOutputAt`, while meaningful-activity fallbacks remain intact.
- Run the affected Rust and desktop suites, TypeScript check, documentation
  currency check, worktree currency check, and lightweight review.
