---
type: Implementation Plan
title: Lifecycle replay and event ordering
description: Preserve confirmed replay-grid ordering and report actual partial lifecycle persistence without enabling SQL routing.
tags: [orkworks, sessions, lifecycle, storage]
status: stable
workflow_status: active
---

# Lifecycle replay and event ordering

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [external payload and partial-result contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion)
and its [scheduling/publication rules](../specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication).

## Outcome and scope

For a persisted ending record, confirm the exact final replay grid before
committing ended metadata. Append the status event only after that commit and
publish only to the original runtime/workspace. Return a private stage result
that distinguishes replay failure, metadata failure, committed metadata with
failed event append, and runtime-only completion for absent storage.

This changes the existing JSON finalizer owner, not format selection. The dormant
SQL lifecycle boundary stays unrouted. No cross-file atomicity, replay pruning,
process adoption, new retry protocol or measured responsiveness is claimed.

## Evidence and decisions

- `complete_session_ending` currently ignores replay persistence failure and then
  writes ended metadata/event. Its current boolean also loses event-append status.
- Replay size precedes metadata; metadata precedes its status event and live
  publication. Use the existing terminal-size serialization gate and metadata
  transaction/cache owners; introduce no competing file/JSON worker.
- Capture original workspace/reader/lease and runtime/control/creation/harness identity
  before blocking-job dispatch or waiting for replay serialization. Recheck at the effect cutoff and
  publication. A reused record or handle cannot inherit a prior finalizer.
- Keep production physical-exit observation, output drain, final inference,
  cleanup and close-first organization with their existing owners. A failed
  replay/metadata stage remains pending and can be retried after repair using
  freshly validated state. Do not retry an uncertain event append blindly.
- Missing storage still permits known physical-runtime completion as today;
  distinguish it from a committed durable record. Event failure cannot roll back
  committed metadata and must be reported as an actual partial result.
- Reject zero/out-of-range renderer resize dimensions before dispatch, and zero
  dimensions at runtime admission. Startup capture otherwise records zero before
  the PTY backend can reject it, leaving an unconfirmable final grid. Preserve
  valid positive `u16` dimensions and existing omitted-field defaults.
- Run final persistence on its own retained blocking job from the async runtime
  finalizer. It owns completion after waiter cancellation. Existing shared guards
  around legacy JSON I/O remain unqualified for complete async storage routing.

## Delivery and checks

- [x] Reproduce failed replay-grid finalization and stale owner effects using real
  files and deterministic pauses before history and before live publication.
- [x] Extract the ending completion into `session_application/ending_completion.rs`,
  with private per-stage outcomes and an error-returning terminal-size write.
  Preserve the boolean compatibility wrapper and absent-storage fallback.
- [x] Exercise grid-before-record-before-event-before-live ordering; replay and
  metadata failures leave ending data intact and recover after repair. Event
  failure reports committed partial success; original creation/control/workspace
  fences prevent stale file/live effects. Verify cancelled waiters retain the
  definitive job and one Tokio worker continues during paused persistence.
- [ ] Update architecture/validation claims, run affected and complete repository
  checks, obtain current-diff review and require all applicable CI before merge.

## Risk review

Dependencies 3 (existing runtime, metadata and external-history owners); blast
radius 3 (physical ending publication); state changes 2 (existing records/files,
no new format); reversibility 2 (code revert, no migration); uncertainty 2
(deterministic failure/owner tests required). Total 12.

Reviewed once: Ready within the accepted storage contract and existing owner
approval. Physical death and metadata completion are distinct. The main risks
are accidental owner adoption and claiming rollback after a committed record;
per-stage results and real-file regressions directly address them. Shared-lock
JSON scheduling and complete SQL lifecycle qualification remain separate gates.
