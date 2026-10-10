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

Evaluation binds one of the approved assignment identity variants, plus the
exact result revision, criteria and rubric versions, reviewer identity, and
immutable evidence references. A child result uses the #741/#742 identity
`(workspaceId, runId, planId, planRevision, taskId, taskVersion,
reservationId, parentSessionId, childSessionId, configurationId,
configurationDigest)`. A root orchestrator result uses
`(workspaceId, runId, rootAssignmentId, parentSessionId, bootstrapId,
bootstrapConfigurationDigest)` from its UI-approved
`OrchestratorBootstrapConfiguration`; root assignments have no child task,
reservation, or child session. These values come from approved state, not
display labels or reporter claims. A change to any assignment-identity member
or rubric requires a new approved revision. `sidecarGeneration` and the
active child `launchGeneration` or parent runtime generation fence report
authority but do not change the assignment identity.

Result manifests use an `AssignmentResultCapability`, separate from #742's
`ResearchReportCapability`. The sidecar binds it to the approved result
identity and active authority generations, and it grants only result-manifest
submission and receipt reads. For child results it binds the child assignment,
sidecar generation, and launch generation; for root results it binds the
bootstrap identity, sidecar generation, and active parent runtime generation.
It grants no research-report, evaluation, or orchestration action. Resume and
revocation follow #742's generation-bound mechanics. Reconcile the new result
scope with #742 before implementation; a run bearer or execution grant is not
report authority. Root evaluations use the explicit user-review path unless a
separately eligible reviewer is approved.

The output contract is assignment-specific, with no global artifact catalog.
It declares at most 32 unique artifact IDs and kinds, using #741's identifier
rule. Reconcile this schema and limit with #741. Every declaration is required
to pass and includes an explicit finite byte cap. The approved configuration
also has a finite total rehash-work cap; values above the server's hard limits
are rejected. An output that exceeds a cap is unsupported and cannot establish
a pass. Exact hard limits are an implementation-plan decision to reconcile
with #741 before code. Example declarations include `changes`
(`workspace_changes`) and `checks` (`verification_report`), or `findings`
(`research_report`).

The worker reports one `present` or `missing` entry per declaration through
its `AssignmentResultCapability`; the payload cannot choose the identity or
active sidecar and launch generations. A present entry names an approved
worktree-relative path or immutable server-held artifact ID/version, its size,
and content digest. A server-held artifact must have been created under this
exact assignment identity and output declaration ID/kind. The sidecar verifies
that binding and the artifact's immutable version, size, and digest; a reference
produced by another assignment is unsupported in version 1. Cross-assignment
artifact reuse requires a separately approved source declaration and is not
implied by possession of an artifact ID. Undeclared or over-cap outputs are
rejected. Exact retries with the same key return the
stored receipt before checking whether the predecessor is still current;
changed content under that key conflicts. The key is scoped to reporter,
assignment identity, active launch generation, and operation.

The sidecar must resolve and open file paths within the approved output scope,
reject symlink or junction targets outside it, and preserve the opened-object
binding for hashing. Only regular files are supported. Every potentially
blocking filesystem operation—including path resolution, open, metadata checks,
reads, and hashing—runs in an isolated worker with a fixed finite concurrency
limit, bounded admission, and a deadline. The sidecar request and lifecycle
paths never wait for that worker past the deadline. Nonblocking flags do not
count as a cancellation or latency guarantee. If an operation misses its
deadline, the output is unsupported and Unassessed; a stuck worker remains
charged against the finite capacity until it exits or the owning sidecar is
restarted. The implementation must not spawn replacement workers or queue
unbounded work around a stuck operation. Exact worker, queue, and deadline
limits are fixed hard limits in the reviewed implementation plan before code.
On every freshness check, the worker resolves and opens the declared path again
within scope, verifies that it still names the same object, then hashes that
open handle. A declared required artifact that is absent at submission or
confirmed absent during freshness checking is known missing and establishes
Needs rework. A replaced path, changed bytes, inaccessible path, timed-out
operation, or unprovable binding is stale/unsupported and Unassessed; no
out-of-scope file may be read or pinned.

Every manifest submission carries the caller's expected current result
revision and digest; the initial state uses `expectedResultRevision: no-head`
with no digest. The sidecar compares both with the current head before
assigning an immutable revision, rejecting stale predecessors. After the sidecar
authenticates the active launch generation, an exact idempotent retry is
resolved before this comparison. The 32-revision limit is per assignment
identity and persists across resumes; resumed writes require the new launch
generation. A changed assignment identity creates a new subject. Exhaustion
is visible and cannot wrap or reset. Evaluations bind the current revision and
digest. Before any
consumer treats a result as current, the implementation revalidates its
referenced output bytes and revision; if it cannot establish freshness, the
result is Unassessed. A known missing output establishes Needs rework directly;
inaccessible, changed, unknown, or unsupported output is Unassessed unless
another uncontested failure exists. A later result revision makes earlier
evaluations historical. The implementation plan chooses the transaction or
revalidation boundary; stale writes cannot restore a current result. This
validates content freshness, not authorship or OS-level confinement.

## Criteria, completeness, and quality

Each criterion has a stable ID, description, and `required` or `optional`
status. New assignments require at least one required criterion. Criteria and
classification are immutable after launch. Optional results are details only and
do not affect completeness or the overall result.

An evaluation provides exactly one `satisfied`, `unsatisfied`, or `unassessed`
outcome for every approved criterion, with a bounded rationale. Satisfied and
unsatisfied outcomes require evidence; an unassessed outcome may omit evidence
and records why it could not be assessed. Evidence references bind an immutable
content digest or report ID/version/digest.
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
and, for report evidence, current source correction/invalidation state.
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
| **Needs rework** | A current, uncontested required criterion is unsatisfied, quality is below `3`, a current, uncontested required-rework finding is present, or a declared output is known missing. |
| **Unassessed** | No failure is established, but a required criterion or quality is unassessed, the result is stale, no eligible reviewer exists, or relevant credible evidence conflicts. |
| **Meets requirements** | Every required criterion is satisfied, quality is `3`, evidence is current, and no relevant conflict or invalidation remains. |

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

An evaluation record stores criterion and dimension outcomes, quality, findings
and required rework, evidence, reviewer/source identity, observation time, and
retry metadata, including the sidecar-derived `rubricSnapshotDigest`; the server
derives the overall result. Each finding has a
stable ID, concise description, location, severity, evidence reference, and
whether it requires rework, matching the approved review-role output contract.
Rationale, findings, and corrections must not contain credentials, secrets,
hidden reasoning, full prompts, or complete transcripts; use safe, immutable
evidence references.

Cost and elapsed-time observations are outside this contract. #745 may define
their bounded integer encoding and allowed units before a later contract
accepts or consumes them.

## Reviewer eligibility and disagreement

A review comes from a declared `review` or `verification` assignment with an
eligible rubric role, or from an explicit user-authorized review. Worker
self-assessment stays separate and cannot affect the result. The current #740
register has no verified child-review profile, so child reports are ineligible
until #740 provides version-specific evidence and #741 binds an eligible
profile. Until then, runtime review uses the explicit user path.

An eligible child reviewer must have a different task, allocation, session, and
configuration from the worker; be outside the potential-contributor set derived
from approved allocations and effective write scopes; and have read access to
the exact result without write access to its output scope. The sidecar—not the
worker—establishes these facts. If it cannot establish the contributor set or
read-only scope, the child report is ineligible and the result stays Unassessed
pending user review. This is assignment-level separation, not OS isolation; a
parent summary alone is not independent review.

Child reports use a reviewer-scoped capability bound to the reviewer
assignment/configuration and exact result revision. It is separate from the
worker's reporting capability and #742's `ResearchReportCapability`; follow
#742's generation, revocation, and retry rules. Resolve exact transport and
record alignment with #742 before implementation. User reviews use the
Electron-authorized user-provenance path and never impersonate a child.

Each evaluation stream has a server-resolved `reviewerIdentity`: either the
assigned reviewer's `reviewerAssignmentIdentity` or a UI-issued `userReviewId`
for an explicit user-authorized review. The ID is stable for that stream and
cannot be supplied as arbitrary display text. A user correction to a child
review is a separate user-authorized evaluation stream, never a revision that
impersonates the child.

A reviewer cannot evaluate their own work. A reviewer may be evaluated by a
different declared reviewer or the user. Do not assign a further child to
evaluate that review; the user is the terminal evaluator. Without user
disposition, leave that evaluation Unassessed. This preserves the parent
design's reviewer-of-review path without unbounded recursion.

A child-authored evaluation affects the worker result only while it has a
current terminal user assessment of the reviewer's own assignment result that
also covers that exact child evaluation. The terminal user evaluation stores a
sidecar-resolved `reviewedEvaluationRef` naming the worker assignment,
`reviewerIdentity`, evaluation revision, and digest the user assessed. The child
evaluation is committed and digested before this user evaluation is submitted,
so the reference is acyclic and cannot be part of the child evaluation it
names. The sidecar derives `reviewerAssessmentRef` as an eligibility projection
linking that exact child evaluation to the reviewer assignment identity, result
revision/digest, and terminal user evaluation stream/revision/digest; it is not
a reviewer-supplied field in the child evaluation record or its digest. Both
references must agree, and the linked user evaluation must derive Meets
requirements. A missing, stale, invalidated, Unassessed, or Needs rework
reviewer assessment makes that child evaluation ineligible; derive the worker
result from remaining current evaluations. A new reviewer-result revision, a
correction/invalidation of the terminal user evaluation, or a new child
evaluation revision invalidates the link. A replacement child evaluation needs
a new terminal user assessment naming its exact digest before it can affect the
worker result. User-authored evaluations of the worker do not need this
reviewer-assessment link.

Keep eligible evaluations separate. Disagreement on a required criterion, a
required-rework finding, or a material fact relevant to a required criterion
or quality outcome reports Unassessed unless another uncontested failure
establishes Needs rework. Optional-only disagreements and disputes about
findings that do not require rework are detail and do not affect the overall
result. A reviewer contests a finding with
up to 32 `findingDisputes` per revision; each cites the target
`(reviewerIdentity, evaluationRevision, findingId)` and one evidence
reference already in the disputing evaluation. The target must be a different
eligible evaluation for the same assignment and result revision. A dispute is
current only while both referenced revisions and the evidence for the target
finding and dispute remain current and verifiable; correction needs a new
dispute. If dispute evidence becomes stale, the dispute no longer suppresses a
current finding. Reject duplicate finding IDs within an evaluation revision.
Reusing an ID in a correction means it is the same logical finding; a
materially different finding gets a new ID. Disputes always name an exact
revision and do not carry forward to a correction. Without an explicit
dispute, findings are separate.
Preserve ratings; do not average or prefer by time or seniority.

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

When #742 ends a run and revokes child authority, that child can no longer
correct its review. The user may submit a separate user-authorized evaluation;
the user never impersonates the child. Invalidation preserves the report and
provenance and excludes it from current results and learning. A later result
revision makes earlier evaluations historical; they never become current
again. Before using an evaluation as current evidence, revalidate each
referenced report's current correction and invalidation state as well as its
existence, scope, version, and digest. A corrected or invalidated source makes
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
outcomes from the same evaluation. If a stale outcome belongs to a terminal
reviewer assessment and makes that assessment stop deriving Meets
requirements, any child evaluation linked to it becomes ineligible. A stale
optional-only outcome that leaves the terminal assessment at Meets requirements
does not break the link; this dependency does not stale unrelated outcomes.

## Record digests

Version 1 record digests are lowercase SHA-256 hex over the #741 recursive
canonical JSON bytes, prefixed respectively by `orkworks.assignment-result.v1\n`,
`orkworks.assignment-evaluation.v1\n`, or
`orkworks.assignment-disposition.v1\n` (each ends in one literal LF). Result
and evaluation digests include immutable identity, predecessor revision/digest,
and semantic fields, including `rubricSnapshotDigest` and any
`reviewedEvaluationRef`. The disposition digest includes assignment identity,
result revision, evaluation stream identity, target evaluation revision/digest,
disposition revision 1, action `invalidated`, and actor `user`; it has no
predecessor. All record digests omit their own digest, bearer credentials,
request idempotency keys, mutable status, observation time, and sidecar-derived
eligibility projections such as `reviewerAssessmentRef`. The fixed domains
prevent these record kinds or later versions from sharing a digest namespace.

## Bounds, retention, and deletion

Use #741's limits of 32 criteria and 16 rubric dimensions. Each assignment
identity allows at most 32 result revisions across resumes and 32 output
artifacts. Each result revision allows at most 16 evaluation revisions:
child-authored streams share at most 12 revisions, and user-authored streams
share the remaining 4 revisions, including corrections. Each user stream allows
at most 2 revisions. A replacement stream may start only after the previous
stream is invalidated and while user capacity remains. Child streams cannot
consume the user allocation. Since each stream needs an initial revision, the
four-revision budget also bounds the total number of user streams. The
assignment-wide cap is 512 across its 32 result revisions, permitting every
result revision to receive an initial child evaluation and a user evaluation.
Each evaluation revision allows at most 32 findings, 32 finding disputes, and
113 evidence references per evaluation revision, enough for maximum criteria,
dimensions, findings, distinct dispute evidence, and a separate result-level
rating reference. Cap
the manifest and each evaluation/disposition at 512 KiB, each reference at
1 KiB, and rationale/finding/correction text at 2 KiB.
Enforce finite aggregate run/workspace admission quotas; reject exhausted
capacity visibly without evicting history. Output and aggregate quota values
must be reconciled with #741
and #745 before implementation. Per-artifact bytes and aggregate rehash work
have finite approved caps bounded by server hard limits; exceeding either
makes the output unsupported.

Reject over-limit reports visibly and never silently drop conflict evidence,
corrections, or provenance. Retention belongs to #745, which may remove a
complete eligible historical subject and its pinned evidence but must retain
evidence referenced by any current evaluation, regardless of its derived
result. To release capacity from a closed ineligible subject, #745 must also
provide an explicit Electron-authorized user purge. A subject is closed only
after its owning run is terminal and all writer capabilities are revoked; it is
ineligible when none of its results can be selected as current or used as
learning input. Purge requires the exact subject identity and expected current
head, is serialized against writes, and is allowed only when no retained record,
learning input, or reviewer credibility link depends on the subject or its
pinned evidence. It atomically
removes that subject's result manifests and receipts, evaluations,
dispositions, and solely referenced pinned evidence; otherwise it fails without
deleting anything. A repeated purge reports already purged and cannot recreate
history. No tombstone is retained after this explicit provenance deletion, and
the released records no longer count toward admission quotas. Retention must
never purge automatically to make room or silently discard provenance. The
implementation plan must define the dependency scan and transactional deletion
seam before implementing this purge.
Workspace deletion fences new and in-flight result/evaluation writes, then
purges every assignment-result manifest revision and receipt, evaluation,
disposition, and pinned-evidence record. The deletion fence prevents stale
writers from recreating deleted records; no assignment-result or evaluation
tombstone survives. If evidence is gone, a retained projection cannot claim a
current reviewed result. Ordinary non-orchestrated sessions acquire no
assignment evaluations.

## Product boundaries and verification

Assignment evaluation is separate from Taskmaster `CoordinationResult` and the
`CompletionPacket` for `ImproveWorkflow` recommendations. It cannot advance
undeclared dependencies or mark user acceptance, merge readiness, or
integration. #746 may show the overall result by default and details on request;
presentation must distinguish Unassessed from failure and missing usage from
zero quality. #745 defines learning eligibility. Stale, invalidated,
conflicting, blocked, interrupted, or sparse evidence cannot be treated as a
clean learning signal, and evaluation cannot change an active configuration.

A future implementation must verify that:

1. Complete evidence derives Meets requirements; any uncontested failure,
   including a required artifact confirmed absent at submission or during
   freshness checking, derives Needs rework; inaccessible or stale output or
   required criterion/quality evidence without a known failure derives
   Unassessed. Stale finding evidence downgrades only that finding and the
   overall result is recomputed. An
   evidenced rating below `3`, a below-standard dimension, or a required-rework
   finding is a failure even when other dimensions are unassessed. Optional
   criteria, lifecycle state, empty legacy criteria, or a rubric with no quality
   dimensions cannot create a pass. A legacy empty rubric remains Unassessed
   only if no independent failure establishes Needs rework. Correcting
   optional-only evidence cannot
   change the overall result; correcting required evidence makes only its
   dependent outcome Unassessed and the result is recomputed. An optional source
   used only for optional detail cannot downgrade an otherwise passing required
   result.
2. Exact retries using the active `AssignmentResultCapability` generation
   return their saved receipt before stale-predecessor rejection; a
   `ResearchReportCapability` cannot submit result manifests. Concurrent first
   evaluations with different idempotency keys cannot both create a stream head;
   new streams compare-and-swap the explicit no-head state. Changed, malformed,
   unauthenticated, oversized, or cross-subject reports fail closed. Missing
   dimension coverage or invalid evidence cannot support a pass; unassessed
   criteria and dimensions may omit evidence with a bounded reason. File links
   cannot escape the approved scope, and each freshness check detects removed
   or replaced paths. A server-held artifact from another assignment or
   declaration is rejected even if its ID, size, and digest are valid. All
   filesystem operations run in an isolated,
   fixed-capacity worker with bounded admission and deadlines; a timed-out
   worker cannot block sidecar control paths or cause unbounded replacement
   workers. Non-regular filesystem objects are rejected without hashing.
   A deliberately stalled filesystem operation returns by the caller deadline,
   and repeated stalled operations never exceed the worker/admission bounds.
   Results cannot remain
   current for any consumer after referenced bytes change. A resume retains the
   assignment subject and revision count but requires its new launch generation;
   changing the assignment identity creates a new subject.
3. Scores compare across assignments only under the same rubric ID, version,
   and canonical `rubricSnapshotDigest`, or an approved normalization rule.
   Matching ID/version with changed snapshot content cannot pool scores; equal
   digests can compare, and an approved normalization follows its own versioned
   rule. A satisfied versus unassessed required outcome or rating derives
   Unassessed, while an independent current failure still derives Needs rework.
   Optional-criterion disagreement affects
   detail only. Finding disputes bind the exact child or user
   `reviewerIdentity`, evaluation revision, and finding ID and reuse cited
   evidence; all finding and dispute evidence is revalidated for current source
   disposition before it affects the result. Stale finding evidence downgrades
   only that finding and the overall result is recomputed from remaining current
   outcomes; stale dispute evidence cannot suppress a current finding. Duplicate
   finding IDs are rejected, disputes do not carry to corrections, and
   informational finding disputes do not affect the overall result. Disputed
   failures are not established.
   Findings retain location, severity, evidence, and required-rework status
   without prohibited sensitive content.
4. Self-review, parent synthesis, unverified profiles, contributors, stale
   output, or unverified read-only scope cannot qualify as child review.
   Reviewer-of-review depth is bounded and ends with user authority. A
   child-authored evaluation requires a current `reviewerAssessmentRef` to the
   exact reviewer result and terminal user evaluation deriving Meets
   requirements. That user evaluation binds the exact child evaluation
   revision/digest it assessed; a changed child evaluation cannot inherit the
   prior assessment, and the linkage is acyclic. Changing the child evaluation
   digest without a new terminal assessment makes it ineligible even when the
   reviewer assignment result is unchanged.
5. Corrections preserve reporter provenance; user corrections use
   Electron-authorized compare-and-swap on their `userReviewId`; a replacement
   stream can start after invalidation while user capacity remains. Ended child
   capabilities cannot correct or impersonate a reviewer. Concurrent
   corrections and invalidations cannot restore an invalidated evaluation or
   exclude a newer revision.
6. Per-assignment result-revision and record caps reject excess work visibly,
   preserve the user-review revision allocation, allow replacements until its
   four-revision budget is exhausted, and include the result-level rating
   evidence reference; aggregate run/workspace quotas also reject excess work.
   Each accepted invalidation creates exactly one immutable disposition bound
   to the assignment, result, stream, evaluation revision/digest, and user
   authority. Its digest inputs and no-predecessor revision are defined; exact
   retries return the saved receipt and stale invalidations cannot replace a
   newer evaluation.
   Retention preserves evidence referenced by any current evaluation
   regardless of outcome and removes eligible complete historical subjects.
   Explicit user purge releases a closed ineligible subject only when no
   current record, learning input, or reviewer credibility link depends on it;
   it fences writes, removes the subject and solely pinned evidence atomically,
   and fails without deletion when provenance is still needed. Quota pressure
   never triggers silent eviction. Workspace deletion purges result manifests,
   receipts, evaluations, dispositions, and pinned evidence.
7. Evaluation cannot launch/retry work, widen permissions, change
   configuration, advance dependencies, accept work for the user, or approve a
   merge.
8. Child and root result identities resolve from their respective approved
   configuration; child launch and parent runtime resumes require their new
   authority generations. When a source report is corrected or invalidated,
   only outcomes citing it become stale; unrelated current outcomes remain
   usable, and optional-only changes cannot alter the overall result.
   Staleness that makes a terminal reviewer assessment stop deriving Meets
   requirements removes eligibility from dependent child evaluations; stale
   optional-only detail does not when the assessment still derives Meets.
   Server-held artifacts are bound to the exact
   assignment and output declaration. The dedicated `AssignmentResultCapability`
   is distinct from `ResearchReportCapability` and any run bearer. Each of the
   32 permitted result revisions can receive a child evaluation and a user
   evaluation within the 12-child/4-user partition of the 16-revision
   per-result evaluation cap. User replacements can continue after invalidation
   until the four-revision allocation is exhausted; each user stream has at most
   two revisions, and the total budget bounds the number of streams.

## Implementation gate

No runtime plan is approved here. Write one only after written review of this
contract and #741, reconciliation of #740's capability disposition, and
alignment with #610 and authoritative specs/ADRs. Resolve output-byte ceilings,
reviewer recursion, and post-run correction authority against #741/#742 and the
parent product contract. The plan then chooses storage, reporting, correction
and invalidation boundaries, #745 retention, and #746 projection from verified
module seams. Code references here are investigation pointers, not
implementation commitments.

User approval of this spec approves only the written contract—not runtime
implementation, role support, assignment launches, automated evaluation, or
changes to ordinary Peon/session behavior.
