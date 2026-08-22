# Task 2B report: Rust create/resume lifecycle slice

## Changed files

- `crates/orkworksd/src/session_application.rs` — owns the create/resume workflows, launch resolution, resume admission, generation-aware rollback, startup handling, and focused seam tests.
- `crates/orkworksd/src/http/session_handlers.rs` — create/resume handlers deserialize/extract, call `SessionApplication`, and map typed results/errors to the existing wire responses. No create/resume workflow implementation remains here.

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

Interface remains `create_session(&self, CreateSessionCommand) -> Result<SessionInfo, SessionError>` and `resume_session(&self, &str) -> Result<SessionInfo, SessionError>`. Create preserves the pre-spawn `creating` result and persistence ordering. Resume preserves generation-aware admission, awaited startup, rollback, and `Internal` mapping for startup failure. HTTP serialization remains `Json<SessionInfo>` with the existing status/body mapping. `AppState.sessions` remains the sole runtime registry.

## TDD evidence

- RED: focused seam tests failed because the application methods returned Axum `Response` and the no-workspace test could not observe `SessionError::Conflict`.
- GREEN: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::` — 3 passed.
- Startup rollback GREEN: `cargo test --manifest-path crates/orkworksd/Cargo.toml 'http::session_handlers::tests::resume_session_startup_failure_eventually_clears_runtime_claim'` — 1 passed.

## Verification

- `cargo test --manifest-path crates/orkworksd/Cargo.toml` — 649 passed, 1 failed. The failure is the existing/flaky `runtime::session_runtime::tests::session_exit_persists_and_replays_unterminated_output_suffix` timeout; all create/resume extraction and rollback tests passed.
- `git diff --check` — PASS.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --all -- --check` — FAIL on pre-existing formatting drift across untouched files.
- `CARGO_TARGET_DIR=/tmp/orkworks-task-2b-target cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings` — FAIL on the existing repository lint backlog (34 errors across untouched metadata, harness, HTTP, runtime, session-view, and watcher code); no extraction-specific lint error remained.
- `bash .claude/hooks/doc-check.sh` — PASS/silent.
- `bash .claude/hooks/worktree-check.sh` — PASS/silent.

## Concerns

- Manifest fmt and clippy remain blocked by baseline repository drift noted above.
- The full suite has one unrelated timeout failure noted above; the focused create/resume and startup rollback tests pass.
