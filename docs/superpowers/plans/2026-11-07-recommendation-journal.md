---
type: Implementation Plan
title: Anchored recommendation deletion journal
description: Publish and recover bounded deletion-only recommendation journals using retained workspace ownership.
tags: [orkworks, taskmaster, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Anchored recommendation deletion journal

Continue [#1243](https://github.com/Rambolarsen/orkworks/issues/1243) after #1490,
within the accepted [external deletion contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion).

## Outcome and decisions

Add source-only, blocking publication and bounded restart recovery for existing
version-2 deletion manifests. Retain the original lease, directory descriptors
and shared operation permit through a result receipt, including failures. Add a
recovery-only binding that bypasses normal legacy startup recovery and does not
expose a partially opened recommendation store.

Revalidate captured bytes and inode identities before publication. Refuse a moved
or replaced canonical metadata root before mutation: the operation registry is
path keyed, so read-only descriptor continuity alone cannot grant destructive
ownership after a rename. All new effects use relative descriptors, no-follow
opens, private exclusive files, identity rechecks, and leaf/empty-directory
removal. This does not provide atomic compare-and-unlink against arbitrary
same-user concurrent filesystem attackers.

Keep backups-before-manifest ordering and the existing manifest.json.tmp name.
Sync backups and ancestors, publish/sync uncommitted and committed manifests,
then unlink verified targets and sync the records directory. Validate the result
graph before cleaning known backups and empty subdirectories; remove the manifest
last and sync every parent. Failed operations retain visible recovery evidence.

Recovery validates all contents before effects. One transaction at a time;
reject aliases, unknown entries, unsupported versions and non-deletion manifests.
Uncommitted/draft backups must be prefixes of unchanged original targets and may
be discarded without rewriting originals. Committed transactions require complete
verified backups until all targets are absent; then interrupted backup cleanup
may finish. Validate reconstructed old and resulting graphs before deletion.

Reuse 1,024 logical records, one MiB per record/manifest, 32 MiB per encoded graph
and backup phase. Permit the additional exact journal directory entry. Bounded
transient copies add memory; no hard I/O deadline or hardware power-loss proof.
macOS/Linux fixtures qualify source behavior; other platforms fail Unsupported.
No production routing, SQL progress, session/job freshness authority, evidence
policy change, producer revocation, retention, activation or latency claim.

## Delivery and verification

- [x] TDD publication, retained receipt ownership, stale identities and no-op.
- [x] TDD restart recovery across drafts, commit, partial unlink and cleanup;
  corruption/unknown data must fail without deleting original or foreign data.
- [x] Qualify every injected filesystem/sync boundary through restart/retry.
- [x] Run store regressions, serial full verifier and current code review;
  document compatibility and remaining SQL composition gates. PR terminal evidence
  records required CI and owned cleanup.

## Plan review

Ready within the accepted deletion architecture. Existing v2 format supports
lossless deletion backups; legacy recovery is path based and unbounded and cannot
be called from this new entry point. Directory identity alone does not unify
path-keyed operation owners after metadata-root or ancestor rename, hence both
canonical-path and physical-directory revalidation before effects.
Dependencies3(shared manifest/graph/lease), blast radius2(unwired implementation),
state changes4(deletion in qualified source path), reversibility4(backups until
commit, intentional committed deletion), uncertainty3(restart and sync boundaries),
total16. Restart fixtures address protocol uncertainty; production activation
remains separately gated.

Manual `/code-review medium` is justified by the destructive protocol and diff
size. Two independent read-only passes covered ownership/correctness and recovery
completeness. Their temporary-manifest findings were reproduced before fixes:
committed journals reject any temp, uncommitted temp must prefix the exact next
canonical payload, and first-manifest temp requires at least one complete backup.
An additional failing regression exposed ancestor rename through a symlink;
the owning session added canonical-path revalidation and reviewed that delta.
Initial manual findings were dispositioned. The first reviewed code artifact SHA256 is
`189994bd478dbc03eccf3b44b48ed7a93a8d6444ed739b96023a9f7527772069`.

## Verification evidence

All 90 store regressions pass, including 14 new journal tests and the existing
read-only rename behavior. The interruption matrix restarts with a new lease
and retries recovery twice after every injected effect/sync boundary. Expected
red failures establish the temporary-data and ancestor-rename corrections.
Earlier full runs passed but their Rust binaries preceded final corrections;
only the final corrected-tree run qualifies final source evidence.

Public-doc currency reminder reviewed: this unwired prerequisite changes no
published installer, onboarding or user-facing flow; canonical architecture and
adapter validation record actual source behavior and remaining gates. CI and
owned worktree cleanup belong in the PR terminal evidence.


At pre-rebase head `0b4d1e5119d9ff350a72b0c8e339f31021c3f1d5`, the final
corrected-tree `bash scripts/verify-repo.sh` passed with
`RUST_TEST_THREADS=1`: 3,131 Rust unit tests (18 existing ignored), seven integration,
31 reporter, one Electron ABI proof, desktop type-check/build and 1,697 tests
(three existing skipped), 1,202 docs tests/build and diff/doc/worktree checks.
No additional skips or filters were introduced. The reviewed source files stayed
unchanged through this run. A local filesystem probe also confirmed case aliases
resolve to the same libc canonical path on the tested macOS volume; this is not
qualification of arbitrary filesystem profiles. Existing bundle-size warnings
concern unchanged desktop assets and do not require changes in this source-only
storage prerequisite.


PR #1502 rebased onto main `6c71d5a51979d29d152ffca65bff4aeb4760e9e5`
after concurrent attention/status refactoring and documentation landed. Its sole
conflict was the plan index; both entries are retained, with this entry placed
among recent plans to avoid competing end-of-file additions. The complete Rust
product diff is byte-identical across the rebase (committed diff SHA256
`d6275c52beb9919b81ecddae0afa7f71920a28962503bb08527af8a6093bf58d`),
so current code review is reused. Post-rebase checks and refreshed current-head
reviews/CI are recorded in the PR.


## PR review cycle 1

Codex findings [4501249745](https://github.com/Rambolarsen/orkworks/pull/1502#discussion_r4501249745)
and [4501249747](https://github.com/Rambolarsen/orkworks/pull/1502#discussion_r4501249747)
were independently reproduced: draft cleanup could lose the complete backup set
needed to validate its remaining temp on retry; retained backup files exhausted
the process descriptor limit before recovery could finish. The corrections sync
validated temp removal before any backup deletion, and retain only bounded
byte/inode observations while reopening and checking individual backup leaves.
The existing schema, graph policy, lease/operation ownership and source-only
selection remain unchanged.

All 93 store tests pass, including interruption/restart during draft recovery
and both publication and 1,024-record partial-publication recovery in an isolated
child with a 64-descriptor soft limit. A post-rebase verifier was deliberately
stopped before these demonstrated corrections; it is not qualifying final
verification. Current-code review and full verification are refreshed for the
new head before merge. The PR lifecycle remains cycle 1 of 3, with its original
two-hour deadline measured from 2026-11-07T10:07:56Z.


Both fresh independent `/code-review medium` passes are clear on the complete
cycle-1 product diff SHA256
`fc915cb3e17a7c74ef3e79d5024c7a3bd16eee374b325875a8f910db10539ef3`.
One pass audited recovery/graph/cleanup completeness; the other audited identity,
lease/permit lifetime and descriptor bounds. The 93-test store result covers both
reproduced Codex findings. Final full verification and refreshed CI/reviews are
recorded at the resulting PR head before merge.
