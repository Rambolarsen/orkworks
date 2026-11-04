---
type: Implementation Plan
title: Captured workflow evidence ownership
description: Proposed implementation and qualification of retained final-scan workflow file writes.
tags: [orkworks, sessions, workflow, sqlite]
status: draft
workflow_status: awaiting-approval
---

# Captured workflow evidence ownership

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [proposed owner design](../specs/2026-11-04-ending-workflow-evidence.md),
under the accepted [storage design](../specs/2026-11-01-sqlite-session-storage-design.md).
Owner approval of the proposed refinement is required before product-code edits.

## Deliverables

- [ ] Refine the existing workflow owner into a shared handle in
  `workflow_observations.rs` (focused private submodules permitted). Preserve one
  instance identity, evaluation generation, mutex, sequence and cache across all
  handles. Add identity-bound weak-registry permits and revoke them on deletion
  before destructive effects. Record the approved refinement in ADR 0042 before
  implementation, preserving its decision and file format.
- [ ] Extend immutable `CapturedEnding` acquisition to retain that exact store,
  permit and original lease before inference, with off-guard registration and
  post-registration authoritative SQL/original-memory checks. Add a bounded
  application-wide file-job admission owner and captured batch method under
  `session_application/sql_writes/lifecycle/history/`. Keep capacity separate
  from SQL worker commands and keep original candidate indexes/keys.
- [ ] Return explicit admission and ordered per-candidate receipts, preserve
  partial/unknown failures without automatically retrying `PersistFailed` and log definitive results after waiter cancellation.
  Recheck original ownership for receipt/evaluator scheduling. Leave production
  routing JSON and full SQL scan selection gated.
- [ ] Update architecture/validation with exact evidence, run affected Rust
  tests and `bash scripts/verify-repo.sh`, complete current-diff medium review,
  required CI and PR cleanup. No measured speedup claim without measurements.

## Required checks

Use TDD for each changed behavior, real workflow files and the real SQL worker.
Existing workflow tests must retain their unchanged recording/deletion semantics.

- Shared-handle parallel appends allocate distinct monotonic sequences; reopen
  reconstructs exact observations/counter/idempotency. Concurrent duplicate keys
  deduplicate and conflicting payloads remain conflicts across handles.
- Delete-first revokes a captured batch without allocating a sequence or
  recreating a segment; append-first finishes before deletion, which removes
  its final segment/cache/idempotency. Failed deletion revokes old permits too.
  Deleting another session preserves this one's permit. Expired permits leave
  no history-sized registry. Trimming and generation share the same owner.
- Register/delete/SQL-validation interleavings reject missing, ended or reused
  identities, including valid-but-conflicting JSON. Workspace/store/reader/lease,
  runtime/control, creation/source/attention replacements never renew authority.
- Five-candidate and exact byte/job/total capacity boundaries; oversized input,
  cancelled queued jobs and two-second pre-admission expiry have no effects.
  A single atomic claim arbitrates cancellation/expiry against file admission;
  pending native mutex jobs retain their bounded reservations until drained.
  Workspace switches cannot multiply the application-wide job/byte limits.
  Verify capacity and original lease/lane release after every definitive result.
- Ungrounded candidates preserve original indexes. Accepted/duplicate/rejected
  and partial persistence results keep exact durable IDs/sequences; failures of
  counter/append/rewrite never imply rollback or unbounded retry. Verify evaluator
  scheduling only for current newly accepted evidence, with detached receipts
  unable to touch replacement state.
- Stall file I/O with an independent OS watchdog on a single Tokio worker.
  Projection/workspace/sessions guards remain available and approved input to
  another live runtime reaches its receiver. Separately check a constrained
  blocking pool for nested-wait deadlocks. Label receiver evidence accurately;
  a new numeric/PTY claim needs its own corresponding fixture.

## Investigated risks and plan review

The source evidence and rejected alternatives are in the design; no separate
assessment artifact is needed. Dependencies 3 (workflow owner, SQL capture and
runtime receipt), blast radius 3 (shared store used by synchronous producers),
state changes 2 (memory-only permits, unchanged format), reversibility 2 (source
revert before activation), uncertainty 3 (new deletion/admission interleavings),
total 13. Per-session weak permits avoid a history-sized tombstone map. Existing
close-first deletion remains a qualification gate, not an implied completed task.

Disposition: ready for design review and owner approval; implementation blocked
on that concrete ownership decision. The builder may choose private type names
and file splits, but may not alter the admission, deletion or failure contract.
