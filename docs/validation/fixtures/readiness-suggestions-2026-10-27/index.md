# Readiness collector-3 fixture evidence

Nine independent, uncached Codex GPT-6.1-Sol low-effort trials from 2026-10-27,
covering baseline, controlled domain-documentation improvement, and unchanged
healthy repositories. Each has exact discovery/final inputs and model outputs.
`trials.json` records their byte hashes, selection, coverage and component ratings.
Fixture source construction is in `taskmaster/readiness_quality_tests.rs`.

All nine calls pairs completed without repair/retry. Every snapshot has 11/14
coverage and no overall score. Domain knowledge changes 1 to 2 in all three
baseline/improved pairs. Healthy-2 rates durable memory 1 rather than 2; this is
retained rather than hidden. Independent semantic review **failed qualification** on that unsupported
underrating. Its full scope and disposition are recorded in the [runtime validation](../../repository-readiness-runtime.md).
These synthetic artifacts contain no live workspace evidence or credentials.


`workflow-trials.json` separately retains exact source bytes and provider outputs,
reports and hashes for targeted positive, healthy and refuted suggestion-stage
trials. Two of three positive proposals were grounded; the third was rejected
as invalid prose and remains zero credit. All three healthy and all three refuted
cases returned no proposal. The wrapper file was not supplied in the refuted
trials, so no delegation verification is credited. The old harness did not save
exact prompts; these trials cannot establish complete matrix qualification.
