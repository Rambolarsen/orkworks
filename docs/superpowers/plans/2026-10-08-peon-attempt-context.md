# Peon AttemptContext Implementation Plan

> **For agentic workers:** Execute this plan inline in the current issue #401 worktree. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace copied Peon diagnostic-attempt guards and manual finalization with one cancellation-aware `AttemptContext`, while preserving stale-attempt protection and existing Peon behavior.

**Architecture:** `AttemptContext` owns the attempt's session/runtime identity, cancellation signal, and generation-safe Drop cleanup. It exposes ordinary and already-locked currentness checks so callers preserve atomic persistence and lock ordering. A non-finalizing view supports detached blocking work, while a separate named `PeonOutputCaptureIdentity` continues to guard captured output ranges.

**Tech Stack:** Rust, Tokio `JoinSet`/`spawn_blocking`, `Arc<AtomicBool>`, existing Peon diagnostic lease map, Cargo tests, local Podman SonarQube.

**Spec:** GitHub issue #401 acceptance criteria and the approved AttemptContext design in the 2026-10-08 conversation. Product behavior remains governed by `specs/orkworks-mvp.md` (Peon observer behavior and metadata priority) and ADRs 0021/0042.

## Global Constraints

- Preserve the existing Peon inference, persistence, retry, cancellation, and scheduler behavior.
- Retain generation and runtime identity checks so stale tasks cannot mutate a replacement attempt or session.
- Retain the workspace-before-sessions lock ordering and checks performed while the sessions lock protects persistence.
- Keep output-capture generation/revision identity distinct from diagnostic-attempt identity.
- Do not change Sonar profiles, exclusions, source/test scope, or analyzer settings during comparison.
- Use `pnpm` for Node package management; this Rust-only change does not add Node dependencies.

---

## File map

- `crates/orkworksd/src/runtime/peon_runtime.rs` owns attempt scheduling, provider execution, cancellation, captured output boundaries, diagnostic attempt state, and attempt-lifecycle tests.
- `crates/orkworksd/src/session_application.rs` owns persistence of Peon inference, input labels, and workflow observations while holding the workspace/session synchronization needed to reject stale writes.
- `docs/superpowers/plans/2026-10-08-peon-attempt-context.md` records this execution plan.

## Task 1: Name the output capture identity

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: inline tests in `crates/orkworksd/src/runtime/peon_runtime.rs`

- [x] Add a failing unit test showing that a captured output identity matches only when input generation, minimum revision, runtime instance ID, and run generation still match; assert that changing each identity component rejects the capture.
- [x] Run the targeted test and confirm it fails because the named identity type/check is not implemented.
- [x] Introduce `PeonOutputCaptureIdentity` with named fields for `input_generation`, `min_revision`, `first_revision`, `last_revision`, `runtime_instance_id`, and `run_generation`. Add one method that checks the live session against all currentness fields used by the current tuple destructuring.
- [x] Replace the six-element `output_boundary` tuple and every positional destructure with the named identity. Use its named fields to construct `PeonObservationOutputRange` and to advance the capture cursor.
- [x] Run the targeted identity test and confirm it passes; run the existing Peon runtime tests.

## Task 2: Add the attempt context and Drop lifecycle

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: inline tests in `crates/orkworksd/src/runtime/peon_runtime.rs`

- [x] Add a failing test that starts attempt A, starts attempt B with a newer generation, drops A's context, and asserts B's lease, in-flight marker, generation, and diagnostics remain intact.
- [x] Add a failing cancellation test asserting a cancelled context is not current and its Drop releases its own unfinished diagnostic state.
- [x] Run both targeted tests and confirm each fails for the missing context behavior.
- [x] Add an owning `AttemptContext` containing `Arc<AppState>`, session ID, `PeonDiagnosticAttempt`, and the shared cancellation signal. Implement `is_current()` and a lock-aware currentness method that accepts the existing session handle, checking cancellation, active runtime identity, current diagnostic generation/lease, and valid attempt state.
- [x] Make `AttemptContext::drop` call the existing generation-safe `cleanup_attempt`. Preserve terminal completed/failed diagnostics; unfinished cancelled or abandoned attempts are removed as today.
- [x] Add a non-finalizing context view for the `spawn_blocking` closure. Keep the Drop owner in the `JoinSet` task so aborting the async task releases the lease immediately even when blocking work is still running; every view operation must still reject the cancelled or stale attempt.
- [x] Run the new lifecycle tests and the existing `stale_attempt_cleanup_cannot_release_a_newer_session_lease`, `shutdown_of_started_blocking_inference_releases_diagnostic_attempt_state`, and timed-out runtime replacement tests.

## Task 3: Route Peon operations through the context

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Test: inline tests in both modified Rust files

- [x] Add focused tests against the new context API proving cancellation/runtime replacement prevents input-label persistence, inference persistence, and workflow-observation writes while a current context still permits the existing successful path. Existing runtime-replacement and workflow-observation tests already pin parts of this behavior; adapt them to use the context where applicable.
- [x] Exercise cancellation at all three production persistence boundaries with an otherwise current runtime and diagnostic lease; assert the live and durable labels, inferred metadata, and workflow-observation store remain unchanged.
- [x] Run the new context tests before implementing the context and confirm they fail to compile because the requested API is absent; keep the existing behavioral tests as the regression oracle for the refactor.
- [x] Replace copied `runtime_identity_is_active`/diagnostic-attempt checks with the context's common currentness methods. Keep checks inside the existing workspace/session locks where they currently close persistence races.
- [x] Pass the context to the production `persist_peon_observation`, `persist_input_label`, and `record_peon_workflow_observations` entry points, removing their pass-through `_for_attempt` variants. Keep test-only setup paths narrowly scoped and unavailable to production callers.
- [x] Move completion, provider failure, timeout, cancellation, observation-count refresh, and side-effect admission behind context methods. Remove every manual `finish_attempt_if_active` exit call; context Drop owns lease and in-flight cleanup.
- [x] Preserve one cancellation/currentness gate before each side effect reachable from the potentially detached blocking closure. Keep evaluator scheduling conditional on an accepted observation and a current, uncancelled context.
- [x] Run the focused tests and confirm current attempts still complete/fail/timeout with the existing diagnostics, while cancelled, replaced, and stale attempts make no writes.

## Task 4: Verify lifecycle behavior and measured reduction

**Files:**
- Modify only the two Rust files above if tests expose a regression.

- [x] Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [x] Run `cargo clippy --manifest-path crates/orkworksd/Cargo.toml`; compare warnings with the recorded baseline of 0 errors and 84 warnings, including their locations.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [x] Run the lifecycle-focused review against every changed cancellation, Drop, persistence-lock, and stale-generation path; confirm existing tests still prove shutdown drain bounds, runtime replacement rejection, and lease-generation ownership.
- [x] Start the local SonarQube stack, capture `after`, and compare `.sonar/reports/baseline.json` with `.sonar/reports/after.json`. Inspect analysis warnings and compare repository and both affected-file complexity and `ncloc`; separately inspect production/test diff lines because Rust inline tests contribute to `ncloc`.
- [x] Confirm all positional six-field capture tuple destructuring and manual `finish_attempt_if_active` calls are gone, and all issue #401 acceptance criteria are met.
- [x] Complete a medium-effort manual review of the actual PR diff, address the cancellation-persistence test gap, and have the reviewer confirm the finding is resolved. The harness did not expose the literal `/code-review medium` command; the repository's requesting-code-review workflow was used.

## Verification record

- Initial clean-main Sonar baseline: repository ncloc 131156, complexity 17153, cognitive complexity 10952. After main advanced, a new clean baseline was captured at `18936e0f`; the paired scan of the exact PR source had the same fingerprint as the PR scan. Current comparison: ncloc 132126 → 132420 (+294), complexity 17545 → 17544 (-1), cognitive complexity 11241 → 11205 (-36); quality gate OK. `peon_runtime.rs`: ncloc +283, complexity +11, cognitive complexity -14; the cancellation-persistence regression test accounts for about 145 ncloc. `session_application.rs`: ncloc +11, complexity -12, cognitive complexity -22. Rust inline tests contribute to ncloc. The pre-existing source-encoding warning remains.
- `cargo fmt --check` passed. Clippy completed with 0 errors and the baseline 84 warnings. Focused input-label tests passed 5/5; AttemptContext tests passed, including the new cancellation-persistence test (1 passed).
- The final full suite run produced one unrelated native-server fixture failure (1818 passed, 1 failed, 5 ignored); isolated rerun failed earlier with `native-unavailable`. Subsequent full-suite retries were stopped by sandbox network denial for `models.opencode.ai`. Baseline full suite passed 1819 tests with 5 ignored.
- Lifecycle review found no remaining findings. The medium-effort reviewer identified a missing cancellation guard test; the new test passed and the reviewer confirmed the finding was resolved.
