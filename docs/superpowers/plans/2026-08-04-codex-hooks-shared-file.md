# Codex Hooks Shared-File Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make OrkWorks' Codex `SessionStart` hook integration actually install in APM-managed repos (including this one), where `.codex/hooks.json` is git-tracked, by rewriting Codex's reporter-script invocation as a `$HOME`-relative shell expression instead of a resolved absolute path, and narrowing the existing tracked-file safety refusal to accept that portable case — Codex only.

**Architecture:** `crates/orkworksd/src/harness/integrations/mod.rs` gains a POSIX-only `portable_reporter_path`/`portable_reporter_invocation` pair. `crates/orkworksd/src/harness/integrations/codex.rs`'s `probe`/`merge` use it instead of the shared `reporter_invocation`. `crates/orkworksd/src/harness/integration.rs`'s `ValidatedWorkspaceTarget::require_local_or_ignored_untracked` is split so a new `require_tracked_or_ignored_untracked` (Codex only) accepts a tracked target while still refusing an untracked-and-unignored one. `JsonHookHandler::load()` picks which check to call based on `harness_id` and the current platform.

**Tech Stack:** Rust (`crates/orkworksd`), `git2`, `dirs`, `serde_json`, `tempfile` (dev-dependency, tests only).

## Global Constraints

- Design doc: `docs/superpowers/specs/2026-08-04-codex-hooks-shared-file-design.md` — read it before starting; every task below implements a specific part of it.
- Scope is Codex only. Do not touch Claude/Gemini/Copilot's invocation format or safety-check behavior.
- The portable rewrite is POSIX-only for this change. Windows Codex keeps today's behavior (absolute path, still refused on a tracked target) — do not attempt PowerShell `$HOME` support.
- Run all commands from the repository root: `cargo build --manifest-path crates/orkworksd/Cargo.toml` / `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- Any test that mutates the `HOME` environment variable must be guarded by a `Mutex` (see Task 1) so parallel `cargo test` threads don't race — this repo's existing precedent is `ENV_LOCK` in `crates/orkworksd/src/peon.rs:772`.
- Every new/changed Rust test follows this repo's existing style: `tempfile::tempdir()` for filesystem fixtures, `git2::Repository::init`/`index.add_path`/`index.write` to simulate tracked files, doc comments only where the *why* isn't obvious from the code.

---

### Task 1: Portable reporter path/invocation builders

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`

**Interfaces:**
- Produces: `pub(crate) fn portable_reporter_path(reporter: &Path) -> Result<PathBuf, IntegrationError>` and `pub(crate) fn portable_reporter_invocation(reporter: &Path, marker: &str) -> Result<ReporterInvocation, IntegrationError>`, both in `crate::harness::integrations` (same module as the existing `reporter_invocation`). Task 3 imports and calls both from `codex.rs`.
- Consumes: `dirs::home_dir()` (already a dependency, used identically elsewhere — see `crates/orkworksd/src/harness/integration.rs:512`), the existing `ReporterInvocation` struct (`mod.rs:48-53`) and `shell_quote` helper (`mod.rs:402-407`).

- [ ] **Step 1: Write the failing unit tests for `portable_reporter_path`**

Add to the `#[cfg(test)] mod tests` block in `crates/orkworksd/src/harness/integrations/mod.rs`, near the existing `reporter_rendering_is_explicit_for_posix_and_powershell` test (~line 1219). First add the shared lock these and later HOME-mutating tests need — put it right after `use super::*;` at the top of the test module:

```rust
    use std::sync::Mutex;

    // Mirrors peon.rs's ENV_LOCK (crates/orkworksd/src/peon.rs:772): several
    // tests below set HOME to a controlled tempdir so dirs::home_dir()
    // resolves deterministically. Without a lock, parallel cargo test
    // threads mutating the same process-wide env var race and flake.
    static ENV_LOCK: Mutex<()> = Mutex::new(());
```

(If `use std::sync::Mutex;` is already present in that block from another change, skip re-adding it — just add the `ENV_LOCK` static.)

Then add:

```rust
    #[test]
    fn portable_reporter_path_rewrites_the_resolved_path_as_home_relative() {
        let _guard = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let reporter = home
            .path()
            .join(".orkworks/hook-scripts/report-harness-event.sh");
        let result = portable_reporter_path(&reporter);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(
            result.unwrap(),
            Path::new("$HOME/.orkworks/hook-scripts/report-harness-event.sh")
        );
    }

    #[test]
    fn portable_reporter_path_errors_when_reporter_is_not_under_home() {
        let _guard = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let outside = tempfile::tempdir().unwrap();
        let reporter = outside.path().join("report-harness-event.sh");
        let result = portable_reporter_path(&reporter);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert!(matches!(result, Err(IntegrationError::InvalidConfig(_))));
    }
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::portable_reporter_path -- --nocapture`
Expected: compile error, `portable_reporter_path` not found.

- [ ] **Step 3: Implement `portable_reporter_path`**

Add to `crates/orkworksd/src/harness/integrations/mod.rs`, right after the existing `reporter_invocation` function (~line 400, before `fn shell_quote`):

```rust
/// Rewrites an absolute reporter-script path into a `$HOME`-relative shell
/// expression, so the resulting hook command is byte-identical no matter
/// whose machine generated it. Required before Codex's reporter invocation
/// is safe to persist into a git-tracked, team-shared `.codex/hooks.json` —
/// Codex has no local-only hooks file the way Claude's `settings.local.json`
/// is local by convention (ADR 0035, ADR 0036). POSIX only; Codex on
/// Windows keeps writing a resolved absolute path (see ADR 0036).
pub(crate) fn portable_reporter_path(reporter: &Path) -> Result<PathBuf, IntegrationError> {
    let home = dirs::home_dir().ok_or_else(|| {
        IntegrationError::InvalidConfig(
            "Could not resolve a home directory to build a portable Codex hook command.".into(),
        )
    })?;
    let suffix = reporter.strip_prefix(&home).map_err(|_| {
        IntegrationError::InvalidConfig(format!(
            "Codex reporter script {} is not under the home directory; cannot build a portable hook command.",
            reporter.display()
        ))
    })?;
    Ok(Path::new("$HOME").join(suffix))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::portable_reporter_path -- --nocapture`
Expected: both tests PASS.

- [ ] **Step 5: Write the failing test for `portable_reporter_invocation`, including real shell execution**

Add to the same test module:

```rust
    #[cfg(unix)]
    #[test]
    fn portable_reporter_invocation_expands_home_and_actually_invokes_the_script() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let script_dir = home.path().join(".orkworks").join("hook-scripts");
        fs::create_dir_all(&script_dir).unwrap();
        let script = script_dir.join("report-harness-event.sh");
        let marker_file = home.path().join("invoked");
        fs::write(
            &script,
            format!("#!/bin/sh\ntouch '{}'\n", marker_file.display()),
        )
        .unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(&script, perms).unwrap();

        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());
        let invocation =
            portable_reporter_invocation(&script, "orkworks:harness-integration:v2:codex")
                .unwrap();
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&invocation.shell_command)
            .status()
            .expect("sh must be available to run the portable command");
        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert!(
            status.success(),
            "portable shell_command failed: {}",
            invocation.shell_command
        );
        assert!(marker_file.exists(), "reporter script was not invoked");
    }
```

- [ ] **Step 6: Run the test to verify it fails to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::portable_reporter_invocation -- --nocapture`
Expected: compile error, `portable_reporter_invocation` not found.

- [ ] **Step 7: Implement `portable_reporter_invocation`**

Add directly below `portable_reporter_path`:

```rust
/// POSIX-only portable counterpart to `reporter_invocation`. Same shape,
/// but the path segment is `$HOME`-relative and double-quoted instead of
/// single-quoted (`shell_quote` always single-quotes, and single-quoted
/// strings don't expand `$HOME` in POSIX shells). Double-quoting is safe
/// here specifically because the path segment is always a fixed,
/// OrkWorks-authored suffix under `$HOME` — never user input, and never
/// containing a `"` or `$` — the same never-untrusted-input guarantee
/// `shell_quote` gives the marker argument, which stays single-quoted and
/// unchanged.
pub(crate) fn portable_reporter_invocation(
    reporter: &Path,
    marker: &str,
) -> Result<ReporterInvocation, IntegrationError> {
    let portable = portable_reporter_path(reporter)?;
    let path_str = portable.to_string_lossy().into_owned();
    Ok(ReporterInvocation {
        program: path_str.clone(),
        args: vec!["--marker".into(), marker.into()],
        shell_command: format!("\"{path_str}\" --marker {}", shell_quote(marker)),
    })
}
```

- [ ] **Step 8: Run the test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::portable_reporter_invocation -- --nocapture`
Expected: PASS.

- [ ] **Step 9: Run the full mod.rs test module to check for regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests`
Expected: all PASS (no existing test touches these new functions yet).

- [ ] **Step 10: Commit**

```bash
git add crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "feat(sidecar): add portable (\$HOME-relative) reporter invocation builder"
```

---

### Task 2: Narrow the tracked-file safety check for a portable-safe caller

**Files:**
- Modify: `crates/orkworksd/src/harness/integration.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub(crate) fn require_tracked_or_ignored_untracked(&self) -> Result<(), IntegrationError>` on `ValidatedWorkspaceTarget`, alongside the existing `require_local_or_ignored_untracked`. Task 4 calls the new method from `JsonHookHandler::load()`.

- [ ] **Step 1: Write the failing test**

Add to `crates/orkworksd/src/harness/integration.rs`'s test module, directly after `git_safety_rejects_tracked_and_unignored_targets_but_accepts_ignored_local_targets` (~line 247):

```rust
    #[test]
    fn git_safety_tracked_or_ignored_accepts_a_tracked_target_but_still_rejects_unignored_untracked(
    ) {
        let workspace = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(workspace.path()).unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        let relative = Path::new(".codex/hooks.json");
        let path = workspace.path().join(relative);
        fs::write(&path, "{}").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(relative).unwrap();
        index.write().unwrap();

        // Unlike require_local_or_ignored_untracked, a tracked target is
        // now accepted — this is the whole point of the relaxation.
        let tracked = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        tracked.require_tracked_or_ignored_untracked().unwrap();

        // An untracked-and-unignored target is still refused, exactly as
        // for every other integration — the relaxation only widens the
        // tracked case, nothing else.
        index.remove_path(relative).unwrap();
        index.write().unwrap();
        let unignored = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        assert_eq!(
            unignored
                .require_tracked_or_ignored_untracked()
                .unwrap_err()
                .code(),
            "not_ignored_target"
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integration::tests::git_safety_tracked_or_ignored -- --nocapture`
Expected: compile error, `require_tracked_or_ignored_untracked` not found.

- [ ] **Step 3: Refactor `require_local_or_ignored_untracked` and add the new method**

In `crates/orkworksd/src/harness/integration.rs`, replace the existing method (currently ~lines 685-724):

```rust
    pub(crate) fn require_local_or_ignored_untracked(&self) -> Result<(), IntegrationError> {
        self.revalidate()?;
        let repository = git2::Repository::discover(&self.identity.canonical_root).map_err(|_| {
            IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Workspace integration files require a Git workspace so tracked files are never edited.".into(),
            }
        })?;
        let workdir = repository
            .workdir()
            .ok_or_else(|| IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Bare repositories cannot contain workspace integration files.".into(),
            })?;
        if fs::canonicalize(workdir)? != self.identity.canonical_root {
            return Err(IntegrationError::UnsafeTarget {
                code: "workspace_repository_mismatch",
                message: "Workspace root does not match the Git worktree root.".into(),
            });
        }
        let index = repository
            .index()
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?;
        if index.get_path(&self.relative, 0).is_some() {
            return Err(IntegrationError::UnsafeTarget {
                code: "tracked_target",
                message: "Integration configuration is tracked by Git and will not be edited automatically.".into(),
            });
        }
        if !repository
            .status_should_ignore(&self.relative)
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?
        {
            return Err(IntegrationError::UnsafeTarget {
                code: "not_ignored_target",
                message: "Integration configuration is not ignored by Git and will not be edited automatically.".into(),
            });
        }
        Ok(())
    }
```

with:

```rust
    pub(crate) fn require_local_or_ignored_untracked(&self) -> Result<(), IntegrationError> {
        self.require_confined_git_target(false)
    }

    /// Codex-only relaxation: unlike Claude/Gemini/Copilot, Codex has no
    /// separate local-only hooks file, so a git-tracked `.codex/hooks.json`
    /// is an expected APM-managed-repo shape, not a misconfiguration (ADR
    /// 0036, issue #276). Safe only because the caller must write a
    /// `$HOME`-relative, machine-independent command
    /// (`portable_reporter_invocation`) rather than an absolute per-machine
    /// path. An untracked-and-unignored target is still refused, exactly as
    /// for every other integration — only the tracked case widens.
    pub(crate) fn require_tracked_or_ignored_untracked(&self) -> Result<(), IntegrationError> {
        self.require_confined_git_target(true)
    }

    fn require_confined_git_target(&self, allow_tracked: bool) -> Result<(), IntegrationError> {
        self.revalidate()?;
        let repository = git2::Repository::discover(&self.identity.canonical_root).map_err(|_| {
            IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Workspace integration files require a Git workspace so tracked files are never edited.".into(),
            }
        })?;
        let workdir = repository
            .workdir()
            .ok_or_else(|| IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Bare repositories cannot contain workspace integration files.".into(),
            })?;
        if fs::canonicalize(workdir)? != self.identity.canonical_root {
            return Err(IntegrationError::UnsafeTarget {
                code: "workspace_repository_mismatch",
                message: "Workspace root does not match the Git worktree root.".into(),
            });
        }
        let index = repository
            .index()
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?;
        let tracked = index.get_path(&self.relative, 0).is_some();
        if tracked {
            if allow_tracked {
                return Ok(());
            }
            return Err(IntegrationError::UnsafeTarget {
                code: "tracked_target",
                message: "Integration configuration is tracked by Git and will not be edited automatically.".into(),
            });
        }
        if !repository
            .status_should_ignore(&self.relative)
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?
        {
            return Err(IntegrationError::UnsafeTarget {
                code: "not_ignored_target",
                message: "Integration configuration is not ignored by Git and will not be edited automatically.".into(),
            });
        }
        Ok(())
    }
```

- [ ] **Step 4: Run the new test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integration::tests::git_safety_tracked_or_ignored -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run the full integration.rs test module to check for regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integration::tests`
Expected: all PASS, including the pre-existing `git_safety_rejects_tracked_and_unignored_targets_but_accepts_ignored_local_targets` unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/harness/integration.rs
git commit -m "feat(sidecar): add tracked-or-ignored safety check for portable-safe integrations"
```

---

### Task 3: Switch codex.rs to portable invocations

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs`

**Interfaces:**
- Consumes: `portable_reporter_invocation` and `ReporterInvocation` from Task 1 (`crate::harness::integrations`).
- Produces: no change to `codex.rs`'s exported `HANDLER` shape or its `probe`/`merge`/`remove` function signatures (still `fn(&Map<String, Value>, &Path) -> Result<FragmentState, IntegrationError>` etc.) — Task 4 relies on these signatures being unchanged.

- [ ] **Step 1: Write the failing tests**

In `crates/orkworksd/src/harness/integrations/codex.rs`, replace the `use super::{...}` import line:

```rust
use super::{
    reconcile_current, reporter_invocation, FragmentState, JsonHookHandler, ToolHookContract,
};
```

with:

```rust
use super::{
    portable_reporter_invocation, reconcile_current, FragmentState, JsonHookHandler,
    ReporterInvocation, ToolHookContract,
};
```

Add a `HomeGuard` test helper and new tests to the `#[cfg(test)] mod tests` block (after `use super::*;`, before the first existing test):

```rust
    use std::sync::Mutex;

    // Points HOME at a fresh tempdir for the guard's lifetime, so
    // portable_reporter_invocation (which reads dirs::home_dir()) resolves
    // deterministically in tests. Restoring HOME happens in Drop (not a
    // plain post-assertion statement) so a panicking assertion mid-test
    // still restores the real machine's HOME instead of leaking the
    // mutation into whichever test runs next — mirrors peon.rs's ENV_LOCK
    // pattern (crates/orkworksd/src/peon.rs:772) but needed here on nearly
    // every test in this module, not just two, so it's worth a guard type.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct HomeGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        previous: Option<std::ffi::OsString>,
        dir: tempfile::TempDir,
    }

    impl HomeGuard {
        fn new() -> Self {
            let lock = ENV_LOCK.lock().unwrap();
            let previous = std::env::var_os("HOME");
            let dir = tempfile::tempdir().unwrap();
            std::env::set_var("HOME", dir.path());
            Self {
                _lock: lock,
                previous,
                dir,
            }
        }

        fn reporter_path(&self) -> std::path::PathBuf {
            self.dir
                .path()
                .join(".orkworks/hook-scripts/report-harness-event.sh")
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn merge_writes_a_home_relative_command_not_an_absolute_path() {
        let home = HomeGuard::new();
        let mut document = Map::new();

        merge(&mut document, &home.reporter_path()).unwrap();

        let command = document["hooks"]["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            command.starts_with("\"$HOME/"),
            "expected a $HOME-relative command, got: {command}"
        );
        assert!(
            !command.contains(home.dir.path().to_str().unwrap()),
            "command must not embed the real (machine-specific) home directory: {command}"
        );
    }

    #[test]
    fn probe_reports_installed_after_merge_and_drifted_for_a_pre_portable_absolute_path_fragment()
    {
        let home = HomeGuard::new();
        let mut document = Map::new();
        merge(&mut document, &home.reporter_path()).unwrap();
        assert_eq!(
            probe(&document, &home.reporter_path()).unwrap(),
            FragmentState::Installed
        );

        // Simulates a fragment written by a pre-fix OrkWorks version, which
        // embedded the resolved absolute path instead of a $HOME-relative
        // one — must read as Drifted (triggering reconciliation on the next
        // install), never silently as Installed.
        let mut stale = Map::new();
        stale.insert(
            "hooks".into(),
            json!({
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!(
                            "{} --marker '{}'",
                            home.reporter_path().display(),
                            MARKER
                        )
                    }]
                }]
            }),
        );
        assert_eq!(
            probe(&stale, &home.reporter_path()).unwrap(),
            FragmentState::Drifted
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::codex::tests -- --nocapture`
Expected: compile errors — `merge`/`probe` still call `reporter_invocation` (infallible, wrong argument shape) and the import no longer resolves.

- [ ] **Step 3: Update `marker_state`, `probe`, and `merge` to use the portable invocation**

Replace `marker_state`'s signature and exact-match block:

```rust
fn marker_state(group: &Value, reporter: Option<&Path>) -> FragmentState {
```

with:

```rust
fn marker_state(group: &Value, expected: Option<&ReporterInvocation>) -> FragmentState {
```

and inside it, replace:

```rust
        let exact = reporter.is_some_and(|path| {
            let invocation = reporter_invocation(path, MARKER);
            // merge() never sets an outer "matcher" — it intentionally
            // matches every SessionStart source. A group edited to add one
            // (e.g. narrowing to "resume") stops firing on startup/clear/
            // compact even though the inner command is untouched, so that
            // must not read as Installed.
            group.get("matcher").is_none()
                && hook.get("type").and_then(Value::as_str) == Some("command")
                && command == invocation.shell_command.as_str()
        });
```

with:

```rust
        let exact = expected.is_some_and(|invocation| {
            // merge() never sets an outer "matcher" — it intentionally
            // matches every SessionStart source. A group edited to add one
            // (e.g. narrowing to "resume") stops firing on startup/clear/
            // compact even though the inner command is untouched, so that
            // must not read as Installed.
            group.get("matcher").is_none()
                && hook.get("type").and_then(Value::as_str) == Some("command")
                && command == invocation.shell_command.as_str()
        });
```

Replace `probe`:

```rust
fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let mut state = FragmentState::Absent;
    for group in groups(document)? {
        let next = marker_state(&group, Some(reporter));
```

with:

```rust
fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let invocation = portable_reporter_invocation(reporter, MARKER)?;
    let mut state = FragmentState::Absent;
    for group in groups(document)? {
        let next = marker_state(&group, Some(&invocation));
```

(the rest of `probe`'s body is unchanged).

Replace `merge`'s invocation line:

```rust
    let invocation = reporter_invocation(reporter, MARKER);
    session_start.push(json!({"hooks":[{"type":"command","command":invocation.shell_command}]}));
```

with:

```rust
    let invocation = portable_reporter_invocation(reporter, MARKER)?;
    session_start.push(json!({"hooks":[{"type":"command","command":invocation.shell_command}]}));
```

`remove()` is unaffected — its call `marker_state(group, None)` type-checks against the new `Option<&ReporterInvocation>` parameter with no code change.

- [ ] **Step 4: Run the new tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::codex::tests -- --nocapture`
Expected: `merge_writes_a_home_relative_command_not_an_absolute_path` and `probe_reports_installed_after_merge_and_drifted_for_a_pre_portable_absolute_path_fragment` PASS. The four pre-existing tests now FAIL to compile (next step fixes them).

- [ ] **Step 5: Update the four pre-existing tests to use portable invocations**

Replace `marker_state_treats_a_foreign_harness_marker_as_ambiguous_not_drifted`:

```rust
    #[test]
    fn marker_state_treats_a_foreign_harness_marker_as_ambiguous_not_drifted() {
        // A stray Claude Code marker sitting alone in .codex/hooks.json (e.g.
        // copy-pasted by mistake) must never be treated as codex's own
        // fragment with a stale invocation — that would make install/
        // uninstall silently overwrite or delete a different harness's hook.
        let group = json!({
            "hooks": [
                {
                    "type": "command",
                    "command": "/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"
                }
            ]
        });
        let reporter = Path::new("/path/to/report-harness-event.sh");

        assert_eq!(marker_state(&group, Some(reporter)), FragmentState::Ambiguous);
    }
```

with:

```rust
    #[test]
    fn marker_state_treats_a_foreign_harness_marker_as_ambiguous_not_drifted() {
        // A stray Claude Code marker sitting alone in .codex/hooks.json (e.g.
        // copy-pasted by mistake) must never be treated as codex's own
        // fragment with a stale invocation — that would make install/
        // uninstall silently overwrite or delete a different harness's hook.
        // The ambiguity check runs before the exact-match check, so a
        // placeholder invocation is fine here — its content is never read.
        let group = json!({
            "hooks": [
                {
                    "type": "command",
                    "command": "/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"
                }
            ]
        });
        let invocation = ReporterInvocation {
            program: String::new(),
            args: vec![],
            shell_command: String::new(),
        };

        assert_eq!(
            marker_state(&group, Some(&invocation)),
            FragmentState::Ambiguous
        );
    }
```

Replace `merge_writes_session_start_nested_under_hooks_object_matching_the_real_codex_schema`'s body from `let reporter = ...` through the `merge(...)` call:

```rust
        let reporter = Path::new("/path/to/report-harness-event.sh");

        merge(&mut document, reporter).unwrap();
```

with:

```rust
        let home = HomeGuard::new();

        merge(&mut document, &home.reporter_path()).unwrap();
```

Replace `marker_state_reports_drifted_when_a_matcher_narrows_which_sources_fire`:

```rust
    #[test]
    fn marker_state_reports_drifted_when_a_matcher_narrows_which_sources_fire() {
        // merge() never sets "matcher" (it intentionally matches every
        // source). A group edited to add one, e.g. "matcher":"resume", no
        // longer fires on startup/clear/compact even though the inner
        // command is byte-for-byte what we generate — it must not be
        // reported Installed.
        let reporter = Path::new("/path/to/report-harness-event.sh");
        let invocation = reporter_invocation(reporter, MARKER);
        let group = json!({
            "matcher": "resume",
            "hooks": [
                {"type": "command", "command": invocation.shell_command}
            ]
        });

        assert_eq!(marker_state(&group, Some(reporter)), FragmentState::Drifted);
    }
```

with:

```rust
    #[test]
    fn marker_state_reports_drifted_when_a_matcher_narrows_which_sources_fire() {
        // merge() never sets "matcher" (it intentionally matches every
        // source). A group edited to add one, e.g. "matcher":"resume", no
        // longer fires on startup/clear/compact even though the inner
        // command is byte-for-byte what we generate — it must not be
        // reported Installed.
        let home = HomeGuard::new();
        let invocation = portable_reporter_invocation(&home.reporter_path(), MARKER).unwrap();
        let group = json!({
            "matcher": "resume",
            "hooks": [
                {"type": "command", "command": invocation.shell_command}
            ]
        });

        assert_eq!(
            marker_state(&group, Some(&invocation)),
            FragmentState::Drifted
        );
    }
```

`extract_marker_ignores_the_marker_text_appearing_outside_the_marker_flag` is unchanged — it doesn't touch `reporter_invocation` or `marker_state`.

- [ ] **Step 6: Run the full codex.rs test module to verify everything passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::codex::tests -- --nocapture`
Expected: all 6 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/orkworksd/src/harness/integrations/codex.rs
git commit -m "feat(sidecar): switch Codex hook merge/probe to portable reporter invocation"
```

---

### Task 4: Wire the relaxed safety check into `JsonHookHandler::load()`

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`

**Interfaces:**
- Consumes: `require_tracked_or_ignored_untracked` (Task 2), `ReporterPlatform::current()` (existing).
- Produces: no change to `JsonHookHandler`'s public shape — `status`/`install`/`uninstall` behavior only.

- [ ] **Step 1: Write the failing tests**

Add three new tests to `crates/orkworksd/src/harness/integrations/mod.rs`'s test module, after the existing `json_handler_conformance_matrix_preserves_unrelated_configuration` test (~line 769, before `codex_confirmation_does_not_claim_the_generic_attention_warning`):

```rust
    #[test]
    fn codex_install_succeeds_against_a_git_tracked_hooks_json() {
        let _guard = ENV_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(workspace.path()).unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        fs::write(workspace.path().join(".codex/hooks.json"), "{}").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(".codex/hooks.json")).unwrap();
        index.write().unwrap();

        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: home.path().join(".orkworks").join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: home.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };

        let status = handler(&IntegrationBinding::Codex).install(&context);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(
            status.unwrap().registration,
            IntegrationRegistration::Installed
        );
    }

    #[test]
    fn codex_install_still_refuses_an_untracked_unignored_hooks_json() {
        let _guard = ENV_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        // No .gitignore entry and nothing added to the index: untracked and
        // unignored — the relaxation must not widen this case.

        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: home.path().join(".orkworks").join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: home.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };

        let status = handler(&IntegrationBinding::Codex).status(&context).unwrap();

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(status.registration, IntegrationRegistration::Error);
        assert_eq!(status.diagnostics[0].code, "not_ignored_target");
    }

    #[test]
    fn codex_install_produces_byte_identical_content_regardless_of_which_machine_installed_it() {
        let _guard = ENV_LOCK.lock().unwrap();

        fn install_from_home(home_dir: &Path) -> Vec<u8> {
            let workspace = tempfile::tempdir().unwrap();
            let repo = git2::Repository::init(workspace.path()).unwrap();
            fs::create_dir(workspace.path().join(".codex")).unwrap();
            fs::write(workspace.path().join(".codex/hooks.json"), "{}").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(".codex/hooks.json")).unwrap();
            index.write().unwrap();

            let previous_home = std::env::var_os("HOME");
            std::env::set_var("HOME", home_dir);

            let assets = tempfile::tempdir().unwrap();
            fs::write(
                assets.path().join(ReporterPlatform::Posix.asset_name()),
                "#!/bin/sh\n",
            )
            .unwrap();
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: home_dir.join(".orkworks").join("hook-scripts"),
            };
            let context = IntegrationContext {
                workspace: workspace.path(),
                workspace_metadata: None,
                orkworks_root: home_dir,
                enabled: true,
                detected_tool: None,
                reporter_assets: &reporter,
            };
            handler(&IntegrationBinding::Codex)
                .install(&context)
                .unwrap();
            let bytes = fs::read(workspace.path().join(".codex/hooks.json")).unwrap();

            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            bytes
        }

        let alice = tempfile::tempdir().unwrap();
        let bob = tempfile::tempdir().unwrap();

        let alice_bytes = install_from_home(alice.path());
        let bob_bytes = install_from_home(bob.path());

        assert_eq!(
            alice_bytes, bob_bytes,
            "installed hook content must not depend on the installing machine's home directory"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::codex_install -- --nocapture`
Expected: `codex_install_succeeds_against_a_git_tracked_hooks_json` and `codex_install_produces_byte_identical_content_regardless_of_which_machine_installed_it` FAIL with `tracked_target` errors (the safety check isn't wired up yet). `codex_install_still_refuses_an_untracked_unignored_hooks_json` already passes today (no code change needed for that case) — that's expected and confirms the test itself is well-formed before the wiring change.

- [ ] **Step 3: Wire the branch into `load()`**

In `crates/orkworksd/src/harness/integrations/mod.rs`, replace:

```rust
    fn load(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<(ConfigFileTransaction, Map<String, Value>, PathBuf), IntegrationError> {
        let target = self.target(ctx)?;
        target.require_local_or_ignored_untracked()?;
```

with:

```rust
    fn load(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<(ConfigFileTransaction, Map<String, Value>, PathBuf), IntegrationError> {
        let target = self.target(ctx)?;
        // Codex is the only integration that can safely accept a
        // git-tracked target — its merge writes a $HOME-relative command
        // (portable_reporter_invocation, codex.rs) instead of an absolute
        // per-machine path, so a committed fragment is byte-identical
        // regardless of who installed it. POSIX only for now (ADR 0036);
        // Codex on Windows keeps the standard, stricter check.
        let portable_safe =
            self.contract.harness_id == "codex" && ReporterPlatform::current() == ReporterPlatform::Posix;
        if portable_safe {
            target.require_tracked_or_ignored_untracked()?;
        } else {
            target.require_local_or_ignored_untracked()?;
        }
```

(the rest of `load()` — `ConfigFileTransaction::open`, document parsing, `reporter` resolution — is unchanged).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::codex_install -- --nocapture`
Expected: all three PASS.

- [ ] **Step 5: Fix the two pre-existing tests whose reporter path is no longer under `dirs::home_dir()`**

The shared conformance-matrix test and the attention-warning test both build a `ReporterAssetResolver` whose `stable_dir` lives under an arbitrary tempdir unrelated to `dirs::home_dir()`. Codex's `probe`/`merge` now require the reporter path to resolve under home (Task 3), so both need `HOME` pointed at the same tempdir their `stable_dir` uses. This is harmless for Claude/Gemini/Copilot, which never call `dirs::home_dir()`.

In `json_handler_conformance_matrix_preserves_unrelated_configuration`, replace:

```rust
            let stable = tempfile::tempdir().unwrap();
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: stable.path().join("hook-scripts"),
            };
```

with:

```rust
            let stable = tempfile::tempdir().unwrap();
            // Codex's portable reporter path (Task 1) resolves against
            // dirs::home_dir(), so HOME must point at the same tempdir the
            // stable reporter directory lives under. Harmless for the other
            // three handlers, which never call dirs::home_dir().
            let previous_home = std::env::var_os("HOME");
            std::env::set_var("HOME", stable.path());
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: stable.path().join(".orkworks").join("hook-scripts"),
            };
```

and add, as the last statement inside the `for case in json_cases()` loop body (after the final assertion, currently `"{} round trip", case.name);` and its closing `);`, still inside the loop's braces):

```rust
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
```

Also wrap the whole loop body in the env lock by adding, as the very first statement inside `for case in json_cases() {`:

```rust
            let _guard = ENV_LOCK.lock().unwrap();
```

In `codex_confirmation_does_not_claim_the_generic_attention_warning`, replace:

```rust
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
```

with:

```rust
        let _guard = ENV_LOCK.lock().unwrap();
        let stable = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", stable.path());
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join(".orkworks").join("hook-scripts"),
        };
```

and add, as the last statement of the test function body (after the existing `claude_confirmation` assertions, before the closing `}`):

```rust
        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
```

- [ ] **Step 6: Run the full mod.rs test module to verify no regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests`
Expected: all PASS, including the two just-fixed tests and everything from Task 1.

- [ ] **Step 7: Commit**

```bash
git add crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "feat(sidecar): activate Codex hook install against git-tracked hooks.json"
```

---

### Task 5: Real-shaped fixture proving activation alongside APM's own tracked hooks

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–4. No new production code — test only.

This closes issue #276's explicit acceptance criterion ("verify it actually activates in a repo like this one... not just in a fresh tempdir fixture") without running the real install against this repo's actual live `.codex/hooks.json` — instead, the fixture is seeded with an equivalent shape: multiple pre-existing tracked hook groups (including `_apm_source: "ponytail"` groups), matching what `git show HEAD:.codex/hooks.json` in this repo actually contains.

- [ ] **Step 1: Write the failing test**

Add to `crates/orkworksd/src/harness/integrations/mod.rs`'s test module, after the tests added in Task 4:

```rust
    #[test]
    fn codex_install_activates_alongside_pre_existing_tracked_apm_hook_groups() {
        let _guard = ENV_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(workspace.path()).unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        // Shape mirrors this repo's real, APM-managed .codex/hooks.json:
        // Stop/sessionStart/userPromptSubmitted/SessionStart/UserPromptSubmit
        // groups tagged _apm_source: "ponytail", none touching OrkWorks'
        // marker. install() must add its own SessionStart group without
        // disturbing any of these.
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{
                    "hooks": [{
                        "type": "command",
                        "command": "ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd); bash \"$ROOT/.codex/hooks/doc-check-stop.sh\""
                    }]
                }],
                "sessionStart": [{
                    "type": "command",
                    "bash": "node \".codex/hooks/ponytail/hooks/ponytail-activate.js\"",
                    "powershell": "node \".codex/hooks/ponytail/hooks/ponytail-activate.js\"",
                    "timeoutSec": 5,
                    "_apm_source": "ponytail"
                }],
                "userPromptSubmitted": [{
                    "type": "command",
                    "bash": "node \".codex/hooks/ponytail/hooks/ponytail-mode-tracker.js\"",
                    "powershell": "node \".codex/hooks/ponytail/hooks/ponytail-mode-tracker.js\"",
                    "timeoutSec": 5,
                    "_apm_source": "ponytail"
                }],
                "SessionStart": [{
                    "matcher": "startup|resume|clear|compact",
                    "hooks": [{
                        "type": "command",
                        "command": "command -v node >/dev/null 2>&1 && node \".codex/hooks/ponytail/hooks/ponytail-activate.js\" || exit 0",
                        "timeout": 5,
                        "statusMessage": "Loading ponytail mode..."
                    }],
                    "_apm_source": "ponytail"
                }],
                "UserPromptSubmit": [{
                    "hooks": [{
                        "type": "command",
                        "command": "command -v node >/dev/null 2>&1 && node \".codex/hooks/ponytail/hooks/ponytail-mode-tracker.js\" || exit 0",
                        "timeout": 5,
                        "statusMessage": "Tracking ponytail mode..."
                    }],
                    "_apm_source": "ponytail"
                }]
            }
        });
        fs::write(
            workspace.path().join(".codex/hooks.json"),
            serde_json::to_vec_pretty(&existing).unwrap(),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(".codex/hooks.json")).unwrap();
        index.write().unwrap();

        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: home.path().join(".orkworks").join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: home.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };

        let status = handler(&IntegrationBinding::Codex).install(&context);

        let persisted: Value = status.as_ref().ok().and_then(|_| {
            fs::read(workspace.path().join(".codex/hooks.json"))
                .ok()
                .map(|bytes| serde_json::from_slice(&bytes).unwrap())
        }).unwrap_or(Value::Null);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(
            status.unwrap().registration,
            IntegrationRegistration::Installed
        );
        let hooks = persisted["hooks"].as_object().unwrap();
        assert!(hooks.contains_key("Stop"), "must preserve APM's Stop hook");
        assert!(
            hooks.contains_key("sessionStart"),
            "must preserve APM's lowercase sessionStart hook"
        );
        assert!(
            hooks.contains_key("userPromptSubmitted"),
            "must preserve APM's userPromptSubmitted hook"
        );
        assert!(
            hooks.contains_key("UserPromptSubmit"),
            "must preserve APM's UserPromptSubmit hook"
        );
        let session_start = hooks["SessionStart"].as_array().unwrap();
        assert_eq!(
            session_start.len(),
            2,
            "must add its own SessionStart group alongside APM's existing one, not replace it"
        );
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --lib harness::integrations::tests::codex_install_activates_alongside -- --nocapture`

Unlike the other tasks in this plan, this isn't a red/green TDD step — Tasks 1–4 already implemented the behavior this test exercises, so it's expected to PASS immediately. It exists to verify that behavior against a realistic multi-group tracked-file shape, not to drive new implementation.

- [ ] **Step 3: If it fails, fix forward**

A failure here is a real regression in Tasks 1–4's logic (most likely cause: `groups()`/`merge()` in `codex.rs` not tolerating the `sessionStart` vs `SessionStart` case-sensitivity, or the extra non-OrkWorks `SessionStart` entry's shape) — investigate and fix before moving on. Do not add new production code speculatively if it passes.

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "test(sidecar): verify Codex install activates alongside real APM-shaped tracked hooks"
```

---

### Task 6: ADR 0036 and documentation updates

**Files:**
- Create: `docs/adr/0036-codex-hooks-portable-reporter-path.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/adr/0035-codex-session-start-hook-not-attention-signal.md`
- Modify: `docs/agents/harness-integration-contracts.md`

- [ ] **Step 1: Write ADR 0036**

Create `docs/adr/0036-codex-hooks-portable-reporter-path.md`:

```markdown
# Codex hook installation uses a portable, home-relative reporter path

- Status: accepted
- Deciders: Lars-Erik, Claude Sonnet 5
- Date: 2026-08-04

## Context

ADR 0035 added a real Codex `SessionStart` hook integration targeting
project-level `.codex/hooks.json`, following the same
`require_local_or_ignored_untracked` safety rule every other JSON-hook
integration (Claude, Gemini, Copilot) uses. That rule assumes the target
file is local-only by convention — true for Claude's `settings.local.json`,
which has a genuine local/shared split (`settings.json` vs
`settings.local.json`). Codex has no such split: `.codex/hooks.json` is its
only hooks file. In any APM-managed repo, including this one, it's
deliberately git-tracked so APM's `ponytail` skill can install real
team-shared hooks there. The safety rule correctly refuses to write to a
tracked target, which left Codex's integration a permanent no-op in exactly
the repos it was built for — tracked as
[issue #276](https://github.com/Rambolarsen/orkworks/issues/276).

The actual hazard the safety rule guards against isn't sharing the file —
it's that every JSON-hook integration's reporter invocation bakes in the
resolved absolute path to `~/.orkworks/hook-scripts/report-harness-event.sh`,
which is per-machine. Writing that into a file every teammate shares means
whoever installs first commits their own home directory into version
control, and the next teammate's OrkWorks reads the fragment as `Drifted`,
not `Installed`. This repo's own committed `.codex/hooks.json` (installed by
APM) already demonstrates the alternative: every command APM writes there
resolves paths at shell-run time (`$ROOT=$(git rev-parse --show-toplevel)`,
repo-relative script references) rather than baking in whoever ran
`apm install`'s absolute path.

## Decision

- Codex's `merge`/`probe` (`crates/orkworksd/src/harness/integrations/codex.rs`)
  now build their hook command via a new `portable_reporter_invocation`
  (`crates/orkworksd/src/harness/integrations/mod.rs`), which rewrites the
  resolved reporter-script path as a `$HOME`-relative shell expression
  (e.g. `"$HOME/.orkworks/hook-scripts/report-harness-event.sh"`) instead of
  an absolute one. The committed command text is now byte-identical
  regardless of whose machine generated it.
- The existing `shell_quote`/`powershell_quote` helpers always single-quote,
  and single-quoted strings don't expand `$HOME` in POSIX shells.
  `portable_reporter_invocation` double-quotes the `$HOME`-relative path
  segment instead — safe because that segment is always a fixed,
  OrkWorks-authored suffix, never user input — while the marker argument
  keeps the existing single-quote escaping.
- `ValidatedWorkspaceTarget::require_local_or_ignored_untracked`
  (`crates/orkworksd/src/harness/integration.rs`) is split into a shared
  `require_confined_git_target(allow_tracked: bool)` helper. A new
  `require_tracked_or_ignored_untracked()` (Codex only) calls it with
  `allow_tracked: true`: a tracked target is now accepted, but an
  untracked-and-unignored one is still refused exactly as before — only the
  tracked case widens.
- `JsonHookHandler::load()` branches on `harness_id == "codex"` (matching
  the existing `is_attention_signal` special-case precedent from ADR 0035)
  to call the relaxed check instead of the standard one.
- This is POSIX-only. Whether Codex's `command` field is parsed by a shell
  that expands `$HOME` on Windows is unverified — `cmd.exe` doesn't, and an
  outer `powershell.exe`'s `-File` argument is not expression-evaluated the
  way inline script text is. Codex on Windows keeps writing an absolute
  path and is still refused on a tracked target: a known, pre-existing
  limitation, not a regression from this change.

## Consequences

- Codex's integration now actually activates in APM-managed repos (this one
  included), closing the gap ADR 0035 left open and closing issue #276.
- A pre-fix, absolute-path Codex fragment installed by an older OrkWorks
  version reads as `Drifted`, not silently `Installed`, once this version
  runs `probe` against it — the next install/reconcile replaces it with the
  portable form.
- Windows Codex support for the tracked-file case remains unresolved,
  tracked as follow-up work rather than solved speculatively here — the
  blocker is verifying which shell actually parses Codex's `command` field
  on Windows and whether `$HOME`/`$env:USERPROFILE` expansion reaches it,
  not a design decision this ADR can make from this repo alone.
- The portable/absolute split means Codex's reporter-invocation code path
  now differs from Claude/Gemini/Copilot's, which was intentional — those
  three have a real local-only file by convention (a tracked instance is a
  misconfiguration, correctly still refused) and don't need this.
```

- [ ] **Step 2: Update ADR 0035's Consequences section**

In `docs/adr/0035-codex-session-start-hook-not-attention-signal.md`, find the bullet beginning `**Unresolved**: project-level` inside the `## Consequences` section and replace it (keep the rest of the section, including the `codex.rs`'s probe/merge/remove duplication bullet, unchanged) with:

```markdown
- **Resolved by ADR 0036**: project-level `.codex/hooks.json` being tracked
  rather than local-only is handled by writing a portable, `$HOME`-relative
  reporter command instead of an absolute one, and relaxing the tracked-file
  safety check specifically for that portable-safe case. See
  [ADR 0036](0036-codex-hooks-portable-reporter-path.md).
```

- [ ] **Step 3: Add the new ADR to the index**

In `docs/adr/README.md`, add a row after the existing 0035 row:

```markdown
| [0036](./0036-codex-hooks-portable-reporter-path.md) | Codex hook installation uses a portable, home-relative reporter path | accepted |
```

- [ ] **Step 4: Update the Codex row in the evidence register**

In `docs/agents/harness-integration-contracts.md`, find the Codex row (starts `| Codex | [Hooks](https://learn.chatgpt.com/docs/hooks) |`). Replace the sentence starting `**Open concern**: this repo's own \`.codex/hooks.json\` is git-tracked...` through `...or whether Codex needs its own model (e.g. \`~/.codex/hooks.json\`, user-level).` with:

```markdown
**Resolved (ADR 0036)**: this repo's own `.codex/hooks.json` is git-tracked, not gitignored (APM installs shared team hooks there) — unlike Claude's `settings.local.json`, Codex has no separate local-only file convention. Rather than a global `~/.codex/hooks.json` fallback, the installer writes a `$HOME`-relative, machine-independent reporter command (POSIX only) so a committed fragment is byte-identical regardless of who installs it, and the `require_local_or_ignored_untracked` rule is relaxed specifically for that portable-safe case. Windows Codex support for the tracked case remains unresolved (unverified `$HOME` expansion semantics for Codex's hook shell on Windows).
```

- [ ] **Step 5: Commit**

```bash
git add docs/adr/0036-codex-hooks-portable-reporter-path.md docs/adr/README.md docs/adr/0035-codex-session-start-hook-not-attention-signal.md docs/agents/harness-integration-contracts.md
git commit -m "docs: add ADR 0036 for Codex's portable reporter path, close issue #276's open question"
```

---

### Task 7: Full validation and wrap-up

**Files:** none (verification only).

- [ ] **Step 1: Run the full sidecar test suite**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml && cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: builds cleanly, all tests PASS (this repo's suite was last confirmed at 549 tests before this plan — expect that count plus the ~11 tests added across Tasks 1, 2, 3, 4, and 5).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets`
Expected: no new warnings.

- [ ] **Step 3: Manually verify against this repo's real `.codex/hooks.json` shape**

This is the closest this plan can get to issue #276's "not just in a fresh tempdir fixture" acceptance criterion without mutating the real tracked file in this repo. Confirm Task 5's fixture test content actually matches the live file:

Run: `git show HEAD:.codex/hooks.json`

Compare its structure (group shapes, `_apm_source` markers, event names) against the fixture literal in `codex_install_activates_alongside_pre_existing_tracked_apm_hook_groups` (Task 5). If APM has changed the real file's shape since this plan was written, update the fixture to match before considering this task done.

- [ ] **Step 4: Run the doc currency and worktree checks**

Run: `bash .claude/hooks/doc-check.sh`
Run: `bash .claude/hooks/worktree-check.sh`
Expected: no unaddressed flags related to this change (ADR/docs updates from Task 6 should already satisfy doc-check's triggers for ADR and contracts-doc changes).

- [ ] **Step 5: Review the full diff**

Run: `git log --oneline origin/main..HEAD` and `git diff origin/main...HEAD --stat`
Confirm the diff only touches the files listed across Tasks 1–6, and that no unrelated file (e.g. this repo's real `.codex/hooks.json`) was modified.

- [ ] **Step 6: Open the PR**

Per `AGENTS.md`'s branch/PR workflow, this work (`apps/desktop`-free but touching `crates/orkworksd/`) requires a PR with a `/code-review` run before merge — default to lightweight effort given the change is scoped to one integration and doesn't touch concurrency, lifecycle, or schema/migration code. Push the branch and open the PR with `Closes #276` in the description (so merging auto-closes the issue), and note that Windows Codex support for the tracked case is an intentionally deferred follow-up (ADR 0036 Consequences), not an oversight.
