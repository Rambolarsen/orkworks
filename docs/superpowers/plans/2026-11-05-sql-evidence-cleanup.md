---
type: Implementation Plan
title: Captured SQL workflow evidence cleanup
description: Remove workflow observations through the shared revocation owner before durable deletion progress.
tags: [orkworks, sqlite, sessions, workflow, storage]
status: stable
workflow_status: implementation-complete
---

# Captured SQL workflow evidence cleanup

Tracking [#1243](https://github.com/Rambolarsen/orkworks/issues/1243), following
#1427 under the accepted [storage design](../specs/2026-11-01-sqlite-session-storage-design.md)
and approved [shared evidence owner](../specs/2026-11-04-ending-workflow-evidence.md).

## Outcome and boundaries

Add one private captured Delete stage for workflow observations. Revoke every
captured writer for the target session under its original shared store mutex,
remove only that segment, synchronize deletion, clear its caches and then set
SQL progress bit `0x02` with `complete=false`. Retain the tombstone/job. Production
remains JSON; no activation, automatic recovery scheduling, retention, recommendation,
image/settings cleanup or final row removal is included.

## Decisions and investigated risks

- Reuse the existing captured deletion runner for lane, physical closure,
  original workspace/lease/reader/runtime checks, organization reservation,
  bounded file capacity and cancellation/expiry admission. Add a private stage
  enum rather than duplicating concurrency machinery. Existing replay remains
  bit `0x01`; both stages preserve all other progress bits.
- Capture the cheap shared workflow store handle with the original memory
  snapshot, never reopen it by path. Evidence cleanup requires the identical
  store instance at revalidation/admission and publication. A replaced store
  fails before admission; detached admitted work finishes on its original owner.
- Admission remains the runner's atomic claim under application guards. The
  admitted job retains lane, reservation, lease and capacity while waiting for
  the evidence mutex, removing files and persisting progress. Application guards
  never wait for the evidence mutex or filesystem. Pre-admission cancellation
  causes no evidence revocation/removal. After admission cancellation cannot
  promise no effects.
- Add a narrow durable method in `workflow_observations/deletion.rs`. Extract
  shared permit-revocation/cache-forgetting helpers so legacy deletion behavior
  is unchanged. The durable method validates exact ASCII session IDs (1–256),
  holds the existing store mutex, revokes before any unlink attempt, and clears
  cached observations/idempotency/diagnostics only after synchronization.
- On Unix retain original workspace/workflow directory handles with no-follow
  from shared-store opening,
  unlink the validated exact NDJSON basename with flags0, and sync directory
  then workspace. Query a positive live NAME_MAX before resolving a basename;
  skip impossible exact names without truncating aliases. Missing files/dir
  still require parent synchronization; retry repeats sync after prior unlink.
  Anchor-open errors disable only the new durable path, preserving legacy
  store-open behavior. Directory/root renames never redirect cleanup to a
  replacement; each shared store retains two original directory FDs.
  Non-Unix returns Unsupported before effects. Sequence and unrelated sessions
  are preserved; no broad hardware durability qualification is claimed.
- Existing deletion lacked directory sync (verified in source), so it cannot
  authorize SQL progress unchanged. A file or sync failure keeps the job and
  revocations; SQL failure exposes completed files separately for safe retry.
  Old permits never regain authority. SQL and evidence are separate outcomes.

## Delivery and verification

- [x] TDD durable owner removal/reopen, revocation on failure, cache/sequence and
  unrelated-session preservation, absent retry/sync failure, no-follow links,
  invalid/max IDs, replaced ordinary directories/root paths and unsupported-platform
  fail-closed behavior.
- [x] Extend captured deletion with original evidence owner and shared stage
  runner. TDD exact progress/reopen, replay preservation, stale store/workspace,
  cancellation/expiry before admission, retained caller loss/detachment and
  file/SQL failures. Reuse replay tests for shared runner/resource boundaries.
- [x] Stall the evidence mutex on one Tokio worker; prove application guards
  stay free and independent input is acknowledged. Run scoped evidence/replay
  suites, full repository verification and current-diff review.
- [ ] Record PR/CI, recommendation tie-off and owned-worktree cleanup on #1243.

## Plan review

Ready within unchanged accepted authority. Least certain boundary was durable
file acknowledgement; inspection confirmed missing directory sync, addressed by
the owner-specific variant rather than changing legacy behavior. Delayed writer
recreation is already constrained by the approved shared-mutex permit fence;
new tests exercise that fence through SQL cleanup. Dependencies3 (existing
application/SQL/evidence owners), blast radius2 (source-only stage and shared
runner), state changes2 (fixtures only), reversibility1 (no live selection),
uncertainty2 (controlled mutex/file/progress failures), total10. Power-loss and
platform/filesystem qualification remain activation gates. No new schema,
capability, content authority or architecture approval is required.

Final local verification passed: nine durable-owner tests, seven captured SQL
cleanup tests and full repository verification (3,024 Rust unit tests, seven
integration tests, 31 OpenCode reporter tests, Electron ABI proof, desktop
type-check/build and 1,691 passing tests, plus docs validation/build). The
manual medium review finding was reproduced with two failing replacement-path
regressions, fixed by original directory handles and re-reviewed clear. PR/CI
and owned cleanup evidence is recorded on the PR and tracking issue.
