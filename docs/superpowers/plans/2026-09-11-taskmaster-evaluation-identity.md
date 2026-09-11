# Taskmaster evaluation identity implementation plan

> **For agentic workers:** Use executing-plans; root is the sole writer.

**Goal:** Carry custom trust identity through cache reservation, invocation and
final recommendation acceptance without enabling custom scheduling yet.

**Architecture:** EvaluationSnapshot carries an optional captured custom identity.
The catalog retains an internal transport kind so custom capture failure cannot
downgrade to native dispatch. Cache keys include the captured identity; final
acceptance validates its binding to the supplied snapshot before holding the
existing identity guard around workspace, evidence and recommendation writes.

**Tech Stack:** Rust, serde/SHA-256, existing Taskmaster stores and fixture tests.

**Spec:** [accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md),
[ADR 0055](../../adr/0055-json-taskmaster-inference-adapters.md), issue #503.

## Constraints

- Custom availability remains execution-inactive; no real CLI calls or grants.
- No provider fallback, credential changes, or Peon configuration changes.
- Preserve live workspace instance, current evidence and dismissal checks.
- One writer; two read-only reviewers with distinct questions, one review round.
- No commits, push, PR, merge or external issue updates in this checkpoint.

## Task 1: Cache and acceptance identity

Files: `crates/orkworksd/src/taskmaster/runtime.rs`, `runtime/inference.rs`,
`evaluator.rs`, new `evaluator/identity_tests.rs`.

- [x] Add actual recommendation-store tests: approved captured output persists;
  revoke/reapprove, definition changes, corrupt trust, workspace changes and
  snapshot-setting mismatch prevent writes. Observe missing binding interface.

  ```rust
  snapshot.custom_inference = runtime.capture_custom_inference(
      &state.harness_store, workspace, &snapshot).unwrap();
  let first_key = snapshot.cache_key("same prompt").unwrap();
  // After revoke/reapprove, old output must not persist and a freshly captured
  // snapshot must produce a different key despite identical model and prompt.
  ```

- [x] Implement snapshot cache key with a versioned SHA-256 record containing
  settings, generation, prompt and optional captured cache identity.
- [x] Add `with_current_evaluation(harnesses, workspace, snapshot, apply)`:
  compare custom captured generation/settings/workspace with the snapshot;
  delegate to custom identity guard or existing native generation guard.
- [x] Replace evaluator cache construction and final commit guard. Retain all
  model/evidence validation and live workspace checks inside the guarded action.
- [x] Run focused tests, including preexisting native-output acceptance tests.

## Task 2: Preserve transport selection

Files: `taskmaster/provider_catalog.rs`, `taskmaster/evaluator.rs`.

- [x] Test that declared command adapters, including overrides of builtin IDs,
  retain an internal Custom transport kind while still execution-inactive.
- [x] Bind a selected custom snapshot before context collection; a failed or
  missing capture aborts. Dispatch captured custom invocations through the
  guarded runner; native selections retain their existing path.
- [x] Add `reserve_snapshot` after a failing test of its missing interface;
  revalidate custom trust under mutation locks through the ledger write. Extract
  the existing ledger logic into `reserve_loaded` without changing its budget,
  cache or interval rules. Verify revoked trust consumes zero reservations and
  repeated identical approved input cannot reserve twice.
- [x] Reproduce stale output clearing a current diagnostic, then route evaluator
  error updates through `record_evaluation_error` under the snapshot identity
  guard. Keep the native generation-only behavior unchanged.
- [x] Run Rust tests/build/fmt and repository verification. Review correctness
  of identity/cache/commit serialization separately from test coverage gaps.
- [x] Address verified review findings, update architecture and this plan;
  report remaining activation/native-mapping/Windows work without publishing.

## Execution and review record

The initial tests exposed missing snapshot/cache/binding/reservation interfaces.
After implementation, actual local recommendation-store tests verified approved
grounded output and rejected revoked/reapproved, changed-definition, corrupt-trust,
changed-fact, stale workspace-instance and mismatched-snapshot results.

Root reproduced a stale diagnostic bug (current error became None after revoked
successful output) and fixed all evaluator diagnostic updates to use identity
guards. Both valid and malformed stale-output cases now preserve current errors.

Correctness review found that native catalog state could become custom between
inspection and binding. A real-store test reproduced an incorrectly accepted
reservation after that change. Native transport now carries the inspected
harness-document revision; binding and each ledger/commit operation reject a
changed revision, keeping the harness lease through the operation. Native cache
keys include that revision. Full native profile mapping is still separate work.

Coverage review identified missing-custom-identity downgrade and reservation
snapshot-mismatch gaps. Removing the captured custom identity reproduced an
incorrect recommendation write; snapshots now require either the captured custom
identity or the inspected native revision. Missing-identity and mismatched
reservation tests pass. Native cache settings/generation tests were also added.

The reported cache-interval confound was rejected after checking the values:
01:00 to 03:00 is 120 minutes, exceeding the default 60-minute interval, so the
test isolates duplicate-key rejection. The supplied cache key is intentionally
an internal evaluator value, documented as derived from snapshot plus prompt,
not an external API parameter. No extra review round was requested.

All 75 focused Taskmaster tests passed after the review fixes (two ignored
helpers). Final post-fix `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited
zero: Rust tests/build/formatting, all 695 desktop tests, application/docs builds,
diff checks and documentation/worktree currency checks passed. Existing warnings
remain. Custom scheduling stays execution-inactive;
activation coverage, full native capability mapping and native Windows validation
remain. No real providers, credentials, commits or external issue updates were
involved.
