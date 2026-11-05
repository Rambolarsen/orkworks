---
type: Design Proposal
title: Original child preparation authorization evidence
description: Proposed canonical permit decision, original control snapshots and immutable recovery row for the accepted late-saving/no-spawn policy.
tags: [taskmaster, hierarchy, preparation, authority, recovery]
status: draft
workflow_status: representation-owner-review-pending
---

# Original child preparation authorization evidence

Tracking: [#1415](https://github.com/Rambolarsen/orkworks/issues/1415),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`260f84cd07cd3a58919eb2c12c5d797a732e5cbe`.

## Outcome and scope

The owner [accepted one original-preparation permit](../../validation/child-preparation-commit-authority.md#scoped-original-preparation-policy-acceptance--2026-11-05)
for further design: an already-authorized complete child-source-attach may finish
saving after later volatile revocation, retaining history/charges but never
starting a child. This document proposes its evidence representation, not another
policy decision. The accepted broker/lineage/tombstone direction and #1398
source-admission interface are reused. #1414's newer RegistryRoot choice remains
independently proposed; no implementation mechanism is accepted by this file.

Keep the accepted child source/control schemas, ordinary schema-2 lineage member,
source/request/receipt/claim/debit domains and all limits unchanged. Do not reuse
ClaimProofV2, DebitIdentity, an ordinary ExecutionGrant, a producer capability or
a parent bearer as this permit. No new bearer or client idempotency property.
Proposed fields/table/domains below need one scoped representation decision before
dependent fixtures or code. No database, SQL, migration, native/model probe,
source registration, runtime activation or executable adapter plan is delivered.

This first representation covers the original, successfully allocated,
single-task-retained-data-execution-v1 attach under its still-current original
parent runtime. It supplies no UI/recovery/resumed-parent authority variant.
Broader legal continuations keep their own source-owner/representation gate;
a new runtime cannot adopt an unknown old attachment or reconstruct preparation.

## Canonical identity and evidence body

Use existing strict canonical UTF-8 JSON C(x), unknown/duplicate-field rejection,
ID/Digest, ChildAssignmentIdentity, ObjectRef, CommitRef, RuntimeRef, source
integer revisions and canonical u64 Counter TEXT. Source JSON numbers never
become Counter strings or rounded JavaScript values. Source bodies keep their
own exact codecs/domains. CreatedAt below is descriptive ordinary creation data,
not the time or proof of an authorization decision.

Propose PreparationPermitIdentityV1 exactly:

```text
{sidecarGeneration:Digest, sequence:positive Counter}
```

It uses its own checked sidecar-wide preparation-permit sequence, distinct from
evaluation/debit identities despite the shared shape. Consume it once at the
successful permit/revoke transition, including an attempt that later rolls back.
No reset on workspace/namespace/resume, maximum-row election, automatic restart,
wrap or restored permit. Use the same qualified fresh sidecar-generation owner;
unknown entropy/ownership or sequence exhaustion denies admission while ordinary
controls continue. One shared hierarchy position remains active-or-retiring;
there is no position or worker per permit class.

Propose OriginalPreparationIntentV1 with exactly these fields, none nullable:

```text
{workspaceId:ID, storageEpoch:Digest, admissionEpoch:Digest,
 expectedCommit:CommitRef,
 ordinary:{databaseEpoch:OrdinaryDatabaseEpoch,
           historyGeneration:Counter, createdAt:Text},
 assignmentIdentity:ChildAssignmentIdentity,
 authority:{kind:"original-parent-source", parentRuntime:RuntimeRef},
 attachment:{childSessionId:ID,attachmentId:ID,attachmentDigest:Digest},
 producerSource:ObjectRef,
 controls:{run:RunControlRead,plan:PlanControlRead,
           task:TaskControlRead,launch:LaunchControlRead}}
```

OrdinaryDatabaseEpoch is the ordinary header's exact canonical lowercase UUID
encoding (8-4-4-4-12 hex digits), not a hierarchy Digest or credential.
historyGeneration is canonical Counter TEXT constrained to 0..9223372036854775807,
the existing ordinary signed generation range. createdAt is the exact nonempty
ordinary created_at string through the existing 2 KiB Text ceiling; it is never
normalized/recreated during historical validation. The full canonical ordinary
record still obeys its independent 1 MiB limit.

RunControlRead, TaskControlRead and LaunchControlRead are the exact tagged full
snapshots in the [accepted control grammar](2026-11-02-child-launch-control-design.md#original-coordination-result-and-recovery-decisions).
Add the corresponding closed metadata snapshot PlanControlRead exactly:

```text
{table:"child_plan_controls", key:PlanRef,
 value:{schemaVersion:1,body:ChildPlanControlV1,lastCommit:CommitRef}}
```

Reuse each control wrapper's existing ceiling, including the 1 MiB run wrapper,
16 KiB plan/task and 32 KiB launch wrappers. Snapshots are decoded canonical
objects, never quoted JSON or arbitrary table/key/value lists.

OriginalPreparationAuthorizationV1 is exactly:

```text
{schemaVersion:1,permit:PreparationPermitIdentityV1,
 intent:OriginalPreparationIntentV1}
```

Its digest is SHA-256 of orkworks.child-preparation-authorization.v1, one LF,
then C(body). These bytes record the original nonsecret decision and source
semantic intent, not a raw request/SQL log or a portable grant. No credential,
wall-clock authorization claim, machine/process/account label or hidden reasoning.

## Required joins and actual authority

Only the qualified original source/lineage owner constructs the intent. Before
the child exists, its authority comes from the exact eligible #742 original
attempt and actual current parent/source runtime, not from a child capability.
A matching RuntimeRef, active plan or reservation alone is insufficient: the
qualified owner must check actual volatile original-attempt authority and every
applicable revoke/close/source condition at the permit cutoff.

| Binding | Required exact relation |
| --- | --- |
| Scope | All source/snapshot keys resolve P.workspaceId/runId and the selected owned store/namespace. expectedCommit.storageEpoch equals storageEpoch and resolves the exact pre-transaction H in that workspace. The admitted run definition supplies admissionEpoch; no caller-selected run or namespace |
| Parent | authority.parentRuntime is kind parent, sessionId=P.parentSessionId, and equals both controls.run.value.body.parentRuntime and controls.launch.value.body.admission.parentRuntime. Its sidecarGeneration equals permit.sidecarGeneration. Resolve original immutable runtime history/digest and actual current native/source authority. No PID/session label/archived approval substitutes |
| Original attempt | Resolve P's exact configuration, plan/input/approval, successful reservation/frozen assignmentTaskVersion and proven original allocation. launch phase is allocated, its attachmentId and spawnRuntime are null and it has no interrupted/unknown outcome. The plan is active, task launching, run executing; all other source-owner eligibility/blocker rules still apply |
| Frozen versus live versions | P.taskVersion equals the reservation and launch admission's frozen assignmentTaskVersion. Captured live taskVersion stays whatever the original control owner produced; never recapture it into P or force it equal to the frozen value. All live control versions/read rows revalidate at H |
| Poststate references | attachment.childSessionId=P.childSessionId. Its ID/digest resolve the complete derived ChildAttachmentSourceV1. producerSource is exactly child-assignment-source / key P and the unchanged ChildAssignmentSourceV1 domain digest. Their original plan/configuration/approval/reservation/allocation/input/criteria/rubric joins must all agree |
| Ordinary | Captured epoch/G equal the exact ordinary header at admission; id=P.childSessionId and createdAt equal the original creating record/lineage projection. Full retained source and ordinary/index membership prove session/attachment uniqueness and absence of previous preparation, including tombstones; there is no caller absence flag |
| Complete prestate | Revalidate H, G, source/control/summary/proof/membership/ledger reads and actual ownership. The four snapshots are the retained decision-control basis, not the complete transaction read set or a completeness checksum |

The task and launch snapshot keys equal P's complete TaskKey; the plan snapshot
key/body.plan equals the original producer source's PlanRef, and all task/launch/
reservation/configuration references resolve that same plan/task. Run snapshot
key/body resolve P's exact run and original run-definition/bootstrap bindings.
Do not substitute another active plan or current counter merely because its
workspace or parent session matches.

The attachment/producerSource references name deterministic **new** bodies to be
published by this attempt, derived from original retained sources. Their presence
in an intent is not a claim that the child source already committed. Before
SQL the source owner validates those exact bytes; only the complete original
joint outcome can establish them. On historical inspection, resolve them from
that retained outcome, not a current head or arbitrary supplied matching body.

The private worker's complete frozen SQL plan also contains its actual normalized
ordinary creating metadata and all derived effects. The permit retains that
exact immutable plan reference; it cannot swap even display fields after claim.
The persisted authorization intent above retains source-relevant identity/control
facts rather than copying unrelated ordinary labels, resume data or user text.
It is not asserted to reconstruct every ordinary metadata byte later. No caller
submits arbitrary ordinary metadata as a preparation request. Exact source retry
returns original P/C/creation identity; it never overwrites later ordinary metadata
with a newly computed display snapshot.

### Freeze complete bytes before the short cutoff

Proof bytes affect retained source charges. Prepare an internal candidate next
sequence under a brief owner-state capture, without consuming or issuing it.
Build/hash the exact candidate body, proof row, complete source charges and
immutable joint effect plan outside the latch. At the actual permit point compare
the retained sidecar/counter/source-transition/current-authority state, consume
that exact next sequence once and install only prebuilt immutable references.
A stale candidate is denied before mutation; no patched body or silent retry.
A prebuilt candidate body without the successful private permit grants nothing.

This proposed compare-and-consume preparation avoids hashing/copying under the
latch and makes the full proof/counter charges immutable before permission.
No new queue or reservation is created by reading a candidate. All worker,
source-fence and live-registry participation still require their qualified owner;
a hash or recorded generation does not prove that cutoff occurred.

## One immutable recovery row

Propose one required logical family in the next closed profile:

| Table / exact key | Value |
| --- | --- |
| child_preparation_authorizations / C(full ChildAssignmentIdentity P) | `{body:OriginalPreparationAuthorizationV1,digest:Digest,lastCommit:CommitRef}` |

The body reconstructs its full primary key. Require uniqueness of full canonical
PreparationPermitIdentityV1 through a derived exact-key index as well; it cannot
name two P values. Key/digest/body disagreement, duplicate permit/key, changed
same-key bytes or missing index/owning proof is unavailable, never overwrite or
collision chaining. The complete key/value obeys the inherited 2 MiB row ceiling;
ordinary 1 MiB joint-job limits and every source/global cap independently win.
No new numerical allowance or a 16 KiB claim is made for large control snapshots.

There is exactly one row per newly committed original preparation, atomically
with the accepted complete joint outcome. Its lastCommit equals that original C,
the immutable attachment/source owning commit and ordinary preparationCommit.
Later bridge/control commits do not change that link. An exact authenticated
retry resolves the original source action/P before preparing a new sequence,
creation timestamp or prestate capture. Conflicting source/configuration/
attachment bytes deny; a valid historical result adds no permit, row, C/G advance,
charge, initialization, grant, startup or spawn. A historical permit is no lookup
credential; current applicable source/UI lookup authority remains required.

Rollback has no committed row. Unknown outcome retains the actual private plan/
position/paired resources until the worker proves the original complete old/new
state. Missing proof cannot be manufactured from a surviving child, ordinary
member, counter or source digest. A new process recovers data, never a volatile
permit, execution sequence or automatic retry.

## Atomic commitments and compatibility

Propose **new-store-only hierarchy format 6**, not an alternate format 5 catalog.
Header and manifest schemaVersion are 6; use
orkworks.hierarchy-storage-change.v6 and orkworks.hierarchy-storage-row.v6 plus LF.
Require closure of the inherited named 33-family inventory, ordinary_child_lineage
as the 34th, and this authorization family as the 35th. Physical index/full
membership qualification remains open. All source/request/receipt/runtime/
ClaimProofV2/debit bodies and domains remain unchanged; ordinary schema2 and its
lineage member stay exactly as accepted. No new proof field enters that member.

Keep the existing closed manifest field shape. commandKind is child-source-attach,
claim is null because this is still outside the #744 evaluation claim class.
Its sorted changes include the authorization insert plus **every** actual
original hierarchy/ordinary-lineage/control/summary/quota effect. Coalesce each
key to one before/final after value; the two existing header/new-proof controls
retain their separate equations. No fixed minimum core is claimed as a full
transaction. Proof insertion alone adds no subjectRevision, receipt slot,
evaluation, runtime bridge event or additional control-version increment.

Hash the new row with the format-6 row envelope:

```text
{table:"child_preparation_authorizations",key:P,
 body:{body:OriginalPreparationAuthorizationV1,digest:Digest}}
```

Exclude only outer lastCommit. Keep intent.expectedCommit and **all embedded
snapshot origin CommitRefs** inside the hash. Inherit format5's exact exclusions
for unchanged logical rows, including the ordinary lineage preparationCommit.
Compute original source/attachment digests and old snapshot/proof links, candidate
authorization digest, exact charges/row commitments, then manifest digest C;
attach only the already excluded new C links afterward. No authorization field
contains its own new C, so no hash fixed point or extra exclusion is needed.

Retain and validate H's direct proof, each snapshot value.lastCommit/direct
origin manifest, original source/runtime/approval/reservation/allocation proof
joins and new C while the authorization survives. Each embedded snapshot key/body
must match its origin manifest's afterDigest using its format-6 row projection;
all origin sequences are no greater than H in the same storage epoch. Read-only
peers keep their links. Native/transactional admission proves freshness at H;
direct hashes alone cannot. Historical validation uses frozen snapshots, never
requires them to equal current mutable rows, and does not recursively replay all
manifest predecessors or elect a latest header from a maximum sequence.

Format5 readers reject format6 before interpreting rows; missing authorization
is not an older format6 base subset. No old format5/ordinary-linked subject is
imported, backfilled, rewritten or reset into this profile. Existing/lost-store
and activation checks still apply; a missing peer is not permission for a fresh
epoch. No migration/downgrade, copied proof or mixed v5/v6 CommitRef interpretation
is selected. The accepted format5 architecture remains recorded unchanged;
this next representation/version choice is owner-pending.

## Accounting and typed retained consumers

Charge each authorization's complete canonical key and committed value projection
(body/digest, including every copied snapshot and old origin link) to the existing
child-source-retained-bytes run/workspace owners of P, with their already prepared
8 MiB / 64 MiB caps. Include the copy in applicable 32 MiB run preparation/report
metadata and complete logical-store accounting. Existing current controls still
charge their own pool: copying them here earns no sharing discount or transfer.
All full wrappers/links, actual ordinary metadata, derived index and membership
maintenance also count in command/physical/native/queue budgets as applicable.

The proof is one additional logical insert relative to the complete original
prepare union. Its source-pool deltas coalesce with that command's other source
charges; do not update a key twice, initialize a missing peer or borrow any of the
five new zero assignment ledgers. Those rows remain used="0"/reserved="0" at C.
No reporting receipt, result/artifact/read-work debit or new quota resource.
The ordinary 1 MiB joint-job limit includes referenced proof/control copies;
source-valid maximum rows can fail this lower bound and remain unavailable.

Register this row as one fixed typed historical metadata consumer, not a new
ObjectRef purpose or caller graph edge. Its recipe resolves producerSource and
original attachment, all original source/configuration/plan/input/approval/
reservation/allocation/run/runtime groups, the four captured control snapshots,
H/origin/C direct proofs and ordinary lineage P/C. Complete forward/reverse/direct
membership is maintained with the original commit. Historical snapshot references
protect their exact origins; current control changes do not retarget them.
The authorization itself is not quality evidence or a subject/material owner.
Every observation/copy obeys the existing cumulative discovery limits; an absent
or unsupported consumer is unavailable, never a truncated set.

Never prune the row or its required proofs on cancellation, ordinary Delete,
run completion, workspace switch or a lost response to free capacity. Missing
retained decision evidence makes original preparation unavailable. Only an
already-authorized complete joint purge may remove it with all source/lineage/
consumer effects and exact existing resource deltas. No new standalone purge,
automatic cleanup, quota reset or refund is proposed. Retained evidence can deny
new work before other source limits fill; that cost is explicit.

## Denial/recovery oracles and handoff

- Reject duplicate/unknown fields, wrong canonical bytes, swapped source-number/
  Counter encodings, parent/generation/scope/P/attachment mismatch, forged snapshot
  keys/origin proofs or source bytes that do not derive the intended poststate.
- Revoke or superseding source transition before permit: no backend/evidence/
  child/preparation. Permit before later volatile close: only exact complete
  paired save may settle, with all closed authorities and no-spawn/read/response
  consequences from the accepted policy.
- Counter exhaustion/stale candidate, changed H/G/read set, actual lease loss,
  incomplete profile/index/membership or capacity overflow: no widened intent,
  patched prestate, replacement worker or new allowance.
- Proven rollback: complete old paired state, no proof/C/G/charge/init advance;
  consumed volatile sequence is not reused. Unknown: retain original plan and
  worker/resources; neither task Drop nor interrupt proves rollback.
- Proven commit/lost response: exact original P/C/source/ordinary lineage and
  authorization under current lookup authority, no second row/permit/charge or
  process. A proof cannot restore parent authority, a grant or an unissued child.
- Corrupt/missing original row/proof, one-sided restore or old-format source:
  unavailable and supported original recovery only, no reconstruction/backfill.

The one scoped representation decision is: separate checked preparation identity
and exact intent/snapshot codec; one immutable P-keyed authorization row plus
exact permit index/direct consumer/proof retention; and fresh format6 commitment
and accounting placement with no new quota. These choices are proposed, not
accepted by the policy decision. Reuse #1414's writer inventory; do not re-review
its unchanged accepted interfaces or adopt its pending RegistryRoot implicitly.

After representation acceptance, close the full source/catalog/paired membership
and native ownership/resource obligations before independently generated complete
old/new/manifest/packing fixtures or a separately approved executable/native plan.
No generic journal, fake admitted proof factory or standalone eligibility flag
substitutes for that work. #1415 delivers this reviewed representation proposal;
#1176 remains open and #740 closed research.


## Bounded review

One bounded read-only lower-cost round independently checked codec/commitment
correctness and builder/recovery completeness. Both approved without a required
correction, including candidate compare-and-consume sequencing, original-versus-
live control versions, semantic proof versus frozen SQL plan, exact retry,
acyclic links, source-pool copy charges and immutable proof/consumer retention.
The physical exact-permit index, full membership/catalog, actual source owner,
ordinary activation and native ordering/resources remain unverified downstream
gates, not results supplied by the review. No extra review round, fixture, code,
proof acceptance or runtime qualification is claimed.
