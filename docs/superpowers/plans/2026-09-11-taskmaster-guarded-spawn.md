# Taskmaster guarded spawn implementation plan

> **For agentic workers:** Use executing-plans for this sequential slice.
> Root owns edits; at most two read-only reviewers, one round before handoff.

**Goal:** Connect the captured custom identity to actual process launch without
holding mutation locks while the provider runs.

**Architecture:** The existing bounded process runner accepts a fallible spawn
callback. Custom transport exposes only guarded execution to production code;
Taskmaster supplies the callback using its captured identity. Native and Peon
execution retain the existing direct-spawn wrapper. Scheduling stays inactive.

**Tech Stack:** Rust, existing process runner, fs2 leases, temporary fixtures.

**Spec:** [accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md),
[ADR 0055](../../adr/0055-json-taskmaster-inference-adapters.md), issue #503.

## Global constraints

- Existing CLI login/config environment; no additional credentials or fallback.
- No real provider invocation, implicit approval, or changes to Peon selection.
- Revalidate through actual spawn; release locks before input/output and waiting.
- Revocation does not kill an already-running invocation; final acceptance is
  a separate guard and remains mandatory before scheduler activation.
- Root is the only writer. No commit, push, PR, or merge without explicit approval.

## Task 1: Guarded process launch

Files: `crates/orkworksd/src/providers.rs`, `providers/custom_inference.rs`,
`taskmaster/runtime/inference.rs`, `taskmaster/runtime/inference/tests.rs`.

- [x] Add local executable fixtures that write a launch marker. Capture and
  approve, then revoke before invocation; assert stale-generation error and no
  marker. Mutation caught: bypassing the guarded callback.

  ```rust
  let captured = fixture.capture();
  fixture.trust.revoke("custom", fixture.trust.generation().unwrap()).unwrap();
  let error = fixture.runtime.invoke_custom_inference(
      &fixture.harnesses, &captured, "fixture context".into()).unwrap_err();
  assert_eq!(error.code, ProviderOperationErrorCode::StaleGeneration);
  assert!(!marker.exists());
  ```

- [x] Run focused tests and observe the missing invocation interface.
- [x] Add `run_prepared_with_spawn<E>` returning `Result<ProcessOutcome, E>`;
  callback returns `Result<std::io::Result<Child>, E>`. Configure pipes/process
  group before callback; reuse existing drain/timeout/cleanup after it returns.
- [x] Add custom transport `run_with_spawn` with typed provider errors and keep
  unguarded `run` test-only. Add runtime `invoke_custom_inference` that prepares
  solely from captured fields and holds `with_current_custom_inference` through
  the callback's `Command::spawn`, reporting stale or unreadable trust safely.
- [x] Verify denied preparation cleanup and approved literal input/output.

## Task 2: Prove lock lifetime

- [x] Use a fixture process that writes its started marker and waits for a
  release file with a bounded deadline. Revoke after it starts; revocation must
  finish before release. Then release, join invocation, and verify the old
  identity cannot apply output. Mutation caught: guarding the entire run.
- [x] Preserve timeout, malformed response, spawn-failure and private-file
  cleanup tests through the new shared runner seam.
- [x] Run focused tests, Rust build/test/fmt, and repository verification.
- [x] Review: correctness verifier checks launch/lock/resource lifetime;
  coverage verifier checks missing fail-closed and native-regression cases.
  Root patches verified findings and records results. No extra review round.
- [ ] Update architecture and issue #503 with this checkpoint and remaining
  cache, result-acceptance and scheduler activation work.

## Execution record

The first focused compile demonstrated the missing runtime invocation method.
Fixture corrections supplied the required model placeholder and moved the
revocation channel sender into its scoped thread. The three guarded-spawn tests
then passed, as did all ten custom transport tests, including denial cleanup.

`RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited successfully: Rust
format/build/tests, all 695 desktop tests, desktop and docs builds, diff and
currency checks passed. Existing warnings remain; native Windows and real
provider execution were not tested.

The correctness reviewer initially repeated the previous checkpoint's harness
lock finding. Root checked the current source and requested clarification; the
reviewer confirmed the shared file lease is present and withdrew that finding.
The corrected guarded-spawn verdict has no actionable findings.

The coverage review requested an explicit harness-lock lifetime probe and exact
verification/stale error assertions. Both were added: while the child waits,
the original store and a separate store must obtain their snapshot locks before
release. All three focused spawn tests passed again after these additions.
Native Windows guarded execution remains unverified and required before release;
no additional review round was opened.

Architecture documentation is updated. Posting this checkpoint to issue #503
was blocked by the approval system because authorization to publish the
implementation details was not established. No comment was posted; owner
approval is required before retrying that external update.

Custom scheduling remains inactive. This slice does not perform usage
reservation or final recommendation acceptance; cache and acceptance integration
remain required. No credentials, real providers, commits, pushes or merges were
involved.
