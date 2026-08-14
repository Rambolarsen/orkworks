# Task 1 Report: Resume stale-handle guard

## Result

Implemented the lifecycle-consistent resume conflict predicate and endpoint regression. The guard now rejects an attached handle, or a non-ended handle unless persisted metadata is ended and no tracked PID exists. An unattached ended handle with no tracked PID is replaceable.

## Exact verification

### RED

Command:

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
```

Status: failed as expected before implementation.

Output summary:

```text
cargo test: 2 errors, 0 warnings (190 crates)
error[E0425]: cannot find function `resume_handle_conflicts` in this scope
```

### GREEN

Commands:

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_rejects
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_replaces_unattached_ended_stale_handle
```

Status: all passed.

Output:

```text
cargo test: 1 passed, 588 filtered out (1 suite, 0.00s)
cargo test: 2 passed, 587 filtered out (1 suite, 0.01s)
cargo test: 1 passed, 588 filtered out (1 suite, 0.16s)
```

Additional checks:

```text
git diff --check                 # passed
bash .claude/hooks/doc-check.sh # passed; no output
bash .claude/hooks/worktree-check.sh # passed; no output
```

`cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` was not clean because the shared worktree contains unrelated pre-existing formatting differences in many Rust files; no formatting changes were applied.

## Commit

`102932c0e75e1ebf73959f5bb07c5c89b6d98ba4` (`fix: resume ended sessions with stale handles`)

## Concerns

No behavioral concerns in the requested scope. Full Rust tests were not run; focused tests and diff checks passed. Formatting remains globally non-clean due to unrelated shared-worktree changes.
