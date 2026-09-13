# Task 4 report: recoverable parent/member persistence

## RED evidence

Added store tests before implementation for:

- complete parent/member transitions and transaction-file exclusion;
- stale expected-file hashes;
- uncommitted and committed manifest recovery;
- graph invariant rejection;
- changed-parent supersession and member release; and
- graph-aware session cleanup.

Command:

```text
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store
```

Result: expected compilation failure. The new tests reported the missing
`apply_rollup_transaction` method and the not-yet-defined
`StaleExpectedHash` and `GraphInvariant` store errors.

## GREEN evidence

Implemented the recoverable transaction manifest, staged replacements,
durable backups, expected SHA-256 checks, startup/read recovery, graph
validation, transaction-directory exclusion, changed-parent lifecycle, and
whole-graph session cleanup.

Chosen signature:

```rust
pub(crate) fn apply_rollup_transaction(
    &self,
    expected: &BTreeMap<String, Option<String>>,
    parent: &Recommendation,
    members: &[Recommendation],
) -> Result<(), StoreError>
```

`Some(old_sha256)` requires the current recommendation JSON bytes to hash to
that value; `None` requires the file to be absent. Expected hashes are
checked before transaction staging. Inputs remain immutable; the store
persists cloned members with `rolled_up` and `rolled_up_by` set atomically
with the parent.

Command:

```text
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store
```

Result: 19 tests passed, 0 failed. The full sidecar suite was intentionally
not run at the user's request.

## Files

- `crates/orkworksd/src/taskmaster/store.rs` — transaction persistence,
  recovery, graph validation, lifecycle, cleanup, and focused tests.
- `.superpowers/sdd/2026-09-13-taskmaster-recommendation-rollups/task-4-report.md`
  — this report.

`crates/orkworksd/src/taskmaster/mod.rs` was unchanged because Tasks 1–3 had
already supplied the required schema fields and `RolledUp` status.

## Final checks

```text
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

Result: passed after formatting only `store.rs` with rustfmt.

```text
git diff --check
```

Result: passed.

## Concerns

- The requested full sidecar test suite was not run, so unrelated crate-wide
  regressions remain outside this report's evidence.
- Existing crate warnings were present during the focused test compilation;
  they were unrelated to the Task 4 files.
- The store preserves existing `put` and single-record lifecycle behavior;
  integration of evaluator rollup application and API filtering remains for
  later tasks.
