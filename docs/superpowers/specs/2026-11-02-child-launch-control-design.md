---
type: Design Proposal
title: Selected child launch control and recovery records
description: Proposed closed direct-execution control, lifecycle, coordination and recovery records for the accepted single-task retained-data source slice; no runtime or transaction qualification.
tags: [orkworks, hierarchy, child-sources, launch, recovery, control]
status: draft
workflow_status: owner-review-required
---

# Selected child launch control and recovery records

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspected base: `c01844092a65aed327fcdcfd08f45ca795b9d2d0`.
Contract: [accepted child sources](2026-10-31-child-review-source-preparation-design.md),
[#742 lifecycle](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table),
[launch baseline](2026-09-26-taskmaster-orchestrated-child-sessions-design.md).
**New control/input/catalog/accounting choices for scoped owner review.** No
acceptance, complete-state fixture, current grant, native event, adapter, database,
migration or runtime implementation follows from drafting or merging this file.

## Decision and selected scope

Keep immutable approved sources and mutable execution controls separate. The
accepted source codecs are unchanged. Close a selected **direct-execution**
control variant for the existing one-task, retained-data worker/reviewer plan;
keep complete original plan definitions and history. This is not the full #742
clarification/research protocol. A material question or research requirement
blocks this variant; it must not erase or flatten an unsupported record to fit.
Existing broader run states/inputs are not imported into it.

Support explicit pause/continue, exact parent resume/reapproval, ordinary child
lifecycle observations, original coordination results, and source-owned recovery
of an interrupted original attempt. Recovery can settle or confirm that attempt;
it never spawns a child or supplies a second attempt under the same TaskKey.
Retry work requires a newly declared task in a separately approved recovery
revision. Practical native guardrails apply to coding tools; internal authority,
source identity and ordinary stop independence keep their existing contracts.

Rejected alternatives: deriving current control from immutable source-row presence;
using generic JSON for blockers/recovery; storing active grants; releasing a slot
on a parent result; overwriting original admission versions; or adopting an old
store/session/worktree because its IDs or paths look plausible.

## Shared types and bounds

Use accepted IDs, digests, canonical UTF-8 JSON, duplicate/unknown-field rejection,
TaskKey, PlanRef, ConfigurationRef, ApprovalRef, ReservationRef, AllocationRef,
InputRef, child AssignmentIdentity, NativePath and native directory-object shapes.
DirectoryObjectIdentity is the accepted closed
`{platform,mechanismVersion,volumeId,fileId}` shape and bounds; NativePath retains
its accepted encoding/byte bounds. Source JSON counters retain their encodings. runVersion, planVersion and taskVersion
are 1..2^31−1; plan/preparation/input revisions are 1..64. Storage counters are
canonical u64 decimal TEXT, including positive CommitRef sequences. Generations
are 64 lowercase hex characters. No timestamp, secret or model self-report is
used as an ownership/ordering proof. Credentials, bearers, raw production
transcripts and hidden reasoning are excluded from retained records.

All record field sets below are exhaustive; every listed nullable field is present.
A complete key/value wrapper obeys its table bound and the existing 2 MiB row cap.
Lower source/command/store/adapter bounds win. Reject combinations that do not fit;
no truncation or claim that all maxima are simultaneously supported.

RuntimeKey is exactly `{kind,workspaceId,runId,sessionId,sidecarGeneration,
launchGeneration}`, with kind parent or child. RuntimeRef adds exactly
`attachmentDigest`. PreparedInputRef is exactly `{workspaceId,runId,
preparationRevision,inputRevision,inputDigest}`. These are direct typed metadata
references, not new ObjectRef kinds, bearers or grants.

## Prepared direct-execution input

DirectDecisionV1 is exactly `{schemaVersion:1,inputRevision,
clarificationNeeded:false,researchNeeded:false,reasons,unknowns:[]}`. reasons has
1..16 nonempty strings, each through 2 KiB. This is a source-owned assessment,
not proof its conclusion is correct or authority to skip a known requirement.
A true need for clarification/research is unsupported here and records a blocker;
there is no conversion of an existing answered/unanswered question to an empty list.

PreparedChildInputV1 is exactly:

```text
{schemaVersion:1,workspaceId,runId,preparationRevision,inputRevision,
 goalDigest,bootstrapConfigurationDigest,decision:DirectDecisionV1,
 records:InputRecordV1[],inputDigest}
```

Use the existing orkworks.preparation-input.v1 + LF domain excluding only
inputDigest. Body is at most 32 KiB; at most 256 unique records, sorted by UTF-8
kind then canonical full record bytes. InputRecordV1 is exactly one of:

- `{kind:"configuration",reference:ConfigurationRef}`.
- `{kind:"review-input",reference:InputRef}`.
- `{kind:"result",reference:ResultRef}` with the accepted exact child result reference.
- `{kind:"research-report",source:ResearchReportEvidenceSource}`.
- `{kind:"prior-plan",plan:PlanRef}` for the exact replaced/recovery plan only.

The source owner derives the complete list: selected configuration; for a reviewer,
its input snapshot, target result/configuration and every selected report source;
plus the replaced plan when applicable. Reject missing/extra/duplicate entries.
Each resolves retained bytes and complete ownership/custody. No excerpt or transitive
citation is consumed merely because a report references it. No parent control,
new plan-definition digest, future reservation or active authority enters records.
Thus input bytes can be frozen before computing the new plan definition digest.

InitialInputV1 stays unchanged and retained at revision 1. The first prepared
manifest advances both preparationRevision and inputRevision to 2; each changed
record set/decision advances both once with checked arithmetic through 64. A new
proposal cannot patch a frozen input or reuse its version with different bytes.
Same-plan reapproval with unchanged current sources uses the original manifest;
historical producer generations need not equal the current runtime. Plans bind
its exact revisions/digest in the existing ParentPlanBinding. Unsupported broader
input variants require their own reviewed codec rather than a generic reference.

## Runtime attachment history

RuntimeAttachmentV1 is exactly one of:

```text
{schemaVersion:1,kind:"parent",parent:ParentAttachmentV1}
{schemaVersion:1,kind:"child",assignmentIdentity:ChildAssignmentIdentity,
 attachmentId,sidecarGeneration,launchGeneration}
```

The parent variant uses only ParentAttachmentV1's accepted attached variant, never
reserved. Reconstruct RuntimeKey from the complete body and source run/workspace.
attachmentDigest is SHA-256 of orkworks.child-source-runtime-attachment.v1 + LF
and the whole canonical body. The actual session/native producer publishes one
immutable row per successfully attached runtime generation. A generation reserved
for an attempted spawn is not such a row. Preserve every referenced generation
after ordinary end/resume; at most 64 parent and 64 per-child runtime attachments
in this selected profile, then fence new hierarchy attachment rather than recycle.

These records do not authenticate themselves. Actual native identity, eligible
unchanged startup bytes and source-owned publication must be established by the
producer and separately qualified implementation. Same-user host access remains
the disclosed coding-tool risk; no OS-origin attestation is claimed here.

## Mutable run and plan controls

ChildRunControlV1 is exactly:

```text
{schemaVersion:1,profile:"single-task-retained-data-execution-v1",
 workspaceId,runId,parentSessionId,runDefinitionDigest,
 bootstrapConfigurationDigest,repositoryBindingDigest,sourceWorktreeBindingDigest,
 runVersion,state,currentInput:PreparedInputRef|null,planReferences:PlanRef[],
 currentPlan:PlanRef|null,pendingPlan:PlanRef|null,parentRuntime:RuntimeRef|null,
 blockers:ControlBlockerV1[],stageContinueRequired,pendingClarification:null,
 pauseReason:null|PauseReason,resumeTarget:null|ResumeTarget}
```

state is assessing, awaiting_execution_approval, executing, blocked, paused,
awaiting_resume_approval, complete or cancelled. planReferences retains submission
order, unique full references, at most 16 plan IDs and 64 revisions per plan
(1,024 entries). The stricter 64 preparation revisions still constrain how many
new distinct proposals this variant can prepare; the list bound raises no limit.
currentPlan/pendingPlan resolve that list; at most one pending proposal exists.
A replacement proposal may be retained while the current child is live, but its
approval must satisfy the baseline's prior-result/terminal/allocation gates.

ControlBlockerV1 is exactly `{id,kind,task:TaskKey|null,detail}`: unique sorted
IDs, at most 16, detail nonempty through 2 KiB. kind is launch-interrupted,
source-changed, capacity-exceeded, profile-exit-required, control-exhausted,
native-unavailable, coordination-failure or recovery-approval-required. Task refs
belong to this run. Unknown blockers/states are unavailable, never dropped.

PauseReason is the existing stage_cancelled, parent_ended, workspace_changed or
sidecar_replaced. ResumeTarget is assessing, awaiting_execution_approval or
executing, retaining the underlying coordination target across repeated pauses.
paused requires a reason/target. stageContinueRequired survives all pause-reason
changes; it is not inferred from them. complete/cancelled are absorbing even when
ordinary child lifecycle/audit data later changes. parentRuntime names retained
history, not liveness; it may survive a pause and never restores volatile authority.

ChildPlanControlV1 is exactly `{schemaVersion:1,plan:PlanRef,planVersion,status,
latestApproval:ApprovalRef|null,task:TaskKey,replaces:PlanRef|null}`. status is
proposed, active, paused, superseded, complete or cancelled. task is the source
plan's only task. replaces is null or the unique prior-plan entry in the exact
prepared input; it cannot be an unbound mutable recovery relationship. All refs
resolve this run, with exact configuration/worktree/source joins. Active requires
a retained approval but does not imply an active grant. Approval replays return
original source receipts; only a fresh authorized approval creates a new volatile
grant only for an eligible unreserved task. If every task is already reserved,
resume restores collection without a new launch grant. Old approvals/history
remain retained.

The initial run_states value remains the accepted historical initial snapshot.
This proposal explicitly places later mutable run state/version in
child_run_controls, rather than widening that old initial-only value codec.
First parent attach performs the existing logical runVersion 1→2 transition in
the new current control. The old seed is never mistaken for current state or used
to reconstruct missing current controls on startup.

Also explicitly extend the existing parent_attachments wrapper from reserved-only
to both already-defined ParentAttachmentV1 variants, within its 4 KiB bound. First
attach replaces reserved with attached; exact resume replaces its current attached
generation while immutable runtime history retains every actual old attachment.
Current parent identity stays bound to the same original run/root/bootstrap.
The parent row, runtime history and current run pointer/version publish together;
source-owned historical initial commitments remain protected by their original
source/proofs. Actual lifecycle publication/recovery still needs qualification.
The existing preparation-retained-bytes owner rows charge parent_attachments
replacement deltas; the new control pool charges only its nine named families.
These are explicit fresh-catalog/storage-placement choices for owner review,
not a silent reinterpretation or migration of existing source bytes.

## Task, launch and ordinary-session controls

ChildTaskControlV1 is exactly `{schemaVersion:1,task:TaskKey,taskVersion,status,
reservation:ReservationRef|null,coordinationResult:null|{requestDigest}}`.
status is pending, launching, running, needs_parent_result, reported_complete,
reported_blocked, reported_failed or launch_interrupted. Pending has no reservation
or result. A successful reservation increments the normal live taskVersion once;
its resulting value is frozen in the original reservation and never recaptured.
Subsequent task/control changes can advance the live version, not historical identity.

ChildLaunchControlV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,reservation:ReservationRef,
 admission:{approval:ApprovalRef,parentRuntime:RuntimeRef,
            expectedTaskVersion,assignmentTaskVersion},phase,
 allocation:AllocationRef|null,attachmentId:ID|null,
 spawnRuntime:null|{sidecarGeneration,launchGeneration},
 observedDirectories:ObservedDirectory[],interruptedAt:null|InterruptedStage,
 reasonCode:null|InterruptionReason}
```

admission is frozen with the original reservation; expectedTaskVersion+1 equals
assignmentTaskVersion and the reservation snapshot. No active grant ID or token
is archived. phase is reserved, allocating, allocated, prepared, spawn-pending,
running, interrupted, ended or settled-no-child. InterruptedStage is allocation,
attachment or spawn; InterruptionReason is native-failure, source-drift,
authority-lost, process-replaced or disposition-unknown. Both are nonnull only
for interrupted. observedDirectories has at most two entries, unique and UTF-8-sorted by kind:
`{kind:"root"|"private-git",path:NativePath,object:DirectoryObjectIdentity}`,
using the accepted full native directory-object shape. These are actual owned
observations, not permission to adopt a matching path. Unknown ownership keeps
the attempt interrupted and capacity held. Native source proof remains required.

Reserved/allocating have no allocation/attachment/runtime; allocated requires its
original allocation and no attachment/runtime. Prepared requires allocation,
original attachment/source and complete fresh preparation, with null spawnRuntime.
Spawn-pending records one proposed generation before process creation, not proof
it started. Running requires retained proof of the original successful spawn generation and
a nonterminal ordinary state. spawnRuntime is frozen once that first outcome is
established; an ordinary UI resume records a new current session RuntimeRef and
can return ended to running without a new orchestrated attempt. It does not
rewrite original spawn history, task identity, coordination result or terminal
run/plan status. A new ordinary runtime cannot itself settle an unknown original
spawn disposition; that remains blocked pending source-owned recovery.
Ended retains the allocation/attachment/history and requires a terminal ordinary
state. Settled-no-child has no attachment/runtime and a proven recovery decision;
it retains its reservation/attempt and any known owned allocation. Interrupted
retains every already-established field; it cannot erase an allocation/attachment
or downgrade an existing successful runtime to a mere proposed generation.

ChildSessionStateV1 is exactly `{schemaVersion:1,task:TaskKey,
reservation:ReservationRef,assignmentIdentity:ChildAssignmentIdentity,
attachmentId,sessionId,producerRevision:Counter,state,runtime:RuntimeRef|null}`.
state is prepared, nonterminal or terminal. Created only with the original unique
attachment and fresh ledgers, prepared/runtime null. The ordinary lifecycle owner
advances producerRevision from "1" with each actual relevant change. This is a
new bridge-owned revision, not a reinterpretation of existing session metadata
revision fields. Actual runtime start/resume records its new RuntimeRef; ending
retains the last reference (null if startup never succeeded). The bridge must
publish/recover coherently with the actual ordinary-session state; a model report
or a copied stored status cannot release capacity.

Capacity is derived, never another caller-supplied counter: count each unsettled
unattached reservation once, or its uniquely attached nonterminal/prepared child
once; terminal children and proven settled-no-child attempts count zero. Do not
count reservation plus child twice. Parent reported_complete does not release a
slot. Derive across all retained plan/task attempts for the run and exact plan;
use complete bounded indexes. Resolve the complete retained run definition through
runDefinitionDigest and all repeated bootstrap/repository/workspace/parent joins.
Its immutable UI-selected maxParallelChildren (1..16) is the run-wide ceiling;
the selected source plan's maxParallelChildren=1 is its lower plan ceiling.
Reservation requires derived run usage+1 <= that run ceiling and derived plan
usage+1 <= the exact plan ceiling, atomically with all source/control checks.
An unknown count/limit denies admission; a plan cannot raise the run ceiling.
A direct ordinary UI resume can make an old child
nonterminal again: preserve it, count it, and fence new launches if over capacity;
do not kill it or pretend ordinary APIs are an orchestration security boundary.
Unknown bridge/source disposition fences launch/read eligibility conservatively.

## Original coordination result and recovery decisions

CoordinationRequestV1 is exactly `{schemaVersion:1,task:TaskKey,
reservation:ReservationRef,childSessionId,expectedTaskVersion,outcome,summary}`.
outcome is completed, blocked or failed; summary is nonempty through 2 KiB.
Its requestDigest uses orkworks.child-source-coordination-request.v1 + LF over
all canonical bytes. This is a new selected closed representation of the existing
parent coordination report, distinct from #744 quality/result/evaluation records.
CoordinationReceiptV1 is exactly `{schemaVersion:1,task,reservation,childSessionId,
acceptedTaskVersion,commit:CommitRef}`, at most 4 KiB. The request, original receipt
and task transition commit together; task.coordinationResult pins requestDigest.

One immutable outcome is retained per TaskKey. Current parent authentication and
scope precede exact retry lookup; identical bytes return the original receipt
before live-version CAS and cause no new version/proof/charge. A conflicting
report cannot overwrite it. New reports require needs_parent_result and exact
current task/reservation/session. They change only coordination/dependency state,
never establish quality, release live-child capacity or reopen a terminal plan/run.

RecoveryDecisionV1 is exactly:

```text
{schemaVersion:1,recoveryId,task:TaskKey,reservation:ReservationRef,
 expectedRunVersion,expectedTaskVersion,reviewContextId,sidecarGeneration,
 kind,allocation:AllocationRef|null,attachmentId:ID|null,
 readSet:RecoveryReadSetV1,observedDirectories:ObservedDirectory[],
 blockers:ControlBlockerV1[],reason}
```

kind is settle-no-child, confirm-original-allocation, confirm-original-attachment
or observe-terminal. reason is nonempty through 2 KiB; blockers retains up to 16
exact original current task-related blocker bodies, sorted uniquely by ID.
readSet retains the original control bodies and their exact committed origins,
as defined below. These are source-owned captures, never client-supplied proof.
The source owner first obtains the bounded original control/native observations.
Electron main reviews those exact facts and authorizes the version-bound operation
under existing UI authority; the owner then revalidates them at admission/commit.
Models/renderers do not supply trusted facts. reviewContextId binds the exact
shown original attempt and current observations;
it is a nonsecret 64-hex context, not a bearer. Hash the entire body under
orkworks.child-source-recovery-decision.v1 + LF. RecoveryReceiptV1 is exactly
`{schemaVersion:1,recoveryId,decisionDigest,runVersion,taskVersion,commit:CommitRef}`,
at most 4 KiB. Authenticate current UI context before exact retained retry lookup;
a duplicate returns the original receipt, never applies recovery twice.

- settle-no-child proves no child metadata/runtime exists and no native operation
  remains unresolved. Any surviving worktree must be the proven original owned
  allocation, retained for manual review; otherwise native path/branch absence
  must be established. It cannot delete, adopt, reuse or launch anything. Record
  settled-no-child, keep task launch_interrupted, and require a new approved
  recovery revision for any retry work. No attached subject/ledger is fabricated.
- confirm-original-allocation requires exact original intent and source-owned
  native-object proof, not a path search alone. Publish/resolve only the original
  allocation outcome; keep launches blocked pending ordinary authority and any
  required exact recovery approval. Missing original ownership is unavailable.
- confirm-original-attachment resolves an already proven complete original
  attachment/preparation outcome and matching session state. It cannot fill
  historical zero ledgers or bless a process started before preparation. An
  incomplete or ambiguous original transaction remains unavailable.
- observe-terminal requires the exact current ordinary lifecycle producer revision
  and terminal state. It updates derived capacity once and issues no new grant.

Decision fields must match the selected kind: no-child has attachmentId and
readSet.session null; allocation confirmation has allocation nonnull and
attachmentId/readSet.session null; attachment confirmation has allocation,
attachmentId and readSet.session nonnull; terminal observation has attachmentId/
readSet.session nonnull and resolves its original allocation. Observations match those complete source descriptors where
present. Never replace an unknown outcome with a successful enum on timeout.
RecoveryReadSetV1 is exactly `{expectedCommit:CommitRef,run:RunControlRead,
task:TaskControlRead,launch:LaunchControlRead,session:SessionStateRead|null}`.
Each read is a closed tagged row snapshot, not an arbitrary table/key/value bag:

| Snapshot | Exact table/key/value |
| --- | --- |
| RunControlRead | `{table:"child_run_controls",key:{workspaceId,runId},value:{schemaVersion:1,body:ChildRunControlV1,lastCommit:CommitRef}}` |
| TaskControlRead | `{table:"child_task_controls",key:TaskKey,value:{schemaVersion:1,body:ChildTaskControlV1,lastCommit:CommitRef}}` |
| LaunchControlRead | `{table:"child_launch_controls",key:TaskKey,value:{schemaVersion:1,body:ChildLaunchControlV1,lastCommit:CommitRef}}` |
| SessionStateRead | `{table:"child_session_states",key:TaskKey,value:{schemaVersion:1,body:ChildSessionStateV1,lastCommit:CommitRef}}` |

Snapshots are full decoded canonical objects, never quoted JSON strings. Their
keys reconstruct the decision's original run/task/reservation; captured run/task
versions equal expectedRunVersion/expectedTaskVersion. blockers equals the exact
selected original blocker bodies in readSet.run. readSet.launch preserves the
original admission and phase; a nonnull session read binds the same original
attachment/assignment and its actual observed bridge revision. Null is not an
absence proof: settle-no-child still requires the source owner's complete bounded
absence/native-quiescence checks and cannot infer them from this shape.

For each read retain its original value.lastCommit and the direct commit_proof
whose manifest after commitment for that exact table/key equals the format-4
row digest reconstructed from the snapshot with only outer lastCommit excluded.
Retain those direct proof/source links while the recovery decision remains.
Every embedded original CommitRef is hashed as ordinary decision data; only the
new recovery row's own lastCommit/receipt.commit are excluded by its projection.
Retain the direct proof for readSet.expectedCommit as well as each original
row origin and the new recovery commit. All are in the same workspace/storage
epoch; original row sequences are no greater than expectedCommit. This does
not require recursively retaining every predecessor. The new recovery commit's
predecessor equals readSet.expectedCommit. Every
replaced captured row's new manifest before commitment must match that snapshot.
Read-only captured rows keep their original links, without a link-only rewrite.

At admission the source owner revalidates all captured decision-control rows
against the exact coherent current header/state. This typed historical decision
basis is not the complete transaction read set: required plan/source/quota/
consumer/index reads remain mandatory and must be closed by their owners. Direct origin proofs establish snapshot
integrity; they do not independently prove freshness at expectedCommit or actual
native authority. That is the source-owned transactional admission requirement,
not a caller checksum assertion. No unbounded predecessor replay is substituted
for these required direct proofs. The enlarged recovery row remains bounded by
the inherited complete 2 MiB row ceiling and the separate control/store quotas;
its frozen copies are fully charged. Capacity may deny before the 32-decision
count ceiling, with no pruning, partial recovery or borrowed allowance.

Validate historical decisions against their frozen source/snapshot and direct
commit joins at original admission; do not require old expected versions, bridge
revisions or runtime generations to equal the current process/control rows.
A recovery decision clears only the captured reconciled native-disposition blockers;
failed/interrupted work retains recovery-approval-required until a separately
approved new task/revision. It clears neither stageContinueRequired, a material
input blocker, nor an unanswered question; no existing grant is restored.

## Closed storage catalog and finite accounting

Add these proposed mandatory families to the single fresh format-4 catalog.
The existing parent_attachments variant extension above is an additional explicit
catalog change, not a tenth new family. These are not alternate profiles chosen
by row presence. All readers must support
the whole reviewed catalog, even empty tables; no old v3/v4 store import, implicit
schema upgrade, receipt repair or mixed-domain rewrite is selected.

| Table / exact key | Exact value | Complete wrapper ceiling |
| --- | --- | ---: |
| child_control_inputs / PreparedInputRef without inputDigest | `{schemaVersion:1,body:PreparedChildInputV1,producer:RuntimeRef,lastCommit}` | 64 KiB |
| child_runtime_attachments / RuntimeKey | `{schemaVersion:1,body:RuntimeAttachmentV1,digest,lastCommit}` | 4 KiB |
| child_run_controls / `{workspaceId,runId}` | `{schemaVersion:1,body:ChildRunControlV1,lastCommit}` | 1 MiB |
| child_plan_controls / PlanRef | `{schemaVersion:1,body:ChildPlanControlV1,lastCommit}` | 16 KiB |
| child_task_controls / TaskKey | `{schemaVersion:1,body:ChildTaskControlV1,lastCommit}` | 16 KiB |
| child_launch_controls / TaskKey | `{schemaVersion:1,body:ChildLaunchControlV1,lastCommit}` | 32 KiB |
| child_session_states / TaskKey | `{schemaVersion:1,body:ChildSessionStateV1,lastCommit}` | 16 KiB |
| child_coordination_receipts / TaskKey | `{schemaVersion:1,request,requestDigest,receipt:CoordinationReceiptV1,lastCommit}` | 16 KiB |
| child_recovery_decisions / `{task:TaskKey,recoveryId}` | `{schemaVersion:1,body:RecoveryDecisionV1,digest,receipt:RecoveryReceiptV1,lastCommit}` | 2 MiB |

Every key reconstructs from the body and original immutable sources; enforce
unique child/parent session ownership across runs, not just uniqueness within a
row's key. Sources, prepared inputs, runtime history, coordination receipts and
recovery decisions are immutable. Each task has at most one launch control and
session-state row, with no second reservation/attempt. At most 32 recovery decisions
are retained per run, then deny new recovery records without recycling old IDs.

Use existing format-4 row/manifest domains. Exclude only outer lastCommit and,
for the two receipt-bearing new tables, receipt.commit. All internal source refs,
original admission values, native observations and current control versions remain
committed. Validate every excluded link against its owning complete proof. Retain
direct proof/source/parent-runtime joins needed by any historical source, result,
coordination or recovery reference. No runtime history or original admission is
inferred from a maximum counter, a remaining directory or a receipt alone.

Propose resource child-control-retained-bytes: **8 MiB per run, 64 MiB per workspace**,
reserved="0", using existing exact run/workspace owner/value codecs. Charge full
canonical keys and committed value projections for all nine families, counting
every occurrence. Mutable replacements use exact original-to-final inventory
deltas; immutable history is never evicted or reset. This is retained metadata,
not lifetime work: shrinking a mutable value changes this resource's byte total
but never refunds artifact read-work, receipt credits or other retained sources.
Count complete copies/links in command/physical budgets regardless of projection.
The existing 32 MiB run preparation/report metadata ceiling also includes these
control records. All stricter global caps remain; no maximum profile is qualified.

Fresh-profile initialization explicitly prepares both control quota scopes before
publishing its first row. Do not add missing rows to a historical store. The accepted
child-source pool remains separate, as do all five assignment/run/workspace
record/receipt/payload/artifact resources and summary/preparation ledgers. Ordinary
end or input/revision change releases no immutable control history. No standalone
control/source purge or replacement-store allowance is proposed.

## Transition and recovery rules

All new source-owned mutations compare exact captured run/plan/task versions and
retain complete bounded source/control/consumer read sets. The initial current run control uses JSON integer runVersion=1 and the exact accepted initial source identities. New
plan/task controls start at integer version 1. Each semantic run-owned control
transition increments runVersion once; each changed plan control increments
planVersion once, and each relevant task/launch/session transition increments
that taskVersion once. Coalesce multi-row effects into those single increments.
Every ordinary state event advances its bridge revision once. Exact retries change none.
Overflow fences hierarchy mutations and automation without wrap/reset. Ordinary
stop/ending remains independent; an unpublishable lifecycle bridge leaves hierarchy
unavailable/conservatively held until supported reconciliation, not a delayed stop.

| Event | Required control/source effects |
| --- | --- |
| Fresh root/profile preparation | Seed one current run control from exact initial sources, no runtime/plan/grant; create required control quota rows through explicit fresh preparation |
| Parent attach/resume | Exact UI/native identity, immutable runtime attachment and current run pointer/version; revoke old volatile authority, create only separately verified current authority |
| Source-owned direct assessment/proposal | Freeze next prepared input/producer and exact immutable source plan; insert pending plan/task controls at version 1; preserve old sources and capacity |
| Exact plan approval/reapproval | Retain existing approval source/receipt, update current plan/latestApproval and status/versions; create a fresh volatile exact-plan grant only for an eligible unreserved task, otherwise collection only; no launch yet |
| Reservation | In one outcome record original reservation and launch admission, task launching/resulting frozen version, and capacity membership before native allocation |
| Allocation begin/outcome | Persist allocating intent before native mutation, then exact original allocation/observations or interrupted state. No new process, path substitution or source-worktree adoption |
| Attachment/preparation | Original attachment/source and session prepared state, five fresh assignment zero rows, summary branch and complete owning effects; only then may a separately authorized spawn begin |
| Spawn outcome | One actual runtime history/session state and running task, or retained interrupted outcome. A spawn-pending generation is not a live process or retry permission |
| Ordinary ending/resume | Coherent source-owned session revision/current runtime and derived capacity; preserve immutable assignment/task-version history and report semantics |
| Parent coordination outcome | Original request/receipt and task status/version; completion only if all reservations settled, all required outcomes complete and no interrupted allocation/pending revision remains |
| Pause/cancel/continue/recovery | Preserve blockers, attempts, input history and stage-continue gate; UI recovery changes only proven disposition. No automatic spawn, cleanup or grant revival |

All commands include their complete actual quota/dependency/summary/control/index
and header/new-proof effects; this table is not a manifest with omitted symbolic
rows. Unknown #743/proposal/learning consumers remain unavailable, never presumed
absent. Register direct metadata consumption of exact PlanRef/task/runtime/input/
reservation/allocation/attachment sources with the owning bounded membership
indexes, including historical references. #1210 debit membership remains its
accepted closed-table consumer; do not turn controls into summary/freshness or
ObjectRef purposes without that source owner's contract.

Resume precedence is unchanged: retain stageContinueRequired; then retained
blockers/unsettled allocation; then unanswered clarification (unsupported here,
never erased); only then classify unchanged approved/pending plans or between-plan
preparation. Same-plan reapproval cannot replace recovery of a failed attempt.
Recovery revisions retain the exact prior-plan input reference, declare a new task
and independent worktree group, and receive fresh exact approval. They never reset
the old attempt or amend its frozen configuration/source identity.

Every launch also needs current in-memory grant/bearer/runtime/source/lease checks;
durable active status is insufficient after restart. Actual ordinary-session
coherence and native authority/commit ordering must be qualified separately. These
source/control actions remain outside #744 publication claims and #1184 debit
permits. No lock-through-SQL/fsync, hidden retry, store reinitialization or delayed
ordinary revocation/stop is authorized. Coding-tool instruction adherence and host
access limitations retain the accepted practical standard; they are not database
or UI approval authority.

## Decision and bounded handoff

Owner choices are this selected direct-execution input/control/lifecycle/recovery
representation; nine new required fresh-catalog families, the existing parent
attachment/current-run storage routing and commitment exclusions;
explicit row/history limits and the separate 8/64 MiB control pool; and the
source-owned original-attempt recovery/ordinary-lifecycle bridge semantics.
Broader clarification/research inputs, parent automation, actual producer entry
points and physical cross-store/native commit mechanisms remain unqualified.
Do not treat acceptance of these codecs as acceptance of an unspecified mechanism.

After scoped acceptance, independently pin all variants, version/CAS/retry and
pause/resume precedence; source reconstruction; capacity cases (including live
completed children, uncertain allocation and ordinary UI resume); key/row/source/
aggregate denials; and exact old/new effects only after complete consumer and
producer contracts close. The existing qualification handoff remains the gate;
no dependent fixture or native/model probe is authorized by this proposal.

Risk: dependencies 4 (ordinary session/native owner plus hierarchy source/storage),
blast radius 4 (shared control/custody/capacity), state changes 3 (new-store-only
additions), reversibility Unknown (no implemented recovery/rollback evidence),
uncertainty 4 (native/current-source bridge unqualified); sum incomplete. Ready
for bounded written review, not implementation or a source-valid full profile.
