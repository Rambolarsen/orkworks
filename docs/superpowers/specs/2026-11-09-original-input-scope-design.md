---
type: Design Contract
title: Original input scope and application retention
description: Proposed strong original workspace and runtime scope on the existing submission record, with an offline implementation boundary before production integration.
tags: [orkworks, terminal, ownership, qualification]
status: draft
workflow_status: proposed
---

# Original input scope and application retention

Tracking: [#1627](https://github.com/Rambolarsen/orkworks/issues/1627), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250). Follows the
[implemented checkpoint/reporting bridge](../plans/2026-11-09-writer-reporting-bridge.md)
in #1620/#1625. Source pin: `0cad04787b6fb9800fd4ffd222f35285335ce228`.
Reuse accepted [input ownership and truth](2026-11-08-terminal-input-outcome-interface-design.md),
[registration/settlement](2026-11-08-maintenance-registration-settlement-design.md)
and [finite bounds/deadlines](2026-11-08-maintenance-settlement-bounds-design.md).
Those approvals remain valid; this new concrete attachment and preparatory
implementation scope require owner approval before code.

## Problem and evidence

The existing application task survives HTTP waiter loss, but it owns a delivery
guard containing only a path/recommendation key. Its workspace target contains
`Option<Weak<WorkspaceLease>>`. Upgrading that weak reference during matching
checks exact live lease identity; it does not retain the lease after workspace
replacement. An `Arc<AppState>` retains mutable application state, not the old
workspace removed from that state.

Relevant source, without redoing the unchanged producer audit:

| Source at the pin | Existing behavior and consequence |
| --- | --- |
| `session_application.rs`, `RecommendationDeliveryWorkspace`, `RecommendationDeliveryTarget`, `RecommendationDeliveryGuard` | Captures weak lease, runtime identity and control sender; guard tracks path/ID in a process-local set. Neither is a retained submission/root owner. |
| `session_application.rs`, `accept_recommendation_with_packet` / `deliver_recommendation_input` | Reserves Executing before spawning the outer owned task. Existing admission checks original workspace and recommendation state before send. |
| `session_application/recommendation_delivery.rs`, `settle_recommendation_delivery` | Matches the original target against current workspace before graph/event effects; Busy waits retain the target/guard. It still consumes a coarse boolean. |
| `runtime/terminal_runtime.rs`, `submit_input_with_admission` / `with_current_runtime_input` | Two existing runtime slots and the input gate retain the detached task; runtime identity/channel checks and the projection fence protect preparation/enqueue/bookkeeping. No real scope is attached to the accounting record. |
| `main.rs`, `WorkspaceState`; `workspace_runtime.rs`, `WorkspaceLease` | Workspace owns an optional strong advisory-lock lease; lease identity is the held file lock, not path/hash/file existence. |
| `session_application.rs`, workspace open preparation | Same-path reopen reuses the existing lease. New lease acquisition is separate. Lease equality alone is not a fresh workspace generation or complete maintenance registration. |
| `runtime/session_runtime.rs`, `RuntimeIdentity` | Runtime identity is the existing instance String and checked run generation; channel equality also matters. It is not an Arc identity or filesystem identity. |
| `runtime/input_submission.rs`, `Core` / `Entry` / `Reporter` | Fixed records retain facts; reporters/observers share Core. Context integers are opaque fixture bookkeeping, with no real root/runtime ownership. |

The optional writer bridge already has the required unique reporter pairing and
actual join operation. Keep that implementation. Do not add another outcome
cache, path-keyed map, joiner task or producer audit to solve retention.

## Proposed decisions

1. Attach one private, immutable strong original scope to an admitted record in
   the existing Core. Scope owns the original workspace lease and exact captured
   runtime/channel intent. Facts and observers never become execution authority.
2. Make application coordination, rather than caller/ack lifetime, responsible
   for retaining the record and scope through all physical and application
   obligations. Preserve existing currentness fences; retaining an old root does
   not authorize effects against a replaced workspace.
3. Approve one preparatory implementation of typed capture, bounded record
   attachment and actual offline retention tests. Production construction,
   RuntimeCommand routing, preparation/bookkeeping integration, recommendation
   result consumption and complete maintenance/resource qualification remain a
   later gated integration. Approval records and code belong in one task.

These are proposed engineering choices, not already approved behavior.

## Exact scope and capture

Use a private scope value containing the captured canonical workspace path,
strong `Arc<WorkspaceLease>`, original session ID, existing RuntimeIdentity and
original control sender. Reuse those types; no numeric hash conversion, token,
new runtime counter, new global identity registry or synthetic RootKey factory.
The candidate Context/slot/serial remains local record identity only. A bound
record must also have this typed attachment; matching Context values cannot
substitute for it or select a replacement attachment.

Capture through an application-private helper from the actual WorkspaceState
and live runtime under their existing authority/fence discipline. The helper
must verify the supplied session/runtime belongs to the captured current target,
including RuntimeIdentity and `same_channel`, before returning the scope. Missing
lease, missing runtime, stale identity or mismatched channel refuses capture.
The no-lease test fallback in the legacy workspace matcher is not available to
this bound path. Do not open/reacquire a lease from a remembered path or upgrade a
weak pointer later and call it original ownership. The helper clones the existing
strong lease while the source owner is still held. No filesystem I/O/await is
introduced under application guards.

The bounded preparatory slice exposes this operation privately for offline
calls; existing production acceptance/delivery does not call it yet. Tests may
construct actual application workspace/runtime fixtures with temporary advisory
leases and existing runtime identity/channel types, without starting a PTY or
harness. Exact source placement is a builder choice: keep AppState access in the
application adapter and immutable scope/retention in a private shared module.
Do not make the accounting mutex call back into AppState.

For bound-path comparison, require captured path and exact lease Arc identity,
plus original session, RuntimeIdentity and channel identity. Same-workspace reopen
with the same lease/runtime/channel remains the same captured target under the
existing rules; a new store instance is not silently a new maintenance incarnation.
A replaced lease or runtime/channel is different even if paths/session strings
match. This comparison is a currentness predicate, never an enqueue capability.

This attachment is a concrete retained workspace/runtime scope, not a complete
maintenance RootKey. Full native directory/parent identity, registered module
scope, root incarnation/readiness and filesystem replacement validation remain
required by the accepted registration contract. WorkspaceIdentity already owns
native directory identity during opening, but WorkspaceState does not retain
that object; this slice does not fabricate it from a path or claim to close that
gap. Production must establish the real root registration before binding jobs.

## Record ownership and lifetime

Install the immutable attachment in the same record admission transition that
allocates its unique reporters and bounded observer. No side table or per-phase
scope copies. Record lookup and reporter pair validation continue to use exact
Core/key/serial. Return the unused attachment and other consumed inputs intact
on admission failure; never replace/erase an older protected entry to make room.
Unbound fixture admission remains available only for existing component tests;
the new bound adapter cannot fall back to it.

The attachment lives in Entry, whose Core remains owned by the application
coordinator and the existing reporters/observers. Writer and return reporters
already retain that Core, so the same scope survives transfer through the actual
worker/join without adding a task, channel or separate root keepalive. The writer
and payload stay with their physical owners; holding a lease/sender does not own
a PTY, prove a receiver is alive or prevent runtime teardown. A sender reference
is accounted for and eventually released; channel liveness is not completion.

For future production integration the application root registration must own the
accounting Owner outside request tasks and workspace selection. It reserves and
attaches the original job before `begin_execution` or any delivery preparation,
then transfers the existing execution/reporting roles gap-free. A failure before
that reservation has no delivery/graph effects. Successful graph reservation
with later delivery failure remains the original job's settlement obligation.
The existing two runtime permits/gate are separate actual execution resources;
the new record is not a replacement permit or proof of a complete resource recipe.

| Event | Required retention and disposition |
| --- | --- |
| HTTP/caller, ack receiver or bounded observer disappears | Application retains Owner/Entry/scope; actual worker/reporters continue. No resend, new slot or deadline. |
| Workspace selection changes | Original scope remains attached. Currentness checks reject replacement-target effects; no lookup through current AppState to reacquire the old scope. |
| Same lease/runtime reopen | Existing target equality rules apply; this is not new readiness or permission to mutate another store. |
| Writer/driver panic, reporter loss, reporting failure, unknown native return | Existing fixed facts and protection survive as available. Lost phase never certifies native cleanup; known full bytes remain full. |
| Input/flush succeeds while preparation/bookkeeping or application settlement is pending | Retain scope and original obligations. Ack/full bytes/driver join alone cannot retire the job. |
| Deadline expires or cancellation requested | Stop waiting/request existing cancellation only; never renew or erase the obligation. Late original reports may still update it. |
| Known-safe record retirement | Existing terminal/phase/live-reporter/protection checks must pass, and coordinator must have discharged any separately owned application/continuation obligations. Remove attachment with the entry; release resources outside accounting/application locks. |
| Coordinator/process loss | Core references can retain scope only while they actually exist. Dropping the last reference releases in-memory objects; no destructor proves completion. Process restart has no recovered live claim and remains unavailable/Blocked until qualified resolution. |

There is no immortal registry, leaked Arc, force-clear, destructor success report,
persisted prompt or automatic restoration of a lost Owner. The offline slice
proves attachment lifetime while a coordinator or actual handle owns Core;
it cannot prove an as-yet-unimplemented production coordinator survives shutdown.

## Finite attachment accounting

A retained scope adds bounded path/session/runtime-ID allocations, one strong
lease reference, one control-sender reference and one shared attachment allocation
per admitted scope. No prompt, credentials, mutable stores, whole WorkspaceState
or AppState goes in the record or snapshot. Observers read fixed facts only;
inspection does not clone the scope/sender/lease or allocate diagnostic strings.

Use explicit finite fixture limits for attachment count, checked retained string/
path backing bytes and lease/sender reference counts in addition to existing
record/observer/backing/input limits. Check actual owned capacities and fixed
representation costs, including the new Entry field, before admission; overflow,
excess or unavailable reservation fails closed. Account for temporaries and error
returns in the bounded capture/transfer contract, not just Vec storage. Return
unused inputs without fabricating successful admission. Exact private limit/type
names and native path storage measurement are builder choices, with checked
platform-correct arithmetic and no lossy path-to-UTF-8 conversion for accounting.

Fixture values remain test inputs, not shipping defaults, a complete physical
fit profile or native-handle measurement. Full production recipe still includes
payload copies, preparation/bookkeeping, root registrations, actual permits,
queue/join/notification storage, protected outcomes, teardown/recovery scratch and
one enforced original absolute deadline. Missing evidence blocks production,
without shrinking existing obligations or admitting another original job.

## Bounded implementation handoff after approval

One child issue/PR records the explicit decision, a concise reviewed plan and
actual code. Scope: private application capture adapter, existing accounting
record attachment/retirement, scoped tests and relevant docs. No change to
production acceptance flow, RuntimeCommand, terminal admission/phase completion,
writer-loop semantics, graph/event actions, public API, storage format, startup,
module registration, shutdown, numeric defaults or production Owner factory.

TDD must use the actual candidate and optional writer bridge, not a parallel mock
ledger or manually completed physical-return receipt:

- Capture real temporary workspace lease plus actual runtime identity/channel
  fixtures. Reject absent lease, stale runtime and different sender; validate
  exact captured target and same-lease reopen behavior.
- Drop the source workspace and caller/observer, retaining the coordinator; a
  competing advisory lease acquisition still fails while the entry is retained.
  Replacing selection/runtime does not change the attached original scope.
- Hold the actual optional writer's flush, then drop the coordinator and waiter
  while physical reporters still exist. The old lease remains held and the real
  worker can publish its checkpoint/result through its original Core. Release
  test gates on failure and use actual join finalization for return evidence.
- Show lost/failed/pending phases or protected records cannot release the
  scope through retirement. Expiry alone never releases it; actual late safe
  settlement can still satisfy the unchanged retirement rules. A weak test witness can observe scope lifetime
  without adding an unbounded production observer API. Do not equate last-Arc
  destruction with physical completion.
- Demonstrate known-safe retirement releases the original attachment after all
  actual relevant component owners finish/drop. Any manually settled local
  fixture phases establish component lifetime only, not production preparation,
  bookkeeping, application settlement or producer qualification.
- Bound capture/admission and refusal paths: exact finite limits, checked overflow,
  path/string capacity and handle counts, full slots, expired deadline, duplicate,
  mixed record/pair and stale serial. Rejected owned inputs remain available.
- Verify production reporting stays None and no production bound capture or Owner
  factory is added. Run scoped/full repository checks, fresh current-code manual
  review and required CI with full comment inventory and owned-worktree cleanup.

This yields actual original lease/runtime retention through the existing record
and physical reporting handles. It does not close the whole application lifetime,
all-module registration, native/physical/resource/deadline profile, Q/readiness,
backup or restore gates. After this bounded code, the next integration decision
must address the complete production coordinator/recipe and preparation/
bookkeeping/graph consumption rather than repeating the attachment inventory.

## Review disposition

Fresh builder review found nothing materially unanswered for planning. The
existing admission error shape will need to preserve rejected owned attachments;
that behavior is already required here, not an unresolved scope choice. Independent
source/authority review found no actionable correctness or authority findings.
The source pin supports weak-lease matching, detached delivery, runtime/channel
identity and shared-Core retention; full root/resource/registration gates remain
separate. A self-check clarified that expiry alone cannot release a record, while
actual late safe settlement can still satisfy unchanged retirement rules. Both
reviews were read-only; no implementation or qualification is claimed.
