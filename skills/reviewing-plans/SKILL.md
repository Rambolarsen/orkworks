---
name: reviewing-plans
description: Use before drafting or substantially revising an implementation plan, and when reviewing a new, revised, or received plan before execution or handoff, especially when scope, complexity, or wording is unclear.
---

# Reviewing Plans

Keep the plan as small and clear as the agreed outcome allows. Assess delivery
complexity separately from plan quality: necessary work can be complex and
still have a good plan.

## Before drafting

Read the agreed requirements and relevant repository evidence. State the
outcome and mandatory constraints. Reuse investigated findings from
`surfacing-blind-spots` when available.

Rate each dimension 1–5 with one concrete reason. Use 2 or 4 between the
anchors. Missing evidence is **Unknown**, never an invented score. Known
unknowns can still justify a high uncertainty rating. For example, untested
rollback means Reversibility is Unknown; it does not prove recovery is hard.

| Dimension | 1: Low | 3: Moderate | 5: High |
| --- | --- | --- | --- |
| Dependencies | One owner; no coordinated rollout | Several known interfaces or owners | Several independent releases or unresolved coordination |
| Blast radius | Isolated behavior | Shared component with known callers | Core behavior across many consumers |
| State changes | No persisted state or contract change | Additive change with compatibility work | Destructive migration or breaking contract |
| Reversibility | Local revert; no data recovery | Coordinated rollback with known steps | Recovery requires manual data repair or loses data |
| Uncertainty | Existing, verified approach | Bounded questions answerable by inspection | Key behavior needs experiments or remains unverified |

Show individual ratings and reasons. Sum out of 25 only when all five are
scored; otherwise mark the total incomplete. These are local judgment anchors,
not measured probabilities. There are no automatic approval or RFC thresholds;
a total must not hide a serious individual concern.

Name a response to each concern that changes the approach: reuse, a specific
investigation, a smaller delivery step, or compatibility/recovery verification.
If implementation depends on an unresolved assumption, plan a bounded
investigation with an observable result before committing to dependent steps.
Carry mandatory requirements forward when proposing phases; scope changes
need user agreement.

## Before execution or handoff

Check the actual draft, including external `writing-plans` output, received
plans, and substantial revisions. Reassess complexity when the approach or
evidence changes.

- **Scope:** Every task serves the agreed outcome or a necessary prerequisite.
  Remove unrelated cleanup and speculative features. Preserve approved
  requirements when simplifying.
- **Simplicity:** Justify new layers, dependencies, and coordinated changes.
  Keep needed compatibility, recovery, and verification work.
- **Clarity:** Name the action, affected component, and expected behavior.
  Replace “handle appropriately” with the actual decision.
- **Verification:** Give relevant files or interfaces, checks, expected
  results, and a stopping point. Brevity must not conceal missing details.
  Include implementation code only to resolve a material ambiguity. Remove
  repeated facts and generic template boilerplate.

Fix issues within the agreed scope before handoff. If information or approval
is missing, state what is needed and hold the affected implementation steps.
Scores never grant implementation approval or bypass repository gates.

## Report

Include the rating table, **Total: n/25** or **Total: Incomplete** with the
unknown dimensions, and **Plan quality: Ready / Revise / Investigate** with a
reason and next action. Put this in the plan, or the response for read-only
reviews. Ready means ready for the next applicable approval or execution gate.

For example: **Investigate — rollback is Unknown.** Rehearse mixed-version
compatibility and recovery in staging before choosing rollout order. Keep all
supported clients in scope. Replace “roll back if necessary” with verified
steps.

Do not create a second assessment document or require a written plan merely
to run this skill on a small bounded change.
