# Evidence-first readiness qualification on 2026-10-31

**PASS: all three positive samples earn independent usefulness credit (two
required); both negative controls finish valid scoring and final no-proposal.**
The owner approved this separate fixed five-sample trial of accepted
[ADR 0099](../../../adr/0099-readiness-evidence-before-candidates.md).

The set used Codex `gpt-6-luna`, medium, scoring prompt v4, focus prompt v3 and
investigation-v1 at code head `4cd953f9d7d3ee80e0661c203c22082e314f9554`.
It consumed **20 logical provider calls**, four per sample, with no retries,
replacement samples or model changes. No fixture command was executed.

| Sample | Calls | Observed result | Independent disposition |
| --- | --- | --- | --- |
| skipped-tests | 4 | Recommendation created | Credit: reconnect the existing asserted suite instead of duplicating it. |
| ci-omits-tests | 4 | Recommendation created | Credit: run the existing behavioral suite in syntax-only CI. |
| assertions-omitted | 4 | Recommendation created | Credit: add outcome assertions to the complete visible test body that lacks them. |
| already-satisfied | 4 | Valid score and final no-proposal | Control passed: existing checks and wiring already cover the evidenced need. |
| evidence-unknown | 4 | Valid score and final no-proposal | Control passed: missing context proves no defect or capability. |

The fresh independent adjudication is retained in `review.json`. All positive
proposals use current repository facts and selected signed capability guidance,
identify distinct bounded changes, and include validation and rollback without
promising score gains. Existing assertions are reused where the gap is wiring.
The proposed checks remain unverified; producing a recommendation does not prove
that a command was executed or that CI failure propagation works.

Score inputs and every output preserve original bytes. Suggestion inputs retain
complete lossless pretty-printed application JSON, not original compact transport
bytes. Sources, outcomes, profile/protocol identities and per-sample call counts
are retained. `inventory.json` hashes each retained evidence file;
`suggestion-discovery` files contain investigation-v1, not standalone discovery.

The [initial failed set](../score-directed-readiness-2026-10-30/index.md) and
[failed prompt-v4 set](../score-directed-readiness-v4-2026-10-31/index.md) remain
unchanged and receive no retrospective credit. This experiment qualifies its
fixed synthetic usefulness gate, not general scoring accuracy, all repositories
or a published installer. PR #1178 still needs its applicable integration gates.
