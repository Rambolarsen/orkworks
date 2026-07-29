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
API. Update the live session value whenever the sidecar receives non-empty PTY
bytes, including a frame that has not yet completed a newline. It represents
observed terminal recency only; it does not change Peon's summary, attention,
event checkpoints, or metadata-source priority.

The live value is immediately available to the API. Durable session-JSON
writes are coalesced to a bounded interval so chatty TUIs do not cause one
atomic metadata rewrite per PTY frame; the runtime flushes the latest pending
value before it finalizes an exited session. After a restart, the most recent
coalesced value remains available.

The session list's “last active” label, recency ordering, and day grouping will
select the newest valid value of `lastOutputAt` and `lastActivityAt`, then
retain the current fallback order of `peonLastInference` and creation time.
This preserves a later meaningful transition such as a terminal exit. The detail panel
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
- Update `docs/agents/domain-entities.md` for the new persisted metadata field
  and `docs/agents/architecture.md` for its API and recency semantics.

## Verification

- Backend regression tests prove that non-empty PTY output, including output
  without a newline, immediately exposes `lastOutputAt` without changing
  `lastActivityAt`; they also pin coalesced persistence and the exit flush.
- Frontend tests prove list labels, ordering, and grouping select the newest
  valid output/activity timestamp while meaningful-activity fallbacks remain
  intact.
- Run the affected Rust and desktop suites, TypeScript check, documentation
  currency check, worktree currency check, and lightweight review.
