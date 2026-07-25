# Harness Tool Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Settings' "Detected" state real. Probe whether each harness's configured launch command resolves to an installed executable (on `PATH` or an absolute override path), feed that into `IntegrationContext.detected_tool` instead of the hardcoded `None`, and let the user fix a false negative by setting a custom absolute path on the Claude Code row.

**Architecture:** A new pure-function probe module (`crates/orkworksd/src/harness/detect.rs`) does a filesystem-only check (no subprocess) for either a `PATH` search or a direct absolute-path check. It's wired into the one production call site that builds `IntegrationContext`. The manual override reuses the harness definition's existing `launch.command` override mechanism (`PUT /harnesses/:id`) end to end — no new backend endpoint or persisted field. The frontend gets new Electron IPC methods and a "Custom path" input on the Claude Code Settings row (the only harness with integration UI today).

**Tech Stack:** Rust (axum, tokio), TypeScript/React (Electron main + preload + renderer). No new dependencies.

**Design doc:** `docs/superpowers/specs/2026-07-25-harness-tool-detection-design.md` (reviewed and fixed — see its commit history for what changed).

**Branch:** `harness-tool-detection` (already checked out).

---

## File Structure

- Create: `crates/orkworksd/src/harness/detect.rs` — the probe (`probe_installed_tool`, `windows_candidate_names`, `is_executable_file`).
- Modify: `crates/orkworksd/src/harness.rs` — register the new module.
- Modify: `crates/orkworksd/src/main.rs` — add a `FakePath` test-env guard and a `make_test_executable` test helper to `pub(crate) mod test_support`.
- Modify: `crates/orkworksd/src/harness/registry.rs` — extract `ResolvedHarness::launch_command()`, use it in `provider_from_harness`.
- Modify: `crates/orkworksd/src/http/integration_handlers.rs` — wire the probe into `run_integration_action`.
- Modify: `apps/desktop/src/harnessTypes.ts` — narrow `HarnessConfig.launch` from `unknown` to a real type.
- Modify: `apps/desktop/electron/preload.ts`, `apps/desktop/electron/main.ts` — add `setHarnessCommandOverride` / `clearHarnessCommandOverride` IPC.
- Modify: `apps/desktop/src/orkworksWindow.d.ts` — expose the two new methods on `window.orkworks`.
- Modify: `apps/desktop/src/components/SettingsModal.tsx` — add the Custom-path input + Clear button to the Claude Code row.

---

### Task 1: Add `FakePath` test guard and `make_test_executable` helper

**Files:**
- Modify: `crates/orkworksd/src/main.rs:270-298` (the `test_support` module, right after `FakeHome`'s `Drop` impl and before `with_fake_home`)

This is test infrastructure, not product behavior, so it's added directly rather than TDD'd against itself — later tasks' tests are what exercise it.

`FakePath` mirrors the existing `FakeHome` guard exactly (same `OnceLock<StdMutex<()>>` pattern for serializing concurrent env-var mutation across test threads), but **prepends** rather than replaces `PATH`. Replacing `PATH` wholesale would strip other concurrently-running tests in the same test binary of real system binaries (e.g. anything that shells out); prepending keeps a fake binary resolving first while leaving the real `PATH` intact for everyone else.

- [ ] **Step 1: Add `FakePath` and `make_test_executable` to `test_support`**

In `crates/orkworksd/src/main.rs`, immediately after the closing brace of `impl Drop for FakeHome` (currently ending at line 293) and before `pub(crate) fn with_fake_home`, insert:

```rust
    pub(crate) struct FakePath {
        previous: Option<std::ffi::OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl FakePath {
        /// Prepends `dir` to the real `PATH`, so a test can plant a fake
        /// executable that resolves first without stripping real system
        /// binaries other concurrently-running tests may need.
        pub(crate) fn prepend(dir: &std::path::Path) -> Self {
            static PATH_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
            let lock = PATH_LOCK.get_or_init(|| StdMutex::new(()));
            let _lock = lock.lock().unwrap();
            let previous = std::env::var_os("PATH");
            let mut dirs = vec![dir.to_path_buf()];
            if let Some(existing) = &previous {
                dirs.extend(std::env::split_paths(existing));
            }
            let joined =
                std::env::join_paths(dirs).expect("test PATH directories must be joinable");
            std::env::set_var("PATH", joined);
            Self { previous, _lock }
        }
    }

    impl Drop for FakePath {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("PATH", value),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    #[cfg(unix)]
    pub(crate) fn make_test_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(not(unix))]
    pub(crate) fn make_test_executable(_path: &std::path::Path) {}
```

- [ ] **Step 2: Confirm it compiles**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml --tests`
Expected: builds successfully. `dead_code` warnings for `FakePath`/`make_test_executable` are expected here — nothing calls them until Task 2 uses them. Not a blocker; there's no `-D warnings` gate in this crate's build.

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "test: add FakePath env guard and make_test_executable helper"
```

---

### Task 2: Detection probe (`detect.rs`) — TDD

**Files:**
- Create: `crates/orkworksd/src/harness/detect.rs`
- Modify: `crates/orkworksd/src/harness.rs:3-7` (register the module)

- [ ] **Step 1: Register the module**

In `crates/orkworksd/src/harness.rs`, the current module declarations are:

```rust
pub(crate) mod definition;
pub(crate) mod integration;
pub(crate) mod integrations;
pub(crate) mod registry;
pub(crate) mod store;
```

Change to (inserting `detect` in alphabetical position):

```rust
pub(crate) mod definition;
pub(crate) mod detect;
pub(crate) mod integration;
pub(crate) mod integrations;
pub(crate) mod registry;
pub(crate) mod store;
```

- [ ] **Step 2: Write the failing tests**

Create `crates/orkworksd/src/harness/detect.rs` with just the test module (the real function bodies come in Step 4):

```rust
//! Detects whether a harness's configured launch command resolves to an
//! installed, executable binary — either on `PATH` or at an absolute
//! override path.

use std::env;
use std::path::Path;

use super::integration::DetectedTool;

pub(crate) fn probe_installed_tool(_command: &str) -> Option<DetectedTool> {
    todo!()
}

fn windows_candidate_names(_command: &str, _pathext: &str) -> Vec<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakePath;

    #[test]
    fn empty_command_returns_none() {
        assert!(probe_installed_tool("").is_none());
    }

    #[test]
    fn absolute_path_that_does_not_exist_returns_none() {
        assert!(probe_installed_tool("/definitely/not/a/real/path/xyz").is_none());
    }

    #[test]
    fn bare_command_not_found_on_path_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let _fake_path = FakePath::prepend(dir.path());
        assert!(probe_installed_tool("definitely-not-a-real-binary-xyz").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn bare_command_found_and_executable_on_path_returns_detected_tool() {
        use crate::test_support::make_test_executable;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(dir.path());

        let detected = probe_installed_tool("toolfake").expect("should be detected");
        assert_eq!(detected.executable, bin);
        assert_eq!(detected.version, None);
        assert!(detected.compatible);
    }

    #[cfg(unix)]
    #[test]
    fn bare_command_found_but_not_executable_returns_none() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        // Deliberately leave default (non-executable) permissions.
        let _fake_path = FakePath::prepend(dir.path());

        assert!(probe_installed_tool("toolfake").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn absolute_path_that_exists_and_is_executable_returns_detected_tool() {
        use crate::test_support::make_test_executable;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);

        let detected =
            probe_installed_tool(bin.to_str().unwrap()).expect("should be detected");
        assert_eq!(detected.executable, bin);
    }

    #[cfg(unix)]
    #[test]
    fn absolute_path_that_exists_but_is_not_executable_returns_none() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();

        assert!(probe_installed_tool(bin.to_str().unwrap()).is_none());
    }

    #[test]
    fn windows_candidate_names_appends_each_pathext_extension() {
        assert_eq!(
            windows_candidate_names("claude", "EXE;CMD;BAT"),
            vec!["claude.exe", "claude.cmd", "claude.bat"]
        );
    }

    #[test]
    fn windows_candidate_names_does_not_double_append_a_known_extension() {
        assert_eq!(
            windows_candidate_names("claude.cmd", "EXE;CMD;BAT"),
            vec!["claude.cmd"]
        );
    }

    #[test]
    fn windows_candidate_names_falls_back_to_default_extensions_when_pathext_is_empty() {
        assert_eq!(
            windows_candidate_names("claude", ""),
            vec!["claude.exe", "claude.cmd", "claude.bat"]
        );
    }
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml harness::detect::tests`
Expected: compiles, then every test panics at the `todo!()` inside `probe_installed_tool`/`windows_candidate_names` (`not yet implemented`).

- [ ] **Step 4: Implement the probe**

Replace the two `todo!()` function bodies (keep everything else, including the `use` statements and the whole `mod tests` block, unchanged) with:

```rust
pub(crate) fn probe_installed_tool(command: &str) -> Option<DetectedTool> {
    if command.is_empty() {
        return None;
    }
    let path = Path::new(command);
    if path.is_absolute() {
        return is_executable_file(path).then(|| DetectedTool {
            executable: path.to_path_buf(),
            version: None,
            compatible: true,
        });
    }
    let path_var = env::var_os("PATH")?;
    let candidates = if cfg!(windows) {
        windows_candidate_names(command, &env::var("PATHEXT").unwrap_or_default())
    } else {
        vec![command.to_string()]
    };
    for dir in env::split_paths(&path_var) {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if is_executable_file(&full) {
                return Some(DetectedTool {
                    executable: full,
                    version: None,
                    compatible: true,
                });
            }
        }
    }
    None
}

/// Filenames to try for `command` in one `PATH` directory on Windows: the
/// bare name plus each `PATHEXT` extension, unless `command` already ends
/// in one of them. `pathext` is the raw `PATHEXT` env var value (semicolon
/// separated, e.g. `".COM;.EXE;.BAT"`); an empty/blank string falls back to
/// the common default `exe;cmd;bat`. This is a plain function (not gated
/// behind `cfg(windows)`) so its logic is unit-testable on any host — the
/// OS decision about whether to call it at all lives in
/// `probe_installed_tool` via `cfg!(windows)`.
fn windows_candidate_names(command: &str, pathext: &str) -> Vec<String> {
    let pathext = if pathext.trim().is_empty() {
        "exe;cmd;bat"
    } else {
        pathext
    };
    let extensions: Vec<String> = pathext
        .split(';')
        .map(|ext| ext.trim().trim_start_matches('.').to_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect();
    let lower_command = command.to_lowercase();
    if extensions
        .iter()
        .any(|ext| lower_command.ends_with(&format!(".{ext}")))
    {
        return vec![command.to_string()];
    }
    extensions
        .iter()
        .map(|ext| format!("{command}.{ext}"))
        .collect()
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(windows)]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}
```

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml harness::detect::tests`
Expected: all tests pass (on Linux/macOS: the `#[cfg(unix)]`-gated tests run too; the Windows-specific tests always run since `windows_candidate_names` has no `cfg` gate).

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/harness.rs crates/orkworksd/src/harness/detect.rs
git commit -m "feat: add PATH/absolute-path executable detection probe"
```

---

### Task 3: Extract `ResolvedHarness::launch_command()` (refactor, no behavior change)

**Files:**
- Modify: `crates/orkworksd/src/harness/registry.rs:75-76` (add method), `:408-411` (use it)

This is a pure extraction of already-tested logic — no new test is written for it; the existing registry test suite is the regression guard.

- [ ] **Step 1: Add the method**

In `crates/orkworksd/src/harness/registry.rs`, immediately after the closing `}` of `build_launch` (currently line 75, right before `pub(crate) fn augment_launch_for_integration`), insert:

```rust
    pub(crate) fn launch_command(&self) -> String {
        match &self.definition.launch {
            LaunchCapability::CommandTemplate { command, .. } => command.clone(),
            LaunchCapability::PlatformShell { .. } => String::new(),
        }
    }
```

- [ ] **Step 2: Use it in `provider_from_harness`**

Still in `crates/orkworksd/src/harness/registry.rs`, find:

```rust
    let command = command_override.unwrap_or_else(|| match &harness.definition.launch {
        super::definition::LaunchCapability::CommandTemplate { command, .. } => command.clone(),
        super::definition::LaunchCapability::PlatformShell { .. } => String::new(),
    });
```

Replace with:

```rust
    let command = command_override.unwrap_or_else(|| harness.launch_command());
```

- [ ] **Step 3: Run the existing registry tests to confirm no regression**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml harness::registry::tests`
Expected: all pass, unchanged from before the refactor.

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/harness/registry.rs
git commit -m "refactor: extract ResolvedHarness::launch_command from provider_from_harness"
```

---

### Task 4: Wire the probe into `IntegrationContext` — TDD

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs:104-112` (wiring), `:141-284` (tests)

- [ ] **Step 1: Write the failing test**

In `crates/orkworksd/src/http/integration_handlers.rs`, inside `mod tests` (after the last existing test, `install_rejects_malformed_existing_settings_file`, before the closing `}` of the module), add:

```rust
    #[tokio::test]
    async fn detected_tool_reflects_probe_result_for_a_resolvable_command() {
        use crate::test_support::{make_test_executable, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin = fake_bin_dir.path().join("claude");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }

    // This one already passes before the fix too (detected_tool was always
    // None, so activation was always forced to Unknown regardless of the
    // real command) — it's a regression guard for the genuinely-not-found
    // path, not a red/green driver. Kept alongside the test above for
    // coverage of both outcomes of the same wiring.
    //
    // It overrides claude-code's launch command to an unresolvable name
    // rather than relying on the ambient PATH lacking a real `claude`
    // binary: FakePath::prepend only prepends (by design — see Task 1), so
    // it can't hide a real `claude` install elsewhere on PATH, and this
    // test must still pass on a machine that has Claude Code installed.
    #[tokio::test]
    async fn detected_tool_stays_absent_when_the_command_is_not_on_path() {
        use crate::harness::definition::{HarnessPatch, LaunchPatch};
        use crate::test_support::FakePath;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "claude-code".to_string(),
                    HarnessPatch {
                        name: None,
                        launch: Some(LaunchPatch {
                            kind: None,
                            command: Some("definitely-not-a-real-binary-xyz".to_string()),
                            args: None,
                            model_prefix: None,
                            login: None,
                        }),
                        default_model: None,
                        resume: None,
                        models: None,
                        peon: None,
                        capacity: None,
                        session_signals: None,
                        integration: None,
                        voice: None,
                    },
                );
                Ok(())
            })
            .unwrap();

        let empty_bin_dir = tempfile::tempdir().unwrap();
        let _fake_path = FakePath::prepend(empty_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], false);
        assert_eq!(body["activation"], "unknown");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "tool_not_detected"));
    }
```

- [ ] **Step 2: Run the new test and confirm it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml integration_handlers::tests::detected_tool_reflects_probe_result_for_a_resolvable_command`
Expected: FAIL — `assert_eq!(body["toolDetected"], true)` fails because `toolDetected` is `false` (the hardcoded `None` wiring hasn't changed yet).

- [ ] **Step 3: Wire the probe in**

In `crates/orkworksd/src/http/integration_handlers.rs`, find:

```rust
    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled: true,
        detected_tool: None,
        reporter_assets: &reporter_assets,
    };
```

Replace with:

```rust
    let probed_tool = crate::harness::detect::probe_installed_tool(&harness.launch_command());

    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled: true,
        detected_tool: probed_tool.as_ref(),
        reporter_assets: &reporter_assets,
    };
```

- [ ] **Step 4: Run both new tests and confirm they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml integration_handlers::tests`
Expected: all tests in this module pass, including both new ones.

- [ ] **Step 5: Run the full Rust test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass, no regressions elsewhere.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets`
Expected: no new warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs
git commit -m "feat: wire the detection probe into IntegrationContext.detected_tool"
```

---

### Task 5: Frontend type — narrow `HarnessConfig.launch`

**Files:**
- Modify: `apps/desktop/src/harnessTypes.ts:1-14`

No test: this is a type-only change (TypeScript's structural typing has nothing to unit-test here); verified by the type-check step below.

- [ ] **Step 1: Add the `HarnessLaunch` type and use it**

In `apps/desktop/src/harnessTypes.ts`, replace:

```typescript
/** Mirrors crates/orkworksd/src/harness/definition.rs HarnessDefinition (v2, resolved-registry shape). */
export interface HarnessConfig {
  id: string;
  name: string;
  launch: unknown;
  defaultModel: string | null;
  resume: unknown;
  models: unknown;
  peon: unknown;
  capacity: unknown;
  sessionSignals: unknown;
  integration: unknown;
  voice: unknown;
}
```

with:

```typescript
/** Mirrors crates/orkworksd/src/harness/definition.rs LaunchCapability. */
export type HarnessLaunch =
  | { kind: "command-template"; command: string; args: string[]; modelPrefix: string | null }
  | { kind: "platform-shell"; login: boolean };

/** Mirrors crates/orkworksd/src/harness/definition.rs HarnessDefinition (v2, resolved-registry shape). */
export interface HarnessConfig {
  id: string;
  name: string;
  launch: HarnessLaunch;
  defaultModel: string | null;
  resume: unknown;
  models: unknown;
  peon: unknown;
  capacity: unknown;
  sessionSignals: unknown;
  integration: unknown;
  voice: unknown;
}
```

- [ ] **Step 2: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no new errors. (If any call site was pattern-matching `launch` as `unknown` with an `as` cast, this may surface it — none are expected today since nothing reads `launch` yet.)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/harnessTypes.ts
git commit -m "refactor: narrow HarnessConfig.launch from unknown to HarnessLaunch"
```

---

### Task 6: Electron IPC — `setHarnessCommandOverride` / `clearHarnessCommandOverride`

**Files:**
- Modify: `apps/desktop/electron/main.ts:337-369` (add handlers, next to `callIntegrationRoute`)
- Modify: `apps/desktop/electron/preload.ts:18-23` (expose them)
- Modify: `apps/desktop/src/orkworksWindow.d.ts:4, 23-25` (type them)

No dedicated unit test: `callIntegrationRoute` and the existing `install-harness-integration`/`uninstall-harness-integration` handlers it backs have none either (this class of thin IPC-to-fetch glue isn't covered by this repo's `node --test` suite — see `apps/desktop/tests/`, which has no `main.ts` test file). Verified by the type-check step and the manual browser check in Task 8.

- [ ] **Step 1: Add the handlers in `main.ts`**

In `apps/desktop/electron/main.ts`, immediately after the existing block:

```typescript
  ipcMain.handle("uninstall-harness-integration", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "uninstall"));
```

insert:

```typescript

  async function setHarnessCommandOverride(harnessId: unknown, commandPath: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    if (typeof commandPath !== "string" || !commandPath.trim()) throw new Error("Invalid command path.");
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "BuiltinPatch",
          patch: { launch: { command: commandPath } },
        }),
      });
      if (resp.ok) {
        return { ok: true, harness: await resp.json() };
      }
      const body = await resp.json().catch(() => ({ error: undefined }));
      return { ok: false, error: (body as { error?: string }).error ?? "Couldn't set the custom path." };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  async function clearHarnessCommandOverride(harnessId: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "DELETE",
      });
      if (resp.ok) {
        return { ok: true };
      }
      const body = await resp.json().catch(() => ({ error: undefined }));
      return { ok: false, error: (body as { error?: string }).error ?? "Couldn't clear the custom path." };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  ipcMain.handle("set-harness-command-override", async (_event, harnessId: unknown, commandPath: unknown) =>
    setHarnessCommandOverride(harnessId, commandPath));

  ipcMain.handle("clear-harness-command-override", async (_event, harnessId: unknown) =>
    clearHarnessCommandOverride(harnessId));
```

- [ ] **Step 2: Expose them in `preload.ts`**

In `apps/desktop/electron/preload.ts`, immediately after:

```typescript
  uninstallHarnessIntegration: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("uninstall-harness-integration", harnessId),
```

insert:

```typescript
  setHarnessCommandOverride: (harnessId: string, commandPath: string): Promise<unknown> =>
    ipcRenderer.invoke("set-harness-command-override", harnessId, commandPath),
  clearHarnessCommandOverride: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("clear-harness-command-override", harnessId),
```

- [ ] **Step 3: Type them in `orkworksWindow.d.ts`**

In `apps/desktop/src/orkworksWindow.d.ts`, change the import line:

```typescript
import type { IntegrationStatusResult } from "./harnessTypes";
```

to:

```typescript
import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes";
```

Then, immediately after:

```typescript
      uninstallHarnessIntegration: (harnessId: string) => Promise<IntegrationStatusResult>;
```

insert:

```typescript
      setHarnessCommandOverride: (
        harnessId: string,
        commandPath: string,
      ) => Promise<{ ok: true; harness: HarnessConfig } | { ok: false; error: string }>;
      clearHarnessCommandOverride: (
        harnessId: string,
      ) => Promise<{ ok: true } | { ok: false; error: string }>;
```

- [ ] **Step 4: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts
git commit -m "feat: add Electron IPC for setting/clearing a harness command override"
```

---

### Task 7: Settings UI — Custom-path input on the Claude Code row

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`

No dedicated component test exists for `SettingsModal` today (no test file references it; this repo doesn't have a React component-rendering test setup — see `apps/desktop/tests/`, which is Node's built-in test runner over plain logic modules). Verified by type-check plus the manual browser check in Task 8.

**Known limitation, stated explicitly rather than silently accepted:** after Save/Clear succeeds, this task re-fetches `claudeIntegration` status (so the "not detected" diagnostic and the Detected state update live), but it does **not** re-fetch the `harnesses` prop from the parent component. That means the Custom-path input's prefill and the Clear button's visibility (both derived from `harnesses`) won't reflect a just-saved override until the Settings modal is closed and reopened. This is acceptable for v1 — the functionally important state (whether the tool now reads as Detected) does update live.

- [ ] **Step 1: Add state, derived values, and handlers**

In `apps/desktop/src/components/SettingsModal.tsx`, change the import line:

```typescript
import type { HarnessConfig, IntegrationStatusResult } from "../harnessTypes";
```

(it already imports both types — no change needed there; confirm it matches).

Immediately after:

```typescript
  const [claudeIntegration, setClaudeIntegration] = useState<IntegrationStatusResult | null>(null);
  const [claudeIntegrationBusy, setClaudeIntegrationBusy] = useState(false);
  const hasClaudeCodeHarness = harnesses.some((h) => h.id === "claude-code");
```

insert:

```typescript
  const claudeHarness = harnesses.find((h) => h.id === "claude-code");
  const claudeLaunchCommand =
    claudeHarness?.launch.kind === "command-template" ? claudeHarness.launch.command : null;
  const claudeHasCustomPath = claudeLaunchCommand !== null && looksAbsolute(claudeLaunchCommand);
  const [customPathDraft, setCustomPathDraft] = useState<string>(() =>
    claudeHasCustomPath && claudeLaunchCommand ? claudeLaunchCommand : "",
  );
  const [customPathBusy, setCustomPathBusy] = useState(false);
  const [customPathError, setCustomPathError] = useState<string | null>(null);
```

Immediately after the existing `uninstallClaudeIntegrationHandler` function:

```typescript
  async function uninstallClaudeIntegrationHandler() {
    setClaudeIntegrationBusy(true);
    try {
      setClaudeIntegration(await window.orkworks.uninstallHarnessIntegration("claude-code"));
    } finally {
      setClaudeIntegrationBusy(false);
    }
  }
```

insert:

```typescript
  async function saveCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.setHarnessCommandOverride("claude-code", customPathDraft.trim());
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setClaudeIntegration(await window.orkworks.getHarnessIntegrationStatus("claude-code"));
    } finally {
      setCustomPathBusy(false);
    }
  }

  async function clearCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.clearHarnessCommandOverride("claude-code");
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathDraft("");
      setClaudeIntegration(await window.orkworks.getHarnessIntegrationStatus("claude-code"));
    } finally {
      setCustomPathBusy(false);
    }
  }
```

Finally, add the `looksAbsolute` helper as a module-level function (outside the component), near the top of the file after the existing type aliases (`type HotkeyAction = ...` / `type OllamaVerificationViewState = ...`):

```typescript
// Mirrors the sole direct-reference condition in the backend probe
// (crates/orkworksd/src/harness/detect.rs::probe_installed_tool): POSIX
// absolute (`/...`), Windows drive-letter (`C:\...` / `C:/...`), or UNC
// (`\\server\...`).
function looksAbsolute(command: string): boolean {
  return command.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(command) || command.startsWith("\\\\");
}
```

- [ ] **Step 2: Render the input**

In `apps/desktop/src/components/SettingsModal.tsx`, find the block ending with the existing diagnostics message:

```typescript
                      {claudeIntegration?.ok && claudeIntegration.status.diagnostics.length > 0 && (
                        <span className="settings-config-status">
                          {claudeIntegration.status.diagnostics[0].message}
                        </span>
                      )}
                    </div>
                  )}
                </div>
              ))}
          </div>
```

Insert a new conditional block right before the closing `</div>` that follows the diagnostics `span` (i.e. right after the `{claudeIntegration?.ok && claudeIntegration.status.diagnostics.length > 0 && (...)}` block, still inside the same `settings-config-item-actions` `<div>`). The condition is just "diagnostics contains `tool_not_detected`" — `.some()` on an empty array is already `false`, so no separate empty-check is needed:

```typescript
                      {claudeIntegration?.ok &&
                        claudeIntegration.status.diagnostics.some((d) => d.code === "tool_not_detected") && (
                          <div className="settings-config-custom-path">
                            <label>
                              Custom path
                              <input
                                type="text"
                                value={customPathDraft}
                                onChange={(e) => setCustomPathDraft(e.target.value)}
                                placeholder="/opt/homebrew/bin/claude"
                                disabled={customPathBusy}
                              />
                            </label>
                            <button
                              type="button"
                              onClick={saveCustomPathHandler}
                              disabled={customPathBusy || !customPathDraft.trim()}
                            >
                              {customPathBusy ? "Saving…" : "Save"}
                            </button>
                            {claudeHasCustomPath && (
                              <button type="button" onClick={clearCustomPathHandler} disabled={customPathBusy}>
                                Clear
                              </button>
                            )}
                            {customPathError && (
                              <span className="settings-config-status">{customPathError}</span>
                            )}
                          </div>
                        )}
                    </div>
                  )}
                </div>
              ))}
          </div>
```

- [ ] **Step 3: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx
git commit -m "feat: add custom-path override input to the Claude Code Settings row"
```

---

### Task 8: Full verification pass

**Files:** none (verification only)

- [ ] **Step 1: Full Rust suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all pass.

- [ ] **Step 2: Rust lint**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets`
Expected: no new warnings.

- [ ] **Step 3: Frontend type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Frontend test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all pass (no test in this suite exercises the new code directly, per the no-test notes in Tasks 6–7, but this confirms nothing else broke).

- [ ] **Step 5: Manual browser check**

Run: `cd apps/desktop && pnpm dev`

With Claude Code genuinely installed on `PATH`:
1. Open Settings → confirm the Claude Code row shows "✓ Notification hook installed" with no "not detected" warning.

With Claude Code temporarily hidden from `PATH` (e.g. rename the real binary or use a shell with a stripped `PATH` to launch the dev app):
2. Confirm the "not detected" diagnostic and the new "Custom path" input appear.
3. Enter the real absolute path to the `claude` binary, click Save — confirm the diagnostic clears and the row shows the normal installed/active state after the status re-fetch.
4. Reopen Settings — confirm the input is now prefilled with the saved path and a "Clear" button is visible.
5. Click Clear — confirm `PUT`/`DELETE` round-trip works and, after reopening Settings again, the row falls back to showing "not detected" (since `PATH` is still stripped in this test).

- [ ] **Step 6: Doc and worktree checks**

Run: `bash .claude/hooks/doc-check.sh`
Run: `bash .claude/hooks/worktree-check.sh`
Address anything flagged.

- [ ] **Step 7: Open the PR**

Per `AGENTS.md`, this touches `crates/orkworksd/` and `apps/desktop/`, so it needs a branch + PR with a `/code-review` pass before merge (lightweight is sufficient — no cross-cutting architecture, concurrency, or protocol/schema change here).

```bash
git push -u origin harness-tool-detection
gh pr create --title "Wire real harness tool detection into Settings" --body "$(cat <<'EOF'
## Summary
- Adds a PATH/absolute-path executable-detection probe and wires it into `IntegrationContext.detected_tool`, replacing the hardcoded `None` that made every harness always show "not detected" (issue #180, reopened).
- Adds a manual custom-path override on the Claude Code Settings row for when the automatic probe is a false negative (reuses the existing `PUT`/`DELETE /harnesses/:id` launch-command-override mechanism — no new backend endpoint).

## Test plan
- [x] `cargo test` (new `detect.rs` unit tests + `integration_handlers` wiring tests)
- [x] `cargo clippy`
- [x] `tsc --noEmit`
- [x] `node --test` frontend suite
- [ ] Manual: Claude Code row shows real Detected state; custom-path override round-trips end to end

Closes #180.
EOF
)"
```

---

## Self-Review Notes

**Spec coverage:** every Design-section item in the spec maps to a task — probe (Task 2), wiring (Task 4, with the shared-helper extraction from Task 3), manual override backend reuse (Task 6, no new endpoint as specified), frontend scope-limited-to-Claude-Code (Task 7, matching the spec's explicit scope note), and the spec's full Testing section (Tasks 2, 4, 8).

**Type consistency:** `probe_installed_tool`, `windows_candidate_names`, `is_executable_file`, `ResolvedHarness::launch_command`, `setHarnessCommandOverride`/`clearHarnessCommandOverride`, and `HarnessLaunch` are named and shaped identically everywhere they're declared vs. called across Tasks 2–7.

**No placeholders:** all code blocks are complete; the one `todo!()` usage (Task 2, Step 2) is the intentional TDD red-phase stub, replaced in Step 4 of the same task.
