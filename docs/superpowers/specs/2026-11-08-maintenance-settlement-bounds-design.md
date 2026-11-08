---
type: Design Contract
title: Bounded cleanup binding and recommendation settlement
description: Accepted conditional reservation, deadline and truthful completion rules for detached cleanup binding and contended recommendation settlement.
tags: [orkworks, sessions, backup, maintenance, resources]
status: stable
workflow_status: accepted-for-qualification
---

# Bounded cleanup binding and recommendation settlement

Tracking: [#1558](https://github.com/Rambolarsen/orkworks/issues/1558), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Consumes the accepted [maintenance admission](2026-11-08-backup-maintenance-admission-design.md)
and [registration/settlement](2026-11-08-maintenance-registration-settlement-design.md)
designs and [constructor/ingress evidence](../../validation/backup-maintenance-writer-audit.md#constructor-ingress-and-completion-coverage--2026-11-08).
Existing backup purposes, v1 bytes, restore/marker authority and no-automatic-stop
rules remain unchanged. This design narrows two missing finite-job contracts;
it does not qualify the whole producer catalog or supply capture resource fit.

Source pin: `a0404c2aa7eae301aad754bb6444de2f9742aa36`. Source was read only.
No product test, SQL/native/model execution, live process/metadata inspection,
stop/capture/restore, new execution allowance or implementation is authorized.

## Accepted choices for further qualification

1. Reserve the complete per-job resource vector and independently retained result/
   protection record before dispatch or the first durable delivery reservation.
   Caller loss and deadline expiry retain those charges until actual completion
   or qualified owner resolution; exhaustion rejects new work.
2. Use one absolute admission/wait deadline per original attempt, without timer
   renewal or automatic retries. Expiry stops future admissions and bounded
   waiting, never an admitted physical operation or its truthful completion.
3. Recommendation settlement gets one initial store attempt, at most one
   availability notification wait and one final store attempt. Contention after
   that, expiry or ambiguous delivery retains a blocked obligation for explicit
   resolution. Settlement never resends input.

After #1560, the owner answered “go ahead” to the explicit three-choice approval
request. [#1561](https://github.com/Rambolarsen/orkworks/issues/1561) records
acceptance of these unchanged choices for further qualification on 2026-11-08.
Reuse this approval with the accepted maintenance/registration/backup/adoption/v1
rules. The fixed attempt graph is not a measured throughput/time/resource allowance.
Numeric profile values remain unavailable until their actual owners qualify them.

## Actual controls and missing guarantees

| Source at the pin | Existing control and limit |
| --- | --- |
| [file_budget.rs](../../crates/orkworksd/src/session_application/sql_writes/file_budget.rs) | Application-wide reservation retains encoded intent bytes and job count across switched-away workspaces. Current constants are 1 MiB per intent, 32 jobs and at most 32 MiB intent bytes. They do not account for all captured owners, handles, allocator overhead, callbacks or recovery storage and are not backup allowances. |
| [deletion.rs](../../crates/orkworksd/src/session_application/sql_writes/deletion.rs), `capture_deletion` | Reserves file intent capacity, captures original deletion/root context and dispatches blocking lease-bound replay/image/settings/recommendation binding. Await has a two-second timeout; timeout does not cancel blocking binding or report its physical completion. |
| [application](../../crates/orkworksd/src/session_application.rs), `accept_recommendation_with_packet`, `deliver_recommendation_input` | Durable execution reservation precedes owned input delivery; an owned task retains the delivery guard across HTTP waiter loss. Target workspace identity holds a weak lease reference. It is not a strong original-root maintenance reservation/result ledger. |
| [delivery](../../crates/orkworksd/src/session_application/recommendation_delivery.rs), `settle_recommendation_delivery`; [operation](../../crates/orkworksd/src/taskmaster/store/operation.rs), `wait_until_available` | BUSY settlement retries after availability, retaining target/guard but without a deadline or attempt cap. Availability internally loops on notifications and does not reserve the next store claim. Current helper cannot be relabelled a single bounded notification operation. |
| [terminal runtime](../../crates/orkworksd/src/runtime/terminal_runtime.rs), `submit_input_with_admission`, `submit_approved_input_to_runtime_with_admission` | Owned input task retains input slot/gate, channel capacity and writer acknowledgement; it also has blocking preparation/bookkeeping. The approved-input wrapper returns `Result<(), ()>`. A lost acknowledgement or error after enqueue cannot be assumed to prove no input effect. Qualified delivery-outcome distinction remains required. |

An existing cap/deadline applies only to the resource or wait it actually bounds.
Neither a closed SQL queue, async timeout nor failed response proves all binding,
input preparation, writer delivery, bookkeeping and graph publication settled.

## Reservation vector and accounting

The actual application owner must supply finite limits and charge units for:

| Dimension | Per-job retained charge and application-wide accounting |
| --- | --- |
| Jobs/roots/slots | Original-root job record, root/owner references, fixed continuation slots, dispatch/completion slots and unresolved obligations, including switched-away roots. |
| Memory | Encoded intent plus bounded in-memory identities, paths, prompt/title payloads where retained, graph/result representations, error records and all child bookkeeping; use actual measured representation/allocation bounds, not serialized size alone. |
| Native handles | Captured directories, leases, cleanup locks, store handles and in-flight worker/input handles, including temporary peak overlap during binding/transfer/teardown. |
| Notifications/time | One registered waiter per permitted wait, bounded retained notification state, one original absolute deadline and bounded caller/coordinator wait. No polling or renewed relative timers. |
| Recovery/results | Bounded retained terminal outcome and unresolved protection identity, durable recovery evidence where required by the existing domain owner, and exact cleanup/teardown scratch. No new durable journal codec is selected. |

For each dimension, checked admission enforces `used + reserved <= limit` before
work can run. Reserve the maximum simultaneously live physical footprint of the
recipe, including handoff overlap; shared resources are charged once to an actual
retained owner with explicit references. Missing limits, unknown allocation/handle
peaks or arithmetic overflow reject before dispatch. No fallback to an uncharged
Tokio task, queue, error string or global map. Existing file, SQL and input-slot
reservations remain owned by their original modules; a complete reservation must
show which charge covers each retained resource without inventing available fit.

Reservation transfer is gap-free. A worker may hold the actual handles while the
coordinator holds the obligation/result owner; both cannot release the last charge
while physical work or unresolved protection still needs it. On known physical
completion, execution resources may be released after safe teardown, while the
bounded outcome/protection record stays charged until consumed or explicitly
resolved. No unbounded completed-result history is introduced. Capacity pressure
rejects new admission; it cannot evict an unresolved job, lease or outcome. A hung
owner can retain its finite slot indefinitely and block new admissions; the design
bounds retained resources, not the duration of arbitrary filesystem/OS I/O.

## One deadline, separate physical truth

The application assigns a checked absolute deadline at original job admission
from the actual qualified owner/profile. It covers queueing, contention waits and
new continuation admissions; child slots inherit the same deadline. Existing
shorter domain deadlines remain binding. Switching root, receiving a notification,
dropping/recreating a waiter or beginning settlement cannot reset it.

Immediately before a slot admission, the transition checks deadline, original
identity, cancellation and predecessors. An admitted operation retains settlement
ownership even if its first physical instruction executes after expiry. Expiry
rejects unadmitted slots and stops the caller's/coordinator's bounded wait. It
records unresolved/Blocked when physical work or protection remains; it never
aborts blocking I/O, cancels an already enqueued prompt or frees its resources.
The original completion path can still report its result without admitting a new
effect. Required but unadmitted progress after expiry remains an explicit durable
recovery obligation, not a forged completed bit or automatic retry. A maintenance
waiter's cancellation does not shorten a previously admitted ordinary job's own
deadline or revoke its required original settlement continuations.

An expired caller receives the available truthful status. The coordinator keeps
one retained completion route; caller reconnect cannot create another waiter/job
or revive effect authority. Unexpected worker/owner loss is unknown, not no-effect.
Late completion may resolve physical uncertainty but cannot reopen ordinary gates,
revive an expired attempt or consume another job's slots. Explicit resolution and
validated release remain the accepted maintenance owner's responsibility.

## Cleanup-binding recipe

Before `spawn_blocking`, reserve the complete binding footprint and independently
register the exact original root/lease/deletion identity and completion destination.
The read-only binding slot creates no deletion authority. The blocking task owns
its capacity, captured handles and eventual result through actual return.

Success before expiry can transfer the bound owners and reservation into the
original `CapturedDeletion` recipe only after the same domain context is
revalidated. Transfer consumes its reserved slot; it cannot extend the roots or
start a fresh deletion. Stale/rejected context starts no file-removal stage.

On timeout/waiter loss, the coordinator remains able to observe the actual
binding result. Late success cannot publish a usable `CapturedDeletion` to the
expired caller or automatically run `remove_*`. It performs only the predeclared
safe disposal of its captured read-only bindings. Handle teardown and any blocking
close remain physically owned and charged until acknowledged; no timeout frees
still-live handles. Binding failure/partial acquisition reports the actual result
and disposes only the owner's acquired resources. If safe teardown or completion
is unknown, retain Blocked/protection ownership. No destructive cleanup, fresh
binding attempt or recovery I/O is smuggled into disposal.

The current two-second wait and intent reservation do not implement this retained
completion/result accounting. Binding, handoff and safe teardown need real owner
operations and peak-resource evidence before a bounded implementation can proceed.

## Recommendation settlement recipe

Admit/reserve before `begin_execution` or its checked variant, retaining original
recommendation/target/runtime/incarnation and strong root/lease ownership through
all physical input/preparation/acknowledgement/bookkeeping/graph results. Holding
that lease does not override currentness checks or authorize writes to a switched
workspace. The existing weak target/guard alone cannot provide this contract.

Input submission is a separate exact slot admitted at most once. Keep distinct
actual outcomes: known not enqueued/no input effect; known writer-confirmed result;
or partial/unknown. Use the real runtime owner's outcome semantics: a coarse
error, lost acknowledgement, panic or timeout is not proof of rejection after
enqueue. Unknown/partial delivery retains protection and cannot select graph
cancellation or completion by guessing. Missing outcome evidence blocks this recipe.
No settlement path sends input again, even after restart or matching hashes.

For a known domain outcome, settlement uses this fixed graph under the original
deadline and actual domain authority:

1. One nonblocking store-claim attempt. On success, perform exactly the original
   complete/cancel action selected by the actual delivery owner; retain physical
   write/publication resources to return. Non-BUSY errors report actual outcome;
   they do not retry filesystem effects.
2. Only if the claim is BUSY with known no effect, register one availability
   notification wait. Register-before-check prevents a missed release. Wait is
   outside application/store/registry guards and ends on notification, deadline or
   loss. A notification does not reserve ownership or prove successful settlement.
3. After a notification, at most one final nonblocking claim attempt before the
   same deadline. BUSY again, stale context or expiry records unresolved/Blocked;
   no second wait, automatic loop, timer renewal or input resend. Each successful
   claim still runs only its original effect slot once.

This requires an actual single-notification/deadline-aware owner operation;
`wait_until_available`'s existing loop does not satisfy it. Each BUSY attempt must
be known side-effect-free. Current graph operations may perform synchronous I/O
under existing domain checks; this proposal does not authorize moving them outside
those checks without a real captured owner/admission interface. No wait or module
call occurs under the maintenance registry lock, and physical waits cannot hold
application/module guards. Missing compatible domain ownership remains a blocker.

The store action and any best-effort delivery event retain separate outcomes;
bookkeeping/event failure cannot turn confirmed input into a rejected prompt or
trigger duplicate event/input publication. Caller loss keeps the original guard
and result/protection record retained. Workspace replacement or exhausted attempts
leaves resolution to the exact existing domain owner; it cannot finalize against
a different store. An explicit later resolution is separately bounded and
admitted under blocked ownership, never a hidden retry of this expired recipe.

## Conditional checks and handoff

Later actual-owner verification must cover reservation exhaustion before dispatch;
peak handle overlap; binding queued/running at timeout; late success/disposal;
lost waiter and worker panic; counter overflow; no-effect BUSY racing notification;
missed release/stealing next claim; final BUSY; expiry before/after enqueue/claim;
confirmed versus unknown input; event failure after graph settlement; switched
root and owner loss; hung teardown; and fresh explicit resolution without resend.
These are required oracles, not executed tests or a probe allowance.

Acceptance covers the three unchanged policy choices for further qualification. Actual
numeric limits/deadlines and enforcement, producer operations, delivery evidence,
full registry/native/external/restore readiness and physical fit stay unavailable
until qualified. The [qualification plan](../plans/2026-11-08-backup-maintenance-qualification.md)
remains Investigate; no checkbox closes merely because this graph is finite on
paper. A later bounded implementation plan needs the actual compatible owners,
resource values/enforcement and meaningful checks. No runtime implementation or
new execution budget follows from drafting or accepting this proposal.


## Review disposition

Fresh builder review found no material ownership or finite-graph defect in late
binding disposal, deadline/notification races or no-resend settlement. Independent
authority/source review found no material conflict with accepted cancellation and
progress rules, unknown delivery handling or unqualified numeric/physical inputs.
These reviews supply no execution evidence or actual producer qualification.
