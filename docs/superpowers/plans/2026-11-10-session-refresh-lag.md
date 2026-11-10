---
type: Implementation Plan
title: Responsive session refresh inspection
description: Deliver session-location and document inspection outside the terminal input fence with stale-result rejection.
tags: [sessions, terminal, performance, plan]
status: draft
workflow_status: implemented
---

# Responsive session refresh inspection

**Goal:** Terminal input delivery and bookkeeping proceed while session refresh
Git/document inspection is paused.

**Design:** [Capture, inspect and recheck](../specs/2026-11-10-session-refresh-lag-design.md).
**Tracking:** [#1222](https://github.com/Rambolarsen/orkworks/issues/1222).
**Execution:** Inline in the owned `session-refresh-lag` worktree; one writer.

## Delivery

- [x] Add deterministic failing regressions in `session_projection.rs` and its
  existing test seams. Stall location validation and document validation, then
  require real input delivery/bookkeeping before inspection resumes. Exercise
  current full-list callers, not just a standalone helper.
- [x] Capture location inputs separately from repository resolution in
  `session_location.rs` and `session_projection.rs`. Reuse current live/durable
  authority and workspace identity. Compare location-only inputs and existing runtime identities; a semantically
  identical restored location needs no new revision protocol. Keep metadata/organization
  ordering and unchanged-output liveness.
- [x] Separate document preparation from publication in `branch_documents.rs`
  and `branch_documents_store.rs` as needed. Preserve store generation fencing,
  per-branch serialization, pin/provenance precedence and no-follow validation.
  Recheck location/workspace inputs under the input fence; repository/document
  inspection and waits stay outside. Keep final complete-list validation.
- [x] Add stale-work regressions for workspace replacement, deletion, resume,
  rebinding, branch changes, missing documents and newer reports/pins.
  Cover shared-root inspection coalescing and fresh final validation. Verify
  failed validation uses unavailable rather than authoritative removals.
- [x] Add/run an isolated large-history benchmark reporting list duration,
  inspection counts and residual fence wait/hold. Do not launch a second coding
  harness, replace production metadata, or restart the running app.
- [x] Record the accepted lock-scope follow-up in the appropriate ADR and
  canonical runtime architecture documentation. Preserve the earlier accepted
  design's history and distinguish source behavior from live qualification.
- [ ] Run targeted regressions, Rust build/test/fmt, then
  `bash scripts/verify-repo.sh`, docs checks and close-out hooks. Complete the
  current-diff code-review gate at the appropriate concurrency risk level,
  open the PR, follow required checks/review, merge through the authorized
  maintainer path and clean up only this owned worktree.

## Review focus

The review must preserve runtime identity without coupling refresh liveness to
every output event; generation changes during prepare/publication; external branch
changes during inspection; locks hidden inside document validation/reconciliation;
and absent/unavailable bound worktrees. Every case has a regression above.

## Risk assessment

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 1 | One Rust sidecar; existing HTTP/stream callers retained |
| Blast radius | 4 | Shared session refresh and terminal input fence |
| State changes | 2 | Existing branch-selection publication; no schema change |
| Reversibility | 1 | Code revert; owner-controlled restart |
| Uncertainty | 3 | Measured hotspot; exact capture/revalidation seam needs tests |

Total: 11/25. Review disposition: **Ready** for implementation after one read-only
builder check. Findings resolved: retain existing runtime identity and compare
location inputs instead of adding a global revision; prepare branch selection
under its existing lock, then use nonblocking lock admission and exact store
generation at fenced publication. The design's output and safety contracts
are the acceptance criteria. No changes to input ownership or
storage activation are authorized by this plan.

## Execution evidence

The real-input regression
`stalled_document_validation_does_not_block_real_input_delivery_or_bookkeeping`
failed on the former fenced document path with “document validation must not
block terminal input delivery”; after the split it passed, including acknowledged
PTY delivery and completed bookkeeping while validation remained paused.
The fresh builder check confirmed the lock/I/O paths and removed the unnecessary
proposal for a new location-revision system. Remaining regression and full-suite
results will be recorded before handoff.

Session projection group: 35 passed, three manual benchmarks ignored. The
explicit isolated 800-row bound-worktree benchmark passed: refresh passes
546.705, 346.803 and 363.782 ms; an observer acquired the input fence 549 times
with a maximum observed wait of 105.005 ms. This is a synthetic shared-root
measurement under concurrent verification, not live keystroke or scrolling
qualification, and the residual metadata/organization fence is measurable.

Independent `/code-review medium` is clear for code diff SHA-256
`9e32000c73f6f130c854fcda17808b3de82cb0231d8ab504e0bb7f0a9e0ef348`.
Medium effort was selected for shared input concurrency and the >500-line
code diff, including tests. The reviewer traced location/runtime/workspace
validation, root reuse, branch-lock admission and store-generation checks.

Docs checks passed 1,204 tests. Documentation-currency warnings were reviewed:
public onboarding and README claims do not need edits for this internal
performance repair; no installer, UI flow, product milestone or architecture
stack claim changed. Canonical architecture and both applicable ADRs carry the
source lock-scope amendment. Source verification does not qualify live adoption.

The initial default-concurrency full verification stopped at Rust tests:
3,475 passed, one failed and 19 ignored. The failure was
`runtime::codex_approval_application::sql_observer_tests::sql_observer_pipeline_preserves_new_work_recorded_during_rpc`,
which returned `Conflict` at the first observation setup (line 391). That test
passed in a fresh exact-name invocation (one test, 0.01 seconds). Its source and
SQL implementation are unchanged by this repair. The parallel failure is not
claimed fixed or conclusively classified; a full four-thread verification run
is required before PR readiness. The failed log is retained at
`/private/tmp/orkworks-session-refresh-verify.log`.

After rebasing onto current main, the four-thread full run also stopped at Rust:
3,475 passed, one failed and 19 ignored. This time the failing test was
`image_context::tests::contended_cleanup_preserves_metadata_and_can_be_retried`:
the immediate post-drop cleanup retry returned `Resource temporarily unavailable
(os error 35)` at line 407. Its exact isolated rerun passed (one test, 0.04
seconds). The image/retention source is unchanged by this repair. No claim is
made about the underlying cause of this intermittent contention. The log is
`/private/tmp/orkworks-session-refresh-verify-bounded.log`. A complete serial
verification is running; both earlier failing tests have passed in that run.

Draft PR: [#1663](https://github.com/Rambolarsen/orkworks/pull/1663).
The source remains the reviewed patch after rebase. The PR remains draft until
full verification has completed; the initial parallel results remain evidence,
not superseded by isolated passes or a later serial result.

## Final source verification

Complete `RUST_TEST_THREADS=1 bash scripts/verify-repo.sh` passed on the rebased
source: 3,476 Rust unit tests with 19 existing/manual ignored, all applicable
Rust integrations, 31 OpenCode reporter tests, desktop type-check, native ABI
repair, 1,721 desktop tests passed (1,724 total with three skips), desktop build,
1,204 docs checks and site build, diff check and both close-out hooks. Log:
`/private/tmp/orkworks-session-refresh-verify-serial.log`.
The source repair does not claim to fix the two separate parallel-suite failures
recorded above. Required hosted Rust CI also passed with its four-thread setting.
The reviewed code diff is unchanged; only this execution record follows it.
PR checks, current-head review inventory and guarded cleanup remain the owning
session's PR lifecycle work. Live adoption remains owner-controlled under #1222.
