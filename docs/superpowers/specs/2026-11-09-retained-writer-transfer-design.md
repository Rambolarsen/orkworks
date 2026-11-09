---
type: Design Contract
title: Retained writer checkpoints and return reporting
description: Proposed fixed byte checkpoints and unique writer/return reporter transfer through the existing worker before any production submission-owner activation.
tags: [orkworks, terminal, ownership, qualification]
status: draft
workflow_status: proposed-owner-review
---

# Retained writer checkpoints and return reporting

Tracking: [#1614](https://github.com/Rambolarsen/orkworks/issues/1614), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250). Follows the actual
[writer-result slice](../plans/2026-11-09-terminal-write-outcomes.md) in #1611
and [submission owner](../plans/2026-11-09-input-submission-owner-candidate.md)
in #1593. PR #1611 implemented [issue #1609](https://github.com/Rambolarsen/orkworks/issues/1609);
the plan tracks that implementation issue. Reuse accepted [input truth/completion](2026-11-08-terminal-input-outcome-interface-design.md)
and [reservation/deadline](2026-11-08-maintenance-settlement-bounds-design.md)
semantics. Prior approvals do not authorize the new bridge or owner changes.

**Status:** proposed for one concrete implementation decision. The user asked to
continue with designing the retained transfer after #1611. No bridge code or writer/native/model/resource qualification experiment ran
for this proposal; ordinary docs validation is separate.
If approved, record acceptance, a bounded plan and actual code in one task;
no separate unchanged acceptance-only task or repeated source inventory.

## Concrete gap and chosen approach

Source pin: `06d658048d03b79952a11727dbf35637dd4fb897`.
The existing writer now returns direct facts through its join, but the driver
discards them. A flush panic can occur before a normally returning outcome
exists. The candidate Owner has only an immutable terminal input field: a
second different `record_input` conflicts. Recording FullWrite/Unknown before
flush and then FullWrite/Succeeded through that same operation would therefore
protect a valid sequence as conflicting. Neither an ack nor a returned tuple
solves the retained-before-notification contract.

Add one fixed monotonic byte checkpoint distinct from the immutable terminal
input field. Integrate unique writer and return reporters into the actual
PendingPtyWrite operation through an optional private reporting path. Exercise
that path with the real Owner and writer/task implementation offline; existing
production constructors keep reporting absent. This obtains actual local
reporting/retention evidence without inventing a production owner/profile.

Alternatives rejected: relaxing every terminal fact to mutable would weaken
conflict protection; recording full bytes only after flush loses known truth on
panic; a second result cache duplicates ownership; a detached reporting worker
adds lifetime/resource obligations. Use the existing record, worker and join.

## Proposed approval scope

1. Extend the private candidate with fixed monotonic checked byte checkpoints,
   strongest-known snapshots and consistency checks against terminal facts,
   preserving immutable terminal report/idempotence/conflict behavior.
2. Add the private optional reporter bridge to the actual writer/task/join and
   direct offline tests. Existing production starts pass no reporters; no
   production Owner construction, input-command field or application routing.
3. Permit this bounded preparatory bridge and its ordinary repository checks
   before complete production/root/resource/native qualification. This is a new
   scoped decision, not an extension inferred from either previous approval.

No change to legacy acknowledgement projection, public responses, graph action,
input authority, startup, queue/stop/resize behavior or numeric/default policy.
No live/native/harness/model or new resource experiment is authorized. Real
production integration still needs its qualified admission/profile and retained
root/resource/phase owner; this reporting path grants no execution capability.

## Checkpoint versus terminal truth

Retain one optional checked byte count in each existing fixed record. It is
absent initially; there is no per-write history, payload, string or new observer.
Only the unique writer reporter can advance it, while that writer phase is
pending and no terminal input report exists. Same count is idempotent; regression,
count above original length, wrong role or late checkpoint is rejected with
protection and prior evidence preserved. For nonempty input, checkpoints are
positive counts only. Zero is permitted for empty input before flush; it says
nothing about command execution. No pre-write invocation marker is needed:
panic in a first call remains Unresolved rather than fabricated no-effect.

The real write loop publishes its checked accumulated positive prefix after each
successful native write return, before another write or flush. After the final
count, or before empty-input flush, the checkpoint therefore supplies known full
bytes even if flush blocks/panics. Interrupted/zero/error returns cannot increase
that count. No record mutex/application guard is held across write or flush.

Snapshot uses immutable terminal input when present, otherwise:

| Checkpoint | Best-known input fact |
| --- | --- |
| Absent | Unresolved. |
| Positive prefix below original length | PartialOrUnknown with that lower bound. |
| Complete original length, including explicit empty zero | FullWrite with flush Unknown. |

The checkpoint is incomplete reporting evidence, not a terminal outcome. A
later normally returning writer can install its first terminal fact, including
actual flush disposition. Reject a terminal report that erases a retained
prefix, changes known full bytes to partial/no-effect/Unresolved, or contradicts
other no-effect claims. Preserve known byte evidence and protection on conflict;
do not let an invalid terminal value hide a stronger checkpoint in snapshots.
Existing identical terminal reports remain idempotent; different terminal
reports still conflict. Terminal Unresolved/partial reports do not become a
retry/resolution API. Explicit protected-owner resolution remains unavailable.

A partial checkpoint can advance to full before terminal settlement; this only
replaces a provisional lower bound from the same original writer. It cannot
clear latched conflict protection or lost/failed phase state. Full/Unknown itself
does not discharge the pending writer, return, preparation or bookkeeping phases.
Retirement still requires the existing known-safe phase/handle conditions.

## Unique pairing and transfer

Use the already admitted unique writer and physical-return reporters. The
reported-start path validates their roles, exact common Core/slot/serial,
pending/unspent writer and return phases, absent checkpoint/terminal input,
and original byte length against the actual moved
String before spawning a worker or calling Write. Context value/path equality
across records/owners is insufficient. No new job, slot, observer or serial is
allocated. Failed validation returns ownership of all consumed inputs (writer, payload,
reporters and existing control/reply handles) to the caller with a fixed reason; it cannot drop a native writer and
claim safe teardown, enqueue anything or settle an unrelated record. Exact
private type names/placement are builder choices.

The pair is not Clone. Move the writer reporter into the existing blocking
closure; keep the return reporter with PendingPtyWrite and its existing join.
Move the payload exactly once as today. The record contains only fixed facts;
it does not take the writer, payload or real root resources. The future
application owner must retain its original strong scope/resource ownership;
the fixture Owner lifetime is not proof that production does so.

The normal order is: checked prefix checkpoints; write/flush return; terminal
input report; writer-phase result; existing coarse acknowledgement attempt;
closure returns writer/outcome. A closed ack receiver changes none of those
reports. No new thread/task/channel/notification loop or report retry is added.

| Actual writer result | Writer phase result after terminal publication |
| --- | --- |
| Nonempty stop before any native call, no flush | NoEffect, with NoInputEffect input fact. |
| Full bytes and successful flush, including empty input | Completed, with actual full/flush fact. |
| Full bytes and failed flush | Failed, while retaining FullWrite/Failed; legacy acknowledgement remains Ok. |
| Partial/zero/error normally returns | Failed, with actual PartialOrUnknown lower bound; legacy Err. |
| Write/flush panic before normal return | Reporter Drop marks Lost; existing checkpoint remains. No fabricated terminal fact. |

Failed is a known returned error plus protection, not successful completion or
safe release. A known full write never selects cancellation/resend because its
flush or another phase failed. This slice does not change the application graph
consumer or invent a force-clear operation.

## Actual join and loss

The driver must take ownership of the pending operation when its existing task
await resolves, before dropping PendingPtyWrite. Its current `pending_write =
None` before result handling would otherwise lose the return reporter. A small
private join-finalization operation must be shared by the driver and tests;
no test may fake physical return by manually setting its reporter.

On a successful task join, the driver has actually received the writer/outcome;
it reports return Completed, then restores the writer at the existing place.
JoinError reports return Failed/protected and keeps existing runtime failure
handling; it does not certify native teardown or fabricate a returned writer.
A post-ack panic can leave known terminal input/writer completion while return
fails. Preparation and bookkeeping are never completed by this bridge.

Dropping PendingPtyWrite before join retains existing cancellation signaling;
the return reporter becomes Lost/protected. The actual blocking closure can
still finish/report through its owned writer reporter. Do not spawn a replacement
joiner, abort blocking I/O, renew a deadline or infer return from Drop/ack. The
bridge cannot promise that panicked locals or a dropped output retained native
handles; unknown teardown remains protected for the later qualified owner.

## Reporting failure and resource limits

Record/report unavailability is not no-input-effect. On a failed checkpoint,
stop further write/flush attempts in the reported path and return the truthful
best direct byte outcome (full bytes remain full with flush Unknown; a prefix
remains partial/unknown). Preserve any already stored checkpoint and conflict
protection. Do not release a protected record, resend, fabricate flush success
or pretend a poisoned/unavailable record is empty. Carry a fixed reporting-error/status alongside the direct byte outcome through
the existing join; it is separate from input truth, and legacy ack still
projects the actual byte fact. No additional error channel is introduced.
Production no-report mode retains its current behavior unchanged.

Terminal/phase publication failure likewise cannot reverse actual written bytes.
If a record cannot be updated/read, it remains unavailable/protected rather than
claiming retained evidence is complete. The loop cannot atomically couple native
I/O with in-memory publication; a panic between a native return and checkpoint
can still lose that latest local count. State only the checkpoint actually
retained. No catch-unwind completion guarantee or durable exactly-once service.

The additional record field/pair/phase status are fixed-size; existing constructor
capacity checks must use their actual new representation. Fixture limits remain
explicit test inputs. Existing reporters retain Core references; dropping an
observer cannot release a record/worker obligation, and late reports settle
already admitted obligations without reserving or renewing slots/deadlines.
The bridge does not supply new input admission authority or validate a physical
execution deadline. Existing ordinary input authority remains required, and
production reporting stays absent until the real admission/resource contract is
qualified. No global registry, growing history, root/payload copy, observer/task
or production default is introduced.

## Bounded implementation handoff after approval

One task/PR records acceptance, a concise reviewed plan and actual code. Scope:
`runtime/input_submission.rs` and its tests, plus the existing
`runtime/session_runtime.rs` writer/task/join and scoped writer tests. No
RuntimeCommand, terminal/application preparation/bookkeeping, HTTP, startup,
recommendation settlement, storage format or production Owner factory change.

- TDD the actual candidate checkpoint transitions and immutable-terminal checks:
  same/increasing/invalid/regressing/late/wrong-role reports, prefix/full-before-
  flush snapshots, conflicts preserving stronger evidence, and lost/failed
  phase/observer capacity and retirement protection. Existing tests stay green.
- Use real Owner admission with explicit finite fixture values and the actual
  PendingPtyWrite/reporting/join operation. Hold preparation/bookkeeping pending;
  never manufacture whole-job completion to call this producer qualification.
- Deterministically hold flush after the last byte, then inspect full/Unknown
  with writer/return pending. Inject flush panic and observe retained full bytes,
  writer Lost and failed return through actual join finalization. Also cover
  partial checkpoint then write panic, first-call panic with no checkpoint,
  flush failure/full ack, lost ack and post-ack/pre-return panic.
- Drop the real pending operation/observers while its worker is held; release
  the fixture gate and observe late writer facts with return Lost/protection.
  Gates release on failure/drop; watchdogs bound broken tests rather than infer
  ordering. Use existing per-invocation test seams, no global hooks or sleeps.
- Reject mixed Core/key/role/length or spent reporting pairs before any worker/
  native call, preserving consumed resource ownership. Exercise report conflict/
  unavailability with deterministic test-only fault control if needed; add no
  production fault switch or successful-empty fallback.
- Verify all production constructors have reporting absent and no Owner factory
  is called outside tests. Run scoped/full repository checks, docs, fresh
  current-code manual review and required CI/complete PR inventory/cleanup.

Pass is actual local checkpoint/reporting transfer through existing operations.
It is not real root/lease/payload resource ownership, all-module registration,
qualified numeric/deadline/native/physical fit, Q/readiness, backup or restore.
Those gates remain open; this proposal does not authorize its code until the
three scoped choices above are approved.

## Review disposition

Fresh builder review found no materially unanswered questions. Independent
source/authority review found no ownership/semantic conflict. Its traceability
note compared implementation issue #1609 with PR #1611; the merged PR was
verified to close #1609, and that relationship is now explicit above. This is a
citation clarification, not a change to scope or prior approval. Reporting
errors have a fixed separate join status, not input-truth coercion or a new
channel. Both reviews were read-only and performed no runtime qualification.
Existing accepted semantics and implementation evidence are reused; no new
source inventory or physical qualification is claimed.
