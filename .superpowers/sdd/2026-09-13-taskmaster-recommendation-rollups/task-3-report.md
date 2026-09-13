# Task 3 report: recommendation schema and pure rollup validation

## Status

Complete. Task 3 adds the defaulted recommendation/evidence schema fields and
the pure bounded rollup snapshot, validation, projection, and stable-ID
helpers. Exact-family evaluator behavior remains unchanged; the existing
constructor sites only received the new default values required to compile.

## Files

- `crates/orkworksd/src/taskmaster/mod.rs`
- `crates/orkworksd/src/taskmaster/rollup.rs`
- `crates/orkworksd/src/taskmaster/evaluator.rs` (compile-fix constructor)
- `crates/orkworksd/src/taskmaster/store.rs` (compile-fix test constructor)
- `.superpowers/sdd/2026-09-13-taskmaster-recommendation-rollups/task-3-report.md`

## Verification

RED evidence:

- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::rollup`
  — failed to compile with 41 errors because the rollup module, status, and
  schema fields were absent.

Green verification:

- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::rollup`
  — 12 passed, 1,184 filtered out.
- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::`
  — 100 passed, 2 ignored, 1,094 filtered out.
- `rtk rustfmt --edition 2021 crates/orkworksd/src/taskmaster/rollup.rs` — passed.
- `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` — passed.
- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml` — 1,193 passed,
  3 ignored in 71.08s.
- `rtk git diff --check` — passed.

## Concerns

- Persistence transactions, graph recovery, model scheduling, API projection,
  and desktop behavior remain intentionally deferred to Tasks 4–6.
- The sidecar retains its existing unrelated compiler warnings; no new warning
  affects the Task 3 implementation.
- The full sidecar run was unrelated to Task 3 and had two pre-existing,
  environment-sensitive provider failures. Provider code was not changed:
  `providers::inference::tests::profiles_preserve_auth_and_never_reuse_interactive_arguments`
  panicked with `login environment changed`, and
  `providers::tests::process_runner_cleans_up_provider_that_closes_stdin_during_prompt_write`
  timed out instead of observing the expected broken-pipe error.

## Fix round 1

The validated cluster list is now sorted by its sorted member-ID key, making
model cluster-array order irrelevant. Focused coverage now explicitly checks
reversed cluster order, oversized serialized input and response, excessive
supplied families and observations, duplicate cluster sets, and per-family
representative/source-session bounds.

Verification for this fix:

- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::rollup`
  — 19 passed, 1,184 filtered out.
- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::`
  — 107 passed, 2 ignored, 1,094 filtered out.
- `rtk rustfmt --edition 2021 crates/orkworksd/src/taskmaster/rollup.rs` — passed.
- `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` — passed.
- `rtk git diff --check` — passed.
- `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml` — 1,195 passed,
  2 failed, 3 ignored; the two failures are the unrelated provider tests
  listed above.
