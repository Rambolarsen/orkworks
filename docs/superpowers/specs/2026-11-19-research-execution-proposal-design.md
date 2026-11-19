---
type: Design Proposal
title: Research-to-execution proposal and approval source
description: Proposed execution proposal decision and plan source, exact approval/rejection/invalidation records and fresh server-held grant binding under the sole research run owner.
tags: [orkworks, research, execution, approval, grants, qualification]
status: draft
workflow_status: proposed-for-scoped-execution-proposal-review
---

# Research-to-execution proposal and approval source

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumers
[#610](https://github.com/Rambolarsen/orkworks/issues/610),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `b3d0e87ed26c` (origin/main, including the clarification
proposal merged in [#1911](https://github.com/Rambolarsen/orkworks/pull/1911)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names the research-to-execution proposal and approval as an unclosed source
contract. This proposal closes that written boundary only: the execution
decision variant, the execution proposal plan source binding the exact
summary/completed-plan/input context, the exact shown-byte approval,
rejection/replacement/invalidation records, and the fresh server-held grant
boundary. It makes new record/action/tool and accounting choices explicit;
they are not acceptance inferred from a merge. Execution-child launch,
reservation, allocation, attachment, turn and coordination under execution
joins, multi-task execution plans, the direct-task skip path and final
terminal continuation remain separate #1689/#610 work. No dependent fixture,
Rust/Electron module, storage format, migration, probe or runtime execution
is supplied. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the accepted same-parent research-to-execution transition representable:
after a completed research plan and its current bounded summary, the live
parent can prepare one new execution proposal under a fresh plan identity,
Main can approve exactly the shown bytes, and approval creates one fresh
server-held grant — while the sole research run counter, immutable completed
research history, live children and capacity accounting stay untouched.

Preserve unchanged: the accepted
[source grammar](2026-11-12-research-source-codecs-design.md) and per-task
chain records; the [input/control prefix](2026-11-12-research-input-control-prefix-design.md)
including the decision union's existing members, the sole run owner and
paired revisions; the
[completion/lifecycle records](2026-11-15-research-completion-design.md)
including the selector's `completedResearchPlan`/`currentSummary`, finish and
cancel fencing, and the summary's exact input binding and currentness rule;
the accepted
[scope-rejection](2026-11-16-research-scope-rejection-design.md),
[prospective-membership](2026-11-16-research-prospective-membership-design.md),
[post-rejection](2026-11-16-research-post-rejection-report-design.md),
[stage-report](2026-11-16-research-stage-report-design.md),
[review-task](2026-11-18-research-review-tasks-design.md) and
[clarification](2026-11-18-research-clarification-design.md) contracts; #742
report/request/head/receipt rules; #744 evaluation/artifact/debit domains;
and the accepted transport/grant rules (the run bearer proposes but never
approves; approval is Electron-authorized; an expired bearer/grant is never
reactivated; a rejected proposal supplies no grant).

Rejected alternatives: reusing the direct-execution decision (`DirectDecisionV1`)
or the child-source plan codec by relabeling; deriving the proposal from the
summary's references instead of exact current bindings; approving through the
run bearer; a second runVersion owner for execution; carrying the research
plan's grant into execution; and treating proposal publication as approval.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Same-parent transition](2026-10-04-orchestrator-preparation-design.md#same-parent-research-to-execution-transition) | A new execution plan has a new `planId`, the same `runId`/`parentSessionId`, its own revision/digest and configurations, and references the completed research plan and input manifest; research plans remain immutable; the summary and proposal display together |
| [Planning is not launching](2026-10-04-orchestrator-preparation-design.md#same-parent-research-to-execution-transition) | No execution child, worktree, retry or tool-profile change is authorized by a summary or research-plan completion; only one plan revision holds a launch grant at a time; subsequent plans wait for completion/cancellation and resolved interrupted allocations |
| [Transport and grants](2026-10-04-orchestrator-preparation-design.md#transport-and-grants) | Electron-authorized approval of an exact plan creates a fresh server-held `PlanExecutionGrant`; launches present the run bearer plus exact run/plan/revision/task identities; the run bearer alone never grants a launch; a rejected execution proposal supplies no grant; final execution completion ends the run |
| [Grant record](2026-10-04-orchestrator-preparation-design.md#logical-records-and-immutable-parent-configuration) | `PlanExecutionGrant` is server-held: grant ID, run/parent/runtime/sidecar generation, exact approved plan revision and definition digest, UI approval ID, state `active`/`paused`/`revoked`; never a child-held credential |
| [Automatic completion](2026-10-04-orchestrator-preparation-design.md#clarification-and-research-lifecycle) | Missing research, conflicting required review or material uncertainty prevents a ready execution proposal; the summary binds exact report versions/digests and review references |
| [Completion summary/selector](2026-11-15-research-completion-design.md#immutable-bounded-synthesis-proposed) | The selector owns `completedResearchPlan` and `currentSummary`; summaries are derived output, not input records; an execution proposal must bind the exact SummaryRef, current input and completed research plan through its own reviewed source codec |
| [Plan completion effects](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Completion revokes that plan's grant, not the planning bearer; every nonterminal child still consumes run capacity; collection can stay active and later corrections invalidate summaries/proposals |
| [Decision union](2026-11-18-research-clarification-design.md#clarification-decision-and-request-records) | The input decision is a closed union; `ResearchDecisionV1` keeps its fixed pair; a fresh assessment is its own revision with current producer; this proposal adds the execution variant as another sibling, never normalizing into the existing ones |
| [Direct-path boundary](../../validation/research-source-coverage-audit.md#missing-source-owners-and-effect-boundaries) | The small direct-task path's skip assessment/input and execution sources remain unclosed in this research profile and cannot fabricate a completed plan or SummaryRef; this proposal closes only the research-to-execution branch |
| [#610 scope](https://github.com/Rambolarsen/orkworks/issues/610) | Ordinary-child orchestration: exact plan approval before worktrees/children; no recursive delegation, automatic integration, silent fallback or runtime availability without exact capability evidence |

These are bounded inspection facts at the inspection base. Use the proposed
research source/control/authority/publication, SessionApplication bridge and
Main approval seams from the
[prefix owner table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
No new module is created; type names are not factories or an existing runtime.

## Selected execution proposal slice

One completed research plan with a current summary yields at most **one**
pending execution proposal at a time, declaring **exactly one** execution
task, under a fresh plan identity in the same run. Multi-task execution
plans, parallel execution tasks, recursive delegation, automatic retries and
cross-plan worktree reuse are unsupported by this slice. A rejected proposal
supplies no grant; a replacement is a new proposal bound to the then-current
input/summary, never a mutation of the rejected bytes.

Execution-child admission, reservation, allocation, attachment, turn and
coordination records under execution joins are **not** closed here: approving
the proposal creates the grant and nothing launches. This is a deliberate
boundary — the next gate closes the execution launch chain under #610's
ordinary-child rules, and no eligible execution task profile is claimed.

## No new input revision: the summary currentness boundary

The parent's decision to propose execution is represented by the proposal
publication itself, bound to the exact current context — **no new input
revision is published and no decision-union sibling is added**. The input
decision union stays exactly as the clarification contract closed it
(`ResearchDecisionV1 | ResearchClarificationDecisionV1`), with both members
unchanged.

This is deliberate, not an omission. `ResearchSummaryV1.input` pins the exact
input at synthesis, and the accepted currentness rule clears `currentSummary`
when that binding becomes stale. Publishing a fresh assessment manifest for
the execution transition would advance the input revision, stale the summary
by that rule, and leave the proposal pinned to a summary that approval's
current-summary revalidation would then deny — while re-publishing the
summary afterwards cannot repair it, because the proposal would pin the older
ref. It would also create a summary→input→decision→summary digest cycle,
which the accepted completion contract explicitly avoided by making summary
publication derived output rather than a current-input record. The accepted
lifecycle instead prepares the summary and the execution proposal
automatically after successful research, with no intervening assessment
revision; this proposal follows that ordering exactly. The proposal source's
own binding (below) carries the assessment context: the exact current
summary, completed plan and input refs, plus the proposing parent's declared
reasons.

The audit's "fresh assessment including skip-research" obligation is
discharged by this authenticated, context-bound proposal publication for the
research-to-execution branch; the small direct-task path's skip assessment
and its execution sources remain separately unclosed and cannot reuse this
branch.

## Execution proposal plan source

`ResearchExecutionProposalSourceV1` is exactly:

```text
{schemaVersion:1,kind:"single-task-execution-source",parentPlanBinding,
 researchBinding:{summary:ResearchSummaryRef,completedPlan:PlanRef,
                  input:PreparedInputRef},
 task:{taskId,configuration:{configurationId,configurationDigest},
       initialPrompt,group},
 reasons:Text[],
 batches:[[taskId]],maxParallelChildren:1,scopeText,planDefinitionDigest}
```

- `parentPlanBinding` is the exact accepted ten-field shape; its
  `preparationRevision`/`inputDigest` resolve the **current** input manifest —
  the manifest at synthesis time, which the summary pins. No new input
  revision is published by this transition, so the summary's exact input
  binding stays current (see the currentness boundary above).
  `task.taskId` is new (never a research taskId);
  `initialPrompt` equals the referenced configuration's assignment
  description; `reasons` is 1..4 nonempty UTF-8 strings through 2 KiB each,
  explaining the proposed approach without hidden reasoning or executable
  task text; `scopeText` is nonempty UTF-8 through 4 KiB describing the
  proposed task scope; `group` is the exact common group shape
  (`{groupId,branchName,absolutePath,baseCommit}`), declaring a new independent
  plan-owned worktree — never a research group, existing directory or
  cross-plan reuse.
- `researchBinding` pins the exact proposal context: the selector's current
  summary ref, the completed research plan ref and the fresh assessment
  input ref. Every member resolves complete original bytes; a digest-only
  match cannot prove the binding.
- `configuration` resolves one full retained v3 configuration under the
  accepted **common leaf shapes** — the exact field sets of the child/research
  leaf grammar with the task's declared role, tools, permission profile and
  capability evidence supplied by its own #741 record; this proposal defines
  **no execution role/tool specialization**, and the concrete profile
  specialization is the separately gated qualification. Approval additionally
  requires the task's configuration role/profile to carry verified capability
  evidence under #741; absent a qualified profile, approval is unavailable —
  no fallback, no relabeled research or direct-execution profile.
- The plan digest uses `orkworks.research-execution-source-plan.v1` + LF over
  the complete canonical body excluding only `planDefinitionDigest` — a new
  domain, distinct from the child-source and research-source plan domains;
  no earlier artifact is retokened. Complete body ≤64 KiB; the existing
  **2 MiB complete approved-plan** accounting counts this body, the full
  configuration and every referenced capability artifact occurrence, with no
  sharing credit.

The proposal is published with the run's **current** input revision in one
outcome — the proposal preparation is one coalesced event that publishes no
new manifest. Admission precedes lookup:
current parent planning authority (bearer/scope/generation under the existing
rules) and the existing parent mutation rate discipline; publication is
unavailable while a current unanswered clarification request exists (a
material unanswered question blocks any dependent plan submission);
publication is unavailable while the stage fence is set; and no further
execution proposal is admitted while an active unexercised grant exists for
this run (the accepted one-grant-at-a-time rule; the next proposal waits
until the granted plan is complete or cancelled under the separately gated
execution chain). An authorized identical publication retry returns its
original outcome before live-version CAS with no mutation.
`ResearchExecutionProposalPublicationV1` is exactly
`{schemaVersion:1,expectedRunVersion,plan:ResearchExecutionProposalSourceV1}`
— the caller supplies the observed versions; the publisher never substitutes
current values before retry lookup. Like the accepted summary publication
input, this is an **offline publication input, not a retained record**: it
carries no digest of its own, belongs to no family, and is validated by exact
shape only. Publication effects: insert the immutable
proposal source object; create the plan control for the execution `PlanRef`
at version 1 with status `proposed` (the accepted status union value used by
the accepted proposal publisher) and the task control for the declared task
at version 1; append the execution `PlanRef` exactly once to the run's
ordered `planReferences` under the unchanged 16-plan/64-revision ceiling, so
the `pendingPlan` pointer resolves that list; bind the run's `pendingPlan`
to the execution `PlanRef`; leave the run's `currentPlan` pointer exactly as
the accepted research-plan completion left it — cleared to null — neither
reviving the completed research plan pointer nor clearing anything further;
advance the sole `runVersion` once; set the
run state `awaiting_execution_approval` — the accepted retained handoff value
gains its positive writer here — only when the actual parent runtime is
active/eligible and no retained blocker or stage gate remains, otherwise
retain the exact gated state (`blocked`, `paused` with any pauseReason, or
`awaiting_resume_approval`, per the accepted precedence rows) with the pending
proposal recorded; invalidate every
recognized earlier pending proposal under the accepted source-owned
invalidation; and publish the cursor event. The current input pointer, its
paired revisions and every retained clarification-answer binding stay
byte-unchanged. While paused during this state,
the retained `resumeTarget` is `awaiting_execution_approval`: that value is
already in the closed resume-target vocabulary (`assessing`,
`awaiting_execution_approval`, `executing`), and the accepted retain rule
keeps the underlying coordination target across repeated pauses — unlike
`awaiting_clarification`, which is genuinely absent from that vocabulary.
Resume reclassification follows the accepted rows.

## Exact approval, rejection and invalidation

The proposed exact UI approval request is:

```text
{schemaVersion:1,action:"orkworks.research-execution-plan.approve.v1",
 plan:PlanRef,approvalId,reviewContextId}
```

`ResearchExecutionPlanApprovalSourceV1` is exactly
`{schemaVersion:1,plan:PlanRef,approvalId,reviewContextId,sidecarGeneration,
requestDigest}`, with `requestDigest` using
`orkworks.research-execution-plan-request.v1` + LF over the complete canonical
request excluding only `requestDigest`; the source digest is
`orkworks.research-execution-plan-approval.v1` + LF
over the full body; the receipt is exactly
`{schemaVersion:1,plan,approvalId,approvalDigest,requestDigest,
commit:CommitRef}` at most 4 KiB. Retain complete reviewed bytes before
approval; require actual current Main authority/workspace/epoch/owner/lease/
health and the exact shown bytes/context before original retry lookup; an
authorized identical retry returns the original receipt before live-version
CAS with no mutation, even after the run becomes terminal; changed bytes
conflict; a revoked authority
cannot be repaired by a stored receipt.

A new approval requires: the exact immutable proposal bytes still pending and
current — its `researchBinding` revalidated against the selector's current
summary/completed plan and the current input (a stale binding denies with the
current context); no retained blocker; `stageContinueRequired` false; settled
research reservations and no interrupted allocation (per the accepted finish
preconditions, which approval inherits); the declared task not already
reserved under any plan; and the run not terminal. Approval effects: insert
the immutable approval source/receipt; set the plan control active with
`currentPlan` bound to the execution `PlanRef`; clear `pendingPlan`; advance
`runVersion` once; advance the changed plan control's `planVersion` once
(coalesced within the same outcome per the accepted rule); and create one
fresh server-held `PlanExecutionGrant`
bound exactly per the accepted record (grant ID, run/parent/runtime/sidecar
generation, exact plan revision and definition digest, UI approval ID, state
`active`). The grant is volatile server-held authority — never persisted as
a credential, never restored by the approval archive, never carried from the
research plan's grant, and revoked by run/stage cancellation, workspace or
sidecar change under the existing rules. Offline packets represent the grant
as **label assertions only** (the precedent label-key pattern, e.g.
`grantFencing:"supplied"`), never a serialized credential JSON; this
proposal defines no grant field spelling beyond the accepted record's named
members. This proposal closes no launch: the
run state remains `awaiting_execution_approval` until the separately gated
execution launch chain closes, and the grant simply exists unexercised.

Rejection is an explicit lifecycle decision, not the scope-revision rejection
contract: the accepted `reject-scope-revision` kind requires a nonnull
active/paused current research plan and a pending replacement with a
`replaces` relationship — preconditions this proposal's prestate (completed
research, `currentPlan` null, fresh execution plan identity) cannot satisfy.
Propose extending the accepted closed `ResearchLifecycleRequestV1` union with
exactly one new kind:

| kind | Additional fields |
| --- | --- |
| reject-execution-proposal | `decisionId,proposal:PlanRef,reason,reviewContextId,sidecarGeneration` |

`decisionId` is Main/sidecar-issued nonsecret ID under the accepted UI-decision
rules; `reason` is nonempty UTF-8 through 2 KiB; the request's common fields
(`run`, `expectedRunVersion`, `input`) bind the exact current input — byte-identical
to the proposal's `researchBinding.input`, since this transition publishes no
new manifest. Keys reconstruct as for every UI kind
(`{kind:"ui-decision",run,decisionId}`); the decision/readSet/receipt machinery,
the unchanged `ResearchLifecycleReadSetV1` (with `pendingPlan` capturing the
execution plan control read at status `proposed` and `currentPlan` null) and
the ≤4 KiB receipt (with `planVersion` null, matching the accepted rule that
only completion/cancel-stage report the current plan's version) are reused
unchanged. Admission: current Main authority/workspace/epoch/owner/lease/
health and the exact shown context before original retry lookup; the exact
pending proposal still exists at status `proposed` under its original
`PlanRef`; no active grant exists for it (rejection applies to a pending
proposal; revoking an active, unexercised grant is a separately gated
decision); no retained blocker; `stageContinueRequired` false; run not
terminal. An authorized identical retry returns the original receipt before
live-version CAS with no mutation, even after the run becomes terminal.

The rejection outcome, in one authenticated union following the accepted
scope-rejection effect pattern: insert the immutable lifecycle
decision/receipt; mark **only** the rejected execution plan `cancelled` and
advance its `planVersion` once — `cancelled` denotes a non-executable
rejected proposal here, not cancel-stage of an approved plan — retaining its
task at pending/version 1 with no reservation, as historical unreachable
declared work; clear `run.pendingPlan`; create no collection/expectation
effects (this slice's execution proposal creates none); retain the rejected
proposal object, approval-less history and all source charges; advance the
sole `runVersion` once; retain the run state `awaiting_execution_approval`
(the accepted "same approval-wait state": the run remains post-research and a
replacement proposal is a new publication bound to the then-current context,
never a mutation of the rejected bytes); and publish the cursor event.
`finish-research`, `cancel-stage` and `cancel-run` retain their accepted
effects, each invalidating the recognized pending execution proposal; stage
fencing rejects proposal publication, approval and rejection while
`stageContinueRequired` is set. Invalidation remains the accepted
source-owned rule: any genuinely changed report source or summary
currentness change invalidates the pending proposal under the accepted
membership contract before it can be approved — the stale-binding denial
above is its admission-time backstop, and because this transition publishes
no input revision, an unchanged report/summary history cannot stale the
binding on its own.

## Families, accounting and membership

Propose two new logical names in the eventual single fresh catalog (no
physical table, layout or format is selected): the object kind
`research-execution-plan / PlanRef` (body
`ResearchExecutionProposalSourceV1` and its plan domain) and the metadata
family `research_execution_plan_approvals / {plan:PlanRef,approvalId}`
(value `{schemaVersion:1,body:ResearchExecutionPlanApprovalSourceV1,digest,
receipt,lastCommit}`, wrapper ≤16 KiB, projection excluding outer `lastCommit`
and `receipt.commit`). The conditional candidate catalog becomes **63 logical
names** (61 after the clarification proposal + one object kind + one family);
this remains neither a final catalog nor a maximum, and initializer/root-row
consequences stay with the separately tracked catalog closure.

Charge exact keys plus committed projections for both to the existing shared
child-source-retained-bytes run/workspace pools (8 MiB / 64 MiB), reserved
zero, inside the 32 MiB run aggregate — the execution plan and its
configuration are retained source occurrences, like research plan sources.
No new allowance, deduplication or ordinary-end refund follows. The
run-control pending/current plan bindings use original-to-final deltas under
the existing control charges.

Required membership in both directions, following the accepted recipes with
no new dependency purpose:

| Owner / key | Complete required typed references |
| --- | --- |
| research-execution-plan / PlanRef | Original run/bootstrap, the current input manifest the proposal binds (no new manifest is published), the exact `researchBinding` members (summary with its report/review provenance, completed plan with its completion record, input ref), the referenced configuration and capability artifacts, and original/new direct proofs |
| research_execution_plan_approvals / `{plan,approvalId}` | The exact proposal object, the shown Main context, the grant-binding generation facts as historical data (never live authority), the original receipt/new C, and the run/plan-control outcomes that consumed the approval |

Unknown or unsupported consumers deny; absence of a pointer never proves
absence of consumers. The accepted discovery bounds (4,096 observations /
16 MiB cumulative, 100,000-visit ceiling and smaller worker limits) apply
unchanged.

## Compatibility

No existing source digest, wire field, report identity, dependency purpose,
version default, cap, receipt version or volatile credential serialization
changes beyond the explicitly proposed lifecycle-request kind, the new plan
kind/domain, the proposal publication record, the new object kind and
family, and the `awaiting_execution_approval` positive writer. The accepted
64-revision and 32 question/answer pairs bounds, 16-plan-reference run
ceiling and every smaller row/worker/command/physical limit win
independently; a maximal combination that exceeds a containing budget denies.
Old stores are never imported, extended on first read or retokened; the
proposal can be reverted without store effects.

## Owner choices and bounded next packet

Accept or revise these proposed choices for **offline components only**:

1. The **no-new-input-revision boundary**: the input decision union stays
   exactly as the clarification contract closed it; the execution transition
   is represented by the authenticated, context-bound proposal publication
   itself (exact current summary/completed plan/input binding, no fresh
   assessment manifest), preserving the summary's exact input currentness
   and avoiding the summary→input→decision→summary digest cycle; the
   direct-task skip assessment stays unavailable.
2. The `single-task-execution-source` plan shape with the exact
   `researchBinding`, new plan domain and unchanged parent-binding/budget
   rules; exactly one declared task; no research-worktree or cross-plan reuse.
3. Proposal publication with `expectedRunVersion`, the `pendingPlan` binding,
   the `awaiting_execution_approval` positive writer, earlier-pending
   invalidation and gate-respecting state selection.
4. Exact approval/rejection/invalidation records under unchanged Main
   authority/retry rules, with the stale-binding denial and inherited
   settled-reservation preconditions, plus the proposed
   `reject-execution-proposal` lifecycle kind (marking only the rejected
   plan `cancelled` as a non-executable rejected proposal, clearing
   `pendingPlan`, retaining the approval-wait state).
5. One fresh server-held `PlanExecutionGrant` per approval under the accepted
   grant record, with no launch closed here and the execution launch chain
   remaining separately gated; two catalog additions (conditional 61→63) and
   shared-pool accounting.

After scoped acceptance, the bounded packet covers `docs/validation/` only:
an independent Python generator, retained JSON, data-only JavaScript
checker/tests and evidence prose, plus documentation routing and LF
attributes. Required examples: the proposal publication bound to the exact
current summary/completed plan/input (including the summary-currentness
preservation and gated/blocked retained-state cases); approval under the exact
shown bytes with identical retry and conflict; stale-binding denial after a
summary/report change; the `reject-execution-proposal` decision with its
cancelled-plan/pendingPlan/state effects and replacement; finish/cancel-stage/
cancel-run invalidation of the pending proposal; grant-boundary examples
(labelled unqualified: the grant is a supplied volatile fact represented as
label assertions only, never persisted); the positive approval example
labels the capability/profile-qualification fact as supplied unqualified
(precedent label-key pattern), and denial cases (bearer-as-approver, research-grant carryover,
multi-task plan, reused worktree group, unqualified profile approval, stale
completed plan, wrapper overflow). All auth, transport, coverage,
currentness, C/command/proof, grant, capability/profile-qualification and
outcome facts remain explicitly
supplied and unqualified. No legal event history, complete transaction,
storage format, runtime/native/model call, database, migration, product
module or executable product plan is authorized.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: accepted-contract correctness (joins, bounds, grant boundary and
the #610/#742/#744 boundary) and builder completeness (material unanswered
offline-component questions). Reviewer cap two; no reviewer edits, fixtures,
runtime, native or model probes. Correct material findings before handoff;
verify docs metadata/index, build, rendered links, prior-artifact byte
preservation, append-only prior bodies and diff/doc/worktree close-out.
Publication/merge of this proposal does not accept the five choices above.

## Local review disposition

One bounded round used two fresh read-only reviewers. The builder review
identified five material gaps: an undefined approval `requestDigest` domain,
the profile-qualification fact missing from the supplied/unqualified list (which
gates the positive approval example), an ambiguous grant representation for
the packet, an unlabeled retained status for the publication input, and an
undefined execution-configuration leaf specialization. The correctness review
confirmed the third decision sibling, source/digest closure, version
coalescing, the grant rules and the candidate arithmetic, and found five
material issues — the resume-target claim contradicting the closed
`ResumeTarget` vocabulary, a nonexistent "pending" plan-control status, four
omitted mandatory publication effects (plan/task controls, `planReferences`
append, the coalesced assessment/proposal outcome and admission
preconditions), an unhandled active-unexercised-grant state inviting a second
proposal, and `currentPlan` wording readable as reviving the completed plan —
plus three minor gaps.

This revision resolves all of them: the request-digest domain is named;
capability/profile qualification joins the supplied-facts list and the
positive example labels it; the grant is represented as label assertions
only; the publication input is explicitly non-retained; the configuration
resolves the accepted common leaf shapes with no execution specialization
defined; the resume target while paused is `awaiting_execution_approval`
(already in the closed vocabulary, retained across pauses); the plan control
uses status `proposed` from the accepted union; the publication union now
creates plan/task controls at version 1, appends the `PlanRef` once to
`planReferences`, coalesces assessment and proposal into one outcome with
authority/rate/no-unanswered-clarification/no-active-grant/stage-fence
preconditions and retry-before-CAS; the one-grant-at-a-time rule explicitly
forbids further proposals while an unexercised grant exists; the
`currentPlan` wording states the accepted cleared-null semantics; terminal
retry wording matches the accepted rule; and the approval outcome advances
`planVersion` once. No accepted technical definition changed and no runtime
qualification follows. Neither reviewer executed code, a fixture, database
or native/runtime process.

After the hosted review of head `f27c84bb`, two further P2 findings were
addressed in this revision: (1) the proposed fresh-assessment input advance
would have staled the summary's exact input binding and created a
summary→input→decision→summary digest cycle — the transition now publishes
**no new input revision and no decision-union sibling**; the input decision
union stays as the clarification contract closed it, and the proposal's own
context-bound publication (with `reasons` moved into the plan source)
discharges the research-to-execution assessment while the direct-task skip
path stays separately unclosed; (2) the referenced "existing rejection" was
the scope-revision rejection, whose preconditions this prestate cannot
satisfy — the proposal now defines the explicit `reject-execution-proposal`
lifecycle-request kind, following the accepted scope-rejection effect
pattern (rejected plan `cancelled`/planVersion+1, task retained pending,
`pendingPlan` cleared, approval-wait state retained, no grant touched).

## Remaining gates

Execution-child launch/reservation/allocation/attachment/turn/coordination
under execution joins, multi-task execution plans, the direct-task skip
path, execution role/profile qualification under #741/#740's closed-research
standard, final execution completion and terminal continuation, actual
native/runtime participation, complete physical catalog/version/initializer/
proof closure and separately approved executable plans remain open.
#1689/#1089/#610 stay open; #740 stays closed research. Gate 2 remains
**Investigate**.
