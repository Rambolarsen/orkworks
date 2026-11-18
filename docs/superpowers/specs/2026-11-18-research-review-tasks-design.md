---
type: Design Proposal
title: Research review tasks and report-consuming outcomes
description: Proposed declared reviewer tasks, retained review input, immutable review outcomes and summary review references under unchanged child, Main and evaluation authority.
tags: [orkworks, research, review, reports, sources, qualification]
status: draft
workflow_status: proposed-for-scoped-review-task-review
---

# Research review tasks and report-consuming outcomes

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumers
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `abb7262d3a3b861d855ff2cea47fd39e07ea02a6` (origin/main,
including the stage-report proposal merged in
[#1807](https://github.com/Rambolarsen/orkworks/pull/1807)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names research reviewers and report-consuming tasks as an unclosed source
contract. This proposal closes that written boundary only: declared reviewer
tasks, retained review input, immutable review outcomes, summary review
references and the collection/consumer/invalidation ownership they add. It
makes new source/action/tool and accounting choices explicit; they are not
acceptance inferred from a merge. Review-outcome transport, adapters, runtime,
complete catalog/initializer and executable plans remain separate #1689 work.
No dependent fixture, Rust/Electron module, storage format, migration, probe or
runtime execution is supplied. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the accepted logical review step representable: an approved research plan
may declare one reviewer task that consumes the researcher's exact selected
immutable report under a separately closed input/outcome grammar, so plan
completion, synthesis and the future execution proposal can honor "missing
research, conflicting required review or material uncertainty blocks ready
execution" without pretending a report receipt is a review result.

Preserve unchanged: the [accepted source grammar](2026-11-12-research-source-codecs-design.md)
and its per-task chain records; the
[input/control prefix](2026-11-12-research-input-control-prefix-design.md)
including prepared-input record variants and the sole run owner; the
[turn/coordination/recovery codecs](2026-11-15-research-continuation-design.md),
which are already TaskKey-generic; the
[completion/summary/lifecycle records](2026-11-15-research-completion-design.md)
with no new field on `ResearchSummaryV1`; #742 report descriptor, request,
head, receipt, correction and child-cutoff rules; the
[selected direct-execution reviewer grammar](2026-10-31-child-review-source-preparation-design.md),
which is never relabeled as research; #744 evaluation/artifact/debit domains;
and the accepted stage-cancel, scope-rejection, prospective-membership and
post-rejection contracts.

This proposal does not select a storage format or physical catalog, close
review delivery/transport, admit reviewer adapters, extend clarification or
execution codecs, add parallel reviewers or reviewer synthesis, or authorize
any runtime/native/model execution. Where an accepted contract says an
unclosed variant is unavailable, it stays unavailable; this proposal adds the
review variant explicitly and nothing else.

Rejected alternatives: converting a research report into a #744 evaluation;
letting the parent grade the review itself; reusing child review-input or
direct-execution review domains; a review record inside the prepared input; an
automatic re-review loop after report correction; and averaging conflicting
reviews into one verdict.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Research lifecycle](2026-10-04-orchestrator-preparation-design.md#clarification-and-research-lifecycle) | Plans list every researcher/reviewer with dependencies and ordered batches; reviewers consume reconciled research; distinct correctness and completeness review questions are declared in the plan |
| [Turn/coordination boundary](2026-10-04-orchestrator-preparation-design.md#clarification-and-research-lifecycle) | The parent checks the report contract and declared review results before reporting completed; it never grades the work itself |
| [Execution gate](2026-10-04-orchestrator-preparation-design.md#same-parent-research-to-execution-transition) | Missing required research, unresolved reviewer conflicts or material uncertainty prevents a ready execution proposal; the summary binds exact report versions/digests and review references |
| [Source codecs](2026-11-12-research-source-codecs-design.md#selected-scope-and-compatibility) | The accepted slice selects one task per plan with `batches=[[taskId]]`; reviewer synthesis is explicitly unsupported there — extending it is a new choice, not an implied widening |
| [Completion summary](2026-11-15-research-completion-design.md#immutable-bounded-synthesis-proposed) | `reviewReferences` may be empty only when complete original membership proves no declared required review; required or existing review records need their separately closed typed source codec; the 128 combined reference ceiling is retained |
| [Coordination codecs](2026-11-15-research-continuation-design.md#parent-coordination-request-and-original-receipt-proposed) | Unsupported required review evidence makes a completed coordination outcome unavailable; the reviewer task reuses the TaskKey-generic turn/coordination families |
| [Membership recipes](2026-11-12-research-membership-publication-design.md#research-metadata-reference-recipes) | Every new consumer/owner needs exact key/body/domain, both lookup directions and complete discovery; unsupported prospective consumers deny the narrow first-report slice rather than count as zero |
| [#744 boundary](2026-10-04-assignment-evaluation-design.md) | A research-report review is not assignment evaluation: no reviewer evaluates itself, the user is the terminal evaluator, conflicts escalate instead of averaging, and evaluation stays separate from coordination success |
| [Selected child reviewer grammar](2026-10-31-child-review-source-preparation-design.md#selected-child-v3-configuration-and-nested-leaves) | Common v3 leaves, reference shapes and framing algorithm are reused; its review-input domain and `review`/`verification` roles are not imported into research |

These are bounded inspection facts at the inspection base, not an inventory of
all writers. Use the proposed research source/control/authority/publication,
SessionApplication bridge and Main approval seams from the
[prefix owner table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
No new module is created; type names are not factories or an existing runtime.

## Selected review slice

One approved research plan declares exactly one researcher task and **zero or
one** declared reviewer task. The reviewer consumes the researcher's exact
selected immutable report. Parallel reviewers, reviewer synthesis,
review-of-review chains, review tasks without a researcher dependency, and a
reviewer reviewing its own outcome are unsupported by this slice. A plan that
declares more than one reviewer task is unavailable, never trimmed.

- `batches` is `[[researcherTaskId]]` without a reviewer and
  `[[researcherTaskId],[reviewerTaskId]]` with one. `maxParallelChildren`
  remains 1; batch ordering serializes the dependency. Existing one-attempt,
  run-wide capacity, plan-revision and exact-approval rules apply to both
  tasks. Declared review questions live in the plan's `scopeText`/task
  descriptions exactly as the lifecycle requires; this proposal adds no new
  question surface.
- The reviewer is a full ordinary research child: its own v3 configuration,
  reservation, allocation, attachment, assignment source, frozen identity
  pair, session state, launch control, turn gate, coordination outcome and
  recovery records under its exact TaskKey. All accepted per-task chain records
  already take a TaskKey and need no shape change. Its frozen pair freezes at
  its own successful reservation; the live counter stays independent.
- The reviewer gets its own plan-owned independent worktree group and
  allocation, like every child. It shares no group with the researcher
  (accepted non-goal), even though its task data is retained startup input.
  Two declared tasks mean two allocated groups; that cost is explicit, and the
  existing worktree/capacity accounting charges both. No cross-plan reuse and
  no automatic cleanup is introduced.
- Reviewer permissions mirror the researcher: no shell, live filesystem,
  external source, connector, Git mutation, native delegation, child
  coordination action, plan approval or code edit. Its review input is exact
  bounded startup data under the accepted native guardrail standard; delivered
  bytes prove no guaranteed containment.

## Research source plan and configuration extension

`ResearchSourcePlanV1` gains exactly one optional member; every existing
member, bound and digest rule is unchanged:

```text
reviewTask: {
  taskId,
  configuration:{configurationId,configurationDigest},
  scope:ResearchScopeRef,
  initialPrompt,group,dependsOn:TaskKey
}|absent
```

`reviewTask.taskId` differs from the researcher task. `dependsOn` is exactly
the researcher taskId; the reviewer task's worktree group, base commit and
configuration identity join the same original sources. When `reviewTask` is
present, `reviewTask.scope` must equal the researcher task's scope reference —
the reviewer reviews that exact approved task scope, not a different one.
`ResearchScopeRef` embeds the owning task key, so every reviewer-side record
that carries a scope reference (plan `reviewTask`, reviewer configuration
joins, reservation, assignment source and review-input snapshot) carries the
**researcher task's** `ResearchScopeRef`, never a reviewer-task-scoped alias:
the scope's internal question/capture/excerpt joins stay task-scoped to the
researcher, and the reviewer is an explicitly declared consumer of that exact
scope (see the membership recipes). `initialPrompt` equals the reviewer
configuration's assignment description. The plan digest domain, 64 KiB body
bound and 2 MiB complete approved-plan accounting are unchanged; the budget
now counts both full configurations, both scope references and every
referenced capability artifact occurrence, with no sharing credit between the
two tasks' retained occurrences. The later review-input snapshot is **not**
part of the plan budget: it is created at the reviewer's
reservation/preparation and charged there under its own object accounting.
The exact UI approval request and approval source/receipt codecs
are unchanged: one approval covers the whole plan including the declared
reviewer task, and the exact shown bytes are the plan's own reviewable
proposal — the complete `reviewTask`-bearing plan source whose
`planDefinitionDigest` the approval source and receipt pin. This proposal
defines no additional displayed-byte payload; the exact Main-owned
shown-byte/version context behind `reviewContextId` remains future Main work
per the prefix owner table, and an offline packet labels that context as
supplied unqualified input.

The reviewer's configuration reuses the full v3
`OutputAssignmentConfiguration` and the accepted research specializations with
these explicit differences:

| Part | Reviewer specialization |
| --- | --- |
| Role/assignment | role `research`; ChildSourceAssignmentV1 exact fields with `inputs=[]`, `dependencies` containing exactly one entry that is the researcher task's full TaskKey object — byte-equal to the plan `reviewTask.dependsOn` value; both worktreeScope path lists empty, worktreeGroupId/baseCommit matching the reviewer's declared group; requirement manifests bind the reviewer taskId |
| Outputs/review | Existing outputContract with `declarations=[]` and both aggregate caps zero; `reviewInput` null. `expectedOutput` describes the review-outcome contract below, not a server artifact or task-success receipt |
| Permission profile | Exact existing requested/effective field sets; readPaths/writePaths/commands/externalSources/connectors/coordinationActions all `[]`; nativeDelegation denied and accessChanges request-only |
| Tools | Exactly `orkworks.research-review.submit.v1`, `orkworks.research-review.receipt.read.v1` and `orkworks.research-review.context.read.v1`, sorted in both requested/effective profiles. No report-submit, result, evaluation or UI action; the reviewer cannot submit a ResearchReport |
| Settings/model/evidence | Existing exact child-startup-settings, child-effective-configuration, child-model-policy and #741 capability-evidence leaves; role and operations match research and the exact executable/settings/model/artifact joins must match this reviewer task |
| Startup ordering | Existing supplied/automatic order plus exactly one `{kind:"research-review-input"}` entry immediately after the supplied entry; 2–34 deliveryOrder entries. No review-input or research-input entry beyond it |

The three tool IDs are **new proposed logical permission identities**, not
current adapter support, mirroring the proposed research-report tool set.
Submit maps to a future closed review-submission wire operation; receipt and
context reads stay limited to this assignment's own receipts and bounded
review/task metadata. They use the existing distinct research capability
family, not parent/report-workflow/#744/UI credentials. Actual transport and
adapter mapping remain unclosed; these names grant no general HTTP, session
read, retained-file fetch or shell bridge.

**No `inputDigest` in the delivery entry, and why.** The accepted dependency
order hashes capture bodies → scope → startup/settings and complete
configuration → prepared input → plan → approval. A reviewer's review-input
snapshot cannot exist at approval: its required `target`/`report` members
bind the researcher's report, which only comes into existence after this same
plan is approved and the researcher task reports. Pinning a snapshot digest in
the approved reviewer configuration would therefore create a
plan→report→snapshot→configuration cycle, unlike the accepted child-review
pattern whose target result already exists at proposal time. This proposal
selects the explicit alternative: the reviewer configuration's delivery entry
carries no digest, the approved plan/configuration pin the task scope and
review contract only, and the exact snapshot — including its target binding to
the then-current selected report — is created at the reviewer's own
reservation/preparation. The report's provenance is governed by the
researcher task's own approved configuration and coordination rules, and the
reviewer's outcome retains the exact reviewed report identity, so the
reviewed bytes are still exactly pinned; they are pinned by the reservation
and outcome records rather than by the pre-approval configuration. This is a
new author choice for scoped review, and the packet must show the ordering
denial (a configuration that embeds an inputDigest for this entry is invalid).

Reviewer startup framing uses the unchanged `ORKWORKS CHILD INSTRUCTIONS v1`
framing and then appends task data as one LF,
`ORKWORKS RETAINED RESEARCH REVIEW INPUT v1` + LF, canonical decimal UTF-8
payload length + LF, the complete canonical review-input snapshot + LF. The
settings deliveryOrder and snapshot join this exact payload. Complete
configuration remains at most 1 MiB and composed startup, including all
automatic content and this data block, at most 256 KiB; larger valid stored
snapshots cannot qualify startup delivery, and no truncation or late streaming
is admitted.

Reservation admission for the reviewer task additionally requires the
dependency condition: the researcher task is `reported_complete` under its
exact accepted coordination outcome, its selected report is the current
selection in the owning collection/expectation and current prepared input, and
no pending proposed scope revision remains. Until then the reviewer
expectation exists from proposal time with a null reservation and the
reviewer task stays unlaunched; no automatic launch follows report arrival.
The reviewer's own reservation, allocation, attachment and assignment-source
events reuse the accepted research source-chain records unchanged, with their
existing atomicity and one-attempt semantics.

## Exact review input snapshot

`ResearchReviewInputSnapshotV1` is exactly:

```text
{schemaVersion:1,task:TaskKey,target:ResearchReportEvidenceSource,
 report:ResearchReport,scope:ResearchScopeRef,
 citedExcerpts:ResearchCitedExcerptV1[],inputDigest}
```

`ResearchCitedExcerptV1` is exactly
`{reference:ResearchExcerptV1,contentBase64,sizeBytes,contentDigest}` where
`reference` is one exact approved excerpt from that scope and `contentDigest`
equals `reference`'s excerpt digest. `contentBase64` is strict padded Base64 of
the exact raw retained excerpt bytes (1..2048), and `sizeBytes` is their exact
length. Bounds: each excerpt ≤2 KiB raw; the citedExcerpts aggregate ≤128 KiB
raw; `report` ≤32 KiB canonical; `scope` ≤32 KiB; the whole snapshot body
≤256 KiB, and the composed startup bound above also wins independently.

Required joins:

- `task` is the reviewer TaskKey. `inputDigest` uses
  `orkworks.research-review-input.v1` + LF over the canonical body excluding
  only `inputDigest`. The snapshot does not exist at plan approval: it is
  created exactly once, at the reviewer's reservation/preparation, binding the
  then-current selected report as `target` (see the delivery-entry rule
  above); the approved configuration pins no snapshot digest.
- `target` is the exact #744 report evidence source selected by the owning
  expectation at snapshot creation, resolved through complete source-owned
  selection — not a caller-supplied head guess or a maximum-version election.
  `report`'s exact identity, sizeBytes and contentDigest must equal `target`;
  any mismatch denies the snapshot.
- `scope` equals the approved task scope reference shared with the researcher
  task. Every file-kind reference in `report.evidenceReferences` resolves
  exactly one approved `ResearchExcerptV1` in that scope; URL-kind references
  cannot occur in this retained-file slice. `citedExcerpts` covers exactly the
  approved excerpts that the report's cited references resolve, sorted
  uniquely by `reference.reference.id` — the approved excerpt's unique
  evidence-reference ID within the scope; no unlisted, extra or cross-task
  excerpt is delivered. A report citing no file-kind references
  yields `citedExcerpts: []` (the array is present and empty); under the
  report codec an established finding requires at least one resolved
  reference, so such a report's findings are all `inferred`/`unverified`.
- The snapshot contains no reviewer configuration digest, session identity,
  bearer, currentness flag, capture bodies beyond the cited excerpts, or
  transitive content fetched because a descriptor cites it. Excerpt bytes are
  explicit declared retained read input, not a citation crawl or a live read
  grant. Missing or unverifiable excerpt bytes make the snapshot unavailable;
  no fetch repairs state.

The snapshot is created once at reviewer reservation/preparation from proven
original sources and is immutable. A corrected researcher report does not
rewrite it; the stale-review rules below govern currency. Input bytes and
digests are provenance, not a current read or publication grant.

## Immutable review outcome

`ResearchReviewOutcomeV1` is exactly:

```text
{schemaVersion:1,task:TaskKey,reservation:ReservationRef,childSessionId,
 sidecarGeneration,launchGeneration,
 reviewedReport:ResearchReportEvidenceSource,
 verdict:"consistent"|"conflict"|"uncertain",
 questionStatuses:[{questionId,status}],
 rationale:Text,citedEvidenceIds:ID[],reviewDigest}
```

- `questionStatuses` has 1–32 unique sorted entries, one per approved
  questionId, with `status` exactly `consistent`, `conflict` or `unverified`.
  The typed surface is the approved scope's research-question set — the
  reviewer's statuses express, per research question, whether the report's
  answer is `consistent` with the cited evidence, `conflict`s it, or remains
  `unverified` (not established, including unanswered). The plan's declared
  correctness/completeness review questions shape the reviewer's instructions
  through the plan description/scopeText and this slice gives them no separate
  typed status surface; a typed review-question codec remains open work and
  its absence must not be filled by inventing IDs. `verdict` is `conflict`
  when at least one status is `conflict`, `consistent` only when every status
  is `consistent`, and `uncertain` otherwise. A `conflict` or `uncertain`
  verdict's `rationale` must name the exact conflicting or unresolved
  questionIds. `rationale` is nonempty well-formed UTF-8 through 2 KiB,
  preserving exact spelling.
- `citedEvidenceIds` is a unique sorted subset of the reviewed report's
  evidence-reference IDs, through which every conflict citation resolves. The
  reviewer cites existing retained evidence only; it introduces no new excerpt
  bytes, fetches no source and cannot cite evidence outside the reviewed
  report.
- `sidecarGeneration`/`launchGeneration` are the reviewer child's actual
  producer identities, derived by the sidecar like report author rules; they
  are never caller-selected. `reservation`, `childSessionId` and `task`
  resolve the reviewer's original reservation and frozen identity.
- `reviewDigest` uses `orkworks.research-review-outcome.v1` + LF over the
  canonical body excluding only `reviewDigest`. Complete body ≤16 KiB.

The outcome is immutable, one per reviewer TaskKey, with no correction loop in
this slice. A changed verdict about the same report requires a new declared
reviewer task in an independently approved plan revision; the original outcome
is never overwritten, averaged or deleted. An exact identical retry returns
its original binding without writes. The outcome is not a #744 evaluation,
result, artifact or receipt-credit event: no evaluation slot, artifact pool or
read-work debit is touched, the reviewer's own criteria/rubric projections
stay unused-for-evaluation, and the reviewer never evaluates its own outcome.
Per #744, a conflicting review escalates to the user as the terminal evaluator;
nothing aggregates or resolves it automatically.

## Review request, admission and family

`ResearchReviewRequestV1` is exactly
`{schemaVersion:1,task:TaskKey,reservation:ReservationRef,childSessionId,
expectedTaskVersion,review:ResearchReviewOutcomeV1,requestDigest}`, with
`requestDigest` using `orkworks.research-review-request.v1` + LF over the
complete canonical request excluding only `requestDigest`.
`ResearchReviewReceiptV1` is exactly
`{schemaVersion:1,task:TaskKey,reviewDigest,acceptedTaskVersion,
commit:CommitRef}` at most 4 KiB; `acceptedTaskVersion` is the reviewer
task's actual live version at admission, read from its task control, never
assumed.

Admission order mirrors first-report rules: current child
capability/generation/epoch/owner/lease/health and request-rate discipline
precede receipt lookup. An authorized identical retry returns the original
receipt before live CAS with no mutation, version, event or charge. Changed
bytes for the same TaskKey conflict. Final reviewer-child revocation still
denies before lookup; no UI or evaluation claim-first path supplies a child
cutoff.

New-outcome preconditions: the reviewer child submits under its current
established runtime and generation-bound capability — the same
authentication path as report submission; no pre-existing turn gate is
required for outcome admission, because the accepted turn gate is established
when that child stops, after (or contemporaneously with) publishing its
deliverable. The reviewer task's turn gate must exist and the child must have
stopped before the parent reports the coordination outcome. The remaining
new-outcome preconditions: the snapshot exists (created at this reviewer's
reservation/preparation) and its `target` equals the current selected report,
the researcher task is `reported_complete` with no pending scope revision, and
the current selected report identity/version/digest equals both
`snapshot.target` and `review.reviewedReport`. The outcome's
publication changes no task coordination version or outcome — like report
publication, it is not a turn result. The parent reports the reviewer task
`completed` only through the existing research coordination operation after
checking that a current review outcome exists; a conflict or uncertain verdict
does not block that coordination outcome, because the accepted lifecycle
blocks ready execution, not coordination itself. Plan completion still
requires every declared task `reported_complete`, so a never-completing
reviewer blocks completion exactly as the baseline predicate requires.

Propose one new immutable logical family in the eventual single fresh catalog
(no physical table, layout or format is selected):

| Family / exact key | Complete value | Wrapper ceiling |
| --- | --- | ---: |
| research_reviews / reviewer TaskKey | `{schemaVersion:1,request:ResearchReviewRequestV1,requestDigest,receipt:ResearchReviewReceiptV1,lastCommit}` | 32 KiB |

The request carries the complete review outcome; there is no separate
duplicate body or wrapper-level digest. The wrapper bound is checked over its
complete canonical value: a maximal legal outcome (16 KiB) embedded in the
request, the request envelope, the receipt (4 KiB) and the wrapper fields fit
only under the 32 KiB ceiling, and a combination that does not fit denies
rather than truncating. It follows the accepted coordination-receipt family
pattern: request and receipt retained, projection excludes only outer
`lastCommit` and `receipt.commit`, and every excluded link resolves its own
original complete proof. Charge exact keys plus committed projections to the
existing shared child-control-retained-bytes run/workspace pools (8 MiB /
64 MiB), reserved zero, inside the 32 MiB run aggregate. No new allowance,
deduplication or ordinary-end refund.

Propose one new immutable object kind, `research-review-input`, keyed by
`{task:TaskKey,inputDigest}` with the snapshot body and its input domain,
following the accepted child-review-input pattern. Charge it to the existing
shared child-source-retained-bytes pools (8 MiB / 64 MiB), with all retained
occurrences counted and no sharing credit. The conditional candidate catalog
becomes **59 logical names** (57 + one object kind + one metadata family);
this remains neither a final catalog nor a maximum, and the actual
initializer/root-row consequences stay with the separately tracked catalog
closure.

## Currency, conflicts and invalidation

A review outcome is **current** only while the exact `reviewedReport`
identity/version/digest equals the currently selected report for the
researcher task, resolved through complete source-owned selection: the report
head registry remains the sole head owner, expectation selection must match
it, and the current prepared input must contain that exact report record. A
matching digest alone, a stored receipt, or absence of a newer head cannot
prove currentness.

- When the researcher's report is corrected and a new version is selected, a
  new input revision publishes and every affected summary/proposal consumer is
  invalidated under the accepted membership contract. The review outcome stays
  retained immutable history but becomes stale: it can no longer satisfy a
  declared required review or be cited in a new summary's review references.
  Replacement requires a new declared reviewer task in a newly approved plan
  revision — never an automatic re-review, a rewritten outcome, or reuse of
  the old reservation.
- **Replacement-revision rules.** A stale declared review is replaced by a new
  research plan revision under the accepted revision rules (a subsequent plan
  is proposed only after the preceding plan is complete or cancelled, each
  revision needs its own exact approval, and revisions stay within the 1–64
  bound). The replacement revision declares only the replacement reviewer
  task, whose `dependsOn` is the **original revision's** researcher TaskKey:
  the original researcher task is never retried, its frozen identity, chain
  records and coordination outcome stay complete immutable history, and its
  currently selected report supplies the dependency condition. The original
  plan is not reopened; when the replacement reviewer completes, its own plan
  revision completes and the lifecycle selector's
  `completedResearchPlan` selects that revision. A revision that declares no
  reviewer simply has none; the "more than one reviewer task is unavailable"
  rule applies per revision, so a revision declaring one replacement reviewer
  plus retaining the stale reviewer as history is two reviewer declarations
  only across distinct revisions, never within one. Multiple successive
  corrections produce a chain of replacement revisions under the same rules,
  each with exactly one reviewer task.
- **Reviewer expectation row is not a mutating consumer.** The reviewer
  expectation row is unchanged by a researcher report correction: its
  `selectedReport` stays null (the review outcome is not a report), its
  collecting state remains source-owned, and no row is rewritten, invalidated
  or tombstoned by the correction. The stale-review condition is resolved
  dynamically through source-owned selection at each gate check, never stored
  on the expectation row. The reviewer expectation remains a discovered
  consumer whose dependency gate now denies until a replacement reviewer task
  is approved; the stale outcome leaves no row-visible state beyond its own
  retained review history.
- A declared required review that is missing (reviewer not yet complete),
  stale, `conflict` or `uncertain` blocks a ready execution proposal under the
  accepted lifecycle rule. Because the execution source codec is still
  unclosed, this proposal records the obligation as a constraint that future
  execution source/effect work must honor; it closes no execution variant.
- Summary publication revalidates currentness at its own admission: a summary
  citing a stale review is denied atomically rather than silently published.
  `uncertainties` may still state material uncertainty; a summary never
  resolves it.
- `reviewReferences` in `ResearchSummaryV1` uses the typed review evidence
  source defined below, unique and sorted by canonical complete
  evidence-source bytes, within the unchanged combined 128 ceiling with report
  references. Empty `reviewReferences` continues to require complete original
  plan/source membership proving no declared required review. Nonempty
  requires each cited review to be the current outcome of its declared
  reviewer task, bound to the currently selected report version; a conflicting
  review is never cited as satisfying the required-review gate, and remains
  visible in retained history and agent details for user escalation instead
  of averaging.

### Summary review evidence source

`ResearchReviewEvidenceSource` is exactly:

```text
{schemaVersion:1,sourceKind:"research-review",
 reviewIdentity:ResearchReviewIdentity,sizeBytes,contentDigest}
```

`ResearchReviewIdentity` is exactly
`{workspaceId,runId,planId,planRevision,taskId:ID,reservationId,
childSessionId,configurationDigest,sidecarGeneration,launchGeneration}` where
`taskId` and the run/plan members form the reviewer task's exact TaskKey and
the remaining members resolve its original reservation, child session and
configuration. `sizeBytes` is the exact byte length of the stored canonical
`ResearchReviewOutcomeV1` body including `reviewDigest` — the hash input, not
a wire-string escaping, wrapper or receipt size. `contentDigest` equals that
outcome's `reviewDigest`. The complete source resolves the exact
`research_reviews` family row through its typed identity; a digest-only
match, a bare task reference or a wrapper reference cannot prove currentness,
and a `sourceKind` tag does not make this a #744 `EvidenceSource`: it creates
no evaluation edge, uses no #744 identity space and enters no #744 source
ledger. Sorting and uniqueness for `reviewReferences` compare complete
canonical source bytes, exactly as the accepted summary rule sorts report
references.

## Collection, expectation and membership extension

The prefix collection is keyed by RunKey with per-task expectations. This
proposal explicitly extends the accepted singleton-collecting-task rule,
including the closed record that carries it. The accepted
`ResearchCollectionV1` has a singular `task:TaskKey` member; this proposal
revises that accepted prefix record with exactly one optional additional
member:

```text
reviewTask:TaskKey|null
```

`task` remains exactly the researcher (primary) task; `reviewTask` is present
and equal to the reviewer TaskKey exactly when the current plan declares one,
and null/absent otherwise. The record is never replaced wholesale by a task
array, and no second current-input pointer appears. `collectionVersion`
increments once for any owned membership/binding/selection change across
either task — including reviewer expectation creation at proposal and any
later owned change — coalescing compound changes as the accepted rule
requires. Expectations exist for every declared task of the active
collection's current plan (one or two in this slice), created atomically at
proposal time. The reviewer expectation binds `dependsOn` context through its
task configuration/plan sources; its `selectedReport` remains null through
this slice because the review outcome is not a report; its stream resolves
only after its own reservation binds, through the original reservation and
configuration. Collecting membership for both declared tasks remains
source-owned state until accepted finish/cancel retirement; plan pause and
completion do not implicitly retire it. This is a new author choice revising
the accepted prefix record for scoped review; the packet must show both the
one-task (null `reviewTask`) and two-task shapes.

The accepted first-report slice permits publication only with the producer
run's one active collection and its exact collecting expectation as the
affected active preparation, denying any additional prospective consumer whose
semantic effect is unclosed. With a reviewer declared, the researcher's report
acceptance gains exactly this closed consumer effect: the reviewer
dependency/launch gate reads the current input and task/plan controls, and
fences reviewer admission until the dependency condition holds — with **no**
expectation-row mutation, no automatic launch, no task-version change and no
grant effect. Complete discovery for first-report eligibility must then cover
the reviewer expectation and its input consumption; an unsupported additional
consumer still denies.

New membership recipes in both directions, following the accepted recipes
with no new dependency purpose or graph owner:

| Owner / key | Complete required typed references |
| --- | --- |
| research-review-input / `{task,inputDigest}` | Original reviewer plan/reviewTask, reviewer configuration/reservation/attachment/assignment source, exact target report identity through the containing snapshot, approved scope, and each cited excerpt's capture/source retention |
| research_reviews / reviewer TaskKey | Original reviewer task/plan/configuration/scope, reservation/child/frozen identity, exact reviewed report identity resolving through its original selection/input history, cited evidence references through the reviewed report, the reviewer's original turn gate and coordination receipt where present, and original request/receipt/new-C proofs |
| Reviewer expectation / TaskKey | Collection RunKey and original configuration/scope; original reservation and selected report only when present; dependency condition resolves through the exact researcher task and its current selection |
| Reviewer scope consumption | Every reviewer-side record carrying a scope reference resolves the researcher task's exact `ResearchScopeRef` by full task/digest identity through the original researcher plan/assignment source; no reviewer-task-scoped scope alias exists, and the scope's internal joins stay researcher-task-scoped |
| Summary `reviewReferences` / ResearchReviewEvidenceSource | The exact `research_reviews` row by full `ResearchReviewIdentity`, its original reservation/child/configuration and reviewed report binding through the containing input history, and the summary's own report/plan/input provenance; citation retains the review in both directions, and currentness is resolved at selection, never assumed from the digest |

A review row must resolve its exact reviewed report through complete
source-owned history, including the input revision that contained it —
absence of a head or of the current selection pointer cannot repair or
re-derive that binding. Both lookup directions and complete owner-family
membership stay required before any current read, publication or purge;
unknown or unsupported consumers deny. The accepted discovery bounds
(4,096 observations / 16 MiB cumulative, 100,000-visit ceiling and smaller
worker limits) apply unchanged.

## Accounting and compatibility

Beyond the family/object charges above: every changed control/summary/
selector/resource original-to-final delta coalesces once per ledger under its
existing pool; two declared tasks double their per-task chain occurrences
under the same caps, and the 2 MiB approved-plan budget now spans both tasks'
sources. All existing run/workspace aggregate, command, physical, discovery
and smaller row/worker limits win independently. A maximal legal combination
(eight-version report histories, both configurations, snapshots and all
chain sources) may exceed a containing budget and must deny; no simultaneous-
maxima fit is claimed.

No existing source digest, wire field, report identity, dependency purpose,
version default, cap, receipt version or volatile credential serialization
changes. Old stores are never imported, extended on first read or retokened;
format-3/4/6 artifacts stay unavailable for adoption. The proposal can be
reverted without store effects; no persistence downgrade or rollback strategy
is claimed for a future implementation.

## Owner choices and bounded next packet

Accept or revise these proposed choices for **offline components only**:

1. One declared reviewer task (zero or one) per approved research plan with
   dependency-ordered batches, two independent worktree groups and unchanged
   plan/approval/capacity rules; no parallel reviewers, synthesis or chains.
2. The closed reviewer v3 specialization: a single full-TaskKey dependency
   entry, three new logical review tool IDs, a digest-free
   `research-review-input` delivery entry with framing and snapshot creation
   at reservation (no pre-approval snapshot pinning), and unchanged
   configuration/startup bounds.
3. `ResearchReviewInputSnapshotV1` with exact target/selected-report binding,
   cited-excerpt delivery only, and the immutable one-per-task
   `ResearchReviewOutcomeV1` with per-research-question statuses, verdict
   rules and no correction loop.
4. The research-review request/receipt family, one-object-kind/one-family
   catalog additions (candidate 57→59, conditional), shared existing pools
   and unchanged projections.
5. The `ResearchCollectionV1` `reviewTask` member with expectation extension
   to every declared task and the closed reviewer consumer effect, the summary
   `ResearchReviewEvidenceSource` and review-reference
   currentness/citation rules, and stale-review replacement only through a
   newly approved plan revision.

After scoped acceptance, the bounded packet covers `docs/validation/` only:
an independent Python generator, retained JSON, data-only JavaScript
checker/tests and evidence prose, plus documentation routing and LF
attributes. Required examples: the two-task chain with dependency-gated
reviewer reservation and the digest-free delivery entry (including the
ordering denial for a configuration that embeds a snapshot digest); one-task
and two-task collection shapes (`reviewTask` null and present); review
admission under the exact target binding, exact retry and conflict cases;
stale review after a report correction with the unchanged reviewer
expectation row and the replacement-revision declaration; summary citation
with the typed review evidence source, empty-vs-required reviewReferences and
denial cases (wrong task, cross-task scope, unlisted excerpt, stale target,
conflict-as-satisfaction, unverifiable excerpt bytes, wrapper overflow). All
auth, transport, coverage, currentness, C/command/proof and outcome facts
remain explicitly supplied and unqualified. No legal event history, complete
transaction, storage format, runtime/native/model call, database, migration,
product module or executable product plan is authorized.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: accepted-contract correctness (joins, bounds, currentness and the
#744/#742 boundary) and builder completeness (material unanswered
offline-component questions). Reviewer cap two; no reviewer edits, fixtures,
runtime, native or model probes. Correct material findings before handoff;
verify docs metadata/index, build, rendered links, prior-artifact byte
preservation, append-only prior bodies and diff/doc/worktree close-out.
Publication/merge of this proposal does not accept the five choices above.

## Local review disposition

One bounded round used two fresh read-only reviewers. The builder review
identified five material gaps: an undefined `ResearchReviewEvidenceSource`, a
wrapper that duplicated the outcome body beyond its ceiling, an unspecified
stale-review replacement-revision shape, an unspecified reviewer-expectation
post-correction state, and unconstructible approval shown bytes. The
correctness review confirmed the candidate-57 baseline, dependency/one-attempt
gating, currentness binding and absence of silent scope widening, and found
nine issues: the pre-approval digest-ordering cycle (delivery entry), the
same two closure gaps, the singular collection `task` member, the
questionStatuses target surface, the wrapper duplication, the cross-task
scope reference, turn-gate ordering, the dependency element encoding and a
colliding citedExcerpt uniqueness key. This revision closes all of them: the
typed evidence source and its membership row are defined; the wrapper drops
the duplicated body/digest; replacement-revision, expectation-state and
shown-byte rules are stated; the delivery entry loses its digest with the
ordering rationale and a packet denial; the collection gains an explicit
optional `reviewTask` member; questionStatuses are explicitly keyed to the
approved research questions with the typed review-question codec left open;
the reviewer is an explicitly declared consumer of the researcher task's
scope; outcome admission follows the established runtime → turn gate →
coordination order; dependencies are full TaskKey objects; and citedExcerpt
uniqueness uses the approved excerpt's reference ID. No accepted technical
definition changed and no runtime qualification follows. Neither reviewer
executed code, a fixture, database or native/runtime process.

## Remaining gates

Actual review submission transport and adapter qualification, the #740
closed-research standard, runtime/native participation, complete physical
catalog/version/initializer/proof closure, execution source codecs, material
clarification, parallel/multi-reviewer research, stage-cancel and
rejection interplay with declared reviews, user-correction input origins and
separately approved executable plans remain open. #1689/#1089 stay open;
#740 stays closed research. Gate 2 remains **Investigate**.
