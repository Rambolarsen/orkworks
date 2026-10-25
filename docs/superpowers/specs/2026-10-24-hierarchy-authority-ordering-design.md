---
type: Design Proposal
title: Hierarchy write authority ordering and revocation tradeoff
description: Accepted scoped authority-claim tradeoff for further design, preserving ordinary stop independence and explicit lease, recovery and implementation gates.
tags: [orkworks, hierarchy, authority, sqlite, recovery]
status: draft
---

# Hierarchy write authority ordering and revocation tradeoff

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `10403df761232450c3018fc0b057ae2253338a37` (PR #1030).
**Scoped #744 authority amendment accepted for further design/qualification;
not an executable handoff.** The owner explicitly answered “yes, go ahead” to
the full claim/lease/retirement tradeoff reviewed in PR #1031 at
`eaa426910170a73e572d1bc2e4a91f087cf11989`. See the
[acceptance record](#owner-acceptance-and-handoff--2026-10-24).

The [storage contract race](2026-10-24-hierarchy-storage-contract-design.md#authority-and-commit-ordering-explicit-no-go)
is addressed by an explicit scoped policy amendment, not by proving the former
revocation-before-physical-commit guarantee. #744's
[atomic-write contract](2026-10-04-assignment-evaluation-design.md#authentication-exact-retry-and-atomic-writes)
now uses claim/revoke ordering only for the listed publications. Excluded actions
retain their existing cutoff and no-go conditions. Native ordering, lease and
resource proof remain required. SQLite/private-worker direction acceptance,
proposed numerical limits and #740's November/no-go gate are unchanged.

## Decision in plain language

**Owner-accepted narrow amendment:** immediately before the private worker starts
one exact #744 producer mutation, let it claim that operation's authority under the
same short in-memory ordering as revocation. If revocation wins, deny the new
write. If the claim wins, that one bounded operation may still commit after
stop/cancellation. Revoke the bearer immediately for every new request, queued
operation, receipt read and further claim. A claim is neither a successful
receipt nor a promise that storage will finish promptly or succeed.

This preserves responsive ordinary stop and a single SQL transaction for
source/receipt/dependency/summary publication. It explicitly gives up the
stronger guarantee that **every revoke preceding physical commit denies that
write**, for this narrow claimed-operation class. At most one workspace command
can hold such a claim in this sidecar instance; queue admission is not a claim.
Already-claimed work cannot launch/resume a child, approve a plan, change scope,
perform cleanup/purge or restore run authority. Source/counter/retention/health
and predecessor rules continue to apply; permission alone cannot make an
ineligible operation succeed.

Example user-visible consequence: a child stops while its #744 candidate-submission
transaction is in progress. The run/child is stopped and has no result-reporting
capability. Its already-claimed candidate may nevertheless appear in retained
history once commit is reconciled. That is candidate admission only; accepted
result publication still requires the existing separate sidecar finalization
proofs. Cancel cannot promise that this one operation was undone. No success is
reported until the complete transaction is durable and the response is separately
authorized for delivery.

## Why a choice is needed

The current combination requires (a) runtime revocation to deny a later durable
commit, (b) commit as the durable mutation's linearization point, and (c) stop/end
to be independent of stuck SQL/fsync with no ordinary core lock across that I/O.
A last-moment check or interrupt supplies no shared instantaneous point between
revocation and the actual durable commit. SQLite documents that an operation
[may finish despite an interrupt](https://www.sqlite.org/c3ref/interrupt.html).
Its [rollback atomicity](https://www.sqlite.org/atomiccommit.html) concerns durable
old/new transaction state, not OrkWorks runtime authority or bounded I/O latency.
This is an inference about this proposed integration, not a claim that all
possible future native adapters are impossible.

| Choice | Authority ordering | Cost / disposition |
| --- | --- | --- |
| Keep current cutoff and demonstrate a native commit/revocation fence | Revoke before physical commit always prevents persistence | Remains legitimate, but no qualified mechanism is supplied. A callback/volatile flag is insufficient. Keep implementation no-go until evidence exists |
| Put revoke/stop behind the SQL worker, or hold the authority lock through I/O | SQL and revocation serialize through potentially stuck I/O | Reject as the recommended path: ordinary revocation/stop latency can depend on storage; any adoption needs an explicit broader responsiveness change |
| One exact operation claim before SQL | Claim/revoke order in memory; claimed operation may commit after revoke | Recommended bounded semantic amendment; requires owner acceptance and native/producer qualification, not a disguised preservation of the existing cutoff |
| Persist tentatively, decide acceptance by a later volatile publication CAS | Revocation may suppress visibility after bytes were written | Deferred: changes acceptance/recovery, adds crash ambiguity between persistent state and the volatile decision, and does not satisfy current persistence denial |
| Compensate after commit or replace a stuck worker | Already-persisted bytes cannot be uncommitted by a flag/timeout | Rejected as proof of the existing cutoff; no journal deletion, replacement worker or blind retry |

## Amendment scope and exclusions

The proposed claim applies only to new authenticated #744 child/parent candidate submission,
child evaluation submission, and exact Electron-authorized #744 user evaluation,
correction/replacement/invalidation writes. Their existing action, generation, key, target and owner bindings
remain exact. A user correction still cannot create an initial child report,
retarget an evaluation, reopen the run or impersonate a child. Final run completion
and other control commands are not producer publication claims.

It does not apply to #742 child/user research-report publication, child
reservation/allocation/spawn, plan approval, launching,
run creation, clarification/proposal transitions, evidence ingestion/clear,
retention/subject purge, ownership transfer, workspace deletion, shared learning
or arbitrary SQL. Those operations keep their own unresolved authority/transaction
contracts and must not inherit the exception just because they share storage.
The [baseline late-child-report rule](2026-09-24-taskmaster-bounded-coordinator-design.md#authentication-validation-idempotency-and-recovery)
permits audit-only retention rather than graph advancement. The newer #742
current-report-head contract has its own revocation, preparation and terminal-run
rules. Neither is amended here. #742 report publication remains no-go until a
separate ordering/reconciliation decision; it cannot use this claim class or
coerce a report into a #744 candidate. Missing strict command/row schemas also
still prevent an executable handoff.

Sidecar finalization is already distinct from producer authority: the
[accepted candidate contract](2026-10-04-assignment-evaluation-design.md#candidate-submission-and-sidecar-finalization)
allows a durable pending candidate to finalize after producer termination, with
exact predecessor, quiescence/immutable-artifact and recovery proofs. It does
not need or acquire a grandfathered producer claim. Resumption, changed bytes,
missing bindings or invalidated quiescence still reject it. This proposal does
not resolve the separate race between finalization and new writable ownership;
that ordering must be qualified before finalization code. Rehash-ledger commands
and durable rejection outcomes retain their source-owned authorization rules;
a claim cannot invent an unreviewed internal authority class.

## Proposed state and linearization points

Keep the new claim inside the private hierarchy publication module. It is not
a second credential, transport route, worker or user-selectable transaction.
The latch protects only bounded immutable references, authority state, the one
active claim and pending/health tokens. It performs no parsing, copying, hashing,
SQL, filesystem, network, waiting or notification callbacks.

| State / transition | Required behavior |
| --- | --- |
| `queued` | Existing bounded FIFO position/rate charge only. The caller may disappear; the waiter detaches. No grandfathered authority and no new source receipt |
| `queued → claimed` | Worker owns the one active slot, strictly validates fixed intent and current source bindings, then under the latch rechecks exact authority/generation/namespace, owned workspace lease and closing/health fences; atomically installs an exact immutable claim |
| `queued → denied` after revoke | Retire the FIFO position through the worker's normal rejection path, preserving source rates and any existing source-owned work ledger; no accepted receipt, source mutation or claim |
| `claimed → executing` | Release the latch before SQL. Recheck authoritative durable predecessor/quotas/dependencies inside the transaction; reject/rollback if they fail. The claim cannot be retargeted or widened |
| `executing → committed` | One SQL transaction installs source, receipt, payload, edges, quota and summary effects plus exact nonsecret authorization-decision metadata. Durable acceptance linearizes here; claim authorization linearized earlier |
| `executing → rolled-back` | Establish rollback before retiring position/claim; expose no accepted source receipt. No automatic retry or new claim, even if the original caller still exists |
| `executing → uncertain` | Keep the one worker/claim/position charged, fence affected reads/actions, and reconcile on that worker. Timeout, interrupt or lost response does not determine disposition |
| `committed → published` | Publish references/invalidate snapshots under the latch before retiring position/claim. Preserve any intervening runtime/health/workspace fence; never clear it using an old claim |
| Sidecar process exits | Volatile claims die. Replacement performs SQLite/source recovery; it resolves durable receipts but never recreates or re-executes a lost claim |

The new authorization point is the latch's `queued → claimed` transition; the
new source is accepted only at qualified durable commit. Revoke is the latch
transition closing the applicable authority. Revoke and claim cannot both win:
revoke-first denies a claim, claim-first permits only that exact attempted
transaction to reach a proven disposition. A repeated request needs fresh
current authentication and normal retry rules; a stored claim annotation grants
nothing to its sender or a restarted sidecar.

### Lock and caller ordering

Use the [proposed ordinary publication lock order](2026-10-23-hierarchy-transaction-storage-design.md#publication-and-ordinary-session-integration):
existing projection lock when required by the ordinary path, then the new
publication latch, then existing workspace/session/catalog locks needed by that
path. Do not acquire projection lock while holding the latch. Metadata-lane
writers announce before entering the lane and publish after releasing it.
Authority validation at claim must use the latch-owned immutable authority
record maintained by participating transitions, not acquire core locks in the
reverse order or call a handler while holding the latch.

Ordinary end/revocation publishes its close/fence under this short ordering,
then releases all such locks before existing PTY/metadata cleanup. It never
waits on the hierarchy worker/claim/SQL future or a durability callback. The
worker holds no core lock or publication latch across SQL/fsync. Existing code
does not yet provide this new authority record or universal transition coverage;
all paths in #1030's inventory need implementation-base integration and races.
Poisoned/unknown/exhausted authority state is closed, not a reset/new claim.

A claim freezes only permission for its exact intent. It does not freeze mutable
source eligibility, grant new artifact reads or bypass dependency/health rules.
Final source validation and changed-set discovery remain domain-owned; an
adapter unable to reconcile an intervening health/ownership change without a
stale current outcome remains no-go. That separate semantic/finalization ordering
is not solved by moving the producer authority cutoff.

## Lifecycle and ownership decisions

| Transition | Proposed exact treatment |
| --- | --- |
| Child/parent runtime end; run cancellation | Immediately close the relevant producer authorities under the latch; deny queued/later claims and all later response commitments. One already-claimed allowed publication may finish, keeping terminal lifecycle/grants revoked |
| Explicit final run finish/completion | Return a bounded conflict while a relevant producer claim or pending candidate makes settlement unresolved. Do not wait for SQL, declare settlement or silently queue finish. Existing ordinary runtime stop and explicit run cancellation remain available |
| Bearer renewal/resume | No claim transfer or continuation. Old generation stays closed; a new exact authorized runtime gets its own source checks and no extra request/receipt/revision budgets |
| Namespace rotation | Close admission first; do not complete rotation/new-epoch activation until the active old-epoch claim has a proven disposition. If it is unresolved, return bounded conflict/unavailable rather than reset keys or queue unbounded rotation. No new old-epoch mutation claim; already claimed work is resolved before old history becomes read-only under the completed rotation |
| Workspace switch | Immediately close old workspace admission and detach its publication scope. The sole worker may drain its one claim against the retained old lease. New workspace ordinary operation continues; its hierarchy storage is unavailable until this same worker has retired/closed the old store. No replacement worker or second hierarchy queue |
| Lease handoff/replacement sidecar | Never voluntarily release the old store's OS lease while any SQL/recovery I/O may continue. A different owner cannot acquire it until the old worker closes/releases or the old process exits. A path/PID marker is not ownership proof |
| Actual lease loss, uncertain ownership, broken locking or unsupported filesystem | No eligibility claim. Stop new admission and fence; interruption is not proof of rollback. The adapter must qualify exclusive ownership through disposition or stay no-go; this proposal supplies no permission to write through lease loss |
| Old command completes after a workspace switch | Persist only to its exact captured store/epoch under the retained lease. No new-workspace reference/event/snapshot publication. Reopening old history later requires normal complete recovery; it restores no bearer/claim |

These are accepted workflow requirements for further design, not assertions
about implemented or qualified behavior.
In particular, retaining a retired store lease can block another sidecar from
opening that old workspace indefinitely if the worker is stuck. There is no
bounded completion/lease-release guarantee and no automatic process kill.
The owner decision must include this cost; numerical SQL cancellation thresholds
do not prove timely retirement.

The lifetime cap is one active-or-retiring hierarchy worker/connection/claim in
this sidecar instance, including after repeated workspace switches. Do not start
another hierarchy worker or accumulate abandoned per-workspace command queues.
A later workspace may retain its ordinary metadata lease while the retired
worker retains only the one old lease/store/immutable command state. This adds
at most one retired workspace lease; it retains no old PTY/runtime or control
authority. A new workspace cannot enqueue hierarchy work around the retired
slot. Ordinary workspace lifetime semantics need explicit reconciliation before
code; this cost is part of the proposed amendment, not an existing lease feature.

Static precedent: [WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs)
(166–187) owns an advisory lock until Drop, and
[workspace adoption](../../../crates/orkworksd/src/session_application.rs) (3688 onward)
retains an Arc lease and revalidates filesystem identity. These facts do not
qualify the new retirement design. Existing generation-bound ending and
[clear_ended_session_tracking_for_runtime](../../../crates/orkworksd/src/session_application.rs)
revoke report tokens; they are integration roots, not a hierarchy claim protocol.

## Durable evidence and compatibility work

No source digest, wire receipt/action, authority generation or numeric quota is
changed by this proposed decision. Record the claim's nonsecret exact identity,
current workspace/sidecar/namespace/generation bindings, action/request digest,
producer-or-UI authority class and fixed target/predecessors as storage metadata
atomically with a successful mutation and its original receipt. Never persist
bearer bytes or treat that metadata as a reusable grant. It must be included in
whole-command/receipt/index/proof budgets and integrity joins.

Before code, close the exact tagged metadata shape, internal claim-ID generation,
canonical bytes and proof placement with #742/#744. If it extends a hashed
storage manifest/row format, version that storage schema/domain and add shared
vectors rather than reinterpret #1030's v1 bytes. Existing source-domain records
and receipts remain exact. A committed source with missing/invalid required
claim proof is unavailable; recovery cannot invent a past decision from a live
run flag, timestamp, highest revision or surviving artifact.

This acceptance record reconciles #744's atomic-write step, #742's references
to the #744 result/evaluation transport cutoff (without changing its research-report
protocol), coherent-read publication requirements and the proposed storage
cancellation/lease/worker lifetime contract together. It includes the reviewed
namespace/finish/retirement cost and strict action exclusions. No permission to
widen the exception to new action families follows from a shared database.
Exact proof metadata, versioned storage shapes and native integration remain
separate prerequisites; accepted source schemas/digests are unchanged.

## Required synthetic race and failure cases

These are written acceptance oracles, not executed tests or a native probe plan.

| Case | Required result under this proposed amendment |
| --- | --- |
| Revoke immediately before claim latch transition | Deny new mutation; no source/object/receipt; request-rate charge stays |
| Claim before revoke, then SQL commits | Exactly that immutable allowed mutation/receipt persists; run/runtime remains ended/cancelled, bearer revoked; no next claim or launch |
| Queue admitted before revoke, worker starts after | No claim, no grandfathered queue, no success receipt |
| Caller cancels while queued/claimed | Detach waiter only; current authority still decides an unclaimed request; claimed work reaches proven disposition without refund/replay |
| Claim wins, then predecessor/slot/quota validation fails | Proven rollback, no accepted receipt, no automatic retry; claim is not capacity/CAS approval |
| Child retry/read after runtime revocation | Deny before receipt lookup/response commitment; original receipt retained for existing authorized UI inspection |
| Commit before response, then revoke | Keep durable receipt; discard uncommitted response. Bytes already response-committed follow the existing read cutoff; no credential revival |
| Cancel races #744 evaluation write into terminal run | Permitted claimed evaluation remains about its exact retained result, subject to existing eligibility/CAS rules; no preparation/proposal/launch transition or terminal-run reopening |
| #742 report requests this claim class | Reject the action mismatch. Preserve its current cutoff and late-child audit-only/terminal-run rules; no report-head or graph-state exception |
| Runtime resumes while candidate submit is claimed | Never retarget candidate to new runtime; reconcile exact candidate/generation and pending-finalization supersession before any acceptance. No restored old grant |
| Namespace rotation races active claim | Close new admission immediately; unresolved rotation conflicts; complete old claim before activating replacement namespace, with old receipts still charged |
| Two workspace switches while old worker is stuck | Ordinary controls remain independent; at most one retired worker/lease/claim; no new hierarchy worker/queue or old publication into current scope |
| Actual lease loss during I/O | Fence and classify unsupported/uncertain; no claim-based ownership exemption or assertion that an interrupt prevented persistence |
| Crash after claim but before commit | Recovery sees complete old state or a complete committed transaction; no claim restoration or re-execution of an uncommitted request |
| Crash after commit before publication/ack | Recover original source/receipt/claim proof and lineage together; no new summary increment or rebuilt authority |
| Source health changes or finalization ownership becomes uncertain | Existing health fence remains until domain reconciliation; claim cannot clear it or assert current pass |
| Finish/purge/launch presented as a claimed publication | Reject action-class mismatch; no use of this exception for settling control state or deleting protected data |

Native execution still needs the separate approved qualification plan: exact
SQLite/library/VFS/OS settings, claim/revoke linearization instrumentation,
blocked-I/O/control-latency/lease retirement tests, crash boundaries and durable
source/receipt/proof/lineage joins. Mock races alone cannot qualify native
storage or prove all ordinary writers participate. All existing unqualified
journal/scratch/row/work/clock limits remain no-go conditions.

## Owner acceptance and handoff — 2026-10-24

The owner accepted **only the listed #744 producer/UI publications** finishing
after revocation when their one exact claim won first,
with immediate revocation for all queued/later work, unchanged durable acceptance
at SQL commit, bounded conflict for unsettled finish/rotation, and at most one
retiring worker retaining the old lease. Ordinary stop/PTY does not wait for it;
a stuck worker can keep hierarchy storage and old-workspace ownership unavailable.

The explicit acceptance followed the reviewed PR and named this tradeoff;
it is distinct from the earlier request to prepare a proposal or its merge.
Accepted scope includes child/parent candidate submission, child evaluation and
exact Electron-authorized #744 user evaluation/correction/replacement/invalidation;
research reports and every listed control/cleanup/evidence/shared-learning action
remain excluded. Sidecar finalization keeps its separate existing authority and
unresolved ownership ordering. Claims and their annotations restore no bearer,
launch grant or authority after recovery.

This acceptance settles the policy for further design/qualification preparation,
not an enforcement mechanism, proof schema, eligible maximum profile, numerical
storage default, native/model probe, executable plan, migration or runtime code.
The reviewed scope, state/lifecycle tables and failure oracles are unchanged.
Exact proof metadata, full domain schemas/budgets, writer/producer/lease and native
qualification, #740 eligibility and separately approved executable plans still
remain before runtime code. Keep #744/#746 open. Next bounded design work is the
strict nonsecret claim-proof metadata and versioned storage commitment mapping;
that new schema requires its own review and supplies no execution permission.


## Claim-proof schema proposal — 2026-10-25

The [strict proof/commitment proposal](2026-10-25-hierarchy-claim-proof-design.md)
defines candidate nonsecret ClaimIdentity/decision/proof shapes, receipt-owned
proof rows, format-2 hash/commit joins and recovery vectors. It changes none of
the accepted action scope, cutoff, lease/retirement cost or finalization authority.
The new schema/version choices remain proposed; no runtime/probe/migration or
native qualification follows from the accepted policy or this schema draft.

## Compact codec acceptance — 2026-10-25

The owner [accepted #1049's exact compact codecs](2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25)
for further design/qualification: proof-v2 exact local references, the three
claimed-mutation metadata rows and full-key retry lookup, canonical original
receipt bytes, scoped format-3 commitments and logical-byte accounting. This
supersedes acceptance-pending wording only for those named choices. Earlier
proof-v1/format-2 encodings and fixture bytes remain historical and unchanged;
no mixed parsing or implicit upgrade is accepted.

Source/request/wire fields, quotas and accepted claim/lease policy are unchanged.
No physical/whole-command qualification, remaining domain/reserved/finalization
schema/default acceptance, migration, probe, executable plan or runtime approval
follows. The linked record lists the remaining gates and next bounded metadata/
budget design work. #740 stays deferred/no-go until November; keep #744/#746 open.
