---
name: reviewing-plans
description: Use before drafting or substantially revising an implementation plan, and when reviewing a new, revised, or received plan before execution or handoff, especially when scope, complexity, or wording is unclear.
---

# Reviewing Plans

Keep plans clear; assess complexity separately from plan quality.

## Before drafting

Read requirements and evidence; state the outcome and constraints. Reuse
`surfacing-blind-spots` findings.

## Plan shape

Use OKF v0.2 metadata (`type`, `title`, `description`, `tags`, `status`). Lead
with the user outcome and plain-language pass condition. Link issues, specs,
concepts, and related plans; state their relation and dependencies, not copied
content.

Split only independently valuable outcomes with their own acceptance checks.
Give each a plan; use a short index for multiple outcomes, status, order, and
dependencies. Keep coupled steps together when they share an end-to-end check;
length alone is not a reason to split. Keep each plan readable on its own. Put
clear implementation details in issues/specs, but retain scope, safety,
compatibility, recovery, and verification constraints. Simplify repetition and
jargon, never requirements.

Rate each dimension 1–5 with a reason; use 2 or 4 between anchors. Missing
evidence is **Unknown**, not guessed. Unknown rollback makes reversibility
Unknown.

| Dimension | 1: Low | 3: Moderate | 5: High |
| --- | --- | --- | --- |
| Dependencies | One owner | Known interfaces/owners | Independent releases or unresolved coordination |
| Blast radius | Isolated | Shared component | Core behavior across consumers |
| State changes | None | Additive with compatibility | Destructive or breaking |
| Reversibility | Local revert | Known coordinated rollback | Manual repair or data loss |
| Uncertainty | Verified approach | Bounded inspection | Unverified key behavior |

Show ratings and reasons. Sum only with all five; otherwise mark incomplete.
Scores are judgment aids, not approval thresholds; call out serious concerns.

For each concern, name an approach change: reuse, investigation, smaller
delivery, or compatibility/recovery check. Resolve assumptions before dependent
work; preserve mandatory requirements in every phase. Scope changes need user
agreement.

## Before execution or handoff

Review drafts, received plans, `writing-plans` output, and substantial
revisions. Reassess complexity when evidence changes.

- **Scope and simplicity:** Every task serves the outcome or prerequisite.
  Remove unrelated work; justify new layers. Preserve requirements and needed
  compatibility, recovery, and verification.
- **Clarity:** Name the action and behavior plainly; replace vague directions
  with the actual decision.
- **Verification:** State the check and expected result. Link to detail instead
  of repeating it, but do not hide ambiguity or a missing check.

Fix issues before handoff. Name missing evidence or approval and hold dependent
work. Scores never grant approval or bypass gates.

## Report

Report in the plan or review: ratings, total (`n/25` or `Incomplete` with
unknowns), and plan quality (`Ready / Revise / Investigate`) with reason and
next action. Ready means ready for the next gate.

Do not create a second assessment document or require a written plan merely
to run this skill on a small bounded change.
