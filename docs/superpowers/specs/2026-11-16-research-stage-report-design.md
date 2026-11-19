---
type: Design Proposal
title: Research reporting across stage cancellation and continuation
description: Proposed original-cancellation source roles, retained report-input history and version-bound continuation under unchanged child and Main authority.
tags: [orkworks, research, reports, cancellation, continuation, source-contract]
status: draft
workflow_status: proposed-for-scoped-stage-report-review
---

# Research reporting across stage cancellation and continuation

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `947733ee` (origin/main, including #1802).

This is the next written prerequisite after the
[post-rejection report packet](../../validation/research-post-report-components.md#reproduction-and-next-qualification-gate).
Existing acceptance remains valid at its recorded scope. The three new choices
below are **proposed for offline components only**; merging this document does
not accept them. No dependent fixture or product/runtime/native/model/database
execution, adapter, migration, physical format or activation follows. Gate 2 stays
**Investigate**.

## Established behavior and remaining gap

The authoritative [Taskmaster extension](../../../specs/taskmaster.md#proposed-ordinary-child-orchestration-extension)
keeps this entire orchestration path separately gated. The accepted
[logical lifecycle](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table)
and [completion contract](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union)
already require stage cancellation to preserve existing child report rights,
collection, native attempts, blockers and history. Only a separate exact Main
continue decision clears the stage gate. These are existing requirements, not
new permissions selected here.

The source gap is the collecting task's role after currentPlan becomes null
because its **approved current stage** was cancelled. The earlier post-rejection
packet instead cancels an unapproved prospective B while collecting A remains
active or completes. A cancelled approved A cannot borrow that rejection or a
paired completion. The original cancellation and retained source role need their
own complete joins before a positive report example is meaningful.

Select one originally approved research PlanRef A, one reserved task TA and its
original successfully spawned, currently eligible nonterminal child RuntimeRef.
No prospective/rejected B, other plan, prior completed research, required review,
material clarification, execution source, new assessment/proposal or resumed
child/parent generation is admitted in this bounded branch. Complete membership
must prove those selected absences; a null pointer is insufficient. Broader valid
contexts remain outside this packet rather than losing their rights or history.

## Original cancellation and retained collecting ownership

Resolve the following complete source facts at captured H, independently of any
caller label or old successful report:

1. Original root/run/bootstrap, ordered PlanRef history, A definition and approved
   input, configuration/scope/captures, approval, reservation, allocation,
   attachment, frozen assignment and actual original successful spawn. Resolve
   current ordinary/native child ownership and generation separately; neither
   an old spawn nor cancelled-plan status proves the runtime is still live.
2. The exact original Main cancel-stage request, lifecycle key/body/digest,
   captured run/plan/selector/input snapshots, original request binding, receipt
   and complete original H/C outcome. Its request targets A as the then-current
   nonterminal plan with exact plan/run versions and current input. It is not
   reject-scope-revision, cancel-run or an arbitrary cancelled row. Validate the
   original cancellation grant revocation, A's resulting cancelled plan version,
   cleared current/pending pointers, run pause/gate and selector pointer together.
3. The current active collection and sole collecting TA expectation still resolve
   their original reservation/configuration/scope/Stream. Cancellation changes no
   collecting role, reservation or report selection; it gives no reason for a
   collection/expectation increment or retirement. Historical and zero-reference
   memberships remain discoverable. Cancellation is a lifecycle gate, not a new
   report source, automatic task result, child-stop event or completion trigger.
4. A remains cancelled with its original resulting version; TA retains its exact
   live coordination version/status/result and frozen assignment identity. No
   later report or continue can reactivate A, reapprove it, transfer the reserved
   task, restore a launch grant or claim a released live-child slot.
5. run.currentInput resolves the latest retained manifest in A's selected source
   context. It may contain no report yet or the latest selected original-stream
   report. Resolve its exact original assessment row/producer and copied decision
   independently. Plan approval input is historical; it need not equal this latest
   manifest after report collection.

Before continuation the selected current run is paused with stageContinueRequired
true, pauseReason=stage_cancelled and its retained logical resumeTarget. Selector
pendingStageContinue names the exact original cancellation; completedResearchPlan,
currentSummary and finishDisposition are null and summaryVersion is zero in this
bounded original-A branch. This is an explicit source selection, not a rule for
all cancellations: earlier completed-plan/summary history requires its own complete
owner and must never be cleared to force these values.

The live parent retains read/wait/existing-result rights only, with no planning
mutation or launch grant. A coherent parent interruption may leave child reporting
eligible under the accepted independent cutoff, but a positive interrupted/resumed
parent source is not constructed here. New Main continuation requires the actual
eligible live parent and any independently required exact-identity resume first;
original authority, current child authority and Main authority are distinct.

## Report producer and immutable input effects

Use unchanged submitResearchReport/correctResearchReport actions, exact original
Stream, complete report/descriptor/normalized request/head/receipt/response shapes,
body/request domains, child author and shared eight-version limit. Normalize and
validate the complete request at its existing admission boundary; current child
capability/generation/runtime/run closure, rate and store/lease/health checks stay
before original receipt lookup. This proposal changes no admission ordering.

For a genuinely new acceptance, capture and revalidate actual live taskVersion,
head/history, run/input, collection/expectation, original cancellation and complete
source/consumer/resource/native/ordinary facts. expectedTaskVersion uses the live
TA control, never the frozen assignment or a synthetic default. expectedRunVersion
and expectedReportHead retain their existing child first/correction semantics;
internal capture still fences the current run even where a child field is null.

Derive the next immutable input from the captured **latest** manifest. Preserve
all existing configuration/scope/capture and other admitted records; insert the
first TA report source or replace only that exact selected stream's source.
Copy the original parent assessment unchanged and retain its distinct original
input/producer/proof. The reporting child is the new input event producer, not
the author of a fresh assessment. Advance preparation/input together once through
64; run.currentInput is still the sole current pointer.

The local report union inserts the new report/head/original receipt and input,
updates TA selection/expectationVersion and collectionVersion once, and advances
the sole research_run_controls.runVersion once. Report source history stays
immutable; report_heads remains the only authoritative current head. A/TA,
reservation, launch, attachment, ordinary bridge/runtime producer revision,
original lifecycle decisions/receipts and selector are reads, not report writes.
No extra increment represents cancellation, which changed no collecting role.

While stageContinueRequired is true, the report preserves the **entire** paused
state/gate/resume target, blockers, parent pointer, pending clarification value
and original pendingStageContinue ref. It neither continues the stage nor chooses
a planning state. Retain cancelled A, planReferences and all capacity/native
uncertainty. New input does not make the copied assessment current or permit
summary, clarification/proposal mutations or launches through the stage fence.

After successful continuation, a report can still select TA while the run is
assessing or blocked, with no intervening new assessment/proposal or broader
consumer. It preserves that run state, all remaining gates and the null stage
pointer. Null currentPlan does not erase TA's proven cancelled-stage ownership.
A fresh parent assessment/proposal winning first leaves this selected context:
revalidate and use that operation's complete owner contract before another new
report; never roll currentInput back or silently widen this branch.

No summary pointer exists in the selected examples, so the selector stays
byte-identical during reporting. Broader historical/current summaries and any
pending proposal, #744/#745/#746/detail/custody/cache owner require their actual
invalidation/fencing/retention effects or deny the whole new publication. They
cannot be inferred absent because A is cancelled or currentPlan is null.

## Explicit continuation consumes current evidence

Reuse the closed continue-stage request and lifecycle body/readSet/receipt from
[the accepted UI contract](2026-11-15-research-completion-design.md#original-completion-and-ui-request-codecs-proposed).
Its unchanged request includes run, expectedRunVersion, **current** input, a new
Main-issued decisionId, exact original cancellation ref, reviewContextId and
sidecarGeneration. The cancellation ref retains its original request/input/H/C;
it is never rewritten to match a later report input. Main must display and bind
the fresh current source context, not recycle the cancellation's old UI context.

For new admission require current Main authority/workspace/epoch/lease/health,
exact shown context and current versions/input; stageContinueRequired true and
pendingStageContinue equal the exact proven original cancellation; no terminal
run; and actual eligible live parent under its current restrictions. Resolve the
original cancellation outcome rather than deriving it from a paused flag. An
unanswered clarification cannot be normalized to null to fit this selected branch.

The new continuation outcome inserts its immutable decision/original receipt,
clears only stageContinueRequired and selector.pendingStageContinue, and advances
runVersion once for that complete union. With retained blockers/unsettled attempts
choose blocked; otherwise, for the proven no-clarification selection choose
assessing. Clear paused-only pauseReason/resumeTarget only when leaving paused.
Keep blockers/results/attempts, input and copied decision, plan/task/collection/
expectation versions, summary high-water and all original history unchanged.
Receipt.planVersion remains null; no cancelled-plan mutation is owned here.

The existing logical clarification branch would choose awaiting_clarification
when a supported unanswered source exists and no higher blocker remains. Its
source codec is still unclosed; this packet must deny that unsupported transition
as a whole, not clear the gate first. Actual parent unavailable/interrupted also
denies new continue until the independently authorized runtime path is satisfied.
Continuation cannot restore any cancelled grant or reuse a reservation/task;
reassessment and every future exact-plan approval remain separate operations.

## Original retries and race boundaries

| Ordering or retry | Required result |
| --- | --- |
| Report commits before a not-yet-accepted continue | Continue's old run/input/shown context fails current admission without writes. A fresh Main decision/context must bind the new input; the old cancellation ref stays exact |
| Continue commits before a captured report | Old internal run/state/gate capture fails revalidation. A newly authorized capture may publish in the selected post-continue context under the same unchanged request rules; never patch an occupied identity or report fields to hide drift |
| Exact accepted report retried after continue or later input movement | After current normalization/authority/cutoff/rate checks, resolve original normalized request/receipt/head-at-acceptance/proof/outcome and return the original closed response before live CAS; no selection, input, gate, version, event or charge change |
| Exact accepted continue retried after a later report | Fresh current Main lookup authority then exact original request/outcome returns original receipt before live CAS; its old input/runVersion is historical, never today's state or a command to clear another gate |
| Original cancellation retried after report/continue | Distinct Main-authorized exact original outcome; never reconstruct its request/snapshots with today's input or set the gate again |
| Run finish/cancel or child closure wins first | Preserve terminal absorption and child revocation before report receipt lookup. Main historical lookup remains distinct; no #744 claim-first or terminal UI correction exception is imported |

Original exact report retries may remain read-only at full input/report/counter
budgets while a new acceptance requiring another version denies. Changed bytes
at an original report/UI identity conflict. Missing/conflicting original key,
receipt/resulting versions/commit/direct proof or unknown outcome is unavailable;
no live reconstruction, case-label election, timeout rollback or replacement
worker follows. A continue retry does not authenticate a current child, and a
report retry does not authenticate Main or satisfy current planning admission.

The run/child/lease/native disposition must serialize with actual publication;
response suppression after an unqualified commit is insufficient. Ordinary
stop/end remains nonblocking and independent of hierarchy SQL. Unknown original
commit/native outcomes retain protected leases/worktree/capacity until their sole
owner proves disposition. No storage/native bridge implementation is selected.

## Complete consumers, resources and bounded offline handoff

The actual read union includes original root/bootstrap and owner admission; all A
source/approval/input/assessment/producer/custody/runtime/reservation/allocation/
attachment/ordinary controls; cancellation and any continuation with full original
snapshots/H/receipts/proofs; latest input/head/history/collection/expectations and
all source/Stream/role lookup generations; selector/history; actual parent/child
closure, native disposition, resource/lease/health and every current/historical
summary, preparation, tracked/detail/custody consumer and derived physical owner.
Original lifecycle snapshots remain historical input/report consumers after the
stage pointer clears. Zero-reference owners and full bidirectional membership
remain required; matching edges and one singleton are not completeness proofs.

The report and Main continuation are separate serialized commands, never one
caller-chosen batch or two independent run counters. Each complete owner outcome
coalesces its original-to-final versions/resources once and commits all required
source/index/custody/consumer/header/direct-proof effects under one C. Preserve
all existing report/control/source/preparation ownership and projections; copied
origins/H/producers/requests remain charged, retained history earns no refund.
No new field/tag/domain/family/profile identifier, ledger, completion trigger,
physical format or admission counter is introduced.

Keep 32 KiB report and normalized request bounds, 32 KiB/256-record inputs,
64 preparation/input and summary versions, eight shared report versions, existing
row/receipt/snapshot caps, 8 MiB/run and 64 MiB/workspace source/control pools,
32 MiB run aggregate and the smaller paired-job/ordinary ceilings. Complete
commands retain 4,096 changed rows/16 MiB; discovery separately retains its
4,096 observations/16 MiB plus 100,000 visited-row ceiling and all smaller worker,
physical/store/index/scratch limits. Necessary small component measurements prove
no complete command/job/store fit. Candidate 57/counts 28/25 remain conditional;
actual initializer/catalog/authority/ordinary/native/history qualification is open.

Accept or revise these **new choices for offline components only**:

1. Prove cancelled A's sole collecting role through its original approved source,
   Main cancellation and retained reservation, independently of null currentPlan;
   preserve the eligible original child and exact historical owners.
2. Apply first/corrected reports to the latest A-context input before or after
   explicit continue, preserving assessment provenance, cancelled plan/task/
   capacity and every stage/blocker gate without a selector rewrite.
3. Bind new continue to the current report/input/run and shown Main context while
   retaining the original cancellation ref; serialize races and exact historical
   retries under distinct unchanged authority with one run owner.

After acceptance, a finite independent Python/JSON/data-only JS packet may build
four alternatives: first/corrected TA report while stage-gated and first/corrected
TA report after explicit continuation. Construct cancellation and continuation
from their real source-before snapshots; do not relabel old post-rejection or
completion rows. Include blocked/no-blocker continuation, unavailable parent,
unsupported clarification, stale UI/internal captures, unchanged selector/plan/
task/bridge, exact report/cancel/continue retries, live versus frozen CAS, missing
original disposition, source/producer/history rewrites and version/width denials.
Use independent literal state/byte checks alongside generator/checker agreement.
Any report/decision component bytes are partial, not complete effects or proofs.

This is a concise documentary handoff, not an executable product plan. Scoped
acceptance precedes those components; complete source/authority/native/consumer/
resource/catalog/initializer/physical-history closure and separate activation
approval still precede a real publisher. Approved replacement handover, arbitrary
new assessments/proposals, resumed generations, earlier completed-summary contexts,
review/clarification/execution and terminal correction stay separate gates.
#1689/#1089 remain open; #740 remains closed research, Gate 2 **Investigate**.

## Bounded review disposition

Root is the sole writer. One bounded round used two fresh read-only Luna
contexts, capped at two reviewers: correctness against the cited accepted
contracts, and builder/completeness questions for the finite offline handoff.
Neither found a material issue or missing choice. Review ran no code/test/
runtime/native/database operation and supplied no actual authority or source
qualification. No additional local review round or dependent fixture follows.

All previous non-Markdown validation artifacts remain byte-identical; the
coverage audit is append-only and its earlier statements remain historical.
Verification and exact-head hosted review/CI evidence belong to this proposal's
PR; neither a clean review nor a merged proposal accepts its three choices.
