# Assignment quality and completeness evaluation

## Scope and status

This document proposes a logical contract for evaluating one configured agent's
result on one approved assignment. It is a specification and research handoff
for issue [#744](https://github.com/Rambolarsen/orkworks/issues/744) in the
agent hierarchy and configuration learning initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738).
It does not define runtime routes, database layout, UI controls, or implementation
availability.

The contract consumes the proposed immutable role and assignment identities in
[#741](https://github.com/Rambolarsen/orkworks/issues/741), the preparation and
report lifecycle in [#742](https://github.com/Rambolarsen/orkworks/issues/742),
and the approved ordinary-child scope in
[#610](https://github.com/Rambolarsen/orkworks/issues/610). At this writing,
#741 remains under review; its exact role, criterion, and rubric interfaces must
be reconciled before an execution plan can be approved. Coding-tool support in
[#740](https://github.com/Rambolarsen/orkworks/issues/740) is a separate launch
eligibility gate. This proposal makes no claim that a role or evaluation adapter
is supported.

Assignment evaluation answers: **Does this inspected result satisfy its
approved requirements and its declared quality standard?** It remains separate
from Taskmaster coordination results, skill usage evidence, process status,
user acceptance, and Git integration. Evaluation cannot approve a plan, launch
or retry work, widen a permission profile, advance an undeclared dependency,
accept a result for the user, or authorize a merge.

The contract describes immutable evaluation snapshots and provenance-preserving
correction/invalidation semantics. A later implementation plan decides which
store and reporting interface carry them. It must not infer that this logical
history requires a particular database, event-sourcing framework, or route.

## Approved criteria and role rubrics

An evaluation consumes the criteria and rubric included in the exact
user-approved `AssignmentConfiguration` described by #741. The assignment
identity includes the approved repository/workspace binding, run, plan ID and
revision, task ID, attempt/allocation identity, configuration ID and digest,
and the assignment's immutable acceptance-criteria and rubric snapshots.
Display labels are not identity.

Each acceptance criterion has a stable ID, description, and `required` or
`optional` classification. New assignments need at least one required
criterion; approval rejects an empty or optional-only set. Optional criteria
may be assessed and shown, but do not change completeness or the overall result.
An evaluator cannot add, remove, reword, reclassify, or reorder approved
criteria after launch. A changed assignment or rubric requires a newly approved
plan/configuration revision.

The versioned role-specific rubric is part of the approved configuration. It
binds a stable rubric ID, version, content digest, declared evaluator role, and
its quality dimensions. The assignment, not an evaluator, determines which
rubric applies. Evaluation reports identify the exact rubric snapshot and may
cite its dimensions; they cannot replace the rubric or introduce a universal
agent-ability scale. Quality describes this assignment result only. It does not
rank a role, persist as agent XP, or imply that an equipped skill caused the
result.

## Result subject and identity binding

An evaluation is valid only for the exact assignment and result revision it
inspected. Its subject binds repository/workspace identity, run, worker plan ID
and revision, task ID, allocation/attempt ID, worker session identity, assignment
configuration ID/digest, approved criteria/rubric snapshot identities, and the
result-revision ID/digest. It also binds the reviewer assignment/configuration
identity and immutable references to the evidence actually inspected. Display
labels are not identity.

The assignment's approved output contract declares a bounded set of stable
artifact IDs and kinds. A result is a versioned `AssignmentResultRevision`:

- The worker submits an output manifest through the authenticated, exact-task
  reporting authority described by #742. The sidecar derives workspace, run,
  plan/revision, task, allocation, session and configuration identities from
  that authority; payload fields cannot select or override them.
- The manifest contains one unique entry for every declared output artifact,
  with `present` or `missing` state. A present entry names its approved source
  (an exact worktree-relative path under the approved worktree binding, or an
  immutable server-held report/artifact ID and version) and a SHA-256 content
  digest. It may not introduce undeclared paths or artifact kinds.
- The sidecar resolves each reference within the approved scope, verifies the
  content digest against the bytes/immutable report it reads, canonicalizes the
  complete manifest with #741’s recursive canonical JSON rules, sorts artifact
  entries by stable artifact ID, and assigns the next positive result revision
  ID. Its digest is SHA-256 over the exact UTF-8 domain separator
  `orkworks.assignment-result.v1\n` (ending in one literal LF) followed by the
  canonical manifest bytes. The stored revision includes its predecessor ID
  and digest, source worker identity, assignment/configuration binding and
  observation time. The reporter cannot provide the revision number, digest,
  predecessor, or current-head marker.
- The accepted sidecar record is the authority for the current result head.
  The sidecar serializes updates for that exact attempt and rejects a stale
  expected task version or a predecessor that is no longer current. Exact
  retries return the same receipt; changed content with the same idempotency
  key conflicts. Any accepted later manifest advances the head; it does not
  rewrite prior revisions. Result revision IDs are positive integers from
  `1..=2^31-1`, assigned monotonically within the attempt and never reset or
  wrapped. Exhaustion closes further reports for that attempt; it does not
  reuse an ID or digest as identity.

An evaluator must bind its report to the sidecar-issued current result revision
ID and digest. At report acceptance, one serialized attempt-level boundary
covers checking the current result head, re-resolving and re-hashing the
manifest's referenced output bytes, accepting the evaluation, and updating its
current projection; no result-manifest update may interleave. If the
implementation cannot provide that boundary, it must revalidate the head and
bytes whenever it reads or projects current evaluation state, and return
Unassessed whenever it cannot. If an artifact is missing, inaccessible, or has changed content,
the sidecar cannot accept a current passing evaluation; it records an explicit
unassessed/stale state or rejects the current evaluation. A later worker report
creates a new result revision, making all evaluations for the predecessor
historical. The implementation plan must prove that exact path resolution,
immutable report lookup, current-head fencing and byte revalidation work for
every supported output type; unsupported types leave the assignment
unassessed. Direct filesystem changes outside the reporting/snapshot mechanism
are not represented as a new revision automatically, so a capability that
cannot revalidate the current declared outputs cannot claim Meets requirements.
This is content freshness, not proof of authorship or OS-level confinement.

A result-revision ID is distinct from plan revision, evaluation revision,
workspace snapshot, and Git commit. A Git revision alone is insufficient: it
may include unrelated changes or omit declared non-file output. The result
manifest is bounded to declared outputs and immutable report references, not a
snapshot of the entire repository. Every required declared output must be
present and verifiable to support Meets requirements. An explicitly absent
required output is evidence of an unsatisfied linked criterion; an output whose
presence cannot be determined remains unassessed.

Evaluations for a new attempt or a different result digest cannot overwrite or
silently update an earlier subject. A report for an authenticated but
superseded result may be retained as history; it cannot restore current status
or learning eligibility.

## Completeness calculation and unassessed state

For every approved criterion, required or optional, the evaluator reports
exactly one `satisfied`, `unsatisfied`, or `unassessed` outcome with evidence
references and a concise rationale. Each outcome applies only to its approved
criterion ID and the bound result digest. Missing, duplicate, or unknown
criterion IDs make the report malformed and it is rejected; uncertainty is
represented explicitly as `unassessed`.

Completeness is the count of satisfied required criteria divided by the number
of required criteria, expressed as a percentage, **only when every required
criterion has been assessed**. If one or more required criteria is unassessed,
show the satisfied/unsatisfied/unassessed counts and leave the percentage
unassessed; do not shrink the denominator or treat unknown criteria as failed.
An established unsatisfied required criterion is a known failure even when
other criteria remain unassessed. Optional-criterion results remain detail only.

A legacy or malformed assignment with no required criteria has no completeness
percentage and cannot be labeled Meets requirements. It remains Unassessed
unless separate, current, uncontested evidence establishes a failure. Never
calculate `0 / 0`.

Blocked, cancelled, interrupted, unsupported, or partially completed tasks
keep their coordination/lifecycle status. They are not automatically poor
quality or incomplete: assess any result that exists against its criteria;
leave unavailable evidence unassessed. A blocker can explain missing work but
cannot satisfy a criterion. Cost and elapsed-time evidence, when available,
is recorded separately as optional observations with a non-negative integer
value, explicit unit, source (`sidecar_clock`, `provider_report`,
`harness_report`, or `user_reported`), and UTC observation time. An estimated
value is explicitly labeled estimated; the contract infers no interval or
uncertainty. Missing cost/time data stays unknown. Neither value changes
quality or completeness
unless a future approved assignment makes it an explicit acceptance criterion.

## Quality rating and evidence requirements

The reviewer assigns one result-level ordinal rating against the approved
role-specific rubric:

| Rating | Meaning |
| --- | --- |
| `0` | Unusable result |
| `1` | Major rework required |
| `2` | Limited rework required |
| `3` | Meets the declared quality standard |
| `unassessed` | Evidence or an eligible reviewer is missing, stale, or disputed |

The evaluation records evidence for the rating against the rubric's named
quality dimensions. It does not average dimension scores into a rating or let a
high score on one dimension hide a documented failure on another. A rating of
0, 1, or 2 establishes Needs rework unless that rating is itself in credible
conflict; rating 3 satisfies the quality part of Meets requirements only when
the evidence is current and uncontested. The rubric may describe role-specific
expectations, but it cannot redefine the version-1 rating meanings above.

Quality and completeness are independent details. A complete result can fail
the quality standard; a high-quality partial result can still fail a known
required criterion or remain unassessed while required evidence is missing.
Every assertion cites bounded evidence tied to the exact result subject. A
terminal transcript, task status, report of completion, test command string,
or self-assigned rating is not sufficient evidence by itself.

## Reviewer identity and self-assessment separation

An evaluation must come from either:

1. an explicitly declared `review` or `verification` assignment in an
   Electron-approved plan, whose approved rubric names an eligible evaluator
   role; or
2. an explicit user review recorded through the user-authorized path.

For child review to qualify as independent, the server enforces all of these
OrkWorks assignment-level conditions. The contributor set is derived by the
sidecar from the approved plan's authenticated task reservations and their
effective write scopes, conservatively including every identity whose scope
overlaps a reported output. It is immutable for that result revision and is
not supplied by the worker. If the complete potential-contributor set cannot
be established, child review is ineligible and the result remains Unassessed
until explicit user review; this does not claim OS-level authorship proof.

- reviewer task, allocation/attempt, session and configuration identity are
  distinct from the result-producing assignment;
- the reviewer is not among the authenticated task identities declared as
  contributors in the result manifest;
- the reviewer configuration is eligible for the approved rubric's evaluator
  role and grants read access to the exact result subject, with no write access
  to its output scope; and
- the authenticated report capability is scoped to that reviewer assignment,
  result subject and current result revision.

A child cannot establish independence by sending an `independent` boolean or
claiming a role. The sidecar derives task/session/configuration identity and
checks the approved plan and permission snapshots. If a supported coding tool
cannot enforce the declared read-only result scope, that child profile is
ineligible and the outcome remains Unassessed until an explicit user review or
another verified profile is available. The parent that coordinates reports is
not an independent reviewer merely because it summarizes them.

This is workflow-assignment independence, not a claim of statistical,
organizational, or same-user process independence. The same coding tool or
model may be used for worker and reviewer; their identities and evidence remain
separate and visible. The product does not claim that distinct child sessions
are immune to shared model bias, shared user credentials, or same-user process
interference. Such native/OS isolation is outside #610. A reviewer that
contributed output under another declared assignment is not eligible to review
that output.

The worker's self-assessment is a separate report type. It may record the
worker's view of its result and claimed evidence, but it never counts as
reviewer evidence, changes the derived result, or substitutes for independent
review. If a reviewer assignment itself needs evaluation, it requires another
declared eligible reviewer or explicit user review. The evaluation's source
identity is authenticated separately from its content; the server derives
which assignment may report, and a payload's claimed source or conclusion
cannot establish authority.

Several eligible evaluations may exist for the same exact result subject. Keep
each reviewer identity and evidence distinct. A conflict exists when credible,
current evaluations disagree on the same criterion outcome, result-level
quality rating, or material fact used by those outcomes. Preserve both reports
and surface the disputed field and evidence; never average scores, select the
newest opinion as truth, or resolve conflict by reviewer seniority.

## Overall result

Derive one result from the current evaluation evidence:

| Result | Rule |
| --- | --- |
| **Needs rework** | At least one current, uncontested required criterion is unsatisfied, or a current, uncontested quality rating is below `3`. A known failure is not hidden by unrelated missing evidence. |
| **Unassessed** | No established failure is present, but any required criterion or quality rating is missing/unassessed, there is no eligible independent/user evaluation, the result binding is stale, or credible evidence conflicts. |
| **Meets requirements** | Every required criterion is satisfied and quality is `3`, all necessary evidence is current, and no relevant conflict or invalidation remains. |

If reports disagree about whether a failure occurred, that disputed failure is
not established; the overall result is Unassessed unless a separate,
uncontested failure exists. If the result changes after review, its old
assessment remains historical and cannot describe the new result as reviewed.

The result labels do not mean user acceptance or readiness to merge. A Needs
rework result can be reported without automatically launching a fix. Any fix,
retry, revised task, or changed rubric needs the normal approved plan revision
and admission gates.

## Result freshness, conflicts, corrections, and invalidation

An immutable `AssignmentEvaluation` record contains schema version, evaluation
stream ID and revision, exact assignment/configuration binding, result-revision
ID and manifest digest, rubric/criteria references, per-criterion outcomes,
quality rating and evidence references, reviewer/source identity, observation
time and authenticated report/idempotency metadata. A digest covers the
canonical immutable evaluation content; display projections and the derived
overall result are recomputed and are not reporter-controlled inputs.

Each independent reviewer or explicit user has a distinct evaluation stream
for one exact result subject. The sidecar assigns its immutable stream ID;
revision 1 starts that stream. A correction is the next revision in that same
stream and must name the expected prior revision/digest. Independent reviewers
create separate streams and do not contend on one global evaluation revision.
Invalidation is a separate, terminal disposition record bound to the exact
target stream ID, revision and digest. Each stream permits at most one
invalidation disposition (revision 1); an identical retry is idempotent and a
different second disposition is rejected. Invalidation freezes the stream:
no correction may append after it. Correction and invalidation serialize
against the expected current stream revision/digest. If correction commits
first, an invalidation targeting the old revision conflicts and requires a
new explicit user action against the new head; if invalidation commits first,
the correction is rejected. The sidecar also serializes these writes against
reads of the current projection. An evaluation-set digest binds the exact
current stream heads and dispositions used by one derived overall result.

A result-manifest head change makes evaluations of its predecessors historical
for current presentation and learning; it does not rewrite or invalidate their
original conclusions. A reviewer report for a superseded subject cannot restore
freshness. An evaluation accepted for a current subject is revalidated against
the exact manifest head and output bytes before it can affect the current
projection.

A correction creates a new immutable evaluation from the original eligible
reviewer identity while that identity holds a valid assignment-scoped report
capability and the stream has not been invalidated, links it to the revision it
corrects, records a bounded correction reason and fresh evidence, and leaves
the prior record intact. A resumed
reviewer must pass #742’s exact identity/configuration revalidation and receive
a fresh capability. After run completion, correction uses the user-authorized
provenance path defined by #742; it cannot impersonate the ended reviewer. An
identical retry with the same idempotency key returns the same receipt;
reusing that key with different content is rejected. A different reviewer who
disagrees submits a separate stream, producing a visible conflict rather than
correcting another person's report.

Invalidation is a provenance-bearing disposition, not deletion or rewriting of
evaluation content. It records the target evaluation ID/revision/digest, actor,
timestamp, reason and authorization source. An explicit invalidation requires
the user-authorized path; the worker or reviewer cannot silently invalidate a
conflicting report. Invalidation removes that stream from current result
derivation and future learning eligibility, but the record remains inspectable
for its retention period with the invalidation reason. If invalidation leaves
required evidence missing, the result becomes Unassessed unless another
uncontested failure remains. A changed result digest independently makes older
evaluations historical; it does not imply that their original review was
erroneous.

Reports for another workspace, run, assignment, reviewer grant, or result
subject are rejected. Exact duplicate reports are idempotent. Idempotency keys
are scoped to authenticated reporter identity, result subject and operation.
A correction compares the expected revision/digest within its own stream; an
invalidation compares the target disposition revision/digest; evaluation
creation compares the current result head. Concurrent new evaluations from
different eligible reviewers are both retained and included in the derived
conflict projection. Concurrent corrections/dispositions to the same stream
are serialized; a stale revision or changed payload conflicts rather than
silently choosing a winner. Each evaluator stream uses revision
compare-and-swap; different reviewer streams remain independently appendable
and both are retained for conflict derivation. Missing or ambiguous subject identity fails
closed. An authenticated late report for an exact old subject may be preserved
as historical only; it cannot enter the current result or learning summary.

## Numerical bounds and record lifecycle

Version 1 uses the upstream #741 assignment limits: at most 32 criteria and 16
rubric dimensions, with their bounded IDs/descriptions. Evaluation records add
these finite limits:

| Data | Limit / behavior |
| --- | --- |
| Independent evaluator streams per exact result subject | 16; the seventeenth distinct reviewer/user stream is rejected with a visible capacity blocker; no stream is silently evicted |
| Immutable snapshots per evaluator stream | 16 total (initial report plus at most 15 corrections); the seventeenth revision is rejected without discarding history |
| Maximum snapshots per exact result subject | 256, derived from the stream and per-stream limits above |
| Declared result artifacts | 32 per result manifest; IDs are unique and each bounded reference is at most 1 KiB |
| Evidence references per evaluation | 32; each stable source reference is at most 1 KiB and binds an immutable content digest or report ID/version/digest |
| Criterion outcomes | At most 32, unique by approved criterion ID; at most one outcome per criterion |
| Rubric dimension references | At most 16, unique by approved dimension ID |
| Finding/rationale/correction/invalidation reason | At most 2 KiB UTF-8 bytes per field; no credential values, full transcripts, or hidden reasoning |
| Serialized result manifest | At most 64 KiB, including metadata and artifact references |
| Serialized evaluation or disposition record | At most 128 KiB, including metadata; referenced payloads are not copied wholesale |
| Cost/time observations | At most 16 per evaluation; non-negative integer value with explicit unit, source, and UTC observation time; estimated values are labeled; unknown remains absent |
| Schema and revisions | `schemaVersion` is exactly 1; stream and evaluation revisions are `1..=16` per stream; result revision IDs are `1..=2^31-1` per attempt; the single optional disposition revision is `1`; IDs/revisions never wrap or reset, and exhaustion fails closed |

The implementation enforces limits before persistence and returns a visible
capacity/error state without dropping conflicting evaluations, correction
history, source evidence, or invalidation provenance. Reaching a per-subject
limit blocks new reviewer reports for that subject; it does not authorize
silently pruning an inconvenient review. Only an explicit user invalidation
with reason can make a report ineligible, and invalidated history still counts
until its retention period expires. Expiring another historical subject may
free aggregate workspace quota, but never frees a stream or snapshot slot on
the saturated current subject. That subject remains blocked until its approved
capacity changes or the user explicitly deletes the entire subject and starts
a newly approved assignment with fresh identities; no partial pruning is
permitted.

Aggregate workspace/repository history retention and forgetting belong to
#745. Its policy may expire complete historical result subjects and their
pinned evidence together after the declared retention period; it must not
expire a current result's only valid evaluation or evidence while that subject
is presented as Meets requirements. Workspace deletion atomically closes
report admission and rotates or discards the opaque workspace admission
generation in the same serialized boundary used to commit evaluation writes,
then removes all evaluation, disposition, and pinned evidence content. Every
report commit, including a handler admitted before deletion began, must compare
its captured generation with the current generation at persistence; stale
generations cannot write or recreate content. Content committed before the
deletion boundary is included in the purge. Reopening uses a fresh unpredictable
generation and new run/plan/attempt/result identities. Old capabilities are
rejected before payload processing; no evaluation content or tombstone survives
workspace deletion. If the implementation cannot prove this fence, it fails
closed and cannot reopen that identity for evaluation.
After evidence is expired or deleted, any retained projection must say evidence
is unavailable and cannot continue to claim a current reviewed result. Report
admission for expired/deleted assignment identities remains closed under the
#742 attempt/generation and retention fences; deletion cannot make an old
report current again. Ordinary non-orchestrated sessions remain unchanged and
do not acquire fabricated assignment evaluations.

## Coordination, presentation, and learning boundaries

An evaluation is not a Taskmaster `CoordinationResult`. Parent-reported
completion may advance only a declared dependency under #610; evaluator
reports and scores cannot do so. A complete/quality-passing child can still
require parent coordination or user review, and parent success does not prove
independent quality. User acceptance, merge approval, and manual integration
remain explicit separate actions.

This contract is separate from `CompletionPacket` in
[`specs/taskmaster.md`](../../../specs/taskmaster.md#workflow-improvement-recommendations).
That packet describes evidence/readiness for one existing `ImproveWorkflow`
recommendation and its action approval. Its readiness projection is not an
assignment quality score; this specification does not extend or repurpose it.
The existing implementation in `taskmaster/completion.rs` is useful precedent
for immutable revision/fingerprint binding, independent reviewer identity,
source-authenticated reports, idempotency, and explicit missing/conflicting
evidence. Its one-workspace-snapshot subject and recommendation readiness rules
do not satisfy the assignment contract by themselves.

The future hierarchy view in #746 may show only one overall result and ordinary
status by default; scores, criteria, reviewer findings, evidence provenance,
conflicts, and invalidation belong in details. Presentation must distinguish
Unassessed from failure and never imply that a missing usage or report means
zero quality. The future learning contract in #745 may associate eligible
evaluations with exact configuration cohorts. It excludes stale, invalidated,
conflicting, blocked, interrupted, or sparse evidence as specified there and
must not claim that an equipped skill caused an outcome. Evaluation cannot
change a current configuration; changes remain proposals for a later approved
assignment.

## Verification cases

A future implementation must include focused contract/store/API fixtures for
at least these cases:

1. A fully assessed assignment with all required criteria satisfied and
   quality `3` derives Meets requirements; optional criteria do not change the
   percentage or result.
2. A required criterion is unsatisfied while another is unassessed; derive
   Needs rework without pretending completeness is fully measured.
3. All required criteria are satisfied but quality is `2`; derive Needs
   rework. A quality `3` with one required criterion missing is Unassessed.
4. An empty/optional-only legacy criteria set has no percentage and cannot
   derive Meets requirements; a separate known failure can still derive Needs
   rework.
5. A blocked/cancelled/interrupted task with partial output is not assigned a
   score from lifecycle status; assess available evidence and leave missing
   items unassessed.
6. Worker self-assessment, parent synthesis, an undeclared reviewer, or the
   same worker session attempting self-review cannot produce an evaluation.
   A declared independent reviewer and explicit user review can.
7. Two credible evaluations disagree on a criterion or quality; preserve both,
   show the conflict, and derive Unassessed unless a separate uncontested
   failure establishes Needs rework. Never average.
8. An evaluation binds the exact assignment/configuration, reviewer
   configuration, criteria/rubric digests, and result manifest. Any changed
   result digest makes the previous record historical and requires fresh
   evaluation.
9. A correction creates a new snapshot linked to the old one; exact replay is
    idempotent, payload change under the same key conflicts, and another
    reviewer cannot edit it. Correction racing invalidation resolves by the
    stream's expected-head compare-and-swap; invalidation freezes the stream.
    Explicit user invalidation preserves the record and provenance while
    excluding it from current derivation/learning.
10. Late, stale-revision, duplicate, cross-workspace, cross-assignment,
    unauthenticated, malformed, oversized, or ambiguous reports fail closed;
    concurrent conflicting writes cannot overwrite each other.
11. Per-subject and per-record limits reject excess data without evicting
    conflicting evidence or the only current valid evaluation; expiring another
    subject cannot free current-subject capacity. Workspace deletion serializes
    with report commits, rotates/discards the admission generation, and purges
    evaluation content; an in-flight handler with a stale generation cannot
    recreate it, and no evaluation tombstone is retained.
12. Time/cost measurements remain optional sourced details and do not alter
    the quality/completeness result unless separately approved as task criteria.
13. Evaluation cannot launch/retry a child, widen permissions, advance an
    undeclared dependency, mark user acceptance, approve a merge, or mutate an
    active assignment/configuration.

## Execution plan and implementation gate

No runtime implementation plan is approved by this document. A scoped plan may
be written only after the #741 configuration contract and this evaluation
contract receive written review, #740's capability disposition is reconciled,
and #610 plus the authoritative specs/ADRs are aligned to the accepted
ordinary-child proposal. The plan must select the result-manifest and
assignment-attempt bindings, owning store/reporting path, correction and
invalidation transaction boundary, aggregate retention policy with #745, and
renderer projection with #746. It must name the exact Rust/desktop files and
focused fixtures after verifying the current module seams; this document's
source references are investigation pointers, not implementation commitments.

Until those gates pass, the supported outcome may remain no-go and no runtime
eligibility or behavior may be claimed. User review of this document approves
only the proposed written contract. It does not approve runtime implementation,
role support, assignment launches, automated evaluation, or changes to ordinary
Peon/session behavior.
