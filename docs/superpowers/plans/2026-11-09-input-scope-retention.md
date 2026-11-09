---
type: Implementation Plan
title: Original input scope retention
description: Approved bounded typed capture and retained scope attachment with actual offline lease and writer lifetime verification.
tags: [orkworks, terminal, ownership, qualification]
status: stable
workflow_status: locally-verified
---

# Original input scope retention

Tracking: [#1630](https://github.com/Rambolarsen/orkworks/issues/1630), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Spec: [approved scope design](../specs/2026-11-09-original-input-scope-design.md).
After #1629 the owner answered “go” to the explicit bounded implementation
question. This task records acceptance, plan and code together. No repeat
approval is needed for unchanged scope.

## Outcome and boundaries

Pass: the actual submission Entry/Core retains the original strong workspace
lease and runtime/channel intent across caller/workspace/owner loss while actual
reporters exist. Failed capture/admission preserves owned inputs. Known-safe
retirement releases the attachment outside locks. Existing production routes
remain unreported and never construct this capture/Owner.

No command routing, actual preparation/bookkeeping or graph consumer, production
coordinator, native RootKey/module registration, profile/default/deadline policy,
maintenance/readiness/restore or live/native/model/resource experiment.

## Decisions and ownership

One inline implementer owns all changed files. Add immutable scope and finite
reservation types in `runtime/input_submission/scope.rs`; keep application locks
and actual target validation in `session_application/input_submission_scope.rs`.
Expose only crate-private access needed by that adapter. Entry takes an optional
owned scope, reusing Core lifetime and unique reporters, not a second result map.
Unbound admission remains for existing component fixtures only.

A finite scope budget uses the existing Tokio owned semaphore permit, one permit
per capture. Each permit conservatively reserves a fixed explicit byte allowance
and one lease/sender reference; constructor checks scope count, total bytes and
reference caps with checked arithmetic. The permit follows capture, rejection
and Entry; no budget renewal on transfer. This includes outstanding rejected
captures without a growing registry or custom reference counter. Capture checks
actual owned path/String capacities and fixed scope representation; no lossy
path conversion. Allocator/Arc overhead and physical fit remain unqualified as
with the existing candidate; no shipping default is introduced.

## Delivery and checks

- [x] TDD private finite reservation/capture and application current-target
  refusal: real temporary lease, existing runtime/channel fixture types, exact
  lease/session/runtime/channel comparison, missing lease/runtime and stale
  workspace/channel refusal, input ownership and capacity/overflow boundaries.
- [x] TDD bound Entry admission through the shared existing transition; refusal
  returns scope intact, same Context cannot replace a protected binding. Scope
  stays with Core after observers/caller and selected workspace disappear.
  Retirement moves the removed Entry out of the lock before resource Drop.
- [x] TDD actual optional writer/flush/join with bound Entry, dropping coordinator
  and waiter while the existing physical reporters retain the original lease.
  Use existing failure-safe gates; preparation/bookkeeping stay pending, no fake
  whole-job completion. Known-safe component retirement is tested separately.
- [ ] Run scoped owner/application/writer cases and `bash scripts/verify-repo.sh`,
  close-out docs/worktree checks, current-diff manual code review, required CI,
  complete comment inventory, Taskmaster tie-off if applicable, merge and cleanup.

Expected red: missing new capture/bound-admission APIs, then behavioral failure
when capture/attachment is absent. Expected green: all scoped and full repository
checks pass with production start sites unchanged. No test-only physical receipt
or new worker/notification path.

## Plan review

Self-reviewed once against all approved handoff requirements. Capture → record
→ existing reporter/worker is sequential and has one writer; no implementer fanout.
Finite scope reservations must outlive failure returns; Entry removal must not
Drop the last resource under its mutex. Tests cover both. Scope permit cannot
reference Core (which would cycle); its existing semaphore has independent
bounded ownership and no submission facts.

Risk ratings: dependencies 2 (existing lease/channel/semaphore), blast radius 2
(private unwired seam), state 2 (additive record field), reversibility 1 (no
production callers), uncertainty 2 (bounded actual offline lifetime evidence):
9/25. Production/physical rollback remains outside this slice and unqualified.

## Execution evidence

Initial red build failed on the absent ScopeBudget/reservation/capture and
admit_bound APIs, as expected. Initial two adapter cases and 25 owner cases
passed after implementation. Full verification first stopped at the host Node22
fs-ext prerequisite; rebuilding that existing dependency under required Node
resolved it without source changes.

A new non-UTF8-path test initially failed before capture: macOS rejected creating
the invalid filename with OS92 Illegal byte sequence. The exact compiled test
reproduced it. The fixture now holds a real lease at a valid temporary root and
passes non-UTF8 encoded intent directly to the storage primitive, checking exact
bytes/matching and actual lease lifetime without claiming filesystem validation.
Production code did not change for that fixture correction.

Fresh /code-review medium approved the seven-file current source manifest with
no findings. Medium is justified by >500 code/test lines and coupled capture/
Entry/reporter lifetime across existing application/runtime modules. The review
was read-only and ran no tests; a focused test-only refresh verified the corrected
fixture and all seven hashes. Production coordinator/native RootKey/profile/
shutdown behavior was explicitly declined as gated, not silently qualified.
The first full Rust run completed with 3,441 passed, 18 existing ignored and
only that new fixture failure. The corrected four scope cases then passed,
including encoded-path storage and exact-capacity/one-byte-below refusal.
The next full run passed all new scope cases but reported unchanged
`probe_process::tests::unix::prefix_capture_preserves_version_policy_and_rejects_help_overflow`.
Its exact compiled test passed isolated (0.04s). The full run had 3,441 pass,
18 existing ignored and only this failure: CleanupUnconfirmed from the silent
exit9 cleanup at probe_process/tests.rs:114. Exact cause remains unproven.
All 50 related cases passed, including all nine new ones. The unchanged full
verification command is rerunning with RUST_TEST_THREADS=4 to bound local test
concurrency; no test is skipped, relaxed or changed for the probe failure.
No probe source, assertion or sandbox policy was changed.


The four-thread full Rust suite passed: 3,442 unit tests, 18 existing ignored,
all integration groups (1, 1, 4, 1) and 31 OpenCode reporter tests. All 50 related
owner/scope/writer cases passed (nine new; encoded-path case Unix-only). No
production bound capture/Owner constructor or reported writer start was added.
The complete `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` passed, including
1,720 desktop tests (three existing skips), desktop/Electron build, all 1,204 docs
tests, OKF/index/site build, whitespace and doc/worktree checks. Informational
README/docs-user/home currency warnings need no edit: this private unwired slice changes no
public behavior, milestone completion or installer claim; canonical architecture,
ADR, scope and qualification plan were updated. Existing VitePress chunk/NO_COLOR
and unchanged Rust warnings are unrelated to the diff. Required CI, final PR
inventory and cleanup still gate merge.
