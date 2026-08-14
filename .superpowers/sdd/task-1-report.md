# Task 1 Report: Resume stale-handle guard

## Result

Fixed the review regression in the resume conflict predicate. The guard now rejects whenever a terminal is attached, persisted metadata is not ended, or a PTY PID is tracked. Replacement is allowed only for an unattached handle with ended metadata and no tracked PID.

## Exact verification

### RED

Command:

```text
cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
```

Status: failed as expected before implementation.

Output summary:

```text
cargo test: 1 failed, 0 passed, 588 filtered out
assertion failed: resume_handle_conflicts(&handle, false, false)
```

### GREEN

Commands:

```text
cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_rejects
cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_replaces_unattached_ended_stale_handle
```

Status: all passed.

Output:

```text
cargo test: 1 passed, 0 failed, 588 filtered out
cargo test: 2 passed, 0 failed, 587 filtered out
cargo test: 1 passed, 0 failed, 588 filtered out
```

Additional checks:

```text
git diff --check                 # passed
bash .claude/hooks/doc-check.sh # passed; no output
bash .claude/hooks/worktree-check.sh # passed; no output
```

`cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` was not clean because the shared worktree contains unrelated pre-existing formatting differences in many Rust files; no formatting changes were applied.

## Commit

`95aea78` (`fix: reject resume when persisted session is live`)

Follow-up test-only fix: `7408a7e` (`test: cover tracked pid resume conflict`). Added an independent assertion for an unattached ended handle with ended metadata and a tracked PID. A temporary restoration of the prior lifecycle-gated predicate produced the expected focused-test failure; the strict predicate was restored before the green run.

Final test-only fix: `7b3b81b` (`test: cover attached resume conflict`). Added an independent assertion that a terminal-attached handle conflicts even with ended metadata and no tracked PID. Focused predicate/rejection/replacement tests remained green; no production predicate changes were made.

## Concerns

No behavioral concerns in the requested scope. Full Rust tests were not run; focused tests passed. Existing compiler warnings remain unrelated to this fix.
