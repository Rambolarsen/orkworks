---
type: Design
title: Captured final-scan workflow evidence
description: Proposed retained file-store ownership and deletion fencing for delayed SQL final scans.
tags: [orkworks, sessions, sqlite, workflow, concurrency]
status: draft
workflow_status: proposed
---

# Captured final-scan workflow evidence

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Status: proposed ownership refinement; owner approval required before implementation.
This document does not authorize live SQL activation.

## Outcome and existing authority

Let a captured SQL final scan record grounded workflow observations without
holding projection, workspace or sessions locks during file I/O. Preserve the
existing workflow store's sequence, duplicate detection, rate limits, trimming,
diagnostics and deletion behavior. A delayed writer must not recreate evidence
after deletion or adopt a replacement workspace/runtime/session.

The accepted [session-storage contract](../../../specs/session-storage.md)
keeps workflow observations outside SQLite, with their existing owner.
[ADR 0042](../../adr/0042-workflow-observations-replace-summary-checkpoints.md)
and its [observation design](2026-08-14-workflow-observation-feedback-loop-design.md)
remain authoritative for content and file format. This proposal refines the
in-process lifetime and deletion boundary of that owner. It does not change
[ADR 0104](../../adr/0104-peon-scheduling-deletion-fence.md)'s synchronous Peon
scheduling fence or qualify the legacy recording paths for lock-free I/O.

## Investigated constraint

At source base `90bc6cb824cfe2125d235a2be406a7847ebb9c2d`,
`SessionApplication::persist_final_peon_scan` borrows the workspace while it
records provider context and workflow observations. `WorkflowObservationStore`
owns a mutex containing the sequence counter, idempotency entries, segment cache
and diagnostics. `delete_session_observations` removes both the segment and
its cached/idempotency state under that mutex. Reopening the same path produces
a distinct owner and can reuse stale caches or conflict over sequence allocation.

The captured SQL provider and history owners are qualified separately in
[#1374](https://github.com/Rambolarsen/orkworks/pull/1374) and
[#1367](https://github.com/Rambolarsen/orkworks/pull/1367).
They do not own workflow files. Merely copying a path or using `spawn_blocking`
with the workspace guard retained would not satisfy the accepted storage contract.
A retained store alone also permits a delayed append after file deletion.

## Proposed owner

Make the existing workflow store an owned, cheaply cloned handle to one shared
in-process state. Share its instance identity, directory, mutex-protected state
and evaluation generation; never call `open` to capture an existing owner.
Existing synchronous callers retain the same methods and behavior. This is one
owner with multiple handles, not multiple independent caches or connections.

Add a private session-scoped write permit to that owner, bound to session ID
and creation identity. Permit registration and revocation use the same store
mutex that serializes record/delete, on blocking threads outside application
guards. An active permit is shared across captures of the same identity; the
registry holds weak references and removes expired entries so it does not grow
with historical sessions. A permit is memory-only authority, not a durable
session record, deletion job or restart tombstone.

Register the permit before the provider inference wait. Capturing it may wait
on the evidence mutex, so do that outside input-critical guards, then revalidate
the immutable original application/workspace/reader/lease, runtime/control,
creation, phase and source snapshot. Check authoritative SQL identity and ending
phase after registration; a capture cannot use valid legacy JSON as a fallback.
If deletion preceded registration, the required SQL/memory checks must reject
the missing, ended or replaced session. If deletion follows registration, its
permit revocation rejects the delayed writer. Do not refresh the snapshot or
hold a mutation lane, queue reservation or store mutex across provider inference.
The capture retains the original workspace lease and exact evidence-store instance.

Before evidence execution, acquire the existing session mutation lane, verify
SQL creation/harness/ending phase, and revalidate the original captured memory
owner outside file I/O. All lane/worker waits occur outside application guards.
Then acquire the evidence mutex on the blocking job and check the permit before
any sequence, segment or cache effect. This is the file admission point.

Deletion revokes every permit for that session under the evidence mutex before
attempting removal, including when removal fails. Removal/cache-clearing keeps
its existing error and sequence semantics. A failed deletion does not revive
old permits; a later capture needs fresh authoritative validation. Existing
ordinary synchronous recording remains available through its existing guarded
application routes; it cannot obtain a captured permit implicitly.

If a bounded batch wins file admission, it finishes under that same mutex before
deletion can remove the segment and forget its idempotency entries. If deletion
wins, the old batch has no effect, including no sequence allocation. Store code
never calls back into application/SQL owners while holding the evidence mutex.
No application guard waits for that mutex on the new path. Existing deletion
routes may still wait on it under their current guards; their conversion remains
part of separate close-first SQL deletion qualification.

## Bounded execution and receipts

The captured adapter accepts at most five candidates, matching Peon's existing
`MAX_WORKFLOW_CANDIDATES`. Validate candidate lengths/confidence with the existing
workflow validator, reject oversized requests explicitly, and ground evidence
against the same bounded final-output snapshot before file admission. Preserve
original candidate indexes and `final-scan:{generation}:{index}` keys; filtering
must not renumber keys. Keep source Peon and existing fingerprint/payload hashes.

Use one application-wide bounded admission reservation for retained file jobs,
shared across current and retained old workspace owners: at most 32 jobs,
at most 1 MiB captured intent per job and 32 MiB total, rejecting unavailable
capacity rather than building an unbounded blocking-task queue. This reservation
is separate from the SQL worker; workflow bytes never become a SQL mutation.
Use a two-second deadline for work that has not reached file admission. Acquire
capacity before scheduling blocking work, retain it through the definitive
receipt and release it on all exits. Do not hold application guards while
checking capacity, waiting for the lane, reading SQL or doing file work.

Cancellation and expiry race file admission through one atomic claim. If they
win before admission, the caller may receive a definitive no-effect result even
while a blocked native mutex acquisition is still draining. That job keeps its
bounded reservation until it exits; it must recheck the claim under the evidence
mutex before touching files or sequence state. If admission wins, neither timer
nor caller cancellation can convert that result into a no-effect promise.
Once a batch
wins admission, its retained job owns the lane, original lease/store/permit,
capacity and definitive receipt through completion, even if the caller disappears.
File/fsync stalls have no hard wall-clock bound; expiry cannot roll back admitted
work. A final memory check labels an admitted result current or detached without
publishing into a replacement runtime.

Return ordered per-candidate results: ungrounded/rejected, accepted (exact
observation ID and sequence), duplicate (original identity), or the existing
typed record error. Distinguish not-admitted cancellation/expiry/stale-owner from
an admitted batch with partial successes or persistence errors. A persist failure
can leave an unknown durable file outcome; do not claim rollback or blindly retry
an unknown result. The captured adapter does not automatically retry `PersistFailed`:
that error currently conflates counter, append and compaction failures. Existing
legacy retry behavior remains unchanged. A deliberate later replay uses the same
keys and exposes the resulting duplicate/conflict/error receipts.

Only accepted new observations request evaluator scheduling, and scheduling must
recheck the original active workspace/lease/store before touching live state.
Duplicates, revoked permits and detached receipts cannot schedule work for a
replacement. SQL metadata success and evidence success remain separate; no
cross-file transaction, compensating deletion or automatic full-batch retry is
introduced. Evidence failure remains visible and does not impersonate successful
metadata finalization.

## Alternatives and boundaries

- Reopen by path: rejected because it duplicates sequence/cache authority.
- Retain application guards on a blocking thread: rejected because a stalled
  filesystem would still block terminal-critical shared locks.
- Retain only a shared store: insufficient because deletion can finish before
  a queued append and the append can recreate its segment.
- Move observations into SQLite: outside the accepted storage scope.

This proposal adds private in-memory ownership and admission, with no schema,
file-format, HTTP, producer-installation or retention-default change. Legacy JSON
routing stays selected. The initial implementation qualifies this owner and its
captured SQL adapter in fixtures; full runtime final-scan routing is a subsequent
integration gate. Close-first SQL Archive/Delete, restart recovery, migration,
activation and numeric responsiveness qualification remain separate work.

## Acceptance and review

The [implementation plan](../plans/2026-11-04-ending-workflow-evidence.md)
defines race/failure checks. Approval covers the shared file owner, revocable
permit, bounded job admission and captured adapter described above. It does not
approve a live workspace migration or relax any existing capability gate.

## Design review

Fresh-context read-only correctness and builder/completeness reviews found no
actionable issues or unanswered consequential questions. They checked this
proposal against the source owner and accepted storage boundaries; they did not
run race tests or qualify runtime behavior. Self-review made application-wide
capacity, the cancellation/admission claim and no automatic `PersistFailed` retry
explicit. The next gate is owner approval, followed by the plan's real fixtures.
