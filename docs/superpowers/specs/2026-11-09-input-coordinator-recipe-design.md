---
type: Design Contract
title: Complete original input coordinator and admission recipe
description: Proposed application coordinator, closed input recipe and qualification gates covering preparation, physical delivery, bookkeeping and recommendation settlement.
tags: [orkworks, terminal, ownership, maintenance, qualification]
status: stable
workflow_status: accepted-bounded-implementation
---

# Complete original input coordinator and admission recipe

Tracking: [#1638](https://github.com/Rambolarsen/orkworks/issues/1638), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250). Source pin:
`5bc01fb75a6ae6f1d4a1362d280a035617448fe8` (worktree base; no runtime source changes in this task).
Consumes the implemented [scope retention](../plans/2026-11-09-input-scope-retention.md)
and [writer reporting](../plans/2026-11-09-writer-reporting-bridge.md).
Reuse accepted [input truth](2026-11-08-terminal-input-outcome-interface-design.md),
[registration](2026-11-08-maintenance-registration-settlement-design.md) and
[bounds/settlement](2026-11-08-maintenance-settlement-bounds-design.md).
Their existing approvals stand. After #1641 the owner answered “keep going” to
the explicit design/private implementation question. [#1644](https://github.com/Rambolarsen/orkworks/issues/1644)
records acceptance, [reviewed bounded plan](../plans/2026-11-09-private-input-coordinator.md)
and code together. The three choices below are accepted for that exact scope;
no production values, activation or broader qualification approval follows.

## Outcome and accepted bounded choices

One application coordinator retains the original submission and all its declared
continuations until actual physical and application obligations settle. It uses
the existing Entry/Core, actual typed scope and unique phase reporters. There is
no second result ledger, polling joiner, path-keyed recovery map or new input
execution authority. Maintenance cannot mistake an empty selected workspace or
lost waiter for a drained original root.

The three new choices for owner review are:

1. An application-lifetime coordinator with bounded original-root registrations
   owns the existing accounting Owners independently of request tasks, selected
   workspace and runtime replacement. Original job admission includes its entire
   resource/continuation recipe before graph reservation or preparation effects.
2. The closed recipe below owns preparation, enqueue/write/physical return,
   bookkeeping/reset/label descendants and finite graph/event settlement, with
   one absolute deadline and gap-free transfers. Unsupported/unbounded descendants
   refuse admission; compatibility wrappers do not bypass the rich route.
3. After approval, allow one bounded private coordinator/recipe implementation
   with explicit finite fixture profiles and offline checks through the existing
   actual capture/record/writer/join operations. Production construction stays
   absent. Unknown producer adapters remain explicitly Unsupported; fixtures or
   manually settled slots cannot qualify real preparation, graph, model or native
   behavior. No new live/native/SQL/model/resource experiment or shipping default.

Choice 3 is an explicit new preparatory scope, not permission inferred from the
completed scope/bridge approvals. Acceptance, concise reviewed plan and code
belong in one task. Production activation is a separate qualified decision.

The first rich recipe here is recommendation delivery. Ordinary terminal input,
plan-review input and startup/editor producers are not implicitly qualified or
removed. Each actual writer family needs its own compatible complete admission
and completion recipe before a production root can be Open under the full catalog.
Keeping a legacy optional/reporting-None path cannot bypass that requirement or
supply maintenance evidence.

## Actual integration gaps, not another scope inventory

| Pinned source and operation | Consequence for the complete recipe |
| --- | --- |
| `session_application.rs`, `accept_recommendation_with_packet` | `begin_execution` currently precedes the owned input task. New coordinator/resource admission must precede that first durable reservation, including packet-checked variant and prompt override paths. |
| `runtime/terminal_runtime.rs`, `submit_input_with_admission` | Two per-runtime permits bound detached async owners, not total application jobs/roots, payload bytes, all queues or physical completion. Gate wait and `reserve_owned` currently have no common original deadline. |
| `runtime/session_runtime.rs`, `CONTROL_CHANNEL_CAPACITY`, runtime constructor and driver | Control channel has 64 entries; driver pending-command length is separately checked against that constant. One writer is in flight. These are logical queue controls, not an aggregate byte/allocator/native profile or permission to multiply/sum capacities into a fit result. |
| `terminal_runtime.rs`, `capture_pending_terminal_input` | Clones data into pending bookkeeping, may clone accumulated `peon.input_buf`, parse a full line and prepare identity reset before enqueue. No complete per-submission allocation/effect reservation. |
| `terminal_runtime.rs`, `record_terminal_input_impl`, `queue_label_hint_at_epoch` | Bookkeeping can grow accumulated input, mutate working/usage/topic state and retain a full line in a label hint. Display label truncation to 100 characters does not bound parser/hint storage or inference descendants. |
| `session_application.rs`, `schedule_prompt_identity_reset_retry`, `schedule_prompt_tuple_clear_retry` | Detached retries have backoff loops; a maximum delay is not a maximum number of attempts, original-root retention or physical return. Cannot be silently included as a finite slot. |
| `session_application/recommendation_delivery.rs`, `settle_recommendation_delivery` | Coarse delivered boolean and Busy/availability loop remain. They do not implement rich input truth or the accepted initial-claim/one-notification/final-claim graph. Event append is separate and best-effort. |
| `runtime/input_submission.rs` and `scope.rs` | Actual fixed records, retained checkpoints and original lease/runtime intent exist. Four phase reporters and scope fixture limits do not account for application settlement or all downstream effects. |

No new claim about installed helpers, physical teardown or complete root/module
coverage follows. Full authoritative native RootKey/parent/scope/incarnation and
startup/readiness ownership remain required, beyond an Arc lease/path match.

## Coordinator ownership and construction

The application service owns the coordinator outside WorkspaceState and all
HTTP/delivery tasks. It retains a finite set of original root registrations and
associated accounting Owners; switching selection never replaces that set.
Each registration must be supplied by the actual maintenance/module constructor,
with the same native RootKey/incarnation, retained owner/lease, catalog/profile
identity and readiness gate. No caller-created u128 Context, path hash, report
token, weak pointer or freshly acquired lease can construct a registration.

An unavailable/incomplete catalog, root identity, profile or readiness input
keeps production admission closed. Same-lease reopen preserves only the existing
scope comparison; it is not a fresh root incarnation or proof of readiness.
New roots cannot appear during Closing to route around the captured set.
Registrations cannot be removed/replaced while original work, protection or
required resolution remains. All root references are acquired before the last
source owner can disappear. Multiple-root work uses one atomic admission across
its exact original registered roots; no partial claims/effects on failure.
The current retained scope is single-workspace only. Multi-root jobs stay
Unsupported until an actual atomic all-root binding/reservation owner exists;
the private handoff below cannot fake that owner with matching Context integers.

Use one existing accounting Entry per original input, augmented with fixed
recipe/continuation/settlement state. Keep one source of input/phase facts; do not
copy snapshots into an independently mutable coordinator outcome cache. A bounded
coordinator index may route exact handles to registered Owners, but owns no
separate truth, growing history or new execution slot. Checked serial exhaustion
refuses admission; no rollover/reuse of a stale record or context.

The coordinator retains Owner/Entry even when no reporters or observers survive.
Reporter Drop is Lost, not slot release. Process/coordinator loss can still drop
in-memory handles: no immortal Arc, destructor completion or reconstructed live
claim. Restart remains blocked/unverified until actual recovery owners establish
state. Readiness/restore/SQL format selection are unchanged by this proposal.

## Atomic admission and lock boundary

Before `begin_execution`, reserve the entire recipe's peak resource vector,
original scope, fixed effect slots, unique completion/report routes and bounded
outcome/protection capacity under the same original deadline. Validate exact
recommendation/packet/target authority and current runtime/channel before effects.
The current two input permits/gate remain separately charged actual resources;
they cannot be acquired twice or substituted with an accounting slot.

Reservations may be prepared outside guards, then committed under a short
coordinator transition after domain context is revalidated. The transition
performs only fixed bookkeeping: no module call, lock acquisition into AppState,
filesystem I/O, wait, native call or callback. Existing projection/workspace/
session/module fences still govern their synchronous effect admission. Do not
hold the coordinator lock while acquiring those guards or running a module.
A unique pre-reserved ticket allows module admission and final coordinator
confirmation without an unowned gap; if domain admission races Closing, either
the original slot is already admitted and must settle, or it is refused before
any effect. The implementation must demonstrate that linearization explicitly.

Failure before durable graph reservation releases only unused reservations and
returns owned inputs intact. If graph reservation committed, later failure is
an original settlement obligation, never a fabricated no-effect aggregate.
Any initial domain lookup must itself be proven read-only or separately admitted
under the complete original recipe: a helper that reconciles/recover-writes while
reading cannot run outside admission merely because `begin_execution` is later.
Missing bounded lookup/recovery ownership is another production blocker.
Payload generation/override expansion must be bounded before this commit: count
actual allocated capacities and formatting/contract expansion peaks, not just
final UTF-8 length. Do not allocate an unbounded prompt/title first and call the
later admission check a complete memory bound. Legacy routes remain unchanged
until the qualified route can preserve their authorized behavior.

## Closed original recipe and exact completion owners

The recipe version fixes slot kinds: one lookup/recovery group, one domain
reserve, one preparation, one dispatch/write admission, the existing writer and
return reporters, one bookkeeping group, bounded reset/tuple subslots, one label
transfer, two graph claim attempts, one notification wait, one graph action, one
optional event and bounded disposal subslots. Reset/disposal maxima and each
module group's internal finite effect count come from its qualified profile;
Unknown keeps that branch Unsupported. They are not chosen dynamically by a
caller or enlarged by a worker. Fixture maxima are explicit finite test inputs.
All slots are predeclared for one job; branches permanently reject unused slots.
A slot is spent before dispatch, admitted once under the original deadline and
predecessor/currentness checks, and completed only by its actual original owner.
A boolean or caller-supplied receipt cannot discharge it. The four implemented
phase reporters remain physical facts; application continuations are separate
fixed obligations in that same Entry, not fictitious extra Completed phases.

| Slot/group | Effect and completion source |
| --- | --- |
| Lookup/recovery | Proven bounded read-only lookup, or an explicitly admitted original module recovery/read effect with actual completion. No implicit mutating preflight. |
| Domain reserve | One `begin_execution` or checked variant. Retain actual committed/no-effect/unknown result and reservation identity, including failure after possible commit. |
| Prepare | Existing sensitivity/pending-input/reset preparation blocking operation. Capture original runtime/scope, payload/parser/snapshot capacities and reset reservation effects. Completion is actual blocking-task return; skipped after expiry is not Completed. |
| Dispatch | One move of the same payload and unique writer/return pair into the exact original command/driver. Reserve queue storage/capacity and command disposition. No second send on callback error, ack loss, partial input or restart. |
| Write/flush and return | Existing optional reporting loop and `PendingPtyWrite::finish_join`; preserve actual checkpoints, terminal input, flush and own task join separately. Ack is notification only. Driver/worker loss remains protected, even after all bytes are known. |
| Bookkeeping | Existing returned preparation state, sensitivity/output boundary and actual input fact select the authorized one-time metadata/reset/working/topic effects. Actual blocking return plus per-effect facts, not mere scheduling, discharges them. |
| Reset/tuple continuation | Any required reset reconciliation must have exact original scope, fixed finite attempt slots, deadline and actual completion owner. Current unbounded retry loops are Unsupported until replaced/adapted under their domain contract; never spawn them from the qualified path as an uncharged fallback. |
| Label/Peon transfer | Reserve parser/hint payload and queue ownership before bookkeeping can enqueue it. A later inference is separately admitted by the actual Peon/module owner under the original registered roots, or a predeclared child with its own finite complete recipe. Transfer is acknowledged and charged gap-free; a hint insertion is not completed inference. Required original descendants retain the original deadline/Closing obligations even with a separate module recipe; they cannot become independent new jobs to escape those bounds. Unrelated future inference needs its own ordinary Open admission and cannot discharge this job. No model allowance is created here. |
| Graph claims/wait | Exactly initial nonblocking claim, at most one registered-before-check notification wait, then one final nonblocking claim, all under the same deadline. Only known side-effect-free Busy permits the wait. Existing looping wait cannot be reused as one notification. |
| Graph action/event | Exact original complete/cancel effect once; optional event once after actual graph commit. Separate committed/failed/unknown statuses; no event retry or rollback of known input. Actual domain owner retains I/O through return. |
| Teardown/resolution | Each physically owned resource has a predeclared bounded return/disposal route. Pending/failed/unknown ownership stays charged/protected; explicit later resolution is separately admitted, never an implicit continuation/resend of an expired job. |

Dispatch atomically spends the original input/write effect admission before the
one enqueue. Queue delay past the deadline does not turn that admitted write into
a fresh effect; write/return reporters describe the same original operation.
Physical completion reports are not new continuation admissions. No queued command
can use a new ticket or renewed deadline to become executable again.

Unsupported reset/label/module ownership must be detected by preflight before
any graph/preparation effect. Looking only at prompt text is insufficient: an
accumulated prior input line, current reset state or bookkeeping decision can
activate a descendant. If that dependency cannot be bounded/identified under
currentness checks, refuse the route. Do not suppress a required product effect
or silently change ordinary terminal label/reset behavior to make it fit.

## Rich result consumption, expiry and retirement

Use direct retained truth, not `Result<(), ()>` or the legacy delivered boolean.
Known full bytes never select cancellation/resend, including flush failure.
The original completion graph may be selected after relevant actual physical
completion and effect accounting; a failed phase can have physically returned
while retaining protection, so it is not coerced to successful completion.
Known no input effect permits cancellation only with preparation/bookkeeping
and their effects accounted for. Partial/unknown input, lost/unknown physical
ownership or unresolved preparation/graph effects remain Blocked/protected.
The original execution/target identity cannot be reassigned to another session.

The coordinator never performs a graph action against a replaced workspace or
store merely because it retained the old lease. Original target/domain authority
is revalidated at every effect admission. A stale target leaves the exact old
obligation for the existing qualified owner; it does not find the selected store.
An admitted graph commit and failed event retain distinct actual outcomes.

All queue/gate/notification/coordinator waits inherit the original absolute
deadline; shorter domain limits remain binding. Expiry rejects unadmitted slots
and ends waiting, never cancels an already admitted native or filesystem effect,
frees its last charge, renews the attempt or fabricates no effect. Late original
reports may refine facts but cannot admit an expired effect or reopen a root.
An unadmitted required continuation after expiry remains an explicit recovery
obligation. Maintenance waiter cancellation cannot revoke already admitted
ordinary work or its predeclared required slots during Closing.

Physical job settlement requires actual acknowledgements of every admitted
operation and permanent rejection/no-effect of unused slots. Record retirement
additionally requires all application/graph/transfer/recovery obligations consumed
or resolved and all relevant handles returned/dropped, using existing protection
rules. A failed/protected record is not evicted under capacity pressure. Release
execution resources only when their actual owner proves safe return/teardown;
retain the finite result/root/protection charges until qualified resolution.
No force-clear API, raw prompt diagnostic, new durable journal or automatic resend.

## Complete resource/profile gate

The production factory requires a qualified profile covering this whole recipe,
with actual constructor/owner identity and a closed set of supported branches.
No default profile, inferred successful empty catalog or partial fallback.

| Required bound/charge | Available evidence; missing qualification |
| --- | --- |
| App jobs, registered roots, records, observers, all continuation/result/protection slots | Candidate fixed record/observer and scope reservation operations exist. Application-wide counts, root/continuation capacities and shared-owner admission are not yet qualified. |
| Input bytes and every overlapping payload/title/session/path/runtime/parser copy | Scope capacities are checked locally. Complete prompt/override expansion, pending clone, accumulated input buffer, parsed line, hint/context, formatting/error/event and teardown scratch peaks remain unqualified. |
| Runtime permits, gate/command/pending/worker/completion storage | Existing two permits, 64-entry channel and pending length control are source facts only. Shared runtime/native resources, queue backing capacity/byte enforcement and peak handoff overlap need actual owner evidence. |
| Lease/directory/store/control/native handles and notification state | Strong lease/sender retention is implemented; native root/parent identity, physical writer/driver/store handles, one-notification operation and peak handle overlap remain owner-qualified inputs. |
| Deadline and bounded resolution | Candidate record expiry exists. Enforced queue/prepare/continuation/graph deadline operations, qualified duration and fixed cleanup/recovery attempt storage are not yet established. |
| Downstream reset, Peon, metadata/event and graph owners | Existing calls/loops above are real dependencies. Each must provide exact bounded admission/transfer/return semantics or the route remains Unsupported. Complete maintenance module registration is separate and still required. |

Each dimension enforces checked `used + reserved <= limit` before dispatch,
including transient overlap and outstanding rejected/captured inputs. Shared
charges have one actual owner and explicitly bounded references. Missing bounds,
arithmetic overflow, unsupported native/physical evidence or profile mismatch
refuses before the first effect. A hung physical owner may retain a finite slot
indefinitely; resource boundedness is not an OS completion-time guarantee.
Fixture limits and the source constants above are not measured production fit.
Obtaining new native/model/resource measurements needs its own bounded authorized
execution proposal; this document and a docs build grant no such allowance.

## Bounded implementation-qualification handoff after approval

One task implements the private coordinator recipe against explicit fixture
profiles, reusing the actual Entry/Core, captured scope and writer/join. Extend
fixed recipe/settlement state in the same record, reserve finite continuation
capabilities and refuse incomplete profiles before any adapter can dispatch.
No production factory/caller or arbitrary successful adapter is added. Use the
existing single-workspace fixture scope; unsupported multi-root ownership cannot
be asserted as a passing atomic-admission test. Offline tests use explicitly
non-authoritative fixture context/profile inputs, not a fabricated production
Open/RootKey/readiness certificate. No production registration constructor is
introduced by this preparatory scope. A module
without actual compatible ownership reports Unsupported and stays closed.

Fixture boundary (engineering choice in this proposal): pure internal state
transition tests may supply cfg(test)-only success/error/no-effect inputs to
exercise finite continuation branches. They prove local transition enforcement
only. Add no production fixture mode, successful placeholder module adapter or
public completion method accepting caller assertions. In tests through the real
capture/Entry/writer route, unavailable preparation/bookkeeping/graph/reset/label
adapters still refuse dispatch or remain pending/Unsupported; synthetic outcomes
cannot settle those real phase obligations or produce a whole-job/readiness pass.
This permits success-branch coverage without manufacturing physical evidence.

Meaningful checks must prove:

- Exhaustion/overflow/profile or stale-root/runtime refusal before domain reserve;
  rollback of only unused reservations; rejection retains owned inputs; fixed
  slots cannot be repeated, enlarged, renewed or rebound to another record/root.
- Atomic ordinary admission racing Closing, root switch and runtime replacement;
  retain original scope/Owner after request/observer loss and after all reporters
  disappear; no clearing of protected result/root capacity.
- Actual optional writer/checkpoint/join transfer, partial/full/flush error,
  panic/Drop and report failure with pending preparation/bookkeeping/graph facts
  visible as pending or Unsupported, never manufactured Completed values.
- Original deadline at each unstarted-slot admission, expiry after actual grant,
  late physical completion without new effects, and distinct physical/application
  obligations preventing premature retirement.
- Fixed initial claim/one notification/final claim state and no-effect Busy
  precondition; unsupported notification or graph owner refuses dispatch. Pure
  state tests qualify local transition enforcement only, not real store I/O.
- Reset/label descendant preflight refuses unsupported state before preparation;
  no legacy infinite retry/model call/hint insertion fallback. Add actual adapter
  checks only once its bounded owner contract is available, not a fake success.
- Full repository verification, current-code manual review and CI; no production
  bound factory/call site. Update the canonical qualification plan with exactly
  the demonstrated local evidence, leaving full producer/native/resource boxes open.

Pass is a concrete private coordinator with an enforced finite original recipe
and real existing capture/writer retention, not production readiness. The next
qualification step must address actual missing preparation/bookkeeping/reset/
label/graph owner operations and numeric/physical inputs, rather than repeating
this recipe or opening admission with Unknown. No separate acceptance-only PR.

## Proposal review disposition before acceptance

Fresh builder review identified one material boundary: successful fixture
transition coverage versus unavailable actual adapters. Resolved here as an
explicit engineering choice: cfg(test)-only local transition inputs are allowed,
while real-route tests keep unavailable phases pending/Unsupported and cannot
manufacture whole-job completion/readiness. No production fixture mode or
successful placeholder adapter is authorized. This clarifies the proposed scope;
it does not qualify a producer or change an earlier accepted contract.

Independent source/authority review found no actionable material findings against
the exact source pin and accepted input/registration/bounds contracts. Existing
semantics and completed code evidence are reused; no runtime or physical
qualification is performed by this proposed document.
