---
type: Design Proposal
title: Research material clarification and changed answers
description: Proposed closed clarification decision/request/answer records, Main-authorized user provenance, atomic input/state effects and gate precedence under unchanged authority.
tags: [orkworks, research, clarification, answers, sources, qualification]
status: draft
workflow_status: proposed-for-scoped-clarification-review
---

# Research material clarification and changed answers

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumers
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `ac522955c55ac97c1ed07e85f057edbb60a4a5a9` (origin/main,
including the review-task proposal merged in
[#1908](https://github.com/Rambolarsen/orkworks/pull/1908)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names material clarification and changed answers as an unclosed source
contract. This proposal closes that written boundary only: the clarification
decision variant, immutable request and answer records, Main-authorized user
provenance, the atomic answer→input/state effect union, and
runtime-pause/stage-gate precedence and historical retention. It makes new
record/action/tool and accounting choices explicit; they are not acceptance
inferred from a merge. Clarification UI wording (#746), transport, adapters,
runtime, complete catalog/initializer and executable plans remain separate
#1689 work. No dependent fixture, Rust/Electron module, storage format,
migration, probe or runtime execution is supplied. Gate 2 remains
**Investigate**.

## Outcome and preserved decisions

Make the accepted clarification lifecycle representable in the selected fresh
research profile: an authenticated parent assessment may ask one bounded
material question, the run enters `awaiting_clarification` with an exact
pending request binding, only an explicit Main-authorized user answer resolves
it, and the accepted answer atomically publishes a new prepared input revision
carrying a fresh parent assessment — so dependent approval stays blocked while
a question is unanswered and independent already-approved research continues.

Preserve unchanged: the accepted
[source grammar](2026-11-12-research-source-codecs-design.md) and per-task
chain records; the [input/control prefix](2026-11-12-research-input-control-prefix-design.md)
including `ResearchPreparedInputV1`'s codec, the sole run owner and prepared
input revision pairing; the
[turn/coordination/recovery codecs](2026-11-15-research-continuation-design.md);
the [completion/lifecycle records](2026-11-15-research-completion-design.md)
including stage-cancel/continue/finish fencing and summary invalidation; the
accepted [scope-rejection](2026-11-16-research-scope-rejection-design.md),
[prospective-membership](2026-11-16-research-prospective-membership-design.md),
[post-rejection](2026-11-16-research-post-rejection-report-design.md),
[stage-report](2026-11-16-research-stage-report-design.md) and
[review-task](2026-11-18-research-review-tasks-design.md) contracts; #742
report/request/head/receipt rules; #744 evaluation/artifact/debit domains; and
the Main approval/authority seams.

Rejected alternatives: normalizing a material question into
`clarificationNeeded:false`; deriving a user answer from terminal text, a model
guess, elapsed time or a preselected option; trusting renderer-supplied
provenance or the parent's run bearer as user evidence; rewriting an answered
request; a second runVersion owner; and silently widening the goal or parent
bootstrap permissions through an answer.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Material clarification lifecycle](2026-10-04-orchestrator-preparation-design.md#material-clarification-lifecycle) | `ClarificationRequest` identifies run, preparation revision, question ID, one material question, why the answer matters and up to four suggested answers; free-text answers are always allowed; only an explicit user answer through the UI resolves it |
| [Answer admission](2026-10-04-orchestrator-preparation-design.md#material-clarification-lifecycle) | Stale answers reject with the current request; duplicate identical answer IDs return the stored result; conflicting replays fail; changed answers create a new preparation revision; the sidecar derives user provenance from the authorized operation, never renderer fields or the run bearer |
| [Assessment and skip criteria](2026-10-04-orchestrator-preparation-design.md#clarification-and-research-lifecycle) | Missing material requirements trigger clarification; record the decision with its input revision, `clarificationNeeded`, reasons and identified unknowns; skip clarification only when supplied evidence suffices |
| [State table](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table) | `awaiting_clarification` presents the question and retains independent approved work; accepted answers return to `assessing` only when the runtime is active and no retained blocker/stage gate remains; resume restores the same pending request identity/version and preparation binding, never a duplicate or replacement |
| [Precedence](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table) | Stage continuation outranks blockers, then pending clarification; stage-cancel fencing rejects clarification/proposal mutations |
| [Cursor events](2026-10-04-orchestrator-preparation-design.md#parent-planning-and-event-capability) | A clarification answer is a parent-relevant UI mutation that advances the run version and publishes a cursor event carrying only bounded type/record references |
| [Prefix controls](2026-11-12-research-input-control-prefix-design.md#proposed-current-control-records-and-sole-version-ownership) | `pendingClarification` stays null in that prefix and an actual clarification record is unavailable there, never discarded; `awaiting_clarification` was not among its selected state values — giving it a positive writer is a new choice |
| [Stage-report continuation](2026-11-16-research-stage-report-design.md#explicit-continuation-consumes-current-evidence) | An unanswered clarification cannot be normalized to null to fit the no-clarification branch; the logical clarification branch chooses `awaiting_clarification` when a supported unanswered source exists and its source codec is still unclosed |
| [Membership recipes](2026-11-12-research-membership-publication-design.md#research-metadata-reference-recipes) | Every new owner needs exact key/body/domain, both lookup directions and complete discovery; unsupported prospective consumers deny rather than count as zero |
| [Goal boundary](2026-10-04-orchestrator-preparation-design.md#material-clarification-lifecycle) | Clarification refines the UI-authorized run goal; it cannot broaden the parent bootstrap's permissions or immutable goal boundary; a genuinely new goal/profile needs a separate UI creation/relaunch decision |

These are bounded inspection facts at the inspection base, not an inventory of
all writers. Use the proposed research source/control/authority/publication,
SessionApplication bridge and Main approval seams from the
[prefix owner table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
No new module is created; type names are not factories or an existing runtime.

## Selected clarification slice

One clarification assessment asks **exactly one** material question per
prepared input revision, matching the lifecycle's preference for one concise
question at a time. After an accepted answer, the fresh parent assessment may
ask the next question in a new revision; unrelated questions queue through
successive revisions, never through one oversized decision. Independent
already-approved research and live children continue; clarification is parent
planning state and pauses no child, collection or capacity. Multiple parallel
clarification streams, anonymous/external answer channels, model-selected
answers, automatic timeout dispositions and clarification-driven profile or
goal changes are unsupported by this slice.

## Clarification decision and request records

`ResearchPreparedInputV1`'s decision field becomes this closed union:

```text
decision: ResearchDecisionV1 | ResearchClarificationDecisionV1
```

`ResearchClarificationDecisionV1` is exactly:

```text
{schemaVersion:1,inputRevision,clarificationNeeded:true,researchNeeded:false,
 reasons:Text[],question:ResearchClarificationQuestionV1}
```

`inputRevision` equals the containing manifest's `inputRevision` — a fresh
authenticated assessment, never a stale copy. `reasons` is 1..4 nonempty
well-formed UTF-8 strings through 2 KiB each. The question is exactly:

```text
{questionId,questionText,whyItMatters,suggestions:Text[]}
```

`questionText` is nonempty UTF-8 through 2 KiB; `whyItMatters` is nonempty
UTF-8 through 1 KiB; `suggestions` is 0..4 unique nonempty UTF-8 strings
through 512 bytes each. Suggestions are display hints only; free-text answers
are always allowed and never validated against them. `questionId` is an ID,
unique across the run and never recycled after conflict or completion.
`ResearchDecisionV1` itself is unchanged: its accepted fixed
`clarificationNeeded:false/researchNeeded:true` pair stays exactly as the
scope-rejection contract fixes it, and no material-clarification content is
normalized into it — the union adds a sibling variant only. The accepted
decision content element "identified unknowns" is represented by the asked
question itself: `questionText` states the unknown, `whyItMatters` states its
materiality, and when research later follows, the research decision's scope
uncertainty fields carry the resolved unknowns. A
clarification manifest contains no research-scope, capture or report records
(none are required by this slice); it may contain the configuration record
when one exists. The unchanged input domain, 32 KiB body bound, paired
revision rule, exact-key historical lookup and stale-assessment rules apply:
an assessment is current only at its own revision with its original parent
producer row.

The request row is published with the clarification manifest, atomically, in
the same outcome. `ResearchClarificationRequestV1` is exactly:

```text
{schemaVersion:1,run:RunKey,inputRevision,
 question:ResearchClarificationQuestionV1,producer:RuntimeRef,requestDigest}
```

Its digest uses `orkworks.research-clarification-request.v1` + LF over the
canonical body excluding only `requestDigest`. `question` is byte-equal to the
containing decision's question entry. `producer` is the exact authenticated
parent runtime that assessed, resolved like the prepared-input producer rule —
never a model-selected or fabricated identity. `inputRevision` binds the
manifest that asked. Immutable; one request per
`{run,questionId,questionVersion}` where `questionVersion` starts at 1 and a
re-asked questionId under a later assessment publishes the next version, never
a rewrite.

Propose two new immutable logical families in the eventual single fresh
catalog (no physical table, layout or format is selected):

| Family / exact key | Complete value | Wrapper ceiling |
| --- | --- | ---: |
| research_clarification_requests / `{run:RunKey,questionId,questionVersion}` | `{schemaVersion:1,body:ResearchClarificationRequestV1,digest,lastCommit}` | 16 KiB |
| research_clarification_answers / `{run:RunKey,questionId,questionVersion,answerId}` | `{schemaVersion:1,request:ResearchClarificationRef,answer:ResearchClarificationAnswerV1,receipt:ResearchClarificationReceiptV1,lastCommit}` | 32 KiB |

`ResearchClarificationRef` is exactly
`{run:RunKey,questionId,questionVersion,requestDigest}`. Projections exclude
only outer `lastCommit` and, on the answers family, `receipt.commit`; every
excluded link resolves its own original complete proof.

## Main-authorized answer admission

The proposed exact UI request is:

```text
{schemaVersion:1,action:"orkworks.research-clarification.answer.v1",
 run:RunKey,questionId,questionVersion,answerId,answerText,
 reviewContextId,sidecarGeneration,expectedInputRevision}
```

This names a new proposed logical Main-authorized operation, not a renderer
token or a change to existing approval actions. `answerId` is Main/sidecar-
issued nonsecret ID, unique across the run, never recycled after conflict, and
never supplied as a trusted identity by a renderer or model. `answerText` is
nonempty UTF-8 through 2 KiB; free text is the only answer form and a
preselected suggestion is just its text, if the user chooses it.
`reviewContextId` and `sidecarGeneration` are Digests binding the exact shown
answer context and current source generation; they record provenance and
cannot authenticate themselves. `expectedInputRevision` pins the current input
at submission — deliberately the lifecycle's own validation triple (current
run, preparation revision, clarification request version) expressed against
this profile's paired input revisions, rather than the lifecycle-request's
`expectedRunVersion`; run currency is validated through the pending binding
and the current-input equality below.

`ResearchClarificationAnswerV1` is exactly:

```text
{schemaVersion:1,run:RunKey,questionId,questionVersion,answerId,answerText,
 requestDigest,origin:ResearchUserActionOriginV1,answerDigest}
```

`requestDigest` resolves the exact answered request row. `origin` records the
user-origin answer provenance as a closed Main-authorization tuple — the same
provenance pattern the terminal-correction authority protocol uses for
Main-authorized user facts:

```text
ResearchUserActionOriginV1 =
 {policy:"research-clarification-answer-v1",reviewContextId:Digest,
  sidecarGeneration:Digest}
```

It is derived by the sidecar from the Electron-main-authorized operation —
never a renderer-supplied field, never the parent's run bearer, and never a
parent or child runtime cast into user provenance: the accepted RuntimeKey
union is closed to `kind` parent/child and cannot hold user provenance, which
is why this proposal defines a separate closed origin type instead of
extending that union. `sidecarGeneration` is the admitting Main UI owner
lifetime; it need not equal any runtime lineage and is validated against its
own owner. The [prefix input rules](2026-11-12-research-input-control-prefix-design.md#proposed-immutable-research-input)
already anticipate "the separate qualified user-correction origin" as a
distinct producer when that path closes; this proposal supplies its concrete
codec and explicitly revises the input row's producer typing below. The
digest uses `orkworks.research-clarification-answer.v1` + LF over the
canonical body excluding only `answerDigest`. The receipt is deliberately
digest-free: its integrity rests on its `commit:CommitRef` and the owning
command's original proof, matching the accepted coordination-receipt
precedent. `ResearchClarificationReceiptV1` is exactly
`{schemaVersion:1,run:RunKey,questionId,questionVersion,answerId,answerDigest,
runVersion,inputRevision,commit:CommitRef}` at most 4 KiB; `runVersion` is the
actual coalesced poststate version and `inputRevision` the newly published one.

Admission order mirrors the accepted Main UI rules: current Main
authority/workspace/epoch/owner/lease/health — including the exact
shown-context/`reviewContextId`/`sidecarGeneration` binding check, placed
with that authority validation before original retry lookup, matching the
accepted lifecycle/UI decision admission order — and the existing
ten-parent-mutation requests-per-minute discipline (duplicates count)
precede original retry lookup. Then, for a new answer:

- The exact request row exists, is immutable, and is the **current unanswered
  pending request**: the run control's `pendingClarification` binding equals
  this `{questionId,questionVersion}` at its original digest, and the current
  input revision equals `expectedInputRevision`. A stale answer — against a
  superseded request version, a superseded input, or an already-resolved
  request — rejects with the current request state and consumes no version.
- `stageContinueRequired` is false. While the stage fence is set,
  clarification mutations are rejected exactly as the accepted
  stage-cancel fencing requires; the stage continue decision comes first.
- The run is not terminal. A terminal run's clarification gate is inoperative;
  it is never permission to answer, and `finish`/`cancel` explicitly retain
  every raw clarification value.
- The run control's `pendingClarification` field type becomes the closed
  binding `{questionId,questionVersion,requestDigest}` when nonnull — an
  exact pointer, not a loose reference; a missing, inconsistent or
  digest-mismatched pointer is unavailable.

Identical accepted retry (same `answerId` and byte-identical request) returns
the original receipt before live-version CAS with no mutation, version, event
or charge, even after the run later completes. Changed bytes for the same
`answerId` conflict. An unavailable current authority/lease/store cannot be
repaired by a stored receipt. No active UI token is persisted or sent to
renderer/model.

## Atomic answer effects

The answer outcome follows the accepted event-induced manifest pattern: an
event (here, the answer) publishes the next input revision with the **copied
unchanged decision**; a fresh authenticated parent assessment is always a
separate, later revision. One accepted answer commits, in one complete
outcome, exactly this local union plus every actual
source/index/resource/proof effect from its complete owners:

| Owner | Required change / preserved facts |
| --- | --- |
| Answer family | Insert the immutable answer row with its original request ref, body, digest and receipt |
| Prepared input | Publish the next immutable input revision (`inputRevision+1`, paired `preparationRevision`) whose records are the answered manifest's records plus exactly one `{kind:"clarification-answer",reference:ResearchClarificationAnswerRef}` entry resolving the accepted answer by full key/digest, and whose decision is the answered manifest's decision **copied byte-unchanged** (possibly the clarification decision itself). The manifest's producer field, explicitly revised for this profile, is the closed union `RuntimeRef \| ResearchUserActionOriginV1`: the answer-induced manifest records the user origin; a fresh parent assessment manifest records the current parent runtime |
| Current run | Clear `pendingClarification` to null — the next pending binding is set only by a later fresh clarification assessment outcome, never within the answer outcome — and advance the sole `runVersion` once |
| Run state | Set `assessing` only when the actual parent runtime is active/eligible and no retained blocker or stage-continue gate remains; otherwise retain the exact gated state — one of `blocked`, `paused` (any pauseReason) or `awaiting_resume_approval` — with the clarification gate cleared. Never clear pause fields unless actually leaving paused. When a run pauses while its current state is `awaiting_clarification`, its `resumeTarget` is `assessing` (the closed resume-target vocabulary never contains `awaiting_clarification` itself) |
| Consumers | Invalidate every recognized pending proposal and current summary whose binding becomes stale under the accepted source-owned invalidation, retaining immutable history; no task outcome, report, head, grant, capacity release or plan mutation follows from an answer |
| Events | Publish the cursor event for the input revision carrying only bounded type/record references |

The **fresh assessment is a separate outcome**. A paused or gated parent
never produces it inside the answer outcome: the answer manifest carries the
copied decision, and only an active, eligible parent — after exact-identity
resume where required — publishes the next revision with its own fresh
decision (`decision.inputRevision` equal to that revision, producer the
current parent runtime) deciding research need, skip, or the next
clarification question. When the parent runtime does not exist
(`pauseReason=parent_ended`) or the run is otherwise gated, no fresh
assessment occurs at all: exact-identity resume first establishes the fresh
parent runtime under its own accepted rules, and the resumed parent's
assessment uses that new runtime. That next outcome owns its own effect
union below. This preserves the accepted report precedent byte-for-byte:
report-induced manifests copy the stale decision, and only the parent's own
reassessment is current. A fresh assessment manifest's record set is the
**retained applicable clarification-answer entries plus the records derived
by its own decision**: every clarification-answer record entry from the prior
revision chain that the fresh assessment uses stays bound in the new manifest
(sorted under the unchanged record rules), followed by the decision-derived
records — a fresh research assessment publishes the configuration record
plus its newly prepared scope and capture records exactly as the accepted
preparation rule derives them; a fresh clarification assessment publishes
the configuration record set defined above with no research records. The
lifecycle requires proposal input revisions to bind the request/answer
records they use, so a fresh assessment never silently drops the answers
that informed it; earlier answers stay bound until a fresh assessment
supersedes them, and the closed record union plus the unchanged 32 KiB input
bound deny — rather than truncate — a chain that no longer fits. Records are
never "gained" across decision variants beyond the retained answer entries:
each fresh manifest derives its non-answer records from its own decision.

`ResearchClarificationAnswerRef` is exactly
`{run:RunKey,questionId,questionVersion,answerId,answerDigest}`. Revision-64
exhaustion denies a new answer requiring input-65 atomically; an otherwise
authorized exact accepted retry remains read-only. Changed-answer semantics
are preserved: a user may change a prior answer only through a later
re-assessment's new request version (a re-asked `questionId` at
`questionVersion+1`), never by overwriting or re-answering a resolved request
version.

### Post-answer revision chain example

For the packet's required example, with the answered manifest being the
first assessment in this profile: input-2 is the clarification manifest
(decision `ResearchClarificationDecisionV1`; records exactly the
configuration record when the run's bootstrap binds one, otherwise empty for
this slice); the answer publishes input-3 with the same record set plus the
clarification-answer record, the byte-identical copied decision and the user
origin as producer; the active parent then publishes input-4 with its fresh
decision and the **retained clarification-answer entry** — a
`ResearchDecisionV1` whose records additionally include the newly prepared
scope/captures when research is needed, the next
`ResearchClarificationDecisionV1` at `questionVersion+1`, or another
supported assessment variant. Each revision is immutable and independently
keyed; no revision is patched.

## Asking-side assessment effects

The asking side is a parent planning mutation with its own admission and
effect union. Admission: current authenticated parent planning authority
(bearer/scope/generation under the existing rules) and the existing parent
mutation rate discipline precede lookup; while `stageContinueRequired` is
true the stage fence rejects it exactly as it rejects proposal mutations.
One new clarification assessment commits, in one complete outcome:

| Owner | Required change / preserved facts |
| --- | --- |
| Prepared input | Publish the next immutable input revision with the closed `ResearchClarificationDecisionV1` (fresh assessment, `decision.inputRevision` = this revision) and its record set: exactly the configuration record when the run's bootstrap binds one, otherwise empty; no research-scope, capture or report records |
| Request family | Insert the immutable request row for the asked question, byte-equal to the decision's question entry, atomically in the same outcome |
| Current run | Set `pendingClarification` to the exact `{questionId,questionVersion,requestDigest}` binding and advance the sole `runVersion` once |
| Run state | Set `awaiting_clarification` only when the actual parent runtime is active/eligible and no retained blocker or stage-continue gate remains; otherwise retain the exact gated/blocked state with the pending binding recorded — the question is asked, and the higher gate still governs |
| Consumers | Invalidate every recognized pending proposal whose binding becomes stale under the accepted source-owned invalidation; no task, report, grant or capacity effect |
| Events | Publish the cursor event for the input revision carrying only bounded type/record references |

## Precedence, resume and retention

Accepted precedence stays: stage continuation, then blockers/unsettled
allocation, then pending clarification, then source/current plan
classification. The answer effect above never bypasses a higher gate; a
retained blocker keeps the run blocked with the clarification gate cleared,
and continue-stage's clarification branch now has its closed source: choose
`awaiting_clarification` when a supported unanswered request exists and no
higher blocker remains.

Exact-identity resume restores the same pending request identity/version and
its preparation binding; it never duplicates the question, replaces the
version or answers it. While `stageContinueRequired` is true, resume/collect/
report paths preserve the pending clarification value unchanged, exactly as
the accepted stage-report contract requires.

Retention: requests and answers are immutable originals; `questionId` and
`answerId` are never recycled; historical answers remain consumers of their
inputs and of every plan/summary that bound them. Required membership in both
directions, following the accepted recipes with no new dependency purpose:

| Owner / key | Complete required typed references |
| --- | --- |
| research_clarification_requests / `{run,questionId,questionVersion}` | Original run/bootstrap, the exact containing input manifest and its clarification decision, byte-equal question bytes, the assessing parent's runtime/proof origin, and original/new direct proofs |
| research_clarification_answers / `{run,questionId,questionVersion,answerId}` | The exact request row, the original Main shown context, the user-origin `ResearchUserActionOriginV1` binding and its Main-authority proof, the newly published input revision and its copied decision, the original receipt/new C, and every consumer that bound the answer's input revision |
| Clarification-answer input record | Its exact answer by full key/digest through the containing manifest, the answered request, and the fresh decision that assessed it |

Unknown or unsupported consumers deny; absence of a pointer never proves
absence of consumers. The accepted discovery bounds (4,096 observations /
16 MiB cumulative, 100,000-visit ceiling and smaller worker limits) apply
unchanged.

## Accounting and compatibility

Charge exact keys plus committed projections for both new families to the
existing shared child-control-retained-bytes run/workspace pools
(8 MiB / 64 MiB), reserved zero, inside the 32 MiB run aggregate. The
clarification manifest is a prepared-input occurrence under its existing
control charges; no new allowance, deduplication or ordinary-end refund
follows. Mutable replacement (run control) uses original-to-final deltas;
immutable requests/answers stay retained. The accepted lifecycle bounds
remain binding and unchanged: at most **64 input revisions** and at most
**32 question/answer pairs per run** — with exactly one question per
clarification assessment, exhausting either bound denies the next
clarification question or answer atomically, without wrap, reset or pruning.
The conditional candidate catalog
becomes **61 logical names** (59 after the merged review-task proposal + two
families); this remains neither a final catalog nor a maximum, and
initializer/root-row consequences stay with the separately tracked catalog
closure. In this document "C" denotes the owning command's commit in the
accepted completion-contract notation, and "new C" its newly committed
proof.

No existing source digest, wire field, report identity, dependency purpose,
version default, cap, receipt version or volatile credential serialization
changes beyond the explicitly proposed decision union, clarification-answer
input-record variant, pendingClarification binding, `awaiting_clarification`
state value, input-manifest producer union and the answer origin type. Old
stores are never imported,
extended on first read or retokened. The proposal can be reverted without
store effects; no persistence downgrade or rollback strategy is claimed for a
future implementation.

## Owner choices and bounded next packet

Accept or revise these proposed choices for **offline components only**:

1. Exactly one bounded material question per clarification assessment, the
   closed `ResearchClarificationDecisionV1` variant in the input decision
   union, byte-bound question/why/suggestion fields and free-text-only
   answers.
2. Immutable request rows keyed `{run,questionId,questionVersion}` with exact
   producer/input binding and the pendingClarification pointer becoming the
   closed `{questionId,questionVersion,requestDigest}` binding; the newly
   admissible `awaiting_clarification` state value with its positive writer.
3. Main-authorized `orkworks.research-clarification.answer.v1` with
   Main-issued `answerId`, the closed `ResearchUserActionOriginV1`
   sidecar-derived user-origin provenance (explicitly not a RuntimeRef
   extension), identical-retry/conflict/stale rules and stage-fence
   rejection.
4. The atomic answer effect union following the accepted event-induced
   pattern: answer row, next input revision with the one clarification-answer
   record variant and the byte-unchanged copied decision plus user-origin
   producer, pendingClarification clear, current-input/runVersion advance,
   consumer invalidation and gate-respecting state selection — with the fresh
   parent assessment as a separate later outcome that **retains the
   applicable clarification-answer entries** in its manifest alongside its
   per-decision record derivation.
5. Two new catalog families (conditional 59→61), shared existing pools,
   unchanged projections and the membership recipes above.

After scoped acceptance, the bounded packet covers `docs/validation/` only:
an independent Python generator, retained JSON, data-only JavaScript
checker/tests and evidence prose, plus documentation routing and LF
attributes. Required examples: the clarification assessment manifest with its
request row and pending binding (including the gated/blocked retained-state
case and the stage-fence denial); the two-revision answer chain — answer
manifest with copied decision and user-origin producer, then the separate
fresh-assessment revision with retained answer entries, its own record
derivation and provenance;
answer admission under exact current request/input binding, identical retry,
conflict, stale request version and stale input cases; gate-respecting state
selection (blocked retained, paused retained with `resumeTarget=assessing`,
awaiting_resume_approval retained, assessing when eligible); changed-answer
re-ask at questionVersion+1; the 32-pairs/64-revision exhaustion denials;
resume restoration of the exact pending binding;
and denial cases (renderer provenance, bearer-as-user, runtime-cast user
origin, duplicate questionId reuse, missing pointer, digest mismatch, wrapper
overflow). All auth,
transport, coverage, currentness, C/command/proof and outcome facts remain
explicitly supplied and unqualified. No legal event history, complete
transaction, storage format, runtime/native/model call, database, migration,
product module or executable product plan is authorized.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: accepted-contract correctness (joins, bounds, precedence and the
Main/user-provenance boundary) and builder completeness (material unanswered
offline-component questions). Reviewer cap two; no reviewer edits, fixtures,
runtime, native or model probes. Correct material findings before handoff;
verify docs metadata/index, build, rendered links, prior-artifact byte
preservation, append-only prior bodies and diff/doc/worktree close-out.
Publication/merge of this proposal does not accept the five choices above.

## Local review disposition

One bounded round used two fresh read-only reviewers. The builder review
identified five material gaps: an unsupplied user-origin producer codec
(the closed RuntimeKey parent/child union cannot hold user provenance), a
contradiction between the answer outcome's fresh-decision branch and its
"clears to null" pending rule, an undetermined post-answer manifest record
composition for the first-assessment case, a missing asking-side effect
union, and categorical rather than enumerated non-active-parent state
mapping. The correctness review independently confirmed the decision union,
stage-fence/precedence, admission order and candidate arithmetic, and found
three material issues — the same chained-question contradiction, the omitted
accepted 32 question/answer pairs cap, and the underspecified fresh-research
record composition — plus five clarity gaps.

This revision resolves all of them: the answer outcome follows the accepted
event-induced pattern byte-for-byte (copied decision, user-origin producer)
with the fresh assessment as a separate later outcome owning its own effect
union and per-decision record derivation; the user-origin provenance is the
closed `ResearchUserActionOriginV1` Main-authorization tuple with the input
manifest's producer field explicitly revised to the union
`RuntimeRef | ResearchUserActionOriginV1`; the asking side has its own
admission/effect table including stage-fence rejection and gated-state
retention; the retained state set is enumerated (blocked, paused with any
pauseReason, awaiting_resume_approval) with `resumeTarget=assessing` while
paused during clarification; the 64-revision and 32 question/answer pairs
bounds are restated as binding with atomic exhaustion denials;
`ResearchDecisionV1`'s unchanged fixed values are stated; the identified-
unknowns representation is explicit; the paused/parent_ended fresh-assessment
case is resolved through exact-identity resume; the shown-context check is
placed with Main authority before retry lookup; and the
`expectedInputRevision` divergence from `expectedRunVersion` is called out.
No accepted technical definition changed and no runtime qualification
follows. Neither reviewer executed code, a fixture, database or
native/runtime process.

After the hosted review of head `e70bd6dc`, one further P2 was addressed in
this revision: fresh assessment manifests now **retain the applicable
clarification-answer record entries** from the prior revision chain alongside
their per-decision derived records, so the input eligible to bind the next
proposal keeps binding the request/answer records it used, as the accepted
material-clarification contract requires; a chain that no longer fits the
unchanged 32 KiB input bound denies rather than truncates.

## Remaining gates

Clarification UI presentation and wording (#746), answer transport/adapter
qualification, the #740 closed-research standard, runtime/native
participation, complete physical catalog/version/initializer/proof closure,
execution and review source codecs, parallel/multi-question clarification,
clarification interplay with stage-cancel replacement chains, and separately
approved executable plans remain open. #1689/#1089 stay open; #740 stays
closed research. Gate 2 remains **Investigate**.
