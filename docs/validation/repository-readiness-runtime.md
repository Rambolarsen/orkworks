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


## Combined assessment follow-up (#1091)

The owner approved [Assess and suggest](../superpowers/plans/2026-10-26-readiness-suggestions.md)
on 2026-10-27. Its regression first reproduced an HTTP 404 and missing visible
next steps/model limits. The combined route now reuses readiness and the existing
workflow assessment under one analysis lease; synthetic transport checks cover
one grounded recommendation in four calls, fresh evaluation after a cached
no-proposal, active-card admission, independent stage failure, input changes and
exact-child recovery. These deterministic checks do not qualify a model.

A 2,100-file skill-tree fixture reproduced starvation of linked architecture and
domain guidance from the bounded catalog. Collector version 3 prioritizes current
documentation during traversal and catalog selection. It retains exclusions,
unsafe-link checks and existing byte/path/depth bounds. Version-2 historical
manifests remain readable and cannot masquerade as current version-3 input.
The original qualification above applies to its retained collector/prompt inputs;
new collector qualification must be supported by separately retained trials.


### Collector-3 live trial disposition

**FAIL: Codex GPT-6.1-Sol, low effort, collector 3**, evaluated on 2026-10-27
with the unchanged readiness-v1 prompt and signed brain-3. All nine uncached
two-call trials completed without repair, retry, rejection or timeout. The
[exact retained synthetic inputs and outputs](fixtures/readiness-suggestions-2026-10-27/index.md)
include byte hashes and all ratings. Independent semantic review inspected all
126 rows: 99 rated, 27 unassessed. The three unknown areas remained unassessed in
every trial; all snapshots honestly have 11/14 coverage and no overall number.
Domain knowledge improved 1 to 2 in all three controlled pairs. No execution,
reliability, CI outcome, technical enforcement or unsupported complete-coverage
claim was found.

One finding prevents qualification: healthy-2 rates durable memory 1 because it
claims the root guide lacks a capture procedure. Supplied fact 0 explicitly
instructs recording lessons in a named persistent file, which supports the rubric's
known low-friction capture action at level 2. Other trials correctly rate this
same fact set 2. The conservative underrating and all original outputs are
retained; no failed sample was repaired or replaced.

The application therefore makes no qualification claim for collector 3. Its
model-profile notice says the current configuration is unqualified; the original
collector-2 qualification remains limited to its original retained inputs.
Scores/area transitions are never fed into proposal evidence. The combined
workflow uses independent current repository citations and the existing
recommendation authority contract. These trials do not qualify arbitrary repos,
real-world performance or end-to-end combined workflow inference. Follow-up
model calibration requires its own retained evidence rather than a silent retry.


### Current-diff review recovery correction

The independent medium-effort code review of `0948957f` found that an exact
committed failed suggestion child could be reported as interrupted after a crash
before parent finalization. New regressions reproduced this for both score and
suggestion failures, and rejected a suggesting phase without a child identity.
Recovery now preserves exact terminal failures with fixed stage codes. Persisted
state/phase/child combinations are validated; missing, unfinished or mismatched
children remain interrupted. The focused readiness set passes 33 tests with the
explicit live trial harness ignored. Final review and full-verifier evidence are
recorded on the PR after this correction.


### Targeted suggestion-stage trials

The existing workflow quality harness separately evaluated three positive,
three healthy and three refuted synthetic repositories with GPT-6.1-Sol low.
Independent review found both surfaced positive proposals grounded and bounded
to repairing the skipped package test script that CI delegates to, with checks
left unverified. The third positive response was rejected as `invalid_prose`
and receives zero credit; no failed trial was repaired or repeated.

All healthy and refuted trials completed with no proposal. In refuted trials the
wrapper source actually delegates to the test suite, but was not supplied to the
model: the response correctly leaves its behavior unknown rather than claiming
verification or inferring a defect. Exact fixture sources, outputs and reports
are retained in the [supplemental corpus](fixtures/readiness-suggestions-2026-10-27/index.md).
The existing harness did not retain exact prompts. Filtered invocations report a
failed whole-matrix adjudication because old-noise/scoped families and review.json
files were not supplied. These targeted outcomes (2/3 positive, 0/3 healthy,
0/3 refuted proposals) do not qualify the full matrix or general model behavior.

The prompt-v1 unsupported readiness underrating was tracked in
[#1097](https://github.com/Rambolarsen/orkworks/issues/1097); collector-3 prompt-v1 scoring
retains its unqualified status. The separately retained prompt-v2 calibration
is recorded below.

### Pending-publication review correction

Automated review found that a combined parent was terminalized while its exact
workflow child still held a recoverable pending-publication marker. An injected
final-commit and immediate-reconciliation failure reproduced the incorrect failed
parent. The parent now retains its durable child linkage while publication is
pending or its result is unreadable. Existing child recovery resolves publication
before the parent reconciles to the exact recommendation; it dispatches no new
inference. The regression also checks that the pending card is hidden, the score
is retained, and provider-call count remains four.


## Collector-3 prompt-v2 calibration (#1097)

The retained healthy-2 disagreement was adjudicated against its exact signed
rubric and supplied facts. Durable-memory level 2 requires a known, low-friction
capture action with a clear place and procedure. Fact 0 explicitly says
“Document lessons in docs/lessons.md.” Fact 6 identifies that persistent file as
the lessons destination. Together they supply both the action and destination;
a separate procedural document is not a rubric prerequisite. The empty lessons
record limits evidence for habitual capture (level 3) and curation (level 4),
not the documented procedure at level 2. Healthy-2's claim that no capture
procedure is described is contradicted by fact 0. Its original rating, rationale,
confidence, input and output remain unchanged in the failed corpus above.

The smallest calibration change is general scoring guidance in
`taskmaster-readiness-v2`: select the highest fully supported level, check the
next level against all supplied facts, and avoid adding requirements absent
from the signed definition. A documented action and destination can establish
a known procedure without establishing habitual usage, observed results or
curation. No fixture path, concept-specific rating override, rubric modification,
new evidence access or retry is introduced. Collector 3, response/discovery
schemas and scoring arithmetic retain their versions. Persisted v1 manifests
remain readable; new v2 inputs have a distinct comparison identity.


**PASS: Codex GPT-6.1-Sol low, collector 3, prompt-v2**, evaluated on
2026-10-27 against signed brain-3 and the unchanged rubric digest
`9f2c7c21b0edfa0e96fd806d9bcc50e2dd983dbf3ab2a45cf5395b0e1aea1a5e`.
The [separate calibration corpus](fixtures/readiness-calibration-2026-10-27/index.md)
retains all nine original discovery/final input/output pairs, source bytes and
hashes, per-trial results and independent semantic review. All nine two-call
trials completed without repair, retry, rejection or timeout. A source-hash
check confirms only the domain glossary changes between baseline and improved;
healthy sources are byte-identical to improved. Both the old and new corpora's
36 input/output byte hashes were verified.

Independent review checked all 126 rows against their own paired inputs:
99 rated rows supported and 27 Unassessed rows justified. Every run has 11/14
coverage and no overall number. Durable memory is 2 in all nine trials. Domain
knowledge changes 1 to 2 in 3/3 baseline/improved pairs. No completed execution,
reliability, habitual capture, curation, technical enforcement or complete-score
claim is invented. Scoping, decomposition and observability remain unassessed.

Context discovery varies: baseline/improved trials 1–2 rate 3, trial 3 rates 1;
healthy trials 1–2 rate 1 and trial 3 rates 3. The exact final inputs within each
source group match, including all improved/healthy final inputs. Review accepts
both levels as defensible readings of documented routing versus unverified
operational accuracy. This variability is explicit uncertainty, not a claimed
repository regression or effect of the glossary change. Other maturity/null
values remain stable except the intended domain gain. Confidence also varies.

The qualification notice now names only this exact profile, rubric and signed
guidance identity with unchanged discovery/final/scoring schemas and native
Codex transport. It explicitly warns that ratings can vary on unchanged
evidence. Original collector-2/prompt-v1 qualification remains separate;
collector-3/prompt-v1, different rubrics/guidance, custom transports and other
model/effort configurations remain unqualified. No live model selection was
changed. These synthetic trials do not establish general accuracy, real-world
performance, full fourteen-area scoring, combined suggestion-stage inference,
or availability in a published installer.
