
# Score-directed readiness qualification on 2026-10-30

**FAIL: one of three positive samples produced a useful proposal; two are required.**
Both negative samples produced no recommendation. This does not qualify the
changed combined workflow as reliably useful, and does not establish rating accuracy.
Issue [#1175](https://github.com/Rambolarsen/orkworks/issues/1175) remains open.

The owner authorized five fixed synthetic samples and at most twenty provider
calls. These samples consumed **17 calls**, on the configured Codex
`gpt-6-luna`, medium, with no retry, replacement or model substitution. A prior
selection-file preflight failed before fixture creation or inference and consumed
zero calls. No fixture command or repository modification was delegated to the model.

| Sample | Calls | Outcome | Quality credit |
| --- | --- | --- | --- |
| skipped-tests | 2 | Scoring rejected unsafe/out-of-bounds prose (`scoring_invalid_prose`); no saved score or proposal. | 0 |
| ci-omits-tests | 4 | Proposed adding the existing behavioral suite to CI, citing current CI/package/guide evidence and the signed verification concept. | 1 |
| assertions-omitted | 4 | No proposal; missed a test body containing calls with no assertions and instead challenged an unsupported test-selection concern. | 0 |
| already-satisfied | 4 | Refuted the candidate using the existing behavioral checks; no proposal. | Negative control satisfied |
| evidence-unknown | 3 | Discovery found no grounded candidate; no proposal or invented defect. | Negative control satisfied |

The CI change is bounded and practically useful: the supplied CI performs a
syntax check while an existing package test script checks catalog invariants.
The proposal includes behavior/failure-propagation validation and restoration of
the prior job as rollback. It links to verification and the saved score without
promising numeric gains. The missed assertions sample is a substantive quality
failure; transport success and an honest no-proposal envelope do not give it credit.

`inventory.json` retains byte counts and SHA-256 hashes. Score-stage inputs and every provider output preserve original bytes.
Suggestion-stage inputs preserve the complete application JSON as lossless
pretty-printed objects; their inventory hashes identify that retained encoding,
not the compact transport encoding. `*-sources.json` preserves synthetic source
text, and `*-outcome.json` preserves score/report/parent linkage. The corpus predates
the final metadata-only catalog-read correction; that correction is covered by
deterministic regressions, and these samples have not been repeated. Fresh independent adjudication (`review.json`) agrees with the table and
confirms the failed gate. The review did not repeat provider calls or tests.

The implementation and its deterministic safety checks remain reviewable in
[PR #1178](https://github.com/Rambolarsen/orkworks/pull/1178). Keep it draft until
the usefulness failure has an explicit disposition. A failed model sample is not
repaired or silently discarded to claim a useful release.
