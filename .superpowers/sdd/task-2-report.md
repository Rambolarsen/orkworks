# Task 2 report: Rust session application seam

## Changed files

- `crates/orkworksd/src/session_application.rs` — added `SessionApplication`, application command/value types, stable application errors, workspace reconciliation implementation, and application operation adapters.
- `crates/orkworksd/src/main.rs` — registered the new module.
- `crates/orkworksd/src/http/session_handlers.rs` — routed workspace, create, resume, attention, plan-selection, delete, and forget handlers through the application seam while preserving request extraction, authorization, response mapping, and existing compatibility implementations.

## Exact interface

```rust
pub(crate) struct SessionApplication { state: Arc<AppState> }

pub(crate) fn new(state: Arc<AppState>) -> Self;
pub(crate) fn open_workspace(PathBuf) -> Result<WorkspaceSnapshot, SessionError>;
pub(crate) async fn create_session(CreateSessionCommand) -> Result<SessionSnapshot, SessionError>;
pub(crate) async fn resume_session(&str) -> Result<SessionSnapshot, SessionError>;
pub(crate) async fn report_attention(&str, AttentionSignal) -> Result<(), SessionError>;
pub(crate) async fn select_plan(&str, PlanSelection) -> Result<SessionSnapshot, SessionError>;
pub(crate) async fn delete_session(&str, bool) -> Result<SessionSnapshot, SessionError>;
```

`SessionSnapshot` is the existing Axum `Response`, preserving the exact legacy response body/status for operations whose compatibility implementation is still in the handler module. `WorkspaceSnapshot` contains `path`, `repo_root`, `branch`, `dirty`, `last_active_session_id`, and `active_harness_ids`. `SessionError` contains `BadRequest`, `Conflict`, `NotFound`, and `Internal` variants.

## TDD evidence

- RED: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::opening_a_workspace_returns_its_application_snapshot` failed because `SessionApplication` did not exist (`cannot find type SessionApplication`).
- GREEN: the same focused command passed after adding the module and moving workspace reconciliation behind `open_workspace`.
- Subsequent full test run passed with 648 tests and 0 failures.

## Verification commands and results

- `cargo test --manifest-path crates/orkworksd/Cargo.toml` — PASS, 648 passed, 0 failed.
- `cargo fmt --all -- --check` — BLOCKED at repository root because no root `Cargo.toml` exists.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --all -- --check` — FAIL due pre-existing formatting drift across many untouched files; the output included files under `harness/`, `metadata.rs`, runtime modules, and `session_view.rs`.
- `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings` — FAIL due the existing repository lint backlog, including pre-existing dead code, type-complexity, argument-count, identity-op, and runtime conversion warnings. New seam import/closure warnings were removed before the final run.
- `git diff --check` — PASS.
- `bash .claude/hooks/doc-check.sh` — PASS/silent.
- `bash .claude/hooks/worktree-check.sh` — PASS/silent.

## Commit

`9f7388a` — `refactor: deepen session application module`

## Concerns

- The new application methods for create/resume/attention/plan/delete/forget currently delegate to renamed compatibility implementations in `http::session_handlers`; this preserves behavior and wire compatibility, but is an intermediate seam rather than a complete relocation of lifecycle logic into the application module.
- `SessionSnapshot` is an Axum response rather than a domain snapshot, because extracting and serializing the existing large create/resume implementations in this task would risk changing response bodies, status codes, startup ordering, or resume compensation behavior.
- Strict fmt and clippy gates remain red for baseline repository issues; no unrelated baseline files were changed to clear them.
