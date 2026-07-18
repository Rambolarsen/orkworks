# Explicit session lifecycle phases with frozen final observed state

- Status: superseded by [ADR 0023](./0023-simplified-session-lifecycle.md)
- Deciders: Rambolarsen
- Date: 2026-07-04

## Context

Session state mixed three concerns in one place: process/outcome state in
`status`, inferred work classification in `phase`, and live observer state in
`observedStatus`. That left lifecycle behavior spread across runtime code,
metadata merge rules, and frontend guards, and created a race window between
process exit and the last Peon inference: a session could end and then have a
stale "blocked"/"waiting" observed status resurface as live attention.

Issue #26 tracks the fix; the detailed design is in
`docs/superpowers/specs/2026-07-03-session-lifecycle-phase-design.md`.

## Decision

- Rename the inferred work-classification field `phase` → `workPhase`
  (`ideation`/`implementation`/`review`/`debugging`/`unknown`).
- Add an explicit runtime lifecycle field `lifecyclePhase`
  (`creating` → `active` → `ending` → `ended`), distinct from `status`. All
  process exit paths pass through `ending`; during `ending` the session
  intentionally remains `status = running` and the intended terminal outcome is
  held in `pendingTerminalStatus` until finalization completes.
- During `ending`, the runtime attempts one final Peon scan (configurable
  timeout, `PEON_FINAL_SCAN_TIMEOUT`, default 2s). Success or failure, the
  session then transitions to `ended` and the last observed state is frozen
  into `finalObservedStatusSnapshot` (value, source, confidence, observed-at).
- `observedStatus` is live state only while `lifecyclePhase = active`. The
  frontend routes attention/sorting from `observedStatus` for active sessions
  and from the frozen `finalObservedStatus` for ended sessions.
- The metadata protocol (`sessions/<id>.json`) gains `workPhase` (with a
  backward-compatible read alias for legacy `phase`), `lifecyclePhase`,
  `pendingTerminalStatus`, `endingObservedStatusSnapshot`, and
  `finalObservedStatusSnapshot`. Reads normalize legacy files into the new
  shape; workspace open reconciles sessions orphaned mid-`ending` by consuming
  their pending terminal status.

## Consequences

- Lifecycle ownership is explicit: the exit-to-`ended` race is closed, and a
  session can no longer regain live attention from historical observer state.
- Terminal transitions must be idempotent — multiple exit paths (kill signal,
  DELETE handler, PTY errors) can race, so `set_session_status` applies the
  `ending` transition at most once and finalization is scheduled only by the
  path that won.
- The domain `Session` aggregate exposed the transition rules
  (`mark_active`/`begin_ending`/`complete_ending`), but the runtime always
  drove transitions through metadata directly and the aggregate was never
  wired in. It was removed as unwired dead code; see
  [issue #181](https://github.com/Rambolarsen/orkworks/issues/181) for a
  future typed-state-machine attempt informed by why this one didn't work.
  The runtime rules described elsewhere in this ADR are unaffected.
- Older metadata files keep working via read-time normalization; new writes
  always use the new field names.
