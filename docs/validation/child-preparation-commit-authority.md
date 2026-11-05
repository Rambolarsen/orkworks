---
type: Authority Investigation
title: Original child preparation commit authority
description: Accepted scoped original-preparation late-saving policy for further design, with representation and native qualification gated.
tags: [orkworks, hierarchy, preparation, authority, qualification]
status: draft
workflow_status: policy-and-representation-accepted-qualification-pending
---

# Original child preparation commit authority

Tracking: [#1410](https://github.com/Rambolarsen/orkworks/issues/1410),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), initiative #738.
Inspection base: `dbf9bd7ecd29bf85cacfa52e869632b9ff087dd5`.

The owner accepted the [three joint-preparation architecture choices](../superpowers/specs/2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05)
for further qualification. That decision does not change source write permission.
The current rule still requires current source authority at original preparation's
physical commit; no qualifying mechanism is established by this inspection.
This is a bounded integration finding, not an impossibility claim. No native
probe, database or model call ran. The existing cutoff remains binding unless
the owner explicitly accepts the narrowly described alternative below.

## Separate decisions and exact source owner

Original child-source-attach precedes any child runtime or evaluation capability.
Its authority is the eligible #742/original ordinary-lineage producer for an
already-reserved, successfully allocated attempt, with current exact source/run/
parent/admission authority. It is not an AssignmentEvaluationCapability, an
ordinary workflow-report token, an archived approval, a reservation ID alone or
a new launch grant for another attempt. The concrete source owner/representation
must still be closed before implementation.

The [accepted attach recipe](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#durable-command-effect-map--2026-11-03)
creates original attachment/source, prepared ordinary lineage/bridge, summary
branch and five assignment zero ledgers together. The ten reconciled run/workspace
peers are already prepared. No process starts before that complete outcome.
The [claim-first policy exclusions](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md#amendment-scope-and-exclusions)
leave source preparation/reservation/allocation/spawn and other source actions
outside evaluation claims. The accepted [artifact debit permit](artifact-read-work-authority.md#owner-acceptance-and-command-proof-handoff--2026-10-31)
permits one work debit only, not original preparation. Sharing a broker or a
short latch imports neither exception.

| Decision | Owning condition |
| --- | --- |
| Joint storage atomicity | Qualified attached rollback transaction proves complete old or complete new ordinary/hierarchy preparation; it does not establish permission |
| Original preparation write permission | Current eligible source/lineage authority through its accepted physical commit cutoff; currently unqualified |
| Native spawn | Separate current #742 admission after proven original preparation, exact generation and actual source/native ownership; a preparation result never grants a late launch |
| Evaluation registration/read/response | Actual runtime/bridge and original source evidence plus current exact authority and response cutoff; historical preparation grants none |

## Inspected mechanisms and limits

The following code is useful evidence of what a current mechanism checks; it
is not a production source publisher or an eligible format-5 implementation.

| Mechanism | Pinned source behavior | Why the source cutoff is not established |
| --- | --- | --- |
| Ordinary execution ticket | [worker.rs](../../crates/orkworksd/src/session_store/worker.rs), ExecutionRequest::grant, ExecutionGrant::execute and MutationCancellation::cancel, arbitrate cancellation against execution with an atomic state, then release application guards before Run | This orders ordinary worker execution, not source revocation against a later physical commit. Its semantics are not a bug to redefine or an accepted source exception |
| Final ordinary SQL check | [internal_mutations.rs](../../crates/orkworksd/src/session_store/internal_mutations.rs) checks intent.ready_at(clock()) after staging and then calls tx.commit | A readiness/deadline check before commit does not serialize all source revocations through the remaining engine/storage work. It is not an artifact/source authority proof |
| Hierarchy journal mechanics | [mechanics/connection.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs), journal_command, builds rows and commits under a test-only connection mutex | No original source authority owner or complete attach transaction is supplied. The database mutex must not be promoted into a stop/revocation lock |
| Evaluation authority | [publication_authority.rs](../../crates/orkworksd/src/taskmaster/publication_authority.rs), try_claim/settle, deliberately retains one exact allowed claim through later close | Factories remain synthetic/dormant. The accepted claim is evaluation-specific; source attachment cannot be encoded as ReadyEvaluation or granted by its token |
| Ordinary close routing | [lifecycle.rs](../../crates/orkworksd/src/taskmaster/publication_authority/lifecycle.rs) and runtime startup/driver close paths close an exact retained carrier before ordinary work | Close ordering is useful, but every production carrier is None and it supplies neither source issuance nor durable write permission |
| Proposed source fence | [Source admission design](../superpowers/specs/2026-11-05-evaluation-source-admission-design.md#proposed-private-interface) invalidates admission tickets and affected binding references under a short latch | A registration/currentness fence is not permission for source persistence after that latch is released. The separate source-admission choices remain owner-pending |

Reproduce the bounded code inspection with:

```bash
rtk proxy rg -n 'struct ExecutionRequest|fn grant|fn execute|fn cancel|ready_at|tx.commit' crates/orkworksd/src/session_store/worker.rs crates/orkworksd/src/session_store/internal_mutations.rs
rtk proxy rg -n 'fn try_claim|fn settle|fn close_binding|fn close_scope|register_source_binding' crates/orkworksd/src/taskmaster/publication_authority.rs
rtk proxy rg -n 'transaction|commit\(|journal_command' crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs
```

These are scoped reads of the named owners and contracts. No symbol absence
proves that an external/future mechanism is impossible. Ordinary SQL producer,
rollout and native eligibility gates are not passed by the source inspection.

## Engine candidates and concrete race

SQLite's [commit-hook documentation](https://www.sqlite.org/c3ref/commit_hook.html)
allows a hook to reject a commit and says a successful hook lets COMMIT continue;
it supplies no contract that later application revocation must abort the remaining
commit. Its callback also cannot execute other SQL on the connection. Inference:
a source check confined to that callback is not evidence of the required
cross-thread revocation cutoff.

SQLite's [interrupt documentation](https://www.sqlite.org/c3ref/interrupt.html)
explicitly permits a nearly completed operation to finish despite an interrupt.
The [progress-handler contract](https://www.sqlite.org/c3ref/progress_handler.html)
concerns callbacks during statement/preparation execution; it is not a guarantee
of prompt interruption of blocked filesystem I/O. These are useful cancellation
mechanisms, not proven rollback or a source-authority interlock. Native
instrumentation might establish a stronger integration, but no such evidence is
claimed here.

A concrete precheck/callback-only schedule is:

1. Source owner validates the current original attempt and complete immutable
   joint intent; its last source check or commit callback returns successfully.
2. The worker continues commit/storage work without a publication/core lock.
3. Stop, run cancel, namespace close or workspace detach wins the short source
   revocation cutoff. Ordinary controls proceed.
4. The original transaction reaches its durable commit, with both ordinary and
   hierarchy preparation present. A later response check rejects delivery.

Joint atomicity makes step 4 complete rather than partial. It does not make
step 4 authorized under the current source rule. Moving revocation behind the
worker or retaining a core/publication latch through SQL/fsync would change the
accepted ordinary-control responsiveness boundary. A future native fence remains
a legitimate candidate, but must prove the actual commit/revocation result and
physical lease/health/auxiliary-file behavior, not just run a callback earlier.

| Candidate | Disposition under current policy |
| --- | --- |
| Ordinary worker ticket or final pre-SQL/readiness check | Does not prove source revocation-before-physical-commit denial |
| SQLite commit hook, progress callback or interrupt alone | API guarantees do not close the demonstrated gap; an observed callback/interrupt is no rollback receipt |
| Attached-database atomic transaction | Candidate joint durability, not source permission |
| Hold source/core/publication lock or delay ordinary stop through commit | Reject: violates the accepted immediate close/stop and no-lock-through-I/O contract |
| Separately qualified native source commit/revoke fence | Retains the stronger rule if demonstrated; no such qualification is supplied here |
| Explicit one-command original-preparation permit | Proposed policy change below; not accepted by the architecture decision or this investigation |

## Owner-pending alternative: one original preparation permit

The author recommends considering this narrow policy alternative for further
design and qualification. It is a new source-specific exception, not a proof
that the existing stronger cutoff has been implemented. If the owner declines,
retain the current rule and the implementation block until a native fence is
qualified. Neither option authorizes a probe or executable adapter by itself.

Immediately before mutation, the sole private worker could acquire one opaque,
immutable original-preparation permit under the same short cutoff as every
applicable source revocation. Revoke-first denies it. Permit-first allows only
that already-authorized child-source-attach transaction to reach proven joint
commit/rollback after later volatile authority revocation. Durable linearization
remains the joint physical commit. This explicitly gives up revoke-before-commit
denial for this one preparation only.

The decision binds the exact original source/lineage authority and generation,
selected workspace and both database epochs, namespace/run/plan/approval,
reservation/frozen task version, proven existing allocation, P/configuration,
unique ordinary session/attachment and complete immutable ordinary/hierarchy
intent/read-set/effects. The original attempt must already be eligible when the
permit is acquired. Queue admission, stale approved bytes or the original
reservation alone cannot issue it. It creates no new reservation or launch attempt.

The permit freezes only persistence permission for that exact intent. All
transactional source/control/header H, ordinary epoch/generation G, membership,
quota, health and expected old/new checks still apply; drift denies/rolls back
rather than refreshing the intent under the old permit. Actual lease loss,
uncertain ownership, corruption or an unqualified filesystem remain unavailable.
No old source action can become current merely by holding a permit.

Do not reuse #744 ClaimProofV2, ReadyEvaluation, the debit permit, an ordinary
ExecutionGrant or a parent/child bearer. No new bearer, queue, worker, connection
or numerical allowance is proposed. Under the cutoff install only prevalidated
immutable references and scalar state; no SQL/fsync/parsing/copy/hash/callback or
ordinary lock acquisition. Immediate stop/close continues independently.

**Explicit exclusions:** no preparation of a fresh root/profile or missing peer
ledgers; no propose/approval/reserve/allocate, filesystem mutation, spawn,
runtime/bridge update, registration, grant restoration, recovery rewrite, purge,
read debit, content fetch or response commitment. The permit is not reusable
for a second SQL attempt. Proven rollback retires it; any later legal retry needs
fresh current source authority and existing exact-attempt rules, never a new
reservation or automatic relaunch. Unknown disposition retains the existing sole
active-or-retiring position/resources; no replacement worker or timeout reset.

The successful transaction must retain nonsecret authorization-decision evidence
bound to the exact original intent and complete paired old/new outcome. Its exact
canonical codec, placement, domain/version compatibility, accounting and retention
need a separate closed representation review before dependent fixtures. No such
codec, row, tag, format change or authority acceptance is selected here. The
currently accepted format-5 architecture is not silently modified. A manifest's
null evaluation claim is neither that future evidence nor permission to proceed.
Never persist credentials or reconstruct a volatile permit from a saved proof.

### Visible consequence and lifecycle cost

If permission wins and cancellation follows, the complete approved but unstarted
child preparation may still be saved: an ordinary creating record/lineage,
original hierarchy attachment/source/bridge/summary and five zero assignment
ledgers, with their exact owning C. Source/control/summary retained charges and
logical rows remain consumed; this event creates no evaluation/report receipt,
result, artifact or read-work charge. The already-existing allocation/reservation
remains the original one; no new worktree or process is created by this permit.

Prepared child versus unattached reservation still counts once under the accepted
capacity derivation, never twice. Cancellation does not refund or erase lifetime
source history. No source grant, UI action or orphan cleanup may label unknown
preparation as rollback or erase its connected custody. A later ordinary terminal
observation or source recovery remains separately authorized and source-owned;
this permit does not perform that second event or fabricate its bridge proof.

No startup future/SessionHandle/native launch may be scheduled from a historical
PreparedChild after authority closed. Fresh spawn admission remains mandatory
after proven preparation. A lost or denied response can therefore leave durable
unstarted history. Ordinary history may show the complete creating row under the
accepted visibility rule; it is not a live child or approval to resume it.
Only applicable current lookup authority may disclose its historical result.

On workspace change, close/detach the old scope immediately. The worker retains
its exact old paired databases/leases through disposition and cannot publish
references into the new workspace. The accepted broker can keep old SQL service
busy/unavailable while retiring; no second hierarchy worker bypasses it. Namespace
replacement cannot reset accounting or activate an epoch around an unknown old
command. Actual lease handoff waits for owned I/O disposition; the proposal
authorizes no writes through actual loss of ownership.

## Complete-state and denial oracles

Written qualification obligations, not executed fixtures. Compare full ordinary
metadata/index/history generation, hierarchy source/control/summary/lineage,
all five new assignment rows and ten unchanged peers, source/control charges,
complete consumer/proof/manifest/header effects and actual native process state.

| Schedule / result | Required oracle |
| --- | --- |
| Revoke before permit | No permit, backend call, ordinary child, hierarchy preparation or spawn; original reservation/allocation remain as their policy requires |
| Permit before revoke, valid unchanged prestate | Only the one exact complete paired preparation may commit. Revoked source/run/runtime/namespace authorities remain closed; no future/startup/spawn, new response commitment or registration follows |
| Changed source/H/G/expected identity, wrong store or second consume | Deny/proven rollback; no widened key, refreshed plan or second SQL attempt under the old permit |
| Proven rollback | Complete old paired state and original source attempt; no new zero rows/summary/charges/C/G advance, no permit reuse or inferred new child attempt |
| Commit then lost response | Complete original paired outcome and decision evidence; exact retry under current lookup authority returns original identity/C with no new row/version/charge/grant/spawn |
| Timeout, task Drop, disconnect, interrupt or unknown commit | Retain exact intent/position/leases/resources and unavailable fence until proven old/new disposition; no rollback assertion or automatic retry |
| Actual lease loss, unsupported locking, corrupt/missing peer or partial state | No eligible adapter; recover only through supported original disposition. Never recreate an empty peer, backfill ledgers or treat one matching digest as proof |
| Workspace/namespace replacement while claimed | Old state only under captured ownership; detach publication immediately, preserve newer fences, no new epoch/accounting reset or replacement worker |
| Source authority remains current under stronger alternative | A native implementation must prove revoke-before-physical-commit denial without delaying immediate ordinary stop; engine atomicity alone is insufficient |

## Scoped decision and finite handoff

The accepted broker/lineage/tombstone architecture is settled within its scope;
no repeat decision on those choices is requested. The pending decision is only:
retain the stronger source cutoff and its current implementation block, or
accept the one-original-preparation permit's explicit late-saving/no-spawn
tradeoff for further design and qualification. No other source action inherits it.

After a scoped policy decision, close the command/authorization-evidence/
compatibility/accounting representation against the accepted original source
contract and complete new profile. Reuse these oracles; do not invent a generic
source permit or treat the ordinary worker API as the source publisher. Full
catalog/physical membership, ordinary activation, all relevant source/runtime
writers and separately approved native/executable qualification remain gates.
#1410 delivers the investigation/acceptance record; #1176 remains open.


## Bounded review

One bounded read-only lower-cost round checked contract/source correctness and
complete failure/lifecycle consequences independently. Both reviewers approved
without a required correction. The author applied the builder's minor wording
clarification naming source/run/runtime/namespace authorities in the late-commit
oracle. Neither review established native behavior, a complete source owner or
an eligible profile. No new review round, implementation, permit acceptance or
native test result is claimed.


## Scoped original-preparation policy acceptance — 2026-11-05

After the read-only review of [PR #1412](https://github.com/Rambolarsen/orkworks/pull/1412),
the owner answered “go ahead” to: “Approve this one-preparation persistence
exception for further design and qualification?” The review explicitly disclosed
that revoke-first denies; permit-first may retain the one already-authorized
preparation after cancellation, including unstarted history, storage charges and
held capacity, with no spawn, capability restoration or automatic retry.

This accepts the one original child-source-attach persistence policy described
above. Immediately before mutation, the sole private worker must win the same
short cutoff as the applicable source revocations for that exact complete
immutable ordinary/hierarchy intent. Revoke-first issues no permit or preparation
write. Permit-first may reach proven joint commit/rollback after later volatile
revocation. Durable acceptance remains the joint physical commit. This explicitly
relaxes revocation-before-commit denial for that one original preparation, not
for any other source action or a second SQL attempt.

All source/control/header H, ordinary epoch/generation G, membership, quota,
health, ownership and expected old/new checks still apply. Changed facts deny or
roll back; a permit cannot refresh its intent, override actual lease loss or make
an unsupported filesystem eligible. Source/run/runtime/namespace authorities stay
revoked. Ordinary stop/input remains independent. Unknown disposition retains the
same sole active-or-retiring position/resources until proven disposition.

The complete prepared child replaces its unsettled reservation in derived
capacity, counted once. Source/control/summary bytes and original lifetime history
remain charged; cancellation supplies no refund, inferred rollback or cleanup.
No new reservation/allocation/worktree/process, startup future, runtime/bridge
update, registration, grant/read/response, recovery rewrite or purge is authorized.
Fresh spawn admission and every other source writer retain their own rules.

Earlier pending-decision wording is historical only for this precise policy.
[#1415](https://github.com/Rambolarsen/orkworks/issues/1415) owns the separate exact
authorization-evidence/intent/row/lookup/commitment/accounting/retention and
compatibility proposal. Its new representation choices are not accepted here.
No ClaimProofV2, debit permit, ordinary ExecutionGrant or saved record becomes
this live permit; no codec/row/tag/version, bearer or numerical allowance is
selected by this decision record. Align the corresponding ADR/closed profile
before dependent implementation.

This approval permits further design and qualification planning. It does not
authorize product code, SQL/database/schema/migration, a native/model experiment,
production registration, a verified profile or activation. Complete catalog/
physical membership, ordinary producer/cutover, all other source/runtime writers,
resource/native evidence and separately approved executable plans remain gated.
RegistryRoot remains a separate proposed mechanism; #1176 stays open.


## Authorization-evidence proposal — 2026-11-05

[ADR 0107](../adr/0107-original-child-preparation-permit.md) aligns the scoped policy
with the earlier ADR. The [authorization-evidence proposal](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md)
defines the original control snapshots, exact immutable intent, one retained
P-keyed recovery row and fresh format-6 placement under existing source budgets.
These representation choices remain owner-pending; no implementation or native
qualification follows. The source-admission interface was separately accepted
through #1414; RegistryRoot remains proposed. #1176 stays open; #740 stays closed.


## Representation accepted; qualification remains — 2026-11-05

The owner [accepted the reviewed authorization record, immutable recovery row and
format-6 placement](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05)
for further qualification after #1420. Earlier representation-pending wording is
historical for those unchanged choices; the original investigation and scoped
policy stand. ADR0107's dated amendment records this without a new policy.
The canonical proposal now lists finite catalog/membership, ordinary producer,
actual source-owner and physical/resource evidence obligations. No native result,
SQL/database/migration, fixture, executable plan or activation follows from this
acceptance. RegistryRoot remains independently proposed; #1176 remains open.
