# Task 2B report: Rust create/resume lifecycle slice

## Changed files

- `crates/orkworksd/src/session_application.rs` — create/resume workflows consume `CreateSessionCommand`; resume invalid-request failures use a typed empty-body error; added application-seam admission-conflict and startup-failure/rollback coverage while retaining the create persistence test.
- `crates/orkworksd/src/http/session_handlers.rs` — HTTP deserialization constructs only `CreateSessionCommand`, maps the typed empty 400, updates launch-resolution tests, and verifies the empty resume response body.

## Exact interface

```rust
pub(crate) async fn create_session(&self, command: CreateSessionCommand)
    -> Result<SessionInfo, SessionError>;
pub(crate) async fn resume_session(&self, id: &str)
    -> Result<SessionInfo, SessionError>;
```

`resolve_session_launch` also accepts `&CreateSessionCommand`; no create/resume application code imports `CreateSessionRequest`. `SessionError::EmptyBadRequest` preserves the existing empty-body 400 for missing resume metadata, unavailable strategy, and failed command construction. Attention, plan, delete, and forget workflows were not moved or changed.

## TDD evidence

- RED: the new seam assertions initially failed to compile because `Result<SessionInfo, SessionError>` could not be compared without `SessionInfo: PartialEq`; the test was corrected to match only the typed error variant.
- GREEN: `CARGO_TARGET_DIR=/tmp/orkworks-task-2b-fix-target cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests:: -- --nocapture` — 5 passed.
- GREEN: focused resume handler tests — 11 passed, including startup failure and empty-body mapping.

## Verification

- Full Rust suite: `CARGO_TARGET_DIR=/tmp/orkworks-task-2b-fix-target cargo test --manifest-path crates/orkworksd/Cargo.toml --quiet` — 649 passed, 3 failed in pre-existing route/harness tests with `Operation not permitted` while creating their test server; no create/resume extraction test failed.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --all -- --check` — baseline failure across untouched files.
- `CARGO_TARGET_DIR=/tmp/orkworks-task-2b-fix-target cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings` — baseline failure with 34 errors across untouched metadata, harness, HTTP, runtime, session-view, peon, providers, and watcher code.
- `git diff --check` — PASS.
- `bash .claude/hooks/doc-check.sh` — PASS/silent.
- `bash .claude/hooks/worktree-check.sh` — PASS/silent.

## Concerns

- Full-suite permission failures, manifest formatting drift, and clippy backlog are baseline/environment issues separated from this slice.

## Commit

`4766187`
