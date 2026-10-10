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

Evaluation binds the workspace, run, task/attempt, approved configuration,
exact result revision, criteria and rubric versions, reviewer identity, and
immutable evidence references. These values come from approved state, not
display labels or reporter claims. A changed assignment or rubric requires a
new approved revision.

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

The worker reports one `present` or `missing` entry per declaration through an
authenticated attempt-scoped capability that follows #742's authority model.
It binds the attempt, approved configuration, sidecar generation, and active
worker launch generation; the payload cannot choose those identities. A
present entry names an approved worktree-relative path or immutable
server-held artifact ID/version, its size, and content digest. Undeclared or
over-cap outputs are rejected. Exact retries with the same key return the
stored receipt before checking whether the predecessor is still current;
changed content under that key conflicts. The key is scoped to reporter,
attempt, and operation.

The sidecar must resolve and open file paths within the approved output scope,
reject symlink or junction targets outside it, and preserve the opened-object
binding for hashing and later revalidation. No out-of-scope file may be read or
pinned as an artifact.

The sidecar assigns immutable result revisions and rejects stale predecessors.
Evaluations bind the current revision and digest. Before any consumer treats a
result as current, the implementation revalidates its referenced output bytes
and revision; if it cannot establish freshness, the result is Unassessed. A known
missing output establishes Needs rework directly; inaccessible, changed,
unknown, or unsupported output is Unassessed unless another uncontested
failure exists. A later result revision makes earlier evaluations historical.
The implementation plan chooses the transaction or revalidation boundary;
either way, stale writes cannot restore a current result. This validates
content freshness, not authorship or OS-level confinement.

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

The approved, versioned, role-specific rubric has an ID, version, evaluator
role, and required quality dimensions with stable IDs. The reviewer assigns
one `meets`, `below standard`, or `unassessed` outcome and evidence for every
dimension, plus one result-level rating. Missing or unassessed dimension
evidence makes quality Unassessed; an evidenced, current rating below `3` or a
below-standard dimension establishes a quality failure even if another
dimension is unassessed. Dimension scores are not averaged.
Comparisons across assignments require the same rubric ID and version unless
an explicit, versioned normalization rule is approved:

| Rating | Meaning |
| --- | --- |
| `0` | Unusable result |
| `1` | Major rework required |
| `2` | Limited rework required |
| `3` | Meets the declared quality standard |
| `unassessed` | Evidence or an eligible reviewer is missing, stale, or disputed |

The meanings above are fixed for version 1. Ratings 0–2 establish Needs rework
when current and uncontested. Rating 3 meets the quality part of the result only
with current, uncontested evidence. Before using an outcome, the sidecar
validates evidence references for existence, scope, version, and digest;
unavailable or unverified evidence makes that outcome Unassessed. A transcript,
task status, completion claim, test command string, or self-rating is not
sufficient evidence by itself. Quality does not rank agents, grant XP, or prove
a skill caused an outcome.

Derive one overall result from current evidence:

| Result | Rule |
| --- | --- |
| **Needs rework** | A current, uncontested required criterion is unsatisfied, quality is below `3`, a current, uncontested required-rework finding is present, or a declared output is known missing. |
| **Unassessed** | No failure is established, but a required criterion or quality is unassessed, the result is stale, no eligible reviewer exists, or credible evidence conflicts. |
| **Meets requirements** | Every required criterion is satisfied, quality is `3`, evidence is current, and no relevant conflict or invalidation remains. |

A disputed failure is not established; report Unassessed unless another
uncontested failure exists. Blocked, cancelled, interrupted, unsupported, or
partial work keeps its lifecycle status: assess available evidence, and leave
the rest unassessed. A blocker explains missing work but does not satisfy a
criterion.

An evaluation record stores criterion and dimension outcomes, quality, findings
and required rework, evidence, reviewer/source identity, observation time, and
retry metadata; the server derives the overall result. Each finding has a
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

A reviewer cannot evaluate their own work. A reviewer may be evaluated by a
different declared reviewer or the user. Do not assign a further child to
evaluate that review; the user is the terminal evaluator. Without user
disposition, leave that evaluation Unassessed. This preserves the parent
design's reviewer-of-review path without unbounded recursion.

Keep eligible evaluations separate. If current, credible evaluations disagree
on a criterion, rating, finding, or material fact, preserve both and report
Unassessed unless another uncontested failure establishes Needs rework. Never
average conflicting reviews or prefer one by time or seniority.

## Revisions, correction, and invalidation

Evaluations and corrections are append-only. A correction is a new revision by
the same eligible reviewer while that review capability is active; a different
reviewer creates a separate evaluation. An authenticated idempotency key is
scoped to reporter, subject, and operation. Resolve an exact prior receipt
before checking the expected revision; changed content under the same key
conflicts. Stale or cross-subject writes fail closed. Invalidation names the
current evaluation revision and digest; it is serialized with corrections,
uses the same idempotency rules, and freezes that evaluation stream once
accepted. A stale invalidation conflicts and cannot exclude a newer correction.

When #742 ends a run and revokes child authority, that child can no longer
correct its review. The user may append a separately attributed correction or
invalidate it through the Electron-authorized path; the user never impersonates
the child. Invalidation preserves the report and provenance and excludes it
from current results and learning. A later result revision makes earlier
evaluations historical; they never become current again.

## Bounds, retention, and deletion

Use #741's limits of 32 criteria and 16 rubric dimensions. Also cap each
subject at 32 artifacts and 16 evaluation revisions; allow at most 32 findings
and 80 evidence references per evaluation revision, enough for the maximum
criteria, dimensions, and findings. Cap the manifest at 64 KiB, an
evaluation/disposition at 512 KiB, each reference at 1 KiB, and
rationale/finding/correction text at 2 KiB. Enforce finite aggregate
run/workspace admission quotas; reject exhausted capacity visibly without
evicting history. Output and aggregate quota values must be reconciled with #741
and #745 before implementation. Per-artifact bytes and aggregate rehash work
have finite approved caps bounded by server hard limits; exceeding either
makes the output unsupported.

Reject over-limit reports visibly and never silently drop conflict evidence,
corrections, or provenance. Retention belongs to #745, which may remove a
complete eligible historical subject and its pinned evidence but not evidence
for a current result presented as Meets requirements. Workspace deletion stops
new reports and in-flight writes, then purges evaluation, disposition, and
pinned evidence. Old writes cannot recreate deleted content; no evaluation
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
   including a known missing output, derives Needs rework; missing, stale, or
   conflicting evidence without a known failure derives Unassessed. An
   evidenced rating below `3`, a below-standard dimension, or a required-rework
   finding is a failure even when other dimensions are unassessed. Optional
   criteria, lifecycle state, or empty legacy criteria cannot create a pass.
2. Exact retries return their saved receipt before stale-predecessor rejection;
   changed payloads, malformed, unauthenticated, oversized, or cross-subject
   reports fail closed. Missing dimension coverage or invalid evidence cannot
   support a pass. File links cannot escape the approved scope. Results cannot
   remain current for any consumer after referenced bytes change.
3. Scores compare across assignments only under the same rubric ID/version or
   an approved normalization rule. Findings retain location, severity,
   evidence, and required-rework status without prohibited sensitive content.
4. Self-review, parent synthesis, unverified profiles, contributors, stale
   output, or unverified read-only scope cannot qualify as child review.
   Reviewer-of-review depth is bounded and ends with user authority.
5. Corrections preserve reporter provenance; ended child capabilities cannot
   correct or impersonate a reviewer. Concurrent corrections and invalidations
   cannot restore an invalidated evaluation or exclude a newer revision.
6. Per-attempt result-revision and record caps reject excess work visibly, as
   do aggregate run/workspace quotas. Retention removes only eligible complete
   historical subjects; deletion fences old writes and purges evaluation
   content.
7. Evaluation cannot launch/retry work, widen permissions, change
   configuration, advance dependencies, accept work for the user, or approve a
   merge.

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
