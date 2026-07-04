---
name: auditing-test-honesty
description: Use when asked to audit test quality or generate test-improvement tasks, when a bug shipped despite passing tests covering that area, or when reviewing whether a test suite actually pins the behavior its names claim.
---

# Auditing Test Honesty

## Overview

**Core question: which passing test would survive the bug it's named after?**

A test is *dishonest* when its name claims to pin a behavior but its assertions would still pass if that behavior broke. Dishonest tests are worse than missing tests: they block new coverage ("that's already tested") and give TDD-produced code false confidence. This audit samples tests in a bounded area, tries to falsify each one's claim, and files issues for the ones that fail.

**A suspicion you did not falsify-test is not a finding.** For every candidate, demonstrate the dishonesty — mentally trace the broken-behavior case through the assertions, or actually mutate the code and watch the test stay green.

## Process

1. **Bound the area** — one test file cluster or the tests of one module (e.g. `crates/orkworksd/src/runtime/`, `apps/desktop/tests/`). Start bounded, follow the data flow (see `skills/surfacing-blind-spots/`).
2. **Sample honestly** — read every test in the area if feasible; otherwise sample the ones guarding the most load-bearing behavior, not the easiest ones.
3. **For each test, state its claim** (usually the name) **and hunt these patterns:**

| Pattern | Symptom |
| ------- | ------- |
| Tautology | Asserts a value the test itself just set, or asserts the mock returns what the mock was told to return |
| Over-mocking | So much is faked that the code under test is barely executed |
| Under-assertion | Exercises the behavior but asserts only a fragment (status code but not body; that a call happened but not with what) |
| Survivable mutation | Deleting or inverting the guarded branch would not fail the test |
| String-presence proxy | Asserts source/script *contains a substring* instead of exercising behavior |
| Missing negative | Only the happy path is pinned; the error/edge branch the name implies is unasserted |

4. **Falsify before filing** — for the top suspects, apply the mutation for real: break the guarded behavior locally, run the test, confirm it stays green, then revert. A test that fails the mutation is honest; drop the suspicion.
5. **Filter and file** — apply the guardrail filter and issue format from `skills/surfacing-blind-spots/` (Phases 3–4). One issue per behavior left unpinned, stating: the test, its claim, the mutation that survives, and the assertion that would catch it.

## Red flags — stop and restart

- An issue saying "improve test coverage for X" with no named test and no surviving mutation
- Filing without running (or rigorously tracing) the mutation
- Counting assertion *quantity* as honesty — one sharp assertion beats ten shallow ones
- Flagging intentional characterization/smoke tests as dishonest — check for a comment or commit message stating the intent first

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Auditing only unit tests | Integration and script-content tests are where string-presence proxies live |
| Proposing test rewrites in the issue | The issue names the unpinned behavior; the fix is the implementer's job |
| Treating a green mutation run as proof of a bug | It proves a *test gap*; the behavior may still be correct today |
