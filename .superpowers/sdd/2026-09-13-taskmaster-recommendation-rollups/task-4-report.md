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

## Fix round 1

### RED evidence

Added focused tests for the review findings before completing the fixes:

- terminal parent IDs cannot be reused, while proposed parents may be updated;
- missing members, parent/member collisions, nested members, and malformed
  graphs are rejected before or during publication;
- staging, manifest commit, partial publication, and cleanup crash points
  recover to a complete old or complete new graph; and
- accepted, completed, dismissed, superseded, expired, and failed terminal
  predecessors preserve lineage by starting a new exact-family generation.

The first focused run intentionally failed with three behavioral failures:
the collision assertion observed the wrong error ordering, the accepted
predecessor test still expected terminal overwrite, and the dismissed
predecessor test lacked the required new-evidence watermark. Those tests and
the implementation were corrected. The final compile-only failure was caused
by the test moving `existing` in `Some(existing)` before cloning it for the
nested-member case; changing that assertion to `Some(existing.clone())`
resolved the ownership error without changing production semantics.

### GREEN evidence

The store now rejects terminal-parent reuse, requires existing member IDs,
rejects ID collisions and nested/self-referential graph inputs, validates the
published graph before manifest cleanup, and rolls back malformed results.
Manifest publication uses a fsynced temporary marker and atomic replacement,
with transaction-directory and parent-directory synchronization before
publication. Fault-injection coverage exercises staging, manifest commit,
publication, and cleanup; startup recovery exposes a complete old or new
graph.

Taskmaster now preserves terminal predecessors and creates a new exact-family
generation with supersession lineage for terminal evidence, including the
required `rollup_generation` value.

The public method signature remains:

```rust
pub(crate) fn apply_rollup_transaction(
    &self,
    expected: &BTreeMap<String, Option<String>>,
    parent: &Recommendation,
    members: &[Recommendation],
) -> Result<(), StoreError>
```

### Files

- `crates/orkworksd/src/taskmaster/store.rs` — review fixes, crash-safe
  manifest publication, graph validation, fault seam, and tests.
- `crates/orkworksd/src/taskmaster/mod.rs` — terminal evidence lineage and
  Taskmaster tests.
- `.superpowers/sdd/2026-09-13-taskmaster-recommendation-rollups/task-4-report.md`
  — appended fix-round evidence.

### Commands and results

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster
```

Result: 140 passed, 2 ignored, 1,074 filtered out; 0 failed.

```text
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

Result: passed.

```text
rtk git diff --check
```

Result: passed.

The full sidecar suite was not run, per instruction; therefore no unrelated
full-suite failures were observed or classified.

### Concerns

- Focused compilation still emits the repository's existing warnings; no
  warning was treated as a test failure.
- The crash-point seam is test-only and does not expand the public store API.
