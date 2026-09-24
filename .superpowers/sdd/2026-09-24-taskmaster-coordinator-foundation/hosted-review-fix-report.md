# Hosted review fix wave — 2026-09-24

- Bound activation to the caller's live revocation generation and store records to the expected instance/workspace on proposal, read, and reopen.
- Reflush existing plan directories during recovery; identical activation and transition retries return the published record after recovery. A deterministic test fault verifies the first call errors after publication and the retry succeeds.
- Added renewed-approval resume for paused plans while keeping generic transitions to Active closed. Rejected uppercase plan IDs at the domain and store boundaries.
- TDD regressions were observed failing before the fixes. Final focused coordinator tests: 48 passed. `cargo build`, `cargo fmt --check`, and `git diff --check` passed.
- Full sidecar test run: 1,358 passed, 2 failed, 3 ignored. The failures were `providers::tests::model_discovery_runs_a_declared_command_without_arguments` (1-second model-list timeout) and `providers::tests::process_runner_cleans_up_provider_that_closes_stdin_during_prompt_write` (timeout instead of expected broken-pipe error). Each passed on an individual rerun.
