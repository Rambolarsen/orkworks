---
type: Implementation Plan
title: Captured ending provider context
description: Qualify final-scan provider metadata against original SQL ending ownership while keeping evidence-file routing gated.
tags: [orkworks, sessions, sqlite, providers]
status: stable
workflow_status: implementation-complete
---

# Captured ending provider context

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [scheduling/publication contract](../specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication)
and [Task 2](2026-11-01-sqlite-session-storage.md).

## Outcome and boundary

A final scan can persist provider context through its original `CapturedEnding`
without reacquiring authority from a replacement runtime. Use the shared provider
reducer, latest SQL creation checks and existing bounded worker/session lane;
require the stored record still be ending. Return an explicit unchanged, committed
or committed-detached receipt plus existing typed storage errors. Preserve all
attention, provenance, activity and title fields; this metadata-only operation
neither projects live provider fields nor writes events/evidence or process state.

Default JSON final-scan behavior remains unchanged. Workflow evidence has its own
file-store deletion, sequence and idempotency owner; it cannot move outside shared
guards merely by capturing a path. Evidence qualification, full SQL final-scan
routing, live activation and measured responsiveness remain separate gates.

## Decisions and investigated risks

- Add an ending-scoped internal provider intent reusing `apply_provider` and
  `WriteScope::NONE`. A stale provider observation must not mutate a record that
  has already ended even if live publication of that ending was detached.
- Borrow the immutable captured handle so a current provider write can precede
  history completion using the same original authority. Do not refresh its
  attention/control/source snapshot or weaken the finish fences.
- Prepare bounded intent before lane waits, retain the original two-second
  deadline and cancellation CAS. No second queue, JSON read/mirror or provider
  dispatch is introduced. Runtime inference remains with its existing owner.
- An owned task retains lane, lease, budget and definitive result after SQL claim.
  Revalidate original memory ownership before claim and completion receipt.
  Replacement after commit yields committed-detached, never rollback. Typed
  unavailable results cannot prove rollback; never blindly retry them.
- Retain metadata-only compatibility: accepted provider writes advance revision
  once even when values repeat, but do not alter attention/activity clocks or
  trigger unrelated live projection changes.

Risk sizing: dependencies 3 (captured owner, worker and shared reducer), blast
radius 2 (dormant final-scan path), state changes 2 (test stores), reversibility 1
(source revert), uncertainty 2 (stored/live phase divergence), total 10.

## Delivery and checks

- [x] Reproduce generic provider mutation changing an already-ended record,
  demonstrating why the final-scan adapter requires a scoped intent.
- [x] Add the guarded provider intent and captured write/typed receipt while
  preserving the existing generic provider command and JSON route.
- [x] Test real SQL values/revisions and preserved clocks, stored phase/creation
  rejection, original owner changes, lane exclusion, cancellation on either side
  of claim, definitive resource retention and executor/input independence.
- [x] Update architecture and validation with exact remaining evidence gates.
- [ ] Run affected tests, full repository verification, current-diff medium review
  and applicable CI before integration.

## Plan review

Ready for inline execution under the existing approved storage contract. Provider
metadata is independently valuable and uses an already-shared reducer. Evidence
files are deliberately not coupled into this mutation: their deletion/idempotency
ownership needs separate qualification before full SQL scan routing. No new
architecture, storage format, automatic retry or deployment is proposed.
