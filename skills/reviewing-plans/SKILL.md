---
name: reviewing-plans
description: Use when reviewing a written implementation plan before execution or handoff, or after material scope, risk, interface, or acceptance-criteria changes.
---

# Reviewing Plans

Follow the preparation tiers in root `AGENTS.md`. Routine work does not require
a written plan or this skill. For feature choices, a short in-chat approach
usually suffices. Review a written plan once before execution/handoff; reuse
approved context and review only material changes in later revisions.

## Plan shape and review

Use concise OKF metadata (`type`, `title`, `description`, `tags`, `status`) for
saved plans. Lead with the user outcome and pass condition. Link the issue,
specs, concepts, and dependencies instead of copying their contents.

- **Scope:** Each deliverable serves the outcome or a necessary prerequisite.
  Split only independently valuable outcomes with separate acceptance checks;
  keep coupled steps together. Remove unrelated refactors and speculative layers.
- **Clarity:** State decisions, owner/module boundaries, material interfaces,
  and dependencies. Preserve exact safety, compatibility, and recovery constraints.
- **Coverage and verification:** Each mandatory requirement has a delivery
  step and meaningful check with an expected result. Link existing criteria or
  test detail where sufficient; resolve uncovered requirements before execution.

Plans do not require full implementation or test code, repeated snippets,
minute-by-minute steps, or file skeletons. A small code example is useful only
when it resolves a contract ambiguity. These local constraints override
external `writing-plans` templates.

## High-risk work

For architecture, protocol/schema migration, security-sensitive, or explicitly
gated work, reuse the `surfacing-blind-spots` investigation and assess these
dimensions with evidence. Use 2 or 4 between anchors. Missing evidence is
**Unknown**; unknown rollback makes reversibility Unknown.

| Dimension | 1: Low | 3: Moderate | 5: High |
| --- | --- | --- | --- |
| Dependencies | One owner | Known interfaces/owners | Independent releases or unresolved coordination |
| Blast radius | Isolated | Shared component | Core behavior across consumers |
| State changes | None | Additive with compatibility | Destructive or breaking |
| Reversibility | Local revert | Known coordinated rollback | Manual repair or data loss |
| Uncertainty | Verified approach | Bounded inspection | Unverified key behavior |

Record ratings/reasons in the same plan; sum only with all five known,
otherwise mark incomplete. Scores are judgment aids, never approval thresholds.
Treat concerns through investigation, smaller delivery, reuse, or explicit
compatibility/recovery checks before dependent work.

## Disposition

Record `Ready / Revise / Investigate`, material findings, and the next action
in the existing plan or handoff. Fix findings before dependent implementation.
Ready means ready for the next applicable gate. Existing authorization remains
valid within scope; missing authority or a specific product gate still blocks
dependent work. Do not create a second assessment document, repeat a review
of unchanged artifacts, or require a written plan merely to apply this skill.
