---
type: Design Proposal
title: Pending research scope revision rejection and atomic completion
description: Proposed exact research replacement-input binding, Main rejection record and typed compound completion trigger without replacement approval or physical qualification.
tags: [orkworks, research, scope-revision, lifecycle, source-contract]
status: draft
workflow_status: proposed-for-scoped-offline-review
---

# Pending research scope revision rejection and atomic completion

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `8d5c94d36f15b27d9808523c94ecb66a7bb432c3`.

This is a written proposal for three new representation choices, not acceptance
or a dependent fixture. The owner authorized the next prerequisite after the
[source coverage audit](../../validation/research-source-coverage-audit.md#bounded-next-task-and-acceptance-boundary).
Existing lifecycle acceptance remains valid; this proposal does not ask to
reapprove it. Gate 2 stays **Investigate**. No runtime/native/model/database
execution, product module, migration, physical format or activation is supplied.

## Required behavior and investigated gap

The [accepted completion rule](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan)
requires atomic rechecking after rejection of a pending scope revision. Every
task must have an explicit reported_complete result, reservations must be
attached/settled, and no interrupted allocation or pending revision may remain.
A live ordinary child PTY does not itself delay completion or release capacity.
The same baseline permits drafting while children are live but prohibits
approving a replacement until prior launched children have results and are terminal.

The [research completion union](2026-11-15-research-completion-design.md#original-completion-and-ui-request-codecs-proposed)
currently supports only coordination/recovery triggers. Its unsupported-trigger
rule correctly prevents committing a rejection alone when completion is due.
The [research prefix](2026-11-12-research-input-control-prefix-design.md#proposed-immutable-research-input)
excludes prior-plan records, while the
[common plan control](2026-11-02-child-launch-control-design.md#task-launch-and-ordinary-session-controls)
requires replaces to resolve the exact prior-plan input entry. A pending pointer
or same planId alone cannot supply that missing source relationship.

These are the two outcome-changing uncertainties investigated here. The proposal
closes them narrowly using an existing source plan and lifecycle family rather
than a generic proposal object, arbitrary pointer relationship or manual finish.
It does not qualify the actual producer/native/proof chain. The coverage audit's
bounded source search and dormant production seams remain the evidence of current
implementation limits; documentary module names are not deployed owners.

## Selected pending proposal and immutable input

Propose adding exactly this variant to ResearchInputRecordV1:

```text
{kind:"prior-plan",plan:PlanRef}
```

Use the common direct-input spelling and meaning, but resolve only the research
registry. This is an explicit research-union extension, not permission to import
all direct/reviewer input variants. For the selected scope-revision proposal,
derive exactly one such entry naming the current original research PlanRef.
Keep every other required configuration/scope/capture/selected-report entry,
UTF-8 sorting/uniqueness, 256-record/32 KiB containing bounds, original assessment
provenance and the unchanged preparation-input domain. No pending plan digest,
grant, future reservation or mutable control enters its own prepared input.

Freeze the next exact input before computing the proposed source plan. Changed
sources/decision advance preparation/input revisions together once through 64;
old input and producer history remain retained. Identical unchanged inputs reuse
their original manifest; a new plan cannot overwrite an existing PlanRef.
ResearchDecisionV1 still has clarificationNeeded=false/researchNeeded=true.
No material-clarification/reviewer/execution variant is normalized into it.

The proposal uses unchanged ResearchSourcePlanV1 and PlanRef as its immutable
identity, in the same run and same planId with planRevision exactly one after
the last submitted revision for that planId (checked through 64). Resolve that
submission from the complete retained ordered planReferences and its original
source/proofs, including rejected/cancelled proposals. For example, approved
revision 1 and retained rejected revision 2 require a changed proposal at 3,
still replacing approved revision 1. Never overwrite revision 2, recycle a
rejected identity, or elect a maximum from incomplete rows to repair history.
It declares a distinct taskId, configurationId and new
plan worktree group, within the same immutable bootstrap/repository permissions.
Complete configuration/scope/capture/capability sources and their original
custody/byte accounting must resolve. Rejected proposals cannot authorize broader
bootstrap permissions, and this selected source grammar still excludes review
dependencies or multiple tasks. Wider revisions need their own contracts.

The required original proposal outcome must atomically retain those sources/input/producer,
create proposed plan/task controls at version 1, update currentInput and pendingPlan,
and append the exact PlanRef once to planReferences. The proposed plan control
has status proposed, latestApproval=null and replaces equal to the current PlanRef
in its exact prepared input. Its only task has status pending, reservation=null,
coordinationResult=null, taskVersion=1. There is no proposed launch/session,
approval, reservation, allocation, ordinary child, assignment or live grant.
Collection/expectation creation and later retirement must follow their complete
owning contracts, not an absent-row inference. **That reserved-current-plan
proposal publisher remains unavailable in the selected singleton prefix.** Its
RunKey collection cannot acquire a second active task or silently retarget the
reserved current task. The prefix explicitly leaves reserved replacements to a
separate source/effect closure. This proposal does not extend that membership
codec or claim a legal positive pending prestate. Finite future components may
stipulate the required complete source membership, but cannot qualify its producer
or construct a source-valid store/history until that separate closure. Both current and pending plans
stay retained; current approved task/reservation/source bindings never move.

Only one pending proposal is selected by the current run pointer. This document
closes the stated source relation and rejection outcome, not arbitrary proposal
replacement, approved revision handover or a fully qualified proposal publisher.
Unknown prior-plan provenance, stale source/current input, unsupported consumer
effects or missing initializer peers make the whole operation unavailable.

## Exact Main rejection request and captured decision

Propose one additional ResearchLifecycleRequestV1 kind, with exactly:

```text
{schemaVersion:1,kind:"reject-scope-revision",run:RunKey,
 expectedRunVersion,input:PreparedInputRef,decisionId:ID,
 plan:PlanRef,expectedPlanVersion,
 proposedPlan:PlanRef,expectedProposedPlanVersion,
 reason:Text,reviewContextId:Digest,sidecarGeneration:Digest}
```

Versions retain 1..2^31−1 JSON bounds. reason is nonempty well-formed UTF-8 through
2 KiB. Main/sidecar issues the unique decisionId and exact displayed-byte context;
the renderer/model cannot authenticate its own ID, digest or generation. The
action must explicitly display the rejected proposed source/delta and the exact
current plan/input/versions. Parent prose is not the rejection authority.

Reuse ResearchLifecycleKey `{kind:"ui-decision",run,decisionId}` and the unchanged
ResearchLifecycleDecisionV1/ReadSetV1/ReceiptV1 wrappers and
`orkworks.research-lifecycle-decision.v1` + LF body digest. No new request digest
domain, source object kind or family is introduced. The closed request union is
explicitly extended; old readers are not research-rejection eligible by row presence.

The readSet captures one H, exact current research_run_controls, lifecycle selector,
current plan and pending plan snapshots. Both plan reads are nonnull; run.currentPlan
equals request.plan and run.pendingPlan equals request.proposedPlan. Current plan
is active or paused, never already complete/cancelled/superseded. Pending plan is
proposed/unapproved and has the replaces relationship above. Expected versions
and current input equal original captured bodies. Read all proposed task/source/
producer/collection/expectation rows and absence memberships through their actual
owners; null fields and missing selected rows do not prove global absence.

The current plan's older approved input remains its own source. The rejection
request binds today's captured currentInput, which may be the newer proposal input.
Never revalidate an old task/result as if its frozen source had changed to that
new manifest. Rejection does not roll currentInput backwards, erase its prior-plan
entry, recycle revisions or create a fresh assessment.

Current Main authority, exact shown context/generation and workspace/epoch/owner/
lease/health precede original UI request lookup. A new request revalidates the
complete captured state and current source/custody at the actual publication
boundary. No late-save permit, parent bearer revival or extra worker is selected.
Run terminal state, stage-continue restrictions, unrecognized clarification/source
variants or stale input/context deny a new selected rejection; preserve all gates.
An interrupted parent is not silently resumed by a Main decision.

## Typed completion trigger and compound effects

Propose adding exactly this ResearchCompletionTriggerV1 variant:

```text
{kind:"scope-rejection",rejection:ResearchLifecycleRef}
```

rejection.key must be the same run's original UI rejection key; rejection.digest
pins the full original rejection decision body. Resolve that body's exact current
plan/pending proposal/source/outcome. This is not a generic reference to any UI
decision and cannot point to finish, cancel, continue, another plan or another C.
Build the rejection body/digest first, then the completion request/body/digest.
Neither contains new C, so there is no digest fixed point. Keep existing lifecycle
source domain and one original completion record per exact current PlanRef.

Apply these original-to-final effects in one authenticated outcome:

1. Retain the rejection UI decision/receipt. Mark only the rejected proposed plan
   cancelled and advance its planVersion once; cancelled denotes a non-executable
   rejected proposal here, not cancel-stage of the approved current plan. Retain
   its task at pending/version 1 with no reservation or result; it is historical
   unreachable declared work, never reusable as a newly approved attempt.
2. Clear run.pendingPlan. Retire only the proposed plan's prospective expectation
   and any collection membership actually owned by it through exact complete
   ownership, with their actual version/index/
   invalidation effects; retain original rows/sources/history and source charges.
   No current approved plan/task/child collection is retired by this rejection.
   Do not invent a second RunKey collection: unavailable reserved-replacement
   membership semantics above block actual publication and legal-history fixtures.
3. Evaluate completion over that candidate, using all actual current-plan tasks,
   turn/results, reservation/launch/ordinary/source/native membership and no
   pending revision. If any condition remains false, retain current plan/status/
   version and its existing grant fences. Do not manufacture a result or completion.
4. If eligible, insert the current plan's automatic completion with the exact
   scope-rejection trigger, mark it complete/planVersion+1, revoke its exact grant,
   clear currentPlan, select completedResearchPlan, and clear an obsolete current
   summary under its existing owner. Collection/report history stays retained;
   no execution approval or task launch follows.
5. Coalesce research_run_controls.runVersion+1 once for the union. Both lifecycle
   decisions capture byte-identical original run/H/selector/current/pending reads,
   not candidate repaired snapshots. Both receipts, all changed controls and the
   full actual effect/proof union bind the same C. Proposed/current plan versions
   each advance once only when their respective control changes. Task and ordinary
   bridge versions do not advance merely because a proposal was rejected.

Rejection's receipt.planVersion stays null, preserving the receipt's rule that
only completion/cancel-stage report the current plan's resulting version. The
automatic completion receipt carries that actual resulting current planVersion.
The rejected proposal's resulting version is verified from captured version+1 and
the original complete outcome, not a second mutable counter or extra receipt field.

Run state selection preserves stage/blocker/clarification and runtime pause
precedence. Incomplete work remains researching or blocked according to its
actual conditions; an approval-wait proposal rejection is not a manual route to
synthesis. True completion enters synthesizing only for an eligible live parent
without higher gates. An interrupted parent stays paused with its reason and
synthesizing resumeTarget; blockers can keep the run blocked despite a completed
plan. Unsupported broader state variants deny rather than discard their data.

Rejected replacement approval and rejection serialize against the same exact
versions/pointer. Approval-first makes this request stale; rejection-first makes
that proposal unapprovable. Fresh changed proposal needs its own input/source/
identity and exact approval, and retains prior-child result/terminal requirements.
Rejection itself requires no child kill/end and releases no capacity. Neither
current task success nor plan completion makes a still-live ordinary child free.
Same-parent planning, independent child reporting and ordinary stop remain under
their existing contracts; stop/input may not wait on hierarchy SQL/fsync.

## Original retry, races and unavailable outcomes

After current Main/context/owner checks, identical request bytes at the occupied
UI key return the original receipt before live CAS, even after later run closure.
Changed bytes conflict. Do not reconstruct the rejection body using today's H,
input, proposed status or plan versions. No grant or version is restored on retry.

The original complete command proof determines whether this rejection also
completed its current plan. If so, resolve the original completion key/trigger/
digest and paired C/receipt. If not, prove that original non-completing outcome
and return its rejection receipt alone. Absence of a current completion row is
not an absence proof, and a later completion with a different trigger is not its
companion. Missing/conflicting original proof/companion makes retry unavailable;
never rerun completion against live state or repair with a fresh decisionId.

Cancellation/input change/another resolution winning before publication denies
the stale new request; source generations and exact H/read set are revalidated.
If this compound outcome wins, later writers include it under their own contracts.
Unknown original commit/native disposition retains its sole owner/protected leases
and capacity; no timeout rollback, replacement worker or invented success receipt.
Ordinary stop stays independent. No new terminal-writing or correction exception.

## Catalog, retention, consumers and limits

Reuse the existing source plan/input registries, controls, lifecycle records and
selector. This adds zero family names; candidate **57** and original selector
admission counts **28/25** remain conditional, not a complete or maximum catalog.
The input/request/trigger unions gain only the three variants explicitly proposed
here. No schema version bump is selected for existing v1 envelopes: eligibility
requires a future complete recognized profile, not optimistic old-reader parsing.
Old child/ordinary and format-3/4/6 artifacts are not widened or retokened.

Charge full retained new input/source copies to their existing preparation/source
pools and complete approved-plan occurrences; charge both lifecycle records and
all changed controls/selector/collections to existing shared control accounting.
Control pools remain 8 MiB/run, 64 MiB/workspace and 32 MiB run aggregate. Original
snapshot/H copies are charged twice in the two records; coalesce ledger deltas
original-to-final once. No deduplication, purge/refund, missing-ledger repair,
new zero ledger or counter reset follows rejection. Sources stay retained with
plan/input limits (16 plan IDs, 64 revisions/input/preparation) and source caps.

Keep 2 MiB decision wrappers, 4 KiB receipts, 1 MiB run snapshots, 16 KiB plan/
selector/task and existing launch/session bounds, smaller 1 MiB paired-job/ordinary
ceiling, 4,096 changed rows/16 MiB command, bounded discovery and physical/scratch
limits. Any overflow denies the whole mutation. Two small records do not prove
one command fits. Exclude only each own outer lastCommit/receipt.commit from its
projection; captured H/origins and the rejection ref inside completion stay hashed.

Retain direct original origins and bidirectional membership for both plans and
their source/configuration/capture/input/producer/prior-plan links, plan/task
controls, current/pending/selector pointers, rejected prospective collections/
expectations, live ordinary/reservation/native owners, turn/result/report source
groups, rejection/completion records, summary invalidation and all resources/
indexes/proofs. Include historical/tombstoned/zero-reference owners and #744–#746
consumers. A matching edge list does not prove completeness. Actual source-owned
discovery and invalidation recipes, full original command proof and initializer
union still need closure under one complete physical profile before history.

## Owner choices and bounded handoff

Accept or revise these choices for **offline components only**:

1. The exact research prior-plan input variant and selected unapproved replacement
   source/control relationship, with immutable input history and no approval path.
2. The version-bound Main reject-scope-revision request in the existing lifecycle
   family, cancelled proposed control semantics and exact original retry/proof rules.
3. The typed scope-rejection completion trigger and one H/C effect union, preserving
   sole run ownership, historical sources, gates/capacity and unchanged limits.

Only after acceptance may a bounded Python/JSON/data-only JS packet supply positive
completion and incomplete-work alternatives, prior-plan/input/source joins, exact
original retry/denial/charge checks and preserved prior bytes. Supplied auth/native/
currentness/source membership/H/C/proof/outcome labels remain unqualified. No full
catalog/header/manifest/row/change hash, legal history, runtime/native/model probe,
product writer, migration, adapter, executable product plan or activation follows.
Research review, clarification, execution and the complete physical profile remain
separate gates, including the reserved-current-plan prospective membership source
closure identified above; keep #1689/#1089 open and #740 closed research.

One writer owns the proposal. One round of at most two read-only Luna reviewers
checks correctness and builder completeness before handoff. Verify all existing
offline docs tests, OKF/index/rendered links, prior artifact bytes and append-only
routing. Review is evidence of this written boundary, not owner acceptance.

## Review and verification disposition — 2026-11-16

One bounded round used two fresh read-only Luna reviews: correctness and builder
completeness. The builder found the three representation choices concrete enough
for finite offline components, while affirming the distinct reserved-replacement
membership/history blocker. The author raised a revision-collision concern and
the correctness reviewer confirmed it: current+1 cannot support a changed
proposal after an earlier rejected revision remains retained. The corrected rule
uses the complete original-owned submission order including cancelled entries,
without maximum-based recovery or identity reuse. No other material issue was found.

All existing 1,881 offline documentation tests and 20 OKF regressions pass; no
new fixtures or code tests were added for this Markdown-only proposal. Final
docs metadata/index/rendered links and close-out evidence are recorded with the
PR. All 569 prior non-Markdown validation artifacts are byte-identical and the
earlier coverage audit is append-only. Existing bundler warnings and another
owner's stale worktree require no unrelated edit. Worktree reporting succeeded;
primary residue remains untouched. No product/native verifier or runtime call
was run, and no broad repository or producer qualification is claimed.

## Scoped owner acceptance and offline handoff — 2026-11-16

The owner answered “go ahead” to the three choices following #1778. All three
are accepted for finite offline components only. This supersedes pending-choice
wording at that bounded scope; existing technical bodies and historical review
statements remain unchanged. The [component packet](../../validation/research-scope-rejection-components.md)
supplies independent prior-plan/source joins and completing/incomplete rejection
alternatives, original retry/denial and occurrence checks. Reserved-current-plan
singleton membership remains unclosed: no second active collection, legal pending
prestate, source-valid publisher/history or runtime qualification follows.

## Canonical prospective membership proposal — 2026-11-16

The [reserved replacement membership proposal](2026-11-16-research-prospective-membership-design.md)
makes collecting A versus non-collecting prospective B and report/input
invalidation choices concrete. Its three author choices remain pending scoped
acceptance; this routing does not extend the earlier offline acceptance.

The earlier stale-input denial stays binding for the accepted slice. The new
proposal requests this exact conditional exception, **only if separately accepted
for a complete recognized research profile**: when an A report advances current
input and makes the exact still-pending B's original input stale-for-approval,
a fresh Main rejection may resolve retained B using today's request.input and
shown context/versions while validating B's older original source/input/proofs.
The differing input identities alone are not source corruption. A stale rejection
request/context, unknown/corrupt source or custody, arbitrary stale proposal,
old-profile reinterpretation or missing original proof remains unavailable.
This adds no request field, domain, receipt or completion trigger; it proposes a
narrow eligibility rule, not a late-save or restored-authority exception.

On rejection, B's unchanged false expectation receives no expectationVersion
increment. The canonical proposal separately selects collectionVersion+1 for
removal of the collection's owned prospective role even while collection.task
stays A. That is its pending first owner choice, not permission to infer a role
from collecting=false. Original task/report/ordinary capacity and conditional
scope-rejection completion rules remain intact.
