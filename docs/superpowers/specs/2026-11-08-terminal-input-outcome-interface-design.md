---
type: Design Contract
title: Private terminal input truth and completion interface
description: Proposed original-submission outcome ownership separating input byte effects, physical completion and application settlement.
tags: [orkworks, sessions, terminal, maintenance, ownership]
status: draft
workflow_status: proposed-owner-review
---

# Private terminal input truth and completion interface

Tracking: [#1569](https://github.com/Rambolarsen/orkworks/issues/1569), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Consumes accepted [registration/settlement](2026-11-08-maintenance-registration-settlement-design.md),
[bounded binding/delivery](2026-11-08-maintenance-settlement-bounds-design.md)
and [maintenance admission](2026-11-08-backup-maintenance-admission-design.md)
rules and the [pinned acknowledgement evidence](../../validation/backup-maintenance-writer-audit.md#delivery-outcome-and-enforced-limit-evidence--2026-11-08).
Existing backup purposes/v1, restore-first and marker authority remain unchanged.

Source context: `33b719333fcc43dd72c91bdf122adda110f7af03`.
Current [RuntimeCommand::Input and PendingPtyWrite](../../crates/orkworksd/src/runtime/session_runtime.rs)
use `Result<(), ()>`; [approved input](../../crates/orkworksd/src/runtime/terminal_runtime.rs)
coarsens error provenance, and [application delivery](../../crates/orkworksd/src/session_application.rs)
selects graph cancellation/completion from that coarse result. The local source
evidence is reused; this document specifies a missing interface, not available
runtime support. No code, product test, live operation or SQL/native/model probe
was executed; no numeric execution allowance is supplied.

## Proposed choices

1. One application-owned original submission record, registered and reserved
   before delivery effects, retains direct terminal-writer facts independently
   of callers and acknowledgement channels. Observation grants no resend authority.
2. A private outcome separates **input-effect truth**, **physical completion** and
   **application settlement**. Rejection provenance, full byte writes and partial/
   unknown effects are not collapsed into one error; flush failure cannot reverse
   a known full write.
3. Recommendation delivery consumes the rich original result. Known full writes
   select completion, known no-input-effect selects cancellation only after actual
   relevant completion, and partial/unknown remains protected for explicit owner
   resolution. No outcome, retry or compatibility wrapper resends input.

These choices are proposed for owner approval. They refine the accepted policy;
they do not implement the producer protocol, choose resource values or qualify a
native profile. No new HTTP/event/storage codec, public capability or durable
idempotency service is introduced.

## Private seam and original ownership

The application admits/reserves its original recommendation job before
`begin_execution` or its checked variant. The terminal owner then receives the
same exact job/slot and immutable input intent under existing current-runtime
and domain admission checks. Intent binds original root/incarnation, session,
runtime/channel identity, submission/slot identity and total UTF-8 byte length.
The existing native command payload remains with the physical owner; do not copy
raw prompt text or credentials into receipts, diagnostics or a durable result file.
A path/hash, selected workspace, report token or stored record cannot construct
submission authority.

The small private interface has three responsibilities; these describe contracts,
not implemented Rust signatures or public protocol fields:

| Operation | Required owned result |
| --- | --- |
| Submit original input | Consume one exact execution slot with reserved resources and absolute deadline; transfer the payload/physical execution to its actual owner and bind one retained submission record. No enqueue on failed admission. |
| Observe original result | Read the original record's current input facts and completion/settlement dispositions, with a bounded waiter if needed. Request cancellation, timeout or disconnect ends waiting, not the record or admitted physical work. |
| Record owner completion | Only the actual original preparation/writer/bookkeeping owner can publish its phase facts/completion into the retained record. No caller-supplied success/failure or replayed receipt discharges it. |

Execution authority is unique and transferred into the worker/command. A bounded
observer is read-only; cloning an observer does not clone a slot, create another
job, start another input or renew its deadline. The application coordinator keeps
the completion obligation and strong original-root/resource references even if
all observers disappear. Existing target/currentness checks remain binding;
retaining an old root does not authorize writing through a replaced workspace.

The retained record is allocated before physical dispatch and survives the
oneshot used to notify a waiter. Each phase transfers ownership gap-free. A
lost handle/channel never authorizes release of resources still used by physical
work. A panic can unwind/drop a worker's local handles; the retained record cannot
guarantee those handles survived. The coordinator retains its known original-root/
protection ownership and records unknown/Blocked if actual completion or resource
ownership cannot be established. Qualified owner resolution must account for the
actual remaining state; no destructor certifies I/O completion or safe teardown.
After process loss, records are not revived as live claims: startup/explicit
owner resolution must establish actual state or remain unavailable. This proposal
adds no persistent exactly-once delivery or automatic recovery/resend protocol.

## Input-effect truth

Use these private semantic cases, with original identity on every observation:

| Fact | Required actual evidence and consumer limit |
| --- | --- |
| Unresolved | Initial state or insufficient retained original evidence. Includes lost acknowledgement/owner, ambiguous native return and panic after possible enqueue/write. Absence of a receipt is not no effect. |
| Known no input effect | Actual owner proves this submission was never enqueued, or the runtime rejected it before its first native write invocation. A stop observed before that invocation can qualify only with this direct provenance. This says nothing about preparation/bookkeeping/workspace effects. |
| Known full byte write | Writer reports successful writes whose checked total equals the complete original byte length. Flush has a separate success/failure/unknown disposition. This is byte-write acceptance, not proof the harness interpreted/executed the command, accepted a model prompt or completed work. |
| Partial or unknown write | A positive reported prefix followed by failure/cancellation, or any native invocation whose effect cannot be excluded. Retain a checked reported-byte lower bound if actually known; zero reported bytes does not prove no effect after an ambiguous native error. No automatic cancellation/completion decision or resend follows. |

Do not classify native `write` error/zero return as known no input effect merely
because no positive count was observed. A platform-specific no-effect guarantee
requires its actual qualified native owner. Successful byte counts are evidence
of native writer acceptance only, not end-to-end tool execution. Invalid counts
or overflow retain unknown/protection rather than fabricate an exact prefix.
For empty input, known full length-zero completion proves no command text was
written and grants no semantic execution claim; it creates no resend permission.

The writer stores facts through its retained original completion route before
notifying a reply channel. A lost notification cannot erase an already retained
known full write. Progress may accumulate a lower bound; once an actual terminal
outcome is recorded it cannot be rewritten because a later observer timed out,
a flush failed or bookkeeping returned an error. A later real original-owner
result can resolve an unresolved observation; receipt/path equality alone cannot.
Duplicate phase reports must be idempotent for the same original outcome;
conflicting reports retain Blocked/protection and cannot select a second effect.
No per-write unbounded history or prompt-copy log is created.

## Physical completion is independent

The record tracks the finite recipe's physical phase obligations separately:
preparation/enqueue decision, native write plus flush return, driver acknowledgement
of worker return, and input bookkeeping. Each phase is pending, known physically
settled with its actual result/no-effect, or unresolved. A known error/partial input
can be physically settled while a recovery/protection obligation still blocks
maintenance. Conversely, a known input fact does not settle other physical phases.

`PendingPtyWrite` currently sends its acknowledgement before returning the writer;
the driver joins separately. The new interface must retain both facts. It cannot
use `accepted_rx` success, task cancellation or Drop as the whole-job completion
source. Teardown/return failure leaves its exact resources/obligation retained.
Bookkeeping/graph/event effects are not declared absent because no input was sent.

Input fact retention itself is bounded work under the reserved record; it cannot
hide new fallible filesystem publication after the last reported phase. Any actual
extra publication/cleanup requires a predeclared finite slot and actual completion.
The record does not issue Q: the application maintenance owner still requires all
registered original jobs/continuations and physical evidence before SQL close/Held.

## Application consumption and compatibility

Application settlement records graph action, optional delivery event and live/
metadata bookkeeping separately, retaining actual partial/failure results.
For the original recommendation job:

- A known full byte write may select its exact completion action after relevant
  physical phases settle; flush failure remains recorded and cannot select
  cancellation or resend. Best-effort event failure never reverses written input
  or authorizes a duplicate graph/event action.
- Known no input effect may select its exact cancellation action only when actual
  preparation/input/bookkeeping completion and their effects are accounted for.
  It cannot cancel another submission or clear unresolved workspace effects.
- Partial/unknown input or physical completion retains the original execution/
  protection identity for explicit bounded resolution; do not guess completion or
  cancellation from coarse `Err`, timeout, stale workspace or model observations.

Use the accepted initial-claim/one-notification/final-claim settlement recipe and
original absolute deadline. The original input slot is never executed again.
Expiry before a graph action's admission leaves a truthful unresolved obligation;
an already admitted action retains physical settlement ownership. Cancelling the
maintenance waiter cannot revoke a pre-Closing ordinary job's required slots.
No automatic stop, runtime adoption, rollback, force close or release is selected.

The future rich path is private to the terminal/application owner seam. Other
existing callers can retain compatibility result shapes only under their existing
domain semantics; compatibility errors must never become evidence for maintenance
or automatic resend. The recommendation maintenance consumer must use the rich
record directly, not reconstruct it from the old wrapper. This document does not
change current HTTP/UI/WebSocket responses or migrate a storage schema. Public
error presentation changes need their own product contract if later required.

## Integration, bounds and conditional checks

Actual integration owners are `terminal_runtime::submit_input_with_admission`
(preparation/enqueue/bookkeeping), `RuntimeCommand::Input`, `PendingPtyWrite` and
its driver join (native writer facts/completion), and
`SessionApplication::deliver_recommendation_input` with recommendation settlement
(application record/graph/event ownership). Existing locks and currentness remain;
use capture/register/revalidate and no wait under application/module/registry
guards. A function/enum definition alone cannot qualify the route.

Before code, close actual finite reservation values/enforcement for job/phase/
observer/notification records, retained payload copies, allocations/handles,
unknown outcomes and teardown/recovery scratch; exact deadline values/native
semantics remain separate inputs. Existing two input slots or 64-command channel
cannot substitute for complete application-wide fit. Missing bounds or owner
operations remain unavailable, not unlimited or successful-empty.

Later actual-owner oracles must cover no enqueue with workspace bookkeeping;
rejection before native call; short writes then stop/error; native error without
known progress; final write then flush error; lost ack after retained full write;
panic before/after enqueue and write; observer loss before/after grant; ack before
worker return; hung flush/teardown; conflicting reports; stale runtime/root; graph
success/event failure; deadline before/after graph admission; and restart with
lost volatile ownership. Assert no resend and no fabricated physical completion.
These checks were not run and supply no probe allowance.

## Handoff and approval boundary

Approval would accept the three interface choices for further qualification.
The [qualification plan](../plans/2026-11-08-backup-maintenance-qualification.md)
remains Investigate until actual owner operations, numeric/enforcement and complete
physical/native/external/restore inputs close. Existing accepted design reviews
and approvals are reused. A later bounded implementation plan must name the real
resource owners and regression checks before code; a mock outcome record cannot
be presented as production delivery evidence. No implementation, live operation,
SQL/native/model execution or new numerical allowance follows from this proposal.


## Review disposition

Fresh builder review found no material ownership/case ambiguity. Independent
authority/source review identified an overstated panic guarantee: local handles
can unwind/drop, so the record now preserves unknown/Blocked protection rather
than promising the physical owner still retains them. Known live physical work
stays charged; actual completion/teardown requires its real owner. Neither review
executed runtime operations or qualified missing resource/native inputs.
