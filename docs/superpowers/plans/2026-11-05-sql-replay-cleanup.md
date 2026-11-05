---
type: Implementation Plan
title: Captured SQL replay cleanup stage
description: Remove replay payloads through an original-owner durable file job before recording recoverable SQL progress.
tags: [orkworks, sqlite, sessions, storage]
status: stable
workflow_status: implementation-complete
---

# Captured SQL replay cleanup stage

Tracking [#1243](https://github.com/Rambolarsen/orkworks/issues/1243), continuing
#1413 and #1419 under the accepted [storage design](../specs/2026-11-01-sqlite-session-storage-design.md)
and [ordinary-session contract](../../../specs/session-storage.md).

## Outcome and boundaries

Add source-only exact tombstone capture and one replay cleanup stage. Delete only
that identity's event NDJSON, terminal replay and terminal-size files through the
metadata file owner; sync removal before recording progress bit `0x01`. Keep the
SQL tombstone/job for later evidence, image, settings and recommendation stages.
Never complete/remove the SQL row/job here. No production route, scheduler,
retention, migration or activation; ordinary JSON deletion remains unchanged.

## Decisions

- `SessionWrites::capture_deletion(id, created)` captures the original workspace,
  lease, reader, organization owner and optional runtime/control/creation before
  the exact SQL tombstone read and rechecks afterwards. Require physical closure
  and canonical finalized tombstone metadata; ordinary rows, stale creations and
  corrupt SQL cannot authorize file effects. A discovery page/receipt alone is
  insufficient. Capture grants no process stop/adoption authority.
- Consuming `CapturedDeletion::remove_replay` uses the same session lane and
  organization reservation as other application writers. Re-read the exact job
  under the lane, then atomically admit the file job under original memory and
  physical-close checks. Two-second pre-admission expiry/cancellation competes
  with admission; after admission retain definitive completion through caller
  cancellation and workspace/runtime detachment. No guard spans filesystem/SQL
  waits. Retain original lease, lane, reservation and capacity through completion.
- Lift the existing evidence file-job budget to the SQL application boundary;
  evidence and cleanup share its unchanged 32-job/32-MiB total capacity and
  1-MiB intent maximum. Do not introduce an unbounded background queue.
- Add a narrow metadata replay-cleanup facade sharing the original checkpoint
  cache, captured without I/O. On Unix open original root/events directories with
  no-follow directory handles, unlink only the three validated exact basenames,
  then sync events and workspace directories. Absent files/directories are
  retryable success after directory sync; errors and partial removals stay
  explicit. Retrying even already-absent files must repeat sync. Unsupported
  platforms fail before file effects instead of acknowledging fake durability.
  Existing production `delete_events` and its I/O ordering stay unchanged.
- After durable files success, write `progress | 0x01` with `complete=false` to
  the exact existing job. Preserve other bits and never claim cross-file SQL
  atomicity. A progress failure leaves durable file effects and a retryable job;
  retries use the same idempotent file owner. All conflicting application writers
  must retain the shared lane/reservation, as required before activation.
- Return an explicit must-use partial receipt: file result, optional progress
  result (not attempted after file failure), and detached status. Admission
  errors have no file effects. Caller loss retains/logs admitted results. No
  cleanup of other identities, legacy JSON, workflow evidence, images, last-active
  settings or recommendations. Native tool/repository content stays untouched.

## Delivery and evidence

- [x] TDD exact closed tombstone capture, durable replay removal/progress/reopen,
  unrelated payload/legacy preservation, and primitive sync-failure retry tests.
- [x] Implement the facade and captured stage; test stale workspace/reader/lease/
  runtime/control/creation, live or missing/corrupt jobs, cancellation/expiry,
  competing reservation/lane, admitted resource retention and detached receipts.
- [x] Test partial file errors, SQL progress failure after durable removal,
  bounded admission and no false final-job completion. Reuse shared budget tests.
- [ ] Stall admitted removal behind a bounded OS gate on a single Tokio worker;
  verify free application guards and another runtime's acknowledged input.
  Verify no nested blocking-worker wait. Run affected suites, full repository
  verification, current-diff medium review, PR/CI inventory and owned cleanup.

## Plan review

Ready under existing storage approval: one coupled stage, no new authority,
external format or production selection. File durability is the prerequisite
found in the existing owner; SQL progress must never precede it. Dependencies 3
(existing action/store/file owners), blast radius 2 (source-only stage plus shared
budget/cache handles), state changes 2 (fixture-only payload removal/progress),
reversibility 1 (no live selection), uncertainty 2 (controlled file/SQL failures),
total 10. Directory sync ordering is tested; physical power-loss and supported
platform/filesystem qualification remain activation evidence, not inferred here.

Local implementation complete: final 15 scoped cleanup tests and full repository
verification passed. Current-diff medium correctness/filesystem reviews are clear
after reproducing and fixing the accepted maximum-ID basename-limit case. PR/CI
and terminal cleanup are recorded in the PR and tracking issue.
