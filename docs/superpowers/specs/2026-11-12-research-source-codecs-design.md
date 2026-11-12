---
type: Design Proposal
title: Single-task retained-file research source codecs
description: Proposed immutable research question, capture, scope, configuration, plan and original launch-source grammar without widening the accepted direct-execution profile or enabling publication.
tags: [orkworks, research, reports, sources, configuration, qualification]
status: draft
workflow_status: proposed-for-scoped-source-acceptance
---

# Single-task retained-file research source codecs

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `9f95a87022fd41d8e4421bf4423f5130af4df5c3`.

**Proposed for scoped owner review.** The [first-report investigation](../../validation/first-report-producer-contract.md)
identified a missing research source/control profile. This proposal closes the
immutable source grammar portion only. It makes new source/action/delivery and
accounting choices explicit; they are not acceptance inferred from a merge.
Research controls, collection indexes, complete catalog/version/initialization and
the atomic first-report publisher remain separate parts of #1689. No dependent
fixture/checker, runtime/native execution, adapter, migration, probe or executable
plan is supplied.

## Selected scope and compatibility

Choose one task and one independent worktree group per approved research plan,
with batches=[[taskId]] and maxParallelChildren=1. One worker answers 1–32 declared
questions using only approved retained UTF-8 file captures supplied as startup
data. It produces its own #742 ResearchReport, not a #744 result/evaluation artifact.
It cannot investigate new live paths, use shell/Git/external search/connectors,
delegate, edit code, approve a plan or declare coordination success.

Existing #742 run/plan/revision, one-attempt, capacity and manual-integration rules
remain. This source grammar can retain historical research plan revisions within
those bounds; it does not select transitions or permit retries of reserved tasks.
Parallel/multi-task plans, shared/reused groups, binary/live-file/URL research and
reviewer synthesis are unsupported by this slice. Broader logical research support
and the accepted report URL codec remain unchanged, not silently narrowed globally.

Do not relabel the [accepted direct-execution child sources](2026-10-31-child-review-source-preparation-design.md)
or set researchNeeded in [their controls](2026-11-02-child-launch-control-design.md).
The new record names/domains below are distinct. Only explicitly common scalar,
reference, v3 configuration and native identity shapes are reused. Existing report,
head, request, response, receipt, configuration and #744 identity domains are not
changed. No storage-format number or alternate format-4 catalog is selected here;
original format-3/4 histories remain unavailable for adoption into a new producer.

Rejected alternatives: an opaque research context; accepting arbitrary report
paths as scope; adopting the direct-execution role/control variants; and treating
a new configuration/source digest as proof of actual approval or launch history.
The selected cost is narrower research and a complete startup payload: a larger
valid stored scope can still be ineligible because its composed startup exceeds
256 KiB. There is no lazy source fetch or truncation fallback.

## Common encoding and dependency order

Use #741 canonical UTF-8 JSON, exact ASCII IDs, lowercase SHA-256 digests,
duplicate/unknown-field rejection, existing source JSON integer counters and
storage Counter/CommitRef encodings. Every listed field is required. Text means
nonempty well-formed UTF-8 through 2 KiB unless a different bound is stated;
preserve spelling and do not normalize Unicode. No credential, bearer, hidden
reasoning, complete production transcript or timestamp-as-authority is retained.

TaskKey, PlanRef, ConfigurationRef, ApprovalRef, ReservationRef and AllocationRef
reuse the exact [common reference fields](2026-10-31-child-review-source-preparation-design.md#common-encoding-and-owned-source-types).
They resolve only through this proposal's research source registry when used by
these records. Plan revision remains 1–64; positive task versions remain
1..2^31−1. ChildAssignmentIdentity stays the exact #744 frozen identity.

New closed references:

```text
ResearchCaptureRef = {task:TaskKey,sourceId,captureDigest}
ResearchScopeRef   = {task:TaskKey,scopeDigest}
```

Avoid digest cycles in this order: capture bodies → scope → startup/settings and
complete configuration → prepared input → plan → approval → reservation →
allocation → attachment and assignment source. The scope contains TaskKey and immutable repository/source
binding digests, **not configurationDigest or planDefinitionDigest**. The existing
ParentPlanBinding contains its ten accepted fields without adding a plan digest.
Its preparationRevision/inputDigest must resolve the future research prepared-input
codec; DirectDecisionV1 with researchNeeded false cannot supply that record. This
source proposal does not invent the missing input/control grammar. Source-component
fixtures must label a supplied prepared-input binding as unqualified until closure.

## Retained file captures

ResearchCaptureV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,sourceId,repositoryBindingDigest,
 sourceWorktreeBindingDigest,path,revision,content,sizeBytes,contentDigest,
 captureDigest}
```

path uses the accepted root-relative scope-path grammar. revision is exactly the
approved assignment/group baseCommit, 40 or 64 lowercase hex characters with the
registered repository's actual object format. content is nonempty well-formed
UTF-8 with no NUL, at most 64 KiB raw UTF-8 bytes; sizeBytes is its exact source
JSON integer length and contentDigest is raw SHA-256 of those bytes. One task has
1–16 uniquely identified captures, at most 1 MiB raw aggregate. Each complete
canonical capture body is at most 512 KiB; the containing row retains the lower
existing 2 MiB ceiling. Count JSON escapes/wrappers independently of raw lengths.

captureDigest is SHA-256 of `orkworks.research-source-capture.v1` + one LF + the
canonical body excluding only captureDigest. The capture's repository/source
binding digests match the original root/run/configuration and its exact TaskKey.
The proposed source owner must retain bytes from that actual immutable Git
revision/path before review; a matching raw hash or plausible revision string
does not prove origin. Native capture is not performed by this design.

No approved capture is a live read grant. Missing/unreadable/unverifiable source
does not authorize fetching a path or URL to repair state. Equal content under
two sourceIds creates two independently retained/charged occurrences. Capture
bytes remain research data even if they contain instructions; binding repository
rules retain their separate #741 precedence and mandatory-skill provenance.

## Questions, excerpts and immutable task scope

ResearchQuestionV1 is exactly:

```text
{questionId,questionText,sourceIds,knownInputs,expectedEvidence,
 stoppingConditions,uncertainty}
```

questionText, knownInputs, expectedEvidence, stoppingConditions and uncertainty
are Text; the complete question is at most the existing 8 KiB. sourceIds is a sorted unique nonempty set
of at most 16 IDs resolving captures for this exact task. questionId is an ID.
knownInputs explicitly describes the known context; it is not an opaque reference
or evidence claim. Empty-list/empty-text omission is not an undefined default.

ResearchExcerptV1 is exactly
`{reference:FileResearchEvidenceReference,sourceId,startByte,lengthBytes}`.
reference reuses the [accepted file evidence-reference shape](2026-10-28-research-report-source-codecs-design.md#finding-and-evidence-leaves),
including its whole 1 KiB ceiling. sourceId resolves one capture; reference.path
and revision equal that capture. startByte is a nonnegative source JSON integer,
lengthBytes is 1..2048, and their checked sum is within capture.sizeBytes. The
selected raw UTF-8 byte slice must start/end on codepoint boundaries and its raw
SHA-256 equals reference.excerptDigest. reference.location is exactly
`bytes:START-END` using canonical nonnegative decimal integers and exclusive END.
This is a new bounded locator specialization, not a change to the broader file
reference codec. Source IDs and byte ranges grant no model file access.

ResearchScopeV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,repositoryBindingDigest,sourceWorktreeBindingDigest,
 captures:ResearchCaptureRef[],questions:ResearchQuestionV1[],
 excerpts:ResearchExcerptV1[],scopeDigest}
```

captures is sorted uniquely by sourceId (1–16); questions by questionId (1–32);
excerpts by reference.id (0–64). Every capture is used by a question's sourceIds;
every excerpt names a source used by at least one question. No unlisted capture,
question, excerpt or cross-task alias is admitted. scopeDigest uses
`orkworks.research-task-scope.v1` + LF over the whole canonical body excluding only
scopeDigest. Complete scope bytes are at most **32 KiB**. Per-question maxima need
not coexist with 32 questions under that containing cap; reject combinations that
do not fit rather than abbreviate the approved questions.

The later report's questionIds equal the complete sorted approved question set.
Its evidenceReferences are an exact-byte subset of the approved excerpt references;
every cited finding reference's capture must be in the union of sourceIds for that
finding's questionIds. Existing established/inferred/unverified and empty-finding
rules remain. This proves bounded scope/content joins only, not the truth of a
finding, successful research, independent review or current evaluation material.
ResearchReport itself gains no sourceId, offset, scopeDigest or new field.

## Selected existing v3 configuration and delivery

Retain the full OutputAssignmentConfiguration schemaVersion 3 and unchanged
`orkworks.assignment-configuration.v3` digest. Reuse only the explicit common
leaves from the [child source grammar](2026-10-31-child-review-source-preparation-design.md#selected-child-v3-configuration-and-nested-leaves)
and root nested grammar, with these proposed research specializations:

| Part | Closed specialization |
| --- | --- |
| Role/assignment | role research; ChildSourceAssignmentV1's exact fields and leaf bounds, inputs=[], dependencies=[], both worktreeScope path lists empty. baseCommit/group match the plan/captures; requirement manifests bind this taskId |
| Outputs/review | Existing outputContract with declarations=[] and both aggregate caps zero; reviewInput null. expectedOutput describes the #742 report contract and scope, not a server artifact or task-success receipt |
| Permission profile | Exact existing requested/effective field sets; readPaths/writePaths/commands/externalSources/connectors/coordinationActions all []; nativeDelegation denied and accessChanges request-only |
| Tools | Exactly orkworks.research-report.submit.v1, orkworks.research-report.receipt.read.v1 and orkworks.research-report.context.read.v1, sorted in both requested/effective profiles; no #744 result/evaluation action or UI authority |
| Settings/model/evidence | Existing exact child-startup-settings, child-effective-configuration, child-model-policy and #741 capability-evidence leaves; role, operations, exact executable/settings/model and full retained artifact joins must match research |
| Startup ordering | Existing supplied/automatic order plus exactly one `{kind:"research-input",scopeDigest}` immediately after the supplied entry; 2–34 deliveryOrder entries. No review-input entry |

The three tool IDs are **new proposed logical permission identities**, not current
adapter support. submit maps only to the existing #742 submitResearchReport wire
operation. Receipt/context reads are limited to that same current assignment's
receipts and bounded report/task-version metadata permitted by #742; their actual
adapter/transport representation still needs separate closure. They use the
existing distinct ResearchReportCapability, not parent/report-workflow/#744/UI
credentials. No general HTTP, arbitrary session read, retained-file fetch or shell
bridge is granted. All operations still intersect current generation/role/scope,
rate and revocation rules before access or receipt lookup. Sources issue no token.

Use the unchanged `ORKWORKS CHILD INSTRUCTIONS v1` framing and boundary text,
rules/role/assignment/skill order and exact existing assignment block shape. Append
research task data as one LF, `ORKWORKS RETAINED RESEARCH INPUT v1` + LF, canonical
decimal UTF-8 payload length + LF, canonical `{scope,captures}` + LF, where captures
is the full ordered ResearchCaptureV1 list. The settings deliveryOrder and source
scope join this exact payload. This is task data, not a replacement system prompt.

Complete configuration remains at most 1 MiB and composed startup, including all
automatic content and this data block, at most **256 KiB**. Larger valid storage
records cannot qualify startup delivery. Reuse existing rule/skill/resource/model
provenance, mandatory-skill closure and lower verified adapter limits; do not
truncate, stream later, invent source fetch authority or substitute root evidence.
Actual model delivery/action mapping remains unqualified. Apply available native
guardrails and disclose instruction-only limits under the accepted standard;
these logical lists do not promise OS containment or reopen #740 research.

## Research plan and exact approval source

ResearchSourcePlanV1 is exactly:

```text
{schemaVersion:1,kind:"single-task-retained-file-research",parentPlanBinding,
 task:{taskId,configuration:{configurationId,configurationDigest},
       scope:ResearchScopeRef,initialPrompt,group},
 batches:[[taskId]],maxParallelChildren:1,scopeText,planDefinitionDigest}
```

ParentPlanBinding and group reuse their exact common shapes/bounds in the accepted
child source plan; initialPrompt equals configuration.assignment.description.
scopeText is nonempty UTF-8 through 4 KiB. All TaskKeys, configuration IDs, base
commit, repository/source/worktree/group/branch identities join original sources;
no parent/control value or active grant is hashed into the plan. The plan digest
uses `orkworks.research-source-plan.v1` + LF excluding only planDefinitionDigest.
Complete body is at most 64 KiB. Existing **2 MiB complete approved-plan** accounting
includes plan, full configuration/scope/capture bodies and every referenced
capability artifact occurrence. Equal retained occurrences have no sharing credit;
references already included in a body's bytes are not another copy of their target.

Proposed exact UI approval request:

```text
{schemaVersion:1,action:"orkworks.research-source-plan.approve.v1",
 plan:PlanRef,approvalId,reviewContextId}
```

ResearchPlanApprovalSourceV1 is exactly
`{schemaVersion:1,plan:PlanRef,approvalId,reviewContextId,sidecarGeneration,requestDigest}`.
Main originates approvalId; the existing nonsecret 64-hex reviewContextId binds
exact shown bytes and current workspace/sidecar. requestDigest uses
`orkworks.research-source-plan-request.v1` + LF over the complete request.
approvalDigest uses `orkworks.research-source-plan-approval.v1` + LF over the full
source body. Receipt is exactly
`{schemaVersion:1,plan,approvalId,approvalDigest,requestDigest,commit:CommitRef}`
and at most 4 KiB. This introduces a research-specific logical approval action,
not a renderer token or a change to existing child/root approval actions.

Retain complete reviewed bytes before approval; require actual current UI/source
authority before exact receipt lookup. A retained approval is no live grant.
Reapproval uses a new original approvalId/history, and reservations retain their
original ApprovalRef. Current grants and their excluded cutoff remain unqualified
control/producer responsibilities, not serialized source fields.

## Original reservation, allocation, attachment and assignment sources

ResearchReservationSourceV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,scope:ResearchScopeRef,reservationId,childSessionId,
 configurationId,configurationDigest,approval:ApprovalRef,
 expectedTaskVersion,assignmentTaskVersion,reservationDigest}
```

Both versions are positive existing task integers; checked expectedTaskVersion+1
equals assignmentTaskVersion. Freeze this original pair with the actual successful
one-attempt reservation, before filesystem/PTY work. It is not the later report's
live CAS version. Domain is `orkworks.research-source-reservation.v1` + LF excluding
reservationDigest. Original live control and approval must establish the values;
a parsed body does not prove that event. Current task/runtime/version ownership
remains for the separate research control contract, never an old direct-execution
control relabel or the R2 captured version-1 initial requests.

ResearchAllocationSourceV1 reuses the exact common fields
`{schemaVersion:1,reservation:ReservationRef,allocationId,group,
allocatedWorktree:SourceWorktreeBinding,allocationDigest}` with domain
`orkworks.research-source-allocation.v1` + LF excluding allocationDigest.
The real source owner must prove newly allocated plan-owned native objects and
exact approved path/branch/base. Existing/foreign directories are not adopted;
partial/unknown outcomes retain their owning attempt/capacity and do not create
a successful allocation source. No native allocation mechanism is chosen here.

ResearchAttachmentSourceV1 is exactly
`{schemaVersion:1,attachmentId,reservation:ReservationRef,
allocation:AllocationRef,assignmentIdentity:ChildAssignmentIdentity,attachmentDigest}`.
Its domain is `orkworks.research-source-attachment.v1` + LF excluding
attachmentDigest. The ordinary lineage owner must prove unique attachment of the
original reserved child/configuration/allocation in the same preparation outcome.
ChildAssignmentIdentity derives from the frozen successful reservation, never the
current live task counter or a copied ordinary session label.

ResearchAssignmentSourceV1 is exactly:

```text
{schemaVersion:1,assignmentIdentity:ChildAssignmentIdentity,
 plan:PlanRef,configuration:ConfigurationRef,scope:ResearchScopeRef,
 approval:ApprovalRef,reservation:ReservationRef,allocation:AllocationRef,
 attachmentId,criteria:CriterionSnapshot[],rubric:RubricSnapshot}
```

Use `orkworks.research-assignment-source.v1` + LF over the complete canonical body,
with no self-digest field. criteria/rubric are exact projections of the approved
configuration under #744; their presence does not create a result or evaluation.
Source/attachment publication must join actual complete fresh subject/ledger and
ordinary lineage preparation before any separately qualified runtime/report
capability delivery. Original native/runtime generation history still comes from
its real producer, not these immutable assignment facts. The report's first child
author/generations and current expectedTaskVersion are derived there at admission.

## Proposed source registry, accounting and ownership

These exact logical keys/wrappers are inputs to the **future complete research
catalog/version decision**, not a partial format-4 implementation or permission
to add tables. No row/manifest storage domain is selected by this source proposal.

| Object kind / identity | Body / source digest |
| --- | --- |
| research-source-capture / ResearchCaptureRef | ResearchCaptureV1 / captureDigest |
| research-source-scope / ResearchScopeRef | ResearchScopeV1 / scopeDigest |
| research-source-configuration / ConfigurationRef | Complete existing v3 configuration / unchanged configurationDigest |
| research-source-plan / PlanRef | ResearchSourcePlanV1 / planDefinitionDigest |
| research-source-evidence / `{task:TaskKey,referenceId,contentDigest}` | Exact common child-source-evidence fields/64 KiB raw occurrence/1 MiB aggregate bounds, with orkworks.research-source-evidence.v1 + LF domain |
| research-assignment-source / full ChildAssignmentIdentity | ResearchAssignmentSourceV1 / its source domain |

Object values use the existing objects envelope; complete bodies reconstruct keys
and source hashes under the explicit research joins. Raw capability artifact data
is separate from file captures and proves bytes, not authenticity of a role or
permission conclusion. No opaque JSON body becomes a qualified capability check.

| Metadata table / exact key | Exact value |
| --- | --- |
| research_plan_approvals / `{plan:PlanRef,approvalId}` | `{schemaVersion:1,body:ResearchPlanApprovalSourceV1,digest,receipt,lastCommit}` |
| research_reservations / TaskKey | `{schemaVersion:1,body:ResearchReservationSourceV1,lastCommit}` |
| research_allocations / ReservationRef | `{schemaVersion:1,body:ResearchAllocationSourceV1,lastCommit}` |
| research_attachments / childSessionId | `{schemaVersion:1,body:ResearchAttachmentSourceV1,lastCommit}` |

Each complete metadata key/value is at most 16 KiB. All rows retain the smaller
source/row caps, including the 2 MiB containing row ceiling. Exclude only named
outer lastCommit and approval receipt.commit from their committed/accounting
projection; source objects use the existing objects projection. Every excluded
link must resolve its original complete proof under the eventual reviewed format.
No component may choose a format from a supplied CommitRef or table presence.

Propose charging all six research object kinds and four metadata families to the
**same existing child-source-retained-bytes** run/workspace pools, not another
allowance: 8 MiB/run and 64 MiB/workspace, reserved zero, exact canonical key plus
committed-value projection per stored occurrence. Share with every other admitted
child-source occurrence in that owner; no reset by epoch, role, scope, report or
ordinary end. Report descriptor/head/receipt charges remain in their existing
report resources; capture/scope/approval/source bytes are not free report metadata
or #744 receipt credit. The existing 32 MiB run preparation/report aggregate also
includes these research source records and applicable controls; each smaller cap
wins. All command/proof/full-store/index/physical limits remain independent.

Fresh scope initialization, full workspace/run resource inventory, semantic
command/read/write sets and direct/reverse index publication remain unclosed.
Missing ledgers or legacy history are unavailable, never zero or repairable by a
source constructor. The accepted format-3 fresh-report-ledger packet is not a
whole research catalog initializer. No automatic import, migration or retokening
of earlier root/child/report sources or receipts is supplied.

The source owners must preserve these logical relationships: plan→configuration/
scope/evidence; scope→captures; assignment→its original plan/approval/reservation/
allocation/attachment/scope; and report→its approved capture/excerpt bindings.
These are retention obligations, not newly admitted dependency-purpose strings.
The complete catalog/control proposal must define exact edge/direct metadata
indexes and their insertion/removal/purge/recovery boundary before graph fixtures.
Retain source bytes while any historical plan/assignment/report requires them;
ordinary end or worktree loss cannot delete them. No automatic purge, source
eviction, cross-workspace sharing or #745 custody bypass is introduced.

## Owner choices and bounded next qualification

Accept or revise these proposed source choices:

1. One-task retained UTF-8-file research, exact questions/excerpts/scope and
   containing caps; no URL/live/binary input or reviewer/parallel plan in this slice.
2. Research-specific source/approval domains, original expected/frozen reservation
   pair, six object kinds/four metadata families and exact common identity joins.
3. Three logical report tool IDs, research-input startup/settings variant and
   payload, using unchanged v3 configuration/report protocols and startup caps.
4. Shared existing child-source pool/accounting and explicit original-source
   retention, without a new allowance or old-store adoption.

After scoped acceptance, independent documentation **source-component** bytes and
denials may validate this grammar only. They must label actual capture origin,
UI approval, reservation/allocation/attachment/runtime authority and complete
publication as unqualified. A new complete transaction fixture must still wait
for research current controls, active collection/expectation/consumer indexes,
one complete catalog/version/initializer, original generation/ordinary bridge
and atomic first-report effects. The actual source-owner modules and native
participation boundaries must be named there; these type names are not factories
or an existing runtime implementation.

Keep #1689 open until its remaining source/control/catalog/effect criteria and
applicable acceptance are met. #1089 Gate 2 remains Investigate; real finish/cancel,
terminal consumers/indexes, UI/lease/storage/custody/native evidence, feature-specific
capabilities and separately approved executable plans remain independent. Existing
logical #741/#742 acceptance and #740's closed research need no repeat approval.
This document can be reverted without store effects; no persistence downgrade
or rollback strategy is claimed for a future implementation.

## Bounded source-design review

Two fresh read-only Luna checks found no material correctness or builder gap for
the proposed source-component slice. They checked field/counter bounds, digest
ordering, scope/report/excerpt joins, original versus live identity, startup and
shared-pool accounting. Prepared-input, actual capture/approval/native origin,
tool mapping, catalog/index/control and publication obligations remain explicitly
unqualified. The reviews performed no fixture/runtime/native execution and do
not accept the author's new source choices on the owner's behalf.

## Scoped owner acceptance and first component packet

On 2026-11-12 the owner answered “go ahead” to the explicit request to accept
the four choices above **for offline source-component fixtures only**. This
records that acceptance; it does not select a storage format or authorize
runtime/native execution, research controls, adapters or publication.

The first bounded packet covers ResearchCaptureV1, ResearchQuestionV1,
ResearchExcerptV1 and ResearchScopeV1 only. Configuration/startup, plan/approval,
reservation/allocation/attachment/assignment, shared-pool accounting and report
consumers remain outside that packet's qualification. The accepted choices for
those sources need no repeat acceptance within this scope; their evidence is
still required. See the [capture/scope component evidence](../../validation/research-capture-scope-components.md).

## Configuration/startup component continuation

The [research configuration/startup packet](../../validation/research-startup-components.md)
continues that scoped acceptance with synthetic complete v3 configuration,
unverified common capability artifacts and exact retained-input composition.
It qualifies neither original plan/approval/launch sources nor native delivery,
prepared input, current controls, storage catalog/accounting or report consumers.
No accepted protocol/domain or original component artifact is changed.

## Original-source chain component continuation

The [research source-chain packet](../../validation/research-chain-components.md)
continues that scoped acceptance with original plan/approval/reservation/
allocation/attachment/assignment source-shape joins and research source domains.
Its prepared-input and CommitRef bindings are explicitly unqualified; they do
not select missing codecs, event history or a storage format. Metadata width
examples and declared source-body measurements are not command/proof/ledger/
catalog or consumer qualification. Current controls, actual source-owner/native
events and atomic first-report publication remain gated.
