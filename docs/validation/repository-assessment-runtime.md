---
type: Validation Evidence
title: Repository assessment runtime validation
description: Deterministic safety checks and the unresolved selected-model usefulness gate for issue 931.
tags: [orkworks, taskmaster, assessment, validation]
status: draft
workflow_status: blocked
---

# Repository assessment runtime validation

Tracking: [#931](https://github.com/Rambolarsen/orkworks/issues/931).
Authority: [approved design and quality criteria](../superpowers/specs/2026-10-18-repository-led-brain-assessment.md)
and [ADR 0086](../adr/0086-repository-led-taskmaster-assessment.md).
This records source validation on 2026-10-18, not published installer behavior.

## Deterministic evidence

The scoped assessment suite passed 84 tests after the review correction and
main's #934 contract clarifications, including
strict discovery/final decoding, shared evidence bounds, safe symlink rejection,
zero/one/two dispatches, stale first-stage admission, final-call file/settings/
workspace mutation, uncited follow-up freshness, format-2 restart interruption,
legacy admission and pending final-commit recovery. Additional fixtures enforce
native UTF-8 path identity and automatic governing-instruction closure within the
shared budget; inaccessible instructions prevent a proposal. A further end-to-end
fixture passed both partial-ancestor and instruction-only budget-overflow cases,
asserting one dispatch, no partial target evidence and an explained no-proposal
report. A pending-recovery narrowing
regression first failed with “recovery restored a disallowed path”; dropping the
manifest whenever recovery drops its report made that regression pass.

The first full `bash scripts/verify-repo.sh` run passed under Node 22.23.3,
including Rust formatting/build/tests, desktop checks and build, docs checks and
build, diff checks, documentation currency and worktree checks. Final current-diff
review and verification are recorded in the PR; this paragraph does not replace
those gates.

## Selected-provider matrix

The explicit ignored test `selected_provider_quality_matrix` uses only synthetic
repositories and the configured Codex `gpt-6-luna`, medium effort selection.
It activates the packaged signed `brain-1` starter in isolated test runtimes;
provider selection and the user's workspaces are not changed. No repository
checks run as part of assessment. The corrected fixtures use built-in Node unit
tests, an explicit package-manager version and a matching minimal lockfile.
The refuted case delegates to a related verification entry point.

The final matrix before #934 collector hardening failed. It does not establish
usefulness for the hardened collector; no positive quality claim is carried over.
Sanitized local evidence is retained at
`/private/tmp/orkworks-931-quality-final`, with selection, fixed fixture files,
per-run records, model outputs and `verdict.json`. Earlier failed diagnostic
runs remain under sibling `orkworks-931-quality-*` directories. No production
prompts, uncited private source files or private Brain pages were retained.

| Fixed fixture | Completed valid assessments | Surfaced proposals | Designed-gap threshold |
| --- | --- | --- | --- |
| Positive no-op test script, no observations | 0/3 | 0 | Failed: below 2/3 |
| Positive gap with 200 old glyph observations | 0/3 | 0 | Failed: below 2/3 |
| Healthy linked unit tooling and CI | 2/3 | 0 | No invented proposal; one rejected run |
| Candidate refuted by related verification entry point | 0/3 | 0 | No surfaced proposal; all runs rejected |

Rejections included unsafe execution-style prose, malformed/unknown/duplicate
fields and malformed JSON. The runtime rejected those outputs without publishing
recommendations. Rejected runs are failures, not valid no-proposal results.
Prompt clarification did not establish the quality threshold. No retries or
fallback models were added to an assessment; diagnostic evaluations were fresh
explicit attempts using the same selection.

A fresh complete matrix on PR #938 head `44a60f5808` after the #934 collector
hardening also failed. Both positive fixtures completed 0/3 valid assessments;
healthy and refuted each completed 1/3 valid no-proposal assessments. No proposals
surfaced, so no semantic adjudications were applicable. Sanitized evidence is
retained at `/private/tmp/orkworks-931-quality-pr938-current`; the selection remained
Codex `gpt-6-luna`, medium effort. This precedes the subsequent normal seed-budget
truncation correction and establishes no quality claim for that correction.

PR review separately exposed ordinary seed-byte truncation being mistaken for
incomplete governing-instruction coverage. A regression reproduced this before
the correction and passed afterward for an exactly full 24-KiB seed and a next
target larger than remaining seed capacity. Both permit bounded follow-up and
manifest reconstruction; genuine instruction-only overflow still returns an
explained no-proposal result. The scoped assessment suite passed 86 tests, with
2 explicit live-evaluation tests ignored.

**Disposition: the usefulness gate remains failed.** Do not claim that this solves
proposal usefulness or mark #931 complete. Keep the runtime PR in draft pending
resolution and a fresh complete matrix. The harness requires explicit retained
semantic adjudication for every surfaced proposal, covering factual grounding,
the designed gap, bounded change, validation and rollback; merely counting
`proposed` is insufficient. `quality_verdict_from_recorded_reviews` checks those
adjudications without dispatching another provider call.
