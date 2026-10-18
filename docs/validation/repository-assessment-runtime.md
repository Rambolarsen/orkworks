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

A subsequent parser replay separated model wire failures from validator false
positives. The prose guard confused English “Make the package…” with a tool
invocation, configuration descriptions with execution instructions, and indirect
questions or rejected no-ops with completed-check claims. Paired regressions
reproduced these errors and also caught an older qualifier leak across sentence
and semicolon boundaries. The correction scopes prose to clauses, distinguishes
repository descriptions from reader instructions, and attaches capability modals
to their verification predicate. Independent review reproduced two bypasses in
the first correction; both received failing regressions and were corrected before
re-review approval. Strict JSON, duplicate/unknown-field, citation, size and raw
command checks remain in force. Final-stage guidance now includes the exact-key
and declarative-style rules already present in discovery, with generic examples.
The scoped suite passed 88 assessment tests, with 2 explicit live tests ignored.

The fresh complete matrix after that correction still failed. Evidence is retained
at `/private/tmp/orkworks-931-quality-prose-fixed` and its selected provider remains
Codex `gpt-6-luna`, medium effort. Both positive fixtures completed 0/3 valid
assessments; healthy completed 2/3 valid no-proposal assessments and refuted
completed 1/3. No proposal surfaced for semantic adjudication. Further ordinary
English `make` phrases and validation requirements encountered prose rejection;
other responses contained malformed or duplicate/unknown fields, and at least
one final title remained an execution-style imperative. An attempted broader
grammar expansion exposed literal-command and completed-claim bypasses in review
and was removed; it is not part of the retained correction. At that revision these were unresolved
compatibility failures, not evidence of a useful or completed implementation.
The targeted correction does not resolve general prose classification or the
selected model's strict-envelope compliance. No acceptance thresholds were
weakened and no inference retries, fallback or response repair were added.

## Native response constraints

The owner approved the [native Codex shape design](../superpowers/specs/2026-10-18-assessment-response-constraints.md)
before integration. Codex CLI 0.163.0 accepted both static stage schemas with the
unchanged `gpt-6-luna` / medium selection in an isolated synthetic compatibility
probe. Both final messages passed the real strict application decoders. The
probe preceded production opt-in; evidence is retained at
`/private/tmp/issue938-response-shape-probe`.

Paired regressions failed before production wiring and passed afterward for the
stage-specific constraint marker and unsupported-schema failure without another
invocation. The scoped assessment suite passed 90 tests with 2 explicit live tests
ignored. A separate read-only review approved the protocol integration. Static
schema files stay in the existing inference temporary directory through cleanup;
ordinary analysis, custom adapters and other native profiles stay unconstrained.
Record format 2 and accepted manifest versions remain unchanged.

The first constrained matrix, retained at
`/private/tmp/orkworks-931-quality-structured`, produced closed JSON throughout.
Healthy and refuted fixtures each completed 3/3 valid no-proposal results.
Positive completed 0/3 valid assessments; positive-with-old-noise completed 1/3.
That one surfaced proposal received explicit semantic adjudication against the
cited no-op package script, CI invocation, required unit-test evidence and existing
arithmetic test. It addressed the designed gap with bounded scope, concrete
unverified validation and restoration of the prior script as rollback. The
retained verdict still failed the positive threshold. Remaining rejections were
prose classification failures; shape constraints alone did not establish
usefulness.

Review identified conflicting prompt guidance that requested validation inside
proposal summaries, encouraging phrases rejected as completion claims. Subsequent
guidance keeps the bounded change, benefit, validation scope and rollback in the
proposal summary, with detailed conditions in explicitly unverified checks. The strict decoder, model,
effort, two-call cap and quality criteria remain unchanged. A discovery-description
scope leak found during review was corrected before the final fresh evaluation.

Two intermediate guidance matrices remain diagnostic evidence. The first,
`/private/tmp/orkworks-931-quality-guided`, completed all 12 trials, but semantic
review marked its six proposals deficient because their summaries omitted the
required validation scope. The corrected-scope matrix at
`/private/tmp/orkworks-931-quality-constrained-final` completed positive 0/3,
positive-with-old-noise 2/3, healthy 1/3 and refuted 3/3. Both surfaced proposals
passed explicit grounding, scope, validation and rollback review; the overall
verdict still failed. No rejected response was counted as a valid no-proposal.

An isolated experiment added static prose patterns to provider schemas. Its
selected-model compatibility invocation failed, so the pattern was removed;
that log does not establish the underlying rejection reason. No pattern is
retained in the implementation. The final bounded wording experiment asks for
prospective declarative descriptions and nouns for execution concerns, while
preserving validation scope in summaries and detailed unverified conditions.

## Final reviewed evaluation

The final fresh matrix at `/private/tmp/orkworks-931-quality-prospective` uses the
same Codex `gpt-6-luna`, medium effort and signed `brain-1` selection. The exact
reviewed source hashes are retained in `code-identity.json`; no code changed
during this evaluation. All 12 trials used distinct canonical workspaces,
actually dispatched discovery and retained non-reused results. Each positive
trial also dispatched the final stage; negative discovery-only conclusions
correctly avoid another call.

| Fixed fixture | Valid assessments | Adjudicated designed gaps | Invented proposals |
| --- | --- | --- | --- |
| Positive no-op test script | 3/3 | 3/3 | 0 |
| Same gap with old observations | 3/3 | 3/3 | 0 |
| Healthy linked tooling and CI | 3/3 | Not applicable | 0 |
| Refuted candidate | 3/3 | Not applicable | 0 |

An independent reviewer checked every cited repository hash and excerpt against
its actual trial files and every Brain citation against the signed starter
bundle. Each of the six proposals identifies the no-op test-script gap, a bounded
change to connect the existing arithmetic test, expected CI benefit, validation
scope and restoration of prior configuration. Detailed conditions remain
explicitly unverified. Each retained `review.json` records grounding, designed
gap, boundedness, validation, rollback and a per-trial rationale. Healthy results
recognize aligned configuration without inventing execution evidence; refuted
results either inspect the related verifier or conservatively decline a proposal
from bounded seed evidence.

**Disposition: the selected-provider usefulness gate passes.** The zero-dispatch
`quality_verdict_from_recorded_reviews` check passed with those adjudications and
an empty failure list. Earlier failed matrices remain failures. This establishes
the approved synthetic threshold, not a guarantee for arbitrary repositories or
future model behavior; assessment did not execute fixture tests or CI. Validation
wording remains descriptive scope rather than executable commands. One proposal
unnecessarily mentions test changes; wiring the existing test is the narrower
first step, but the recommendation remains within the bounded test target.

Full repository verification passed for the protocol integration: 2,115 Rust unit
tests plus integration checks, 1,350 desktop tests passed with one skip, desktop
build and docs build. After final wording clarification, 90 assessment tests and
20 inference tests passed, with explicit live probes ignored in those scoped
runs; formatting, documentation metadata/index and close-out checks also passed.
Read-only reviews approved the protocol integration and current prose guidance.
The documentation-currency advisory needs no further user-doc edit: the existing
source-build assessment guide describes this bounded behavior and makes no new
published-installer claim. PR CI and current-head review remain separate merge
gates.
