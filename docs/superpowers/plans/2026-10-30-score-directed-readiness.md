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
Design: [accepted ADR 0098](../../adr/0098-score-directed-readiness-improvements.md).
Status: owner approved the concrete design, implementation and maximum twenty-call
synthetic qualification budget on 2026-10-30 ("keep going"). The earlier one-run
approval was separate and is exhausted. Runtime execution now follows ADR 0098.

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
