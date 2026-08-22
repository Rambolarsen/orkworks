# Task 2B report: Rust create/resume lifecycle slice

## Changed files

- `crates/orkworksd/src/session_application.rs` — create/resume now expose typed `SessionInfo` application results and focused seam tests.
- `crates/orkworksd/src/http/session_handlers.rs` — create/resume workflows return typed results; HTTP handlers own JSON/status mapping; legacy create/resume entry points were removed.

## Exact interface

```rust
pub(crate) async fn create_session(
    &self,
    request: CreateSessionCommand,
) -> Result<SessionInfo, SessionError>;

pub(crate) async fn resume_session(
    &self,
    id: &str,
) -> Result<SessionInfo, SessionError>;
```

Create preserves the pre-spawn `creating` result and persistence ordering. Resume preserves generation-aware admission, awaited startup, rollback, and `Internal` mapping for startup failure. HTTP serialization remains `Json<SessionInfo>` with the existing status/body mapping.

## TDD evidence

- RED: focused seam tests failed because the application methods returned Axum `Response` and the no-workspace test could not observe `SessionError::Conflict`.
- GREEN: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::` — 3 passed.
- Existing startup rollback coverage: `cargo test --manifest-path crates/orkworksd/Cargo.toml 'http::session_handlers::tests::resume_session_startup_failure'` — 1 passed.

## Verification

- `cargo test --manifest-path crates/orkworksd/Cargo.toml` — PASS, 650 passed, 0 failed.
- `git diff --check` — PASS.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --all -- --check` — FAIL on pre-existing formatting drift across untouched files.
- `CARGO_TARGET_DIR=/tmp/orkworks-task-2b-target cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings` — FAIL on the existing repository lint backlog (34 errors across untouched metadata, harness, HTTP, runtime, session-view, and watcher code).
- `bash .claude/hooks/doc-check.sh` — PASS/silent.
- `bash .claude/hooks/worktree-check.sh` — PASS/silent.

## Concerns

- The lifecycle bodies remain in `http/session_handlers.rs` as non-HTTP `*_workflow` helpers in this bounded slice; the application-facing interface and HTTP mapping are typed, while a later slice can relocate the implementation without changing the contract.
