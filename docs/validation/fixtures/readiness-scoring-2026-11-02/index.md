# Collector-4 prompt-v4 readiness scoring qualification

**FAIL: successful transport and storage do not establish supported maturity ratings.**
Tracks [#1285](https://github.com/Rambolarsen/orkworks/issues/1285).

The owner approved nine fixed standalone scoring trials and at most eighteen
logical provider calls on 2026-11-02. The existing harness ran baseline,
improved and healthy three times each at code head
`11ef66fe93c9db1a0212378011e8841982546c2e` (the exact head is recorded in
`experiment.json`). Codex `gpt-6-luna`, medium reasoning effort, used collector 4,
discovery schema 2, final schema 1, scoring prompt v4 and scoring schema 1 with
signed brain-3 guidance. Exact rubric, guidance and comparison identities are
retained with every dispatch.

All nine trials completed and saved snapshots using **18 calls**, two per trial,
with no application-level sample retries, replacement samples or profile changes.
Underlying provider network retries were not traced. A configuration
preflight failed before creating trials or invoking inference because its input
lacked required settings fields. Correcting that input consumed zero calls and
did not retry a model sample. The full live budget is exhausted.

| Trial | Assessed areas | Stored overall score | Domain maturity |
| --- | --- | --- | --- |
| baseline-1 | 12/14 | None | 1 |
| baseline-2 | 12/14 | None | 1 |
| baseline-3 | 11/14 | None | 1 |
| improved-1 | 11/14 | None | 2 |
| improved-2 | 14/14 | 36 | 2 |
| improved-3 | 11/14 | None | 2 |
| healthy-1 | 14/14 | 50 | 3 |
| healthy-2 | 14/14 | 52 | 3 |
| healthy-3 | 12/14 | None | 2 |

Stored scores are retained observations, not endorsed results. Observed domain
maturity changes from 1 to 2 in all three baseline/improved pairs. Only `docs/domain.md` differs between the source repositories. However,
only pair 1 supplies the same final fact set except for that glossary. Pair 2
swaps test evidence for implementation evidence; pair 3 adds implementation
evidence. The result is one isolated final-evidence comparison, not three.
All improved and healthy source bytes are identical, yet their ratings and assessed coverage
vary. Improved-1, improved-3 and healthy-1 have byte-identical final inputs;
domain maturity still changes from 2 to 3. Healthy-2 and healthy-3 also have
byte-identical final inputs. This variation is model judgment, not a repository
improvement or regression.

Independent semantic adjudication covers all 126 concept rows against their own
exact final input/output pair. It identifies unsupported claims of clean-setup
reliability, ongoing architecture/domain maintenance, guaranteed agent-tool
availability and documented task decomposition. For example, healthy-1 and
healthy-2 award domain level 3 without evidence of updates alongside domain
changes. Improved-2 awards task-decomposition level 2 from a general small-plan
requirement without an explicit decomposition workflow. These findings prevent
qualification.

`review.json` retains the independent row-level adjudication. Separate correctness
and completeness checks and the owning review disposition are retained alongside
it. Read `review-disposition.json` for the owning resolution: it corrects
reviewer labels, observed-score claims, and evidence-control overstatements, and
preserves the disputed policy thresholds. Initial reviewer category counts are
not accepted as final error counts. No correction rewrites model responses.
The failure is limited to this fixed scoring configuration and supplied evidence;
it does not revoke the separate [evidence-first suggestion usefulness pass](../score-directed-investigation-2026-10-31/index.md),
qualify other profiles, establish general accuracy, or describe an installer.
No fixture command was executed by the assessment and live settings were unchanged.

Original discovery/final input and output bytes, dispatch identities and persisted
outcomes are retained under each trial prefix. `sources.json` files retain exact
synthetic source text. `experiment.json` records approval, scope and code/source
hashes; `source-check.json` and `matrix.json` record source equality and observed
ratings. `inventory.json` records retained file byte counts and SHA-256 hashes.
No prior failed or qualified corpus is overwritten or regraded.

The exact `run.log` bytes are stored losslessly as the UTF-8 `text` string in
`run-log.json`, with their original byte count and hash. Reviewer references to
`trials/<trial>/<file>` map to the retained flat `<trial>-<file>` names.
