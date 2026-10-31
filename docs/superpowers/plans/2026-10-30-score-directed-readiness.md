---
type: Implementation Plan
title: Score-directed readiness suggestions
description: Gated implementation plan for feasible scoring evidence and recommendations tied to readiness capabilities.
tags: [orkworks, taskmaster, readiness, implementation]
status: draft
workflow_status: in-progress
---

# Score-directed readiness suggestions

Tracking: [#1175](https://github.com/Rambolarsen/orkworks/issues/1175).
Design: [accepted ADR 0100](../../adr/0100-score-directed-readiness-improvements.md).
Status: owner approved the concrete design, implementation and maximum twenty-call
synthetic qualification budget on 2026-10-30 ("keep going"). The earlier one-run
approval was separate and is exhausted. Runtime execution now follows ADR 0100.

## Outcome and pass conditions

A combined assessment can admit affordable complete evidence closures, save an
honest readiness result, and seek one practical change explicitly tied to an
assessed capability. A missing fact or low rating never manufactures a defect.
Scoring formulas, old history and standalone analysis behavior remain unchanged.

Implement inline in the owning worktree after one approval of the concrete ADR,
this plan, and its finite synthetic qualification budget. No second coding
harness is launched. Runtime/code review follows the root review gate.

## Deliverables and checks

- [x] **Feasible readiness evidence.** In `assessment_context.rs`, readiness's
  discovery schema/transport markers, `readiness.rs`, and the independent
  Electron/renderer readiness validators, implement collector 4 and discovery 2
  as specified. Test a full seed plus eight large preferences, shared ancestor
  instructions, an omitted expensive item followed by an affordable item, exact
  byte/file boundaries, and zero admitted extras. Assert every admitted closure
  is complete, every omission is represented, and final scoring runs within the
  existing limits. Changed/unreadable paths fail rather than being omitted.
  Replay currentness using the recorded admitted set. Existing workflow
  requested-set and collector behavior must remain unchanged.
- [x] **Score-directed assessment context.** In `readiness.rs`, `assessment.rs`,
  and the existing context/guidance owners, bind the exact newly saved score,
  signed criterion index, bounded area pointers and purpose to both suggestion
  stages. Test partial/null ratings, maturity four, a positive confirmed gap,
  counterevidence, unsafe generated transition text, and unrelated suggestions.
  Assert at most four calls, fresh repository citations and mandatory concept
  citations; no proposal may cite a score as proof or claim a numeric gain.
- [x] **Durable link and presentation.** Extend `assessment_report.rs`, the
  existing recommendation assessment provenance, access/currentness handlers,
  and independent desktop contract types with the ADR's references. Preserve
  supported legacy formats and bounds. Test commit/restart/parent-child linkage,
  missing or unrelated scores, rubric changes, access narrowing, history
  eviction, corruption/oversize, and unchanged existing **Fix with AI** gates.
  Show the exact capability and saved assessment beside its existing card.
- [ ] **Quality and close-out.** Retain the five synthetic combined trials
  specified by the ADR, with no more than twenty provider calls and no retries
  or model substitution. Independently adjudicate grounded change, named
  capability, next condition, benefit and safety. Record failures honestly.
  Run scoped regressions, `bash scripts/verify-repo.sh`, and current-code
  review. Open/adopt the implementation PR and follow its terminal lifecycle;
  close #1175 only when its full acceptance criteria are met.

## Risk and recovery review

| Dimension | Rating | Evidence and mitigation |
| --- | --- | --- |
| Dependencies | 3 | Existing score, collector and assessment owners; no new external service. |
| Blast radius | 3 | Shared collector/persistence code; readiness-purpose isolation and legacy tests required. |
| State changes | 3 | New bounded collector/assessment formats and additive provenance; old reads retained. |
| Reversibility | 3 | Disable new combined admission and forward-fix; retained new records cannot safely downgrade. |
| Uncertainty | 4 | Current live run hit evidence limits; changed prompt usefulness remains unqualified. |

Total: 16/25. Ratings size investigation and checks, not approval thresholds.
Re-run semantics, source revalidation and legacy display are required acceptance
checks, not work deferred to a later hardening task.

Plan review: **Ready for owner decision**, not implementation approval. One
independent builder review raised two outcome ambiguities. Both were resolved
without changing scope: all-budget-omitted preferences still reach seed-only
scoring, while any encountered path/access/I/O/currentness failure aborts the
entire attempt even when other preferences fit. Add explicit mixed-failure and
all-omitted coverage assertions to the first deliverable. Owner approval was subsequently received on 2026-10-30; implementation is in progress.

## Execution evidence and bounded handoff

The first three deliverables are implemented in draft PR #1178. Deterministic
checks cover exact budget boundaries, unread omissions, exact source context,
unknown/maximum levels, unrelated concept rejection, recovery and source eviction.
The [five fixed synthetic samples](../../validation/fixtures/score-directed-readiness-2026-10-30/index.md)
consumed seventeen provider calls and **failed usefulness qualification**: one of
three positive samples earned credit, with two required. Both negative controls
produced no recommendation. No failed sample was retried or replaced. The
remaining three calls are not permission to replace samples or change models.
Keep the PR draft and #1175 open; this branch is not a claim that usable analysis
is solved. Full verification and independent current-code review are recorded in
the PR before handoff. A changed inference approach and any additional live
qualification require a concrete next scope and fresh authorization.

## Offline prompt correction, 2026-10-31

The owner requested continued work after the CI handoff. Retained failures isolate
two model mistakes: literal command text in scoring rationale, and treating a
documented coverage claim as proof despite a visible test body without assertions.
Apply bounded prompt corrections within the accepted authority and call limits:
scoring prompt v4 gives safe role-description examples and distinguishes visible
assertions from documentation; focus prompt v2 checks capability prerequisites
before next-level optimizations. Preserve the single-candidate refutation gate,
strict prose validator, signed citations, unknown-evidence rules and four-call cap.
Collector/discovery schemas remain 4/2. Collector-4 prompt-v3 history remains
readable, while the changed prompt identity invalidates currentness and reuse.
The failed corpus stays unchanged and receives no new credit from offline replay.
No additional live calls are authorized or made by this correction pass. A new
fixed qualification set requires a concrete budget decision after the corrected
implementation is reviewable.

### Results of the separately authorized v4 set

The owner approved one new five-fixture set, at most twenty calls, by answering
“keep going” to that explicit request. It consumed sixteen calls; unused budget
does not authorize replacement samples. The [retained independent review](../../validation/fixtures/score-directed-readiness-v4-2026-10-31/index.md)
still fails the gate: only the CI proposal earns credit. The ambiguous skipped
suite proposal and missed assertion defect receive none; the unknown control
fails prose validation. Preserve both failed sets.

The unchanged unknown-control output now supplies an offline regression for a
narrow false positive in negative noun lists (“issue breakdown”). Correcting
that grammar preserves imperative/command rejection and unknown totals, and
never grants retrospective quality credit. The remaining investigation problem
is evidence planning before a positive candidate exists. Changing that protocol
requires a separately reviewed concrete design before dependent implementation;
do not spend more calls on prompt-only replacements.

## Approved evidence-first follow-up (ADR 0099)

[Accepted ADR 0099](../../adr/0099-readiness-evidence-before-candidates.md) is the
concrete next decision. It moves only combined readiness discovery from candidate
justification to bounded evidence planning. It does not add calls or tools, relax
proposal grounding, change standalone discovery, or expand access.

Owner approved implementation on 2026-10-31 ("go ahead"), with no additional
live provider calls. Delivery:

1. Add the readiness-only closed investigation response/parser and native shape,
   with rejection tests for unknown/duplicate keys, unsupported concept pages,
   invalid catalog IDs and oversized selections. Keep standalone discovery-v1.
2. Route a captured readiness continuation through that evidence plan and final
   assessment, retaining current signed-page pinning, manifest restoration,
   currentness/lease/recovery fences and final provenance. Advance focus identity.
   Prove candidate-free evidence reading does not grant proposal/action authority.
3. Verify honest final no-proposal for both negative cases, useful reuse of an
   existing asserted suite, and a distinct visible assertion gap. Run repository
   checks and fresh current-code review. Only then request separate live budget;
   preserve and independently adjudicate every fixed sample without replacement.

| Dimension | Rating | Evidence and mitigation |
| --- | --- | --- |
| Dependencies | 3 | Known parser, native constraint, assessment and context owners. |
| Blast radius | 3 | Shared orchestrator; server-owned purpose isolates standalone behavior. |
| State changes | 2 | Private response shape changes; persisted format stays 3 with existing digest/read IDs. |
| Reversibility | 3 | Disable combined admission and forward-fix; old code must reject changed identity. |
| Uncertainty | 4 | Ordering failure is retained; improved inference remains unqualified until fixed trials. |

Total 15/25. The owner subsequently approved implementation; no live calls
are included in that approval.
Plan review: **Ready for owner design decision**; the owner subsequently
approved implementation on 2026-10-31. The review itself was not approval.
One fresh-context medium review checked ADR 0099 and this material plan section
for buildability, bounded evidence/authority, compatibility/recovery and honest
qualification. No actionable findings. The review separately approved the narrow
prose correction; it did not run provider calls or treat offline replay as quality.

Evidence-first implementation: the real orchestrator regression was
observed rejecting a candidate-free plan before the change, then accepting it,
reading the selected implementation and reaching final assessment in four calls.
Closed-plan, both no-proposal routes, signed citations and existing recovery/access
regressions are deterministic fixtures. Scripted final responses establish evidence
availability and routing only; they do not qualify model usefulness or regrade the
retained failures. Fresh live qualification still requires separate authorization.


Fresh medium code review found that standalone admission could reuse a combined
report and reconstruct its readiness source. A failing router regression confirmed
that cache collision. Admission now has an explicit standalone capture purpose,
reuses only format-2 reports, and avoids loading failed score sources when no
proposed assessment recommendation requires a freshness fence. The regression
passes for both completed and failed/evicted combined sources; standalone still
uses discovery-v1 and produces a format-2 report. Combined recommendation,
currentness and recovery consumers retain their saved purpose. Fresh re-review of
the full current diff found no remaining actionable issues. No live calls were made.
