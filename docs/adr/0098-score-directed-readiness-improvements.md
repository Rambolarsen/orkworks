---
type: Architecture Decision
title: Score-directed readiness improvements with feasible evidence selection
description: Proposed connection from assessed capability gaps to independently grounded next changes, with bounded evidence admission.
tags: [orkworks, architecture, taskmaster, readiness]
status: stable
workflow_status: approved
---

# Score-directed readiness improvements

- Status: accepted; owner approved this design, implementation and the maximum
  twenty-call synthetic qualification budget on 2026-10-30 ("keep going").
- Tracking: [#1175](https://github.com/Rambolarsen/orkworks/issues/1175)
- Date: 2026-10-30
- Implementation and checks: [score-directed readiness plan](../superpowers/plans/2026-10-30-score-directed-readiness.md)

## Context and evidence

The owner wants concrete changes that improve agent readiness, rather than a
score followed by unrelated workflow analysis. The accepted [combined action](0096-readiness-workflow-suggestions.md)
saves readiness first, but its [implementation plan](../superpowers/plans/2026-10-26-readiness-suggestions.md)
explicitly excludes scores and generated transitions from suggestion prompts.
`readiness::run_suggestion` transfers the captured evaluation settings to the
independent assessment without the scored areas. This matches that contract;
the proposal below changes the product decision rather than describing it as an
implementation violation. Observation-noise investigation #1077 stays separate.

The installed app exposes **Assess and suggest**. The retained 46/100 snapshot
completed on October 26 using collector 2 and Luna medium, before the combined
flow shipped. It is historical evidence, not a failed current combined run.

The owner approved one current combined run on October 30. Its parent started at
05:33:28.922Z and ended with `scoring_failed`. The readiness child reported
`evidence_evidence_limit`; the UI stated that selected evidence exceeded the
assessment allowance and no score was saved. The suggestion stage never started.
The exact model-selected ID set was not retained, so this does not establish
which individual files or instruction closure exhausted the allowance. No
additional live run, repair, model change, or workspace file edit was performed.

## Decision

Keep **Assess and suggest** and its existing recommendation lifecycle. Connect
its second stage to the exact readiness snapshot created by that operation.
Use scored areas as investigation pointers, then independently substantiate an
improvement with current repository facts and verified rubric guidance. Keep the
standalone **Assess readiness**, **Assess workflow**, and **Analyze now** purposes
distinct. Preserve the overall score formula and all historical scores.

### Feasible evidence before scoring

Readiness discovery version 2 returns exactly `{version: 2, preferredFileIds}`,
with at most eight distinct existing catalog IDs in priority order. These are
preferences for bounded admission, not a request to read an indivisible set.
Workflow discovery and its existing requested-set semantics do not change.

Validate every preferred catalog ID and its safely inspectable path/closure metadata
before budget admission. Any encountered validation, access, I/O or currentness
failure invalidates the entire attempt, including otherwise affordable evidence
and any earlier budget omissions. Do not save a new score or dispatch suggestions
from that attempt; prior history remains. An unaffordable preference does not
require reading its source content, which remains unknown.

The repository evidence owner admits preferences in order. Each item includes
all governing instructions not already supplied. Admit the entire file/closure
only when it fits the remaining byte and file allowances; otherwise omit that
preference with a fixed `evidence_limit` reason and consider the next one.
Never partially supply a governing-instruction closure. Do not read extra file
content to estimate costs: use safely checked bounded-prefix sizes and the
existing instruction/path owner. A changed, unreadable, disallowed, or unsafe
selected path still interrupts/fails the run under existing rules; budget
omission cannot hide an I/O or access failure. Revalidate admitted files and
instructions before scoring and publication as today.

Keep seed limits at 16 files/24 KiB, aggregate limits at 32 files/64 KiB,
per-file reads at 8 KiB, and excerpts at 2,000 characters. No retries, access
expansion, larger budgets, or fallback scoring of an interrupted requested set.
An empty admitted preference set proceeds to the final scoring call using the
safely collected seed evidence and recorded coverage limits. This is a completed
seed-only assessment, not an interrupted run. Final decoding decides whether its
evidence supports a full score or a partial result; admission never fills unknown
areas merely to produce a total. Areas without enough evidence remain unassessed;
omission never establishes a
missing capability or a zero rating.

Readiness collector 4 records both the original preference list and admitted
`requestedFileIds`, plus up to eight `{id, reason: "evidence_limit"}` omissions in
its existing bounded manifest. Its discovery digest covers the original version-2
response. Restoration reconstructs the recorded admitted set and verifies its
recorded closure; it never silently reallocates a saved capture. Preserve readers
for collectors 2 and 3. Workflow collector 3 remains unchanged. New readiness
prompt identity is `taskmaster-readiness-v3`; final scoring response version and
history envelope format remain unchanged. The UI identifies budget omissions as
coverage limits in the saved result, including when an overall score is available.

### Scored areas as pointers, repository facts as evidence

After saving the score, capture an in-memory `readinessContext` from that exact
snapshot: snapshot ID, rubric digest, and the fourteen canonically ordered areas.
Each area supplies concept ID, maturity or null, confidence or null, its bounded
saved `nextTransition`, and the verified rubric page ID/hash. Supply the signed
next-level criterion when maturity is below four; four has no higher level and
null is never treated as zero. Generated transitions remain untrusted hypotheses.
This added input is capped at 48 KiB inside the existing 128-KiB prompt ceiling.
Do not include full histories, uncited source, private guidance, or terminal replay.

Discovery sees the area pointers and their signed criterion index. It seeks one
bounded improvement and names its capability through existing `knowledgePageIds`
citations, choosing at most three rubric concept pages. Those cited pages are
then included in the final stage's existing eight-page guidance allowance,
alongside the required assessment guidance. Existing source collection bounds
and at most eight follow-up catalog IDs remain in force for workflow assessment.
Combined discovery validates criterion-index references against the captured
verified rubric; final citation validation uses the actual supplied concept pages. The shipped rubric maps multiple capabilities to one shared playbook,
so that playbook alone cannot identify a capability. Concept-page references
require an existing signed page of type `concept` at
`concepts/{canonical-concept-id}.md`; missing concept pages remain unavailable,
never synthesized or assigned arbitrary capability labels.
Neither page identity nor a low score proves a repository gap.

The final proposal must cite current repository facts and at least one selected
concept page. Its existing summary names the capability, the relevant next
maturity condition, the bounded repository change, expected practical benefit,
validation scope and rollback. A low or unknown rating by itself, an omitted
file, optional breadth, or missing execution history is insufficient. Existing
counterevidence must refute a candidate that is already satisfied. If nothing is
supported, return the existing explicit no-proposal result explaining what was
checked and why; do not claim the entire repository has no improvement opportunity.
Never promise a point increase. A later assessment measures change and model
variability under the existing comparison contract.

### Identity, persistence and existing authority

Use the existing installation-wide lease and four-call ceiling: at most two
scoring calls and two suggestion calls. No extra model call for area selection.
Keep the configured provider/model/effort, active-card, deduplication, dismissal,
publication recovery, and explicit **Fix with AI** gates. No automatic edits or
session launches are added.

Combined workflow assessments use assessment-record format 3, extending format 2
with `readinessReference: {snapshotId, rubricDigest}`. Require UUID snapshot
identity and a 64-character lowercase hexadecimal digest. Keep the 64-KiB record
cap. Standalone workflow assessment continues writing format 2; readers preserve
all currently supported older records. Unknown formats remain unavailable.

The recommendation's existing assessment provenance gains an optional
`readinessReference: {snapshotId, rubricDigest, conceptIds}`. The concept list
contains one to three unique canonical IDs derived from final rubric citations,
not invented model identifiers. Existing recommendations without it remain valid.
Render linked capability names and the originating saved assessment beside the
existing card; do not display an estimated numeric gain.

The exact reference, selected rubric guidance and purpose enter the assessment
fingerprint and currentness checks. Resolve the same permitted score under the
workspace/access locks before each dispatch and publication; never substitute a
newer or unrelated score. Workspace/settings/provider/trust/guidance changes or
redacted/missing in-flight inputs interrupt continuation. Existing privacy
reconciliation redacts the affected score-derived context/provenance before
exposure. Recovery never invokes inference. After historical score eviction,
retain only the source-free reference and label the score link unavailable;
repository-based recommendation actions keep their existing independent evidence
and authority gates and acquire no authority from a missing historical score.

## Compatibility, qualification and rollback

This supersedes only the exclusion of scored-area pointers in the accepted
combined plan and readiness's indivisible discovery-request semantics. All other
[ADR 0095](0095-repository-agent-readiness-history.md) and ADR 0096 boundaries
remain. The owner approved this scoped supersession on 2026-10-30.

Changed prompt/collector versions invalidate reuse and comparison compatibility
under existing rules without rewriting old snapshots. Old score/assessment formats
remain viewable under current access policy. Older binaries cannot interpret new
collector/assessment formats: disable the new combined admission and forward-fix
rather than hand-editing or downgrading retained new records.

Qualification is required before claiming useful score-directed inference.
Retain exact synthetic inputs, model/effort and raw outputs for three positive
combined trials, one already-satisfied case and one evidence-unknown case, with
at most twenty provider calls total. At least two positive trials must produce
valid, independently adjudicated changes tied to the intended rubric transition;
negative cases must produce no invented recommendation. Rejected, interrupted,
over-budget and no-proposal positive trials receive no quality credit. This
budget uses synthetic repositories only, does not change the configured profile,
and requires owner authorization as part of implementation approval. Do not
repeat failed trials or silently switch models. Failed qualification is reported,
not hidden behind passing transport tests or presented as a useful release.
