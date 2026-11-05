---
type: Implementation Plan
title: Captured SQL close-first Delete admission
description: Admit recoverable deletion jobs only after the captured runtime finalizes, preserving original ownership and definitive SQL results.
tags: [orkworks, sqlite, sessions, runtime]
status: stable
workflow_status: implementation-complete
---

# Captured SQL close-first Delete admission

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [storage contract](../../../specs/session-storage.md),
[storage design](../specs/2026-11-01-sqlite-session-storage-design.md) and
[organization ownership plan](2026-11-05-sql-organization-actions.md).
The owner authorized continuing after recovery discovery in #1413.

## Outcome and boundaries

Add a source-only `CapturedOrganization::begin_delete` path that stops and
awaits the exact original runtime before atomically admitting its SQL tombstone
and progress-zero deletion job. A `DeletionAdmission` identifies the session
and creation plus committed/current or committed-detached outcome. It means
cleanup is pending, never physical deletion success or cleanup authority.

No external file/evidence removal, cleanup-stage policy, scheduler, projection
selector, HTTP token, production activation, schema change or JSON fallback.
The existing `apply(Archive/Restore)` contract and revision receipts remain.
Ordinary `apply(Delete/Close)` continues rejecting unsupported complete actions.

## Decisions and ownership

- Extend the existing prepared organization-action ticket with typed Delete
  preparation and a typed result distinguishing revision changes from deletion
  admission. Keep the existing bounded mutation queue, session lane, execution
  request/grant, cancellation CAS and retained completion resources. Do not copy
  the close/wait/grant state machine into a second application owner.
- Share the captured organization's private execution path. Delete is allowed
  from captured Active, Closed or Archived. SQL preflight and original memory
  checks still precede the action reservation and captured stop. Revoke its
  exact optional evaluation carrier off application guards before rechecking
  and signaling the original sender. Production carriers stay None.
- Preserve the 15-second close budget and 25-ms polling, leaving the SQL lane
  free for finalization. Require the same physical close predicate, including
  PID/control/resume/attachment cleanup, before preparing the mutation.
- For Delete, reread the latest coherent organization record after closure,
  outside guards; check original creation and organization revision and use its
  latest metadata revision. Closure legitimately advances metadata revision.
  The existing `begin_delete` transaction independently checks exact creation,
  metadata/organization revisions and canonical finalization; concurrent later
  mutations conflict without admitting a job. No blind retry or file rollback.
- Keep original lane, lease, action reservation and budget through definitive
  SQL completion/publication. Cancellation before the atomic grant prevents
  admission; after grant it drains completion even if grant delivery is late.
  Original-owner invalidation only; detached receipts never update replacement
  runtime metadata. The tombstone remains recoverable after resource release.
- Future external cleanup must capture its own exact identity, lease and
  existing evidence owners before any destructive effect. Neither this receipt
  nor discovery pages grant that authority. SQL identity cannot be reused while
  its tombstone exists. Production remains JSON.

## Deliveries and checks

- [x] Write real-store failing Delete admission regressions, verify missing API
  failure, then implement the shared path and prepared typed command/result.
- [x] Verify Active close/finalizer independence and all physical cleanup gates,
  Closed/Archived/history-only admission, progress-zero job and reopen recovery,
  unchanged JSON and external payloads, stale original owners, invalid/missing
  SQL, revision changes during admission and independent canonical close checks.
- [x] Verify timeout/cancel before grant, queue rejection/expiry, admitted
  cancellation with retained lease/lane/reservation, detached completion and
  successful-grant delivery after cancellation/deadline. Reuse existing bounded
  test gates, extending them to the prepared Delete command.
- [x] Stall Delete SQL on a single Tokio worker with an OS watchdog, verify
  application guards remain free and another runtime receiver acknowledges
  approved input. This is receiver-stage evidence, not PTY latency qualification.
- [ ] Run affected suites and full `bash scripts/verify-repo.sh`, current-diff
  medium review, complete PR/CI inventory, applicable recommendation tie-off and
  owned worktree cleanup. Document source-only admission and remaining cleanup.

## Plan review

Ready for inline execution under existing storage approval. One coupled outcome;
no new persistent format or external-destructive policy. Dependencies 3 (known
runtime/action/worker owners), blast radius 2 (private shared action path), state
changes 2 (source-only fixture tombstones), reversibility 1 (source-only revert),
uncertainty 2 (controlled cancellation/queue stages), total 10. Review checked
revision changes during finalization, stale grants, receipt meaning and external
cleanup exclusions; no unresolved design choice changes accepted architecture.

Local implementation and verification completed: 13 Delete admission regressions,
35 captured SQL write tests and full repository verification passed. Fresh medium
correctness and contract/completeness reviews found no actionable issues. PR/CI
and terminal-state cleanup are recorded in the PR and tracking issue.
