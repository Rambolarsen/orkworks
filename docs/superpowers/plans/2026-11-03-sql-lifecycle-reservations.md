---
type: Implementation Plan
title: SQL lifecycle write reservations
description: Join dormant lifecycle metadata writes to the existing per-session SQL application lane without enabling production routing.
tags: [orkworks, sessions, sqlite, lifecycle]
status: stable
workflow_status: implementation-complete
---

# SQL lifecycle write reservations

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Implements the accepted [scheduling and publication contract](../specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication)
and [Task 2](2026-11-01-sqlite-session-storage.md). Execution is inline under the
existing owner approval; no format selection or new architecture is proposed.

## Outcome and boundary

A dormant lifecycle transition/finalization can use the same exact worker/session
lane as Peon and native metadata. It captures the original runtime, control,
creation, source and live phase; waits without application guards; revalidates
before execution; and publishes only its committed result to that original
handle. Cancellation before execution rejects the write; cancellation after the
execution claim retains definitive ownership through publication.

This qualifies the metadata application boundary only. Production JSON lifecycle,
resume/spawn, replay-size persistence, external events, close-first organization
and failure cleanup keep their existing owners. No lifecycle command spawns,
kills, archives or resumes a process. No SQL routing or native gate is enabled.

## Decisions and investigated risks

- Reuse `PreparedInternalMutation::lane`, admission bytes/slots, the original
  two-second deadline and `RuntimeWriteResources`; no second queue/registry.
- Expose typed transition/finalize requests, deriving reducer fields from the
  current exact handle rather than accepting caller-supplied live authority.
- Keep close-intent reservation distinct from the short mutation lane:
  `session_actions.rs` holds its organization reservation while awaiting the
  runtime finalizer. Reusing it for finalizer admission would deadlock.
- Capture and claim under bounded memory-only application guards; SQL admission,
  waits and execution occur outside them. Revalidate original workspace lease,
  runtime/control/creation/harness and phase before claim and publication.
- Reuse existing lifecycle reducers. Starting an ending transition is not proof
  of physical process exit; ended publication does not satisfy external cleanup.
- After commit a replacement handle yields committed-detached, not rollback.
  Metadata-only failure must not claim external events or live process success.

Risk sizing: dependencies 3 (existing worker, reducer and runtime owners), blast
radius 2 (dormant boundary), state changes 2 (test SQL stores only), reversibility
1 (unrouted code revert), uncertainty 2 (new lifecycle concurrency coverage).
Total 10. Production cutover remains outside this task.

## Delivery and checks

- [x] Add regression tests using the real SQL worker for transition/finalize
  parity, same-session cross-writer lane exclusion, different-session lanes,
  stale workspace/runtime/control/creation/source/phase, queued cancellation,
  execution cancellation, detached publication and typed storage failures.
  Observe a semantic failure before the implementation.
- [x] Add `sql_writes/lifecycle.rs` as the sole owner of typed lifecycle capture,
  claim and committed live publication. Reuse the existing worker path and
  `Outcome`; keep legacy lifecycle methods and external effects unchanged.
- [x] Exercise application guards and one Tokio worker during stalled execution;
  prove the lane/lease survives cancelled waiters through definitive completion.
- [x] Update the validation record with the exact source-only scope, missing
  external/lifecycle owners and no activation/performance claim.
- [ ] Run affected Rust tests, `bash scripts/verify-repo.sh`, current-diff
  medium review and applicable CI before PR integration.

## Plan review

Ready for inline execution within the approved contract. Reviewed once against
worker admission/cancellation, existing lifecycle reducers and close-first action
ordering. The principal gap is intentionally bounded: this does not route the
production lifecycle or authorize process/external-file effects. Those owners
must join before complete writer qualification or cutover.
