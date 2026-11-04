---
type: Implementation Plan
title: SQL finalization history ordering
description: Qualify dormant SQL finalization with ordered external replay and event stages and definitive partial results.
tags: [orkworks, sessions, sqlite, lifecycle]
status: stable
workflow_status: implementation-complete
---

# SQL finalization history ordering

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [scheduling](../specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication)
and [external payload](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion)
contracts and [Task 2](2026-11-01-sqlite-session-storage.md).

## Outcome and boundaries

Add an explicit dormant SQL finalization operation: confirm the original runtime's
final grid, commit the lifecycle reducer, append its committed status event, then
publish to the original live owner. Return replay, metadata and event outcomes
separately. Preserve the existing metadata-only lifecycle operation and production
JSON routing. No process exit, resume/spawn, archive/delete, migration, installed
producer or measured latency qualification is included.

## Decisions and investigated risks

- Reuse the worker's bounded prepared command, deadline, per-session application
  lane and retained workspace lease; no new scheduler or retry registry.
- Capture original workspace/reader/runtime/control/creation/source before dispatch.
  Read exact SQL identity under the session lane before external effects; missing,
  corrupt or replaced SQL data cannot authorize a JSON fallback.
- One retained blocking owner holds the existing terminal-size serialization gate
  across grid, SQL and event work. It drives async worker waits through Tokio's
  handle, leaving Tokio workers and shared application guards available. SQL is
  not queued until grid confirmation, so a file stall cannot occupy the SQL worker.
- Revalidate memory ownership after acquiring the grid gate and before SQL execution
  and live publication. File effects already admitted retain their original lease;
  replacement handles receive no publication. Existing metadata-only writes continue
  sharing the session lane; close-intent reservation stays separate.
- Failed grid prevents metadata/event. SQL errors report the confirmed grid and storage error; unavailable results do not prove rollback.
  Failed event preserves committed metadata and permits original live publication.
  No blind append retry or cross-file atomicity is claimed. Caller cancellation
  competes with the existing SQL execution claim; the blocking owner retains all
  resources and records definitive failures after waiter loss. A pre-SQL cancelled
  operation may already have written the grid, which is an explicit partial effect.

Risk sizing: dependencies 3 (worker, reducers, external history), blast radius 2
(dormant operation), state changes 2 (test stores only), reversibility 1 (unrouted
source revert), uncertainty 2 (new compound-stage coverage), total 10.

## Delivery and checks

- [x] Reproduce missing grid/event behavior against the current real SQL finalizer.
- [x] Extract shared metadata execution/publication helpers and add the explicit
  ordered finalizer with typed partial outcomes and retained ownership.
- [x] Test real grid and event failures, SQL rejection, repair, no blind event retry,
  original ownership before effects and after commit, cancellation, lane/lease
  retention, and available application guards/Tokio/input during a stalled stage.
- [x] Update architecture and validation evidence with exact remaining gates.
- [ ] Run affected tests, full repository verification, current-diff medium review
  and required CI before PR integration.

## Plan review

Ready for inline execution within the existing approved design. Reviewed against
the worker's separate grant/execute cutoff, cancellation CAS and completion-resource
ownership. The blocking owner must never hold projection/workspace/sessions guards
while waiting or doing file I/O. The global grid gate is an existing serialization
constraint, not a new input lock; complete producer routing remains separately gated.
The one-blocking-thread regression reproduced a recursive validation-pool wait; the
retained owner now runs those identical short memory checks directly on its own
thread, while normal async callers still dispatch them off the Tokio worker.
