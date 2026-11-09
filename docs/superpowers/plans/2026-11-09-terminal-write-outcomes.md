---
type: Implementation Plan
title: Direct terminal writer outcomes implementation
description: One bounded writer and driver refactor with direct byte and flush facts and unchanged acknowledgement behavior.
tags: [orkworks, terminal, ownership, qualification]
status: stable
workflow_status: locally-verified
---

# Direct terminal writer outcomes implementation

Tracking: [#1609](https://github.com/Rambolarsen/orkworks/issues/1609), parent #1250.
Spec: [accepted writer/driver slice](../specs/2026-11-09-terminal-writer-outcomes-design.md).
The owner answered “keep going” to the explicit implementation question after
#1607. Acceptance, this plan and actual code belong to one logical task. Execute
inline; do not request repeat design/execution approval.

Pass: actual PendingPtyWrite returns fixed direct facts with its writer through
its existing join; existing acknowledgements/callers behave as before. No
submission-owner activation or native/whole-application qualification follows.

## Decisions and scope

- Keep the write operation in `runtime/session_runtime.rs`: `write_pty_input`
  borrows the existing writer and byte slice, checks the existing stop source,
  and returns `PtyWriteOutcome { input_bytes, fact }`. Reuse existing private
  `InputFact`/`Flush` enums; the accounting Owner stays unwired. Update comments
  to distinguish shared facts from production accounting construction.
- `PtyWriteOutcome::legacy_result` is the single coarse projection: full write
  yields Ok, other outcomes Err. No additional payload, result registry, task,
  channel, native handle, deadline/default or dynamic error/string is retained.
- `PendingPtyWrite::task` returns `(writer, outcome)`. Construct outcome after
  normal loop/flush return, project/send acknowledgement, then return both.
  Driver restores writer at the same join point, discarding outcome for now.
  Panic is still JoinError/unknown, never a fabricated outcome.
- Tests live in `runtime/session_runtime/write_outcome_tests.rs`. Use bounded
  scripted Write adapters for direct-loop cases and actual blocking tasks for
  lifetime cases. One invocation-local cfg(test) post-ack callback/barrier is
  permitted inside the real closure; no global hook or production callback.
- Preserve RuntimeCommand, terminal/application preparation and settlement,
  public protocols, startup, queue/resize/stop ordering. No source inventory
  rewrite or unrelated cleanup. Update canonical architecture, acceptance/ADR
  and qualification evidence within this task.

## One task and its checks

- [x] Write direct-outcome tests first and observe missing operation/result red.
  Cover full/short/interrupted writes with exact remaining slices, error/zero
  before and after a known prefix, stop before any invocation and after partial
  or final write, malformed counts without overclaim/panic, empty input and
  flush failure. Assert literal facts and existing acknowledgement projection.
- [x] Implement the minimal loop/result and existing task/driver transport.
  Run direct tests green. No production accounting Owner call is introduced.
- [x] Add failing actual-task tests, then the test-only barrier support: dropped
  ack still yields direct join result, ack precedes held closure return, blocked
  flush/write does not complete early, stop cannot undo final bytes, and panic
  returns JoinError without no-effect/physical completion. All fixture barriers
  release on assertion failure; watchdog timeouts bound broken tests, not infer
  order. Existing writer/startup/stop regressions must remain green.
- [x] Run scoped tests, fmt/build and full `scripts/verify-repo.sh`; inspect
  failures before fixing. Run docs/close-out checks and fresh current-code
  `/code-review low`, escalating only for documented concurrency/diff risk.

Required CI and complete PR-comment inventory still gate maintainer merge/cleanup;
those forge results are recorded in the PR/issue rather than inferred locally.

## Review and risk

Review focus: partial truth versus rejection; full write versus flush failure;
ack versus blocking return; stop while physical I/O runs; panic/driver loss.
Each has a direct or actual-task test above. No detached completion claim is
inferred from a stopped waiter. Unknown physical/native/root/resource fit stays
unknown; complete ledger and numeric/deadline gates are unchanged.

Dependencies 2 (existing writer/task + private fixed facts); blast radius 2
(shared writer, coarse result unchanged); state changes 1 (no durable change);
reversibility 1 (local revert, no schema); uncertainty 2 (testable concurrency).
Total 8. Plan self-review against the accepted scope: Ready; every requested
case maps to a test, and no dependent application/native ownership is included.
Fresh design builder/source reviews from #1607 are reused unchanged. Final fresh
code review still required. Verification evidence will be recorded here.


## Execution evidence

Seven direct-operation tests first failed because PtyWriteOutcome/write_pty_input
did not exist, then passed after minimal implementation. Four actual-task tests
then failed against the old task return shape/missing post-ack seam; the existing
worker/join refactor made all 11 tests pass (zero failures). Actual callbacks
exist only under cfg(test), are per invocation and have release-on-drop gates;
watchdogs bound broken fixture waits rather than infer ordering. No live PTY or
harness operation ran in these scoped tests. Full repository verification and
fresh code review subsequently passed as recorded below.


## Current-code review

Fresh diff-scoped `/code-review low` completed with APPROVE and no findings.
It covered all four code files including the untracked test module; the reviewer
verified the exact SHA-256 manifest later recorded in PR evidence and inspected
the 11-test green log without rerunning/contending the full verification. No
new authority, payload retention or production accounting activation was found.
The manual gate applies to this exact code; later code changes require refresh.
The complete repository verification subsequently passed on this unchanged code.


## Final local verification and limits

`bash scripts/verify-repo.sh` passed on the reviewed code: prerequisites; Rust
format/build and 3,371 unit tests (18 existing ignored, zero failures) plus all
integration binaries; 31 OpenCode tests; desktop type-check and Electron ABI
check; 1,720 desktop tests (3 existing skipped, zero failures) and build; 1,204
docs tests, OKF/index/site build; whitespace/doc-check/worktree-check. No new
ignored/skipped test was added. The exact code review manifest still matches.

User/home/README currency nudges need no edit: public caller/UI behavior,
installer availability and advertised architecture/milestones do not change.
Canonical architecture now records direct facts and the discarded driver result.
Existing source-only SQL unused/dead-code and build chunk/NO_COLOR warnings are
outside this diff; no new warning was reported for the changed writer.

This establishes actual local producer-result and ordinary repository regression
evidence only. There is no production submission-owner construction, retained
application fact reader, original root/resource/phase integration, qualified
numeric/deadline/native/physical profile, Q/readiness or restore result. Panic
or driver loss can still lose local facts; no live/model experiment was run.
