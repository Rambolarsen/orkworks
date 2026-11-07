---
type: Implementation Plan
title: Owned recommendation removal batches
description: Preserve bounded original leaf bytes and prepare complete graph deletion batches for existing recovery manifests.
tags: [orkworks, taskmaster, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Owned recommendation removal batches

Prerequisite under [#1243](https://github.com/Rambolarsen/orkworks/issues/1243),
after #1482, within the accepted [external deletion contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion).

## Outcome and boundaries

Prepare a complete immutable removal batch from the existing anchored bounded
inventory, retaining the original lease, binding and shared operation permit.
Preserve exact source bytes and observed device/inode identity for each leaf;
normalizing typed records cannot supply lossless recovery backups. No additional
filesystem scan or I/O is allowed during batch preparation.

Reuse one pure graph-closure helper for existing legacy deletion and captured
batch preparation: include parents, members, reverse parent references and
superseded parents until the whole connected deletion closure is selected. Seed
membership uses the existing exact source/evidence/affected/watermark reference
predicate. Do not introduce a different protection or graph policy.

Expose borrowed target views with canonical target/backup basenames, hashes of
original bytes, bytes and captured identity. Prepare both committed=false/true
payloads in the existing version-2 rollup manifest format, with deletion-only
entries, deterministic ID order and one-MiB per-manifest cap. Empty/no-reference
inventories remain valid no-op batches. The existing 1,024-entry, one-MiB record
and 32-MiB aggregate encoded-byte bounds remain unchanged. Retained raw bytes add
bounded transient memory; this is not a persistent record/history cache or a
measured RSS claim.

No journal or backup writes, rename/unlink/recovery, SQL progress, application
wiring, source/tombstone freshness grant, protection-policy change, final deletion,
migration or activation. Copies of views/manifest data remain non-authoritative
snapshots. The future writer must recheck current leaf/lease identities and
qualify bounded durable publication/recovery before using the batch. Stale producer
revalidation/revocation and exact SQL job identity remain separate gates.

## Delivery and checks

- [x] TDD exact raw-byte/hash/identity preservation, compatible deletion manifests,
  complete parent/member/superseded closure, all reference channels, no-op and
  deterministic batches, owned guard/lease lifetime and preparation without I/O.
- [x] Retain captured raw leaf proofs in cleanup inventory; add owned preparation
  in a focused cleanup child module; share existing graph closure without changing
  legacy mutation/publication/recovery behavior.
- [x] Run all store regressions, full verifier and current-product medium manual
  review; update canonical architecture/validation.
- [ ] Record PR CI and owned cleanup at terminal state.

## Plan review

Ready under accepted ownership/recovery architecture. Source inspection found
preflight discarded raw bytes after strict parsing; serializing typed records
would drop unknown members and original formatting. Existing deletion expands
through forward and reverse rollup references including superseded parents.
Version-2 manifests already encode deletion through old backup/hash plus absent
new/staged values; reuse that contract without a new journal schema or activation.
The batch is not a destructive capability and cannot establish current external
file/source freshness from a past inode observation.

Dependencies3(inventory/graph/manifest validators), blast radius2(shared pure
closure, unwired preparation), state changes1(read-only), reversibility1(revert),
uncertainty2(raw proof/graph edges), total9. Existing graph cleanup regressions
protect shared semantics; exact bytes, all edge directions, detached lifetime,
and no-I/O preparation fixtures qualify this independently before file effects.

Verification investigation: the initial oversized-manifest fixture assumption was
incorrect; the 1,024-leaf/250-byte-ID payload is 1,045,563 bytes and fits the cap.
The corrected regression qualifies supported acceptance without a product change.
An existing delivery fixture sampled legitimate post-ACK blocking bookkeeping;
it now awaits the real input gate before probing settlement locks. All 76 store
tests pass after this test-only phase handshake, and both independent medium
delta reviews are clear on product SHA256
`dd39e9c40905ea04947d8bbe092192972e648423bf43dd9354727958bc39ede3`.
An unrelated unrestricted full-run NeedsTrust/Unknown assertion poisoned shared
fixture mutexes; its isolated run passed, and classification evidence is retained
under [#957](https://github.com/Rambolarsen/orkworks/issues/957#issuecomment-6703936588).
The unrestricted run is failed; the four-thread run separately failed one observer setup with Conflict;
[its isolated pass and exact failure](https://github.com/Rambolarsen/orkworks/issues/957#issuecomment-6704000823)
are retained. Final verification uses serial Rust concurrency without skipping
tests. No trust/activation production logic changed.

Final full verifier passed with `RUST_TEST_THREADS=1`: 3,112 Rust unit tests
(18 existing ignored), seven integration, 31 reporter, one Electron ABI proof,
desktop type-check/build and 1,697 tests (three existing skipped), docs 1,202
checks/build and diff/doc/worktree checks. No additional skips or filters were introduced for the
serial result. The failed parallel runs remain failed, with causes still tracked
under #957 except the corrected owner-fixture phase. Public-doc reminder reviewed:
this source-only preparation makes no installer/user-flow or activation claim.
The precommit worktree warning reflects a new branch at the main base with owned
uncommitted changes; it is not a merged PR and cannot authorize cleanup.
