---
type: Implementation Plan
title: Runtime ending persistence boundary
description: Connect runtime history completion to captured SQL ownership without enabling full SQL final-scan routing.
tags: [orkworks, sessions, sqlite, runtime]
status: stable
workflow_status: implementation-complete
---

# Runtime ending persistence boundary

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [storage scheduling and publication contract](../specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication)
and [Task 2](2026-11-01-sqlite-session-storage.md).

## Outcome and boundary

The runtime's history-completion stage can consume either its existing JSON
completion or an explicitly captured SQL completion owner. SQL captures original
workspace/reader/lease, runtime/control/creation and source state before a long
final-scan wait; completion must not recapture a replacement control/source owner.
Return the backend's complete typed stages, including partial failures and unknown
completion. Preserve the default JSON runtime flow and all physical exit/cleanup,
resume/spawn and close-first organization ownership.

This connects only history completion. The current final Peon scan still owns
JSON provider-context writes and workflow-observation files. Selecting SQL for
that entire pipeline before those owners are qualified would create divergent
JSON writes or lose evidence. No production selector, full SQL final-scan entry,
workflow-store ownership change, live activation or performance claim is added.

## Decisions and investigated risks

- Extract a private captured SQL ending handle from the existing history owner.
  Keep its fields private and consume it at completion; bind it to the exact
  application state as well as existing workspace/runtime/source fences.
- Reuse the current bounded prepared SQL command, worker/session lane, lease and
  cancellation claim. Do not hold the mutation lane across provider inference or
  create another queue. The metadata budget starts with the prepared command;
  the runtime driver remains responsible for bounded final scans and scheduling.
- Add a small runtime persistence boundary with explicit JSON/SQL selection and
  typed backend completion. The existing runtime chooses JSON. Its error logging
  must preserve unknown JoinError semantics; SQL failures retain StoreError kinds
  and definitive per-stage results rather than becoming a boolean success.
- Missing/replaced workspace/runtime/control/creation/source must be rejected
  before external effects and again before commit/publication. A stale admitted
  operation may report earlier progress; no retry or rollback inference is added.
- Close-first waiters retain their current physical cleanup/control-channel and
  persisted-ending gates. SQL completion alone cannot archive, delete, resume,
  spawn or prove process exit.

Risk sizing: dependencies 3 (runtime, captured SQL owner and legacy finalizer),
blast radius 2 (default JSON path plus dormant SQL stage), state changes 2 (test
stores), reversibility 1 (source revert), uncertainty 2 (ownership across waits),
total 10.

## Delivery and checks

- [x] Reproduce adoption of a changed control owner when only runtime identity is
  retained across a simulated scan wait and SQL captures ownership afterwards.
- [x] Extract the consumable SQL history capture; preserve the existing direct
  SQL API by capturing and immediately consuming the same owner.
- [x] Connect typed runtime history completion to the existing JSON finalizer and
  explicit captured SQL handle. Preserve current JSON behavior and errors.
- [x] Test real runtime-boundary SQL success, early capture/stale ownership,
  mismatched application state, storage errors, partial outcomes, cancellation,
  and close-first physical-cleanup semantics; retain existing history tests.
- [x] Update architecture and validation evidence with the final-scan routing gate.
- [ ] Run affected tests, full repository verification, current-diff medium review
  and applicable CI before integration.

## Plan review

Ready for inline execution within the existing approved storage contract. The
important investigated gap is the final Peon owner: connecting the entire runtime
would otherwise retain JSON provider/evidence side effects on a SQL path. The
bounded completion boundary is independently testable and closes the original
control/source capture gap without claiming that broader qualification. Existing
SQL execution and partial-result protocols remain unchanged.
