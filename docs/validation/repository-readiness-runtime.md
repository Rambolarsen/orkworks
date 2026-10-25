---
type: Validation Evidence
title: Repository readiness runtime validation
description: Reproducible transport and semantic acceptance evidence for the fourteen-area readiness score.
tags: [orkworks, taskmaster, readiness, validation]
status: draft
---

# Repository readiness runtime validation

Tracks [#1063](https://github.com/Rambolarsen/orkworks/issues/1063) and the
[readiness specification](../../specs/repository-agent-readiness.md).
This records source verification, not published installer availability.

## Signed guidance and offline proof

The owner-approved `brain-3` format-2 publication supplies all fourteen concepts
and five definitions per concept. Electron and Rust independently verify its
actual generated packaged activation proof with the unchanged public pin.
The public payload matches the exact approved candidate in every field;
[publication operations](../agents/taskmaster-knowledge-publication.md#complete-readiness-rubric-publication)
records artifact identities. Focused starter/proof checks passed 33 Electron
checks and three Rust production-starter checks.

## Reproducing the selected-provider trials

`readiness_quality_tests.rs` is an explicitly ignored live-provider harness.
It uses production capture, guarded inference, collection, strict decoding and
snapshot persistence, plus test-only local input/output capture. It never changes
live settings or live workspace metadata. Its explicit selection file is read
only for the configured model; each isolated runtime uses source-code context.

```bash
ORKWORKS_READINESS_SELECTION_FILE=/private/local/selection.json \
ORKWORKS_READINESS_QUALITY_DIR=/private/local/fresh-readiness-evidence \
  cargo test --manifest-path crates/orkworksd/Cargo.toml \
  selected_provider_readiness_quality_matrix -- --ignored --nocapture
```

Use a fresh, private evidence destination. Existing sample directories are never
overwritten. An optional `ORKWORKS_READINESS_QUALITY_CASE` selects `baseline`,
`improved` or `healthy`; full acceptance requires all three. Each case has three
independent uncached trials, each with at most discovery and scoring calls and
no repair or retry. Retained inputs, outputs, dispatch identity and final outcomes
are necessary for semantic adjudication; transport success alone is insufficient.
The default repository suite never dispatches these live trials.

The fixture is a small TypeScript inventory catalog with a documented entry
point, architecture, setup, test definitions, CI configuration and an empty
lessons record. No verification results or maintenance history are supplied.
The controlled change replaces only `docs/domain.md`: a stub glossary becomes
specific SKU, stock, duplicate and invalid-input definitions with references to
the corresponding code and tests. Healthy repeats the improved repository with
no further source changes. No fixture commands are executed by the assessment.

## Diagnostic trials and dispositions

The selected provider was Codex `gpt-6-luna`, medium effort. The first nine-trial
matrix had two complete snapshots and seven rejected responses. Its raw evidence
is retained privately. Rejections exposed a shared prose false positive for
capitalized Node platform descriptions containing semicolons, and invalid mixed
null maturity/non-null confidence fields. Regressions reproduced both problems
before fixes. Prose handling retains shell-command and completed-verification
rejection; native constraints now distinguish rated and unassessed row shapes.

A second diagnostic matrix had four complete snapshots and five rejected
responses. It exercised the corrected nullable schema but preceded the complete
platform-description correction. Semantic inspection also found ratings whose
own limits acknowledged a missing prerequisite, such as ongoing documentation
maintenance. The final prompt explicitly requires every rubric condition and
forbids awarding a level whose missing requirement appears in its own limits or
next transition. Links alone do not establish ongoing maintenance, and static
configuration does not establish observed execution or reliability. All earlier
failures remain part of the evidence rather than being silently retried.

## Final acceptance

The final Codex `gpt-6-luna`, medium-effort matrix **failed** independent
semantic review of all 126 rows. Six responses became snapshots; three were
rejected. The controlled domain improvement was grounded in all three paired
raw responses (1 to 2), but that alone did not satisfy acceptance.

| Trial | Coverage | Published overall | Domain rating | Snapshot |
| --- | --- | --- | --- | --- |
| Baseline 1 | 13/14 | — | 1 | Yes |
| Baseline 2 | 11/14 | — | 1 | Yes |
| Baseline 3 | 12/14 | — | 1 | Yes |
| Improved 1 | 14/14 | 38/100, unsupported | 2 | Yes |
| Improved 2 | 13/14 | — | 2 | Yes |
| Improved 3 | 14/14 | — | 2 | No |
| Healthy 1 | 11/14 | — | 1 | No |
| Healthy 2 | 12/14 | — | 2 | No |
| Healthy 3 | 13/14 | — | 2 | Yes |

Missing decomposition or scoping evidence became zero in three raw rows.
Planning expectations were also promoted to decomposition capability without an
example or stepwise workflow. An empty lessons destination was treated as an
observed record. Some environment ratings rose despite limits explicitly
acknowledging unverified setup reliability or consistent use. Consequently,
Improved 1's complete coverage and overall number are unsupported, and the
unrelated environment gains fail attribution. Ordinary conservative variation
in discovery, architecture and domain judgments was distinguished from these
unsupported capabilities. No response claimed completed test execution.

The three parser rejects were prospective “Verified build…” noun phrases and
“Currentness and use across common tasks are not established.” The shared
workflow classifier does not fully distinguish these from verification claims
and imperative actions. The readiness prompt/schema now explicitly request
prospective next-transition noun phrases beginning “Evidence of” and descriptive
“usage” wording; a fresh matrix is required to establish the effect. The strict
rejection behavior remains in place.

The healthy fixture sources are byte-identical to improved. Healthy 2 selected
one extra source fact during discovery, so it is a no-change workspace trial,
not an identical-final-input trial. Other improved/healthy final inputs match.
Their rating and coverage variation must not be attributed to repository fixes.

This failed evidence does not establish score usefulness. The runtime PR remains
draft, with no merge or installer claim. Required criteria remain semantically
grounded scored rows, no fabricated execution or complete-coverage claims, and
improvement of the intended component in at least two of three paired trials
without unrelated invented gains. Identical no-change scores are not required.
A more capable model may be qualified in isolated fixtures if selected by the
owner; live application settings must remain unchanged without that instruction.

## Historical freshness review

Current-diff review at `e7210343` found two required freshness errors: transient
workspace/runtime identity made an unchanged reopened workspace look changed,
and every restoration failure looked like a detected repository change. Both
regressions failed before corrections. Persisted input fingerprints now omit
transient instance/generation while independent admission checks retain them.
The collector distinguishes observed content/catalog differences from unavailable
or unsafe reads before the status projection chooses changed or unknown.
Focused readiness checks passed 26 tests with the opt-in live matrix ignored;
regressions also preserve stale-instance/generation cancellation and classify
changed, removed, non-text and symlinked follow-up evidence.
