# Score-directed prompt-v4 qualification on 2026-10-31

**FAIL: one of three positive samples earns independent usefulness credit; two
are required.** The owner authorized this separate five-sample experiment after
the [previous failed set](../score-directed-readiness-2026-10-30/index.md).

The fixed set used Codex `gpt-6-luna`, medium, scoring prompt v4 and focus prompt
v2, source-code context and bundled signed starter guidance at code head `2dd887968669f56733473c289d725d75414a81c0`. It consumed **16
logical provider calls**, with no retries, replacement samples or model changes.
Unused budget is not permission to replace failed samples. No fixture command
was executed. The earlier failed corpus is unchanged.

| Sample | Calls | Observed result | Independent disposition |
| --- | --- | --- | --- |
| skipped-tests | 4 | Recommendation created | No credit: unclear wiring change and possible duplication of existing assertions. |
| ci-omits-tests | 4 | Recommendation created | Credit: connect the existing behavioral suite to syntax-only CI. |
| assertions-omitted | 3 | No candidate / no proposal | No credit: missed the positive source defect. |
| already-satisfied | 3 | No candidate / no proposal | Negative control passed. |
| evidence-unknown | 2 | Scoring rejected prose | Invalid control; no valid saved score or suggestion. |

The fresh independent adjudication is retained in `review.json`. Transport
success and a stored recommendation do not establish useful advice. The skipped
script is a real problem, but its proposal asks for focused assertions already
present in the test file instead of explicitly connecting the existing suite.
That ambiguity receives no credit. The CI proposal is independently useful and
bounded, with signed verification and current repository citations, validation,
and rollback. Its exact invocation still requires confirming pnpm availability.

Score inputs and every provider output preserve original bytes. Suggestion
inputs preserve complete application JSON in a lossless pretty-printed encoding;
inventory hashes identify the retained encoding. Sources, outcomes and byte
hashes are retained alongside the stage files. Failures are not repaired or
reclassified by later offline codec regression tests.

PR #1178 remains draft and issue #1175 remains open. This set does not establish
that usable analysis is solved. The missing-candidate case indicates that
requiring a positive candidate before gathering implementation evidence can stop
investigation too early. Any change to that protocol needs a concrete reviewed
design; another prompt-only trial is not authorized by this exhausted fixed set.
