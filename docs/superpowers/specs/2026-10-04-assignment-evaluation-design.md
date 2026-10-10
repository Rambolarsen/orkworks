---
type: Design
title: Assignment quality and completeness evaluation
description: Define the evidence, evaluation, correction, and retention contract for configured agent assignments.
tags: [agent-hierarchy, assignment-evaluation, taskmaster]
---

# Assignment quality and completeness evaluation

## Scope and dependencies

This specification defines how to evaluate one configured agent's result for
one approved assignment. It is the written contract for issue
[#744](https://github.com/Rambolarsen/orkworks/issues/744) in initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738). It defines no
runtime routes, storage, UI, or coding-tool support.

It depends on the assignment identities and role configuration in
[#741](https://github.com/Rambolarsen/orkworks/issues/741), report authority in
[#742](https://github.com/Rambolarsen/orkworks/issues/742), and ordinary-child
scope in [#610](https://github.com/Rambolarsen/orkworks/issues/610). Before an
implementation plan, reconcile those contracts, #740's capability gate, and the
authoritative specs and ADRs.

Evaluation answers: **Does this exact result meet its approved requirements and
quality standard?** It cannot approve or launch work, retry a task, widen
permissions, advance an undeclared dependency, accept work for the user, or
authorize a merge. It does not change ordinary Peon behavior.

## Assignment and result identity

An evaluation binds one approved assignment identity, the exact accepted result
revision/digest, sidecar-derived criteria and rubric snapshot digests, reviewer
identity, and immutable evidence. The tagged child/root values defined under
[Canonical records](#canonical-records) are #744's encoding of identities resolved
from #741/#742 approved state; those documents do not currently define a tagged
assignment-identity schema. Display labels and reporter claims cannot supply
identity. Changing an identity member or rubric requires a new approved
assignment. Runtime generations fence authority without changing the assignment
subject. Root results use explicit user review; v1 child reviewers evaluate only
child results.

### Declared outputs and authorized sources

The approved assignment declares at most 32 unique output IDs/kinds, using
#741's identifier rule. Each `OutputDeclaration` has exactly `id`, `kind`,
`maxBytes`, and `source`. `maxBytes` is a positive finite integer. `source` is
either `{"kind":"workspace-path","path":<exact approved root-relative file>}`
or `{"kind":"server-artifact"}`. A path declaration must be within the
assignment's effective `readPaths` and approved output scope; a worker that
creates it also needs the corresponding effective write permission. Reject a
configuration that cannot authorize the required read. The root is the child's
allocated plan-owned worktree, or the root's approved `sourceWorktreeBinding`.
Use #741's descendant-path rules; the reporter cannot select another path or
root. A file's existence beneath a worktree never authorizes reading it.

All declarations are required to pass. The configuration also declares a finite
total rehash-work cap. Reject declarations above server hard limits. Exact hard
limits and this declaration schema must be reconciled with #741 before code.
`workspace_changes` and `verification_report` may be declared kinds; v1 path
sources are regular files, not directory trees. A server artifact is created
under the exact producing assignment and declaration ID/kind, with an immutable
ID/version, size and digest. Another assignment's artifact cannot satisfy the
declaration. Cross-assignment artifact reuse needs a separately approved source
contract and is unsupported in v1.

A child assignment producing an evaluation declares an `assignment_evaluation`
server-artifact output and the exact target result as an approved input. Its
review capability binds that output declaration as well as the target result.
Accepting an evaluation atomically publishes its immutable canonical record as
an artifact owned by the reviewer assignment/declaration and returns the
artifact ID/version, byte size and content digest. Artifact versions are the
evaluation revisions for that stream; one declaration binds one target result
and reviewer stream. The reviewer includes the selected immutable artifact
version in its own candidate manifest. Publication is a scoped reporting action,
not a product-file write, and requires no shell or worktree write capability.
The artifact content is the canonical `AssignmentEvaluation` object below;
its content digest is SHA-256 of those bytes without a domain prefix. Its
evaluation digest uses the evaluation domain. Both values are verified.
User-authored evaluations are server records and do not impersonate a child
output. A bare #742 research report cannot satisfy a result output declaration
until #742 supplies the assignment/declaration-bound immutable artifact wrapper.
Reports may instead be evidence under the source contract below.

### Candidate submission and sidecar finalization

The live producer submits one immutable `AssignmentResultCandidate` per authority
generation through a distinct `AssignmentResultCapability`. The sidecar binds
that capability to the approved assignment and active child-launch or
parent-runtime generation. It grants only candidate submission and receipt
reads, not evaluation, research reporting, orchestration, or filesystem access.
Submission supplies one present/missing output per declaration and the expected
accepted-result predecessor. Identity and generation come from the capability;
body values must match. The sidecar validates declarations, approved sources,
finite caps and server-artifact ownership before persisting the candidate.
An accepted candidate is a proposal to snapshot these exact bytes, not an
accepted result or a claim that the bytes have been inspected.

Resolve an exact retry after authenticating the current capability/generation
and before predecessor checks. A changed payload under the same key conflicts.
A second candidate in the same generation conflicts unless it is an exact
retry. Retry keys are scoped to reporter, assignment, authority generation and
operation. Compare-and-swap the expected accepted-result predecessor before
storing the candidate. At most 32 candidate submissions may be accepted across
all resumes of an assignment, including candidates later rejected or superseded;
this bounds their records and receipts as well as the 32-result-revision limit.
Exhaustion never wraps or resets. Validation failures do not create a candidate.

Producer termination revokes its capability, following #742; no dead process
must submit a final result and no post-exit producer grant survives. The durable
candidate remains pending for **sidecar finalization**. For a path-backed
candidate, after observing the exact producer's terminal state, revoking its
write authority and recording the #610/#742 user quiescence acknowledgement,
the sidecar snapshots path outputs and verifies server outputs. An artifact-only
candidate may finalize while the producer remains live because all its output
versions are already immutable; it requires no worktree read or extra quiescence
action. In either case the sidecar derives the available time/cost records. It rechecks the candidate's predecessor and atomically publishes the
verified snapshots, accepted result revision/digest, dependency edges and
finalization receipt. The receipt names the candidate digest. Repeating
finalization resolves that receipt without creating another revision. The
sidecar performs path-backed finalization under the recorded user-authorized
quiescence transition; artifact-only finalization follows the authenticated
candidate submission. Neither needs a further producer request or impersonates
the producer. A rejection is also recorded as a durable finalization outcome,
so retries do not repeat failed work or create a result revision.

A resume before acceptance supersedes the pending old-generation candidate and
invalidates its quiescence evidence; the new live runtime needs a fresh grant,
a new candidate and fresh acknowledgement. Restoration of write authority,
changed bytes, stale predecessors or unverifiable bindings rejects finalization
without changing the accepted result head. There is no automatic producer retry
or relaunch. A separately authorized resume or revised assignment is needed to
produce a replacement. A restarted sidecar may finalize a durable candidate
only after freshly reconciling its approved identity, exact terminal producer,
write revocation and user acknowledgement; missing recovery evidence blocks
finalization and never restores producer authority. Run completion does not
discard a pending candidate. A subject is not closed until its pending candidate
has an accepted/rejected finalization outcome, been superseded, or been
explicitly abandoned through the
Electron-authorized user path.

### Snapshot admission and freshness

The sidecar derives the root from the approved allocation/source-worktree
binding and revalidates that binding before each snapshot operation; a reporter
cannot supply the root. Open each declared file with one atomic descriptor-
relative operation from a retained handle to that exact approved root. Before
opening, validate
the declared path against the effective read scope and output declaration.
Enforce beneath-root containment and no-follow semantics for every component;
reject symlinks, junctions and reparse points. Never resolve a name and then
open it by name or compose separate path checks and opens. Preserve the opened
handle for type, size and digest checks. If the platform cannot provide this
operation, reject the path source before reading any bytes.

Only regular files with no hard-link aliases are supported. Prove a link count
of one from the opened handle before reading, after copying and before admission.
If a required proof is unavailable or changes, discard the snapshot. While the
producer remains quiescent, copy each bounded output into an immutable
assignment/declaration-bound object; verify both source and completed object
against the candidate size/digest. Stage copying outside the dependency
transaction and publish the verified snapshot with the result atomically.
Every potentially blocking filesystem operation runs in an isolated worker
with fixed finite concurrency, bounded admission and a deadline. Control and
lifecycle requests never wait beyond that deadline. A stuck worker remains
charged until it exits or the sidecar restarts; do not spawn replacements or
queue unbounded work around it. Exact worker/queue/deadline limits are fixed
implementation-plan decisions. Nonblocking flags do not prove bounded latency.

A declared output verified absent at finalization is known missing and establishes
Needs rework. Publish it as a missing accepted output, including when the
candidate claimed presence; retain the candidate digest as provenance of the
rejected presence claim. An inaccessible, changed, oversized, timed-out or unprovably bound
output is unsupported: do not publish an accepted snapshot or infer missingness;
show Unassessed for the pending/rejected candidate unless an independent failure
is established. A `missing` entry is confirmed against the declared path or the server
assignment/declaration artifact index before it establishes failure; a producer's
assertion alone is not proof. If the claimed-missing source exists, reject the
candidate rather than silently selecting its bytes.
No out-of-scope bytes may be read or pinned.

Accepted snapshots are authoritative. Freshness checks verify those immutable
objects and the accepted result head, never the mutable worktree path. Preserve
path/size/digest provenance in the manifest. A failed immutable-object check makes
the supported output/outcomes Unassessed; it does not silently rebind evidence to
new bytes. Later worktree edits, equivalent replacements and authorized cleanup
cannot change an accepted result or manufacture Needs rework. New output needs a
new result revision. Result-head publication is atomic; stale finalization cannot
restore an earlier head. This establishes content freshness, not authorship or
OS confinement.

Before a dependent successor gains writable ownership under #610, confirm all
retained path outputs have verified immutable snapshots. Snapshot publication,
dependency admission/removal, retention, cleanup and ownership transfer use the
dependency boundary below. Scan before durable ownership transfer or successor
write access; unresolved required snapshots block handoff. Snapshots count
against artifact/workspace quotas and retention rules.

### Descriptive time and cost evidence

The accepted result always has a `timeCostEvidence` array, empty when nothing is
available, with at most 32 sidecar-resolved records. Aligning with #745's proposed
units, a record is exactly one of:

- `{"kind":"elapsed","elapsedMilliseconds":<integer 0..604800000>,"authorityGeneration":<the result's tagged generation>}`.
- `{"kind":"cost","costMicroUsd":<integer 0..1000000000000>,"providerId":<ID>,"modelId":<ID>,"pricingSnapshotId":<ID>,"usageRecordId":<ID>,"usageDigest":<Digest>}`.

There is at most one elapsed record, measured with a monotonic clock from that
generation's accepted start to terminal event. An unavailable/cross-restart or
over-limit duration is omitted, never estimated from timestamps. A cost source
must be an authenticated immutable provider usage record reporting USD and
binding the provider, model and pricing snapshot. Preserve its ID/digest; reject
duplicate usage sources, estimates and unverifiable or unconvertible amounts.
Cost remains absent until the implementation plan identifies and approves that
usage source; #742 currently supplies no provider-billing contract. No new usage
collection authority is implied. Each record is at most 512 bytes. Integers are
lossless, nonnegative and never encoded as floats. Sort by UTF-8 `kind`, then
`usageRecordId` for cost; an elapsed record has no secondary sort key.

The complete array participates in the result digest and 512 KiB result cap.
Missing evidence is absence, never zero. Metrics remain alongside both scores,
do not affect completeness/quality, and are compared only under #745's matching
provider/model/pricing/cohort rules. #745 must reconcile its now-stale exclusion
of #744 metrics before consuming them; this spec does not enable shared learning.

## Criteria, completeness, and quality

Each criterion has a stable ID, description, and `required` or `optional`
status. New assignments require at least one required criterion. Criteria and
classification are immutable after launch. Optional results are details only and
do not affect completeness or the overall result.

An evaluation provides exactly one `satisfied`, `unsatisfied`, or `unassessed`
outcome for every approved criterion, with a bounded rationale. Satisfied and
unsatisfied outcomes require evidence; an unassessed outcome may omit evidence
and records why it could not be assessed. In the evaluation record, each
`EvidenceReference` is one sidecar-derived `sourceKey`, a 64-character lowercase
SHA-256 hex value. Bare content digests and caller-chosen object IDs are invalid.
A source key is SHA-256 over `orkworks.assignment-evidence-source.v1\n` (one
literal LF) followed by #741 canonical JSON for the exact `EvidenceSource`
object defined below. Assignment and report source descriptors have complete
local schemas; no nonexistent tagged identity is imported from #741/#742.
The sidecar derives and resolves keys from stored records. Identical bytes or
reused IDs in different assignments remain distinct; a collision with a
different descriptor is rejected. A source must belong to the same workspace;
v1 does not admit cross-workspace result/evaluation references.

A report descriptor resolves an immutable #742 report and must match its stored
identity, size and content digest. A corrected report gets a new source key.
Report evidence is current only while its exact immutable version is the
current correction head. #742 must define expected-version/digest CAS,
exact-retry resolution before head checks, and atomic correction publication;
arrival time or numeric version alone cannot select that head. Until that
contract is approved and reconciled, research-report evidence cannot establish
a current #744 outcome. #744 adds no independent report invalidation authority;
withdrawal without replacement remains unsupported until #742 specifies it.
These report-source gates are distinct from evaluation invalidation below.
Missing, duplicate, or unknown criterion IDs make the report malformed and it is
rejected.

Completeness is the percentage of required criteria satisfied, but only when
every required criterion has been assessed. Otherwise show the
satisfied/unsatisfied/unassessed counts and leave the percentage Unassessed.
Never shrink the denominator or calculate `0 / 0`. A known, uncontested
unsatisfied required criterion remains a failure even if other required
criteria are unassessed.
Legacy assignments without required criteria cannot pass; absent other known
failures, they are Unassessed.

#741's `CriterionSnapshot` has no separate version field. The sidecar derives
`criteriaSnapshotDigest` as lowercase SHA-256 over
`orkworks.criteria-snapshot.v1\n` followed by the canonical JSON bytes of the
approved `CriterionSnapshot[]`, ordered by stable ID and containing each
criterion's `id`, `requirement`, and `description`. This binds the exact
criteria used by child and root assignment variants without inventing a
reviewer-supplied version. It is immutable assignment context and is included
in each evaluation digest.

The approved, versioned, role-specific rubric has an ID, version, evaluator
role, and 1–16 required quality dimensions with stable IDs. The sidecar derives
`rubricSnapshotDigest` as lowercase SHA-256 over `orkworks.rubric-snapshot.v1\n`
followed by #741's canonical JSON bytes for the exact approved `RubricSnapshot`
(`id`, `version`, `dimensions`, and `evaluatorRole`). It is immutable
assignment context, not reviewer input, and is included in each evaluation's
immutable identity/digest. New assignments cannot use an empty rubric; a legacy
assignment with no dimensions cannot pass; absent another established failure,
it remains Unassessed. For each dimension the reviewer assigns `meets`,
`below standard`, or `unassessed`, plus one
result-level rating. `Meets` and `below standard` outcomes and an assessed
result-level rating require evidence; an unassessed dimension or rating may
omit evidence and records a bounded reason. An unassessed dimension or rating
makes quality Unassessed; an assessed dimension with missing evidence also
makes quality Unassessed. An evidenced, current rating below `3` or a
below-standard dimension establishes a quality failure even if another
dimension is unassessed. Dimension scores are not averaged.
Comparisons across assignments require the same rubric ID, version, and
`rubricSnapshotDigest`, unless an explicit, versioned normalization rule is
approved. Matching labels or reusing an ID/version does not make different
dimension descriptions or evaluator roles equivalent. Configuration learning
must keep different snapshot digests in separate cohorts unless that approved
normalization applies.

| Rating | Meaning |
| --- | --- |
| `0` | Unusable result |
| `1` | Major rework required |
| `2` | Limited rework required |
| `3` | Meets the declared quality standard |
| `unassessed` | Evidence or an eligible reviewer is missing, stale, or disputed |

The meanings above are fixed for version 1. A current, uncontested rating
below 3 establishes Needs rework even when credible reviewers disagree on the
exact rating. Conflicting ratings that all establish below 3 preserve their
individual values and still establish that failure; disagreement across the
threshold (for example, 2 versus 3) makes quality Unassessed unless another
uncontested failure exists. Rating 3 meets the quality part of the result only
with current, uncontested evidence. Before using an outcome, the sidecar
validates every evidence reference supporting a criterion outcome, quality
dimension or rating, finding, or dispute for existence, scope, version, digest,
and, for report evidence, current source correction state under #742.
Unavailable, changed, or unverifiable evidence makes the supported outcome
Unassessed. A stale required-rework finding is itself unassessed and cannot
establish failure; derive the overall result from the remaining current
outcomes. A dispute whose evidence is not current is ineffective and cannot
suppress a current finding.
A transcript, task status, completion claim, test command string, or self-rating
is not sufficient evidence by itself. Quality does not rank agents, grant XP,
or prove a skill caused an outcome.

Derive one overall result from current evidence:

| Result | Rule |
| --- | --- |
| **Needs rework** | A current, uncontested required criterion is unsatisfied, quality is below `3`, a required output is known missing, or a current, uncontested required-rework finding has a valid failing impact target. |
| **Unassessed** | No failure is established, but a required criterion or quality is unassessed, the result is stale, no eligible reviewer exists, or relevant credible evidence conflicts. |
| **Meets requirements** | Every declared output is present/verified, every required criterion is satisfied, quality is `3`, evidence is current, and no declared relevant conflict or invalidation remains. |

A disputed failure is not established; report Unassessed unless another
uncontested failure exists. Blocked, cancelled, interrupted, unsupported, or
partial work keeps its lifecycle status: assess available evidence, and leave
the rest unassessed. A blocker explains missing work but does not satisfy a
criterion.

Across multiple eligible current evaluations, an `unassessed` required
criterion, quality dimension, or rating is not an abstention: it blocks a pass
even when another evaluation supplies an assessed outcome. With no established
failure, the aggregate is Unassessed. A current, uncontested failure still
establishes Needs rework despite other unassessed outcomes. Conflicting assessed
outcomes follow the disagreement rule above; a success versus unassessed is
Unassessed, while an unassessed opinion cannot erase an otherwise uncontested
failure.

An evaluation record stores criterion/dimension outcomes, quality, findings,
required rework, evidence, reviewer/source identity and the sidecar-derived
`rubricSnapshotDigest`. Observation time and retry metadata accompany the
record outside its canonical object; the server derives the overall result. Each finding has a
stable ID, concise description, location, severity, evidence reference, and
whether it requires rework, matching the approved review-role output contract.
A required-rework finding also names one or more unique `requiredImpactTargets`:
an approved required criterion, rubric dimension, or result-level rating. A
target has the exact tagged form `{"kind":"criterion","id":<stable ID>}`,
`{"kind":"quality-dimension","id":<stable ID>}`, or
`{"kind":"rating"}`. The array is a set serialized in ascending UTF-8 byte
order by `(kind, id)`, with an absent rating ID treated as the empty string.
Each target must have a corresponding current failure in that evaluation
(`unsatisfied`, `below standard`, or rating below `3`); optional criteria are
never valid targets. To derive Needs rework, the same target must also be an
uncontested aggregate failure across eligible current evaluations. If assessed
outcomes conflict for a target, a finding pointing to it cannot override that
conflict; the aggregate remains Unassessed unless another uncontested failure
establishes Needs rework. There are at most 49 targets, the maximum combined
set of 32 criteria, 16 dimensions, and one rating. Reject a required-rework
finding with no valid target. Optional-only findings cannot affect the overall
result.
Rationale, findings, and corrections must not contain credentials, secrets,
hidden reasoning, full prompts, or complete transcripts; use safe, immutable
evidence references.

The result manifest preserves available elapsed-time and provider-cost
evidence under the bounded `timeCostEvidence` contract above. #745 may consume
these exact values and provenance but cannot redefine their units or promote
missing evidence into zero or an estimate. Cost remains absent until an authenticated usage-record source is approved,
and #745 consumption remains subject to the shared-learning gate below.

## Reviewer eligibility and disagreement

A child-result review comes from a declared `review` or `verification`
assignment with an eligible rubric role, or from explicit user review. Root
results use user review only. Worker self-assessment stays separate and cannot
affect the result. #740 currently has no verified child-review profile; child
reports remain ineligible until version-specific evidence and #741's approved
profile establish support. Until then, runtime review uses the user path.

A child reviewer must have a different task, allocation, session and configuration
from the worker, be outside the potential-contributor set derived from approved
allocations/effective write scopes, and have approved read access to the exact
result without write access to its output scope. The sidecar establishes these
facts; if it cannot, the report is ineligible pending user review. Parent
synthesis alone is not independent review. These are assignment-level controls,
not OS isolation.

The reviewer-scoped `AssignmentEvaluationCapability` binds the reviewer
assignment/configuration, active sidecar/launch generation, exact target result
revision/digest, and its declared evaluation output. It permits evaluation
submission and receipt/artifact reads only; it grants neither product writes nor
worker-result submission. Follow #742's generation, revocation and retry rules.
Accepting a child evaluation and publishing its declared immutable output are
one atomic reporting action. User reviews use Electron authorization and user
provenance. Resolve adapter transport and #741/#742 record alignment before code.

Each evaluation stream has a server-resolved `reviewerIdentity`: the exact child
reviewer assignment or a UI-issued `userReviewId`. It is stable for that stream,
not arbitrary display text. A user correction to child review starts a separate
user stream; it never impersonates or changes the child's report.

**Reviewer performance assessment is optional and user-provided.** A user may
evaluate a reviewer's declared result, with `reviewedEvaluationRef` naming the
exact evaluation artifact inspected. Without that user assessment, reviewer
performance is Unassessed. Do not assign a child reviewer to evaluate another
reviewer's evaluation; reject such a child capability or submission. Performance outcomes, missing assessments, corrections
and invalidations of those assessments do not grant/revoke the original review's
eligibility or affect its worker target. There is no reviewer-certification
chain. Original review eligibility follows the independent-role, provenance,
current-result and evidence rules above. The user may separately correct or
invalidate the original review through the normal user path. This implements
the parent design's optional performance disposition without recursive review.

Keep eligible current evaluations separate. Aggregate assessed conflicts by
approved required criterion, quality-dimension ID and result-level rating; never
average or prefer time/seniority. An unassessed required outcome blocks a pass
but cannot erase an otherwise uncontested failure. Optional-only conflicts do
not affect the overall result.

Material-fact conflicts must be **declared** by a reviewer/user, not inferred from
matching prose or finding IDs. Represent a material uncertainty as `unassessed`
on each affected criterion/dimension/rating, with rationale and available source
evidence; contest an existing finding with `findingDisputes`. That makes the
conflict visible and invokes the aggregate rules. A caller who identifies a
material contradiction must not simultaneously assert that its affected target
is satisfied/meets without resolving it. The sidecar validates declared targets
and references, not natural-language truth. No automatic semantic claim detector
or shared cross-review finding-ID namespace is promised.

Each revision allows up to 32 finding disputes. A dispute names exactly the
other evaluation's `(reviewerIdentity, evaluationRevision, findingId)` and one
source key already cited in the disputing evaluation. Targets must be unique,
eligible, current evaluations for the same assignment/result revision. Finding
IDs are unique only within one stream/revision. Reusing an ID in a correction
means the same logical finding; a materially different finding gets a new ID.
Disputes bind exact revisions and do not carry to corrections. Without a declared
dispute, findings remain separate; shared required-impact targets still aggregate
through their criterion/dimension/rating outcomes. A current dispute of a
required-rework finding makes its affected required targets disputed and
Unassessed unless another independent, uncontested failure establishes Needs
rework. An informational/optional-only dispute changes detail only.

A dispute is effective only while the target revision and both its finding and
dispute evidence remain current/verifiable. Stale dispute evidence cannot
suppress a current finding; stale finding evidence cannot establish failure.
The user resolves remaining material conflict by a new/corrected user evaluation
or explicit invalidation of the conflicting original stream, preserving history.

## Revisions, correction, and invalidation

Evaluations and corrections are append-only. A correction is a new revision by
the same eligible reviewer while that review capability is active; a different
reviewer creates a separate evaluation. An authenticated idempotency key is
scoped to reporter, subject, and operation. Each submission carries its expected
evaluation revision and digest. A new `reviewerIdentity` stream uses an explicit
no-head revision with no digest; compare-and-swap that state before creating
revision 1. Corrections require the current revision and digest. Resolve an
exact prior receipt before checking the expected revision; changed content under
the same key conflicts. Stale or cross-subject writes fail closed. Every invalidation,
regardless of child-capability state, requires Electron-authorized user
provenance; child reviewers cannot invalidate their own or another report.
Invalidation names the current evaluation revision and digest; it is serialized
with corrections, uses the same idempotency rules, and freezes that evaluation
stream once accepted. A stale invalidation conflicts and cannot exclude a newer
correction.

Each accepted invalidation creates one immutable `AssignmentEvaluationDisposition`
record for the exact assignment identity, result revision, evaluation stream
identity (child `reviewerIdentity` or user `userReviewId`), and current
evaluation revision/digest. Its semantic action is `invalidated`, its actor is
`user` under Electron authorization, and its disposition revision is 1 with no
predecessor. There is at most one disposition per evaluation stream; it cannot
be corrected or superseded, and its acceptance permanently freezes that stream.
The request idempotency key and bearer are not record fields. An exact retry
returns the saved receipt before current-head checks; a different request or a
stale target conflicts.

For a user-authored evaluation stream, the Electron-authorized path may append
a correction to the same `userReviewId` stream using compare-and-swap on the
current evaluation revision and digest. The correction keeps user provenance,
uses the same idempotency rules, and cannot alter a child-authored stream. A
stale expected revision conflicts; an invalidated stream is frozen and cannot
be corrected. After invalidation, a replacement stream may start with a new
`userReviewId` and no-head compare-and-swap while user revision capacity
remains. Further replacements follow the same rule until that budget is used.
For each result revision, the sidecar serializes user-stream creation through a
single active-user-stream slot. Initial creation compare-and-swaps `no-head`;
replacement creation compare-and-swaps the exact invalidated `userReviewId`
and its disposition digest. Slot advancement, first evaluation revision, and
revision-budget consumption are atomic. Concurrent creations from the same
slot have one winner; losers conflict, while an exact retry returns the
winner's saved receipt. A stream cannot become active unless its predecessor
has an accepted invalidation.

When #742 ends a run and revokes child authority, that child can no longer
correct its review. The user may submit a separate user-authorized evaluation;
the user never impersonates the child. Invalidation preserves the report and
provenance and excludes it from current results and learning. Freshness and
eligibility changes affect derived projections, never immutable outcome fields
or digests. A later result
revision makes earlier evaluations historical; they never become current
again. Before using an evaluation as current evidence, revalidate each
referenced report's current correction state under #742 as well as its
existence, scope, version, and digest. A corrected source makes
only the criterion, dimension, rating, finding, or dispute outcome that cites
that source stale: stale cited evidence makes that outcome Unassessed, and a
stale dispute is ineffective. Other outcomes remain current when all of their
own evidence and identity bindings are current. Never silently rebind an old
reference to a corrected source; a reviewer must submit a new evaluation to
change the affected outcome. Recompute the overall result from the remaining
current outcomes. When a corrected source supports only an optional criterion
or informational finding, that detail becomes stale but cannot change
completeness or the overall result. If it also supports a required criterion or
quality outcome, only those affected outcomes become Unassessed, subject to
known-failure precedence. Learning consumes only current outcome evidence under
#745; stale outcomes are ineligible without discarding unrelated current
outcomes from the same evaluation. Optional user assessment of reviewer
performance follows the same freshness rules for its own outcomes; it never
changes the original review's eligibility.

## Canonical records

These are normative version 1 logical JSON objects, not route definitions.
Every named field is present; `null` is used only where explicitly allowed.
Reject additional properties, duplicate JSON keys, unknown tags, duplicate IDs,
invalid references and values outside the stated bounds before persistence.
`ID` means #741's nonempty ASCII `[A-Za-z0-9._-]`, at most 128 bytes; `Digest`
is exactly 64 lowercase SHA-256 hex characters. Revisions/counters are plain
lossless decimal JSON integers without signs, leading zeros, fractions or
exponents, never strings or rounded JavaScript Numbers. `u64` is 0..2^64-1;
positive artifact versions are 1..2^64-1. Result revisions are 1..32, evaluation
revisions 1..16, plan revisions 1..64 and task versions 1..2^31-1. Runtime
generation values use #742's opaque 64-character lowercase hexadecimal encoding,
not numeric counters. `Text` is UTF-8, bounded to 2 KiB; outcome rationales are
nonempty. All records also obey the aggregate byte caps below.

### Identities and references

| Type | Exact properties / variants |
| --- | --- |
| `AssignmentIdentity` child | `kind: "child"`, `workspaceId: ID`, `runId: ID`, `planId: ID`, `planRevision`, `taskId: ID`, `taskVersion`, `reservationId: ID`, `parentSessionId: ID`, `childSessionId: ID`, `configurationId: ID`, `configurationDigest: Digest` |
| `AssignmentIdentity` root | `kind: "root"`, `workspaceId: ID`, `runId: ID`, `rootAssignmentId: ID`, `parentSessionId: ID`, `bootstrapId: ID`, `bootstrapConfigurationDigest: Digest` |
| `AuthorityGeneration` | `kind: "child-launch"` or `"parent-runtime"`, `sidecarGeneration: Digest`, `generation: Digest`; tag must match the assignment variant |
| `ResultPredecessor` | `{"kind":"no-head"}` or `{"kind":"revision","resultRevision":<revision>,"resultDigest":<Digest>}` |
| `EvaluationPredecessor` | `{"kind":"no-head"}` or `{"kind":"revision","evaluationRevision":<revision>,"evaluationDigest":<Digest>}` |
| `ReviewerIdentity` | `{"kind":"child","assignmentIdentity":<child AssignmentIdentity>}` or `{"kind":"user","userReviewId":<ID>}` |
| `ReporterSource` | `{"kind":"child","authorityGeneration":<child-launch AuthorityGeneration>}` or `{"kind":"user"}`; must agree with authenticated reviewer identity |
| `EvaluationReference` | `assignmentIdentity`, `resultRevision`, `resultDigest`, `reviewerIdentity`, `evaluationRevision`, `evaluationDigest`, with the types above |
| `SourceVariant` | `{"kind":"workspace-path","path":<approved exact root-relative path>}` or `{"kind":"server-artifact","artifactId":<ID>,"version":<positive artifact version>}` |
| `RequiredImpactTarget` | `{"kind":"criterion","id":<required criterion ID>}`, `{"kind":"quality-dimension","id":<dimension ID>}`, or `{"kind":"rating"}` |

Child identity values come from the approved assignment/allocation and #742's
runtime records; root values come from the approved bootstrap/run. The sidecar
copies values into these local wrappers without adding fields to #741/#742
source records. Unknown/absent identity members cannot be inferred. Reconcile
these exact mappings with those owners before implementation; the canonical
schemas themselves are fully defined here.

### Candidate, result and evidence sources

`OutputEntry` is exactly one of:

- `{"declarationId":<ID>,"declarationKind":<ID>,"state":"missing"}`.
- `{"declarationId":<ID>,"declarationKind":<ID>,"state":"present","source":<SourceVariant>,"sizeBytes":<u64>,"contentDigest":<Digest>}`.

A present entry's source kind/path must match its approved declaration; size
must fit its cap. Missing entries acquire known-missing status only through the
sidecar's finalization checks. A nonexistent server artifact cannot be a present
entry. Output arrays have exactly one entry per approved declaration, sorted by
UTF-8 declaration ID. Each accepted output resolves an immutable
assignment/result/declaration-bound object; physical pin location is metadata,
not a semantic record field.

`AssignmentResultCandidate` has exactly `schemaVersion: 1`,
`assignmentIdentity: AssignmentIdentity`, `predecessor: ResultPredecessor`,
`authorityGeneration: AuthorityGeneration`, and `outputs: OutputEntry[]`.
`AssignmentResult` has exactly those properties plus `resultRevision`,
`candidateDigest: Digest`, and `timeCostEvidence`, whose complete record shapes
and order are defined above. Candidate identity/generation/predecessor/entries
are preserved, except a sidecar-confirmed absence changes a claimed-present
entry to `missing`. No present output may be replaced with different bytes. A failed candidate does
not acquire an accepted result revision.

`EvidenceSource` is exactly one of:

- Output: `schemaVersion: 1`, `sourceKind: "assignment-result-output"`,
  `assignmentIdentity: AssignmentIdentity`, `resultRevision`, `resultDigest`,
  `declarationId: ID`, `declarationKind: ID`, `source: SourceVariant`,
  `sizeBytes: u64`, `contentDigest: Digest`.
- Report: `schemaVersion: 1`, `sourceKind: "research-report"`,
  `reportIdentity: ReportIdentity`, `sizeBytes: u64`, `contentDigest: Digest`.

`ReportIdentity` has exactly `workspaceId: ID`, `runId: ID`, `planId: ID`,
`planRevision`, `taskId: ID`, `reservationId: ID`, `childSessionId: ID`,
`configurationDigest: Digest`, `sidecarGeneration: Digest`,
`launchGeneration: Digest`, `reportId: ID`, and `reportVersion` (integer 1..8).
Workspace identity is sidecar-derived; the remaining values map to #742's
immutable report descriptor. The descriptor's content digest must equal the
source `contentDigest`. This local source-kind tag discriminates the identity;
it does not presume an upstream tagged report schema. Report freshness remains
gated on #742's correction-head contract.

### Evaluation and disposition

An `Outcome` has exactly `outcome`, `rationale: Text`, and
`evidenceReferences: Digest[]`. `CriterionOutcome` adds `criterionId: ID` and
uses `satisfied | unsatisfied | unassessed`. `QualityDimensionOutcome` adds
`dimensionId: ID` and uses `meets | below-standard | unassessed`.
`RatingOutcome` has exactly `rating` (integer 0..3 or string `unassessed`),
`rationale: Text`, and `evidenceReferences: Digest[]`. Assessed outcomes require
at least one current evidence reference; unassessed outcomes may have none.
Each array of evidence references is a unique set sorted by UTF-8 source key.
There is no separate nested `Outcome` property in the criterion/dimension
objects; the listed properties are directly on each object.

A `Finding` has exactly `id: ID`, `description: Text`, `location: Text`,
`severity: ID`, `evidenceReference: Digest`, `requiresRework: boolean`, and
`requiredImpactTargets: RequiredImpactTarget[]`. Description/location together
are at most 2 KiB. Severity is a descriptive label; it does not itself
determine pass/failure. The impact array is empty for an
informational finding; a required-rework finding has 1..49 unique valid failing
targets under the rules above. Sort targets by UTF-8 `(kind, id)`, with absent
rating ID treated as empty. `FindingDispute` has exactly `target` and
`evidenceReference: Digest`; `target` has exactly `reviewerIdentity`,
`evaluationRevision`, and `findingId: ID`. The source key must also be cited in
an outcome, rating or finding of that disputing evaluation.

`AssignmentEvaluation` has exactly `schemaVersion: 1`, `assignmentIdentity`,
`resultRevision`, `resultDigest`, `reviewerIdentity`, `reporterSource`,
`evaluationRevision`, `predecessor: EvaluationPredecessor`,
`criteriaSnapshotDigest: Digest`, `rubricSnapshotDigest: Digest`,
`criterionOutcomes: CriterionOutcome[]`,
`qualityDimensionOutcomes: QualityDimensionOutcome[]`, `rating: RatingOutcome`,
`findings: Finding[]`, `findingDisputes: FindingDispute[]`,
`correctionReason: Text | null`, and
`reviewedEvaluationRef: EvaluationReference | null`.
Types of previously named identities/revisions/digests are those above. Initial
revisions have a no-head predecessor and null correction reason; corrections
have the exact preceding revision/digest and a nonempty reason. The performance
reference is nonnull only for optional user-authored assessment of a reviewer's
result; it must resolve an evaluation artifact in that exact result. It is never
an eligibility reference. Other evaluations use null.

Criterion/dimension arrays cover exactly the approved IDs and sort by those
UTF-8 IDs; findings sort by ID. Disputes sort by canonical JSON bytes of their
unique target object. All collections are present, including empty arrays.
Canonical quality outcome tags are `below-standard`; prose “below standard”
means that tag. Observation time and transport receipts are separate metadata.

`AssignmentEvaluationDisposition` has exactly `schemaVersion: 1`,
`assignmentIdentity`, `resultRevision`, `reviewerIdentity`,
`evaluationRevision`, `evaluationDigest`, `dispositionRevision: 1`,
`action: "invalidated"`, and `actor: "user"`. It has no predecessor. The target
result/stream/revision resolves the immutable evaluated result and current
stream head; no caller-selected actor is accepted.

## Record digests

Apply #741's recursive canonical serializer: object keys sorted by UTF-8 bytes,
validated array order retained, compact deterministic UTF-8 JSON escaping, and
lossless decimal integers. Hash the **entire exact logical object** above with
SHA-256 after the corresponding UTF-8 prefix (each ends in one literal LF):

| Object | Domain prefix |
| --- | --- |
| `AssignmentResultCandidate` | `orkworks.assignment-result-candidate.v1\n` |
| `AssignmentResult` | `orkworks.assignment-result.v1\n` |
| `AssignmentEvaluation` | `orkworks.assignment-evaluation.v1\n` |
| `AssignmentEvaluationDisposition` | `orkworks.assignment-disposition.v1\n` |
| `EvidenceSource` | `orkworks.assignment-evidence-source.v1\n` |

There is no separately constructed digest-input inventory. In particular the
accepted result hashes its complete `timeCostEvidence`, and evaluation hashes
all nested outcomes/rationales/evidence, findings/targets/disputes, correction
reason, and nullable performance reference. Digests themselves are stored as
metadata outside the hashed object. Overall result, completeness percentage,
physical pin location, eligibility projections, subject revision, observation
time, credentials, request idempotency keys and receipt metadata are excluded
because they are not object properties. No unlisted semantic field affects a
v1 digest; changing the shape requires a new domain version. The approved
criteria/rubric snapshot digest algorithms remain those defined above.
The implementation plan must supply shared cross-language byte/hash fixtures
for every variant, nesting, null, order and maximum integer; canonicalization
cannot compensate for a missing field schema.

## Bounds, retention, and deletion

Use #741's 32-criterion and 16-dimension limits. Each assignment has at most
32 accepted candidates across resumes, 32 accepted result revisions and 32
output declarations. Each result revision allows at most 16 evaluation
revisions: child streams share 12 and user streams share 4, including
corrections/replacements. Each user stream allows at most 2 revisions. A
replacement starts only after invalidation and while user capacity remains;
child streams cannot consume it. The assignment-wide evaluation cap is 512,
allowing each of 32 accepted results an initial child and user evaluation.
Each evaluation has at most 32 findings, 32 disputes and 113 evidence-reference
occurrences across its outcomes/rating/findings/disputes. Each reference is a
64-character source key and fits the 1 KiB reference cap. Candidate, result,
evaluation and disposition records are each at most 512 KiB; each
rationale/finding/correction text is at most 2 KiB. Per-output bytes and aggregate
rehash work obey finite approved caps bounded by server limits.

Enforce finite aggregate run/workspace admission quotas. Reconcile values with
#741/#745 before code; include staged candidates and immutable reviewer artifacts
in their producing assignment's quota. Reject exhausted capacity visibly without
silently dropping conflicts, corrections or provenance. No automatic eviction
makes room for new reports.

### Local dependency boundary and subject purge

The sidecar maintains an authoritative incoming-dependency index for retained
records, current selections and approved learning inputs. Publishing/removing
an edge is atomic with its owning record and snapshot publication. Edge changes
and the final scan/action for retention, explicit purge, workspace deletion,
worktree cleanup and ownership transfer serialize through the workspace
dependency boundary. Hold it from the final scan through deletion, sealing,
cleanup or durable ownership transfer. A reference either commits first and
protects its target, or follows the operation and resolves a retained immutable
target or fails. Subject revision alone cannot fence another subject's edges.
Keep copying and unrelated ordinary writes outside this narrow boundary.
The implementation plan defines index recovery and transactional seams before
code; no learning input may depend on mutable worktree bytes.

Ordinary retention belongs to #745 and may remove only a complete historical
subject whose results cannot be current/learning input and whose records/evidence
have no incoming retained dependency. Retain every source needed by any current
evaluation regardless of outcome, including optional user performance records;
those records protect only their exact references and do not create credibility
chains. Apply the same incoming-learning guard to retention and explicit purge.

Explicit Electron-authorized purge may remove a closed subject's own current
result selection and history only when no other retained record, consumer
selection or learning input depends on it. A subject is closed after its run is
terminal, write authority and reporting grants are revoked, and all pending
candidates have accepted/rejected finalization outcomes, are superseded, or
are explicitly abandoned. Purge compares the exact
identity and caller-observed `subjectRevision` under the boundary, then atomically
clears its selection and deletes its candidates/manifests/receipts, evaluations,
dispositions and solely referenced pinned evidence; otherwise delete nothing.
`subjectRevision` is sidecar-issued checked u64, advanced with candidate
admission/finalization/supersession/abandonment, accepted result/evaluation writes,
correction, invalidation, user-slot change and retained-edge additions/removals.
Overflow visibly blocks further mutations and purge; it never wraps. The token
is metadata, excluded from record digests and removed by purge. An absent target
returns the same not-found/no-op result whether previously purged or nonexistent.
No tombstone remains and no deleted history can be recreated by stale reporting.

### Workspace deletion and repository learning gate

Workspace deletion is **not** an unconditional purge of protected evidence.
It first durably fences new/in-flight candidate, finalization, evaluation and
dependency writes under the workspace lease. Follow #745's
`pendingWorkspaceDeletion`, all-subject learning fences and durable
`learning_reconciled` / `sources_reconciled` phases. Repository-learning and
workspace stores are separate transactions, with #745's repository-then-workspace
lock order; a workspace-only scan cannot authorize deleting a source protected
by a repository family or surviving workspace.

After learning contributions/cards reconcile, release only dependencies no
longer needed. A source still required by another current result, reviewer
performance record or retained learning family must remain protected. Before
deleting its workspace metadata, the #744 custodian must durably transfer that
immutable source, its identity/provenance and authoritative dependency record
to custody independent of the deleting metadata. Transfer must preserve exact
references, incoming-edge guards and recoverability; it cannot rewrite a digest
or turn a missing source into a reviewed result. If that custody operation is
not defined/available, retain the workspace fence and metadata and report pending
deletion. Never claim success after deleting protected bytes. Purge only eligible
sources, verify surviving dependencies, then advance #745's source phase and
remove metadata. Restart resumes durable phases, keeps writes fenced and never
revives producer capabilities. No assignment-result/evaluation tombstone remains
after successful deletion; custody records for protected sources are retained
sources, not deleted-subject tombstones.

**Shared learning remains gated.** This contract supports workspace-local
result/evaluation references only. Repository-shared #745 input admission and
cross-workspace source custody require one jointly reviewed ownership, dependency
publication, locking and crash-recovery contract before implementation. Until
then do not admit shared learning dependencies; if a retained dependency already
exists or cannot be reconciled, keep deletion pending. #745's learning-only
terminal projections do not replace protected source provenance. Deferring shared
consumption leaves independent evaluation usable and does not weaken #745's
deletion requirement. Ordinary non-orchestrated sessions acquire no assignment
evaluations.

## Product boundaries and verification

Assignment evaluation is separate from Taskmaster `CoordinationResult` and the
`CompletionPacket` for `ImproveWorkflow`. It cannot launch/retry work, widen
permissions, advance undeclared dependencies, accept work, declare merge
readiness or authorize integration. #746 may show the overall result with
details on request; distinguish Unassessed from failure. #745 defines learning
eligibility: stale, invalidated, conflicting, blocked/interrupted or sparse
evidence is not a clean signal, and evaluation cannot change active configuration.

A future implementation must verify these contract cases:

| Case | Required behavior |
| --- | --- |
| Complete, current independent evaluation | All declared outputs verified, all required criteria satisfied, quality 3 and no declared relevant conflict yields Meets requirements |
| Known failure with other unknowns | Uncontested required failure, evidenced rating below 3/dimension below standard, or sidecar-confirmed missing output yields Needs rework |
| Missing/stale/disputed evidence, unsupported output or no eligible reviewer | Unassessed absent a separate established failure; lifecycle status stays separate |
| Empty legacy required criteria/rubric | Cannot pass or divide by zero; independent failures still establish Needs rework |
| Optional-only result, finding or stale source | Changes detail without downgrading an otherwise passing required result |
| Same target satisfied versus unassessed / contradictory assessed outcomes | Unassessed; an unassessed opinion cannot erase a separate uncontested failure |
| Same-failure ratings 0/1/2 versus threshold conflict 2/3 | Preserve ratings; first establishes failure, second is disputed/Unassessed absent another failure |
| Required-rework finding with no valid failing impact target / optional target | Reject; a finding cannot bypass aggregate conflict on its target |
| Material-fact conflict with different finding IDs | Reviewer/user declares affected outcomes unassessed or an exact finding dispute; sidecar does not infer semantic equivalence from prose/IDs |
| Current finding disputed with stale evidence / source correction | Ineffective dispute cannot suppress current failure; stale finding cannot establish failure; corrections stale only dependent outcomes |
| Worker/root candidate submitted while producer lives | Stage path-backed candidate until quiescence; artifact-only candidate may finalize immediately; root identity is bootstrap-bound |
| Producer ends before final result | Revoke its grant; sidecar finalizes staged candidate after exact terminal/revocation/quiescence checks without another producer request |
| Resume, changed bytes or predecessor races finalization | Supersede/reject candidate, preserve accepted head; fresh generation/acknowledgement required, no automatic retry |
| Restart with pending candidate | Reconcile exact identity/terminal/revocation/acknowledgement or block; never revive credentials |
| Exact active-generation retry / changed same-key payload / revoked generation | Return saved receipt before predecessor check / conflict / reject before receipt lookup |
| Child evaluation publication with no product-write capability | Atomically store canonical evaluation and declaration-bound immutable artifact; receipt can populate reviewer's candidate |
| Wrong assignment/declaration artifact or bare research report as output | Reject; artifact possession or identical content does not authorize reuse |
| Narrow readPaths with a broad worktree root | Reject undeclared/out-of-read-scope path before opening; root containment is insufficient |
| Symlink/reparse/hard-link escape or platform without required atomic operation | Reject before reading/pinning; no path-based fallback |
| Concurrent source mutation, stalled filesystem operation or exhausted worker capacity | Reject unsupported snapshot, bound request latency/capacity, preserve head; no unbounded replacement workers |
| Accepted snapshot versus later worktree edit/cleanup | Verify immutable object/current head; mutable path changes cannot alter accepted result |
| Worktree handoff racing reference admission | Serialize dependency scan/ownership transfer, block unresolved snapshots and retain all immutable dependencies |
| Independent child review with no user performance grade | Review may affect worker result; reviewer performance remains Unassessed and cannot gate eligibility |
| Child attempts reviewer-of-review / user performance correction | Reject recursive child review; user performance changes only its own result, never original review eligibility |
| User correction/invalidation and concurrent stream creation | Preserve provenance; CAS current head/slot; stale mutation conflicts; invalidation freezes stream |
| Research report corrected under #742 | Only citing outcomes stale; source use blocked until approved correction-head CAS/currentness contract exists |
| Canonical reload/cross-language fixtures | Every tagged/nested shape, null, integer and ordering produces identical bytes/digest; duplicate/unknown fields fail |
| Changed metric or semantic field | Result/evaluation digest changes; full timeCostEvidence is hashed; request metadata/projections do not affect hashes |
| Unavailable metric source | Omit evidence, never fabricate zero/estimate; USD/millisecond provenance and #745 comparison boundaries remain |
| Candidate/result/evaluation/quota exhaustion | Reject visibly, preserve history and 12-child/4-user revision partition; no silent eviction |
| Retention/subject purge racing an incoming dependency | Same boundary protects current sources; exact subjectRevision CAS; purge deletes only unreferenced closed history |
| Workspace deletion with surviving source consumer | #745 learning/source phases and custody transfer complete first, or retain fence/metadata and report pending |
| Unavailable cross-workspace custody/shared learning contract | Reject shared admission; never substitute a workspace-only transaction or delete protected evidence |

## Implementation gate

This is a proposed written contract, not runtime implementation approval.
Before a runtime plan, review #741/#742 mappings, output declarations and the
new scoped candidate/evaluation reporting actions; reconcile #740's capability
evidence, #610, authoritative specs/ADRs, #745 retention/custody and #746
projection. The parent scope supplies independent review and optional terminal
user performance assessment; this contract adds no recursive review or mandatory
certification. Define finite artifact/rehash/worker/run/workspace limits,
canonical byte fixtures, sidecar finalization/recovery and dependency-index seams
before code. Research-report freshness waits on #742's correction-head contract;
provider costs stay absent until their authenticated source is approved; shared
learning stays gated on the joint custody contract. A plan cannot defer the
normative race outcomes above or silently enable unsupported adapters.

Approval approves only the written contract, not role support, launches,
automated evaluation, changes to ordinary Peon/session behavior or runtime code.
