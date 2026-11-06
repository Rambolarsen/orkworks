---
type: Implementation Plan
title: Captured SQL session-image cleanup
description: Remove one original image bundle through its shared publication owner before recording SQL progress.
tags: [orkworks, sqlite, sessions, images, storage]
status: stable
workflow_status: implementation-complete
---

# Captured SQL session-image cleanup

Continue [#1243](https://github.com/Rambolarsen/orkworks/issues/1243) after #1449,
within the accepted [session-storage design](../specs/2026-11-01-sqlite-session-storage-design.md)
and [image ownership contract](../../../specs/session-image-context.md).

## Outcome and decisions

Add only `CapturedDeletion::remove_images`, recording progress bit `0x04` after
durable file success with `complete=false`. Keep the exact tombstone/job; no
production selection, migration, retention, settings/recommendation cleanup or
final row removal. Legacy image publication/recovery/deletion APIs stay unchanged.

- Reuse the existing bounded captured deletion runner, its retained slot,
  original memory/lease/reader/runtime and exact SQL identity checks. Bind image
  root and target directory handles read-only in the existing blocking capture
  outside application guards; retain independent image binding errors as file
  outcomes so other stages remain usable.
- Add a narrow SQL facade inside the image storage owner, reusing its private
  directory verification, no-follow relative opens and global publication lock.
  Metadata root must match the captured workspace lease. Pinned parent handles
  survive path replacement; a replaced target directory fails before unlink.
  Captured absence rejects later entries and requires explicit recapture/retry.
  When the entire image root was absent, admitted removal atomically creates
  the private root (EEXIST means retry), acquires the same publication lock and
  rechecks target absence. Capture remains read-only; all success paths retain
  the publication fence through progress, including initial root absence.
- Only UUIDs can own published image bundles. Non-UUID ordinary IDs are an
  explicit no-image success without resolving their paths. macOS/Linux file
  behavior is exercised by fixtures; other platforms return Unsupported before
  effects.
- Acquire the existing nonblocking image lock after admission, outside guards;
  keep it through definitive file/progress outcome. Busy publishers/recovery
  make cleanup retryable. Verify bounded manifest owner/session before effects;
  never delete ambiguous bundles. An empty directory without its marker may
  finish a prior partial deletion under the exact SQL job and exclusive lock.
- Enumerate at most the accepted eight images plus owner marker; reject extra
  entries before effects, never recurse. Unlink payload leaves without following
  links; synchronize the child before removing its owner marker, then sync the
  child, remove the verified target directory and sync image/workspace parents.
  Retry must sync even after absence/prior unlink. Failures retain partial
  receipts and progress is not attempted after file error.

## Delivery and verification

- [x] TDD real image-bundle removal and exact SQL progress/reopen; unrelated
  bundles, replay/events, workflow evidence and legacy metadata survive.
- [x] Test publication contention, guard retention across SQL progress failure,
  malformed/foreign/missing markers, nested obstruction, bounded enumeration,
  sync-failure retry, original directory/lease identity, late-created paths and
  unsupported/non-UUID behavior. Reuse existing runner cancellation/capacity tests;
  prove pre-admission cancellation and admitted detached completion for images.
- [x] Stall admitted image work on one Tokio worker; verify free application
  guards and acknowledged independent input. Run scoped suites, full repository
  verification and current-diff medium review.
- [ ] Record PR/CI and owned cleanup on the PR and tracking issue.

## Plan review

Ready under existing storage/image authority. Main uncertainty was durable
acknowledgement: legacy image removal does not sync an already-absent target and
reopens the child, so use a pinned SQL variant rather than repurpose legacy
semantics. Publication lock/root creation is an admitted effect, never capture I/O.
A fresh reviewer found a check/sync gap on initial root absence; a real publisher
was admitted in the regression before the fix. Establishing the existing private
namespace after admission closes this gap without changing publication or adding
a lock protocol. Both medium delta reviews of the changed absence path are clear.
Dependencies3(existing application/SQL/image owner), blast radius2(private stage
plus reused helpers), state changes2(fixtures only), reversibility1,
uncertainty2(controlled file/lock/progress faults), total10. Physical power-loss,
platform activation and numeric latency remain separate qualification gates.

Final local verification passed: 19 focused image tests and full repository
verification (3,048 Rust unit tests, seven integration, 31 OpenCode reporter,
Electron ABI proof, desktop type-check/build and 1,691 tests, docs 1,202 checks
and build). The publication-fence finding was reproduced red, fixed and
re-reviewed. Generic public-doc reminders need no user-facing claim change for
this private stage; architecture and validation document the actual boundary.
