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

Evaluation uses the exact user-approved `AssignmentConfiguration`:
repository/workspace, run, plan and revision, task, allocation/attempt, worker
session, configuration ID/digest, criteria, and rubric. IDs and digests define
identity; display labels do not. Each evaluation also binds this assignment, the
result revision/digest, the reviewer assignment/session/configuration, approved
snapshots, and immutable evidence references. A changed assignment or rubric
requires a new approved revision.

The expected output contract is specific to each assignment; there is no global
catalog of artifact IDs or kinds. This proposal requires the approved contract
to name its expected artifacts and caps the list at 32. The current #741 draft
has an expected-output-contract field but does not yet define this structured
ID/kind schema, so reconcile the schema and cap with #741 before implementation.
The worker reports a manifest through #742's authenticated task-scoped
authority. The sidecar derives the assignment identities from that authority;
the payload cannot choose them. The manifest has exactly one `present` or
`missing` entry for each declared artifact. A present entry names an approved
worktree-relative path or immutable server-held report/artifact ID and version,
plus a SHA-256 content digest. Undeclared outputs are rejected.

The sidecar resolves each reference within the approved scope, verifies its
bytes and digest, sorts entries by artifact ID, canonicalizes the manifest
using #741's rules, and assigns a monotonically increasing result revision.
Updates must name the current predecessor and reject stale task versions; they
never rewrite prior revisions. The digest is SHA-256 of
`orkworks.assignment-result.v1\n` followed by the canonical manifest bytes;
each revision records its predecessor and authenticated worker source. Exact
retries are idempotent; changed content with the same key conflicts. Revisions
are `1..=2^31-1` per attempt; they never wrap or reset. Exhaustion closes
reporting for that attempt.

An evaluation binds the sidecar-issued current result revision and digest.
Acceptance must serialize the current-head check, output re-hash, evaluation
write, and current projection update against result-manifest changes. If that
boundary cannot be provided, every current read/projection must revalidate the
head and bytes and return Unassessed when it cannot. Missing, inaccessible,
changed, or unsupported outputs cannot support a passing result. A later
manifest creates a new revision; earlier evaluations remain history and cannot
restore current status or learning eligibility.

Every required declared output must be present and verifiable to pass; a known
missing output makes its linked criterion unsatisfied, while unknown presence is
unassessed. Direct filesystem changes are not automatically revisions. The
implementation must revalidate declared outputs before claiming a current
result; this proves content freshness, not authorship or OS-level confinement. A
Git commit alone is not the result identity because it may contain unrelated
changes or omit non-file outputs.

## Criteria, completeness, and quality

Each criterion has a stable ID, description, and `required` or `optional`
status. New assignments require at least one required criterion. Criteria and
classification are immutable after launch. Optional results are details only and
do not affect completeness or the overall result.

An evaluation provides exactly one `satisfied`, `unsatisfied`, or `unassessed`
outcome for every approved criterion, with evidence and rationale. Evidence
references bind an immutable content digest or report ID/version/digest.
Missing, duplicate, or unknown criterion IDs make the report malformed and it is
rejected.

Completeness is the percentage of required criteria satisfied, but only when
every required criterion has been assessed. Otherwise show the
satisfied/unsatisfied/unassessed counts and leave the percentage Unassessed.
Never shrink the denominator or calculate `0 / 0`. A known, uncontested
unsatisfied criterion remains a failure even if other criteria are unassessed.
Legacy assignments without required criteria cannot pass; absent other known
failures, they are Unassessed.

The approved, versioned, role-specific rubric has a stable ID, version, content
digest, evaluator role, and quality dimensions. The reviewer assigns one
result-level rating; dimension scores are evidence and are not averaged:

| Rating | Meaning |
| --- | --- |
| `0` | Unusable result |
| `1` | Major rework required |
| `2` | Limited rework required |
| `3` | Meets the declared quality standard |
| `unassessed` | Evidence or an eligible reviewer is missing, stale, or disputed |

The meanings above are fixed for version 1. Ratings 0–2 establish Needs rework
when current and uncontested. Rating 3 meets the quality part of the result only
with current, uncontested evidence. A transcript, task status, completion claim,
test command string, or self-rating is not sufficient evidence by itself.
Quality does not rank agents, grant XP, or prove a skill caused an outcome.

Derive one overall result from current evidence:

| Result | Rule |
| --- | --- |
| **Needs rework** | A current, uncontested required criterion is unsatisfied or quality is below `3`. |
| **Unassessed** | No failure is established, but a required criterion or quality is unassessed, the result is stale, no eligible reviewer exists, or credible evidence conflicts. |
| **Meets requirements** | Every required criterion is satisfied, quality is `3`, evidence is current, and no relevant conflict or invalidation remains. |

A disputed failure is not established; report Unassessed unless another
uncontested failure exists. Blocked, cancelled, interrupted, unsupported, or
partial work keeps its lifecycle status: assess available evidence, and leave
the rest unassessed. A blocker explains missing work but does not satisfy a
criterion.

Cost and elapsed time are optional observations. Each has a non-negative integer
value, unit, source (`sidecar_clock`, `provider_report`, `harness_report`, or
`user_reported`), and UTC observation time; estimates are labeled. Missing
values remain unknown. These observations do not change the result unless
separately approved as criteria.

## Reviewer eligibility and disagreement

A review must come from either a declared `review` or `verification` assignment
in an approved plan, with a rubric-eligible role, or an explicit user review
through the user-authorized path.

A child reviewer is eligible only when the sidecar verifies that:

- its task, allocation, session, and configuration differ from the worker's;
- it is not a potential contributor to the reported outputs, based on
  authenticated plan reservations and effective write scopes that overlap those
  outputs;
- its approved configuration grants read access to the exact result and no write
  access to its output scope; and
- its report capability is scoped to that reviewer, result subject, and current
  result revision.

The worker cannot declare the contributor set or claim independence. If the full
potential-contributor set or read-only scope cannot be established, child review
is ineligible and the result stays Unassessed until explicit user review. This
is assignment-level separation, not statistical, organizational, or OS
isolation; the same coding tool or model may serve as worker and reviewer. A
coordinating parent is not an independent reviewer just by summarizing reports.

Worker self-assessment is stored separately and never affects the derived
result. Source identity is authenticated; payload claims cannot establish
authority. A reviewer who contributed to the output under another declared
assignment is ineligible for that output.

Keep eligible evaluations separate. If current, credible evaluations disagree on
a criterion, quality rating, or material fact, preserve and show both with the
disputed field and evidence. Do not average, prefer the newest opinion, or
resolve by reviewer seniority. A separate uncontested failure can still
establish Needs rework.

## Revisions, correction, and invalidation

Each reviewer or explicit user has a separate stream for one exact result
subject. The sidecar assigns the stream ID; revision 1 starts it. A correction
appends an immutable revision to that stream using expected-revision/digest
compare-and-swap. Different reviewers create independent streams. The derived
result binds the exact stream heads and dispositions it used.

A correction requires the original eligible reviewer identity and a valid scoped
capability. Resumed reviewers must pass #742 identity/configuration checks and
receive a fresh capability. After run completion, correction uses #742's
user-authorized provenance; it cannot impersonate an ended reviewer. A different
reviewer submits a separate stream, not a correction.

Exact retries with the same idempotency key return the same receipt; changed
content under that key conflicts. Keys are scoped to authenticated reporter,
subject, and operation. Stale, unauthenticated, ambiguous, or
cross-workspace/assignment reports fail closed. A report for an exact superseded
subject may be retained as history only.

Only the user-authorized path can invalidate a report. Invalidation records its
target stream revision/digest, actor, time, reason, and authority; it does not
erase the evaluation. There is at most one terminal invalidation per stream
(revision `1`); identical retries are idempotent and different second
dispositions are rejected. Invalidation freezes the stream and excludes it from
current derivation and learning. Correction and invalidation compare-and-swap
against the same expected stream head: if correction wins, invalidating the old
head conflicts and requires a new user action; if invalidation wins, correction
is rejected. If invalidation leaves required evidence missing, the result
becomes Unassessed unless another uncontested failure remains.

A result-head change makes predecessor evaluations historical. Current
evaluation projections must be revalidated against the result head and output
bytes; stale evaluations never become current again.

## Bounds, retention, and deletion

Version 1 uses #741's limits of 32 criteria and 16 rubric dimensions, plus:

| Data | Limit |
| --- | --- |
| Reviewer/user streams per result subject | 16 |
| Snapshots per stream | 16, including the initial evaluation |
| Snapshots per subject | 256 |
| Result artifacts / evidence references | 32 each |
| Serialized manifest / evaluation or disposition | 64 KiB / 128 KiB, including metadata; referenced payloads are not copied |
| Artifact/evidence references | 1 KiB each |
| Rationale, finding, correction, or invalidation reason | 2 KiB UTF-8 each |
| Cost/time observations | 16 per evaluation |
| Schema and revisions | Schema `1`; stream revisions `1..=16`; result revisions `1..=2^31-1` per attempt; one disposition revision `1`; never wrap or reset |

Enforce bounds before persistence. Capacity errors must be visible; never
silently evict conflict evidence, correction history, or invalidation
provenance. Reaching a subject limit blocks new reports for that subject.
Expiring another subject may free aggregate workspace quota, but not slots on
the saturated subject. Recover capacity only through an approved capacity change
or user-directed deletion of the whole subject followed by a newly approved
assignment with fresh identities.

Aggregate retention and forgetting belong to #745. It may expire complete
historical subjects and their pinned evidence together, but not the only valid
evidence for a current result presented as Meets requirements. Workspace
deletion closes report admission and rotates or discards its opaque generation
in the same serialized boundary as report commits, then purges evaluation,
disposition, and pinned evidence content. Every commit—including a handler
admitted earlier—checks its captured generation at persistence; stale
generations cannot write or recreate content. Reopening uses a fresh
unpredictable generation and new run/plan/attempt/result IDs. No evaluation
tombstone survives deletion. If evidence is gone, a retained projection cannot
claim a current reviewed result. Ordinary non-orchestrated sessions acquire no
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

A future implementation must verify at least these cases:

1. All required criteria satisfied and quality `3` derives Meets requirements;
   optional criteria do not affect it.
2. Any uncontested known failure derives Needs rework despite unrelated
   unassessed evidence; missing required evidence without a known failure
   derives Unassessed.
3. Empty/optional-only legacy criteria cannot pass; lifecycle status alone never
   sets quality or completeness.
4. Self-review, parent synthesis, undeclared reviewers, contributors, stale
   output, or unverified read-only scope cannot qualify as child review.
5. Conflicting reviews are preserved and never averaged; corrections are
   immutable/idempotent, and invalidation freezes the stream.
6. Stale, malformed, duplicate-with-changed-payload, oversized, unauthenticated,
   cross-subject, and concurrent reports fail closed without overwriting
   history.
7. Bounds reject excess records without eviction; retention removes whole
   eligible historical subjects with evidence; deletion fences in-flight writes
   and removes evaluation content.
8. Evaluation cannot launch/retry work, widen permissions, change configuration,
   advance dependencies, accept work for the user, or approve a merge.

## Implementation gate

No runtime plan is approved here. Write one only after written review of this
contract and #741, reconciliation of #740's capability disposition, and
alignment with #610 and authoritative specs/ADRs. The plan must choose
result/attempt identity bindings, store and reporting path,
correction/invalidation transaction boundary, #745 retention policy, and #746
projection, based on verified module seams and focused fixtures. This document's
code references are investigation pointers, not implementation commitments.

User approval of this spec approves only the written contract—not runtime
implementation, role support, assignment launches, automated evaluation, or
changes to ordinary Peon/session behavior.
