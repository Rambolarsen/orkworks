---
type: Design Proposal
title: Research reports after scope rejection
description: Proposed post-rejection collector and retained input source joins, first or successor report effects and typed summary invalidation without reopening plans or extending authority.
tags: [orkworks, research, reports, scope-revision, invalidation]
status: draft
workflow_status: proposed-for-scoped-offline-review
---

# Research reports after scope rejection

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `0034029f68347d57b0df495c7a01046bb77cfd25`.

This is a written proposal for the next boundary from the
[accepted prospective packet](../../validation/research-prospective-components.md#reproduction-and-remaining-gates).
Its new eligibility/source-effect choices require scoped owner acceptance before
finite offline components. Existing #1787/#1790 and earlier lifecycle/report
acceptance remain binding; no repeat approval of those choices is requested.
Gate 2 remains **Investigate**. No fixture, product writer, runtime/native/model/
database execution, physical format, migration or activation is supplied.

## Required behavior and missing source relation

The [accepted child report rules](2026-10-04-orchestrator-preparation-design.md#authenticated-report-submission)
allow a current live child to report while its stage plan is paused or complete,
provided the run remains active and its reporting capability is not revoked.
Plan completion revokes a launch grant, not that independent report right.
The [accepted summary invalidation rule](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union)
requires current input/selection and every affected summary/proposal consumer to
change atomically. Unknown effects make publication unavailable.

After rejecting B, run.pendingPlan is null but currentInput may still retain B's
configuration/scope, prior-plan A and stale original B assessment. Collection.task
still identifies reserved A. A can remain the current active/paused plan or be
selected as completed research after the original completion predicate succeeds.
The [prospective source table](2026-11-16-research-prospective-membership-design.md#rejection-ordering-and-original-outcomes)
explicitly does not qualify a positive post-resolution report writer. Neither a
missing pending pointer nor a cancelled status chooses the new input source.

Propose resolving that source from retained original records and outcomes, then
using the unchanged report protocol to update only the actual collecting stream.
Do not roll currentInput back to A, reactivate B, retarget A's reservation or make
report arrival undo a successful coordination result/plan completion.

## Selected post-rejection source and eligibility

One future complete recognized research profile must select the original owners
at admission and recognize these relations. No profile identifier, version switch,
row-presence adoption or source resolver fallback is selected here. The currently
unqualified product connection remains unqualified.

Resolve all of the following under one actual captured H, with complete source,
index and original-command correspondence rather than caller labels:

1. Current input K has the exact B configuration/scope/capture group and prior-plan
   A. Resolve its copied decision through the unique original fresh B assessment
   row and parent producer/runtime/proof. Follow its original input binding into
   the exact retained B source plan and full PlanRef; do not elect a maximum,
   select a matching task name or infer a plan from a digest alone.
2. B has proposed/unapproved original source, no reservation/child/launch, and a
   retained cancelled control. Its original **reject-scope-revision** decision,
   receipt and complete outcome bind that exact B and prior-plan A. Cancel-stage,
   run cancellation or an arbitrary cancelled proposal is not that disposition.
   Resolve all retained input/report advances since the original outcome under
   their own source proofs; the original UI request.input need not equal K.
3. The sole active collection selects A's original TaskKey; A's expectation is
   collecting with its original reservation/configuration/scope and exact selected
   report or null before first acceptance. K's zero/one A report occurrence must
   agree exactly with that selection and authoritative head/history; mismatch is
   unavailable source, not repaired by the next report. B's expectation stays non-collecting,
   reservation=null, selectedReport=null with its original version/history. Every
   other retired owner is retained and classified; no second collector or stream.
4. In the current-plan variant, run.currentPlan equals A and its recognized control
   is active/paused with its actual original result/turn state. In the completed
   variant, currentPlan is null, A's control is complete and the lifecycle selector
   selects A as completedResearchPlan. Resolve A's exact original completion and
   trigger/outcome under its existing contract, not a case label. It may be the
   compound rejection outcome or a separately closed later completion. Unknown
   completion/disposition or an active plan whose predicate should already have
   completed makes the slice unavailable; report arrival cannot repair it.
5. There is no newer selected pending plan or unclosed execution/review/clarification
   consumer. Complete original plan/source/control membership proves the selected
   no-required-review variant and absence of unsupported active consumers. Null
   pointers alone prove neither absence nor lack of retained historical owners.

B is resolved through exact source/current-input relationships and original
rejection proof; it is not newly selected as pending. The current input and all
historical B/assessment/report sources remain protected even though B cannot be
approved. Frozen A/B inputs, plan/task identities, configuration scopes, reservations
and original expected/frozen version pair never move.
This selected chain preserves B's original assessment through report-only input
advances. A later fresh reassessment or other source change is not silently mapped
back into that original B input; its post-resolution reporting relation needs its
own source/effect closure. Preserve it and deny this slice rather than calling
healthy new source bytes corrupt or revoking the child's logical reporting right.

These are new positive local eligibility choices, not closure of every possible
post-resolution state. Known current summaries are supported below. A new pending
proposal, actual execution, required review, material clarification or unknown
consumer effect is outside this slice and denies the whole new publication.
Preserve its data and existing logical rights; do not normalize it into this case.

## New report and immutable input effects

First apply the [unchanged acceptance/retry order](2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections).
Current capability, assignment/generation, admission epoch/owner/lease/health and
request-rate checks precede receipt lookup. Normalize only the unchanged bounded
request and sidecar-derived child author against its actual original assignment.
This selected writer is child-only. It selects no user-origin input producer or
terminal correction mapping; preserve user authorship and original history rather
than normalizing an unsupported user/correction source into a child event.
An exact authorized original receipt returns before live task/head CAS and before
new-publication role selection. A later pending plan, input or head must not turn
that read-only retry into fresh invalidation. Revoked/ended/resumed runtime or
terminal run authority still denies before lookup; no late child-write exception.

For a new request, require the actual live A taskVersion and exact authoritative
head CAS. Generation fields are original version provenance, not stream identity;
a qualified resumed child uses its current capability/history while earlier
versions retain their old runtime sources. This proposal proves no native resume.
If there is no head, prove the complete empty original history and accept only
version 1 with expectedReportHead=null. Otherwise retain reportId and accept only
head.version+1 through the existing eight-version ceiling. No gap, withdrawal,
branch, changed same-version request, head maximum election or new allowance.

Capture/revalidate K, all selected sources/roles above, original report/head/
receipt/history, lifecycle/summary and every consumer/resource/proof/range before
one original-to-final outcome:

| Owner | Required effects / preserved facts |
| --- | --- |
| A report registry | Retain immutable report, exact normalized original request/closed receipt and authoritative new head under existing domains/caps; never adopt another task's stream |
| Current input | Create next paired preparation/input revision from K, preserving B configuration/scope/captures/prior-plan A and copied original assessment unchanged; insert or replace only A's exact selected report source |
| Input producer and originals | Record the actual reporting child's current RuntimeRef as the new manifest event producer; retain B's older assessment row and its original parent producer/proof separately, plus all old input/report versions |
| A collection/expectation | Select exact new A report; advance expectationVersion and collectionVersion once for actual selection change; collection.task remains A and active/collecting stay true |
| Rejected B | Read/fence and retain original source/control/rejection/expectation; no new pending role, expectation version, reservation, report, approval or child |
| Current run | Change sole currentInput and checked runVersion once for the union; preserve current/completed plan selection, planning/runtime pause, blockers, stage-continue and other unchanged gates |
| Task/plan/runtime/ordinary | No taskVersion, coordination outcome, planVersion/status, launch grant, parent attachment, ordinary producerRevision, child end or capacity release follows report selection |
| Summary and tracked consumers | Apply the exact recognized invalidation below and every required owner-run/detail fence; no material/evaluation revision merely because a report exists |

The new record uses the existing research-report input variant. That source
resolves A's original assignment/configuration/scope, independently of B's
configuration/scope retained by K. B captures never become A report evidence.
The copied decision keeps its original decision.inputRevision and exact body;
the new child event is not a fresh assessment. Resolve that decision by the
original paired key and producer/proof, never a copied value or today's parent.
A fresh parent assessment and exact new proposal/approval remain necessary for
later launch work; neither B nor A's consumed grant is revived.

For the already-selected #1790 examples only, input 4→5, run 22→23, collection
5→6, A expectation 3→4 and head 1→2 illustrate this union. A task remains at its
actual 8 or 9 and B expectation at 1. Other starting versions use checked owner
values, not these defaults. First-report-after-rejection has its own exact null
selection/history prestate; do not claim it by retokening these correction bytes.

## Exact current-summary invalidation

Reuse the [accepted selector and immutable summary](2026-11-15-research-completion-design.md#immutable-bounded-synthesis-proposed).
Complete source-owned discovery includes every current and historical summary,
its original input/assessment/report/runtime/source references, zero-reference
owners and downstream consumers. A null currentSummary or summaryVersion zero
is not an inventory proof.

When currentSummary is nonnull, require the exact source-owned current summary
for this run and completed A, bound to K and the complete actual selected report
set before this publication. Its no-required-review variant needs original plan
membership proof. An inconsistent selector/digest/input/head is corruption or
unavailable source, not an opportunity to repair it by clearing the pointer.
A current summary of an unsupported plan/review/execution variant denies the slice.

The new input/head/selection makes that exact current binding stale. Clear only
selector.currentSummary, write its owned new lastCommit, and keep summaryVersion
at its high-water value. Retain completedResearchPlan and every other unchanged
selector field, immutable summary bytes/digest/version/history and all original
sources. Do not publish a replacement summary, reset its counter or refund its
retention. A null pointer has no invented selector write/version on this event.
Current-plan A has no summary of a not-yet-completed A; an incompatible selected
summary fails the complete source checks rather than being flattened into null.

Invalidate required source-run detail/current-summary snapshots before another
current read, using their actual existing owners/fences. Classify every #744–#746
historical/current consumer separately. No generic graph writer, fabricated
material value or new projection field is introduced. Unsupported dependent
proposals, execution effects or other owner-run updates make the whole publication
unavailable; a single local collection is not proof those sets are empty.

Later parent synthesis uses its separately authorized exact current-input/head/
collection/source checks and consumes the next summary version under the existing
64 ceiling. An original old-summary retry returns its historical original ref;
it cannot reselect the now-cleared pointer or assert currentness. A run event is
only a hint, not parent resume, model-call permission or execution approval.

## Ordering, retries and unknown outcomes

Serialize reports against current authority revocation, report publishers,
rejection/completion, summary publication, finish/cancel and every relevant source/
consumer change at the qualified publication boundary. All effects share original
H, one C, one original receipt and checked coalesced versions; no post-commit fixup.

- If a new summary wins first, a later report must discover and invalidate its
  exact current binding. If the report wins, a stale new summary publication
  fails its current input/head/run checks; an original summary retry stays historical.
- If finish/run cancellation wins, child authority is revoked before even report
  retry lookup. If report wins, a previously captured finish/input/summary context
  is stale; a new explicit finish must satisfy the existing valid-summary rules.
- A competing current-input, head, membership, source-health, runtime or header
  change denies the captured new report. Do not patch/rebase the request or
  retry silently against a newer B/A source. Rejection facts remain original.

No report mutation completes, uncompletes or reopens a plan/run, clears a blocker,
ends a child or frees a slot. Report-only acceptance cannot repair missing original
completion/receipt/source history. Unknown native/commit disposition holds its
original owner/protected leases/capacity; no timeout rollback, second worker,
receipt invention, maximum-based recovery or authority restoration. Ordinary
input/stop remains independent of hierarchy SQL/fsync and reconciliation locks.

## Resources, compatibility and bounded handoff

Keep the existing canonical/duplicate/unknown-field rules, request/input/report/
summary/receipt/head codecs and domains, all smaller row/job/worker/source and
content ceilings, shared retained pools and 32 MiB run aggregate. Inputs retain
32 KiB/256-record/revision-64 bounds; summaries retain 8 KiB/64 versions; report
requests retain 32 KiB/eight report versions. Any required next counter/version,
source copy or complete union exceeding a bound denies before publication.
An otherwise authorized exact original retry is read-only and consumes no new
input/source/summary version; rate and current authority still precede it.

Charge every actual new report/input/producer/retained source/index/proof occurrence
and original-to-final mutable control/resource delta through existing owners.
Retain B, old reports, old inputs and all summaries; no digest deduplication,
eviction, new ledger/allowance or invented initialization. Discovery and final
validation share existing cumulative 4,096 observations/probes and 16 MiB canonical
bytes, independently of 100,000 visited rows and smaller worker/physical limits.
Whole command 4,096 changed rows/16 MiB and lower paired/ordinary ceilings remain
separate. No maximum profile, complete accounting or actual budget fit is proved.

Zero new field/kind/family/counter or storage format is selected; candidate 57
and counts 28/25 remain conditional. A future complete profile must recognize
this source/effect eligibility before use. Existing child/direct/ordinary and
format-3/4/6 samples remain unchanged, not adopted into a profile by row presence.

Accept or revise three new **author choices for offline components only**:

1. Resolve rejected B from retained current-input/original-assessment/source and
   exact original rejection proof, with collecting A resolved through current or
   selected completed research ownership after pendingPlan is cleared.
2. Admit first or successor current-child A reports into that B-derived immutable
   preparation with preserved independent source/assessment provenance and only
   the stated report/input/selection/run changes; do not retarget or reopen tasks.
3. Include exact typed current-summary pointer invalidation, historical retention,
   retry/race/source-owner effects and complete-consumer denial under unchanged
   codecs, version owners and limits. Broader execution/review/clarification stays gated.

After acceptance, a bounded independent Python/JSON/data-only JS packet may compare
supplied post-rejection first/correction, active/completed A, null/current-summary,
original retry and stale/race/overflow alternatives with explicit source/auth/
native/complete membership/H/C/proof/outcome assumptions. It establishes neither
a legal prestate/history nor an actual publisher. Full catalog/index/initializer,
source/ordinary/authority/currentness/resource/native and physical proof, broader
consumers and separately approved product plans remain gates. No dependent fixture
is supplied now. Keep #1689/#1089 open and #740 closed research.

One writer; at most two fresh read-only Luna reviewers in one round check source
correctness and consequential builder questions before handoff. Verify existing
offline docs/OKF/site/rendered links, previous bytes, append-only routing and
close-out checks. Written review and merge do not accept these new choices.

## Static review disposition — 2026-11-16

One bounded round used two fresh read-only Luna reviewers. Correctness found no
material conflict with accepted report authority/retry ordering, retained B input,
current/completed A, summary high-water/history, counters or consumer-denial rules.
The builder found no consequential question before planning the bounded offline
packet after acceptance. The author made the report-only original-assessment
scope explicit; later fresh reassessment is not silently adopted into this branch.
Neither review establishes source joins, complete consumers, original proofs or
publisher behavior as product capability. No fixture/runtime/native/database
execution followed. Existing offline docs/OKF/rendered-link and previous-byte
verification is recorded with the proposal PR; merge alone is not acceptance.
