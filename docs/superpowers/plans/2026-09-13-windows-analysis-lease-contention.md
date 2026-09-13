# Windows Analysis Lease Contention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Classify native Windows `fs2` lock contention as a busy Taskmaster analysis lease without hiding unrelated I/O failures.

**Architecture:** Keep the existing file-lock lease and cross-process exclusion unchanged. Add one local predicate in `taskmaster/runtime.rs` that recognizes `WouldBlock` everywhere and Windows `ERROR_LOCK_VIOLATION` (33) on Windows only; use it at the existing lease boundary.

**Tech Stack:** Rust, `std::io`, `fs2`, Cargo unit tests.

## Global Constraints

- Preserve the application-wide single-evaluation lease contract in `specs/taskmaster-knowledge.md`.
- Preserve cross-process exclusion; only classify supported contention errors as busy.
- Use the existing Rust dependency set; add no dependencies.
- Validate with native Windows targeted and full Taskmaster tests where available.

---

### Task 1: Classify native analysis-lock contention

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/runtime.rs`
- Test: `crates/orkworksd/src/taskmaster/runtime.rs` inline `tests` module

**Interfaces:**
- Consumes: `std::io::Error` returned by `fs2::FileExt::try_lock_exclusive`.
- Produces: `try_analysis_lease() -> Result<Option<fs::File>, String>` returning `Ok(None)` for `WouldBlock` and Windows raw OS error 33, and `Err` for other errors.

- [ ] **Step 1: Add the failing classifier tests**

  Add tests that assert `WouldBlock` is contention, Windows raw error 33 is contention, and a different raw error is not contention. Guard the Windows-specific assertion and implementation with `cfg(windows)`.

- [ ] **Step 2: Run the classifier test and confirm the expected red failure**

  Run:

  ```text
  cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::runtime::tests::analysis_lease_contention_errors_are_classified -- --exact --nocapture
  ```

  Expected: the new Windows error-33 assertion fails because the current code recognizes only `ErrorKind::WouldBlock`.

- [ ] **Step 3: Implement the minimal predicate and use it in `try_analysis_lease`**

  Add a private `is_analysis_lease_contention(&std::io::Error) -> bool` helper. Return true for `ErrorKind::WouldBlock`; on Windows also return true when `raw_os_error() == Some(33)`. Replace the inline `WouldBlock` guard with this helper. Leave file-open errors and every other lock error unchanged.

- [ ] **Step 4: Run the targeted regression and full Rust validation**

  Run:

  ```text
  cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::runtime::tests::inference_lease_excludes_independent_runtime_instances_until_drop -- --exact --nocapture
  cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::runtime::tests:: -- --nocapture
  cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
  ```

  Expected: the native contention regression, classifier tests, Taskmaster runtime tests, and formatting check pass; unrelated errors remain rejected by the helper test.

- [ ] **Step 5: Review the diff and commit the scoped fix**

  Run `git diff --check` and inspect `git diff -- crates/orkworksd/src/taskmaster/runtime.rs`. Commit the plan and implementation together with:

  ```text
  git add docs/superpowers/plans/2026-09-13-windows-analysis-lease-contention.md crates/orkworksd/src/taskmaster/runtime.rs
  git commit -m "fix: classify Windows Taskmaster lease contention"
  ```
