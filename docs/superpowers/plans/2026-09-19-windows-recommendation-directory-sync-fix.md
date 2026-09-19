# Windows Recommendation Handoff Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Windows Taskmaster recommendation acceptance succeed without weakening file-level publication checks, and cover the regression in focused tests and Windows CI.

**Architecture:** Keep the recommendation store's atomic transaction protocol unchanged. On Windows, treat directory-entry syncing as a best-effort durability operation because the current Rust standard-library open does not obtain a syncable directory handle; keep staged-file flushing, `MoveFileExW` write-through for new files, and `ReplaceFileW`/rename errors fatal. Close every temporary file before handing its path to Windows replacement APIs.

**Tech Stack:** Rust (`orkworksd`), `tempfile`, Tokio tests, GitHub Actions PowerShell test routing, Node test runner for desktop checks.

## Global Constraints

- Taskmaster `improve_workflow` acceptance remains an explicit user-confirmed action into the user's active session; this fix must not add automatic execution or session creation.
- Do not ask users to run OrkWorks elevated or change filesystem ACLs; the failure is an API-semantics mismatch.
- Flush staged file contents before publication; preserve existing Windows `ReplaceFileW` and `MoveFileExW(MOVEFILE_WRITE_THROUGH)` publication behavior.
- Unix directory-sync failures remain errors; only Windows directory syncing becomes best-effort.
- Use `pnpm` for Node package-management tasks.
- All changes remain on the isolated `windows-recommendation-fix` branch and go through a pull request.

---

### Task 1: Add failing store and platform-behavior tests

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/store.rs:182-197` (`put` test coverage near the existing store tests)
- Modify: `crates/orkworksd/src/taskmaster/store.rs:2187-2191` (`sync_directory` platform tests)

**Interfaces:**
- Consumes: `RecommendationStore::open`, `RecommendationStore::put`, `RecommendationStore::get`, the existing `recommendation` test fixture, and `sync_directory`.
- Produces: A focused test proving an existing recommendation can be replaced, plus platform-specific expectations for directory syncing.

- [ ] **Step 1: Write the replacement regression test**

Add a store test using the existing fixture and update the same recommendation twice:

```rust
#[test]
fn replaces_an_existing_recommendation() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
    let mut first = recommendation("replace-existing", "session");
    store.put(&first).unwrap();

    first.status = RecommendationStatus::Accepted;
    store.put(&first).unwrap();

    assert_eq!(
        store.get("replace-existing").unwrap().unwrap().status,
        RecommendationStatus::Accepted
    );
}
```

- [ ] **Step 2: Make the directory-sync test platform-specific**

Keep the current missing-directory failure assertion on Unix and add the Windows contract explicitly:

```rust
#[cfg(not(windows))]
#[test]
fn reports_directory_sync_failures() {
    let missing = tempfile::tempdir().unwrap().path().join("missing");
    assert!(matches!(sync_directory(&missing), Err(StoreError::Io(_))));
}

#[cfg(windows)]
#[test]
fn treats_directory_sync_as_best_effort_on_windows() {
    let missing = tempfile::tempdir().unwrap().path().join("missing");
    assert!(sync_directory(&missing).is_ok());
}
```

- [ ] **Step 3: Run the focused tests to verify the regression is present**

Run from the repository root:

```powershell
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::accept_recommendation_submits_prompt_and_transitions_status -- --exact --nocapture --test-threads=1
```

Expected on the current Windows implementation: the replacement/acceptance path fails with a Windows I/O error, and the existing missing-directory test fails because `File::open` returns `PermissionDenied`. The Unix run should retain the existing missing-directory failure behavior.

- [ ] **Step 4: Commit the failing tests**

```powershell
git add crates/orkworksd/src/taskmaster/store.rs
git commit -m "test: reproduce Windows recommendation store failures"
```

### Task 2: Fix Windows temporary-handle and directory-sync behavior

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/store.rs:182-197` (`put`)
- Modify: `crates/orkworksd/src/taskmaster/store.rs:1238-1249` (`sync_directory`)

**Interfaces:**
- Consumes: The tests from Task 1 and `crate::harness::integration::atomic_replace`.
- Produces: A `put` path that closes its temporary file before replacement, and a platform-compiled `sync_directory` whose Windows implementation is explicitly best-effort.

- [ ] **Step 1: Close the single-record temporary file before replacement**

Insert the handle drop immediately after `sync_all` and before checking the target or calling `atomic_replace`:

```rust
file.write_all(&json).map_err(StoreError::Io)?;
file.sync_all().map_err(StoreError::Io)?;
drop(file);
let target_existed = path.exists();
crate::harness::integration::atomic_replace(&temp, &path, target_existed)
    .map_err(StoreError::Io)?;
```

This prevents Windows `ReplaceFileW` from seeing the source temporary file as open for writing. Do not change the existing error handling for file creation, writes, flushes, or publication.

- [ ] **Step 2: Compile separate Windows and Unix implementations of directory sync**

Replace the runtime `cfg!` error-kind exception with platform-specific bodies:

```rust
#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    let _ = path;
    // Rust's standard File::open does not request the Win32 directory-handle
    // semantics needed for Unix-style directory fsync. File contents are
    // still synced before publication; new-file publication uses
    // MOVEFILE_WRITE_THROUGH. ReplaceFileW has no write-through flag, so
    // Windows cannot provide equivalent directory-entry crash durability here.
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    fs::File::open(path)
        .map_err(StoreError::Io)
        .and_then(|directory| directory.sync_all().map_err(StoreError::Io))
}
```

Keep the helper private and retain Unix errors as `StoreError::Io`; do not swallow ACL or publication errors.

- [ ] **Step 3: Run the focused tests to verify the fix**

```powershell
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::accept_recommendation_submits_prompt_and_transitions_status -- --exact --nocapture --test-threads=1
```

Expected: the store tests and existing acceptance regression pass on Windows; Unix retains its missing-directory error assertion and all store tests pass.

- [ ] **Step 4: Commit the implementation**

```powershell
git add crates/orkworksd/src/taskmaster/store.rs
git commit -m "fix: allow Windows recommendation handoff"
```

### Task 3: Route the regression through Windows CI

**Files:**
- Modify: `.github/workflows/pr-ci.yml:144-162` (Windows required-test discovery list)
- Modify: `.github/workflows/pr-ci.yml:163-176` (Windows focused test commands)

**Interfaces:**
- Consumes: The exact Rust test names from Tasks 1 and 2.
- Produces: A Windows CI job that fails if either the focused store regression or the end-to-end acceptance regression is missing or failing.

- [ ] **Step 1: Require both tests in the Windows discovery list**

Add these exact entries to `$required`:

```powershell
"taskmaster::store::tests::replaces_an_existing_recommendation",
"session_application::tests::accept_recommendation_submits_prompt_and_transitions_status",
```

- [ ] **Step 2: Run the focused suites in the Windows job**

Add a PowerShell step after the existing Taskmaster checks:

```yaml
- name: Test Windows recommendation persistence and handoff
  shell: pwsh
  run: |
    cargo test --locked --manifest-path crates/orkworksd/Cargo.toml taskmaster::store
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo test --locked --manifest-path crates/orkworksd/Cargo.toml session_application::tests::accept_recommendation_submits_prompt_and_transitions_status -- --exact --test-threads=1
```

- [ ] **Step 3: Validate the workflow edit locally**

Run:

```powershell
git diff --check
```

Expected: no whitespace errors. Confirm the required names match `cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd -- --list` on Windows.

- [ ] **Step 4: Commit the CI routing**

```powershell
git add .github/workflows/pr-ci.yml
git commit -m "ci: cover Windows recommendation handoff"
```

### Task 4: Verify the complete change

**Files:**
- Verify: `crates/orkworksd/src/taskmaster/store.rs`
- Verify: `.github/workflows/pr-ci.yml`
- Verify: `apps/desktop/tests/FixWithAiDialog.test.ts`, `apps/desktop/tests/api.test.ts`

**Interfaces:**
- Consumes: The completed implementation and CI changes from Tasks 1–3.
- Produces: Evidence that the Windows fix is formatted, tested, and does not regress the desktop handoff contract.

- [ ] **Step 1: Run Rust formatting and the full sidecar suite**

```powershell
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: formatting passes and the full Rust suite passes.

- [ ] **Step 2: Run the focused desktop tests**

From `apps/desktop/`:

```powershell
node --experimental-strip-types --test tests/FixWithAiDialog.test.ts tests/api.test.ts
```

Expected: all existing Fix-with-AI and API tests pass; no renderer or IPC contract changes are needed.

- [ ] **Step 3: Inspect the final diff**

```powershell
git diff --check HEAD~3..HEAD
git status --short --branch
```

Expected: only the design/plan documentation, recommendation-store Rust code/tests, and Windows CI routing are changed; the worktree is clean after commits.

- [ ] **Step 4: Run the required review gate before PR handoff**

Invoke `/code-review low` against the final branch diff, address any findings, then provide the PR handoff with the exact test evidence. Do not claim completion until verification output is available.
