---
type: Implementation Plan
title: Shared recommendation operation ownership
description: Fence complete recommendation operations without waiting under application locks before captured SQL deletion integration.
tags: [orkworks, taskmaster, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Shared recommendation operation ownership

Prerequisite under [#1243](https://github.com/Rambolarsen/orkworks/issues/1243),
after #1465 and the accepted [storage contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion).

## Outcome and boundary

Keep RecommendationStore as the recommendation/rollup graph authority. Add an
owned nonblocking operation permit shared by clones and independent opens of the
same canonical workspace root. Reads participate because they perform recovery.
A busy operation returns typed Io/WouldBlock before recommendation I/O; no caller
waits behind a slow operation while holding application locks.

- Use a short memory-only registry of weak owner references keyed by canonical
  workspace root; prune expired entries at open. Root initialization/canonical
  resolution precede registry lookup; recommendation directory creation,
  migration, recovery and validation follow permit acquisition. The registry
  never performs filesystem work while holding its mutex and caches no records.
- Acquire once per public store operation across the complete transition and
  all nested get/put/recovery work. Split only get/put internals to avoid
  recursively acquiring the same non-reentrant permit. Preserve transition,
  graph/hash/packet validation, transaction publication and recovery algorithms.
- Audit actual opens: workspace preflight and assessment-access redaction open
  independent stores; both must converge on the same owner, including filesystem
  aliases after canonicalization. No per-MetadataStore or adopted-only fence.
- The workspace lease remains cross-process authority. This registry is process
  local and canonical-path scoped, not a directory identity or deletion grant.
  A future cleanup facade must bind original handles/lease, bound graph I/O,
  retain the permit through SQL progress, and qualify durable retry separately.

No captured recommendation deletion/progress bit, new recovery format, SQL
activation, migration, protection-policy change or final Delete completion.
Existing synchronous recommendation I/O remains in its current application paths;
this prerequisite establishes store ownership only.

## Delivery and verification

- [x] TDD: held permit rejects reads, mutation, graph deletion and independent
  reopen before recovery; release permits retry. Real graph/packet transitions
  retain their semantics without nested self-conflict.
- [x] Test shared canonical alias opens, independent roots, recovery after a
  busy open, early-error/panic release, owned lifetime, complete mutation
  contention and free application guards/acknowledged terminal input.
- [x] Run store/assessment-access regressions, full repository verifier and
  current-diff medium review.
- [ ] Record PR CI and owned cleanup at terminal state.

## Plan review

Ready under existing recommendation and accepted ordinary-storage ownership.
Inspection found get/list/list_with_hashes recover transactions, composite
mutations call get/put, and assessment_access::reconcile_metadata opens separately.
A blocking store mutex would introduce input-lock waits; adopted-store cloning
alone would leave the independent redaction open unfenced. Canonical root weak
owners preserve the existing logical owner without a second record cache.
Dependencies3(existing store/application/redaction), blast radius3(all store
operations), state changes1(no format/algorithm changes), reversibility1,
uncertainty2(existing graph recovery plus contention fixtures), total10. Original
file anchoring, finite graph-read bounds and durable SQL cleanup remain next-stage
qualification, not claims of this prerequisite.

Investigated next-stage boundary: graph publication can add records and verifies
only supplied expected hashes. A caller snapshot retained across separate store
operations still requires exact source-session/tombstone revalidation or producer
revocation before captured cleanup activation. The permit deliberately does not
claim to qualify stale application snapshots or change graph publication policy.

Final verification passed: 53 store tests (including nine new owner tests),
14 assessment-access tests, then full repository verification with 3,089 Rust
unit tests (18 existing ignored), seven integration, 31 reporter, one Electron
ABI proof, desktop type-check/build and 1,694 tests (three existing skipped),
docs 1,202 checks/build and diff/doc/worktree checks. Independent medium owner/
concurrency and producer/recovery reviews were clear on the unchanged product
diff. The public-doc reminder needs no installer or user-flow claim change for
this ownership prerequisite; internal architecture/validation state its limits.
