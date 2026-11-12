---
type: Design Proposal
title: Research prepared input and current-control prefix
description: Proposed source-owned research input, current control, collection and version contract through initial report admission, with explicit producer and publication gates.
tags: [orkworks, research, preparation, controls, qualification, history]
status: draft
workflow_status: proposed-for-scoped-prefix-review
---

# Research prepared input and current-control prefix

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `60beae2a3f53d8ee4ec6fec4f18d2bcab20c6d15`.

**New proposal for written owner review.** The earlier scoped acceptance covers
[immutable research sources](2026-11-12-research-source-codecs-design.md), not the
new input/control choices here. This document supplies a selected first-report
prefix and named ownership/effect boundaries. It is not an executable plan,
complete transaction/history fixture, storage-format choice or runtime/native
authorization. Gate 2 remains Investigate. No dependent fixtures accompany it.

## Scope, preserved contracts and remaining product coverage

Select the accepted one-task retained-file research source profile. Preserve its
configuration, question/capture/scope/plan/approval/reservation/allocation/
attachment/assignment bytes and domains. Preserve #742 report descriptor,
request/head/receipt, capability, rate, correction, retry and child-cutoff rules.
The [selected direct-execution controls](2026-11-02-child-launch-control-design.md)
remain unchanged; research never enters them by setting researchNeeded true.

Close source shapes and local control effects for initial research assessment,
proposal, approval, one original launch attempt and first report selection.
Material UI clarification, report-consuming research children, parallel/reviewer
research, coordination-turn/result/recovery codecs, synthesis/execution handoff,
real finish/cancel effects and full consumer/catalog publication are separate
closures. Their accepted logical behavior is not removed. Unknown or broader
records are unavailable for this prefix, never flattened into null/empty fields.

New choices below are the author's proposal, not inferred owner acceptance.
Rejected alternatives are an opaque prepared-input digest; reuse of the false
direct-execution decision; a second runVersion owner; a report-shaped context
standing in for the original attempt; and inferring zero consumers from no head.

## Inspected owners and proposed responsibilities

These are bounded source facts, not an exhaustive search for differently named
writers or evidence of installed behavior:

| Existing source | Observed responsibility / limit |
| --- | --- |
| [taskmaster/coordinator.rs](../../../crates/orkworksd/src/taskmaster/coordinator.rs) and [coordinator_store.rs](../../../crates/orkworksd/src/taskmaster/coordinator_store.rs) | Explicitly dormant data/persistence foundation; no runtime authority. Its older PlanRevision/StoredPlan is not this accepted source/control profile |
| [taskmaster/child_sources.rs](../../../crates/orkworksd/src/taskmaster/child_sources.rs) | Dormant byte validator, no I/O/publication/eligibility; direct worker/reviewer and initial-input validation is not a research producer |
| [hierarchy_storage/mod.rs](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs) and [connection.rs](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Generic mechanics are test-only; production open returns unqualified profile before touching its path. No admitted research store/publisher exists behind it |
| [publication_authority.rs](../../../crates/orkworksd/src/taskmaster/publication_authority.rs) and [lifecycle.rs](../../../crates/orkworksd/src/taskmaster/publication_authority/lifecycle.rs) | Dormant evaluation authority/close-only facade, no production factory. #744 claim-first semantics are not a child-report commit cutoff |
| [runtime/session_runtime.rs](../../../crates/orkworksd/src/runtime/session_runtime.rs), handle_runtime_exit | Closes the actual runtime generation before asynchronous ending work; ordinary ending must retain that independence |
| [session_application/ending_completion.rs](../../../crates/orkworksd/src/session_application/ending_completion.rs) | Captures/revalidates actual workspace/runtime and reports honest separate legacy replay/metadata/event outcomes; no atomic hierarchy bridge |
| [session_application/sql_writes/lifecycle.rs](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) | Private SQL lifecycle preparation, separately gated; no current research bridge proof |
| [Electron main.ts](../../../apps/desktop/electron/main.ts) and [sidecarClient.ts](../../../apps/desktop/electron/sidecarClient.ts) | Main owns trusted desktop integration. Generic sidecar transport and the open-plan header do not qualify research UI approval authority |

Proposed new module boundaries, **not existing files or factories**:

| Proposed owner | Sole responsibility and required existing boundary |
| --- | --- |
| `taskmaster/research_sources.rs` | Closed research source/input decoding and exact source joins; actual captures/native observations come from their qualified owners |
| `taskmaster/research_control.rs` | Bounded semantic reducer for current run/plan/task/launch/input/collection state and complete typed local effects; no model-supplied completeness flag |
| `taskmaster/research_authority.rs` | Separate volatile parent grant and assignment-bound ResearchReportCapability; current authentication/rate before lookup and a separately qualified report cutoff, not evaluation claim-first |
| `taskmaster/research_publication.rs` | Combine local effects with every actual source/consumer/resource/index/proof effect in one qualified store outcome; no sequence of generic journal calls |
| `session_application/research_bridge.rs` | Capture actual ordinary lifecycle/runtime outcomes through SessionApplication and its qualified store owner; publish coherent hierarchy bridge facts, never poll/copy status as proof |
| `http/research_handlers.rs` | Thin future handler/application seam for the unchanged #742 child report operation; no additional caller authority |
| `apps/desktop/electron/researchApproval.ts` | Future Main-owned exact displayed-byte/version context for research approval/continue; renderer/model supplies no trusted native facts or new bearer |

The existing in-memory RuntimeGeneration number is not automatically a 64-hex
hierarchy launch generation. A qualified source owner must bind actual runtime
identity to freshly issued hierarchy generations. No numeric-counter encoding,
timestamp, ordinary label or source hash substitutes for that event.

The accepted [ordinary session storage contract](../../../specs/session-storage.md)
still has activation gates. Neither legacy cross-file ending nor the private SQL
adapter establishes a shared atomic hierarchy transaction. The exact bridge and
report cutoff mechanisms remain blocking qualification work. Ordinary input,
stop and ending must not wait on hierarchy SQL/fsync or retain input-critical
locks through it; unavailable bridge publication conservatively fences hierarchy
automation/capacity instead of delaying the ordinary lifecycle.

## Proposed immutable research input

Use existing canonical UTF-8 JSON, ID/Digest, duplicate/unknown-field rejection,
source JSON integer bounds and storage Counter encodings. All listed fields are
required. Existing initial input remains retained at preparation/input revision 1.
No fake runtime producer is created for that initial root source.

ResearchDecisionV1 is exactly:

```text
{schemaVersion:1,inputRevision,clarificationNeeded:false,researchNeeded:true,
 reasons:Text[],scope:ResearchScopeRef}
```

inputRevision is 2..64 for this research decision; reasons is 1..16 nonempty well-formed UTF-8 strings through
2 KiB each. Scope/questions retain known inputs, expected evidence, stopping
conditions and uncertainty; this decision does not erase them into unknowns=[].
It states an authenticated parent's assessment, not that its conclusion is true.
Material clarification or another decision variant requires a reviewed codec;
it cannot be normalized into clarificationNeeded false.

ResearchPreparedInputV1 is exactly:

```text
{schemaVersion:1,workspaceId,runId,preparationRevision,inputRevision,
 goalDigest,bootstrapConfigurationDigest,decision:ResearchDecisionV1,
 records:ResearchInputRecordV1[],inputDigest}
```

Use unchanged `orkworks.preparation-input.v1` + LF excluding only inputDigest.
Complete body is at most 32 KiB; records at most 256 unique entries, sorted by
UTF-8 kind then canonical full-record bytes. Selected record variants are exactly:

- `{kind:"configuration",reference:ConfigurationRef}`.
- `{kind:"research-scope",reference:ResearchScopeRef}`.
- `{kind:"research-capture",reference:ResearchCaptureRef}`.
- `{kind:"research-report",source:ResearchReportEvidenceSource}` using the exact accepted #744 evidence-source shape.

For this prefix, derive exactly one configuration, its one approved scope, every
scope capture, and zero/one selected report from the collection/expectation
ownership below. No arbitrary caller list, transitive citation fetch, review
input, prior-plan/recovery record, future reservation, new plan digest or live
grant is admitted. Resolve complete original bytes/custody and exact identities.
The initial proposal can compute sources → configuration → input → plan without
a digest cycle; prospective collection rows are published with that proposal.

First research preparation advances both revisions 1→2. Every changed source
set or decision advances both once through 64; identical bytes reuse the original
manifest. Freeze each manifest before referencing it from a plan. Run.currentInput
is the sole current pointer; historical approved plan bindings never move.
Thus preparationRevision equals inputRevision in this fresh prefix. An older
decision at revision k resolves the unique retained input key for this run at
preparation/input revisions k/k and its verified original digest/producer; a
matching decision value without that original row is not sufficient.

A report-induced new manifest copies the original decision **unchanged**. Its
decision.inputRevision may therefore be less than the new manifest.inputRevision.
It must never be greater. Resolve that older immutable manifest by the k/k key
above and compare the exact same decision; do not let a reporting child make a
fresh assessment. The older row must be a fresh assessment at k, with its own
original parent producer and retained runtime/proof origin. That parent need not
be the currently live parent generation.
A new proposal/launch revalidation requires a decision assessed against the
current input. An authenticated parent reassessment creates another immutable
revision with decision.inputRevision equal to that revision; stale assessment
is never promoted by collection, resume or matching scope hashes.

The input row's producer RuntimeRef records the actual event origin: current
parent for assessment, current reporting child for child-report invalidation,
and the separate qualified user-correction origin when that broader active path
is closed. The latter is not silently represented as a child or forged parent
runtime here. No user-correction input producer codec is selected by this prefix.
Exact report retries create no manifest, producer change or revision.

Concretely, input-2 has decision.inputRevision=2 and producer=parent RuntimeRef P.
Input-3 has the identical decision-2 body and producer=reporting child RuntimeRef C.
P is retained and verified through **input-2's row**, not stored in input-3's
single producer field. C proves only input-3's source-event origin, not authorship
of decision-2. A current/fresh decision at revision n requires that same row n
to have the authenticated parent producer; a stale copied decision requires its
distinct original assessment row. Missing original row/runtime/proof denies the
join. Every new research manifest itself has revisions 2..64; input-1 remains
only the unchanged root initial-input codec.

The earlier [source-chain packet](../../validation/research-chain-components.md)
used a labelled unqualified preparation-1 binding. It remains unchanged and is
not adoptable as this prepared input. Any later prefix fixture must construct its
own input-2-bound plan/approval/source bytes; no old digest or history is retokened.

## Proposed current control records and sole version ownership

ResearchRunControlV1 uses the exact ChildRunControlV1 field set with profile
`single-task-retained-file-research-prefix-v1`, and currentInput a nonnull existing
PreparedInputRef. The initial pointer resolves the original root input-1 source;
subsequent pointers resolve ResearchPreparedInputV1. pendingClarification remains
null in this selected prefix; an actual broader clarification record is unavailable,
never discarded. Keep the accepted full immutable run/bootstrap/repository/source
bindings, ordered unique planReferences, currentPlan/pendingPlan, parent RuntimeRef,
bounded blockers, stageContinueRequired, pauseReason and resumeTarget.

Selected state values are assessing, awaiting_research_approval, researching,
blocked, paused and awaiting_resume_approval. The values `synthesizing`,
`awaiting_execution_approval`, `complete` and `cancelled` are retained handoff/terminal
values only: this proposal does not close positive writers into them. Their real
effects remain under the logical #742 and Gate 2 obligations. No executing or
awaiting_clarification body is converted into this prefix. ResumeTarget preserves
the corresponding assessing/research approval/researching/synthesis/execution-
approval target, never paused itself or a grant. Resume precedence remains stage
continue, blockers/unsettled allocation, pending clarification, then source/current
plan classification; unsupported retained evidence blocks this prefix.

Use accepted ControlBlockerV1, pause reasons and all list/text bounds. runVersion,
planVersion, taskVersion and the new collection/expectation versions are source
JSON integers 1..2^31−1; preparation/input/plan revisions 1..64. planReferences
keeps the accepted 16 plan IDs / 64 revisions ceiling and submission order; it
does not raise the 64 input/preparation limit or authorize overlapping plans.

Propose research_run_controls as the **sole** mutable runVersion owner for a fresh
run in this selected profile. Original run_states stays its historical seed.
child_run_controls and the terminal profile's mutable run_states variant cannot
also own this RunKey. Conflicting/missing placement is unavailable, not resolved
by the largest counter or present tables. Future execution/terminal continuation
must preserve this same version history under one reviewed complete catalog;
switching counters/families or initializing again is not a handoff mechanism.
This proposal selects neither that catalog/version nor a migration.

ResearchPlanControlV1, ResearchTaskControlV1, ResearchLaunchControlV1 and
ResearchSessionStateV1 reuse the exhaustive fields, nullability, statuses/phases,
interruption observations, capacities and one-attempt semantics in the accepted
[common control definitions](2026-11-02-child-launch-control-design.md#task-launch-and-ordinary-session-controls),
with refs resolving only the research source registry and new research families.
ResearchPlanControlV1 is the accepted plan control shape with research PlanRefs.
Admission freezes original approval/parent RuntimeRef and the original expected/
assignment pair. No active bearer/grant is retained. Source generation history,
prepared versus spawned distinctions and unknown-outcome capacity holding remain.

ResearchRuntimeAttachmentV1 uses the common parent/child RuntimeAttachmentV1
body variants and RuntimeKey/RuntimeRef, but whole-body digest domain exactly
`orkworks.research-source-runtime-attachment.v1` + LF. Only an actual successful
attachment publishes history, never spawn intent. Keep at most 64 parent and
64 per-child generations, then fence hierarchy attachment without recycling IDs.
The session bridge's producerRevision remains its own positive u64 Counter from
"1", not an ordinary metadata revision renamed. None of these source bodies
authenticate themselves or restore volatile authority after restart.

An authorized identical approval retry returns its original source receipt and
changes no control version or grant. Fresh same-plan reapproval after resume is
a new Main-authorized original approval context, revalidated against current
input/source/runtime; it can grant only a still-unreserved declared task. If the
task is already reserved, restore collection only. It never rewrites the
reservation's original ApprovalRef, frozen version or original launch attempt.
Exact parent resume with every task already reserved restores bounded collection
without plan reapproval, under fresh current parent authority where that operation
requires it. It issues no launch grant and leaves the independent child reporting
authority/lifecycle rules unchanged.

Each semantic run-owned mutation advances runVersion once. Each changed plan
control advances planVersion once. Each relevant task/launch/session transition
advances live taskVersion once, coalesced within one compound event; each actual
ordinary bridge event advances its producerRevision once. Report publication is
the explicit exception: it changes neither task coordination version nor outcome.
Exact retries advance none. Exhaustion denies the whole hierarchy mutation,
without wrap/reset, partial source publication or an unversioned blocker write.
Ordinary end/stop remains independent and hierarchy becomes unavailable/held if
its coherent bridge update cannot publish.

## Positive prospective collection ownership

Propose one prefix collection keyed by RunKey `{workspaceId,runId}`:

```text
ResearchCollectionV1 =
 {schemaVersion:1,workspaceId,runId,collectionVersion,active,task:TaskKey}
ResearchExpectationV1 =
 {schemaVersion:1,task:TaskKey,collection:{workspaceId,runId},collecting,
  configuration:ConfigurationRef,scope:ResearchScopeRef,
  reservation:ReservationRef|null,selectedReport:ResearchReportEvidenceSource|null,
  expectationVersion}
```

Both versions start at 1. A changed reservation binding, selected report or
collecting membership increments expectationVersion once; any owned active
membership/binding/selection change increments collectionVersion once. Coalesce
compound changes and increment the run owner once. Exact retries increment none.

active and collecting are booleans, source-owned memberships rather than model
completeness flags. A proposal atomically creates an active collection and its
one collecting expectation **before any report exists**. Before reservation,
reservation/selectedReport are null. Reservation binds its original ref; the
eight-field stream is then derived by resolving that source, not caller supplied.
First acceptance selects its exact immutable report evidence source. Selection
is not another authoritative head; report_heads remains the sole current-head
owner. Active selection must match that registry after the publication outcome.

The run current-input pointer is authoritative. Collection stores no second
pointer. For this prefix its selected task has the current prepared decision's
scope/configuration. A pre-reservation proposal replacement must atomically retire
the old unreserved expectation (collecting false), retain all its history, create
the new expectation and update collection.task/current input/pending proposal.
Reserved/recovery/synthesis replacements need their separate codec/effect closure;
the reducer cannot retarget an existing reservation or erase old relationships.

Required logical membership indexes are:

- all expectations by collection RunKey, including retained retired rows;
- collecting expectations by collection RunKey, exactly the active collection's singleton task in this prefix, empty only when that collection is explicitly inactive;
- expectations by resolved exact Stream, reconstructed through original reservation/configuration, plus direct current-input and historical immutable-input source consumption.

They are derived/maintained by the same source-owner outcome as their owning
rows. Prove complete bounded owner-family membership, no orphan/duplicate task
or stream alias, and stable owner/index generations at final validation. A missing
index, overflow, or caller's claim that the set is complete denies publication.
Physical indexes, full workspace inventory roots and their proofs must be closed
in the one complete catalog before transaction fixtures; this logical index list
is not already that proof.

Plan pause/completion and ordinary child end do not implicitly retire collection
membership. An active run can still receive reports under the accepted child
rules; each genuinely affected active preparation must be invalidated. Future
qualified finish/cancel must atomically close the proper memberships while
retaining selected historical reports; terminal correction creates no preparation
transition. No such writer or terminal transaction is supplied here.

This positive local relation does not prove global consumer completeness. Other
active preparations, tracked-run detail, custody and historical evidence consumers
still need exact owning membership/index closure under the
[first-report investigation](../../validation/first-report-producer-contract.md#why-first-report-absence-does-not-settle-consumers).
No head, no immutable incoming edge, or this singleton local collection cannot
prove those other sets empty. All discovery/revalidation counts toward the
accepted 100,000-visit bound; never truncate or split publication.

## Proposed normal version trace and local effect contract

The following is an illustrative successful prefix with no intervening events,
not a hard-coded report CAS value. Existing source fixtures retain only the
reservation's original pair; their absent live controls cannot supply this trace.

| Event | runVersion after | live taskVersion after | Original frozen assignment version |
| --- | ---: | ---: | ---: |
| Fresh initial run control | 1 | absent | absent |
| Actual first parent attachment | 2 | absent | absent |
| Input-2 / research proposal / task creation | 3 | 1 | absent |
| Exact plan approval | 4 | 1 | absent |
| Original reservation | 5 | 2 | 2 |
| Allocation intent | 6 | 3 | 2 |
| Original allocation outcome | 7 | 4 | 2 |
| Coherent attachment/fresh preparation/ordinary prepared bridge | 8 | 5 | 2 |
| Original spawn intent | 9 | 6 | 2 |
| Actual successful spawn/runtime bridge | 10 | 7 | 2 |
| First report plus input-3/selection | 11 | 7 | 2 |

Report expectedTaskVersion must read the actual live task control. It is 7 only
for exactly this trace; never substitute 1, the frozen 2, or a guessed generation.
Keep expectedRunVersion/expectedReportHead null for first child publication and
unchanged report/request/receipt codecs. Preserve the old R2 expected-version-1
component requests and original source-chain bytes without rewriting them.

The local source-owner read/write unions below are mandatory logical effects,
not a complete generic row manifest or executable command API:

| Logical event / owner | Read and fence | Local writes / preserved facts |
| --- | --- | --- |
| Fresh research preparation / publication owner | Original root/run/bootstrap/initial input and fresh resource inventory | Sole run control version 1, no runtime/plan/grant; no missing-ledger repair |
| Parent attach / bridge + control | Exact ordinary/native parent identity, original bootstrap, current run/parent row | Attached parent wrapper, immutable research runtime history and current pointer/run version together; no replayed authority |
| Prepare proposal / source + control | Current parent/planning authority, run/input versions, complete proposed source/custody set | New research input, immutable plan, plan/task controls, collection/expectation and pending pointer; no allocation or grant |
| Approve / UI + control | Exact shown source bytes/review context, current input/assessment, run/plan/source/lease and immutable capacity limits | Original approval/receipt, active plan/pointer versions and eligible volatile exact-plan grant; no child or slot consumption yet |
| Reserve / control | Exact original plan/config/scope/approval, current parent grant/runtime, pending task and complete run/plan capacity | One reservation, frozen admission/launch control, task launching/version, expectation binding/index and run version atomically before native allocation |
| Allocate / native owner + control | Original intent, source/native ownership, current controls | Persist allocating intent before mutation, then original allocation or interrupted observations; retain attempt/capacity, never adopt/search a plausible directory |
| Attach/prepare / application + publication | Original allocation, unique child/session/source, complete fresh assignment preparation | Attachment/assignment source, ordinary prepared fact, session bridge and all required subject/zero-ledger/summary effects coherently; no spawn until successful outcome is proven |
| Spawn / runtime + bridge + control | Original prepared child, exact approved startup/settings/role, qualified generation/capability mapping | Intent first, then actual runtime history/current bridge/running phase and coalesced live task/run versions, or interrupted original outcome. Intent is no successful attachment |
| First report / authority + publication + control | Auth/generation/epoch/lease/health/rate before receipt lookup; original source/custody, actual live CAS, absent accepted history, complete current collection/consumer/resource read set | Report object/head/original receipt; selected report, collection/expectation versions, immutable input-3/current pointer and run version once. Copy assessment-2 as stale. No task outcome/version, capacity release, plan completion or grant |
| Ordinary end/resume / actual lifecycle + bridge | Actual current runtime/source revision, original unique attachment/attempt | Coherent bridge/current-runtime history, live task/run increments for that event and derived capacity; immutable reservation identity, outcome and terminal run/plan state stay unchanged |

Approval does not claim or release a live-child slot. Reservation derives both
run and plan usage across every retained attempt/ordinary child, counts each
unsettled reservation or its unique attached prepared/nonterminal child once,
and checks usage+1 against both immutable limits. Parent outcomes do not free
capacity; unknown bridge disposition is conservatively held. A normal ordinary
resume can consume capacity again without authorizing another orchestrated
attempt. Preserve it and fence new launches if limits are exceeded.

For the new first-report request, the exact **local** durable reads are:

- research_run_controls[RunKey], original research_plan_controls[PlanRef], research_task_controls[TaskKey], research_launch_controls[TaskKey], and research_session_states[TaskKey];
- original research_plan_approvals, research_reservations, research_allocations and research_attachments under their accepted source keys, plus full source configuration/scope/captures/assignment and required original/current runtime attachments;
- research_collections[RunKey], the complete owning expectation families and their collection/Stream indexes, current research_control_inputs row and original decision-producer input history;
- report_heads[Stream], report_receipts at the accepted first-version key and all uniqueness/history/proof lookups needed to prove no accepted initial version, plus every actual run/workspace resource/lease/health origin.

The local changed rows are exactly a new immutable report, compact head and
original receipt; the new research input/producer row; expectation selection/
version; collection version; and sole current-run input pointer/version. The
plan/task/launch/session/runtime and original source rows are reads, not report
writes. Resource rows, source/custody/dependency/summary/consumer protections,
physical indexes and header/new proof remain additional **actual** effects from
their complete owners. They cannot be replaced by a symbolic producer boolean
or inferred absent here. No whole-store command/row total is qualified.

Child reporting does not require a live parent or launch grant if its own
generation-bound authority is still valid in an active paused run. Collection
does not clear stageContinueRequired, blockers, pause reasons or coordination
outcomes. Final run completion/cancellation and child runtime end still revoke
before receipt lookup. The read set and all source/index fences are revalidated
at the separately qualified commit boundary; drift denies rather than silently
retargeting the normalized request or repairing historical state.

Every command also includes all actual source/control/preparation/summary/custody/
consumer/resource/index/header/new-proof effects from their owners. This table
does not qualify a complete byte/row total. The first-report union cannot commit
until the global catalog/consumer/cutoff/bridge closures are complete. An identical
accepted report retry, after current authentication and rate checks, returns its
original receipt before live CAS without any write, revision, event or charge.

In the example, input-3 contains report-1 and exact copied decision-2. Parent
reassessment would need a further input revision before any new proposal whose
decision must be current; report arrival itself cannot make the parent's decision.
Revision-64 exhaustion denies a new report requiring input-65 atomically; an
otherwise authorized exact accepted retry remains read-only. Final child
revocation still precedes lookup; terminal UI authority is not an initial-child
publication exception. Nonblocking revocation/commit qualification must not reuse
#744 claim-first or merely suppress a response after an unqualified commit.

## Proposed logical families, accounting and recovery boundary

These names are proposed inputs to a future **single complete fresh catalog**,
not a partial schema, format-4 amendment or optional registry selected by rows:

| Family / exact key | Exact wrapper value | Complete wrapper bound |
| --- | --- | ---: |
| research_control_inputs / PreparedInputRef without inputDigest | `{schemaVersion:1,body:ResearchPreparedInputV1,producer:RuntimeRef,lastCommit}` | 64 KiB |
| research_runtime_attachments / RuntimeKey | `{schemaVersion:1,body:ResearchRuntimeAttachmentV1,digest,lastCommit}` | 4 KiB |
| research_run_controls / RunKey | `{schemaVersion:1,body:ResearchRunControlV1,lastCommit}` | 1 MiB |
| research_plan_controls / PlanRef | `{schemaVersion:1,body:ResearchPlanControlV1,lastCommit}` | 16 KiB |
| research_task_controls / TaskKey | `{schemaVersion:1,body:ResearchTaskControlV1,lastCommit}` | 16 KiB |
| research_launch_controls / TaskKey | `{schemaVersion:1,body:ResearchLaunchControlV1,lastCommit}` | 32 KiB |
| research_session_states / TaskKey | `{schemaVersion:1,body:ResearchSessionStateV1,lastCommit}` | 16 KiB |
| research_collections / RunKey | `{schemaVersion:1,body:ResearchCollectionV1,lastCommit}` | 16 KiB |
| research_expectations / TaskKey | `{schemaVersion:1,body:ResearchExpectationV1,lastCommit}` | 16 KiB |

Keys reconstruct from bodies/original sources. Initial input-1 remains its existing
root-owned source, not a forged research input with a null producer. All lower
source/2 MiB row and global limits apply; full physical/proof/index shapes remain
unclosed. No additional ObjectRef kind, edge-purpose string, report wire field,
receipt version or active authority serialization is introduced.

Propose sharing the existing child-control-retained-bytes pools, 8 MiB/run and
64 MiB/workspace, reserved zero, with every other admitted control occurrence.
Charge exact canonical keys plus committed value projections, excluding only
named outer lastCommit for these non-receipt-bearing prefix families. Internal
sources, producer refs, selections and versions remain committed. Preserve
separate source/report/assignment resources and the existing 32 MiB run aggregate;
no new allowance, #744 credit, deduplication or ordinary-end refund. Mutable
replacement uses original-to-final deltas; immutable inputs/history stay retained.
Fresh quota creation must be part of the complete initializer, never backfilled.
The common parent_attachments replacement stays in its existing preparation
resource; the new immutable research runtime history is a separate control-pool
occurrence. No parent/source/assignment row is made free or double-assigned to a
different pool by the new control-family names.

Recovery must preserve complete old/new outcome or unavailable reconciliation,
including original inputs, attempts, native observations, generations, sources,
report history and resource/proof ownership. Unknown allocation/spawn disposition
holds its original attempt/capacity. This prefix supplies no recovery decision/
coordination-turn receipt codec, no automatic relaunch/cleanup and no second
attempt. Those research-specific histories, synthesis/execution continuation,
real finish/cancel, full consumer membership and the ordinary-storage bridge
remain blockers for complete positive history fixtures and all activation.

## Owner choices and bounded next handoff

Accept or revise these **new** proposed choices for offline prefix-component
fixtures only:

1. Research-specific prepared input/decision/record variants under the unchanged
   input domain; first manifest 2, paired revisions and explicitly stale copied
   assessments after report-induced input changes.
2. One research current-run owner, research control/runtime families and the
   accepted coalesced live-version rules; frozen reservation identity remains
   independent of actual current report CAS. No old profile/store adoption.
3. Positive collection/expectation ownership before a head exists, one local
   collecting task in this prefix, exact source-derived indexes/selections and
   no inference of global consumer absence.
4. Shared existing control pools and the named source-owner/module/effect
   boundaries, with no format/cutoff/ordinary-bridge qualification inferred.

After scoped acceptance, only independent input/control-prefix component bytes
and denial checks may follow. They must retain all origin/authority/index/store/
bridge/cutoff limits above and preserve every earlier artifact byte. No complete
first-report transaction/history, native probe, migration, adapter, application
runtime or executable implementation plan is authorized by this handoff.
The broader first-report and terminal Gate 2 obligations stay open.

The bounded component deliverables are confined to `docs/validation/`: a Python
generator, retained JSON, data-only JavaScript checker/tests and evidence prose,
plus documentation indexes/check routing and LF attributes. They may model
typed local effects as synthetic data, never as an executable publisher. The
proposed Rust/Electron module table is documentary ownership only: no files,
codecs, production/dormant reducer APIs, factories, routes, hooks, authority
handles or wiring under `crates/` or `apps/` may be created by this handoff.
Product implementation would require a separate approved design/plan and all
applicable capability/activation gates; this document supplies no such plan.

## Written review disposition

Two fresh read-only Luna checks reviewed correctness and the bounded builder
handoff. The correctness pass confirmed the original/frozen-versus-live version
trace and identified ambiguous producer wording. The author clarified that the
older parent assessment and the newer child invalidation retain separate input
rows/origins, with an exact k/k historical lookup rather than two producers in
one field. The builder asked whether future module code was in scope; the
handoff now explicitly permits only offline documentation fixtures and forbids
even dormant Rust/Electron module or reducer implementation. These clarifications
add no runtime authority and do not accept the new choices on the owner's behalf.
No reviewer executed a fixture, Rust, native process or application runtime.
