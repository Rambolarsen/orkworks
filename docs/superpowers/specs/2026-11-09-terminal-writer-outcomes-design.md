---
type: Design Contract
title: Direct terminal writer and driver outcomes
description: Proposed first producer slice returning fixed-size byte and flush facts from the existing writer without activating submission ownership.
tags: [orkworks, sessions, terminal, ownership, qualification]
status: draft
workflow_status: proposed-owner-review
---

# Direct terminal writer and driver outcomes

Tracking: [#1604](https://github.com/Rambolarsen/orkworks/issues/1604), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250). Follows the actual
[source-only owner](../plans/2026-11-09-input-submission-owner-candidate.md)
delivered in #1593. Reuses the accepted [input-outcome semantics](2026-11-08-terminal-input-outcome-interface-design.md),
[resource/deadline policy](2026-11-08-maintenance-settlement-bounds-design.md)
and [one-candidate exception](2026-11-09-submission-owner-qualification-sequencing.md).
The exception is exhausted by that candidate; it does not authorize this code.

**Status:** proposed. The user asked to continue with defining the first real
producer integration. This document proposes the smallest actual writer-result
slice and its implementation handoff for a single owner decision. No new code,
writer/native probe, fixture execution or resource qualification was performed.
Approval can cover the concrete slice and its normal repository/offline checks
together; do not split unchanged acceptance into another docs-only task.

## Problem and choice

Source pin: `1e012aafd067b4a410f71a7fe32fe76cdfad8c2c`. The
[writer and acknowledgement](https://github.com/Rambolarsen/orkworks/blob/1e012aafd067b4a410f71a7fe32fe76cdfad8c2c/crates/orkworksd/src/runtime/session_runtime.rs#L368-L429),
[driver join](https://github.com/Rambolarsen/orkworks/blob/1e012aafd067b4a410f71a7fe32fe76cdfad8c2c/crates/orkworksd/src/runtime/session_runtime.rs#L2885-L2925)
and [prepare/bookkeeping owner](https://github.com/Rambolarsen/orkworks/blob/1e012aafd067b4a410f71a7fe32fe76cdfad8c2c/crates/orkworksd/src/runtime/terminal_runtime.rs#L723-L818)
are the concrete producer seams. Reuse the canonical audit's existing evidence;
these links locate the proposed change rather than claiming a new qualification.

`PendingPtyWrite` owns the actual blocking write/flush loop, sends a coarse
`Result<(), ()>` acknowledgement, then returns its writer. Its driver separately
awaits the blocking task. The original caller can disappear while that work
continues. The new submission owner cannot infer full/partial/no-effect facts
from the old acknowledgement, and acknowledgement is not driver-observed return.

Propose a fixed-size direct result from that existing loop, carried alongside
the returned writer through the existing task join. The same result determines
the existing acknowledgement projection. The production driver recovers its
writer exactly as today; this slice does not retain the result in an application
ledger or use it to change recommendation settlement.

This obtains the missing producer operation in actual production source, with
its real loop exercised through offline `Write` adapters. It is deliberately
less than end-to-end original-submission integration. A passing test of this
operation cannot qualify application ownership, live PTY behavior or maintenance.

Alternatives considered:

- Inferring facts from the existing acknowledgement cannot distinguish partial
  writes or a failed flush after full write. Reject it.
- Changing acknowledgements/application settlement now would also require the
  original root/resource/result owner, bounded reservation/deadline and protected
  resolution integration. Those inputs are still unqualified. Defer that larger
  change instead of sending a richer value through an unqualified consumer.
- Another standalone owner or simulated drained producer adds no direct writer
  evidence. Use the existing writer and driver rather than a parallel algorithm.

## Proposed approval scope

1. Refactor the existing `PendingPtyWrite` loop to produce direct fixed-size byte
   and flush facts, and return those facts beside its original writer through the
   existing blocking task. Reuse the same algorithm, stop source and ordering.
2. Preserve the existing acknowledgement and caller/HTTP/application behavior
   through one explicit projection of that result. Add offline behavioral tests
   through the same operation and real blocking task/join ownership.
3. Permit this preparatory writer/driver refactor before complete application/
   native/resource qualification. Do not construct or call the submission owner
   in production, create an original input claim or activate maintenance. This
   is a new narrowly scoped implementation decision, not a broad extension of
   the previous one-module exception or a qualified production profile.

The existing user-authorized terminal operation keeps its existing authority.
No new way to submit input, retry a prompt, stop/adopt a runtime or bypass a domain
check is introduced. Approval of this source change is not permission to run a
live coding tool, native PTY experiment or new numerical resource experiment.

## One direct result, one existing worker

Keep the implementation local to `runtime/session_runtime.rs` and a scoped
writer test module if splitting tests improves locality. Extract only the
existing write loop if needed for tests. Tests and production must call that
same implementation; no second writer algorithm or new task/queue/subscription.
Exact private type names and test-file placement are builder choices.

The direct result has fixed-size scalar/enum fields only: original byte length,
input fact (no input effect, full write with flush disposition, or partial/unknown
with a checked reported prefix) and, if needed, a fixed discriminator for invalid
writer output. It holds no prompt copy, string, path, native handle or serialized
receipt. It cannot construct a live root, runtime, submission identity or permit.
Reuse one fact vocabulary with the candidate where practical; moving private
fact types is allowed only if the owner remains unwired and visibility is no
broader than needed by these runtime modules. Do not duplicate semantic enums
just to preserve a file location, or widen an interface to satisfy a test.

The existing `String` moves into the same blocking task and is not cloned by
this change. The existing writer, cancellation atomic/watch receiver, task and
acknowledgement retain their present owners. The task returns a fixed result
with the writer; no separate worker or reply channel is added. One fixed copy
may feed the compatibility projection while the original accompanies join.
No dynamic history, observer registry, global map, production default or new
budget setting is permitted by this slice.

## Truth at the writer

The classification is direct evidence from this invocation, not from elapsed
wait time, a model signal, a failed acknowledgement or a selected workspace.

| Actual event | Direct result and compatibility projection |
| --- | --- |
| Stop/cancel observed before any native write call, with nonempty input | Known no input effect; legacy `Err(())`. No bookkeeping/workspace-effect claim. |
| Successful writes cover the complete original length, flush returns success | Full write / flush succeeded; legacy `Ok(())`. No tool execution claim. |
| Full length written, flush returns error | Full write / flush failed; legacy `Ok(())`, retaining the existing warning. Never reverse known bytes. |
| Short writes followed by stop/error/zero return | Partial/unknown with checked successful prefix; legacy `Err(())`. No automatic graph choice or resend. |
| Error/zero return after entering a write, no successful bytes | Partial/unknown with prefix zero; legacy `Err(())`. Zero is not qualified native no-effect evidence. |
| Interrupted write | Keep the existing same-worker loop and remaining slice; it does not create another input submission. A later stop/error still uses the conservative invocation history. No new retry mechanism. |
| Writer reports a count larger than supplied slice, or arithmetic is invalid | Reject that count without indexing panic or increasing the verified prefix; partial/unknown, legacy `Err(())`. No exact effect claim from malformed writer evidence. |
| Empty input | Full length zero with actual flush disposition, preserving current `Ok(())`; no command execution claim. |

Check each positive count against the supplied remaining length and checked
prefix accounting. Do not add the invalid count to a truthful lower bound.
Flush is attempted only after all bytes were successfully accounted for, exactly
as today. A stop arriving after the final write does not turn it into rejection;
flush error cannot authorize a second input submission.

No new write deadline is invented. The existing loop can wait indefinitely in
OS I/O, flush or repeated interruption. Its stop checks do not abort an already
entered native call. Complete finite admission/deadline and retained resource
fit remain inputs to the later application integration, not claims of this type.

## Acknowledgement, return and loss

Construct the result after the normally returning loop/flush; derive the old
acknowledgement once, attempt its send, then return the writer and direct result
from the same closure. An unavailable acknowledgement receiver must not discard
or rewrite that returned result. The acknowledgement remains its present coarse
shape and must not be presented as a rich-result/maintenance source.

For a deterministic ack-before-return test, permit one invocation-local
`cfg(test)` post-ack barrier between the send attempt and closure return. It is
absent from production, has no global state, cannot alter byte/flush facts and
uses only fixture-bounded synchronization. Hold that exact closure, receive ack,
assert join is pending, release the barrier and await its result. This measures
closure return ordering, not ongoing native I/O. Do not infer that interval from
a sleep or a racing `is_finished` check without the held barrier.

The driver observes successful blocking-task return separately. Recover the
writer on the same path and preserve output draining, initial-prompt handling,
kill ordering and resize/input ordering. No driver return is inferred from the
acknowledgement. The driver may consume/discard the direct result after the join
in this preparatory slice; there is no application result reader yet. Do not add
logging of prompts or an ad-hoc result cache to make it look retained.

A panic before normal return may lose local byte/flush evidence. It remains a
join error/unknown original outcome, never no-input-effect or a fabricated direct
result. In particular, a flush panic after the last write is not covered by a
retained full-write record in this slice. Do not catch panic to fabricate normal
writer return, claim its handles survived, or mark physical settlement complete.
Dropping the driver/join handle may detach physical work; this slice does not
solve that ownership loss. A hung flush or writer still blocks actual return.
These explicit limits prevent the producer seam being mistaken for the accepted
independently retained original-submission contract.

## Mapping to the existing owner and remaining integration

| Candidate obligation | This slice's evidence | Still required before actual owner integration |
| --- | --- | --- |
| Preparation | None; existing fenced prepare/enqueue is unchanged. | Gap-free original root/runtime/slot binding, pre-effect reservation and actual no-enqueue completion. |
| Writer | Direct normally returning write/flush fact from the existing loop. | Actual unique writer reporter and retained publication before caller notification, including known bytes before a possibly panicking flush. |
| Physical return | Existing driver observes task return and recovers writer separately from ack. | Unique return reporter, original result/resource ownership, panic/teardown and driver-loss handling. |
| Bookkeeping | None; existing post-delivery path is unchanged. | Actual metadata/graph/event completion ownership and truthful protected settlement. |

No candidate `PhaseResult` is set by this slice. A fixed report is not an
execution capability or physical-owner certificate. Offline tests may compare
its cases with candidate semantics, but manually feeding a fixture reporter does
not count as producer integration. Production owner construction, real strong
root/lease references, complete payload/allocation/native-handle charging, bounded
observers/results, absolute deadline enforcement and exact protected resolution
remain gated. No all-module registry, Q/readiness or backup/restore box closes.

## Bounded implementation handoff after approval

One implementation task/PR should record acceptance and execute this slice; no
separate acceptance-only follow-up. Keep changes to the writer/driver and their
necessary private types/tests. Do not alter `RuntimeCommand::Input` payload or
acknowledgement fields, `terminal_runtime` preparation/bookkeeping, HTTP, application
recommendation settlement, runtime startup or public/storage protocols.

- Write failing tests against the actual writer operation first: pre-call stop;
  short/full/interrupted writes; zero/error with zero/positive known progress;
  checked invalid count; flush failure after full; empty input; stop after final
  write. Assert exact input slices/call counts and unchanged legacy projection.
- Exercise `PendingPtyWrite` itself with bounded in-memory `Write` adapters and
  deterministic barriers: lost ack retains the returned result; ack can precede
  blocking closure return; blocked flush prevents return; stop while a call is
  entered does not fabricate its completion; panic produces join error without
  a no-effect or successful-return claim. Release every test barrier before exit.
  Use no wall-clock sleeps to infer ordering or live/native harness invocation.
- Implement the minimal extraction/result/driver shape; preserve existing
  cancellation, acknowledgement, queue ordering, startup and teardown tests.
  Verify no production `input_submission::Owner` construction/reporting, extra
  payload copy, worker/queue, background observer or result registry.
- Run scoped Rust tests, formatting/build and full `scripts/verify-repo.sh`, docs
  checks and fresh current-diff `/code-review low` (escalate only for documented
  diff size/concurrency risk), then required CI/PR lifecycle. Ordinary repository
  fixtures remain ordinary verification, not native/model/resource qualification.

Acceptance criteria are direct writer evidence and unchanged compatibility/
control behavior, not end-to-end retention or whole-application fit. Unexpected
need for production owner activation, another phase, new allocations/handles,
resource/default policy or an effectful probe requires a new bounded decision
before expanding implementation.

## Review disposition

Fresh builder review found the ack-before-return assertion needed a deterministic
post-ack hold point. The handoff now permits one invocation-local `cfg(test)`
barrier, absent from production; lost-ack evidence remains only task-join result
transport, not application retention. Independent authority/source review found
no material findings in the pinned loop/driver mapping, compatibility, panic/
loss limits or explicit new approval/resource gates. Both reviews were read-only;
no runtime operations or physical qualification were executed.
This document is not accepted
and cannot authorize its dependent code until the owner approves the three scope
choices above. Existing outcome and candidate approvals remain valid unchanged.
