# Task 5 report

## Implementation

- Added the bounded Taskmaster semantic rollup request, server-owned evaluation token, combined-response parser, stale/provider/workspace revalidation, and transactional application path.
- Preserved exact recommendations on unavailable, malformed, invalid, stale, or failed rollup paths.
- Fixed the successful rollup application path to return success after `apply_rollup_transaction` completes.
- Fixed changed-set reparenting so members of a superseded proposed parent can be assigned to the successor while omitted members are released by the existing store transaction helper.
- Extended store ID validation for the authoritative `rollup:<64 lowercase-or-uppercase hex characters>` format without permitting path separators or other unsafe filename characters.
- Added a store regression test proving stable rollup IDs survive transactional persistence and reload, plus evaluator coverage for bounded prompts, untrusted-data labeling, combined response parsing, invalid/overlapping output, stale tokens, unavailable selection, same-set updates, changed-set release, and idempotence.

## Files

- `crates/orkworksd/src/taskmaster/evaluator.rs`
- `crates/orkworksd/src/session_application.rs`
- `crates/orkworksd/src/taskmaster/runtime.rs`
- `crates/orkworksd/src/taskmaster/store.rs`
- `crates/orkworksd/src/taskmaster/evaluator/rollup_tests.rs`

## Verification

- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator`: **24 passed**.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator::rollup_tests`: **6 passed**.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store`: **24 passed**.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`: passed.
- `git diff --check`: passed.

## Fix round 2

Addressed the second review pass:

- combined legacy and rollup output is validated as one atomic response, including legacy citations, before either section can mutate state;
- native rollup revalidation now compares the live effective provider/model and generation under the runtime persistence lock;
- pre-existing POSIX `rollup:<digest>.json` files migrate to the encoded cross-platform filename on store open;
- executing rollup parents retain their lifecycle state and cannot be replaced by a changed member set.

Verification:

- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator`: 27 passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store`: 25 passed.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml`: passed.
- `git diff --check`: passed.

An initial sandboxed full sidecar run completed with 1,208 passed, 10 unrelated environment/pre-existing failures, and 3 ignored tests. A rerun with normal filesystem access was started to distinguish those failures, but was interrupted before its final summary by the user. No Task 5-focused test failure remained.

## Commit

`feat: evaluate bounded Taskmaster recommendation rollups`

## Fix round 1

Addressed review findings:

- logical `rollup:<64-hex>` IDs now use platform-safe encoded filenames while
  preserving the logical ID in JSON, lookups, and transaction manifests;
- legacy enrichments/proposals and rollups now share one strict response
  envelope, with whole-response validation before either section mutates state;
- multiple clusters are assembled into one complete graph and published by one
  recoverable transaction;
- rollup application reuses the existing custom-inference and native-harness
  identity guards, including trust/capability and document-revision checks.

Verification:

- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator::rollup_tests`: 9 passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator`: 27 passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store`: 25 passed.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`: passed.
- `git diff --check`: passed.
