---
type: Design Proposal
title: Current hierarchy summary detail projection
description: Proposed bounded authoritative result-summary details and exact local viewed-summary behavior.
tags: [orkworks, hierarchy, projection, evaluation, review]
status: draft
---

# Current hierarchy summary detail projection

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#746](https://github.com/Rambolarsen/orkworks/issues/746),
[#744](https://github.com/Rambolarsen/orkworks/issues/744), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Source baseline: main `273a02f1` (2026-10-26).

**Status: proposed for owner and producer review.** The owner authorized defining
this contract after the [projection review](../../validation/hierarchy-projection-contract-review.md).
The concrete new response/viewing choices below are the author's proposal, not
already accepted architecture, a runtime implementation or launch eligibility.
#740 remains deferred/no-go until November. Canonical source records, accepted
summary material/revisions and the narrow immutable-record read remain unchanged.

## Outcome and chosen boundary

An explicit current-result inspection explains which required outcomes/findings
are current and shows authoritative completeness/quality without asking the
renderer to reconstruct aggregation from historical reviewer records. Choose a
separate logical `ReadHierarchySummary` operation, rather than expanding every
overview or relabeling the existing immutable-record read as an aggregate.
Concrete HTTP/IPC names and implementation plans remain separate.

Reuse [SummaryMaterialV1](2026-10-20-hierarchy-read-recovery-design.md#durable-summary-material)
byte-for-byte as read-owned metadata, including all current required outcomes and
required findings. Add approved descriptive context and bounded source-labelled
assessment/metric projections alongside it. These additions are **not** material
fields and do not change any source digest or summary revision rule. The existing
[ReadHierarchyRecord](2026-10-20-hierarchy-read-recovery-design.md#exact-result-and-evaluation-detail-reads)
continues to return original immutable claims with whole-record freshness.
Neither operation grants actions or retrieves arbitrary evidence/files.

## Strict request and exact selection

Use the [overview's](2026-10-17-hierarchy-projection-payload-design.md)
ID/Digest/Counter types and #744's complete tagged AssignmentIdentity. Every
property is required; reject duplicate/unknown properties, unknown variants and
out-of-bound values. Do not coerce integers, names or identities.

`SummarySelection` has exactly:

| Property | Meaning |
| --- | --- |
| `assignmentIdentity` | Complete #744 root or child subject, including the frozen admission task version for children |
| `accepted` | Exact overview accepted-head variant: `{kind: "none"}` or `{kind: "result", reference: ResultRef}` |
| `summaryRevision` | Exact known overview token, canonical decimal string `"1".."2147483647"` |

Renderer request has exactly `schemaVersion: 1`, `runId: ID`,
`expectedSnapshotVersion: Counter`, and `selection: SummarySelection`.
Electron main adds exactly `workspaceId` and `sidecarGeneration` from its owned
selected-workspace/sidecar context. Both request boundaries retain the existing
**4 KiB** detail ceiling; this is not an extension of the overview's 1 KiB request.
No renderer workspace, lifetime, token, path or opaque digest-only selector is
accepted. Bound child identities use the admission snapshot, never today's task
counter or a #743 skill binding to complete a missing member.

Selection requires a fresh complete overview in the current local request
context, with no unresolved latest-refresh failure, equal-version conflict or
exhaustion fence. A summary-detail success cannot clear any overview failure
fence or make retained stale overview data current. Selection must appear exactly
in that current complete run overview's known
EvaluationDisplay with known summary. Its accepted reference must name the same
assignment. Planned/reserved nodes with not-started evaluation and unavailable
summaries have no selectable current-summary reference. An accepted head of none
can still be inspected with its known Unassessed/failure material, but does not
create an accepted result or a viewed-result token.

Authenticate with the existing Electron-only UI authority and app-window/main-
frame sender/origin checks before source lookup or existence disclosure. Parent/
child reporting bearers cannot authorize this operation. Validate current
workspace/lease, admitted run, owned lifetime and exact selected node under the
existing coherent-publication boundary; it cannot create/recover source records.

## Successful response

Success has exactly `schemaVersion: 1`, `workspaceId`, `runId`,
`sidecarGeneration`, `snapshotVersion`, `selection`, and `detail`.
Scope and selection echo the validated request; snapshotVersion must still equal
expectedSnapshotVersion at publication. Success describes a coherent current
read at its linearization point, not a guarantee of future currentness.

`detail` has exactly these properties:

| Property | Exact shape / source |
| --- | --- |
| `material` | Complete existing SummaryMaterialV1 from the current proven durable row; both assignment and accepted head equal selection |
| `criteria` | Exact approved CriterionSnapshot[] in #744 snapshot order, including optional descriptions for immutable context; at most 32, each description at most 2 KiB |
| `rubric` | Exact approved RubricSnapshot: id, bounded version, 1..16 dimension ID/description pairs, evaluatorRole; descriptions at most 2 KiB |
| `evaluations` | Exact unique EvaluationReference[] copied from this node's overview, at most 16; same assignment/accepted result, canonical reference order |
| `outcomes` | OutcomeDetail[] covering exactly material.requiredOutcomes, in the same canonical target order, at most 49 |
| `findingSources` | FindingSource[] covering exactly material.requiredFindings, in its canonical logical reviewer/finding order, at most 512 |
| `completeness` | Server-derived CompletenessDisplay below |
| `quality` | Server-derived QualityDisplay below |

Criteria/rubric bytes must verify the material's existing snapshot digests using
#744's snapshot domains and serializer. No instruction body, permission profile,
full configuration or mutable catalog text substitutes for this approved
context. The rating target has the fixed #744 rating meanings; it has no invented
criterion ID/description. Optional criterion descriptions are context, not extra
required outcomes or contributors to completeness.

`material` keeps all its existing strict fields, ordering, finding text/location/
severity/targets and current/disputed classifications. It contains no u64 source
artifact metadata; these bounded identities/revisions are safe display types.
Never add a provenance, metric or display field inside that material object.
No JSON string of an unvalidated canonical u64 object is embedded here. Render
all descriptions/finding text as plain text, never HTML, executable Markdown or
terminal input.

### Outcome details and exact reviewer attribution

`OutcomeDetail` has exactly `target: RequiredImpactTarget`,
`fullyAssessed: boolean`, and `assessments: AssessmentAnnotation[]` (at most 16).
Target resolves exactly one required criterion, rubric dimension or rating in
material and approved context. Its aggregate disposition is the corresponding
material entry, not a second renderer calculation.

An AssessmentAnnotation has exactly:

| Property | Type / rule |
| --- | --- |
| `evaluationDigest` | Digest resolving exactly one complete entry in detail.evaluations |
| `claimed` | Original criterion tag satisfied/unsatisfied/unassessed, dimension tag meets/below-standard/unassessed, or rating integer 0..3 / string unassessed, selected by the target kind |
| `effective` | Same target-specific value domain after #744's currentness/eligibility checks; an unsupported original claim is not promoted to assessed |
| `participates` | Boolean: #744 says this evaluation stream/head/reviewer participates in the current aggregate; ineligible history is false, not an extra Unassessed vote |
| `sourceState` | current/stale/unavailable, for this outcome's supporting evidence, independently of whole-record freshness |
| `reasons` | Unique fixed codes from declared-unassessed, missing-evidence, stale-evidence, evidence-unavailable, declared-conflict, invalidated, superseded, reviewer-ineligible; at most 8 |

Assessments are unique and sorted by evaluationDigest. Cover every selected
overview evaluation for this target; if its source cannot be accounted for,
return unavailable rather than silently omitting an adverse/unknown contributor.
A stale outcome in an otherwise eligible evaluation becomes effective Unassessed
under #744; stale optional content cannot invalidate unrelated required outcomes.
A declared Unassessed outcome may be current but still prevents full assessment.
Invalidated/superseded/ineligible history never contributes. Conflicts/disputes
are resolved only by #744's declared-target rules, not prose similarity.

A digest here is a **response-local join key**, not authority to fetch an object.
The sidecar validates each complete reference/source identity; two different
references resolving to one digest make this response unavailable. Explicit raw
record inspection resolves the full reference and uses the existing exact
ReadHierarchyRecord contract/overview membership checks; it never submits the
local digest, list index or a substituted latest revision as a selector.

`fullyAssessed` is a producer projection, not “some reviewer supplied a number”.
It requires applicable eligible assessments, current/verifiable support for each
assessed claim, no participating Unassessed claim and no unresolved target-level
conflict under #744. Preserve known independent failures in material even when
this is false. Different current ratings all below 3 may establish a threshold
failure without selecting one exact rating; preserve their individual values.

FindingSource has exactly `reviewerIdentity`, `findingId: ID`,
`originEvaluationDigest: Digest`, and `disputeEvaluationDigests: Digest[]`.
The logical key matches exactly one material finding. Origin resolves its exact
current revision/finding in evaluations; dispute digests are a unique sorted set
of at most 16 references with current effective disputes of that exact target.
No stale dispute suppresses a failure. Material's disputed classification and
#744's effective source set must agree; absent provenance is unavailable, not
an inferred finding or an undifferentiated renderer warning. Historical findings
outside current material remain immutable-record claims, not current work.

### Completeness and quality displays

These fields make the projection explicit; #744 remains the aggregate owner.
The following display classifications are proposed for owner/producer review.
They do not change stored ratings, eligibility, failure precedence or outcomes.

CompletenessDisplay has exactly `required`, `satisfied`, `unsatisfied`,
`unassessed` (integers 0..32), and `percentage`, exactly `{kind: "unassessed"}`
or `{kind: "assessed", numerator: 0..32, denominator: 1..32}`.

Count only required criteria. The three counts partition required. A fully
assessed meets criterion counts satisfied; a fully assessed needs-rework
criterion counts unsatisfied. Otherwise count it Unassessed, preserving any
independent current failure in material. Thus a failure plus a participating
Unassessed claim can keep overall Needs rework while this criterion remains
in the completeness Unassessed count. This is conservative completeness display,
not suppression of the failure. Assessed percentage is allowed only when required
is positive and unassessed is zero; numerator equals satisfied, denominator
required. The renderer may format that exact server-supplied fraction as a
percentage, but may not shrink the denominator or derive its own assessed state.
A legacy zero-required subject has zero counts and Unassessed percentage, never
0/0 or a pass. Missing approved source bytes instead fail unavailable.

QualityDisplay is exactly `{kind: "unassessed"}`, `{kind: "rating", value: 0..3}`,
or `{kind: "disagreed", values: <sorted unique 0..2 integers, 2..3 entries>}`.

A single rating is returned only when every required quality dimension and rating
has current fully assessed support and participating ratings agree. Value 3
additionally requires every dimension meets the standard; never display quality
3 as a pass over an established below-standard dimension. All-below-3 rating
disagreement with otherwise fully assessed dimensions returns disagreed with the
individual values, not an average/minimum/latest reviewer score. Threshold-crossing
2/3 disagreement, missing/unassessed quality, or an unsupported combination returns
Unassessed; current individual claims remain visible in outcome annotations.
An independent quality failure still produces overall Needs rework through #744.
Do not reinterpret these display values as acceptance or a reason to launch work.

The numeric/count display choices above are additional proposed presentation
rules. They need producer/owner agreement; they do not silently amend #744's
canonical schema or accepted aggregate precedence.

## Coherence, failures and budgets

Capture/revalidate exact source membership, accepted head, evaluation heads and
eligibility, evidence/dependency health, approved snapshots, current summary row
and its material/revision under the accepted publication boundary. Read only
selected subject records and maintained source-owner currentness proofs. No
transcript, raw output file, arbitrary evidence body or new cross-workspace scan
is fetched to assemble details. Missing currentness proof is unavailable, never
permission to compute it from terminal prose or unbounded rehash work.

The existing run snapshot version must also invalidate for every visible field
of this detail projection, including per-outcome source health, participant
eligibility and metric changes that leave material and overview display bytes
unchanged. Publish that invalidation under the same boundary before another
current detail read; equal versions cannot stand for differing detail data.
This is explicit proposed writer/invalidation coverage for the new read, not a
new counter or a change to durable material revision semantics. Incomplete writer
coverage makes the detail operation unavailable until qualified.

Assemble outside the short publication lock and revalidate before serving. If
snapshot/selection/material/head/health changes, return stale-reference and require
a fresh overview plus explicit reselection. Do not retarget to the latest accepted
result/evaluation/summary or reuse an old successful detail in a new scope.
Same-scope unchanged source retries may return identical data, with no viewed or
revision side effect. A temporary recovery fence, lost/corrupt/exhausted lineage,
missing source snapshot/provenance or unprovable aggregation returns unavailable.
The immutable-record read can remain usable when its own narrower proofs hold.

Failures use the existing fixed detail codes stale-reference, unavailable,
invalid-request, limit-exceeded and denied, never raw source/error text. A denial
reveals no existence. Failures carry no partial detail, invented empty outcome
list, zero metrics, current success or viewed token. Known overview summary
unavailability leaves this operation disabled; it is not a revision-1 repair path.

| Resource | Bound / behavior |
| --- | --- |
| Requests, renderer/main and main/sidecar | Existing 4 KiB detail request limit, including echoed complete selector members |
| Complete response | Existing 4 MiB detail ceiling, UTF-8, depth 32; existing 2 MiB material/row cap also applies |
| Arrays/source records | 32 criteria, 16 dimensions, 49 material outcomes × at most 16 annotations; 16 exact evaluation refs; 512 material findings/source links; narrower source bounds win |
| Shared read pools | Overview, immutable-record and summary-detail reads together share 60 attempts/minute and 2 in-flight reads per selected workspace |
| Assembly | Existing 2 coherent attempts within one 5-second deadline; cancellation releases slots |
| Storage | No new summary row, log, receipt, persisted viewed record or backend quota; all existing lineage/source limits remain |

Authenticate, charge attempts and validate bounded input before lookup; acquire
one shared read slot before assembly. Count serialized bytes/depth/collections
incrementally on sidecar, Electron and renderer boundaries; never allocate an
unbounded object and check afterwards. Serve all selected material/context and
contributors or fail limit-exceeded; never paginate/truncate into a complete
claim. Source-valid maxima need not fit the response cap: unavailable detail is
the explicit tradeoff. An implementation plan must prove request/response/work
packing for its claimed supported slice; this document does not qualify one.

## Explicit display and local viewed identity

The viewed identity remains workspace, run, complete assignment identity,
accepted result revision/digest and summaryRevision. Only an accepted result
can acquire a viewed-result token. Do not include transport snapshotVersion in
the stored identity or generate a backend acknowledgement capability.

Mark it viewed only after all of the following:

1. The user explicitly opens this node's **current result summary** in the active
   visible details/run-summary surface. A hidden prefetch, restoration, parent
   inspection or immutable-record selection is insufficient.
2. A complete successful ReadHierarchySummary matches the currently inspected
   node, workspace/run/lifetime, expected snapshot, accepted head and summary
   token. Revalidate those local contexts through render commit/paint; a late,
   stale, failed or replaced response cannot mark anything viewed.
3. The surface renders the complete material summary: overall, all required
   outcome classifications and current/disputed required findings with their
   labels. Render all of it in the open vertically scrollable summary section;
   no collapsed/omitted material section or “load more” can count as full display.
   No eye tracking, full-scroll/read claim or extra approval click is introduced.

The viewed mark records explicit opening and complete presentation, never proof
that every word was read. Raw record details and optional provenance disclosures
may stay collapsed without hiding required material. Merely having fetched the
bytes or knowing the token is insufficient. The renderer stores bounded local
presentation state under #746's existing 64-run session-local cache; no durable
server state, unread acknowledgement, user acceptance or grant follows.

Changing required material clears only that exact subject's viewed/reopen
suppression via a new durable token. Pass→Unassessed→Pass remains three tokens.
Freshly revalidated unchanged material can retain an earlier viewed identity
across snapshots/lifetimes; old in-flight reads can never create a new mark.
Extra detail changes alone follow the accepted material rule: for example, a
rating change 0→2 that stays below-standard updates snapshot/details but does
not add a field to SummaryMaterialV1 or manufacture a new summary revision.
Such a token records viewed **material**, not every later nonmaterial metric.
All existing terminal-coordination, unresolved-action/failure, live-child/
reservation, focus and inspection protections still govern auto-collapse.

## Required review and fixture cases

These are exact logical acceptance obligations, not executed production/source-
valid fixtures or native evidence. Concrete wire goldens/checkers belong to the
reviewed qualification/implementation handoff after this schema is accepted.

| Case | Required disposition |
| --- | --- |
| R2 history A: C1 evidence stale, C2 current failure, optional O current | Overall Needs rework; C1 material/annotation Unassessed/stale; C2 current failure; completeness percentage Unassessed |
| R2 history B: optional O stale, C1/C2 current failures | Same immutable historical record and overall may remain; C1/C2 detail classifications current, required completeness assessed; O cannot downgrade them |
| Required failure plus a participating Unassessed claim on that same criterion | Preserve Needs rework; criterion is not fully assessed, counted Unassessed for completeness; never hide the known failure |
| Ratings 0 and 2; all required quality dimensions assessed | Disagreed [0,2], no manufactured single score; known quality failure retained |
| Ratings 2 and 3; no other failure | Quality Unassessed, conflict visible; never average to a pass/failure |
| Missing quality dimension plus current uncontested rating 1 | Quality Unassessed, preserved known failure; no zero or fully assessed claim |
| Rating 3 with an established below-standard dimension | No quality-3 pass display; current claims preserved; overall remains Needs rework |
| Optional-only correction / numeric below-standard rating or detail-only currentness changes with material unchanged | Advance snapshot for changed detail data; old expected version fails; unchanged material token/viewed identity, no implicit source rewrite |
| Finding disputed by current evidence, then dispute evidence becomes stale | New authoritative material/currentness; ineffective dispute cannot suppress a current finding; exact source references remain labelled |
| Missing admitted summary row / initial summary capacity unavailable | Unavailable; read never creates a row or revision 1 |
| Snapshot/head/token/workspace changes during read or before paint | Failure/drop and no new viewed mark; no retargeting or automatic refresh selection |
| Explicit current-result open succeeds, all material rendered in active surface | Store the exact local viewed identity; no session selection, unread acknowledgement or approval |
| Hidden mount/prefetch/restore/record-only inspection, failed/truncated material, or no accepted result | No viewed-result token; ordinary read-only inspection may remain available |
| Current summary token q1 was viewed; corrections yield q2 then q3 with original material | q3 unviewed even if its bytes resemble q1; no auto-collapse before explicit display |
| Repeated evaluation digest maps to differing full references / provenance missing | Unavailable, never digest-only identity guessing |
| Required source, member, text, count, depth, request or response budget exceeded | Bounded failure with no partial current detail or history mutation |
| Third read or 61st shared-pool attempt | Existing bounded refusal; new operation creates no extra pool |
| Parent/child bearer, foreign scope or invalid sender | Denied before disclosure; no filesystem capability or raw evidence access |

## Owner/producer handoff

Review the proposed operation/selector and current-summary display scope, the
conservative completeness/quality presentations, and explicit complete material
viewing as one bounded contract. Reconcile #744 aggregation/snapshot/provenance
and #746 consumer/viewed semantics without changing accepted source/material
bytes. Keep #1089 open until that scoped owner/producer review is resolved.
Merging this proposal records the design, not that acceptance.

After acceptance, produce bounded exact request/response and adversarial fixture
vectors, close source/producer/native proof gaps and seek a separately approved
runtime plan. No IPC route, new action, migration, model probe or live hierarchy
integration is delivered here; #740 and storage/capability gates remain separate.

## Builder check — 2026-10-26

One fresh read-only Luna-tier builder check read this draft and reported
“nothing important left” before an implementation plan could be prepared.
That check raises no blocking planning question; it is not owner/producer
acceptance, executed codec evidence or native/runtime qualification. No proposed
source field, metric presentation, viewed behavior or bound becomes accepted by
that result alone.

After the builder check, an author clarification explicitly lists detail-only
source/metric changes in snapshot invalidation coverage. It implements the
draft's intended exact-snapshot coherence without adding a counter or material
field. No second independent review of that clarification is claimed.
