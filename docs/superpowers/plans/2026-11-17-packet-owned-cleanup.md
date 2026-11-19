---
type: Implementation Plan
title: Preserve packet failures through owned process cleanup
description: Approved offline correction for fast-exit Darwin cleanup, original-child ownership and safe cleanup reporting.
tags: [orchestration, testing, processes, macos, safety]
status: draft
workflow_status: implementation-in-progress
---

# Preserve packet failures through owned process cleanup

Outcome: synthetic packet tests retain their original cap, deadline or exit
failure while cleanup targets only the original owned group and reports its
actual disposition. Tracking: [#1741](https://github.com/Rambolarsen/orkworks/issues/1741).
The owner approved this concrete offline design on 2026-11-18 ("yes, go ahead").
Approval covers implementation and synthetic verification only.
The [packet](../../validation/parent-native-packet.md) has zero live budget;
this change authorizes no native/model/discovery command or new canary attempt.

## Reproduced evidence

Investigation base: `13f517e5` on Darwin arm64, Python 3.9.6. The unchanged
46-test synthetic suite passed in 8.82 seconds. A controlled fake client then
exited after an alive poll but before the cap check. Non-consuming exit
observation confirmed the same original child was still unreaped. The real
TERM group call returned errno 1; the raised cleanup PermissionError replaced
`supervisor-file-cap`. Receipt: started true, exitCode 0, termination steps false.
No released identifier was signaled by the diagnostic wrapper.

An additional owned-child probe confirmed that an unreaped exited leader can
still yield EPERM on this host. Retaining the child alone is insufficient.
The existing [probe cleanup contract](2026-11-10-probe-exit-confirmation.md)
and `probe_process.rs::exited_leader_cleanup` already combine non-consuming
original-child exit with a two-entry Darwin group-membership query. This
proposal adopts that invariant for the packet; the earlier runner approval
was scoped to its own implementation and grants no packet change by itself.

Python's native `os.waitid` wrapper is absent here. A temporary ctypes probe of
libSystem waitid returned pending, then original-child exit 0/17 or termination
15 without consuming status; a later final wait returned 0/17/-15. A following
waitid returned ECHILD. The installed SDK and a compiled layout check agree on
Darwin siginfo size 104, PID offset 12 and status offset 20. A bounded libproc
query returned only the original leader, including after its exit/EPERM.

Evidence stays in `/private/tmp/orkworks-packet-root-1o3mlgav/`: baseline,
controlled-reproduction.json/driver, native-wait-proof.json, membership-proof.json
and SDK layout/compile evidence. The temporary anchor-process experiment also
worked, but adds a process/identity and IPC boundary that this existing proof
pattern avoids. At that investigation checkpoint, no repository executable source or manifest had changed.
These controls establish the current failure mechanism; historical schedules
and native-canary outcomes remain unclassified.

## Proposed design

### Original-child ownership

Keep Popen's existing argv, cwd, environment, stdio and start_new_session behavior.
One private owner is the exclusive waiter for that Popen child. Runtime polling
uses `waitid(P_PID, original PID, WEXITED | WNOHANG | WNOWAIT)`; Popen.poll/wait
must not consume exit status until group cleanup is finalized. Before each group
signal, repeat that original-child proof. ECHILD, invalid/mismatched status or a
reaped owner loses authority: no later numeric PID/PGID signal is permitted.
Reject automatic SIGCHLD reaping before launch; packet callbacks do not reap.

Use Python os.waitid when available. For the existing Darwin Python 3.9 route,
use a small ctypes binding to libSystem waitid with the SDK layout/constants
above, alongside libproc proc_listpids. Unsupported observation/ABI fails before
spawn. Other platforms get no guessed native layout or process-list fallback.
The native entry remains pinned to Darwin arm64; POSIX synthetic coverage also
runs on Linux. No dependency or new helper process is introduced.

### Finite cleanup

Preserve the existing TERM/grace/KILL/reap envelope: at most one TERM, at most
one KILL, at most two seconds for the TERM stage and two seconds for final
confirmation/reaping. Confirmation and final wait share their stage endpoint;
no restarted window, signal retry or additional launch is introduced. These
are cooperative endpoints, matching the existing probe contract: synchronous
native calls and scheduler delays are not preemptible hard wall-clock bounds.
Check the endpoint around each native call and wait; expired confirmation
never becomes accepted cleanup. A hard preemption service is outside scope.

- TERM succeeds: observe exit without reaping during the existing grace stage;
  perform the KILL stage while original-child authority is retained to cover
  remaining group children. Reap only after the final group signal.
- ESRCH is a no-group result, with original-child authority still established.
- On Darwin EPERM, successful cleanup requires both the same original child's
  non-consuming exit and the bounded native query accepting no member other
  than that original leader. Use the existing runner's two-PID buffer contract:
  reject native errors, invalid lengths/alignment, truncation/extra membership,
  mismatched or unknown members. A live leader never qualifies.
- Pending exit confirmation consumes the current stage's remaining time. It
  sends no additional signal. Failed TERM confirmation can proceed only to the
  already bounded KILL stage with retained authority. Failed KILL confirmation
  is unconfirmed cleanup, not success. No group signal follows reaping or loss.
- Reap the original child within the remaining endpoint and retain its observed
  signed exit code. Unknown membership, permission refusal with live/nonleader
  members, status loss, or reap timeout remains explicit unconfirmed cleanup.
  Do not widen permissions or claim OS containment/all-descendant absence.

This adopts the original-child/group-proof pattern; it is not unconditional
EPERM acceptance. Linux permission failures get no Darwin confirmation rule.

### Primary failure and receipts

Retain the original exception object and fixed supervisor check code when a
cap/deadline/callback or nonzero foreground exit also has cleanup failure.
With no primary failure, unconfirmed cleanup is an error. Known foreground
exit status must be checked before cleanup can replace its meaning.

Extend the existing optional process projection with an allowlisted
cleanupDisposition: signaled, exited-leader-confirmed, unconfirmed, or not-started.
An optional cleanupFailure contains only fixed cleanup stage/category and
bounded numeric errno/null. Never serialize error text, exception class names,
argv, paths, PID lists, environment values or child output. Existing started
and exitCode fields remain. terminationStepsCompleted is true only after
cleanup is accepted and the original child is reaped; document that confirmed
Darwin exit is an accepted disposition rather than pretending a signal succeeded.
A started process with unconfirmed cleanup makes the batch incomplete even if
raw-root deletion succeeds. Raw cleanup keeps its separate existing report.

## Boundaries, compatibility and recovery

All implementation is in the test-only packet: observer.py, test_packet.py and
its closed manifest, plus the packet's current documentation/index. Models,
tools, source inputs, native argv/identity joins, raw/output/time limits,
no-retry policy and approval matching remain. Existing receipts and historical
manifest commitments are immutable. Update the observer pin by the normal
review path; a changed manifest matches none of the spent approvals. Any future
live attempt still needs a separate exact owner decision.

The summary remains version 1 with optional allowlisted cleanup details;
update its documented meaning and denial tests together. Rollback reverts code,
tests, current manifest and current documentation as one unit. It does not
restore live authorization or rewrite historical receipts. No product storage,
sidecar runtime, installed tool or release artifact changes.

## Delivery after approval

One owner performs these coupled steps; no parallel writers are needed.

- [x] Add deterministic failing regressions for exit between an alive observation
  and cap handling, primary plus cleanup failure, and every signal after reaping
  or ECHILD. Keep the original cap/deadline/clean-exit and stdio tests.
- [x] Implement the private original-child observer and bounded cleanup state
  transitions. Reproduce the original controlled failure on restored old code,
  then show green on the correction without suppressing genuine refusal.
- [x] Add denial coverage for a live leader, remaining group member, invalid or
  truncated native result, unsupported observation, status mismatch, expired
  confirmation/reap budget and automatic reaping. Retain an unrelated sentinel
  and assert no signal targets it or a released numeric identifier.
- [x] Add safe projection tests with private sentinels; demonstrate that forced
  lost-primary and permission-acceptance mutations are caught. Refresh only
  changed pins/current docs, keeping exact-approval denial before subprocesses.
- [ ] Run all synthetic packet tests on macOS and Linux, docs checks/build,
  `bash scripts/verify-repo.sh`, diff/doc/worktree checks and current-diff
  `/code-review medium`. Keep the 30-second docs wrapper unchanged; use
  controlled clocks/native answers for denial matrices and real owned children
  for the exit/cap and bounded cleanup paths. Medium is justified by process ownership, native ABI
  binding and failure-reporting changes. Record unrelated failures separately;
  no native capability qualification or historical attribution follows.

## Investigated risks and review

The principal hidden assumption was that a missing foreground process implies
a safely reusable group identifier. The controlled failure shows both exited
unreaped EPERM and current unconditional post-wait group signaling. Membership
alone also accepts a live leader, so it must be combined with original-child
exit. Non-consuming identity is required before the query and every signal.
The existing repository proof contract supplies the bounded alternative to a
new process/IPC controller. Its ctypes adaptation and failure precedence remain
implementation risks requiring the denial tests above.

| Dimension | Rating and reason |
| --- | --- |
| Dependencies | 2: one packet owner; standard Python plus existing Darwin OS APIs |
| Blast radius | 2: shared packet discovery/prompt supervisor, gated offline scope |
| State changes | 1: ephemeral ownership and optional safe receipt fields only |
| Reversibility | 1: coherent source/manifest/docs revert; no stored authority |
| Uncertainty | 3: host primitives reproduced; implementation and Linux checks pending |

Total 9/25, a planning aid rather than an approval threshold.
Review disposition: Ready for owner design approval after one independent
builder/plan review on 2026-11-17 (`gpt-6.1-sol`, user-approved substitution).
The reviewer found one material question: cooperative endpoints versus hard
wall-clock guarantees. The plan now explicitly retains cooperative native-call
semantics from the existing probe contract. No other material question or
source-contract contradiction was reported. Implementation, cross-platform
qualification and current-code review remain future checks.
Owner design decision: approved on 2026-11-18. This offline correction supplies
no live canary budget.

Primary API references: [Python waitid](https://docs.python.org/3.9/library/os.html#os.waitid),
[Apple killpg](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/killpg.2.html),
and the installed Darwin SDK sys/signal.h, sys/wait.h and sys/proc_info.h.

## Implementation evidence — 2026-11-18

The approved original-child observer, fixed cleanup endpoints and closed receipt
projection are implemented in the test-only packet. The macOS synthetic suite
passes all 61 tests (8.958 seconds). Both mutation controls fail as intended:
replacing the primary error and accepting EPERM without membership proof.
Repository verification passed, including Rust (3,597 passed / 21 ignored),
desktop (1,777 passed / 5 skipped), docs (2,056 passed) and both builds.
The unchanged 30-second packet wrapper passed in 9.43 seconds. Current-code
`/code-review medium` approved with no P1/P2 findings; Linux CI remains pending. These checks supply no live native budget or qualification.
