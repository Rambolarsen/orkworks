# Session Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move session-listing policy and state write-back behind a deep `SessionProjection` module while preserving the existing `SessionInfo` wire contract and runtime behavior.

**Architecture:** Add `crates/orkworksd/src/session_projection.rs` with one crate-visible `SessionProjection::list() -> Vec<SessionInfo>` operation. It borrows the existing `Arc<AppState>`, uses a shared `projection_lock` to serialize listings and workspace replacement, performs blocking reads/probes without holding state locks, and owns capacity/provider write-back. `list_sessions` becomes an async HTTP adapter that only orchestrates `spawn_blocking`, maps join failures to HTTP 500, and serializes JSON.

**Tech Stack:** Rust, Tokio, Axum, `std::sync::Mutex`, existing `MetadataStore`, `SessionHandle`, `ProviderManager`, `session_view`, `git`, `procfs`, and Rust unit/HTTP tests.

## Global Constraints

- Preserve `AppState.sessions` as the sole live-session registry.
- Preserve `WorkspaceState.metadata` as the persisted session source of truth.
- Preserve the exact `SessionInfo` JSON fields, lifecycle vocabulary, and response ordering behavior.
- Use the lock order: projection lock, then workspace or sessions lock, then provider-manager internal locks.
- Never hold `state.sessions` or `state.workspace` while performing filesystem, process-cwd, or Git work.
- Recoverable metadata/Git/cwd failures degrade as today; a blocking-task `JoinError` returns HTTP 500 with an empty body.
- Run `cargo test --manifest-path crates/orkworksd/Cargo.toml` for Rust verification.
- Update Rust module-layout documentation when the new module is added.

---

### Task 1: Create an owned implementation checkout

**Files:** No source changes.

- [ ] Run `git worktree list --porcelain` and `git status --short --branch`.
- [ ] Because this changes `crates/orkworksd/`, create `session-projection` as a branch in the primary checkout when no concurrent writer is active; otherwise create `../orkworks-session-projection` with `git worktree add ... -b session-projection`.
- [ ] Confirm the approved design commits are present and the checkout is clean.

### Task 2: Add characterization tests before extraction

**Files:** Modify `crates/orkworksd/src/http/session_handlers.rs` tests; inspect `main.rs` test support and `providers.rs` helpers.

**Produces:** Tests at the current `list_sessions` seam for live/remembered precedence, metadata fallback, no-workspace behavior, capacity/provider behavior, Git deduplication, ordering, and stale write-back rejection.

- [ ] Add a live-over-remembered test: duplicate IDs yield one record; live runtime fields win and persisted durable fields remain available.
- [ ] Add missing-directory, corrupt-remembered-file, and corrupt-live-metadata tests. Assert remembered records are omitted, live records remain, and the handler succeeds.
- [ ] Add a no-workspace test. Assert live sessions remain, remembered sessions are absent, and provider capping is still derived from live harness observations.
- [ ] Add a stale pending-write-back test. Mutate one snapshot identity field before write-back and assert latch, pending, visible-once, counters, and origin remain unchanged.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions`; expected: focused tests pass.
- [ ] Commit with `git add crates/orkworksd/src/http/session_handlers.rs && git commit -m "test: characterize session projection behavior"`.

### Task 3: Add the coordination seam

**Files:** Modify `crates/orkworksd/src/main.rs` and `crates/orkworksd/src/http/session_handlers.rs`; create `crates/orkworksd/src/session_projection.rs`.

**Produces:** `AppState.projection_lock`, `mod session_projection`, and `SessionProjection::new(Arc<AppState>)` with `pub(crate) fn list(&self) -> Vec<SessionInfo>`.

- [ ] Add `projection_lock: std::sync::Mutex<()>` to `AppState`; initialize it in production and every test fixture.
- [ ] Add the module declaration and a skeleton that borrows `Arc<AppState>` without a second registry or metadata authority.
- [ ] Make `set_workspace` acquire `projection_lock` before `state.workspace`, keeping it through replacement and reconciliation.
- [ ] Run the focused Rust tests and commit with `git commit -m "refactor: add session projection coordination seam"`.

### Task 4: Move snapshot and pure `SessionInfo` assembly

**Files:** Modify `crates/orkworksd/src/session_projection.rs` and `crates/orkworksd/src/http/session_handlers.rs`.

**Produces:** Live/remembered projection behind `SessionProjection::list`, with `session_view.rs` remaining pure.

- [ ] Move live-handle tuple extraction and capture workspace root/path plus identity. Release `sessions` and `workspace` locks before constructing `MetadataStore` or reading files.
- [ ] Construct metadata readers from the captured root, move canonical live and remembered field mapping, and preserve live-over-remembered precedence.
- [ ] Preserve current ordering: live `HashMap` iteration followed by remembered metadata ordering; add no new ordering guarantee.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions`; expected: all characterization tests remain green.
- [ ] Commit with `git commit -m "refactor: move session snapshot projection"`.

### Task 5: Move capacity, cwd/Git, conflicts, and provider publication

**Files:** Modify `crates/orkworksd/src/session_projection.rs` and the `list_sessions_*` tests in `crates/orkworksd/src/http/session_handlers.rs`.

**Produces:** Complete session projection policy behind one interface.

- [ ] Move bounded/raw capacity detection and pending visibility transitions. Compare runtime generation, latch, pending flag, visible-once flag, output counters, and resume-scan origin before every write-back; rejected writes change nothing.
- [ ] Move effective cwd precedence: reported cwd, live process cwd, launch cwd. Perform process and Git work outside state locks and retain one Git probe per unique cwd.
- [ ] Move conflict calculation and provider publication. Key maps by resolved `harness_id`; checking masks capped display; first reset hint wins; remembered sessions do not inherit live capacity.
- [ ] Hold `projection_lock` for the complete projection, workspace identity commit, and provider update. Recheck workspace identity in the commit section; on mismatch discard write-backs and return an empty list for that request.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions` and `cargo test --manifest-path crates/orkworksd/Cargo.toml provider`.
- [ ] Commit with `git commit -m "refactor: centralize session projection policy"`.

### Task 6: Thin the HTTP adapter and update documentation

**Files:** Modify `crates/orkworksd/src/http/session_handlers.rs`, `crates/orkworksd/src/main.rs`, `crates/orkworksd/AGENTS.md`, and `docs/agents/architecture.md`.

**Produces:** `list_sessions` as a thin Axum adapter and documented module ownership.

- [ ] Replace the handler body with `Arc<AppState>` cloning, `spawn_blocking(|| SessionProjection::list())`, `Json(Vec<SessionInfo>)` success, and HTTP 500/empty-body handling for any `JoinError`.
- [ ] Remove only projection helpers/imports now owned by `session_projection.rs`; retain DTOs, authorization, and unrelated handlers in `crates/orkworksd/src/http/session_handlers.rs`.
- [ ] Document `session_projection.rs` as stateful session-listing projection, distinguish it from pure `session_view.rs`, and record the projection lock and lock order.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`; expected: zero failures.
- [ ] Commit with `git commit -m "refactor: thin session listing handler"`.

### Task 7: Verify and request review

**Files:** No planned source changes; fix only verification findings.

- [ ] Run `cargo fmt --all -- --check`, `git diff --check`, and the full Rust test suite again.
- [ ] Run `bash .claude/hooks/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`; address documentation drift and report unrelated worktree warnings without touching other owners’ worktrees.
- [ ] Inspect `git status --short` and `git diff main...HEAD --stat`; confirm only the projection implementation, tests, and required docs changed.
- [ ] Request the manual `/code-review` gate because this changes `crates/orkworksd/` locking and projection concurrency.
