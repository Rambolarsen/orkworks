---
type: Validation Evidence
title: Repository readiness runtime validation
description: Reproducible transport and semantic acceptance evidence for the fourteen-area readiness score.
tags: [orkworks, taskmaster, readiness, validation]
status: stable
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

## Luna medium qualification — failed

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
“usage” wording; the subsequent low-effort matrix below establishes the effect for that profile. The strict
rejection behavior remains in place.

The healthy fixture sources are byte-identical to improved. Healthy 2 selected
one extra source fact during discovery, so it is a no-change workspace trial,
not an identical-final-input trial. Other improved/healthy final inputs match.
Their rating and coverage variation must not be attributed to repository fixes.

These failed results do not establish score usefulness for Luna. They remain
retained separately from the qualified profile below. Acceptance still requires
semantically grounded scored rows, no fabricated execution or complete-coverage
claims, and improvement of the intended component in at least two of three paired
trials without unrelated invented gains. Identical no-change scores are not
required. Live application settings remain unchanged.

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

The bounded current-code `/code-review medium` follow-up approved
`116766131891bb1eac3cb539bd897e04613d6832`: both required findings resolved,
no new Critical/Required findings and no minor deferrals. The reviewer inspected
source/tests and diff formatting, without independently rerunning builds or
provider calls. The full verifier passed 2,269 Rust tests with eleven ignored,
1,537 desktop tests with two skipped, all four integration binaries, native ABI,
reporter, type, build and documentation checks. The final collector ordering and
symlink refinement additionally passed all 26 readiness and 14 context tests.

## Alternative model qualification

An isolated GPT-6.1-Sol medium-effort matrix completed six snapshots. The final
scoring calls for trial 3 of baseline, improved and healthy each hit the existing
90-second native timeout and produced no snapshot. The other six calls appear
conservative and stable: unknown scoping, decomposition and observability remain
unassessed, and the two completed domain pairs change 1 to 2 without unrelated
changes. This medium-effort profile is not qualified by these results. The live application's
model selection was not changed. The low-effort matrix below uses the same model,
fixtures and timeout to assess operational suitability. No failed trial
is repaired, retried or replaced in retained evidence.

## Qualified profile

**PASS: Codex GPT-6.1-Sol, low reasoning effort**, evaluated on 2026-10-25 with
signed `brain-3` and rubric digest
`9f2c7c21b0edfa0e96fd806d9bcc50e2dd983dbf3ab2a45cf5395b0e1aea1a5e`.
All nine independent two-call trials completed and saved snapshots without
repairs, parser rejections or timeouts. Independent review checked all 126 rows:
99 assessed rows were grounded, and 27 Unassessed rows were justified. No required
semantic findings remained. The [retained synthetic evidence](fixtures/repository-readiness-2026-10-25/index.md)
contains exact final inputs, parsed outputs, original byte hashes, source hashes
and public guidance identity. Original outputs and all failed matrices also remain
in a private local evidence archive.

Each value below holds in all three trials per group. U means Unassessed with
null maturity/confidence and an explicit reason.

| Component | Baseline | Improved | Healthy |
| --- | --- | --- | --- |
| Context discovery | 1 | 1 | 1 |
| Progressive disclosure | 2 | 2 | 2 |
| Instruction scoping | U | U | U |
| Architecture awareness | 2 | 2 | 2 |
| Domain knowledge | 1 | 2 | 2 |
| Environment reproducibility | 1 | 1 | 1 |
| Tool accessibility | 1 | 1 | 1 |
| Task decomposition | U | U | U |
| Verification | 1 | 1 | 1 |
| Observability | U | U | U |
| Change safety | 1 | 1 | 1 |
| Feedback loops | 1 | 1 | 1 |
| Durable memory | 2 | 2 | 2 |
| Human oversight | 2 | 2 | 2 |

The controlled glossary change improved domain knowledge 1 to 2 in **3/3 pairs**,
exceeding the required two of three. No unrelated maturity/null value changed.
All improved/healthy source files and final provider inputs matched exactly.
Those no-change trials showed no maturity or coverage variability; prose,
citations and durable-memory confidence varied reasonably between medium/high.
Every snapshot retained honest 11/14 coverage and null raw total/overall score.
No missing evidence became zero, and no completed execution, maintained practice
or technical enforcement was invented.

This qualifies the approved synthetic fixture criteria for this **model/effort
profile only**. It does not qualify Luna, medium effort, other models, arbitrary
repositories or full fourteen-area scoring. Deterministic tests separately verify
full-coverage arithmetic and comparison presentation. No live application model
selection was changed. Source verification and this model-specific evidence do
not establish availability in a published installer.

## Prose-decoder regression qualification

The 2026-10-26 follow-up [#1075](https://github.com/Rambolarsen/orkworks/issues/1075)
adds a retained prose corpus under
`crates/orkworksd/tests/fixtures/assessment-prose.json`. It covers nominal usage,
ordinary make/just phrasing, prospective evidence, executable commands,
arbitrary recipe names, completed-check claims and control characters. Both
complete response decoders exercise every case in each of five prose positions;
the cases do not waive schema, citation, rating or size checks. A scheduler
regression also publishes a complete grammatical readiness response and reloads
its saved history; the independent storage validator uses the same prospective
transition policy and still rejects attached completed-check claims. Prospective
verification noun phrases are allowed only in readiness next transitions and
workflow unverified checks. Descriptive fields retain completed-claim rejection.

An offline regression also decodes all nine previously qualified readiness
outputs against their retained exact inputs and rubric. Each retains 11/14
coverage with no overall score. This checks decoder compatibility, not a new
semantic judgment or model qualification. No provider call is made.

Prompt and native-schema word-avoidance guidance remain unchanged. The parser
now accepts the reproduced grammatical constructions, while other ambiguous
phrasing remains conservative. The Make exception is sentence-initial,
capitalized prose; embedded raw lowercase make retains its prior conservative
rejection, even when it could be read as English. Prospective verification
exceptions use a bounded positive grammar for a single evidence noun fragment,
not a finite-verb denylist. Unknown extensions retain conservative rejection;
past/plural command introductions and attached factual predicates remain blocked. Removing prompt workarounds would change the
qualified input and requires separately retained provider evidence; the
existing model-specific qualification above is not expanded by these tests.
