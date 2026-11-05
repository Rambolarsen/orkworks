---
type: Implementation Plan
title: Captured SQL close-first organization actions
description: Qualify source-only Archive/Restore through original runtime closure, bounded SQL admission and retained action reservations.
tags: [orkworks, sqlite, sessions, runtime]
status: stable
workflow_status: implementation-complete
---

# Captured SQL close-first organization actions

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [storage contract](../../../specs/session-storage.md),
[storage design](../specs/2026-11-01-sqlite-session-storage-design.md) and
[close-first organization contract](../../agents/runtime-contracts.md#session-organization).
The owner authorized this integration after #1405 with “keep going”.

## Outcome and boundaries

Qualify source-only SQL Archive/Restore using the existing exact runtime closure
predicate and organization reservation. Preserve the original workspace lease,
metadata reader, organization owner, session creation, runtime identity and
control channel across reads, shutdown, queue waits and result publication.
A captured typed action view is the internal stale-action precondition; it is
not an HTTP token or a new public protocol. The existing HTTP action route and
production projection remain JSON. Delete and its recoverable external cleanup,
resume/spawn selection, migration and activation remain separate gates.

## Decisions and interfaces

- Add one exact `organization_state` worker read returning canonical metadata,
  indexed state and organization revision in the same read transaction. Keep
  existing canonical validation and finite queue/result bounds.
- `SessionWrites::capture_organization` captures memory before the SQL read and
  rechecks it afterwards. It returns an immutable `CapturedOrganization` with
  derived state; it holds no action reservation or lane during display.
- Consuming `CapturedOrganization::apply` permits Archive from Active/Closed and
  Restore from Archived. Re-read current SQL identity/organization revision and
  recheck original memory before acquiring the existing organization reservation
  or capturing its existing `CapturedRuntimeStop`. Close its exact optional
  evaluation carrier outside all application guards, retain the reservation/lease
  through that blocking job and recheck original ownership before signaling the
  captured sender. Preserve the already approved dormant close-only facade;
  production carriers remain None and no attachment/authority factory is added.
  Unsupported actions reject.
- Active Archive waits at most the existing 15 seconds, polling every 25 ms,
  for the exact original runtime to satisfy `runtime_closed`, including no PID,
  receiver closure, ended/dead, no resume and no terminal attachment. Never hold
  the SQL session lane while waiting for the finalizer; the action reservation
  must not block that finalizer.
- Prepare a typed organization command through the existing bounded worker and
  same per-session lane. The execution grant rechecks original ownership,
  physical closure and the close deadline before SQL starts. SQL itself checks
  canonical finalization and exact organization revision transactionally (#1405).
- Retain lane, lease, action reservation, queue budget and definitive result
  through execution/publication. Before the grant, cancellation prevents the
  organization write (a sent kill cannot be rolled back). After the grant,
  cancellation retains completion; a detached result cannot publish into a
  replacement runtime/workspace. Preserve metadata/native/activity fields.
- Memory validation/publication runs on blocking jobs; SQL and shutdown waits
  run outside application guards. Source-only receipts distinguish committed
  and committed-detached results, with the new organization revision. No JSON
  organization/file writes or fallback, no blind retries and no atomicity claim
  for external files. SQL projection/token publication is a separate selector
  integration; this owner only invalidates the original current session view.

## Deliveries and checks

- [x] Add a failing real-worker/runtime action regression. Add the coherent
  organization read and prepared typed command/resource receipt without changing
  existing internal mutation callers or the public action route.
- [x] Compose capture/admission/close-first wait/execution/publication with the
  original owners. Test canonical SQL metadata and unchanged legacy files,
  Archive/Restore and exact revisions, invalid/stale action views, runtime,
  control, workspace/lease/reader/organization replacement, missing/unavailable
  SQL, physical cleanup withheld after persisted ending and close timeout.
- [x] Test cancellation before claim and after admission, retained reservation,
  lane and lease, finalizer independence while close waits, queue expiry and
  conservative detached completion. Stall the real SQL stage on a single Tokio
  worker with an independent OS watchdog; check application guards and another
  runtime input receiver. This is receiver/stage evidence, not numeric latency
  or crash-surviving process adoption.
- [ ] Run affected suites and full `bash scripts/verify-repo.sh`, current-diff
  medium code review, complete PR feedback/CI inventory, recommendation tie-off
  if applicable and owned worktree cleanup after the PR reaches terminal state.

## Plan review

Ready for inline execution under the approved storage/runtime preservation
contract; no new authority, schema, external cleanup policy or production
selection. Dependencies 3 (existing runtime/action/worker owners), blast radius 2
(source-only composition and additive worker API), state changes 2 (fixture-only
SQL organization), reversibility 1 (source-only revert), uncertainty 2 (controlled
queue/cancellation boundaries), total 10. The typed view replaces no production
HTTP token; no SQL selection or physical-cleanup claim is inferred from metadata.
