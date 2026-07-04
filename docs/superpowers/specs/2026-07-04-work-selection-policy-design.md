# Work Selection Policy Design

Date: 2026-07-04
Status: approved

## Summary

Update the repo's work-picking policy from milestone-first to a hybrid stability-first rule.

Agents should prefer work that restores or stabilizes existing functionality before starting new milestone feature work. This includes user-visible bugs, regressions, failing tests, and correctness or data-integrity bugs. When no meaningful stabilization work is open, agents should return to milestone-ordered feature delivery.

## Decision

Adopt a hybrid issue-selection rule:

1. Prefer open issues that fix or stabilize current behavior.
2. Treat regressions, failing tests, correctness bugs, and data-integrity bugs as stabilization work even if they are not yet user-visible.
3. When there is no meaningful stabilization work available, pick from the lowest incomplete milestone and work forward in milestone order.
4. When both a bugfix and a feature slice are plausible, break ties in favor of current usability and data correctness.

## Scope

This change updates repo workflow guidance only. It does not change product scope, milestone definitions, or the issue board as the source of implementation tracking.

## Documentation Changes

- Update `AGENTS.md` so agent workflow guidance reflects the new hybrid rule.
- Update `README.md` so the public repo guidance matches `AGENTS.md`.
