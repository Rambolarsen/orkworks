# improve_workflow recommendation lifecycle statuses

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-08-31

## Context

ADR 0042's "Embedded recommendation evidence" decision records the passive
`improve_workflow` recommendation variant as reaching only
`proposed`/`dismissed`/`superseded` in this version. `RecommendationStatus::Superseded`
is defined in the shared canonical status enum, but a repo-wide search shows it
is never constructed anywhere in the crate: `evaluate_workflow_improvements`
only produces `Proposed`, and `RecommendationStore::dismiss` only transitions
`Proposed` → `Dismissed`. When a dismissed recommendation's evidence later
qualifies for a resurfaced successor, the evaluator creates a new `Proposed`
record carrying `supersedes_recommendation_id` pointing at the dismissed
predecessor, and the predecessor's own status stays `Dismissed` — consistent
with the workflow-observation design's commitment that "the dismissed record
remains immutable history" (see
`docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md`).

The reachable-status claim in ADR 0042 therefore conflicts with both the
implementation and the design doc. Issue #343 surfaced the contradiction and
resolved it in favor of correcting the recorded contract rather than adding a
transition that would rewrite dismissal history.

## Decision

For the passive `improve_workflow` variant, only `proposed` and `dismissed`
are reachable; no evaluator transition ever writes `superseded`. The
`superseded` status remains part of the canonical Taskmaster recommendation
status vocabulary — valid for shared deserialization and available to future
recommendation variants that replace workspace state — but is documented as
never produced by this evaluator.

The lineage of a resurfaced recommendation is carried forward by the
successor's `supersedesRecommendationId` pointer, not by mutating the
predecessor. A dismissed predecessor keeps `dismissed` as its terminal status
permanently.

This decision supersedes the reachable-status clause of ADR 0042's
"Embedded recommendation evidence" bullet. The remainder of ADR 0042 —
workflow observations replacing summary checkpoints, the report-token
capability, storage bounds, and Taskmaster correlation — remains accepted and
unaffected.

## Consequences

- The spec (`specs/taskmaster.md`) and the workflow-observation design doc are
  the authoritative statements of the reachable statuses; both were updated in
  the same change as this ADR (issue #343, PR #410).
- `RecommendationStatus::Superseded` stays in the enum for shared
  deserialization of recommendation files, and tests may still construct it
  for deserialization coverage; no production path assigns it.
- Future reviewers should read ADR 0042's reachable-status list as amended by
  this ADR, not as a standalone contract.
