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

## Owner acceptance — 2026-10-23

The owner [accepted the reviewed evaluation behavior and defaults](https://github.com/Rambolarsen/orkworks/issues/744#issuecomment-6367492079)
after reviewing the contract at main `bb6e480a02da1fbce33b5d0cae9bdfc781bb00e6`.
The accepted choices are:

- One overall result: Meets requirements, Needs rework, or Unassessed.
- A completeness percentage only when all required criteria are assessed, and
  the fixed 0–3 quality meanings below. Unknown evidence remains unknown;
  established required failures produce Needs rework. Conflicting or unassessed
  required outcomes block a pass rather than being averaged.
- Independent review, separate worker self-assessment, and optional user-only
  terminal assessment of reviewer performance, without recursive review.
- Exact result-version binding and provenance-preserving correction history.
- Descriptive-only time/cost evidence that cannot affect quality or optional-skill
  preference, and retention of evidence needed by retained consumers. Deletion
  remains pending when safe custody cannot be established.
- Per result, 12 child and four user evaluation revisions, with at most two
  revisions per user stream. Exhaustion blocks writes rather than evicting history.

This acceptance settles those evaluation choices. It does not establish an
eligible adapter, approve a billing source, accept the separate hierarchy
read/recovery proposal, or satisfy the joint shared-learning custody contract.
Cross-contract mappings, technical recovery agreements and the scoped execution
handoff remain open. Runtime implementation and launches require their separate
approvals; #740 capability-evidence work remains deferred until November.

## Technical contract acceptance — 2026-10-24

The owner accepted the scoped #741/#742/#744 technical amendments in response
to the explicit acceptance question following the
[PR #1019 joint review](https://github.com/Rambolarsen/orkworks/pull/1019).
The reviewed contract is the joint handoff at
`f9a1102798a8797edcd6722d5c1edb5685c1d304`. Acceptance covers:

- The frozen reservation `assignmentTaskVersion`, exact child-attachment proof
  and separation from live task CAS/runtime generations.
- The typed exact-result review input, independently scoped retained-read
  capability, dependency protection and read/revocation cutoff. Executable
  evaluation uses output-bearing child v3; v2 retains its input-only shape.
- Structured output declarations, child v3/root-bootstrap v2 placement and
  digest domains, source/plan byte limits and conservative candidate rehash
  ledger/recovery rules.
- The result/evaluation/user reporting action, carrier/envelope, immutable
  receipt and inspection contracts, including sidecar-issued user-stream
  identity, the stable UI retry namespace and exact-retry/CAS ordering.
- The reporting request/rate/concurrency defaults and protected receipt-credit
  accounting, including the 1,088-slot / 17 MiB assignment, 64 MiB owner-run and
  256 MiB workspace receipt pools and fenced cold-start window.

This supersedes owner-acceptance-pending wording for those named amendments,
not unrelated component choices or their qualification gates. The earlier
accepted evaluation/timing choices remain unchanged. No schema, numeric limit,
canonical digest domain or authority rule changes in this acceptance record.

SQLite mechanism/ADR 0090 and its storage limits are **not** accepted by this
technical-amendment decision. The later
[architecture-direction acceptance](2026-10-23-hierarchy-transaction-storage-design.md#scoped-architecture-direction-acceptance--2026-10-24)
accepts one workspace SQLite/private-worker DELETE/EXTRA direction for further
design/qualification only; detailed storage choices and numeric limits remain proposed.
Source limits still must compose with the complete command/row/publication
limits of a separately accepted, qualified storage design; a 16 MiB source
maximum is no promise that a 16 MiB complete command can carry it with metadata.
Broader #741/#742 lifecycle/role choices, artifact/evidence/read/worker/scratch
and overall storage quota contracts, #745/#746 custody/transaction qualification,
#740 role/profile evidence, and scoped implementation/qualification plans retain
their separate gates. No eligible profile, executable runtime plan, endpoint,
model probe, migration or runtime implementation is authorized by this acceptance.
Keep the component issues open for those deliverables.

## Assignment and result identity

An evaluation binds one approved assignment identity, the exact accepted result
revision/digest, sidecar-derived criteria and rubric snapshot digests, reviewer
identity, and immutable evidence. The tagged child/root values defined under
[Canonical records](#canonical-records) are #744's encoding of identities resolved
from #741/#742 approved state; those documents do not currently define a tagged
assignment-identity schema. Display labels and reporter claims cannot supply
identity. Changing an identity member or rubric requires a new approved
assignment. Runtime generations fence authority without changing the assignment
subject. The [configuration/reporting reconciliation](#configuration-and-reporting-reconciliation--2026-10-23)
records the pending mapping of mutable upstream `taskVersion` to this immutable
identity; a current counter must not silently replace a stored identity member.
Root results use explicit user review; v1 child reviewers evaluate only child results.

### Declared outputs and authorized sources

The approved assignment declares at most 32 unique output IDs with bounded
kind labels, using #741's identifier rule. Each `OutputDeclaration` has exactly `id`, `kind`,
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
total rehash-work cap. Reject declarations above server hard limits. The
[proposed structured output contract](2026-10-04-agent-role-configuration-design.md#proposed-structured-output-contract--2026-10-24)
now supplies versioned placement, canonical ordering and numeric limits; written
acceptance and joint consumer/implementation review remain required before code.
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
provider/model/pricing/cohort rules in the [aligned learning contract](2026-10-04-configuration-learning-design.md#optional-elapsed-time-and-cost-observations).
This alignment does not enable shared learning; consumption remains subject to
the shared-learning gate below.

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
current correction head under [#742’s proposed head protocol](2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections):
exact expected-version/digest CAS, exact-retry resolution before head checks,
and atomic publication with dependent freshness fences. Arrival time or numeric
version alone cannot select that head; a saved receipt does not establish
currentness. User-authored replacements retain their explicit provenance and
assignment lineage, never child authorship. Until that contract is approved and
reconciled, research-report evidence cannot establish
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
it does not presume an upstream tagged report schema. Report freshness consumes
[#742's current-head and correction contract](2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections).
That written contract now defines head CAS, exact version/digest validation,
dependent invalidation and atomic recovery. Written acceptance and a reviewed
joint implementation boundary remain gates; source events alone cannot establish
currentness.

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

Assignment evaluation is separate from the Taskmaster parent coordination
result (an exact version-bound parent outcome per the
[authoritative Taskmaster spec](../../../specs/taskmaster.md)) and from the
`CompletionPacket` that completes an `ImproveWorkflow` recommendation. It
cannot launch/retry work, widen
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
| Research report corrected under #742 | Only citing outcomes stale; consume the written correction-head CAS/currentness contract after acceptance and joint transaction review |
| Canonical reload/cross-language fixtures | Every tagged/nested shape, null, integer and ordering produces identical bytes/digest; duplicate/unknown fields fail |
| Changed metric or semantic field | Result/evaluation digest changes; full timeCostEvidence is hashed; request metadata/projections do not affect hashes |
| Unavailable metric source | Omit evidence, never fabricate zero/estimate; USD/millisecond provenance and #745 comparison boundaries remain |
| Candidate/result/evaluation/quota exhaustion | Reject visibly, preserve history and 12-child/4-user revision partition; no silent eviction |
| Retention/subject purge racing an incoming dependency | Same boundary protects current sources; exact subjectRevision CAS; purge deletes only unreferenced closed history |
| Workspace deletion with surviving source consumer | #745 learning/source phases and custody transfer complete first, or retain fence/metadata and report pending |
| Unavailable cross-workspace custody/shared learning contract | Reject shared admission; never substitute a workspace-only transaction or delete protected evidence |

## Hierarchy summary revision ownership

For the proposed #746 projection, the sidecar owns material result summaries
and their revision state. This is read-projection metadata, not a new member of
strict `AssignmentResult`, `EvaluationReference`, or evaluation digest payloads.
It is also distinct from purge/CAS `subjectRevision` and ordinary session
activity summaries or unread state.

The proposed producer contract is:

- Key a durable summary row by the complete tagged root/child
  `AssignmentIdentity`. Bind its material state to the exact accepted result
  revision/digest (or explicit absence), authoritative overall result, required
  findings, and required outcome freshness/conflict dispositions. Missing or
  corrupt source state yields unavailable, not an invented Unassessed summary.
- `summaryRevision` is a canonical decimal string from `"1"` through
  `"2147483647"`, without leading zeros. The sidecar increments it once per
  committed material-state change after initial revision `"1"`. Initialize only
  for a newly admitted summary subject, atomically with its first material state;
  a missing row for a previously admitted subject is corruption, not initialization.
  The renderer compares it only for equality
  within that complete assignment identity; it never orders different subjects
  or treats it as a grant, result revision, or evidence generation.
- A correction round trip advances twice even when the final displayed state
  resembles the initial state. Optional-only detail changes, timestamps,
  polling, navigation and restart with unchanged revalidated material state do
  not advance it. No content hash or process-local counter substitutes for this
  durable revision lineage.
- Publish source eligibility, authoritative material state and revision under
  one reviewed serialization/recovery boundary, including #742 report-head
  corrections. Until reconciliation completes, fence affected reads rather
  than pair new evidence with an old passing summary. Events only notify readers.
- Retain the row while the assignment's result summary or viewed/collapse
  identity can be served. Never evict/recreate it with revision 1 for the same
  assignment. Purging the subject removes its summary; retained navigation
  displays summary unavailable and cannot auto-collapse on an old viewed token.
  Missing/corrupt lineage cannot be regenerated from the latest outcome alone.
- Exhaustion refuses publication of another current summary and exposes it as
  unavailable without blocking an otherwise authorized source correction or
  serving the previous disposition as current. Never wrap or reuse revisions.

This gives the consumer a bounded stable token; it does not select a storage
engine or authorize runtime work. Exact canonical material-state bytes, row and
workspace byte/count limits, transaction/recovery fixtures, deletion ordering
and lossless transport require review in the scoped runtime plan. See the
[mapping fixtures](2026-10-15-hierarchy-projection-mapping.md#summary-revision-and-action-freshness).

The [read/recovery follow-up](2026-10-20-hierarchy-read-recovery-design.md#durable-summary-material)
proposes exact material bytes, row/workspace limits and transaction/recovery cases
for joint #744/#746 review. The owner [accepted four recovery-contract choices
on 2026-10-23](2026-10-20-hierarchy-read-recovery-design.md#owner-acceptance--2026-10-23).
Producer agreement remains pending for the full material/classification, budget,
publication/recovery and narrow-detail scope. Storage qualification and separately
approved runtime plans remain gates.

### Proposed hierarchy producer mappings

The [producer reconciliation](2026-10-20-hierarchy-read-recovery-design.md#producer-reconciliation--2026-10-23)
maps this contract's exact identities, accepted head, snapshot digests,
current required outcomes/findings and immutable detail bytes to the recovery
material. This contract remains the aggregate authority; the projection copies
its classifications rather than scoring independently. Result/evaluation
publication, user invalidation/replacement, source-currentness changes and purge
must publish dependency and summary transitions or unavailable fences coherently.
Optional-only corrections may change detail references without changing material;
#743 telemetry alone supplies no evaluation evidence. Source quotas remain
independent of the additional summary storage caps. Producer agreement remains
pending for all material/classification, budget, recovery and detail choices;
concrete writer/storage/crash qualification and runtime approval are not delivered
by this proposed mapping.

### Proposed hierarchy display encoding

The [hierarchy transport proposal](2026-10-17-hierarchy-projection-transport-design.md#lossless-display-encoding)
uses explicit display DTOs with decimal strings for u64 metadata/artifact fields.
Canonical records above retain their integer tokens and digest bytes. This
proposed consumer codec does not finalize artifact retrieval, summary storage,
or the joint recovery boundary. The recovery-choice owner acceptance is linked
above; producer agreement and broader display-encoding acceptance remain open.

## Configuration and reporting reconciliation — 2026-10-23

This is the proposed #744 handoff to #741 configuration and #742 authority,
reviewed against source at `907decdf9769b6c0551d192ac96d897c9a481c4d`.
It preserves the accepted evaluation defaults. The tables distinguish existing
source mappings from proposed additions; they do not claim upstream acceptance,
new routes, adapter eligibility or runtime implementation. Hierarchy writer,
storage and crash qualification under #746 remains a separate work item.

### Identity and approved snapshots

| #744 value | Source and exact join |
| --- | --- |
| Child workspace, parent, plan/revision, task and configuration ID/digest | Copy #741's approved `AssignmentConfiguration`; join its workspace/parent to #742's admitted run, and its exact plan/revision/task to the successful allocation. All members must agree; a matching task label is insufficient |
| Child `runId`, `reservationId`, `childSessionId` | Resolve #742's admitted run and exact allocation/runtime records. These are not #741 configuration fields and must not be added to its configuration digest after approval |
| Root workspace, run, root assignment, parent and bootstrap ID/digest | Copy #741's `OrchestratorBootstrapConfiguration` and join #742's run definition to its exact bootstrap digest. A later `ParentPlanBinding` cannot replace the root subject with a plan/task identity |
| Child `AuthorityGeneration` | Use `kind: child-launch`, the authenticated sidecar generation and that child's current launch generation. A resumed session ID alone cannot establish the runtime |
| Root `AuthorityGeneration` | Use `kind: parent-runtime`, the current sidecar generation and #742's current parent runtime generation; use #742's same opaque 64-character hex encoding, not the adapter generation |
| Criteria/rubric digests | Derive from the approved child configuration or root bootstrap using this document's snapshot domains. #741 preserves meaningful criteria order in its configuration digest; #744 separately sorts criteria by stable ID for its criteria-snapshot digest. Neither digest replaces the other |
| Report evidence identity | Copy the immutable #742 report's exact run/plan/revision/task/allocation/configuration/runtime/report version; derive workspace from stored ownership. User replacements preserve their stored child lineage and explicit user provenance; they are not a new child runtime |

**Task-state version discrepancy.** #741 explicitly excludes `taskVersion` from
immutable configuration and #742 defines it as mutable state. #744 currently
includes it in the canonical child `AssignmentIdentity`. Reading the latest task
counter for each result, correction or review would therefore create different
subjects for one approved assignment and could strand existing references.

The [proposed #742 admission-version contract](2026-10-04-orchestrator-preparation-design.md#proposed-evaluation-admission-version-snapshot)
now defines durable reservation metadata `assignmentTaskVersion`: the resulting
task version committed with the successful child reservation, before child
attachment/spawn. Copy that exact value into every #744 child identity's
`taskVersion`, including reviewer identities. Resolve the remaining members from
the same reservation/configuration and its uniquely verified child attachment;
a reservation without that proof cannot supply a complete child identity.

Later task-state changes and exact resumes do not rewrite the frozen identity.
Current task-version CAS, where an operation requires it, stays separate request
metadata and is checked against current #742 state; it is never taken from the
frozen identity. #743's task-state evidence binding retains its own semantics.
The #742 amendment supplies capture, exact-retry, overflow, attachment, retention
and crash/recovery rules; #741's consumer boundary links the same proposal.

The owner accepted this cross-contract amendment in the
[technical acceptance record](#technical-contract-acceptance--2026-10-24).
Producer/storage proof and capability/implementation gates remain open;
acceptance does not make the mapping eligible for runtime implementation. Missing
historical admission evidence cannot be reconstructed from today's counter or a
report timestamp. Already-hashed #744 objects must match the proven snapshot;
never rewrite their identity/digest. Removing `taskVersion` instead would change
canonical objects and every consumer reference, requiring a coordinated
schema/fixture revision. This proposal retains the current canonical shape.

### Output declarations and reviewer inputs

#741's v1 assignment has a 2 KiB output-contract **text** bound. Its separately
[proposed output-bearing versions](2026-10-04-agent-role-configuration-design.md#proposed-structured-output-contract--2026-10-24)
now define an inline structured declaration array, ordering and numeric caps.
That technical amendment remains proposed, not implemented support. Prose cannot
supply missing IDs, paths, byte caps or reporting permission. The cross-contract
requirements remain:

- Bind the exact structured declarations and finite total rehash-work cap into
  the approved child configuration or root bootstrap, and thus its canonical
  digest. Preserve the existing declaration schema above; validate unique IDs,
  all-required semantics, the 32-output limit, authorized roots/read scopes and
  required worker write scopes before approval.
- Specify the structured manifest's field placement, canonical declaration
  ordering and aggregate byte accounting in #741. Do not silently overload its
  text field or exempt the manifest from configuration/plan bounds. The new
  amendment proposes exact per-output/server/rehash and per-plan defaults;
  accepting those defaults and qualifying their consumers remain gates.
- Bind a reviewer's exact target `(assignmentIdentity, resultRevision,
  resultDigest)` as an approved immutable input and its `assignment_evaluation`
  server-artifact declaration to one reviewer stream. The
  [proposed #741 v2 input](2026-10-04-agent-role-configuration-design.md#proposed-typed-reviewer-input--2026-10-24)
  now supplies a typed target/read-scope proposal; the output-bearing v3
  amendment combines it with structured declarations. Their technical acceptance
  remains gated. Result drift requires a separately
  approved assignment; it cannot retarget an existing capability.
- Validate the rubric's evaluator role, independence and exact read access
  before issuing review authority. A server-artifact reporting permission does
  not require product-file write access or permit arbitrary artifact retrieval.

**Review-target timing choice accepted — 2026-10-24.** Require fresh approval
of an exact accepted result for the initial child-review flow, as specified
below. The owner accepted this choice and the extra user step in the
[PR #999 session](https://github.com/Rambolarsen/orkworks/pull/999). Technical
producer/schema agreement and runtime implementation remain separately gated.

These are proposed schema requirements, including the separate proposed v2
review-input amendment, not implemented #741 support. Required output
authorization or target binding that cannot be represented
blocks admission. A bare #742 `ResearchReport` remains evidence only: it lacks
an assignment/declaration-bound immutable output wrapper. No report-to-result
conversion or cross-assignment artifact reuse is introduced.

### Proposed exact-result review approval

**Owner-accepted timing choice — 2026-10-24.** In the initial flow, approve a child reviewer
only after its exact target result has been accepted by sidecar finalization.
A worker's initial plan can describe the intention to obtain independent review,
but that intention is not an executable reviewer assignment, dependency node,
reservation or launch grant. Reject an executable reviewer configuration with
an absent, predicted, mutable or deferred target. This is the accepted timing restriction
for the initial flow; it does not remove independent review from the workflow.

Two approaches were considered:

| Approach | Consequence |
| --- | --- |
| Fresh approval against an accepted result — owner selected | Exact target is part of the immutable reviewer configuration from the start. The user approves a later review plan, adding an approval step |
| Approve a worker-bound future-result selector | Could avoid that step, but needs a new approved selector schema, deterministic one-time binding, digest/authority separation, correction races and durable recovery proof. Existing dependencies and grants do not supply that contract |

The second approach is deferred, not prohibited forever. Do not substitute a
`latest` query, choose the first candidate, or append a result digest to an
already approved configuration. A separate reviewed amendment would be required
before offering automatic worker-to-reviewer launch for future results.

#### Approval and lifecycle

1. Finish the worker's existing approved assignment. A completion message,
   parent coordination result or accepted candidate alone is insufficient:
   #744 sidecar finalization must publish the immutable accepted result and
   its revision/digest. Missing outputs may still yield a reviewable partial
   result under the existing evaluation rules; do not require a passing score.
2. Resolve and retain the exact result and evidence under the existing custody
   rules. The review input names the worker's full `AssignmentIdentity`,
   `resultRevision` and `resultDigest`; the reviewer's own run/plan/task identity
   remains distinct. Show the target, scope, rubric, independence, evidence
   access and declared evaluation output in the proposal. A label or source
   session ID is insufficient.
3. For a future worker result, use an explicitly UI-created review run with its
   own reviewed bootstrap, goal and child ceiling, followed by exact review-plan
   approval. Do not reopen the completed worker run or transfer its planning
   bearer, launch grant, task reservation or approval. Final execution completion
   still revokes the worker run's authority under ADR 0077. This proposal adds
   no execution-to-execution transition or automatic replacement parent.
4. If an exact accepted result is already available when an otherwise eligible
   plan is first proposed, that plan may declare its reviewer directly. The
   target must already be bound before approval; no extra result-selection step
   is needed. Neither case creates branches, worktrees or children before the
   relevant exact-plan approval.
5. Revalidate exact input identity, retained custody, current source freshness,
   independence, permissions, profile eligibility and ordinary launch fences at
   approval and again at reviewer admission. A newer accepted result or changed
   cited source before admission makes the proposal stale: prepare a new exact
   proposal and obtain approval. A failed approval/admission check creates no
   reviewer reservation and leaves the result available for explicit user review.
6. Once admitted, the reviewer remains bound to that result. A later correction
   does not retarget its configuration, capability or evaluation stream. Preserve
   historical review under its original subject and apply the existing freshness
   rules; reviewing the replacement requires another approved assignment. Review
   completion does not restart the worker or launch remediation.

The new run consumes retained immutable result snapshots and approved evidence;
it does not adopt or reuse the worker's worktree, branch or live filesystem.
Its own worktree allocation follows the ordinary plan rules. Reading an exact
retained result is a proposed typed input authorization, not cross-assignment
**output** reuse: the reviewer publishes its own `assignment_evaluation` artifact.
No arbitrary artifact lookup, live path read, cross-workspace import or report-to-
result conversion is granted. An unresolved read/custody/identity mapping blocks
child review rather than falling back to the old worktree. Root results and
optional reviewer-performance assessments retain their user-only rules.

This deliberately adds a user action compared with the aspirational same-plan
worker/reviewer examples in #742. Automatic collection/proposal preparation
within a still-authorized run remains subject to #740; a completed run cannot
prepare or launch a new run using revoked authority. Run creation and approval
are user actions, not an automatic continuation or inferred consent. The
proposal does not claim a review UI or retained-result transport already exists.

#### Producer agreement and contract cases

#741 must bind the typed target and exact result/evidence read authorization
into the reviewer configuration and its digest; its existing text input field
cannot silently stand in for that schema. #742 must reject unresolved reviewer
assignments before approval/admission and reconcile its workflow examples with
this restriction. #744 continues to resolve worker and reviewer identities from
their respective reservations, including their own frozen admission versions.
Run completion of the worker does not erase the retained subject or grant the
reviewer authority over the worker run. Same-workspace retained-result references
across runs require explicit schema, custody and authorization agreement before
an executable handoff; this proposal does not establish their eligibility.

These synthetic cases define expected contract behavior, not executed tests:

| Case | Required outcome |
| --- | --- |
| Initial plan contains worker plus executable reviewer with no accepted result | Reject reviewer configuration; a review intention grants no launch or reservation |
| Candidate stored but finalization is pending or rejected | No exact-result reviewer approval; expose the missing accepted result |
| Worker finalized a partial result with a required output missing | Exact-result review may be proposed; evaluation retains the known failure |
| Worker run completes before review is proposed | Keep its authority revoked; user creates a separate review run and approves its exact plan |
| Review plan targets a retained child result from a prior run | Require complete worker identity and explicit same-workspace typed read/custody agreement; no worktree adoption |
| Target changes between proposal, approval and admission | Fence the stale proposal; new exact target needs fresh approval, with no reservation from the failed check |
| Target changes after reviewer admission | Keep the old target immutable; apply existing freshness/history rules and require separate approval for replacement review |
| Parent/sidecar ends before review launch | Existing generation/resume/reapproval rules apply; no deferred selection or automatic relaunch |
| Required snapshot/evidence is missing, deleted or inaccessible | Block child review; explicit user review remains a separate authorized path |
| Review finds rework | Record the evaluation; no implied worker resume, retry, remediation or merge |

**Disposition.** The owner accepted the fresh exact-result approval choice,
including a separate user-created review run for future output and its extra
approval step. This accepts the timing/user-flow choice only; #741/#742 producer
agreement remains open. The typed target/output schemas, exact aggregate bounds,
reporting actions, retained-reference custody, #740 eligibility and scoped
implementation approval remain separate gates. No runtime change, new capability
profile, model probe or implementation plan is supplied by this proposal.

### Reporting actions and lifecycle boundaries

The operation labels here describe #744 semantics. The later
[proposed reporting protocol](#proposed-result-and-evaluation-reporting-protocol)
now supplies action IDs, logical envelopes and carrier routes, with matching
#741/#742 projections. These remain proposed, not implemented endpoints or
verified adapter tools. Technical acceptance, delivery and finite request/receipt
quota qualification remain gates before an executable plan. Neither the parent's run
bearer nor `ResearchReportCapability` is an evaluation/result capability.

| Operation | Required authority and atomic boundary |
| --- | --- |
| Submit candidate / read own receipt | Distinct `AssignmentResultCapability` bound to exact worker/root assignment and current runtime generation. Authenticate current authority before retry lookup; resolve an exact receipt before predecessor CAS. Candidate acceptance consumes the assignment's 32-candidate budget, not a result revision |
| Finalize candidate | Sidecar-owned operation over a durable candidate, not a post-exit producer call. Recheck predecessor and immutable outputs; path sources additionally require exact terminal/revocation/quiescence proof. Atomically publish accepted result, retained objects/dependencies and finalization receipt, or a durable rejection |
| Submit/correct child evaluation / read own receipt or artifact | Distinct `AssignmentEvaluationCapability` bound to reviewer assignment/current generation, exact worker result and evaluation declaration. Authenticate before retry; resolve receipt before stream-head CAS. Commit evaluation, declared immutable artifact, dependency edges and revision-budget use together |
| Create/correct/replace user evaluation | Electron-authorized user path; derive user provenance and review-stream identity. Serialize stream-slot CAS, predecessor/disposition checks, evaluation publication, dependencies and user budget; never modify child provenance |
| Invalidate evaluation | Electron-authorized user path only; exact stream-head revision/digest CAS atomically records the disposition, freezes the stream and removes its current influence. Exact retry returns its receipt; a later unrelated write cannot reopen the stream |
| Correct a cited research report | #742 owns authority, report-head CAS and retry semantics. #744 participates in dependent freshness publication; it cannot issue a report correction or select a head independently |

Child reporting grants follow #742's runtime-end/resume, sidecar/workspace,
run-cancellation/final-completion and namespace-rotation fences. A parent result
grant additionally ends with that parent runtime or run authority. A paused or
completed stage alone does not restore launch authority or revoke a still-valid
reporting grant while the run remains active. A resumed producer needs fresh
credentials/generation; old credentials cannot replay even an accepted request.
Receipts remain durable provenance, not an authorization bypass.

Revocation stops producer writes; it does not delete accepted records or pending
candidates. Sidecar finalization continues only with this document's recovered
proofs. Run completion alone neither accepts a candidate nor makes its subject
purgeable. User operations remain separate after producer authority ends and
cannot resume/reopen the run. No runtime user-review path is implemented merely
because its provenance contract is defined here.

### Cross-contract verification cases and disposition

These are required synthetic cases for contract fixtures, not executed runtime
or adapter tests:

| Case | Required outcome |
| --- | --- |
| Task state advances after admission | Same frozen #744 subject; current CAS uses separate live metadata. Missing admission snapshot blocks mapping |
| Child resumes with unchanged approved configuration | Same subject, fresh runtime authority; pending old candidate superseded, accepted immutable results retained |
| Root moves from research to execution | Same bootstrap/root identity; a plan binding supplies no replacement criteria or result authority |
| Same criterion set arrives in another order | #744 sorted snapshot digest is stable; the full #741 configuration remains bound to its originally approved array order |
| Worker can write output but cannot authorize snapshot read | Reject configuration before approval; file existence grants no read |
| Reviewer target changes while its capability remains live | Reject retargeting; no latest-result substitution or new evaluation stream on an undeclared subject |
| Report token or parent planning bearer submits an evaluation | Deny wrong capability class even if assignment IDs match |
| Exact evaluation retry arrives after revocation | Reject authentication; a stored receipt cannot revive authority |
| Producer ends with a staged path candidate | No final producer submission required; finalize only after exact terminal/revocation/quiescence proof, otherwise retain pending/rejected disposition |
| Report correction changes cited evidence | Preserve immutable evaluation bytes; atomically stale affected outcomes or fence current reads under the joint publication contract |

Existing joins and the discrepancy inventory are recorded here; the frozen
admission version now has a matching proposed #742 producer/#741 consumer
contract. The reviewed identity, typed output/target and reporting-action
amendments now have the scoped owner acceptance recorded above; their remaining
producer/storage/adapter qualification and implementation gates stay open. The separate
exact-result approval timing choice above is owner accepted. Aggregate artifact/rehash/worker,
request/receipt and run/workspace limits still need exact values. #740 eligibility,
#745 shared custody, #746 producer/storage qualification and separately approved
runtime plans remain independent gates. Keep #744 open; this reconciliation does
not mark its remaining definition criteria or implementation handoff complete.

## Implementation gate

The evaluation behavior/defaults have the owner acceptance recorded above.
Technical reconciliation and the scoped execution handoff remain pending; this
is not runtime implementation approval.
Before a runtime plan, agree the proposed [#741/#742 identity, declaration and
reporting mappings](#configuration-and-reporting-reconciliation--2026-10-23), including
the frozen admission task version and typed output/target schemas; reconcile #740's capability
evidence, #610, authoritative specs/ADRs, #745 retention/custody and #746
projection. The parent scope supplies independent review and optional terminal
user performance assessment; this contract adds no recursive review or mandatory
certification. Define finite artifact/rehash/worker/run/workspace limits,
canonical byte fixtures, sidecar finalization/recovery and dependency-index seams
before code. Research-report freshness requires acceptance and joint implementation
of #742's written correction-head contract;
provider costs stay absent until their authenticated source is approved; shared
learning stays gated on the joint custody contract. A plan cannot defer the
normative race outcomes above or silently enable unsupported adapters.

Approval approves only the written contract, not role support, launches,
automated evaluation, changes to ordinary Peon/session behavior or runtime code.

## Proposed review-input custody

This is the proposed #744 custody side of the
[#741 typed review input](2026-10-04-agent-role-configuration-design.md#proposed-typed-reviewer-input--2026-10-24)
and [#742 admission/read grant](2026-10-04-orchestrator-preparation-design.md#proposed-retained-review-input-admission).
The owner accepted this reviewed schema/read-custody amendment in the
[technical acceptance record](#technical-contract-acceptance--2026-10-24).
Joint producer/storage qualification and separately approved implementation/
qualification plans remain required.

The review target copies this document's complete child `AssignmentIdentity`,
`resultRevision` and `resultDigest`; it changes no #744 canonical result,
evaluation, evidence or digest domain. Read evidence uses the existing exact
`EvidenceSource` variants and source-key algorithm. Worker and reviewer identities
remain independent, with their own reservation/frozen admission-version proof.
Do not substitute the new review run's plan/task/runtime fields into the target.

Proposal publication must protect the exact result manifest, required provenance
and criteria/rubric snapshots, selected output objects and selected evidence
with incoming dependency edges. The owner key is the review proposal's exact
run/plan/revision/task/configuration identity, which exists before allocation;
it is not a fabricated reviewer session/reservation. Retain the worker's source
identity proof while referenced. Approval and allocation record their consumer
bindings without a gap in protection. An accepted evaluation/artifact/result
adds its own required edges atomically with publication, under the existing
workspace-local dependency boundary.

Stage bounded copying/integrity work outside that boundary. At publication,
revalidate source/head/custody and atomically commit the proposal/edges, or
publish neither. A purge that commits first makes proposal publication fail;
proposal publication first prevents purge. Temporary staged bytes supply no
review authority and must be reconciled by the bounded storage/recovery plan.
Use the same boundary through approval/reservation/source publication; if the
stores cannot share that atomic fence, deny admission until the jointly reviewed
storage protocol supplies it. Per-subject revision CAS alone cannot protect a
reference owned by another run.

Rejection, cancellation, supersession and reviewer termination revoke their
applicable authority but do not erase retained references. A retained historical
proposal/configuration/evaluation still protects its exact dependencies. Release
edges only with deletion of the corresponding owner under existing retention
and purge rules and after checking all other consumers; never expire pins while
a retained immutable owner still requires them. Charge proposals, approved
configurations, edges and snapshots to existing plan/configuration limits and
finite workspace quotas; exhaustion blocks new proposals, not evidence eviction.
The unresolved aggregate quotas and crash/storage proof remain explicit gates.

Recovery rebuilds or verifies these owner-to-source edges before enabling reads,
admission, purge or cleanup. An uncertain index, incomplete proposal publication,
missing snapshot or unsupported recovered identity fences affected operations;
do not reconstruct from live files, today's task counter or an old bearer.
Source-run completion cannot delete protected bytes. Cross-workspace references,
shared-learning admission, workspace deletion/custody transfer and #746's storage
qualification retain their separate joint gates; this proposal enables none.

The following are synthetic contract expectations, not executed runtime tests:

| Case | Required outcome |
| --- | --- |
| A v1 configuration contains `reviewInput`, or a v1/v2 descriptor asks for executable #744 evaluation authority | Reject unsupported producer version; no implicit upgrade to output-bearing v3 |
| Target worker IDs change while reviewer IDs stay constant | Different output-bearing v3 configuration digest; new exact approval required |
| Reviewer runtime generation changes on exact resume | Same configuration/target, fresh read credential after revalidation |
| Target/result/evidence belongs to another workspace or repository | Deny before publication; no path/name-based import |
| Missing output is selected | Read only known-missing manifest metadata; evaluation preserves known failure |
| Output path now contains different bytes | Read retained approved snapshot only; no live path read |
| Read request names an unlisted output/source or source-run operation | Deny even if the artifact exists and the source run is terminal |
| Required evidence exceeds finite input budget or verified delivery limit | Reject proposal; no truncation or broader-profile fallback |
| Proposal publication races subject purge | One atomic winner: retained edges protect sources, or proposal fails without dangling references |
| Source changes after approval but before reservation | Fence stale approval; no reservation/read grant; new exact proposal required |
| Source changes after reservation but before spawn | Retain the unique reservation and block launch; no automatic retry or grant delivery |
| Source changes after reviewer admission | Keep original target/read scope; apply historical/freshness rules without substitution |
| Proposal is rejected but historical configuration is retained | Keep referenced evidence protected until owner deletion is safe |
| Sidecar restarts after proposal persistence | Recover dependency proof before read/admission/purge; no old credential revival |
| Read starts, then revocation wins before response commitment | Discard buffered response and deny delivery; request acceptance grants no bytes |
| Response commitment wins, then revocation precedes client receipt | Only the already committed response may arrive; deny all later commitments without claiming bytes can be retracted |

**Disposition.** This supplies concrete proposed record placement, a separate
read-authority class and workspace-local dependency ownership across #741/#742/
#744. It does not settle structured output-array placement, reporting action/
transport envelopes, aggregate resource quotas, frozen-version acceptance,
shared custody, #746 storage/qualification, #740 eligibility or implementation
approval. Keep the component issues open.

## Proposed output byte and rehash accounting

This is the #744 consumer of #741's
[proposed structured output contract](2026-10-04-agent-role-configuration-design.md#proposed-structured-output-contract--2026-10-24).
It defines the logical limits and budget semantics covered by the scoped
[technical acceptance](#technical-contract-acceptance--2026-10-24).
Reporting/transport integration, native snapshot qualification, finite broader
worker/storage quotas and a separately approved executable plan remain gates.
Existing candidate/result/evaluation canonical shapes and digest domains remain
unchanged; they bind the new producer descriptor through its configuration or
bootstrap digest.

### Payload versus metadata accounting

Check all declaration maxima and aggregate sums before approving a configuration,
creating a result capability or accepting a candidate. These pre-candidate checks resolve
committed producer/artifact metadata; content verification waits for the accepted
candidate's work ledger. Prior artifact production retains its own separate
producer budget. Candidate entries still match every declaration in canonical ID order, including known/missing entries;
a reporter cannot add or relabel an output. For a claimed-present entry, verify
its declared size is within the exact declaration and checked aggregate. At
finalization, measure and verify actual immutable bytes; reporter sizes alone
cannot establish admission. A file/artifact whose actual size exceeds its cap
is unsupported and rejects finalization, not a known-missing output or a partial
truncated success. Known-missing entries contribute zero accepted payload bytes
but retain the existing finalization proof and Needs rework semantics.

`maxSnapshotBytes` bounds the sum of present output payload sizes in one
candidate/result. Charge each declared occurrence, regardless of physical
storage deduplication or source-kind differences. Its declared-capacity sum is
reserved conservatively before any output exists; actual missingness cannot
make an oversized declaration set approvable. The child/root's approved cap
applies to every candidate across exact resumes, never a newly inferred cap.

Serialized candidate, result, evaluation and disposition metadata each retain
the existing 512 KiB bound. Declaration JSON counts within the containing
configuration/plan byte bounds. Payloads are not embedded in candidate/result
JSON and get no exemption from their own declaration/aggregate caps. An
evaluation's canonical bytes must satisfy both the existing 512 KiB record cap
and its reviewer output declaration before atomic publication as an artifact.
Its later inclusion as a result output charges those artifact bytes to that
result's payload sum too; this is a second consumer budget, not a second output
identity. Prior artifact creation/reporting work is not free or charged to a
future nonexistent candidate: it remains governed by separately required
producer/reporting quotas. No generic server-artifact publication route is added.

### Rehash budget, retries and recovery

`maxRehashBytes` bounds output-content processing per accepted candidate,
including failed finalization attempts and interrupted recovery work. Charge
one byte for each byte consumed by each content-digest pass. A copy and source
hash using the same buffer are one pass; verifying the completed immutable copy
is another. Hashing the same data again, including checking an existing artifact
or a staged destination after restart, is charged again. A one-byte oversize
probe is charged as well, even if it is not hashed or retained. Bounded metadata
record hashing is governed by the separate 512 KiB record limits, not disguised
as output payload. Scope/identity/link checks still require their proofs; a byte
budget is neither an I/O deadline nor evidence of quiescence.

Before each potentially blocking content pass, durably reserve an upper bound
on its byte charge in a candidate-owned work ledger, serializing the checked
increment with that candidate's finalization state. Stage the bounded pass
outside the dependency transaction. Never begin an uncharged pass. Charge stays
consumed even if the pass reads less, fails, times out or is interrupted; this
conservative ledger never refunds or wraps. Each content pass reserves at least
one byte unit even for empty content, so repeated zero-byte verification is not
uncharged. An empty declaration set needs no output-content pass. If detecting an oversized source
requires reading one extra byte, reserve that byte before probing. Inability
to reserve a required pass atomically records a terminal `budget-exhausted`
finalization rejection and its receipt without publishing a result revision.
It does not classify an output as missing or reset the candidate budget. Exact
retries then resolve that stored rejection and cannot schedule another pass.
A completed failed finalization likewise records its durable rejection; only
an interruption before an outcome may leave a pending candidate for the existing
recovery flow. Every recovery pass needs a new positive reservation, with no
unbounded free attempt loop; request/rate/worker limits remain separate gates.
Reserve no more than the approved cap. Native worker/deadline/queue qualification
remains separate; stuck I/O does not justify replacement workers or unbounded
retries. The minimum two-times-snapshot cap permits the ordinary source-copy/hash
and destination-verification passes; it does not promise spare probe/recovery
capacity for every maximum-sized candidate.

Retain the ledger with the durable candidate, outside its canonical object and
digest. Receipt lookup for an already finalized/rejected candidate returns that
outcome without new reads, snapshot creation or budget consumption. A crash
before a reserved pass leaves its charge consumed. Recovery may reuse only
independently verified staged work under the existing identity/quiescence and
storage contract, charging any new verification passes; uncertain or exhausted
work proof blocks finalization. It must not infer work from a present staging
file or reset a counter on restart/resume. Candidate finalization, rejection and
ledger changes need the jointly reviewed storage/atomicity boundary before code.

The existing 32 accepted-candidate cap includes rejected and superseded
candidates across resumes. A separately authorized resume can admit a new
candidate while capacity remains; it cannot replenish the old ledger or
re-finalize its bytes under a new ID automatically. At the proposed maxima,
one assignment can declare at most 512 MiB of accepted-result payload and
2 GiB of candidate output hashing; one plan's child contracts imply at most
2 GiB and 8 GiB respectively across their 32-candidate allowance. These are
conservative logical bounds, not a complete disk/CPU quota or a permission to
consume them. They exclude bounded record bytes, prior artifact-production
work, retained evidence/read budgets, scratch duplication and storage overhead,
which require their own finite admission/retention/worker quotas. Root accounting
is separate and counted once per root assignment, not once per plan. Protected
evidence is never evicted to make these numbers fit.

### Contract examples and disposition

These are synthetic verification expectations, not executed runtime tests:

| Case | Required outcome |
| --- | --- |
| Same declaration set arrives in a different order | Normalize by ID before rendering/hash; same structured contract bytes |
| Empty output contract but task/criteria require a produced artifact | Reject scope/contract mismatch; an empty array cannot manufacture completion |
| Zero-byte content verification is interrupted before an outcome | Keep its positive reserved charge; recovery needs remaining capacity and cannot loop at zero cost |
| Duplicate ID, duplicate path source or unknown object property | Reject before approval; never merge entries or grant alias reads |
| Same kind labels on different authorized paths | Distinct IDs/outputs remain distinct and both consume capacity |
| Per-file maximum or sum of declaration maxima exceeds a cap | Reject before outputs/branches/children exist; later missing outputs do not cure it |
| Two 8 MiB path declarations, 16 MiB snapshot cap and 32 MiB rehash cap | Fits source-contract limits for two full content passes only; newly storing those 16 MiB plus mandatory metadata exceeds the proposed 16 MiB complete-command cap and must be rejected |
| Fifth 16 MiB child contract in one plan | Reject the 80 MiB declared sum above the 64 MiB plan cap |
| Reviewer evaluation record fits 512 KiB but exceeds its smaller output declaration | Reject evaluation/artifact publication atomically; no oversized artifact or stream revision |
| Root output declaration changes in a later plan | Deny bootstrap mutation; require a separately reviewed UI-created run |
| Claimed-small file is larger at finalization | Bound reads/probes by reserved budget; reject rather than truncate or label missing |
| Finalizer crashes after reserving a pass | Keep charge consumed; revalidation/repeated passes consume remaining budget |
| Exact retry after accepted/rejected finalization | Return stored receipt/outcome without rehash or quota reset |
| Old candidate exhausted, then user authorizes an exact resume | Preserve old outcome/ledger; only a new candidate under existing generation/32-candidate gates may proceed |
| Readable output has no approved write/read or artifact-producer scope | Deny configuration/capability; the declaration is no access grant |
| Output payload fits but configuration or result JSON is oversized | Enforce every independent byte cap; no attachment/metadata exemption |

**Disposition.** Placement, schema versions, sorting and numeric byte/rehash
limits have matching #741/#744 contracts with scoped owner acceptance.
Producer/storage qualification remains required for those defaults and ledger
semantics. Reporting-action
and request-envelope mappings, artifact wrapper contracts, aggregate run/workspace
and worker/scratch quotas, shared custody, #746 qualification, #740 eligibility,
the separately proposed identity/review-input amendments and approved runtime
plans remain independent gates. Keep #741/#742/#744 open.

## Proposed result and evaluation reporting protocol

This is the proposed #744 protocol with matching
[#741 action projection](2026-10-04-agent-role-configuration-design.md#proposed-assignment-reporting-actions)
and [#742 transport/authority mapping](2026-10-04-orchestrator-preparation-design.md#proposed-assignment-reporting-transport).
Its reviewed actions/envelopes have scoped owner acceptance; joint consumer/
storage qualification remains required. It changes no canonical candidate/result/
evaluation/disposition object or digest domain and
implements no route or tool. Previously accepted evaluation/timing behavior
remains unchanged; earlier technical amendments retain their separate gates.

### Action-specific request objects

All requests have exactly `schemaVersion: 1`, `action`, `idempotencyKey`, and
the action-specific properties below. `idempotencyKey` uses the existing ID
encoding. Every property is required; reject unknown properties, duplicate JSON
keys and invalid lossless integers before persistence. Bearers and UI authority
stay in transport headers, never these objects or their canonical records.
The schema's full action IDs also identify the requested operation; a route,
role, model instruction or claimed body identity cannot substitute for one.

| `action` | Additional exact properties / required authority |
| --- | --- |
| `orkworks.result.submit.v1` | `candidate`: complete canonical `AssignmentResultCandidate`; current `AssignmentResultCapability` for that child/root assignment |
| `orkworks.evaluation.submit.v1` | `evaluation`: complete canonical `AssignmentEvaluation`; current `AssignmentEvaluationCapability` for the reviewer, exact target and evaluation declaration; covers first revision and corrections |
| `orkworks.user-evaluation.create.v1` | `evaluationDraft`, `expectedUserSlot`; Electron-authorized user path; covers initial/replacement stream creation |
| `orkworks.user-evaluation.correct.v1` | `evaluation`: complete canonical user `AssignmentEvaluation`; Electron-authorized user path and exact existing active user stream |
| `orkworks.user-evaluation.invalidate.v1` | `disposition`: complete canonical `AssignmentEvaluationDisposition`, plus `resultDigest`; Electron-authorized user path only, including invalidation of child streams |

`evaluationDraft` has exactly the properties of `AssignmentEvaluation` except
`reviewerIdentity` and `reporterSource`, which are absent. It requires a no-head
predecessor and evaluation revision 1. The sidecar derives user provenance and
atomically issues a new `userReviewId` with the successful slot transition,
then constructs the unchanged canonical evaluation. The caller cannot choose
that identity or impersonate a child. `expectedUserSlot` is exactly
`{"kind":"no-head"}` or
`{"kind":"invalidated","userReviewId":<ID>,"dispositionDigest":<Digest>}`.
The latter must resolve the same result's current slot and accepted invalidation;
it is not the new stream's evaluation predecessor. An exact create retry returns
the original issued identity rather than allocating another one.

For child writes, every supplied identity/runtime/provenance field must equal
the sidecar's capability binding; claimed fields grant no authority. User
correction must retain user provenance and the same issued stream; invalidation
must resolve its supplied result digest and exact current evaluation revision/
digest under UI authorization. Existing reviewer-role/independence, rubric,
source, artifact-output, correction-reason, user-slot and revision-budget rules
apply unchanged. Optional reviewer-performance assessment remains user-only.
There is no external finalization, candidate-edit, child-invalidation, generic
artifact-upload, task-completion or launch action in this protocol.

These writes change result/evaluation state only. Their predecessor CAS is the
candidate's `ResultPredecessor`, evaluation's `EvaluationPredecessor`, explicit
user-slot predecessor or disposition's exact stream head. They have no
`expectedTaskVersion`: reporting does not mutate coordinator task state. The
frozen identity `taskVersion` is not live task CAS metadata. Any separate
coordination transition retains #742's own authority/version/receipt rules.

### Receipt and artifact reads

Read requests have exactly `schemaVersion: 1`, `action`, and the additional
properties below. Reads have no mutation idempotency key and create no stream,
revision or new receipt. Authorization is rechecked for every response.

| `action` | Additional properties / scope |
| --- | --- |
| `orkworks.result.receipt.read.v1` | `mutationIdempotencyKey`; current result capability; resolves only this assignment/generation's `result.submit` admission receipt and candidate/finalization status |
| `orkworks.evaluation.receipt.read.v1` | `mutationIdempotencyKey`; current evaluation capability; resolves only this reviewer/generation/target's `evaluation.submit` receipt |
| `orkworks.evaluation.artifact.read.v1` | `evaluationRevision`, `evaluationDigest`; current evaluation capability; reads only an already published immutable artifact in this capability's one reviewer/target/declaration stream |

The last operation is not an arbitrary artifact-ID lookup or the separate
`AssignmentReviewReadCapability` for worker inputs. A current resumed reviewer
may read retained revisions of its own stream under its newly validated grant;
it cannot replay old-generation mutation receipts or revive old write authority.
The proposed Electron-only candidate inspection below supplies finalization
visibility after producer revocation; it is not an existing implemented route.
Producer read endpoints do not accept UI credentials as substitutes.
The new mutation receipt is returned directly by each successful user operation,
so an exact retry can recover a lost response without a new user stream.

`UserCandidateReceiptReadRequest` has exactly `schemaVersion: 1`,
`action: "orkworks.user-report.receipt.read.v1"`, `assignmentIdentity`, and
`candidateDigest`. Electron main authorizes this explicit selected-subject read
using existing UI authority; the sidecar resolves exact workspace ownership and
the retained candidate. It returns the same result receipt-read reply, including
the original admission and eventual accepted/rejected finalization receipts,
without requiring/reissuing a producer credential or reopening its run. This
applies after producer end, run completion, restart and namespace rotation to
retained read-only history. The finalization outcome may remain pending until
its existing recovery proofs are reconciled. A missing/purged candidate yields
404 with no tombstone/existence oracle outside authorized scope.

The required selected-subject projection supplies the exact candidate identity/
digest under UI authorization; it is a proposed UI/producer mapping, not an
implemented history endpoint. Missing/unverified pointers show unavailable,
never an inferred/latest accepted result. There is no arbitrary artifact read,
filesystem access, source-run control, global listing or mutation in this UI
operation. Other user history/inspection projections remain separate UI work.

A `ReportingMutationReceipt` has exactly `schemaVersion: 1`, `action`,
`idempotencyKey`, `requestDigest`, `recordKind`, `recordDigest`,
`reviewerIdentity`, `recordRevision`, and `artifact`. `recordKind` is
`candidate | evaluation | disposition`. Candidate receipts have null reviewer,
revision and artifact. Evaluation receipts have the derived reviewer identity,
the accepted evaluation revision, and, for child evaluation only, `artifact`.
Disposition receipts name the invalidated reviewer, disposition revision 1 and
null artifact. `artifact` is null or exactly `artifactId`, `version`,
`sizeBytes` and `contentDigest` with the existing encodings; its version matches
the evaluation revision. A receipt never stores or returns a bearer.

A result receipt-read reply has exactly `schemaVersion: 1`, `admissionReceipt`,
`candidateState` (`pending | finalized | rejected | superseded | abandoned`),
and `finalizationReceipt`. The admission receipt is immutable; current state is
a derived projection, not a replacement receipt or result acceptance. The
finalization receipt is null while no accepted/rejected outcome exists, otherwise
exactly `schemaVersion: 1`, `candidateDigest`, `outcome` (`accepted | rejected`),
`result`, and `reasonCode`. Accepted outcome requires `result` containing exactly
`assignmentIdentity`, `resultRevision` and `resultDigest`, and null reason;
rejected outcome requires null result and a bounded nonsecret ID reason code.
It does not include output bytes. Supersession/abandonment keep their existing
stored disposition; neither manufactures an accepted result. Evaluation receipt
reads return the immutable mutation receipt; artifact reads return exactly
`schemaVersion: 1`, `artifact`, and the canonical `evaluation`.

### Authentication, exact retry and atomic writes

1. Enforce bounded transport framing without buffering unbounded input; then
   authenticate the correct credential class/current generation and namespace,
   workspace/run/runtime fences. Unknown/revoked authority cannot inspect even
   an existing receipt. Reject mixed authority headers. Valid authority used
   for another assignment/action is denied, not widened to that scope.
2. Charge bounded request/rate admission immediately after authentication,
   including malformed requests and exact retries, then strictly validate the
   bounded request shape and fixed identity/scope binding before receipt lookup.
   Existing receipt lookup consumes no new candidate/revision/storage quota;
   new-record capacity checks occur only after no exact receipt is found.
3. Compute `requestDigest` as SHA-256 of
   `orkworks.assignment-report-request.v1\n` plus the complete canonical
   validated mutation request, including action and idempotency key, with one
   literal LF. Transport headers/credentials are excluded. Child retry keys
   include authenticated assignment/reviewer, target where applicable, current
   authority generation, action and key. User keys use the fixed authority-class
   tag `workspace-ui`, the stored workspace/admission-epoch namespace, exact
   result subject, action and key; stream/slot values remain in the request
   digest. This local UI authority class is not a per-account principal: token
   bytes, process IDs and Electron session IDs are never retry identity. Token
   renewal/restart preserves this namespace and durable receipts, subject to
   successful current UI authentication. Namespace rotation keeps old mutation
   history read-only and rejects replay under its old epoch. Missing/corrupt
   namespace proof blocks mutation; do not invent a new principal or reset keys. A matching durable receipt returns that exact
   receipt before mutable head, source-freshness or remaining revision-capacity
   checks. Changed canonical content under the same scoped key conflicts.
4. For a new write, revalidate semantic source/rubric/output eligibility and
   current subject/stream/slot rules. Compare the exact predecessor, enforce
   candidate/revision/artifact budgets and commit the canonical object, artifact
   where applicable, dependency edges, stream/slot state and immutable receipt
   atomically. Recheck current authority at that commit under the same fence as
   runtime end/revocation. Revocation first denies persistence; commit first
   leaves the record/receipt durable, without reviving authority after revocation.
5. Finalization remains sidecar-owned over a durable candidate, with #744's
   output/rehash ledger, quiescence, predecessor and atomic publication rules.
   Parent results, reports, process exit or receipt reads never finalize a
   candidate or advance an undeclared dependency. User invalidation/correction
   cannot reopen or resume the worker/reviewer run.

A target head changing after reviewer admission never retargets a capability.
An eligible correction remains about the exact original result; if that result
is now historical it remains historical and cannot affect the new result's
current assessment. New assessed outcomes must still meet the existing evidence
freshness rules. A new current target requires a new approved assignment.
Historical record/receipt inspection is not current evaluation eligibility.

Payload parsing, canonical hashing and staged bounded artifact work stay outside
long-held dependency transactions. Final source validation, authority and CAS/
publication share the existing workspace dependency/producer boundary; inability
to provide that atomic fence blocks implementation. A crash yields either no
accepted mutation or a complete recoverable record/artifact/receipt/edge
transaction. Recovery reconciles that proof before writes, retries or purge;
it never reconstructs a receipt from an orphan artifact or guesses acceptance.
Reads use #742's final response-commitment/revocation cutoff, including receipts
and own artifacts; committed bytes may arrive after revocation, uncommitted
responses are discarded. No blocking network I/O occurs inside that cutoff.

### Finite envelopes, errors and contract cases

Proposed maximum UTF-8 request/reply frames are 540,672 bytes (528 KiB) for
mutation requests and own-evaluation artifact replies, and 16,384 bytes (16 KiB)
for read requests, mutation receipts, receipt-read replies and errors. The
embedded candidate/evaluation/disposition retains its separate 512 KiB cap;
create drafts are bounded by both the frame and completed canonical evaluation
cap after derived fields are added. No multipart/compressed/upload alternative
bypasses these counts. The [proposed reporting quotas](#proposed-reporting-request-and-receipt-quotas)
now supply request/rate/receipt defaults and recovery rules. Their acceptance,
per-request execution/response buffering deadlines and broader storage/worker
qualification remain required gates before code.

Use `201` for a newly committed mutation, `200` for an exact mutation retry or
successful authorized read; `400` invalid schema, `401` missing/revoked authority,
`403` wrong credential class/identity/scope, `404` no record in authorized scope,
`409` predecessor/slot/generation/key conflict or exhausted record capacity,
`413` any independent byte cap exceeded, `429` rate admission denied, and `503`
unavailable/recovering state. Error bodies have exactly `schemaVersion: 1`,
`code` (nonsecret bounded ID), and `message` (at most 2 KiB). They disclose no
cross-scope existence or credentials and grant no automatic retry. No receipt
is created for a rejected new mutation; committed invalidations/corrections are
records and receive receipts. Finalization rejections retain their distinct
sidecar-owned receipts even after producer credentials expire.

These are synthetic contract expectations, not executed runtime tests:

| Case | Required outcome |
| --- | --- |
| Planning/research/report token submits a candidate or evaluation | Deny wrong capability class; no record or task transition |
| Child claims user provenance or requests invalidation | Deny; UI authority is required and provenance is sidecar-derived |
| Same live credential/key/content retries after its head advanced | Return the exact immutable receipt before head/revision-capacity checks |
| Same scoped key changes predecessor, target or payload | Conflict; no overwritten receipt or second mutation |
| Revoked runtime replays a previously accepted key | Reject authentication before lookup; retained receipt is no authority |
| Request quota/rate is exceeded on an exact retry | Deny request admission without consuming a new revision or returning a bypass receipt |
| Two user creations from the same no-head/invalidated slot | One atomic winner; loser conflicts; winner retry retains the issued user identity |
| User replacement names a disposition from another result | Reject scope/slot mismatch; preserve existing history |
| Revocation races publication | One atomic ordering: rejected write or durable object/artifact/receipt/edges |
| Crash occurs between artifact staging and transaction commit | No inferred acceptance; reconcile staged bytes and complete transaction proof before retries |
| Reviewer reads an artifact ID/revision outside its own stream | Deny; immutable existence grants no access |
| Source changes after reviewer admission | Preserve exact historical target and outcome freshness; no latest-result substitution |
| Draft frame fits but sidecar-derived canonical user evaluation exceeds 512 KiB | Reject entire mutation before stream/slot/artifact publication |
| Candidate admission receipt exists but finalization has not completed | Return pending status, not an accepted result or coordination completion |
| Producer is revoked before sidecar finalization completes | Explicit authorized UI candidate inspection reads retained pending/final receipt; producer replay remains denied |
| Electron token renews after a lost user-create reply | Current UI authentication plus unchanged workspace/epoch/key returns the same issued user identity; rotation still fences old mutations |

**Disposition.** Action IDs, request/receipt shapes, proposed carriers and
retry/CAS/revocation order now have matching proposed #741/#742/#744 mappings.
The scoped technical acceptance is recorded above; adapter delivery/effective
restriction evidence, broader run/workspace/worker/storage quotas, artifact wrappers,
shared custody, #746 qualification, earlier identity/read/output proposals and
separately approved implementation plans remain independent gates. No runtime
API, model probe, profile eligibility, scoring authority or automatic retry is
delivered. Keep the component issues open.

## Proposed reporting request and receipt quotas

This is the proposed quota layer for the reporting protocol above, with matching
[#742 admission/recovery](2026-10-04-orchestrator-preparation-design.md#proposed-reporting-quota-admission)
and [#741 eligibility](2026-10-04-agent-role-configuration-design.md#proposed-reporting-quota-eligibility)
boundaries. These logical defaults have the scoped owner acceptance recorded
above; joint storage/consumer qualification remains required. They create no runtime limiter, filesystem space
guarantee, new request field, record/digest version or implementation authority.
They apply to the four proposed assignment-reporting carrier routes only;
research reports, parent proposals/events and retained worker-input reads keep
their separately required contracts and limits.

### Request admission and independent lanes

| Authenticated reporting traffic | Proposed rolling 60-second limit |
| --- | --- |
| Producer mutations per immutable producer assignment | 10, shared by result/evaluation actions and all its resumed generations |
| Producer receipt/artifact reads per immutable producer assignment | 30, shared by its read actions and resumed generations |
| Producer mutations / producer reads per workspace | 120 / 240, in separate pools |
| Electron-authorized user mutations per exact target result | 10, shared by create/correct/invalidate |
| Electron-authorized user mutations per workspace | 30, across targets and token refresh |
| Electron-authorized candidate inspections per workspace | 60, independent of producer pools and user mutations |
| Concurrent authenticated reporting requests per workspace | Four producer slots and two reserved UI slots; no reporting wait queue |

A rate gate keeps at most its limit's admitted-attempt timestamps on a monotonic
clock. Remove entries at least 60 seconds old before testing capacity; an
available entry is consumed before receipt lookup even if the request later
fails, is malformed, or is an exact retry. When full, reject without appending
another timestamp or extending the window. An admitted attempt is never refunded
because later validation, another gate or publication fails. These are rolling
windows, not calendar-minute boundaries or unlimited burst/refill buckets.
A denied attempt creates no receipt, per-key denial record or unbounded log.

Apply the authenticated workspace/action-lane gate immediately after credential
validation, before full body parsing. For producers, consume the workspace gate,
then the immutable producer-assignment gate from the stored capability binding,
then acquire a producer slot before allocating/parsing the full frame. Claimed
request IDs cannot choose a bucket. For UI mutations, consume the coarse
workspace gate, acquire a UI slot, parse the bounded envelope and resolve the
authorized existing result, then consume its result gate before exact-receipt
lookup. Stop at the first denied gate; earlier admitted gates stay charged and
unreached gates are not charged. A
malformed or unknown/cross-scope UI target cannot allocate a new target bucket.
Store no bucket keyed by arbitrary bearer bytes, request keys or unknown IDs.
Counters/buckets exist only for owned scopes and live windows; idle expired
buckets may be removed without dropping unexpired attempts. Existing frame and
authentication limits still reject unknown/revoked authority without exposing
stored receipts. Bounded unauthenticated ingress remains a transport prerequisite.

Changing credentials/generations, pausing/completing a stage, correcting a report
or exact resume does not reset a live scope's window. UI token renewal uses the
same workspace UI lane. After sidecar start/replacement has established exclusive
workspace ownership and fenced prior reporting admission, reject these reporting
requests for one full verified monotonic 60-second cold window before opening
fresh rate windows; old credentials are already fenced. Every pre-fence attempt
is at least 60 seconds old when that interval ends: `attempt <= fence` and
`open >= fence + 60s`, so `open - attempt >= 60s`. This conservatively covers
pre-restart attempts without persisting/comparing process-clock timestamps or
trusting wall-clock adjustments. An unprovable clock keeps admission closed.
Repeated restarts never buy fresh request capacity. Other lifecycle/control
routes, including stop/cancel, are not placed behind this reporting cold window.

Acquire a lane's concurrent slot before allocating/parsing a large frame; keep
it charged until all request/response buffers and any in-flight work are released,
including committed transport buffers. Full capacity returns 429 immediately;
there is no hidden queue or spill into the UI's reserved slots. Stuck work stays
charged until it exits or the sidecar restarts; timeout alone cannot spawn a
replacement. Internal candidate finalization has its separate required worker
budget. These request slots and byte frames do not qualify native I/O deadlines,
transport buffers or scratch/storage bounds.

### Receipt ownership, reservations and finite storage

Retain one immutable mutation receipt per accepted mutation and at most one
accepted/rejected finalization receipt per candidate. Exact retries/reads add
none; validation/conflict/rate/oversize/capacity denials add none. Existing
canonical records and request digests provide the payload binding: never retain
a second raw request body or an unbounded retry log. Stored receipt metadata,
including its owner/routing/key bindings and inline response, is at most 16 KiB
per receipt slot. At most one request-key lookup binding and one canonical-record
pointer belong to a slot, included in that bound; reads, credential renewal and
new runtime generations create no extra lookup aliases. It obeys the smaller
wire-frame limit independently; reject a new mutation if its full required stored
metadata cannot fit.

Candidate admission/finalization receipts belong to the producing assignment.
Evaluation/disposition receipts belong to the evaluated target assignment/result,
even when a reviewer in another run produced the artifact. Charge their target
run's pool, including after that run completes. Reviewer lookup indexes are
bounded references to that one stored receipt, not uncharged receipt copies.
Artifacts/records and their evidence retain their own producing/target ownership
and byte quotas; receipt accounting does not reassign them or authorize reuse.

| Retained receipt capacity | Proposed limit / reservation rule |
| --- | --- |
| Per evaluated/producing assignment | 1,088 slots / 17 MiB including reserved capacity |
| Per owner run | 64 MiB across all its subjects, active or terminal |
| Per selected workspace | 256 MiB across all owner runs/epochs, active or historical |
| Candidate admission | Reserve two 16 KiB slots: admission and its possible finalization receipt |
| User review capacity for a possible new result | At candidate admission also reserve eight 16 KiB slots: four user evaluation writes and four possible user-stream invalidations |
| New child evaluation revision | Reserve two 16 KiB slots in the target subject: its mutation receipt and a possible later user invalidation |

These reservations are hierarchical and atomic with admission/publication: a
slot consumes its full 16 KiB in assignment, owner-run and workspace pools even
when the final receipt is smaller or storage deduplicates bytes. The 1,088-slot
bound is 64 candidate/admission-finalization slots plus twice the existing
512-evaluation-revision ceiling. It is a conservative bound: one disposition per
stream means not every revision needs its reserved invalidation slot. User
capacity is covered by the four-per-result partition, never taken from the
child's twelve revisions. All 32 possible candidate results reserve the user
partition conservatively before result finalization. Explicitly, 32 candidates
times ten slots is 320; at most `32 * 12 = 384` child evaluation revisions times
two slots is 768; together they are 1,088. The 512 evaluation ceiling includes
up to 128 user revisions already covered by candidate/result user credits; it
is not an additional 512 child revisions or 64-candidate allowance.

A candidate therefore needs ten slots (160 KiB) at admission. Finalization uses
its already reserved outcome slot without another storage-admission check. If
it publishes a result, transfer the eight user slots atomically to that exact
result. A terminal rejection/supersession/abandonment that can no longer publish
a result may release those eight unused user slots with its durable outcome;
keep the candidate's admission/outcome reservation while its history is retained.
The eight-credit transfer remains within that same assignment/owner run, with no
second charge or capacity release: only the credit owner becomes the exact
accepted result. No other request may consume an admitted result's unused user
slots. User
create/correct uses its four reserved write slots; user-stream invalidation uses
its reserved invalidation slots. Invalidating a child stream uses the slot
reserved for that stream's first accepted child revision; unused correction
invalidation reservations remain protected until safe owner deletion.

Each result's four user-write credits cover **all** initial, replacement and
correction writes together, not four writes per stream. Creating a user stream
uses one write credit and earmarks one of the four user-invalidation credits to
that issued stream; correcting it consumes only another write credit. Invalidating
it consumes its earmarked credit once and freezes it. Child-stream invalidation
uses only that child's first-revision credit, never a user-stream credit.
Correction-reserved child credits remain conservative unused capacity; they
cannot fund a different stream, second invalidation or extra evaluation revision.
Every transfer/consumption is recorded under the same target-result owner.

A full pool rejects a **new** candidate/child-evaluation reservation with 409
`reporting-capacity-exhausted` before any record, artifact or receipt commits.
It does not prevent an admitted candidate's finalization receipt, an eligible
reserved user write/invalidation, an exact retry/read, or cancellation. Those
operations retain their authority/rate/CAS/revision/source checks and may still
fail for another reason. Reservations are logical capacity, not proof that the
filesystem can durably write; storage-unavailable/recovery still fails closed.
Do not raise limits, evict protected evidence or change an approval to make room.
Capacity exhaustion is a reporting blocker, not evidence of poor agent quality
or task completion/failure. Preserve ordinary work and existing outcome evidence;
a denied submission creates no candidate or automatic retry/resume permission.

### Recovery, retention and visible exhaustion

Keep charged/reserved/consumed slot ownership as bounded durable metadata outside
canonical report objects and their digests. Capacity reservation, receipt/record
publication, user-credit transfer/release and dependency updates share the same
reviewed transaction/recovery fence as the owning mutation. A crash exposes
neither an uncharged receipt nor an irreversible reservation for a mutation that
never committed; uncertain transaction proof fences new writes until reconciled.
Do not reconstruct quotas from only receipt byte sizes: unused but owed credits
must remain charged. A missing/corrupt ledger or ambiguous storage outcome yields
503 for affected mutations; verify retained receipts before retry/read service.

Rate windows may restart only after the cold interval above; durable receipt
capacity does not reset on restart, runtime resume, token refresh, run completion
or namespace rotation. Old epochs stay read-only but still consume workspace
capacity. Explicit eligible subject purge/retention releases only that owner's
receipts and unused credits atomically with deletion under #744's incoming-
dependency fence. Rejected proposals and terminal runs are not blanket release
authority. An index/reference needed by retained history or a current consumer
keeps its target protected; no quota compaction may erase provenance. Workspace
deletion/shared custody retain the existing joint gates, not a capacity bypass.

Use 429 with a bounded rate/concurrency/cold-window reason for temporary request
admission denial, 409 for new-receipt capacity exhaustion and 503 for unverified
recovery/storage. The existing error-frame shape and 2 KiB safe message bound
are unchanged. An authorized producer receives only a bounded reason for its
scope; it cannot inspect other assignments' usage. User diagnostics may explain
the applicable quota and safe retention/recovery options under existing UI
authority, without a new arbitrary listing/control action. Waiting for a rate
window restores only request capacity, never revision slots, receipts, approvals
or capabilities. No automatic retry, resume, purge or cancellation is authorized.

These are synthetic contract expectations, not executed limiter/storage tests:

| Case | Required outcome |
| --- | --- |
| Exact accepted retry is the eleventh producer mutation inside 60 seconds | 429 before receipt lookup; no new revision, receipt or denial history |
| Attempt fails after coarse admission but before subject validation | Keep its coarse charge; no unknown-subject bucket or refund |
| Credential refresh or exact resume occurs inside a live rate window | Same immutable scope/window; no burst reset |
| Sidecar restarts or wall clock moves backward | Monotonic cold window before request admission; durable receipt credits remain charged |
| All producer slots are busy | Reject extra producer requests; preserve reserved UI slots and independent stop/cancel routes |
| Finalization occurs after another mutation fills the receipt pool | Use its reserved outcome slot; producer revocation remains enforced |
| Child writes fill otherwise unreserved receipt space | Preserve accepted results' reserved four user writes and stream invalidations |
| Candidate is rejected before it can produce a result | Atomically release only its unused eight result-user credits; retain candidate history/outcome capacity |
| Child stream is corrected before user invalidation | Use its first-revision invalidation credit once; other retained credits do not create extra disposition rights |
| Exact retry/read arrives when durable receipt pool is full | Apply current auth/rate limits, then return existing receipt without new storage capacity |
| Capacity-admission transaction is interrupted | Recover atomic reservation/object/receipt proof or fence writes; no uncharged acceptance or guessed refund |
| Namespace rotates with historical subjects retained | No old mutation replay or workspace-capacity reset; authorized history reads remain scoped |
| Eligible subject purge races a retained receipt dependency | Existing dependency boundary protects the target or releases owner capacity with deletion, never both |
| Rate window expires after record/revision capacity is exhausted | New request admission is possible; exhausted durable record/receipt capacity stays exhausted |

**Disposition.** Reporting rate, concurrency and receipt reservation/exhaustion
rules have concrete owner-accepted defaults across #741/#742/#744.
Ledger/store/clock/worker qualification, broader artifact/evidence/read/run/
workspace/scratch resource quotas, #745/#746 custody/recovery, #740 eligibility,
earlier technical amendments and separately approved implementation plans remain
gates. The receipt pool is separate from #742's 32 MiB preparation/report-metadata
limit; neither replaces the other's checks or bounds all workspace storage.
No runtime limiter, endpoint, profile or automatic maintenance is delivered.

## Joint technical review handoff — 2026-10-24

**Disposition: scoped #741/#742/#744 technical choices owner accepted on
2026-10-24; not an executable implementation handoff.** See the
[acceptance record](#technical-contract-acceptance--2026-10-24). Review base:
`f1a298dfa9cb0a7d03d1116fd9af04faaf0be900`. This joint static pass checked the
identity, read-input, output, reporting and quota amendments across #741/#742/
#744 against the parent product flow, accepted ADR 0077 and the proposed #746
transaction/read contracts. Two bounded read-only readers checked identity/
approval and storage/custody interactions; the owning session reconciled the
findings and acceptance inventory below. It ran no native crash/model probes.

### Integration findings and disposition

| Finding | Disposition |
| --- | --- |
| #742 review admission and a #744 fixture still name child v2 after structured outputs require v3 | Corrected to executable output-bearing child v3 with the embedded typed input; v2 remains its original input-only proposal and gains no output/reporting grant |
| Parent product flow and the small-task example imply future-result review is executable in the worker's original plan | Reconciled with the already accepted timing choice: record the intention, finalize the result, then require a separately UI-created review run and exact approval; no old-run continuation or approval transfer |
| A 16 MiB source-payload example can be mistaken for an eligible 16 MiB complete storage command | Explicitly count newly written output bytes together with metadata/receipts/edges/lineage under the command cap; reject when the combined change cannot fit. No limit increase, external-store exception or maximum-profile eligibility is inferred |

The storage ceiling is a separate, stricter composition limit, not an extra
16 MiB allowance for snapshots. Before qualifying a concrete producer profile,
prove its complete atomic publication fits all source, command/row, lineage,
queue, scratch and main-store limits. A narrowed approved output contract or a
separately reviewed change to the storage design may be necessary. Native
payload placement/publication and worst-case overhead proof remain unresolved
qualification work; this review does not select an external object store.

### Acceptance boundary and remaining gates

| Contract / decision | Current disposition |
| --- | --- |
| Evaluation behavior/defaults, including 12 child / 4 user revisions per result | Owner accepted on 2026-10-23; unchanged by this review |
| Exact-result timing and the separate user-created review run for future output | Owner accepted in PR #999; parent/example drift corrected here |
| Frozen `assignmentTaskVersion` reservation snapshot and exact attachment proof | Owner accepted on 2026-10-24; producer/storage proof remains open |
| Typed review input, independent retained-read capability and dependency protection | Owner accepted on 2026-10-24; executable review uses child v3 and qualification remains open |
| Output-bearing child v3 / root bootstrap v2, structured declarations and byte/rehash ledger | Owner accepted on 2026-10-24, including source defaults; producer/native/storage qualification remains open |
| Reporting action IDs, routes/envelopes, exact receipts, user retry namespace and inspection | Owner accepted on 2026-10-24; adapter delivery and Electron/preload integration remain open |
| Request windows, cold restart, reserved receipt credits and 17/64/256 MiB receipt pools | Owner accepted on 2026-10-24; clock/worker/storage qualification remains open |
| Hierarchy recovery behavior versus its SQLite mechanism | Recovery behavior has its separate owner acceptance; ADR 0090's SQLite/private-worker DELETE/EXTRA direction is separately accepted for further design/qualification only; detailed choices/defaults, producer agreement and native qualification remain open |

No unchanged canonical candidate/result/evaluation/disposition domain was
rewritten. The review found no further contradiction in the frozen/live version
separation, capability classes, receipt-before-CAS retry order, target history,
1,088-slot arithmetic, cold-window derivation or protected-source retention.
Those are written-contract checks, not implementation or capability evidence.

The scoped technical amendments above are accepted. Before runtime planning,
resolve the separately gated broader component choices and complete the
artifact-wrapper and broader evidence/read/worker/scratch/storage quota and
publication contracts, and resolve #745/#746 joint custody/recovery qualification.
#740 still has no eligible role/profile and capped evidence work remains deferred
until November; #741 prohibits a runtime launch plan without that evidence.
Any later executable slice needs its own approved implementation/qualification
plan and canonical/route/clock/crash fixtures. Provider costs remain absent
without an approved authenticated usage source; shared learning remains gated.
Keep #741/#742/#744/#746 open. This handoff records scoped technical acceptance;
it does not itself accept SQLite architecture/limits, close unchecked implementation
criteria, approve runtime code or grant automatic maintenance.
