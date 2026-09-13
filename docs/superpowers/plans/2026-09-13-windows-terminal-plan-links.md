# Windows Terminal Plan-Path Links Implementation Plan (historical)

> Implemented on 2026-09-13. Verification passed for the renderer/Dockview
> suite, TypeScript, formatting, and focused selection safety. The broader
> Windows `plan_handoff` suite retains pre-existing separator-sensitive
> assertions documented in the review report.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing `/C:/...` terminal plan links usable on Windows by normalizing their display-only leading slash at the sidecar path boundary.

**Architecture:** Keep the renderer, preload IPC, Electron main process, Review panel, and terminal-link matcher unchanged. Preserve the exact clicked path through the existing authenticated selection route; in `plan_handoff.rs`, strip exactly one leading slash from a drive-qualified path only on Windows, then run the existing canonicalization, worktree-family, allowlist, Markdown, and file checks unchanged.

**Tech Stack:** Rust 2021, `orkworksd` plan-handoff module, `git2`, `tempfile`, TypeScript, xterm.js, Node’s built-in test runner, pnpm.

## Global Constraints

- The renderer passes the exact clicked terminal text and session ID; it does not normalize paths or gain filesystem access.
- Windows-only normalization accepts `/X:/...` and `/X:\\...` as aliases for `X:/...` and `X:\\...`; malformed forms remain rejected.
- The sidecar remains authoritative for worktree-family validation, containment, supported plan/spec roots, regular-file status, and `.md` extension.
- Validation must finish before metadata or event writes; rejected selection must have no persisted-state mutation.
- Existing live and historical terminal callbacks, session targeting, and singleton Review-tab behavior remain unchanged.
- Do not add arbitrary local-file opening, generic renderer filesystem access, or arbitrary terminal input.
- Use pnpm for Node commands and run Windows-specific path assertions on Windows.

---

### Task 1: Add regression tests for the existing click contract and Windows resolver

**Files:**
- Modify: `apps/desktop/tests/terminalLinks.test.ts`
- Modify: `crates/orkworksd/src/plan_handoff.rs` (test module)
- Modify: `crates/orkworksd/src/session_application.rs` (existing rejection test)

**Interfaces:**
- Consumes: existing `terminalPlanPaths`, `resolve_printed_plan_path`, and `SessionApplication::select_plan` behavior.
- Produces: failing Windows resolver coverage and a characterization test proving the renderer forwards the exact `/C:/...` text unchanged.

- [ ] **Step 1: Add the renderer characterization test.**

  Append this test to `apps/desktop/tests/terminalLinks.test.ts` near the existing absolute-path tests. It must remain a literal display-path test; no user-specific filesystem is accessed.

  ```ts
  test("detects slash-prefixed Windows absolute plan paths without rewriting them", () => {
    const path = "/C:/Users/froma/source/repos/orkworks-multi-workspace-design/specs/multi-workspace.md";
    assert.deepEqual(terminalPlanPaths(`Created ${path}`), [path]);
  });
  ```

- [ ] **Step 2: Add the platform-gated normalizer tests.**

  Add tests inside the existing `#[cfg(test)] mod tests` in `crates/orkworksd/src/plan_handoff.rs`. The Windows test pins the exact accepted aliases and malformed inputs; the non-Windows test pins the unchanged-input contract.

  ```rust
  #[cfg(windows)]
  #[test]
  fn normalizes_only_one_leading_slash_from_a_windows_drive_alias() {
      assert_eq!(normalize_windows_drive_alias("/C:/repo/specs/plan.md"), "C:/repo/specs/plan.md");
      assert_eq!(normalize_windows_drive_alias("/C:\\repo\\specs\\plan.md"), "C:\\repo\\specs\\plan.md");
      assert_eq!(normalize_windows_drive_alias("//C:/repo/specs/plan.md"), "//C:/repo/specs/plan.md");
      assert_eq!(normalize_windows_drive_alias("/1:/repo/specs/plan.md"), "/1:/repo/specs/plan.md");
      assert_eq!(normalize_windows_drive_alias("/C:relative/specs/plan.md"), "/C:relative/specs/plan.md");
  }

  #[cfg(not(windows))]
  #[test]
  fn leaves_windows_drive_display_aliases_unchanged_on_non_windows() {
      assert_eq!(normalize_windows_drive_alias("/C:/repo/specs/plan.md"), "/C:/repo/specs/plan.md");
  }
  ```

- [ ] **Step 3: Add a Windows real-path resolution test.**

  Add a `#[cfg(windows)]` test beside the existing printed-path tests. Build the fixture from `tempfile`, initialize Git in that fixture, create `specs/plan.md`, convert its native path to forward slashes, prepend one display slash, and assert that `resolve_printed_plan_path` returns the fixture’s canonical worktree root and `specs/plan.md`. This must fail before the implementation because `Path::new("/C:/...")` does not resolve to the fixture.

  ```rust
  #[cfg(windows)]
  #[test]
  fn resolves_a_single_slash_prefixed_windows_drive_plan_path() {
      let workspace = tempfile::tempdir().unwrap();
      let plan = workspace.path().join("specs/plan.md");
      std::fs::create_dir_all(plan.parent().unwrap()).unwrap();
      std::fs::write(&plan, "# plan").unwrap();
      git2::Repository::init(workspace.path()).unwrap();
      let printed = format!("/{}", plan.to_string_lossy().replace('\\', "/"));

      let (root, relative) = resolve_printed_plan_path(workspace.path(), &printed).unwrap();

      assert_eq!(root, workspace.path().canonicalize().unwrap());
      assert_eq!(relative, "specs/plan.md");
  }
  ```

- [ ] **Step 4: Strengthen the existing selection rejection test.**

  In `select_plan_application_seam_rejects_unresolvable_path`, after the `select_plan` call returns `Err(SessionError::Conflict)`, read the session and event log and assert `plan_path.is_none()` and that no `session.plan_selected_by_user` event exists. This pins the validation-before-write guarantee without changing the application seam.

- [ ] **Step 5: Run the focused tests and confirm the new Windows test fails.**

  Run from the repository root:

  ```powershell
  pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts
  cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff
  cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path
  ```

  Expected: the renderer characterization and existing tests pass; on Windows, the new resolver test fails because the leading display slash is not yet normalized. No production code is changed in this task.

- [ ] **Step 6: Commit the regression tests.**

  ```powershell
  git add apps/desktop/tests/terminalLinks.test.ts crates/orkworksd/src/plan_handoff.rs crates/orkworksd/src/session_application.rs
  git commit -m "test: cover slash-prefixed Windows plan paths"
  ```

### Task 2: Normalize the display alias at the sidecar boundary

**Files:**
- Modify: `crates/orkworksd/src/plan_handoff.rs`

**Interfaces:**
- Consumes: the exact `printed_path` supplied by `SessionApplication::select_plan`.
- Produces: `normalize_windows_drive_alias(&str) -> String`, used only before `Path::new` in `resolve_printed_plan_path_with_home`.

- [ ] **Step 1: Add the minimal Windows-gated helper.**

  Place this helper immediately before `resolve_printed_plan_path_with_home`. It must recognize one leading `/`, one ASCII drive letter, `:`, and one path separator. It must not rewrite `//C:/`, `/1:/`, `/C:relative`, or any other input.

  ```rust
  fn normalize_windows_drive_alias(path: &str) -> String {
      #[cfg(windows)]
      {
          let bytes = path.as_bytes();
          let is_drive_letter = bytes.get(1).is_some_and(|byte| byte.is_ascii_alphabetic());
          let has_drive_separator = matches!(bytes.get(3), Some(b'/') | Some(b'\\'));
          if bytes.first() == Some(&b'/')
              && is_drive_letter
              && bytes.get(2) == Some(&b':')
              && has_drive_separator
          {
              return path[1..].to_owned();
          }
      }
      path.to_owned()
  }
  ```

- [ ] **Step 2: Apply the helper before platform path parsing.**

  In `resolve_printed_plan_path_with_home`, keep the control-character check and exact `printed_path` for logging, then construct `Path` from the normalized local string:

  ```rust
  let normalized_path = normalize_windows_drive_alias(printed_path);
  let printed = Path::new(&normalized_path);
  ```

  Do not change the subsequent absolute/relative branching, canonicalization, Git common-directory comparison, worktree-relative calculation, supported-root allowlist, regular-file check, or Markdown extension check.

- [ ] **Step 3: Run the focused tests to verify the fix.**

  ```powershell
  cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff
  cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path
  ```

  Expected: PASS, including the Windows drive-alias resolution test and malformed-alias normalizer tests. The selection rejection test must still show no plan reference or user-selection event after failure.

- [ ] **Step 4: Commit the implementation.**

  ```powershell
  git add crates/orkworksd/src/plan_handoff.rs
  git commit -m "fix: normalize Windows terminal plan paths"
  ```

### Task 3: Verify the unchanged renderer and Review-tab integration

**Files:**
- No production files; verify `apps/desktop/src/terminalLinks.ts`, `apps/desktop/src/terminalStore.ts`, `apps/desktop/src/components/HistoricalTerminal.tsx`, `apps/desktop/src/App.tsx`, and `apps/desktop/src/components/DockviewApp.tsx` remain unchanged.

**Interfaces:**
- Consumes: the existing terminal-plan link provider, live/historical callbacks, authenticated IPC, session refresh, and singleton Review panel.
- Produces: evidence that the Rust-only fix preserves the existing click-to-Review flow.

- [ ] **Step 1: Run the renderer terminal-link and Dockview tests.**

  ```powershell
  pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts tests/dockview.test.ts
  ```

  Expected: PASS, including exact `/C:/...` detection, unchanged callback text, live/historical callback wiring, session targeting, and one Review panel.

- [ ] **Step 2: Run TypeScript validation.**

  ```powershell
  pnpm --dir apps/desktop exec tsc --noEmit
  ```

  Expected: PASS with no renderer/preload contract changes.

- [ ] **Step 3: Run formatting and diff checks.**

  ```powershell
  cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
  git diff --check
  git status --short --branch
  ```

  Expected: Rust formatting passes, the diff has no whitespace errors, and only the planned sidecar/test files are changed.

- [ ] **Step 4: Confirm the Windows acceptance path manually.**

  In a Windows OrkWorks session whose terminal prints:

  ```text
  /C:/Users/froma/source/repos/orkworks-multi-workspace-design/specs/multi-workspace.md
  ```

  Hover and click the path. Expected: the existing Review tab opens for the selected session, reads the linked-worktree artifact, and no external editor or Explorer window is launched.
