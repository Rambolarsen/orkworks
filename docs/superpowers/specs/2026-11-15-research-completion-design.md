---
type: Design Proposal
title: Research completion, synthesis and UI closure records
description: Proposed source-owned research plan completion, immutable summaries and version-bound finish/cancel/continue decisions with retained gates, authority fencing and catalog obligations.
tags: [orkworks, research, completion, synthesis, cancellation, catalog]
status: draft
workflow_status: proposed-for-scoped-completion-review
---

# Research completion, synthesis and UI closure records

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `761540e709827ff555935e1916caf29c564615d5`.

**New representation and publication proposal for scoped owner review.** The
owner authorized this next written contract. That does not accept the new source
codecs, selector/history placement, summary-version bound or initializer extension
below. Earlier [turn/result/recovery acceptance](2026-11-15-research-continuation-design.md#scoped-owner-acceptance-—-2026-11-15)
remains valid for its offline packet. Merge publishes a proposal, not acceptance
or producer qualification. Gate 2 remains Investigate.

## Outcome and preserved lifecycle

Make the selected research path representable from its final coordination result
through automatic plan completion, bounded synthesis and explicit UI closure.
Reuse the [accepted logical lifecycle](2026-10-04-orchestrator-preparation-design.md#logical-lifecycle-acceptance-—-2026-11-08)
and [baseline completion predicate](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan):

- A research plan completes automatically when every declared task is
  reported_complete, all reservations are attached and settled, and no interrupted
  allocation or pending proposed scope revision remains. Recheck atomically when
  any transition resolves the last blocker. There is no new approval or pause
  between successful research and same-parent synthesis.
- Completion revokes that exact plan's launch grant, not the live parent's
  planning bearer. Every nonterminal child still consumes run capacity and remains
  an ordinary session. No child is ended or integrated to make room.
- Research-only/no-go/declined-execution finish is an exact UI choice before any
  execution approval, with settled reservations and no interrupted allocation.
  It ends run authority; synthesis alone cannot finish or authorize execution.
- Stage cancellation retains read/wait/existing-result parent rights but fences
  planning mutations until separate version-bound UI continue. Runtime resume
  cannot clear that gate. Run cancellation is terminal and does not kill children.

The current source path stays the one-task retained-file research profile.
Previously accepted run/input/plan/task/launch/session/runtime/report/receipt and
ordinary-lineage field sets/domains stay unchanged. research_run_controls remains
its sole current runVersion owner; run_states remains historical. The new selector
has no runVersion and never owns currentInput. No existing proof is retokened.

No dependent fixture, Rust/Electron module, runtime API, native/model call,
physical database/profile/header/manifest/row hash, migration, adapter or
executable product plan is supplied here. Actual source participation, authority
cutoffs, physical atomicity and complete storage/consumer qualification remain
separate gates. Required review/clarification/execution-plan source variants not
closed by this selected path are unavailable rather than erased to fit it.

## Source trace and documentary owners

| Source | Constraint / observed limit |
| --- | --- |
| [Baseline automatic completion](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan) | Final successful result and later blocker resolution run the completion check atomically; live PTYs do not delay plan completion |
| [Research-to-execution lifecycle](2026-10-04-orchestrator-preparation-design.md#same-parent-research-to-execution-transition) | Summary binds report/review versions, stays within 8 KiB/128 refs, and future execution needs its own new plan and exact approval |
| [Pause/resume/terminal precedence](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table) | Stage gate, blockers and clarification survive interruptions; complete/cancelled absorb; finish is not goal acceptance |
| [Research control owner](2026-11-12-research-input-control-prefix-design.md#proposed-current-control-records-and-sole-version-ownership) | One run owner and independent original frozen P/live task version; collection can remain active after plan completion |
| [Continuation sources](2026-11-15-research-continuation-design.md) | Exact turn/result/recovery bodies, original request/receipt and source-owned historical proofs; finite packet is not a legal history |
| [Membership and publication](2026-11-12-research-membership-publication-design.md#complete-discovery-and-the-positive-first-report-slice) | Current and historical consumers and both lookup directions must be complete; unsupported prospective owners deny the narrow slice |
| [Terminal producer gap](../../validation/terminal-producer-history-consumer-contract.md#terminal-producer-contract-to-close) | TerminalReportRunV1 has different labels/shape and excludes gates/clarification/blockers and broad consumers; no cast or real producer is established |
| [Fresh resource recipe](2026-11-13-research-initialization-catalog-design.md#exact-fresh-owner-resource-recipe-proposed) | Shared control/source/report/preparation pools already exist at admission; no missing-ledger backfill or new allowance |
| [Dormant coordinator](../../../crates/orkworksd/src/taskmaster/coordinator.rs) and [production connection](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Data-only foundation and unqualified-profile refusal remain; no completion/summary/closure publisher is inferred |

The source facts above are bounded inspection, not installed behavior or an
exhaustive writer inventory. Use the proposed research source/control/authority/
publication, SessionApplication bridge and Main approval seams in the
[prefix owner table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
No new module or factory is created. Actual ordinary/native owners provide
liveness, disposition and capacity facts; neither renderer nor model can supply
a trusted completion set or Main-authorized closure decision.

## Shared types and the new lifecycle selector

Use existing canonical UTF-8 JSON, duplicate/unknown-field rejection, IDs/Digests,
RunKey, PlanRef, TaskKey, ReservationRef, RuntimeRef, PreparedInputRef, CommitRef,
and research report evidence-source types. Full keys derive from original source
ownership. Source versions remain bounded JSON integers; storage counters remain
their exact Counter TEXT encodings. Every nullable field listed is present.

ResearchSummaryKey is exactly `{workspaceId,runId,summaryVersion}`;
ResearchSummaryRef adds exactly summaryDigest. summaryVersion is a distinct
source-output version, 1..64, not inputRevision, taskVersion or runVersion.
The proposed fixed current selector ResearchLifecycleStateV1 is exactly:

```text
{schemaVersion:1,run:RunKey,summaryVersion,
 completedResearchPlan:PlanRef|null,currentSummary:ResearchSummaryRef|null,
 pendingStageContinue:ResearchLifecycleRef|null,
 finishDisposition:null|"research-only"|"no-go"|"declined-execution"}
```

Here summaryVersion is a high-water value 0..64, starting at 0; it does not reset
when currentSummary is invalidated, on resume/cancel or under another namespace.
New summary creation consumes the next value exactly once. Do not infer it from
stored maxima or reuse deleted/missing IDs. This is the sole summary-version and
current-summary owner, not a second current-run/input owner. A nonnull summary
matches this run and the high-water value; null does not mean no historical
summaries. completedResearchPlan is the exact latest successfully completed
research plan selected by its owning completion outcome, not a maximum scan.

pendingStageContinue pins the original stage-cancel decision while the run's
stageContinueRequired is true; false requires null. Terminal runs may retain a
true gate and its historical pointer. Terminal state makes it inoperative, never
permission to continue. finishDisposition is nonnull only for the corresponding
UI research finish, and remains null for run cancellation. No selector field
restores a bearer/grant or changes a frozen source.

A missing selector, inconsistent counter/pointer, conflicting source owner or
missing historical member is unavailable. Never initialize this row on first
summary/finish lookup or add optional fields to an old store to make it fit.

## Immutable bounded synthesis proposed

ResearchSummaryV1 is exactly, with no self-digest field:

```text
{schemaVersion:1,run:RunKey,summaryVersion,researchPlan:PlanRef,
 input:PreparedInputRef,producer:RuntimeRef,
 findings:Text[],uncertainties:Text[],
 assignmentChoices:{description:Text,reason:Text}[],
 reportReferences:ResearchReportEvidenceSource[],reviewReferences:[]}
```

The whole canonical body is at most 8 KiB. findings, uncertainties and
assignmentChoices each independently have at most 16 entries; description and
reason are separate required strings on every assignment choice. All prose fields are nonempty UTF-8 through 2 KiB. reportReferences is unique and
sorted by the canonical full accepted evidence-source bytes, with at most 128
entries. The lower complete-body limit wins. This selected one-task path admits
reviewReferences empty **only when complete original plan/source membership proves
no declared required review**. Required or existing review records need their
separately closed typed source codec; they are never discarded into this empty
variant. Broader report/review support retains the accepted 128 combined ceiling.
Descriptions/reasons explain decisions, not hidden reasoning or executable tasks.

Hash `orkworks.research-summary.v1`, one LF and the complete canonical body.
Retain exact original report versions/digests/size/source-group joins, the
completed plan, current input and its original assessment provenance, and actual
parent RuntimeRef source. The report set is derived from the completed plan's
required streams and source-owned current collection/head selection. A caller's
reference list or a digest-only match cannot prove required coverage/currentness.
Missing research, conflicting required review or material uncertainty prevents a
ready execution proposal; uncertainty can still be stated in this research summary
or support a no-go finish. A summary never grades the task or changes its outcome.

Only the current parent planning authority may publish a summary, with the
existing shared ten-parent-mutation requests/minute discipline before lookup
(duplicates count as requests). Stage-continue restrictions still deny this new
planning mutation. Require the selected completed research plan, exact current
run/input/version, complete source/head/collection membership and eligible bound
parent/runtime. Publishing while a retained blocker remains cannot clear it or
turn blocked into ready. Default synthesis with a live parent and no gates is
automatic; no additional UI approval is introduced for preparation.

The offline publication input ResearchSummaryPublicationV1 is exactly
`{schemaVersion:1,expectedRunVersion,summary:ResearchSummaryV1}`. The caller
supplies the proposed summary version from the observed selector; the publisher
never substitutes the current next version before retry lookup. Derive the key
from summary.run and summary.summaryVersion. After current authority/rate checks,
look up that exact key: identical complete summary bytes return its original ref
and original lastCommit proof without writes; any changed summary byte conflicts.
expectedRunVersion is an admission precondition, not retained summary identity,
and may differ on a read-only retry. Never elect a summary by content or maximum.
If the key is absent, require expectedRunVersion to match the current run and
summary.summaryVersion to equal selector.summaryVersion+1, then perform every
new-publication source/currentness check. Gaps, stale versions and competing
updates deny atomically. Retry of an accepted old key returns its historical
source only; it never reselects currentSummary or asserts its current validity.

New acceptance inserts the immutable summary and advances the selector's
summaryVersion/currentSummary and sole runVersion once in one complete outcome.
It does not change currentInput, a task, live-child capacity or grant. Its
producer/runtime is historical source, not a reusable credential. A changed summary is a separately admitted next summary version, never
an overwrite. Exhaustion/quota/source drift denies a new source atomically;
retain prior summaries and use the existing control-exhausted/unavailable path
without counter reset, pruning or fabricating input changes.

Summary publication is derived output, not a new current-input record. This
avoids a digest cycle or silently broadening the accepted input-record union.
An execution proposal must eventually bind the selected exact SummaryRef as well
as the current input and completed research plan through its own reviewed source
codec. This document supplies no execution-plan variant or positive approval path.

## Original completion and UI request codecs proposed

ResearchLifecycleKey is exactly one of:

```text
{kind:"plan-completion",plan:PlanRef}
{kind:"ui-decision",run:RunKey,decisionId:ID}
```

ResearchLifecycleRef is `{key:ResearchLifecycleKey,digest:Digest}`. Automatic
completion has one original record per exact PlanRef. UI decision IDs are Main/
sidecar-issued nonsecret identities for an exact shown context, unique across
this run; they are never bearer credentials, reused after conflict, or supplied
as trusted identities by a model/renderer.

ResearchLifecycleRequestV1 is a closed tagged union. Its common fields are
`{schemaVersion:1,kind,run:RunKey,expectedRunVersion,input:PreparedInputRef}`.
Add **exactly** the fields for the selected kind below:

| kind | Additional fields |
| --- | --- |
| plan-completion | `plan:PlanRef,expectedPlanVersion,trigger:ResearchCompletionTriggerV1` |
| finish-research | `decisionId,researchPlan:PlanRef,summary:ResearchSummaryRef,disposition,reviewContextId,sidecarGeneration` |
| cancel-stage | `decisionId,plan:PlanRef,expectedPlanVersion,reason,reviewContextId,sidecarGeneration` |
| continue-stage | `decisionId,cancellation:ResearchLifecycleRef,reviewContextId,sidecarGeneration` |
| cancel-run | `decisionId,reason,reviewContextId,sidecarGeneration` |

finish disposition is exactly research-only, no-go or declined-execution; reason
is nonempty UTF-8 through 2 KiB. reviewContextId and sidecarGeneration are Digests
binding the exact Main-reviewed action/current source generation. They record
provenance and cannot authenticate themselves. Versions retain the existing
1..2^31-1 bounds. Keys reconstruct from the request: plan-completion uses plan;
all UI kinds use run/decisionId. request.input is the exact captured current input,
not necessarily the older input frozen into a completed plan's approved definition.

ResearchCompletionTriggerV1 is exactly one of:

```text
{kind:"coordination",task:TaskKey,acceptedTaskVersion,requestDigest:Digest}
{kind:"recovery",task:TaskKey,recoveryId:ID,decisionDigest:Digest}
```

The coordination form pins the original ResearchCoordinationRequestV1 and its
receipt's resulting acceptedTaskVersion, not a pre-trigger version. The recovery
form pins the exact accepted ResearchRecoveryDecisionV1 body/digest under its
original `{task,recoveryId}` key. Both resolve the same plan/run and complete
source outcome, including its actual resulting task version. Recompute the
completion predicate over the trigger's candidate poststate; the captured
lifecycle readSet and expected versions remain the original pre-trigger state.
Commit trigger, completion and their coalesced effects in the same C, so neither
form hashes that new C. Trigger retry resolves the original source outcome and
its completion record, not a fresh request against the poststate.

The accepted lifecycle also requires checks after other blocker resolutions,
including pending scope-revision rejection. Those source codecs are not supplied
by this selected union: a transition that would complete the plan through an
unsupported trigger is unavailable until its typed trigger/source effects are
closed. It cannot commit only the resolution and omit completion, insert a fake
coordination/recovery digest, or introduce a manual completion delay. The bounded
positive packet covers coordination completion and a supported recovery that
actually clears the final condition; it claims no broader writer qualification.

The source-owned body ResearchLifecycleDecisionV1 is exactly:

```text
{schemaVersion:1,key:ResearchLifecycleKey,
 request:ResearchLifecycleRequestV1,readSet:ResearchLifecycleReadSetV1}
```

ResearchLifecycleReadSetV1 is exactly:

```text
{expectedCommit:CommitRef,run:ResearchRunControlRead,
 lifecycle:ResearchLifecycleStateRead,
 currentPlan:ResearchPlanControlRead|null,
 pendingPlan:ResearchPlanControlRead|null}
```

Reads have exactly `{table,key,value}`, full decoded original snapshots. run and
plan use their unchanged research shapes; lifecycle reads research_lifecycle_state
under RunKey with `{schemaVersion:1,body:ResearchLifecycleStateV1,lastCommit}`.
run snapshot ceiling is 1 MiB; each plan snapshot 16 KiB; lifecycle snapshot 16 KiB.
Keys reconstruct the request run and original plan pointers. Expected run/plan
versions match captured bodies exactly; input equals captured run.currentInput.
A null plan read matches a null pointer but proves no global absence. Original
root/bootstrap/parent/source/summary/task/turn/coordination/allocation/native/
quota/consumer/index reads still form part of the actual full transaction read
set; these fixed snapshots are not a checksum replacing it.

Hash `orkworks.research-lifecycle-decision.v1`, one LF and the complete canonical
body. The original snapshot origins/H remain hashed. The new C is not inside
this body. ResearchLifecycleReceiptV1 is exactly:

```text
{schemaVersion:1,key:ResearchLifecycleKey,decisionDigest:Digest,
 runVersion,planVersion:null|JSON-version,commit:CommitRef}
```

Receipt is at most 4 KiB; runVersion is the actual coalesced poststate version.
planVersion is the actual resulting version of the single target current plan
for plan-completion/cancel-stage, otherwise null (it is not a report of every
invalidated pending plan). Receipt, decision and full effects commit together.

For UI operations, authenticate current Main authority, exact workspace/epoch/
owner/lease/health and shown action context before original decision lookup.
Compare an existing row's **original request**, not newly reconstructed current
snapshots. Identical accepted request bytes return the original receipt before
live-version CAS without writing, even after the run becomes terminal; Main
lookup authority is distinct from the revoked parent bearer. Changed bytes at
the same UI key conflict. An unavailable current authority/lease/store cannot be
repaired by a stored receipt. No active UI token is persisted or sent to renderer/
model. New decisions then capture/revalidate current snapshots and exact request
versions before the complete owning transition. No admission-first permit,
late-saving exception or restored planning credential is introduced here.

Automatic completion is part of its authenticated source trigger, not a new
parent request/rate allowance. An exact original coordination/recovery retry returns its
original outcome; it does not reconstruct a new completion request against live
versions. Rechecking an already completed plan verifies its original record and
is read-only. It never appends a second completion under a changed current version.

## Source-owned transition and effect union

| Transition | Required admission / complete local effects |
| --- | --- |
| Automatic research plan completion | Check every declared task's reported_complete and exact original turn/result/source group, all attached/settled reservations, no interrupted allocation or pending scope revision. In the triggering compound outcome insert one completion record, mark that plan complete/version+1, revoke its grant, clear run.currentPlan, select completedResearchPlan, clear any obsolete currentSummary, and advance runVersion only once with the trigger. Preserve planReferences, current input, immutable outcomes and live children/capacity |
| Publish synthesis | Bind exact current completedResearchPlan/input/head/collection/source set and actual current parent; insert summary, advance summaryVersion, select its ref and coalesce runVersion. Do not change currentInput, planning state gates, task outcomes or grant |
| UI finish-research | Require original completed research plan, current valid summary/input, no execution approval/allocation/reservation under complete plan inventory, settled reservations and no interrupted allocation. Set run complete; record disposition; revoke run/child-report authority and every grant, wake waits run_closed, invalidate every recognized pending proposal, clear current/pending plan and parent runtime pointers, retain summary/history/children and every blocker/gate/clarification value |
| UI cancel-stage | Target exact current nonterminal research plan/version. Mark it cancelled/version+1, revoke its grant, invalidate recognized pending proposals, clear current/pending plan, set paused/stage_cancelled with the prior logical coordination resume target, set stageContinueRequired and pendingStageContinue to this exact new record. Keep the live parent read/wait/existing-result bearer rights and current child report rights, blockers, native attempts/capacity and summaries/history |
| UI continue-stage | Require the exact pending cancellation ref, stage gate, current versions/input and actual eligible live parent (fresh exact runtime resume first if required). Clear only stageContinueRequired and its selector pointer. Choose blocked for retained blockers/unsettled allocation, otherwise clarification when a supported unanswered source exists, otherwise assessing. Preserve those sources; clear pause fields only when actually leaving paused. No cancelled grant, reservation or task is reused |
| UI cancel-run | Require current version-bound Main authority, not settled-native/ended-child preconditions. Set cancelled and revoke all run/child-report/grant authority; wake waits closed, invalidate recognized pending proposals, clear current/pending plan and parent runtime pointers. Preserve raw blockers/gates/clarification values, completed plan/summary, attempts and unknown native disposition/capacity. Record no-child/rollback only through its separately proven owner; never kill, delete or adopt children/worktrees |

A final successful one-task parent outcome normally triggers coordination and
plan completion in the **same** C, with one runVersion/taskVersion advance and
one planVersion advance; it is not a second journal call or manually approved
completion step. A later source transition resolving the last pending condition
runs the same check under its supported typed trigger; unsupported triggers deny
the whole transition until closed. The older continuation packet's result-only task views are
partial examples, not a completed-plan publication history to adopt or rewrite.

After automatic completion, select synthesizing only when current runtime/
planning gates permit. If actual parent interruption already requires pause,
keep the pause and synthesize resumeTarget, retaining higher-priority stage gates
and blockers; do not revive its bearer. A retained blocker keeps the run blocked,
a supported unanswered question keeps its clarification gate, and future exact
resume follows the accepted precedence. Terminal run states never become paused
or synthesized because a child later ends/resumes or a report arrives.

Plan completion and stage cancellation **do not retire** the run's active
collection/expectation. Valid live children may continue reporting while the run
is active/paused. Each genuinely changed report source atomically updates its
accepted head/history, current input/collection and every affected summary/
proposal consumer. Clear currentSummary when its exact report/input binding
becomes stale, retain immutable summary bytes/version/history, invalidate all
recognized dependent pending proposals and fence stale launches. Changes after
actual execution started need the accepted visible blocker/fencing behavior;
this selected contract supplies no positive execution writer. Missing/unknown
consumer effects make publication unavailable, not a reason to omit them.

Finish/run cancellation explicitly retire every proper active collection and
collecting expectation in the owned complete set, preserving selected reports,
versions and historical relations. Coalesce changed collection/expectation/run
versions once, without making terminal report corrections create new preparations.
No final task coordination outcome, evaluation, source ledger or lifetime debit
is rewritten/refunded. Ordinary ending/resume remains independently observable
and can change actual capacity without reopening a terminal plan/run.

Unknown or unsupported pending plan/clarification/review/execution codecs block a
successful publication rather than being dropped to fit these selected records.
The positive finish example is completed selected research with no execution
proposal/approval. A broader decline action with a pending execution proposal
still requires its closed source and invalidation owner. This does not remove the
accepted broader finish behavior or pretend it is qualified here.

## Fencing, rollback and terminal compatibility

The authority owner serializes launch/report publication and every completion/
closure fencing decision with the complete state mutation under its qualified
nonblocking boundary. Durable acceptance is the full physical commit; callbacks,
transport aborts or suppressed responses prove neither commit nor rollback.
Ordinary input/stop/end must not wait on hierarchy SQL/fsync or retain an input-
critical lock through it. Actual mechanism and all direct writers remain unqualified.

Run cancellation must not wait for native allocation/spawn disposition or PTYs
to end. It can retain an unknown attempt while fencing future automation. If
original store I/O/lease/commit disposition is unknown, retain the captured sole
owner and immutable plan until proven, close applicable volatile authority under
its existing close-only rules, and expose unavailable/reconciling rather than a
successful terminal receipt. No replacement worker, timeout rollback assertion,
partial terminal header or standalone safety row is authorized by this proposal.

An earlier original-preparation permit can settle only under its accepted exact
read-set/ownership rules. A committed closure advancing H/G or changing captured
facts defeats that old preparation; a permit cannot reopen authority. If the
original preparation commits first, closure must revalidate/include that child
and its actual disposition. No race turns a permitted late save into a late spawn.

For each stored snapshot retain original direct proof membership at/before H in
the same storage epoch, complete before-image/source correspondence and the new
outcome proof. New manifest predecessor equals H. Every excluded C link resolves
its own original proof; no predecessor replay, maxima election, source/table
fallback or receipt reconstruction substitutes for actual currentness/coverage.
Proven rollback keeps the complete old state; proven commit keeps the complete
new state; unknown keeps original ownership and fences dependent work.

There is **no TerminalReportRunV1 producer or conversion** here. Keep research
state complete/cancelled in research_run_controls, preserving any true raw stage
gate, blocker or clarification source. Do not rewrite it into the old completed/
cancelled run_states variant, erase fields to satisfy that narrow shape or add a
second runVersion. A read-only display label complete→completed is not persisted
source/profile qualification. Retained report-consuming summaries/proposals also
fall outside the older terminal evaluation-only correction profile. Its separate
Main correction exception grants no initial child write or finish/cancel authority.
Until a complete source/consumer/profile mapping is qualified, those broader
terminal UI corrections remain unavailable for this research path.

## Logical catalog, initialization, projections and resources

Propose three mandatory families in the eventual single complete fresh catalog:

| Family / exact key | Exact value | Complete wrapper ceiling |
| --- | --- | ---: |
| research_lifecycle_state / RunKey | `{schemaVersion:1,body:ResearchLifecycleStateV1,lastCommit}` | 16 KiB |
| research_summaries / ResearchSummaryKey | `{schemaVersion:1,body:ResearchSummaryV1,digest,lastCommit}` | 16 KiB; body 8 KiB |
| research_lifecycle_records / ResearchLifecycleKey | `{schemaVersion:1,body:ResearchLifecycleDecisionV1,digest,receipt:ResearchLifecycleReceiptV1,lastCommit}` | 2 MiB; receipt 4 KiB |

Lifecycle records and summaries are immutable. Summary versions 1..64 are
consumed once; lifecycle UI IDs and per-plan completion keys are never recycled.
Existing retained-byte/discovery/command limits bound the full lifecycle history;
no new history allowance, eviction or independent worker is introduced. The
single mutable selector is retained even before any plan/summary/event exists.

Original root admission creates selector body with its exact run,
summaryVersion 0 and all four nullable pointers/disposition null, alongside the
initial research current run owner. Workspace initialize/draft prepare/cancel
creates no selector. All fourteen workspace, nine run and five assignment resource
names/creation meanings stay unchanged. Admission charges the new control
occurrence; it does not fabricate another zero ledger. Relative to the earlier
conditional root counts, this adds exactly one logical row: initialized 27→28,
never-initialized 24→25; other owned effects, K and obligations remain. These are
conditional counts, not complete executed manifests/packing or initialization proof.

Charge exact keys plus committed projections for all three families to the
existing shared child-control-retained-bytes run/workspace pools, 8 MiB/64 MiB,
reserved zero, and the 32 MiB run aggregate. Summaries are retained control/output
occurrences under this explicit proposed placement; root/source/report/preparation
and all copied historical data keep their original separate charges. No source
transfer, digest deduplication, summary-row credit or lifetime work refund follows.
Every changed control/summary/selector/resource original-to-final delta coalesces
once per ledger. New commands cannot create missing ledger peers or repair old
history. Account for all actual summary/index/proof/ordinary/physical occurrences.

Projection excludes only outer lastCommit and, for lifecycle_records only,
receipt.commit. Complete request, UI contexts, original snapshots/H and source
refs stay committed; summary body contains no new C. The new lifecycle record
ref inside the **selector poststate** uses the already computed decision digest,
not the selector's digest or C, so no fixed point is introduced. All smaller
source, 2 MiB row, 1 MiB paired job/ordinary, 4,096 changed-row/16 MiB command,
metadata discovery, scratch/store/index/physical limits win independently. A legal
1 MiB run snapshot plus copied peers can exceed its containing job and must deny;
no coupled maximum or complete byte total is qualified.

The accepted 54-name candidate would become **57 logical names** if these choices
are accepted, still not a final catalog or maximum. Physical locators/index
unions, initializer/direct proofs and exact command variants must close under one
recognized complete profile before full-state/history fixtures. Do not register
partial format7/next, infer a format from row presence, retoken old format-3/4/6
commitments or silently widen deployed child/ordinary readers. Earlier artifact
bytes and accepted codecs remain unchanged.

## Source membership and invalidation obligations

| Owner | Required exact retained membership in both directions |
| --- | --- |
| Summary | Original root/run/bootstrap, completed plan/task/turn/coordination/source group, exact input including older assessment provenance, actual parent runtime, every report/source/capture and any supported review; current versus historical consumption is explicit |
| Lifecycle selector | Exact run, selected completed research plan, current summary and original stage-cancel record when nonnull; high-water summary history and gate/run-state correspondence, including zero-reference initial owner |
| Lifecycle record | Exact request refs, original Main context or authenticated completion trigger, all typed snapshots/H/proofs, original plan/result/turn/reservation/allocation/ordinary lineage and native disposition where applicable, summary/cancellation refs and original receipt/new C |

Extend only the closed logical metadata families and origin recipes for these
records/snapshots; add no generic field path, user graph, ObjectRef kind or
purpose string. All historical summaries remain report consumers even after
currentSummary is null or the run is terminal. Source-owned current-summary/
pending-proposal relationships drive invalidation separately from immutable
retention. Every complete inventory must include zero-reference owners, current
and historical runs, retired expectations and tombstoned ordinary linkage.

The actual read/write union includes all original source/control/lifecycle/plan/
input/turn/coordination/report/runtime/allocation/ordinary owners, affected active
preparations, #744 custody/evaluation/artifact/summary, #745 exclusions, #746
tracked runs, ledgers/leases/health, physical source-to-index correspondence,
header and original/new proofs. A known list of matching edges cannot prove all
owners were enumerated. Unknown/unsupported consumers deny. Discovery and final
validation share 4,096 logical observations/16 MiB plus the existing 100,000-visit
ceiling and smaller worker limits, without truncation or splitting one outcome.
A fixed receipt/proof or metadata count alone qualifies none of that union.

## Owner choices and bounded next packet

Accept or revise four new representation/placement choices for **offline
components only**:

1. One immutable automatic completion record per exact research plan, published
   with its trigger's complete effects and coalesced versions, preserving live
   parent synthesis and ordinary child capacity under the accepted lifecycle.
2. The closed 8 KiB summary source, distinct 1..64 output versions/current selector,
   exact report/input/parent provenance and source-owned invalidation; required
   review/execution variants remain separate rather than flattened into emptiness.
3. Closed version-bound Main finish/stage-cancel/continue/run-cancel requests,
   captured decision/read/receipt shapes and preserved terminal gates/history,
   without a TerminalReportRunV1 cast, new permit or changed child cutoff.
4. Three mandatory lifecycle/summary families, original-admission selector creation,
   shared control occurrence accounting/projections and exact historical membership;
   candidate 57 and root counts remain conditional, not a selected storage format.

After scoped acceptance, only independent Python/JSON source/selector/decision/
summary examples and data-only JavaScript checks may follow. Build separate
source-derived snapshots; preserve all prior bytes. Include exact literal state
and disposition names; automatic completion versus UI finish; same-parent/no-new-
grant behavior; live-child/unknown-attempt capacity; stage cancellation, runtime
pause/resume precedence and explicit continue; terminal absorption/preserved gates;
UI retry before live CAS; summary revision/provenance/coverage/invalidation;
missing/extra origins, wrong families/digests/keys, exhaustion and independent
lower wrapper/job limits. Source/native/auth/coverage/currentness/C/command/proof/
outcome facts stay explicitly supplied and unqualified. No legal event history,
full catalog/header/manifest/row/change hash, runtime/native/model call, database,
product module/adapter/migration or executable product plan is authorized.

Positive execution/recovery-plan/required-review/clarification source variants,
actual source/authority/ordinary writer participation, full physical catalog/
index/proof/initializer and coupled/native/resource qualification remain blockers
before complete state/history or implementation. #1689/#1089 and broader component
issues stay open; #740 remains closed research. Gate 2 remains Investigate.

## Bounded review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only Luna
reviewers: accepted lifecycle/authority correctness and builder/evidence completeness.
Reviewer cap two; no edits or runtime/native/model/database probes. Correct any
material findings before handoff; verify docs metadata/index/tests/build/rendered
links, previous-artifact bytes, append-only source bodies and diff/doc/worktree
close-out. Publication/merge does not accept these four choices.


### Local review disposition — 2026-11-15

Two fresh read-only Luna reviews completed in the bounded round. The lifecycle/
authority review found no material contradiction. The builder review identified
three ambiguities: summary retry version selection, later completion-trigger
identity and assignment-choice cardinality. This revision supplies the closed
summary publication input and exact-key retry order, a coordination/recovery
trigger union with unsupported-trigger atomic denial, and explicit independent
16-entry bounds for all three summary lists. No runtime qualification follows.
