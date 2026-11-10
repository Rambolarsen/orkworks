---
type: Implementation Plan
title: Confirm a settling probe leader after Darwin EPERM
description: Reproduced cleanup race and a proposed bounded confirmation phase retaining original-child ownership and group proof.
tags: [harnesses, probes, macos, safety, testing]
status: draft
workflow_status: implementation-verification
---

# Confirm a settling probe leader after Darwin EPERM

Outcome: avoid discarding valid discovery output when Darwin rejects a group
signal before the original child's exit becomes observable. Tracking:
[#957](https://github.com/Rambolarsen/orkworks/issues/957). This is a proposed
cleanup lifecycle correction, approved by the owner on 2026-11-10 ("go ahead"
after the concrete PR #1661 design request). This authorizes its bounded
implementation and synthetic fixtures; no installed-tool or model probe is needed.

## Reproduced evidence

Investigation base: `cd6415f0384561d52b952a4c7d20fdd3c48275a7`, macOS arm64.
The unchanged exact image-capability matrix passed once (one test). Two bounded
100-iteration diagnostic runs subsequently failed, each stopping on its first
unexpected assertion:

| Run | Observed result |
| --- | --- |
| Typed probe errors only | Zero-based iteration 10, stable 1.0.0 with valid image help and exit 0: false versus expected true; version probe returned CleanupUnconfirmed |
| Native cleanup trace | Zero-based iteration 17, stable 0.163.0 with valid image help and exit 0: false versus expected true; version probe returned CleanupUnconfirmed |

The second run captured this ordering on the same original owned Child while
its owner mutex was held:

1. Pre-signal WNOWAIT status: `None`.
2. Group SIGKILL: return -1, errno 1 (EPERM).
3. Existing `only_exited_leader` exit check: `Ok(None)`; membership was not queried.
4. A diagnostic-only following exit check: `Ok(Some(true))`.

This confirms an exit-observation race in the current cleanup path. The
[#1643 correction](https://github.com/Rambolarsen/orkworks/pull/1643) covers a
leader already observable as exited at the post-signal check; this reproduction
crosses that later check. It does not identify the exact cause of the historical
#1658 failure, which retained no typed probe error.

Only generated shell scripts were executed; no installed coding tool, model,
credentials, session state or integration configuration was used. Diagnostics
printed fixed errors/native status and fixture-owned PIDs, not command output.
Temporary instrumentation and the repetition loop were removed. Local logs:
`/private/tmp/orkworks-image-957-matrix-diagnostic.log` and
`/private/tmp/orkworks-image-957-cleanup-diagnostic.log`. These are ephemeral local
evidence, not CI artifacts. An attempted independent Python reproduction stopped
before spawn because the installed Python lacks Popen's process_group argument;
it supplies no additional native evidence.

## Proposed decision

Retain the [approved ownership contract](../specs/2026-11-07-probe-process-ownership-design.md)
and [ADR 0072](../../adr/0072-codex-session-process-isolation.md#amendment--2026-11-07-own-short-discovery-probe-processes).
Add an explicit private pending-confirmation state for Darwin EPERM when the
original child's non-consuming exit check still returns None. Pending is never
successful cleanup or capability evidence.

- Record that the original group signal was attempted exactly once. Do not send
  another signal from confirmation, reaping, cancellation or shutdown.
- While the caller remains active, asynchronously observe that same original
  Child, at the existing 10 ms polling interval, releasing the owner mutex before
  every await. Do not wait for voluntary exit before the initial group signal.
- Use one settlement endpoint: min(operation deadline, cleanup start + one
  second), shared by exit confirmation and final reaping. Do not restart or add
  a window. Native syscalls retain the contract's cooperative deadline limit.
- Once exited, apply the existing bounded native group-membership proof. A live,
  unknown, truncated or foreign-member result is never accepted. ECHILD loses
  authority and forbids later numeric signaling. Acceptance still requires the
  original child reaped within budget and the original capture/exit policy met.
- Keep the original Child and its admission permit throughout confirmation.
  The reaper must not consume exit status while a caller's confirmation is
  pending. Each state transition and status/reaping action uses the same owner
  mutex, with no lock held across async polling.
- On caller drop, timeout or service close, finalize pending confirmation as
  unconfirmed and transfer only exact-child reaping to the existing service.
  The dropped future need not be polled again; no late success is published.
  A non-reapable child continues retaining its existing permit.
- Preserve Linux/Windows behavior, capture limits, three-second caller deadlines,
  64-owner admission, caller environments, error mappings and provider policy.

This selects an internal lifecycle change, not unconditional EPERM acceptance,
a probe retry, a timeout increase or a new native identity source.

## Delivery and checks after approval

One owner implements `crates/orkworksd/src/probe_process.rs` and its existing
`probe_process/tests.rs` fixtures. Keep caller parsing and the image matrix
unchanged. Record the approved confirmation detail in the canonical probe
contract and ADR amendment before dependent implementation.

- [ ] Add a deterministic failing regression around the signal/status boundary:
      make the first post-EPERM status observation pending, then expose the
      same unreaped child's real exit. Assert valid captured output, one signal,
      original-child identity, final reaping and permit release.
- [ ] Implement bounded asynchronous confirmation with the states and deadline
      above. Add denial cases for a remaining live group member, permanently
      pending exit, failed status/membership proof and deadline exhaustion.
- [ ] Exercise drop, shutdown and concurrent reaper sweeps during confirmation;
      verify no late signal, premature reap, successful closed-service result or
      capacity release before reap. Retain unrelated-sentinel coverage.
- [ ] Run the original exact image matrix, a finite 100-iteration fake-tool
      diagnostic matrix, probe-service/consumer groups and a temporary negative
      control removing confirmation. Remove broad diagnostics/repetition before
      the final diff; record failures as failures.
- [ ] Run pinned Node 22 frozen dependency prerequisites, full
      `bash scripts/verify-repo.sh`, clippy and current-diff `/code-review medium`.
      Complete platform CI; distinguish macOS evidence from Linux/Windows checks.
      Keep unrelated #957 failures and historical causes explicitly open.

## Initial investigated risks and plan review

The main uncertainty was whether this was load-induced timeout or cleanup
refusal; native traces resolve it to the latter for the reproduction. Source
inspection also shows that `Owner::reap` currently reaps any cleanup disposition:
a pending state therefore requires explicit reaper exclusion until confirmation
is finalized. A blocking wait under the owner mutex would stall runtime and
shutdown work; asynchronous polling is part of the proposal. No additional
signal or acceptance without original-child exit/group proof is needed.

| Dimension | Rating and reason |
| --- | --- |
| Dependencies | 2: one private runner and its existing callers/reaper |
| Blast radius | 3: shared discovery runner; new transition restricted to Darwin |
| State changes | 1: in-memory invocation state, no durable schema or migration |
| Reversibility | 1: local revert restores conservative rejection; no persisted authority |
| Uncertainty | 3: race reproduced; pending/drop/reaper implementation remains unverified |

Total 10/25. Authored scope/clarity/coverage review: Ready for the owner decision;
Investigate for implementation until approval and failing-first fixtures. No
independent review, full-suite success, runtime fix or historical-cause closure
is claimed by this document. The required owner decision is this bounded
pending-confirmation lifecycle, including shared settlement budget and
cancellation/reaper disposition. Existing ownership and deadline approvals stand.

## Implementation evidence — 2026-11-10

The approved private pending state is implemented in `probe_process.rs`.
Confirmation retains the original child/permit, excludes pending reaper
consumption, and shares one endpoint with reaping. Drop and service close
finalize pending confirmation without another group signal.

The exact new exit-gap regression failed first with CleanupUnconfirmed in the
old cleanup path. Initial implementation testing exposed changed timeout error
precedence in an existing settlement test; that compatibility difference was
corrected. The final probe group passes all 20 tests, including 500 ms operation
deadline versus one-second settlement exhaustion, caller abort/service close,
remaining live members, ECHILD and invalid native query lengths (-1, 3, 12).
The earlier 100 exact fake-CLI matrices all passed (1,000 version/help cases);
no installed coding tool or model was invoked.

Two read-only Luna reviewers inspected the final Rust diff SHA-256
`6b60e5555293416d8f938e9948b7af1f38912a0dc35530a17064f743c749a919`.
Correctness/security review completed at medium effort without findings.
Coverage review found two missing denial cases, now added; a later concern
about timeout mapping was withdrawn after examining capped capture and the
passing fixture. No further changes were requested.

The default-concurrency full verifier failed: 3,473 Rust unit tests passed,
one failed, 18 ignored. The unchanged
`sql_observer_pipeline_preserves_new_work_recorded_during_rpc` expected metadata
revision 4 but observed 3 at `sql_observer_tests.rs:415`. A fresh exact process
passed that one test. No cause, historical linkage or full-run success is
inferred. Final four-thread full verification is in progress; no test, timeout,
exclusion or committed CI concurrency setting changed for that run.

Clippy passes with repository warnings; the runner's manual-is-multiple-of
warning is on its unchanged native byte-alignment expression. Node 22 docs
metadata/index/link/build and all 1,204 documentation tests passed. Final
CI, full-verification and merge dispositions will be recorded on PR #1661.
The authenticated Taskmaster recommendation inventory returned zero records.
