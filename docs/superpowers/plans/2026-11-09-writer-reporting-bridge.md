---
type: Implementation Plan
title: Retained writer reporting bridge implementation
description: Fixed byte checkpoints and unique writer/return reporting through the existing task with production reporting absent.
tags: [orkworks, terminal, ownership, qualification]
status: stable
workflow_status: locally-verified
---

# Retained writer reporting bridge implementation

Tracking: [#1620](https://github.com/Rambolarsen/orkworks/issues/1620), parent #1250.
Spec: [accepted checkpoint/transfer design](../specs/2026-11-09-retained-writer-transfer-design.md).
After #1616 the owner answered “keep going” to the explicit implementation
question. Record acceptance and code together; execute inline without another
acceptance-only task or repeat execution approval. Reuse the design reviews.

Pass: real Owner checkpoints and the actual writer/task/shared join preserve
known bytes and distinct phase facts, including failure/loss. All production
starts remain unreported, with no production Owner factory or profile claim.

## Interfaces and ownership decisions

- `input_submission::Entry` gains one optional byte checkpoint. Snapshot derives
  strongest known provisional input from it until immutable terminal input is
  installed. `Reporter::checkpoint` accepts only monotonic writer-phase progress
  before terminal/phase settlement; invalid/late/wrong-role reports protect.
  Terminal consistency rejects erasing a prefix or downgrading known full bytes,
  preserving prior evidence and existing idempotent terminal reports.
- `Reporter::validate_pair` borrows the unique writer/return roles and checks
  exact Core/key/serial, both phases pending, no input/checkpoint and original
  length. Validation does not settle or mutate either record. Invalid assembly
  returns a fixed reason; caller still owns the original roles/resources.
- Private `ReportedPtyInput` owns all reported-start inputs: original writer,
  String, existing ack/stop handles and two reporters (plus cfg(test) post-ack
  callback when applicable). `PendingPtyWrite::start_reported` validates before
  dispatch and returns the entire input bundle plus fixed reason on refusal.
  No payload copy, uncharged replacement task, reporter clone or hidden drop.
- Existing production `start` uses reporting None. `write_pty_input` receives an
  optional borrowed writer reporter and returns its direct outcome plus fixed
  optional reporting error. Checkpoints follow each positive count and empty
  zero before flush. Failed publication stops further write/flush in this
  optional path, keeping full/Unknown or truthful partial input and separate
  error. Normal mode behavior and legacy projection remain unchanged.
- Worker owns the writer reporter: terminal fact and actual writer-phase result
  precede ack. Full/flush failure and partial/error settle writer Failed, no-effect
  settles NoEffect, full success Completed; panic drops Lost. Preserve first
  reporting error without retrying or changing byte truth.
- PendingPtyWrite owns optional return reporter. Shared async `finish_join`
  awaits its own task and takes/settles that reporter before returning to the
  driver, which can then clear pending safely. No caller-supplied join result
  can fabricate completion. Actual success settles return Completed; JoinError
  settles Failed; ordinary early Drop leaves Lost.
  Return-report failure accompanies a successful direct outcome as fixed status;
  on JoinError it is logged as a fixed diagnostic and the original JoinError is
  preserved. Unavailable/poisoned records cannot claim successful settlement.

## One task, meaningful checks

- [x] TDD actual candidate transitions first: missing checkpoint/pair operation
  red, then monotonic/provisional/full snapshots, immutable terminal consistency,
  wrong/late/regressing/oversized/empty progress, latched conflict/phase protection
  and actual representation/capacity checks green. Reuse the direct std-only
  harness for fast cycles, then Cargo/full verification.
- [x] TDD actual writer bridge next: actual Owner fixtures retain preparation/
  bookkeeping pending; direct worker checkpoints visible while flush held;
  full/prefix/first-call panic and actual shared join finalization; dropped/lost
  ack and post-ack panic; early Pending/observer Drop and late writer result;
  failed checkpoint stops later native calls with truthful bytes/error status;
  invalid mixed Core/key/role/phase/length input returns every consumed resource
  before a worker/native call. Use deterministic release-on-drop gates and
  fixture watchdogs, no sleeps or fabricated physical reporter completion.
- [x] Implement minimal optional bridge in session_runtime and scoped tests.
  No RuntimeCommand/HTTP/application/startup/graph/storage/default change.
  Verify every production start is None and no Owner construction outside tests.
- [x] Run scoped checks, formatting/build, full `scripts/verify-repo.sh`, docs
  and close-out checks; complete fresh current-code review before PR handoff.

Required CI and complete comment inventory still gate maintainer merge/cleanup;
those forge results are recorded in the PR/issue, not inferred locally.

## Review focus and risk

Focus: checkpoint truth versus immutable terminal conflicts; refusal retains
resources; panic preserves only retained facts; ack differs from return; dropped
return/observer does not release worker/record; unavailable reporting fails closed.
The checks above exercise each through actual module/task interfaces. Native
return-to-checkpoint panic can still lose the newest count. Actual root/resource/
phase profile, native/physical fit, maintenance and restore remain unqualified.

Dependencies 2 (existing record and worker/join); blast radius 2 (optional path,
production None); state changes 2 (fixed in-memory field, no durable schema);
reversibility 1 (local revert); uncertainty 3 (new lifecycle transfer covered by
deterministic tests). Total 10. Plan self-review: Ready for this approved slice;
no new authority/default, unsupported resolution or hidden phase completion.
Fresh current-diff code review remains required. Execution evidence follows here.


## Implementation decisions and initial evidence

Nineteen actual owner tests passed after the initial missing-operation red.
A separate guard regression failed (Protected expected, Ok observed), then
preflight was strengthened to refuse latched protection or preparation NoEffect.
Validation preserves all inputs/state; this prevents starting the optional path
against already inconsistent evidence, without changing reporting of an already
running original worker.

The join interface is async and awaits its own task instead of accepting a
caller-provided Result. This implements the same approved ownership ordering
with a stronger interface: return publication happens before the driver can
clear pending. Revised interface self-review: Ready, same scope/checks.


## Checkpoint guard and bridge evidence

All 21 owner tests passed in the actual std-only module harness. Two additional
guard cases failed before their fixes: provisional byte truth now rejects later
NoEffect phase claims, and a checkpoint without terminal input cannot retire
even if a component caller marks all phases complete. Those misuse tests are
not physical producer evidence.

Nine actual Owner/worker/join tests failed initially for missing reported-start,
join and reporting-status operations (after correcting the test module path),
then passed. Empty/full/prefix/first-call panic, flush failure, lost/post-ack
notification, pending/observer loss, reporting poison and refused input ownership
are exercised without completing preparation/bookkeeping. Existing direct writer
tests also passed in the runtime run. The runtime filter ended with 832 passed,
one existing SQL observer setup Conflict and four ignored; full verification is
not yet claimed. The failure is in unchanged SQL observer source before its
stale-response assertion. An isolated check and full verification follow.


## Current-code review and isolated failure check

Fresh diff-scoped `/code-review medium` completed with APPROVE and no findings.
Medium is justified by new checkpoint state and task/Drop ownership across the
two inseparable modules and their tests. The reviewer verified the exact five-
file SHA-256 manifest, including the new nested test file, and did not contend
with full verification. No production reporting activation/authority was found.
Later code changes require refreshed review.

The unchanged SQL observer setup test passed in isolation (one test, 0.03s).
It failed at the first begin_native_observation unwrap before its stale-response
assertion in the broader run. That source has no diff; existing 300 ms freshness
across SQL work suggests timing sensitivity but the log does not prove the exact
cause. No assertion or unrelated code was changed. Complete repository
verification remains the completion gate and is running on the reviewed code.


## Final local verification

`bash scripts/verify-repo.sh` passed on the unchanged reviewed code: prerequisites,
Rust formatting/build, 3,414 unit tests (18 existing ignored, zero failures) and
all integration binaries; 31 OpenCode tests; desktop type-check/Electron ABI
check/build; 1,720 desktop tests (3 existing skipped, zero failures); 1,204 docs
tests/OKF/index/site build; whitespace/doc-check/worktree-check. All 41 related
owner/writer cases passed, including the 17 new tests. No ignored/skipped case
was added. The earlier SQL observer setup test also passed in this parallel run;
no unrelated code/assertion was changed and exact earlier cause stays unproven.

The user/home/README currency nudges need no edit: current public/UI behavior,
installer availability and advertised milestones do not change. Canonical source
architecture records the optional bridge and its limits. Existing SQL source-
only unused/dead-code and build chunk/NO_COLOR notices are outside this diff.
The std-only micro-harness reports its unused cross-module test poison helper;
full Cargo tests use it, and it is absent from production.

Actual local checkpoint/worker/return evidence is established, with no production
reporting/Owner factory, application/root/resource/profile integration, qualified
physical/deadline/native fit, Q/readiness or backup/restore claim. Newest native
return-to-checkpoint information and lost physical ownership can remain unknown.
