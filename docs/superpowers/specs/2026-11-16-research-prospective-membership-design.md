---
type: Design Proposal
title: Reserved research replacement membership and report invalidation
description: Proposed singleton collecting ownership, pending replacement membership and report-induced input invalidation while preserving original sources and approval gates.
tags: [orkworks, research, membership, scope-revision, source-contract]
status: draft
workflow_status: proposed-for-scoped-offline-review
---

# Reserved research replacement membership and report invalidation

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `2a527076e804725beb3cedab6af17fc7d6de3e16`.

This written proposal addresses the next boundary from the
[accepted scope-rejection packet](../../validation/research-scope-rejection-components.md#reproduction-and-qualification-handoff).
Its new choices require scoped owner acceptance before dependent offline
components. Existing lifecycle and scope-rejection acceptance stands; no repeat
approval is requested. Gate 2 remains **Investigate**. No product writer, runtime,
native/model/database execution, fixture, physical format or activation is supplied.

## Gap and selected boundary

The [accepted prefix](2026-11-12-research-input-control-prefix-design.md#positive-prospective-collection-ownership)
has one RunKey collection whose task is collecting. Its safe pre-reservation
replacement retires that unreserved expectation before changing the collection's
task. It explicitly excludes replacement of a reserved current task. A live child
can still report while another scope is proposed; neither its reservation nor its
historical report source can be retargeted to that proposal.

The [accepted rejection contract](2026-11-16-research-scope-rejection-design.md#selected-pending-proposal-and-immutable-input)
retains a current approved PlanRef A and an unapproved replacement PlanRef B,
with distinct tasks/configurations/worktree groups. Submission advances
run.currentInput to B's prepared input, but freezes both source plans. Rejection
must retire B's prospective relations without retiring A's report collection.
The current singleton relation cannot express that by selecting B as collecting.

Propose a narrow extension using the existing rows and source relationships.
Keep A as the only collecting task. B has its own non-collecting expectation,
classified as prospective through the exact run.pendingPlan and proposed plan
source/control. This is a new semantic membership choice, not an inference that
all collecting=false rows are already prospective or retired. It adds no field,
family, second RunKey collection, independent graph owner or mutable run counter.

Only this unapproved reserved-current-plan replacement is covered. Approval and
handover of collecting ownership to B, replacement of one pending proposal by
another, review/clarification/execution variants and terminal corrections remain
separate contracts. No legal history or actual publication is qualified here.

## Exact classification and source ownership

Resolve all complete rows and original sources at one captured H:

| Role | Required exact relationship | Expectation / collection consequence |
| --- | --- | --- |
| Current collecting task A | run.currentPlan resolves its approved current research plan and task; the collection.task equals A; A's original reservation/configuration/scope/stream resolve independently | Collection active=true, A collecting=true; preserve original reservation and selectedReport, including null before first report |
| Pending prospective task B | run.pendingPlan resolves B's proposed/unapproved control; its original input contains the exact prior-plan A; source task/configuration/scope and replaces join are those accepted in #1778 | B expectation uses the same collection RunKey, collecting=false, reservation=null, selectedReport=null; B task remains pending/version 1 |
| Retained rejected task | Its original rejection/outcome and cancelled proposal control prove disposition; it is not the current pending PlanRef | Retain its non-collecting expectation and complete original source/proof membership; do not recycle it |

A boolean, task name, same run ID or row presence cannot choose a role. A proposed
control not selected by the exact pending pointer is not silently classified as
retired. Unknown/duplicate/conflicting role or missing original disposition makes
this slice unavailable. Both current and pending plan inputs retain their own
original producer/custody proofs. Nullable fields stay null; no future reservation,
stream, selected report, approval, assignment or ordinary child is fabricated for B.

The common prefix's rule equating collecting-task configuration/scope with the
current prepared decision is explicitly amended **only** for this extension.
Collecting A resolves its frozen approved source, while currentInput may describe
B's configuration/scope. The run pointer remains the only current-input pointer;
the collection gains no second input field. Frozen inputs, reservations, task
versions and identities never change to make the two scopes appear equal.

Submission atomically publishes B's retained source/input/producer, proposed plan
and pending task controls, B's expectation, run.currentInput/pendingPlan and the
ordered PlanRef occurrence under their original owners. B expectationVersion
starts at 1. A's expectation is byte-identical. Because the collection's owned
prospective membership changes, collectionVersion advances once although its
task/active fields do not change; runVersion advances once for the union.
All ledger/index/proof effects must be included; nulls or absent heads prove none
of them. No positive command or membership proof is supplied by this description.

## Current reports and immutable input invalidation

Propose supporting the exact collecting A report as the sole additional local
invalidation source while B is pending. Authenticate/report-admit under the
unchanged child generation, cutoff, live CAS, head, byte/rate/custody and original
receipt rules. A pending proposal never grants B or the parent A's reporting
capability. Old reports retain their original assignment/runtime/scope sources.

On a new accepted A report, retain the report/head/original receipt and update
A's selectedReport through its own owner. If that changes the current preparation,
create one new immutable manifest from the exact captured run.currentInput:

- Preserve its configuration, scope, captures and exact prior-plan A record.
- Replace the selected A report occurrence with the new exact accepted report
  evidence source, or insert it on first selection. This source resolves A's
  original configuration/scope/assignment, not B's. B's captures do not become
  evidence for A's report. Full original source-group membership remains required.
- Copy the original decision unchanged, including its older decision.inputRevision,
  and retain the original parent assessment row/producer/proof. The reporting
  child is the new manifest's event producer, not the author of that assessment.
- Advance preparation/input revisions together once through their existing 64
  ceiling and update run.currentInput. Do not edit B's frozen input/plan bytes.

This explicitly extends the prefix's selected-report derivation across distinct
current/prospective scopes; it does not admit arbitrary extra reports. The new
manifest retains at most the one collecting A selected source. Its original
source still uses the existing accepted research-report input variant and #744
inline evidence-source shape, with no fabricated standalone evidence object.
A second collecting task or unsupported affected preparation denies the slice.

B remains selected as pending and its control remains proposed at the same
planVersion. Its frozen input differs from the new currentInput, so it cannot be
approved. Invalidation is derived from exact input identity/freshness; no new
stale flag, automatic cancellation receipt, substituted source plan or launch
grant is introduced. Existing UI must explain stale approval eligibility through
its future qualified projection; this proposal supplies no renderer behavior.

A fresh parent reassessment is required before any fresh launch proposal. It
cannot mutate B or revive its identity. Reject B through its original Main path
before a later distinct submission under the accepted complete submission order;
arbitrary pending-to-pending replacement is outside this extension.

Report-induced manifest changes do not change coordination results, current or
pending task versions, original reservations, ordinary bridge versions or child
capacity. Advance A expectationVersion and collectionVersion once for actual
selection/membership changes and runVersion once for the complete report union.
Exact accepted report retries select the original response without new inputs,
versions, sources, producers or grants. A report alone does not complete a plan.

## Rejection, ordering and original outcomes

The accepted Main rejection binds today's exact run.currentInput and expected
versions. It still resolves B's **original frozen** input and exact prior-plan A
relationship. After report invalidation, their differing input identities are
expected; they do not mean B's retained source is corrupt. This proposal makes
that rejection eligibility explicit. Actual source/custody/proof invalidity or a
stale captured current-input pointer still denies. Stale B input denies approval,
not an otherwise authorized explicit rejection of that exact displayed proposal.

Rejection clears only B's pending selection and prospective membership role.
Keep B expectation collecting=false with its null reservation/report and original
sources; do not delete it or rewrite it as A. B's cancelled control and original
outcome establish retained historical disposition. B expectationVersion does not
advance when its body is unchanged. CollectionVersion advances once for removal
of the owned prospective relation. A's task/expectation/selected report remain.
The accepted rejection and optional automatic completion still share one H/C,
sole run advance and original outcome/retry proof. No additional receipt field
or completion trigger is introduced.

Report and rejection publish against one exact captured owner/input/membership
read set. If a new report wins, a previously captured Main rejection is stale;
a fresh shown context may reject B using the new input. If rejection wins, a
later new report must discover the resulting membership and use its separately
qualified effect owner. This document does not close a positive post-rejection
or post-completion report writer: currentInput still retains B-derived sources
and collection/report rights remain intact. Missing broader invalidation effects
make publication unavailable; they do not revoke the child, retire A or justify
retargeting its sources. An original identical report or UI retry remains the
original outcome after current authority checks, never a fresh invalidation or
completion attempt. Missing paired/original proofs make retry unavailable.

Neither branch kills a child, releases capacity, changes a launch grant by
report selection, restores a bearer, rolls back currentInput or repairs a maximum
revision. Unknown native/publication disposition retains its original ownership
and independent stop/input availability. The accepted higher blocker, clarification,
stage-continue and pause precedence remains; unsupported state variants deny.
Approval/handover races require that separate complete writer and its existing
prior-child result/terminal rules, not an inferred transfer in this proposal.

## Discovery, accounting and compatibility

The [accepted membership discipline](2026-11-12-research-membership-publication-design.md#complete-discovery-and-the-positive-first-report-slice)
requires complete authoritative consumers, not an affected list supplied by a
caller. Extend its bounded local slice to this one exact B prospective consumer:

- Derive A from current plan, collection and original task/reservation/configuration;
  derive B from pending plan, frozen input/prior-plan/source and its expectation.
- Discover all expectations by RunKey, including prospective and retained rejected
  rows; the collecting subset remains exactly A. Resolve stream membership only
  for original bound reservations. B has no stream.
- Retain both directions for every input/configuration/scope/capture/prior-plan/
  report and producer/runtime/source/proof occurrence, including zero-reference,
  historical and cancelled owners. Both current and frozen input reads count.
- Classify actual #744–#746 consumers and invalidation/snapshot effects. Any other
  unsupported prospective consumer or unproven complete owner/index relation
  denies the entire publication; matching tuples or absent heads cannot repair it.

Documentary responsibilities remain with the proposed research input/control,
membership and publication owners named in the coverage audit. This adds logical
recipes, not deployed source modules, physical indexes or a generic graph writer.
All admission and final revalidation shares the existing cumulative 4,096 logical
observations/probes and 16 MiB canonical discovery bytes, independently of every
smaller worker/deadline/content/command/native/physical limit. No split/truncation.

Charge new retained source/input/producer occurrences and controls to their existing
shared pools. Retain B and every rejected predecessor; no refund, purge, deduped
body, new zero ledger or second allowance follows invalidation/rejection. Mutable
replacements use full original-to-final deltas once; all real owner/index/proof
occurrences and summary/tracking effects remain accounted independently. Preserve
8 MiB/run and 64 MiB/workspace shared control/source boundaries, 32 MiB run aggregate,
32 KiB/256-record inputs, all existing row/receipt limits, 4,096 changed rows/16 MiB
command and the smaller applicable ordinary/job ceiling. Any overflow denies.
A finite pair of inputs proves neither a maximum profile nor whole-command fit.

No envelope/schema version, source/request/receipt domain, family name, physical
format, header, manifest, row/change hash or migration is selected. Candidate 57
and selector counts 28/25 remain conditional. The unchanged field shapes do not
make earlier readers eligible: a future complete profile must explicitly recognize
this extended membership/input meaning. Old prefix/component bytes remain unchanged;
they are not silently widened into a reserved-replacement history.

## Choices and bounded handoff

Accept or revise these **author choices for offline components only**:

1. Derive one pending non-collecting prospective B expectation beside the sole
   collecting reserved A task, through exact existing run/plan/source roles.
   Membership changes advance the collection version even when its task stays A.
2. Admit only A's selected report into the current B-scoped immutable preparation,
   retaining distinct source scopes and copied stale assessment provenance.
   Preserve B as pending/unapprovable; explicit Main rejection remains eligible
   against the fresh current input and original B source.
3. Close the stated compound report/rejection role, version, discovery and
   accounting effects without new codecs/families/counters or approval handover.

After scoped acceptance, finite independent Python/JSON and data-only JavaScript
components may compare supplied A/B/prospective/rejected alternatives, report-input
invalidation, original retry and stale/overflow denials. Explicitly label supplied
complete membership/auth/native/H/C/outcome facts. They cannot establish original
creation authority, complete source/index/currentness coverage, a physical catalog,
initializer, legal event history, qualified publisher, runtime or activation.
Those gates and broader review/clarification/execution remain open. Keep #1689 and
#1089 open and #740 closed research. No dependent fixture is included here.

One writer owns this document. One bounded round of at most two read-only Luna
reviewers checks source correctness and consequential builder gaps before handoff.
Verification covers existing offline docs/OKF tests, rendered links, unchanged prior
artifact bytes and append-only routing. It grants no acceptance or producer proof.

## Static review disposition — 2026-11-16

One bounded round used two fresh read-only Luna reviewers. Correctness found no
actionable contradiction in the proposed roles, distinct report/input source
provenance, immutable assessment history or rejection/version rules. The builder
asked whether prospective membership should itself advance collectionVersion
while collection.task stays A. That is the explicit first author choice, not an
unanswered encoding detail; it remains pending scoped owner acceptance. No other
consequential builder question was identified. Neither review establishes source
membership completeness, legal history or publisher behavior. No new fixture or
runtime work followed review. Final docs/OKF/rendered-link and preserved-byte
checks are recorded with the proposal PR; existing technical evidence is unchanged.

## Rejection freshness and profile eligibility clarification — 2026-11-16

The [scope-rejection routing appendix](2026-11-16-research-scope-rejection-design.md#canonical-prospective-membership-proposal-—-2026-11-16)
states the proposed stale-input exception against that contract's earlier
normative denial explicitly. Existing acceptance remains binding until these
new choices are accepted. Only B's original input becoming stale-for-approval
through the selected A report/input advancement is allowed in this new rejection
branch; the actual Main request.input/context/generation/versions must be current,
and B's original source integrity, custody and proofs must still validate.
This distinguishes rejection of an exact invalidated proposal from approval of
stale bytes or acceptance of a stale UI request. Report arrival leaves B pending
and the completion condition unresolved; exact authorized rejection owns its
resolution and any compound completion.

The unchanged field shapes do not authorize a profile switch or adoption of old
prefix fixtures. A future complete profile must select original source/run
ownership at admission and explicitly recognize these relations before they
are used, under the [catalog and owner-selection gate](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required).
No replacement profile identifier, second run owner or delayed initializer is
selected here. CollectionVersion advances for the owned prospective-role changes
as proposed above; an unchanged B expectation does not receive a made-up version.

After rejection or conditional completion, currentInput can still retain B-derived
configuration/scope and stale assessment provenance although B is no longer
pending. Preserve the retained source/input/producer/PlanRef joins. Do not roll
the pointer back, reactivate B, force its scope to equal A's, or reuse the
current/pending classification table as a positive post-resolution report writer.
That publisher and its complete invalidation/collection owner remain explicitly
unclosed. Existing reporting rights and historical membership are preserved;
new publication is unavailable when its required owner/effect union is unsupported.

## Scoped owner acceptance and offline packet — 2026-11-16

The owner answered “keep going” immediately after the explicit request to accept
all three #1787 choices for offline checks only. This records that scoped acceptance
and supersedes pending-choice wording only at that scope. The technical proposal
and historical review remain unchanged. The alternative held draft #1786 is not
adopted, edited or closed by this acceptance.

The [finite component packet](../../validation/research-prospective-components.md)
compares current collecting A and prospective/rejected B, cross-scope report input
invalidation with original assessment provenance, fresh Main rejection and original
retry/measurement denials. Auth/native/source/currentness/complete membership/H/C/
proof/outcome inputs remain supplied. No source-valid publisher, legal history,
physical profile or product/runtime permission follows. Post-rejection/completion
report writers and approval handover remain separately unqualified.

The merged #1786 freshness/profile clarification remains binding. This acceptance
covers finite offline components only; it does not select or qualify a complete
recognized research profile or turn stale UI requests into eligible rejection.
