---
type: Design Proposal
title: Retained child reviewer source and preparation records
description: Proposed bounded single-task retained-data child source codecs, exact approval/reservation/allocation/attachment joins, fresh ledgers and source consumer membership for artifact qualification.
tags: [orkworks, hierarchy, child-sources, review, preparation, custody]
status: draft
---

# Retained child reviewer source and preparation records

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `98ce5503`, after the
[artifact acceptance/source audit](../../validation/evaluation-artifact-source-closure.md).
**New source/profile/codec/budget choices for owner review. Drafting or merging
does not accept them, pass Gate 2, authorize dependent fixtures, or create a run,
grant, worktree, child, database, migration or runtime implementation.**

## Outcome and selected slice

Retain exact approved child source bytes and original producer outcomes so an
artifact writer can resolve the target/reviewer identities, input/configuration
and fresh ownership without guessing from ordinary session metadata. Keep plan
approval, successful reservation, filesystem allocation, unique attachment and
source preparation distinct; no receipt or source digest creates authority.

The [existing one-task description](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan)
is not a closed plan codec, and accepted root-only leaf codecs cannot be silently
relabeled as child records. Propose a selected **single-task retained-data child
source slice**, not a general child/orchestration schema:

- One task, one independent worktree group and one batch per exact plan, with
  plan maxParallelChildren=1. Retain complete original definitions; a multi-task,
  dependency-chain or worktree-reuse plan is unsupported, never trimmed to fit.
  Existing run-wide capacity, plan/revision limits and exact approval still apply.
- A retained-data worker uses role `verification`, null reviewInput, inline
  assignment inputs and an empty output contract for a no-materialized-deliverable
  task. A reviewer uses `review` or rubric-eligible `verification`, nonnull exact
  reviewInput and its one required assignment_evaluation server-artifact output.
  These are source-codec examples, not evidence that empty outputs prove success.
- No model-visible shell, live filesystem, external source, connector, Git,
  native delegation or child coordination action. Retained review input is
  delivered as exact bounded startup task data under the existing scoped read
  authority and a verified adapter; a prompt restriction alone is not enforcement.
- Review targets are child results with matching retained source records, no
  materialized outputs, and zero selected output IDs. The first evidence slice
  supports existing research-report evidence only. Root/performance review,
  arbitrary path/output evidence and other artifact producers stay unsupported.

This narrowing, the selected leaf representations, plan/source domains and
source-budget default need owner acceptance. Broader profiles keep their existing
logical contracts and remain separate codec/qualification work. No legacy child
or old synthetic fixture is upgraded into this profile.

## Common encoding and owned source types

Use #741 canonical UTF-8 JSON, strict duplicate/unknown-key rejection, IDs,
Digests, lossless source JSON integers and storage Counter TEXT unchanged. Preserve
meaningful array order; sort only specified unique sets. No credentials, hidden
reasoning, raw production transcripts or mutable path fetches enter source data.
NativePath, repository/worktree object identity, SnapshotProvenanceV1 and
ExecutableIdentityV1 use the accepted common leaf definitions in the
[root source codec](2026-10-26-root-preparation-source-codecs-design.md),
[root nested source grammar](2026-10-27-root-bootstrap-source-schemas-design.md)
and #745 binding contract. Reuse those **explicit common shapes**, not root-only
assignment/permission/coordination semantics.

The following reference objects are closed (type names do not add JSON fields):

| Type | Exact fields |
| --- | --- |
| PlanRef | workspaceId, runId, planId, planRevision, planDefinitionDigest |
| TaskKey | workspaceId, runId, planId, planRevision, taskId |
| ConfigurationRef | task:TaskKey, configurationId, configurationDigest |
| ApprovalRef | plan:PlanRef, approvalId, approvalDigest |
| ReservationRef | task:TaskKey, reservationId, reservationDigest |
| AllocationRef | reservation:ReservationRef, allocationId, allocationDigest |
| InputRef | task:TaskKey, inputDigest |

Revisions use existing 1..64 plan / 1..2^31−1 task bounds. ResultRef,
EvaluationReference and full child AssignmentIdentity retain their exact #744
fields. runId remains admissionEpoch + '-' + positive canonical runSequence,
never a caller-selected replacement. References resolve complete retained bytes,
not only matching digest strings. A reference is neither a bearer nor a read grant.

## Selected child v3 configuration and nested leaves

Retain the full existing OutputAssignmentConfiguration schemaVersion 3, including
outputContract and nullable reviewInput, all mandatory rules/skills/resources,
repository/source-worktree descriptors, model/evidence/settings, exact rendered
instructions and configurationDigest. The unchanged digest is
`orkworks.assignment-configuration.v3` + LF + canonical full configuration excluding
only configurationDigest. The 1 MiB configuration / 256 KiB composed startup bounds
and lower verified adapter limits remain ceilings, not promises of simultaneous fit.

Close the selected child assignment as ChildSourceAssignmentV1, exactly:

```text
{schemaVersion:1,label,description,inputs,expectedOutput,criteria,
 dependencies:[],worktreeScope:{readPaths:[],writePaths:[]},
 worktreeGroupId,baseCommit}
```

Use the root assignment's explicit label (512 bytes), description (4 KiB),
expectedOutput (2 KiB) and ordered criteria/input leaf bounds as new child
specializations. Only `{kind:"inline",id,content,contentDigest}` inputs are legal
here: 0..32 unique IDs, exact nonempty UTF-8 through 4 KiB and raw SHA-256. No
opaque reference variant or future task dependency is admitted. Reviewers have
inputs=[]; their readable data is the typed reviewInput snapshot below.
worktreeGroupId is an ID. baseCommit is exactly 40 or 64 lowercase hex characters,
with the registered repository's actual Git object format, object existence and
clean base verified by the native source owner before approval/reservation.
No hash-looking string alone establishes a Git object or filesystem state.

The remaining selected leaves use the exact field sets/bounds of the cited root
nested grammar with only these explicit child substitutions, requiring review:

| Leaf | Child specialization |
| --- | --- |
| Permission profile | Exact tools/readPaths/writePaths/commands/externalSources/connectors/coordinationActions/nativeDelegation/accessChanges fields; all access lists except tools are []; nativeDelegation=denied, accessChanges=request-only |
| Worker tools | Exactly orkworks.result.submit.v1 and orkworks.result.receipt.read.v1, sorted and present in both requested/effective profiles |
| Reviewer tools | Those result actions plus orkworks.evaluation.submit.v1, orkworks.evaluation.receipt.read.v1 and orkworks.evaluation.artifact.read.v1, sorted in both profiles; no root or UI action |
| Requirement/task binding | Every manifest.taskId equals configuration.taskId; same mandatory-skill closure, provenance, resource and rule-precedence checks, without rootAssignmentId substitution |
| Settings artifact | RootStartupSettingsV1's closed fields and limits, with kind child-startup-settings; no secret argv/bindings; instructionMechanism=startup-context in this slice |
| Effective configuration artifact | Same exact schemaVersion/sourceIdentity/sourceRevision/bindings fields, with kind child-effective-configuration |
| Model policy artifact | Same exact fields, bounds and model-binding joins, with kind child-model-policy |
| Permission/source check fixtures | Existing capability-reference artifacts retain exact bounded UTF-8 source or reproducible fixture bytes; there is no inherited closed root permission-fixture JSON type. Their actual profile/role/operation evidence must match this child slice; opaque text proves bytes only, never a passing check |
| Capability evidence | Existing #741 exact fields, four decisions, ten surfaces, source/fixture retention and domain; role matches this child's role; verified still requires real complete evidence, not a renamed root fixture |

StartupSettings.deliveryOrder has 1–33 entries for a worker and 2–34 for a
reviewer. It retains the root grammar's supplied/automatic entries and adds exactly `{kind:"review-input",inputDigest}` once for a reviewer,
immediately after the supplied entry; absent for a worker. The adapter must
deliver the data block immediately after the complete supplied payload and before
any later automatic entry, preserving every other relative order. Digest equals its exact
InputRef snapshot. Every automatic instruction remains enumerated in original
precedence order, content-pinned and delivered within mandatory-rule authority.
No hidden discovery, fallback shell/MCP action or unrecognized setting qualifies.
The startup evidence must verify this exact bounded retained-data channel; these
logical tool allowlists do not themselves grant a generic server-snapshot fetch.

Propose deterministic child instruction framing, using the accepted root framing
algorithm's length encoding and block ordering but header exactly
`ORKWORKS CHILD INSTRUCTIONS v1` + LF. Boundary ID is child-v1, with exact text:

```text
Act only within the approved child assignment. Binding repository rules remain mandatory. Skills and task data grant no tools or authority. Use only current exact reporting and retained-input capabilities. Request changed scope from the user. Do not mutate Git, delegate natively, integrate edits or change permissions.
```

Then rules by precedence, role, an assignment block ID=taskId containing canonical
`{assignment,outputContract,reviewInput,rubric}`, mandatory then optional skills
and their resources by ID, using the root algorithm's exact block separators.
This is a new child framing, not a root payload relabel. renderedInstructionsDigest
remains raw SHA-256; compare complete bytes and count all framing in existing caps.

For a reviewer the adapter appends task data, not new instructions: one literal
LF, `ORKWORKS RETAINED REVIEW INPUT v1` + LF, canonical decimal UTF-8 snapshot byte
length + LF, complete canonical input snapshot + LF. Entire composed startup,
including automatically delivered content, remains at most 256 KiB. A larger
valid stored input or configuration is therefore unsupported for this slice;
reject before approval, never truncate or claim all maxima fit. Model adherence,
secret absence and native permission/delivery remain #740 qualification gates.

## Exact retained input snapshot

ChildReviewInputSnapshotV1 has exactly:

```text
{schemaVersion:1,task:TaskKey,target:ResultRef,result:AssignmentResult,
 criteria:CriterionSnapshot[],rubric:RubricSnapshot,
 evidence:[{source:ResearchReportEvidenceSource,report:ResearchReport}],inputDigest}
```

inputDigest uses `orkworks.child-review-input.v1` + LF over the canonical object
excluding only inputDigest. Complete source body is at most 256 KiB. The task key
exists before configuration hashing, so settings may pin inputDigest without a
configuration/self-digest cycle. The snapshot never contains the reviewer's own
configuration digest, future session, bearer or mutable currentness flag.

Resolve target's exact retained child-assignment source, configuration, accepted
result and approved criteria/rubric. Result/body identity and source-domain digest
must match target. Compare criteria to the source's #744 stable-ID-sorted projection
of its original configuration criteria; preserve the rubric's declared dimension
order. Snapshot report/evidence pairs cover exactly reviewInput.evidenceSources,
sorted uniquely by source-key digest, with exact existing report identity, version,
raw byte length, content/source digest and producer/custody proof. No transitive
reference crawl or original captured excerpts is delivered merely because a
report descriptor cites them. Those remain under their source owner's custody.

The selected reviewer has outputDeclarationIds=[] and maxReadBytes is a positive
lossless integer at most 256 KiB. Derive its bound from complete canonical result,
criteria/rubric and every selected report size; compare against approved/verified
limits and the stricter complete startup package. Source change before reservation
follows existing admission revalidation; later source changes preserve immutable
bytes and use existing historical/currentness/health rules, never rewrite snapshot.
Input bytes and digests are data/provenance, not current read or publication grants.

## Single-task plan and exact approval

Propose ChildSourcePlanV1, a new selected ordinary-child plan source codec:

```text
{schemaVersion:1,kind:"single-task-child-source",parentPlanBinding,
 task:{taskId,configuration:{configurationId,configurationDigest},
       input:InputRef|null,initialPrompt,group},
 batches:[[taskId]],maxParallelChildren:1,scopeText,planDefinitionDigest}
```

parentPlanBinding is exactly the existing ten #741 fields: runId,
parentSessionId,planId,planRevision,runDefinitionDigest,bootstrapConfigurationDigest,
repositoryBindingDigest,sourceWorktreeBindingDigest,preparationRevision,inputDigest.
Its plan/workspace identity joins the enclosing source/run and configuration;
no extra schemaVersion or plan digest is inserted into that accepted binding.
The plan's TaskKey/PlanRef derive from those bindings and the retained run's workspace.
Use `orkworks.child-source-plan.v1` + LF over the complete canonical plan excluding
only planDefinitionDigest. This chooses a previously unclosed selected plan encoding;
it does not reuse/rehash legacy coordinator PlanRevision or claim support for a
broader ordinary-child plan definition.

configuration references exactly one separately retained full child configuration.
input is null for the worker and the exact snapshot for a reviewer. initialPrompt
is exactly configuration.assignment.description; system instructions stay separate.
scopeText is nonempty UTF-8 through 4 KiB. group is exactly
`{groupId,branchName,absolutePath:NativePath,baseCommit}`; IDs/base match assignment,
branchName is nonempty exact UTF-8 through 256 bytes, validated as an allowed Git
ref by the source owner, and path uses accepted NativePath bounds/encoding.
The path/branch are deterministic source-owned choices displayed before approval;
no existing/foreign worktree is adopted, no main branch or shared group is allowed.
Actual worktree object identity is recorded only after verified allocation below.

Plan body is at most 64 KiB. Its existing 2 MiB complete approved-plan budget counts
this body plus full referenced configuration, input snapshot and every referenced
capability/settings/model artifact occurrence. No hash reference makes bytes free;
no alias/deduplication discount. Validate complete source closure before publishing
an immutable proposal, and again before approval/reservation. This is a stricter
selected slice, not an increase to general plan/configuration limits.

Store proposal and its sources before UI review. Existing Electron-main authority
reviews the exact immutable bytes/context. Proposed approval request is exactly:

```text
{schemaVersion:1,action:"orkworks.child-source-plan.approve.v1",plan:PlanRef,
 approvalId,reviewContextId}
```

This names the existing exact-plan approval operation, not a new bearer or child
permission. approvalId is main-originated; reviewContextId is a nonsecret fresh
64-hex volatile main/sidecar context bound to the exact shown bytes and current
workspace. Source owner revalidates it and current authority through its own
excluded approval cutoff; a source receipt is not proof that cutoff was qualified.
requestDigest uses `orkworks.child-source-plan-request.v1` + LF over full request.
Any byte/context drift requires a new proposal/context and exact review.

ChildPlanApprovalSourceV1 is exactly
`{schemaVersion:1,plan:PlanRef,approvalId,reviewContextId,sidecarGeneration,requestDigest}`.
Its digest uses `orkworks.child-source-plan-approval.v1` + LF over the whole body.
The receipt is exactly `{schemaVersion:1,plan,approvalId,approvalDigest,requestDigest,
commit:CommitRef}`, at most 4 KiB. Reconstruct the request and compare exact bytes
and digest on replay. Each reapproval has a new immutable approvalId/history row;
reservations retain their original ApprovalRef. Current PlanExecutionGrant is
server-held #742 authority, not a field that makes this archive active. UI auth
precedes retry lookup; old receipts restore no grant after pause/revoke/restart.

## Reservation, allocation, attachment and assignment source

These source records are historical producer outcomes, not a replacement mutable
run/task/grant state machine. Only the respective #742 transition may emit them;
models/clients cannot upload records or pick source facts. A successful event and
its source publication must share the owning atomic/recovery boundary. A copied
boolean, task counter, path or hash cannot substitute for that evidence.

ChildReservationSourceV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,reservationId,childSessionId,
 configurationId,configurationDigest,approval:ApprovalRef,
 assignmentTaskVersion,reservationDigest}
```

The sidecar reserves fresh IDs before filesystem/PTY work. Freeze the resulting
successful reservation taskVersion as assignmentTaskVersion with the exact plan/
configuration/approval and launching transition, per accepted #742 rules. It is
1..2^31−1, causes no extra counter increment and never updates to live taskVersion.
Hash with `orkworks.child-source-reservation.v1` + LF excluding only its digest.
Exact launch retry returns the original committed IDs/snapshot; ambiguous, missing,
legacy or conflicting reservation state is unavailable, never recaptured/backfilled.

ChildAllocationSourceV1 is exactly:

```text
{schemaVersion:1,reservation:ReservationRef,allocationId,group,
 allocatedWorktree:SourceWorktreeBinding,allocationDigest}
```

Hash with `orkworks.child-source-allocation.v1` + LF excluding allocationDigest.
The allocation owner verifies this exact planned path/branch/base, newly created
plan-owned worktree, repository family and native object identity; group equals
the approved one and allocatedWorktree is the full immutable descriptor for the
created group. SourceWorktreeBinding from configuration still identifies the
original source; it cannot be swapped for the new allocation descriptor. Partial
or foreign filesystem results retain interrupted allocation/no-go, not attachment
or grant delivery. No automatic cleanup/retry or cross-plan group reuse is added.

ChildAttachmentSourceV1 is exactly:

```text
{schemaVersion:1,attachmentId,reservation:ReservationRef,
 allocation:AllocationRef,assignmentIdentity:ChildAssignmentIdentity,
 attachmentDigest}
```

Hash with `orkworks.child-source-attachment.v1` + LF excluding attachmentDigest.
The ordinary-session lineage producer uniquely attaches the pre-reserved child
session to this reservation/configuration/allocation in its owning durable event.
Full child identity derives from original reservation snapshot, complete configuration
and that attachment, not worktree names/timestamps/current counters. The immutable
attachment remains after ordinary end; resume needs fresh actual runtime authority,
never a replacement historical identity. Duplicate/foreign attachment is unavailable.

ChildAssignmentSourceV1 has exactly:

```text
{schemaVersion:1,assignmentIdentity:ChildAssignmentIdentity,
 plan:PlanRef,configuration:ConfigurationRef,approval:ApprovalRef,
 reservation:ReservationRef,allocation:AllocationRef,attachmentId,
 input:InputRef|null,criteria:CriterionSnapshot[],rubric:RubricSnapshot}
```

Its source digest is `orkworks.child-assignment-source.v1` + LF over the complete
canonical body, without a self-digest property. criteria/rubric are the exact #744
projections of the original full configuration, including stable-ID criteria order.
This is a retained provenance object, not a reporting capability. Publish it only
with proven unique attachment and complete fresh source preparation; reserved IDs
alone cannot create a valid #744 subject, result or reviewer stream.

## Closed format-4 source registry and commitment

Complete the previously deferred child-source portion of the one accepted artifact
profile catalog; do not create an alternate format-4 profile selected by row presence.
New required kinds/tables/purposes below need explicit owner acceptance. All readers
must support the whole closed catalog, even empty tables, or report unavailable.
No old v3 store or old v4 subset is imported as admitted child history.

New objects use the existing objects envelope; full logical bodies are canonical
BLOBs, raw/source digests are checked, and each body reconstructs its key through
explicit source joins:

| Object kind / exact identity | Body and digest |
| --- | --- |
| child-source-plan / PlanRef | ChildSourcePlanV1; workspace comes from exact retained run; plan-definition domain above |
| child-source-configuration / ConfigurationRef | Complete child v3 configuration, unchanged configuration digest; run joins its exact plan, not a caller field added to configuration |
| child-review-input / InputRef | ChildReviewInputSnapshotV1 and its input domain |
| child-source-evidence / `{task:TaskKey,referenceId,contentDigest}` | `{schemaVersion:1,task,referenceId,contentBase64,sizeBytes,contentDigest}`; raw exact cited artifact bytes, strict padded Base64; source object digest uses orkworks.child-source-evidence.v1 + LF over complete body |
| child-assignment-source / full child AssignmentIdentity | ChildAssignmentSourceV1 and complete source domain above |

Source evidence is internal #741 snapshot retention, not arbitrary producer upload.
Each capability reference resolves one exact evidence object in this task; retain
only referenced IDs and reject extra/dangling content. At most 16 references,
64 KiB raw bytes each and 1 MiB raw aggregate; every canonical/Base64 wrapper also
counts. sizeBytes is a lossless source JSON integer equal to the decoded byte length,
0..65536. Primary and other fixture references retain bounded exact UTF-8 bytes;
selected child settings/model/effective projection artifacts must decode into their
explicit types with exact raw and semantic joins. Other reproducible check
fixtures keep their exact source bytes; no generic JSON object is interpreted as
configuration or a passing permission test. Root-only fixtures cannot become child proof by
renaming. Source-codec validation does not authenticate capability conclusions.

New metadata tables (each complete key/value pair at most 16 KiB):

| Table / exact key | Exact value |
| --- | --- |
| child_plan_approvals / `{plan:PlanRef,approvalId}` | `{schemaVersion:1,body:ChildPlanApprovalSourceV1,digest,receipt,lastCommit}` |
| child_reservations / TaskKey | `{schemaVersion:1,body:ChildReservationSourceV1,lastCommit}` |
| child_allocations / ReservationRef | `{schemaVersion:1,body:ChildAllocationSourceV1,lastCommit}` |
| child_attachments / childSessionId | `{schemaVersion:1,body:ChildAttachmentSourceV1,lastCommit}` |

A task has one immutable reservation and allocation in this slice; the reservation
already binds childSessionId, and attachment key/identity must equal that reserved
value. Together these provide both lookup directions without another hidden index.
Source/plan/configuration/input/evidence objects retain their smaller body limits;
every complete canonical key/value pair also obeys the existing 2 MiB row ceiling.
Maximal source bodies need not fit with their envelope; deny instead of raising it.

Use accepted format-4 row/manifest domains and exact object projection. For the
four metadata tables exclude only outer lastCommit, plus receipt.commit on the
approval table; all other fields and internal source digests remain committed.
Attach/validate excluded CommitRefs through the owning manifest, never hash cycles.
A linked proof/header establishes integrity, not event authority or whole-store
rollback protection. Existing root preparation/run/parent/input records use their
already accepted closed logical codecs under explicit fresh format-4 preparation;
this supplies no conversion of old receipts/commitments or root-only child substitute.

## Consumer membership and retention

Register new purpose `child-source-provenance`, with exact full ObjectRefs:
plan→configuration/input; configuration→its cited source-evidence objects/input;
child-assignment-source→its exact plan/configuration/input. Null input produces no
edge. Each endpoint/kind/task/digest must match the source body; no arbitrary edge
is accepted. Config/input sources are not separate mutable latest aliases.

The snapshot's existing review-input edges protect its exact worker result and
selected assignment-evidence-source objects. Existing evidence-source→report
custody protects the report versions; no transitive crawl is granted. Plan/source
metadata rows are retained as one PlanRef source group, including all original
approvals, reservation, allocation and attachment. Root/run preparation and parent
source records remain protected while any retained plan/source group names them;
the run owner resolves that bounded membership from exact PlanRefs, not generic
ObjectRefs fabricated for metadata tables.

Discover all incoming/outgoing consumers from the source-owned complete index,
including historical configurations, snapshots, approved plans and retained child
sources; selected/current rows alone are insufficient. Register these kinds with
the artifact/custody/health/retention owners and use their actual bounded closures.
Unknown #743/preparation/proposal/launch/learning consumers are unavailable/no-go,
not assumed absent to enter this slice. Preserve detail-only snapshot invalidation
and existing subjectRevision/summary/material distinctions on every affected source
publication. Stored digests never establish source freshness or a current read grant.

Initial source groups have no standalone pruning/purge: keep full approved bytes,
source references and history even after child/parent end. The accepted connected
artifact no-purge rule and #745 workspace deletion/shared-learning fences remain
binding. Native cleanup/forget/quarantine must preserve required immutable source
proof or fence dependent reads; do not silently reconstruct an attachment from a
remaining session. No new joint deletion/release path is selected here.

## Atomic source preparation and finite accounting

Propose new retained resource `child-source-retained-bytes`, at most **8 MiB per
owner run / 64 MiB per workspace**, inclusive across historical plans/epochs. It
has no assignment/provisional-session pool: proposal/configuration sources exist
before a valid attached AssignmentIdentity. Its existing quota_usage value has
checked canonical used Counter TEXT, reserved="0" and lastCommit. Required fresh
run/workspace initialization belongs to explicit root/profile preparation, never
an opportunistic missing-row repair by a child source action.

Charge complete canonical key/value projections of every source object, approval,
reservation, allocation and attachment row above, excluding only committed link
paths. Count all retained occurrences/wrappers; no dedup/head-only/epoch reset or
release on ordinary end. Root preparation, report, result/evaluation/artifact,
read-work, receipt, summary/lineage/proof and physical/index budgets remain separate.
No accepted quota/store ceiling is raised. The one-task source and no-pruning costs
are new proposed policy; shared capacity can deny future plans without a purge route.

| Source-owned transition | Exact new source/preparation effects, plus all owning control/dependency/summary/quota/proof effects |
| --- | --- |
| Immutable proposal publication | Insert exact configuration, cited evidence objects, optional input snapshot and plan source with complete new provenance/review-input edges; retain all referenced source bytes and charge run/workspace source ledgers before showing a reviewable proposal |
| Exact UI approval/reapproval | Insert immutable approval source/receipt and the real #742 exact-plan approval/grant transition under its own authority cutoff. Source row alone issues no grant |
| Successful reservation | Insert original immutable reservation/snapshot together with the normal #742 launching/task-version, group/run-capacity and one-attempt effects, before filesystem work; overflow/conflict leaves no partial reservation |
| Verified allocation | Insert exact allocation outcome with native object/path/base verification and owning interrupted/success disposition; never infer success from an existing directory |
| Unique attachment + source preparation | Insert attachment and child-assignment-source, full provenance membership and initial subject/ledgers together with exact ordinary lineage/#742 attachment outcome. Only then can separately qualified actual runtime/read/report capability issuance occur |

At the final preparation initialize this provably new subject's summary_state with
subjectRevision "1" and existing no-result summary/checkpoint capacity branches;
never manufacture passing/empty criteria or reconstruct missing lineage. Initialize
all required assignment record/payload/reporting-receipt/artifact-retained/artifact-
read-work quota rows at zero, with exact owner mapping. Existing run/workspace rows
are preserved, with checked aggregate deltas; new source/archive metadata is not
charged to the zero target record or artifact-content pools. A reviewer gets no
child evaluation slots until its actual target evaluation publication; existing
source actions do not pre-consume candidate/target receipt credits or imply a result.

The authoritative #742 control transitions, child lineage writer, initial root/run
source and complete consumer effects are prerequisites, not optional symbolic
omissions. Source-owner native event publication and these rows must have one
proven atomic/recovery outcome; if underlying control/state/custody codecs or the
common transaction boundary are missing, do not produce a supposedly complete
manifest or source-valid fixture. This design closes retained source representations
and required joins, not every mutable orchestration state or native grant mechanism.

Add proposed command kinds child-source-propose, child-source-plan-approve,
child-source-reserve, child-source-allocate and child-source-attach. Their complete
union coalesces all changed owner/resource rows and actual dependency/subject/
summary/control/source indexes, header/new proof and any proven unreferenced-proof
changes. Reject missing/extra/duplicate effects or unexplained link churn. Count
all key/value/source copies, control/proofs, row visits and native costs; no deletion
credit, post-commit metadata repair or caller-supplied affected set is allowed.
All source actions remain excluded from #744 write claim-first; queue admission,
a durable archive or a response check does not waive current native authority.
No SQL/fsync across ordinary core locks or delayed stop is authorized. #1184's
read-work problem remains separate; no native fence is claimed for these actions.

Capacity failure proposes bounded 409 `child-source-capacity-exhausted`, preserving
complete old source/control state, protected credits and existing outcome evidence.
It is no quality failure, automatic retry or destructive cleanup. Missing/corrupt
history, consumer coverage, ledger or uncertain outcome remains unavailable.
No canonical receipt/body is reconstructed from a high-water counter or latest ID.
Interrupted allocation remains interrupted until explicit supported reconciliation;
no owner/session/path adoption or replacement worktree creation occurs automatically.

## Compatibility, owner choices and handoff

New choices for scoped review: the single-task retained-data/no-live-access profile,
selected child leaf/framing and input delivery, exact new source/approval domains
and immutable record joins, closed format-4 source catalog/purpose additions,
8 MiB run / 64 MiB workspace no-pruning source pool and explicit preparation order.
Accepted source/identity/request/report/artifact/claim semantics remain unchanged;
new approval request identity only closes this selected exact-plan operation.
No older plan, configuration, root codec or synthetic child identity is upgraded.

The [qualification handoff](../plans/2026-10-31-child-review-source-qualification.md)
stops at owner decision, then exact codec/component and original source creation/
consumer/control/authority closure, before complete independent transaction fixtures.
Existing conditional sources remain unchanged evidence. #1183 remains open for
review and source-owner agreement; #1176 Gate 2, #1184, #740 and native/executable
plans remain separate. No source-valid complete profile, database/migration,
fixture/checker, model/native probe, launch or runtime implementation is delivered.


## Proposed debit accounting consumer — 2026-10-31

The [#1210 representation proposal](2026-10-31-artifact-read-debit-proof-design.md#recovery-accounting-completeness-and-retention)
adds a proposed direct historical accounting consumer: artifact_read_debits rows
retain exact producerSource, artifact/resolved evaluation and direct commit-proof
validation joins. Reconcile that closed-table retention membership with this
source/consumer registry before complete-state qualification. It is not a new
ObjectRef dependency purpose, evaluation-freshness consumer or current-summary
input. The initial no-purge profile supplies no debit/source eviction or repaired
zero ledgers. These new representation/retention choices need scoped acceptance;
this annotation does not accept the broader #1183 proposal or pass native gates.
