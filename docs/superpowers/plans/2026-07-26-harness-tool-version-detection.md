# Harness Tool Version Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a harness definition declare a minimum required version, probe that version for real (spawning `<command> --version`) when such a harness is detected, and set `DetectedTool.compatible` from the comparison instead of the current hardcoded `true` — starting with Codex's hooks framework (v0.114), which issue #103's Codex adapter needs.

**Architecture:** A new optional `min_version` capability field on `HarnessDefinition`/`HarnessPatch` (matching the existing `peon`/`capacity` pattern). A new async `probe_tool_version` in `harness/detect.rs`, sitting alongside the existing sync `probe_installed_tool`, only invoked when a harness declares `min_version`. `http/integration_handlers.rs::run_integration_action` becomes `async fn` and is reordered so the version probe's `.await` never happens while either of its two lock guards (`state.workspace`, `state.harness_catalog`) is held — both guard types are `!Send`, so awaiting while either is in scope would not compile, and even if it did, it would serialize unrelated requests behind the probe's timeout.

**Tech Stack:** Rust, `tokio::process::Command` + `tokio::time::timeout` (both already available via the existing `tokio = { version = "1", features = ["full"] }` dependency — no new crates), `libc` (already a dependency) for one Unix-only process-liveness assertion in tests.

**Spec:** `docs/superpowers/specs/2026-07-26-harness-tool-version-detection-design.md` (reviewed and patched — read it for full rationale on each design decision below; this plan implements it as-is).

---

## File Structure

No new files. All changes land in four existing files, each already responsible for the layer it's being extended in:

- `crates/orkworksd/src/harness/definition.rs` — the `VersionRequirement` type and the `min_version` field on `HarnessDefinition`/`HarnessPatch`, following the exact pattern already used for `capacity`/`session_signals`.
- `crates/orkworksd/resources/harnesses-v2.json` — the one builtin entry (`codex`) that declares a `minVersion`.
- `crates/orkworksd/src/harness/store.rs` — one-line fix to `legacy_definition` so the crate keeps compiling once `HarnessDefinition` gains a new field.
- `crates/orkworksd/src/harness/detect.rs` — the new version-probing primitive, next to the existing PATH-probing primitive it complements.
- `crates/orkworksd/src/http/integration_handlers.rs` — the wiring: the only place `detect.rs` functions are called from, and the only place the lock-reordering fix applies.

---

### Task 1: Data model — `min_version` on `HarnessDefinition`/`HarnessPatch`

**Files:**
- Modify: `crates/orkworksd/src/harness/definition.rs` — `HarnessDefinition` struct (~line 9), `HarnessPatch` struct (~line 157), `HarnessPatch`'s manual `Deserialize` impl (~line 207), `apply_patch` (~line 393 onward), tests module (~line 696 onward). Locate each by the code shown in the steps below, not by line number — this file shifts as earlier steps land.
- Modify: `crates/orkworksd/resources/harnesses-v2.json:6` (codex entry)
- Modify: `crates/orkworksd/src/harness/store.rs:431-453` (`legacy_definition`)

- [ ] **Step 1: Write the failing test**

Add this test to the `tests` module in `crates/orkworksd/src/harness/definition.rs`, near the other `codex()`/`apply_patch` tests (e.g. right after `null_removes_only_optional_capabilities`):

```rust
#[test]
fn min_version_round_trips_through_serde_and_patch_and_the_codex_builtin_has_it_set() {
    // The codex builtin entry declares the hooks framework's minimum version.
    let definition = codex();
    assert_eq!(
        definition.min_version,
        Some(VersionRequirement { min: (0, 114, 0) })
    );

    // A sparse patch can set min_version on a harness that doesn't have one.
    let set_patch: HarnessPatch =
        serde_json::from_str(r#"{"minVersion":{"min":[1,2,3]}}"#).unwrap();
    let patched = definition.apply_patch(&set_patch).unwrap();
    assert_eq!(
        patched.min_version,
        Some(VersionRequirement { min: (1, 2, 3) })
    );

    // Explicit null clears it, same as every other optional capability.
    let clear_patch: HarnessPatch = serde_json::from_str(r#"{"minVersion":null}"#).unwrap();
    assert!(definition
        .apply_patch(&clear_patch)
        .unwrap()
        .min_version
        .is_none());

    // Omitting the field entirely leaves the builtin's min_version untouched.
    let noop_patch: HarnessPatch =
        serde_json::from_str(r#"{"name":"Configured Codex"}"#).unwrap();
    assert_eq!(
        definition.apply_patch(&noop_patch).unwrap().min_version,
        definition.min_version
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml min_version_round_trips`
Expected: compile failure — `VersionRequirement` doesn't exist yet and `HarnessDefinition`/`HarnessPatch` have no `min_version` field. A compile error is the correct RED state here (there's no way to write a runtime-failing test against types that don't exist yet).

- [ ] **Step 3: Add the `VersionRequirement` type and the `min_version` field**

In `crates/orkworksd/src/harness/definition.rs`, add this new struct near the other capability structs (e.g. right after `CapacityCapability`, around line 98):

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionRequirement {
    pub min: (u64, u64, u64),
}
```

Add the field to `HarnessDefinition` (line 9-23):

```rust
pub(crate) struct HarnessDefinition {
    pub id: String,
    pub name: String,
    pub launch: LaunchCapability,
    pub default_model: Option<String>,
    pub resume: Option<ResumeCapability>,
    pub models: Option<ModelCapability>,
    pub peon: Option<PeonCapability>,
    pub capacity: Option<CapacityCapability>,
    pub session_signals: Option<SessionSignalBinding>,
    pub integration: Option<IntegrationBinding>,
    pub voice: Option<VoiceCapability>,
    pub min_version: Option<VersionRequirement>,
}
```

Add the matching field to `HarnessPatch` (line 157-168):

```rust
pub(crate) struct HarnessPatch {
    pub name: Option<String>,
    pub launch: Option<LaunchPatch>,
    pub default_model: Option<Option<String>>,
    pub resume: Option<Option<ResumePatch>>,
    pub models: Option<Option<ModelCapability>>,
    pub peon: Option<Option<PeonPatch>>,
    pub capacity: Option<Option<CapacityCapability>>,
    pub session_signals: Option<Option<SessionSignalBinding>>,
    pub integration: Option<Option<IntegrationBinding>>,
    pub voice: Option<Option<VoicePatch>>,
    pub min_version: Option<Option<VersionRequirement>>,
}
```

- [ ] **Step 4: Wire `min_version` into `HarnessPatch`'s manual `Deserialize` impl**

`HarnessPatch` has a hand-written `Deserialize` (not `#[derive]`) that explicitly allowlists field names and reads each one. Find the `impl<'de> Deserialize<'de> for HarnessPatch` block and update both the allowlist and the field construction:

```rust
        reject_unknown_fields(
            &fields,
            &[
                "name",
                "launch",
                "defaultModel",
                "resume",
                "models",
                "peon",
                "capacity",
                "sessionSignals",
                "integration",
                "voice",
                "minVersion",
            ],
        )?;
        Ok(Self {
            name: required_patch_field(&fields, "name")?,
            launch: required_patch_field(&fields, "launch")?,
            default_model: optional_boundary_field(&fields, "defaultModel")?,
            resume: optional_boundary_field(&fields, "resume")?,
            models: optional_boundary_field(&fields, "models")?,
            peon: optional_boundary_field(&fields, "peon")?,
            capacity: optional_boundary_field(&fields, "capacity")?,
            session_signals: optional_boundary_field(&fields, "sessionSignals")?,
            integration: optional_boundary_field(&fields, "integration")?,
            voice: optional_boundary_field(&fields, "voice")?,
            min_version: optional_boundary_field(&fields, "minVersion")?,
        })
```

If you skip this step, the test in Step 1 fails at `serde_json::from_str::<HarnessPatch>(r#"{"minVersion":...}"#)` with `unknown patch field minVersion` (from `reject_unknown_fields`), not at compilation — a different, still-red failure mode worth recognizing if you see it.

- [ ] **Step 5: Wire `min_version` into `apply_patch`**

Find `impl HarnessDefinition { pub(crate) fn apply_patch(...) }` and add this alongside the other simple capability-replacement lines (`patch.capacity`, `patch.session_signals`, `patch.integration` — search for `if let Some(value) = &patch.capacity`):

```rust
        if let Some(value) = &patch.min_version {
            result.min_version = value.clone();
        }
```

- [ ] **Step 6: Add `minVersion` to the codex builtin entry**

In `crates/orkworksd/resources/harnesses-v2.json`, find the `codex` entry (starts with `{ "id": "codex", ...`) and add `"minVersion": { "min": [0, 114, 0] }` before its closing `}`:

```
old: ..."integration": { "kind": "codex" }, "voice": null },
new: ..."integration": { "kind": "codex" }, "voice": null, "minVersion": { "min": [0, 114, 0] } },
```

Every other entry in that file (`gemini`, `aider`, `copilot`, `generic-shell`, and the `legacySnapshots` array) is untouched — they have no `minVersion` key, which deserializes to `min_version: None`.

- [ ] **Step 7: Fix the other three places that construct `HarnessDefinition`/`HarnessPatch` literally**

Adding a field to a struct breaks every exhaustive (non-`..Default::default()`) literal of that struct, not just the one in `apply_patch`. Three more exist:

`crates/orkworksd/src/harness/store.rs:426-454`, function `legacy_definition`, builds a `HarnessDefinition` field-by-field for pre-v2 migrated entries. Add `min_version`, falling back to the safe builtin adapter's value the same way `capacity` already does:

```rust
        capacity: safe_adapter.and_then(|definition| definition.capacity.clone()),
        session_signals: None,
        integration: None,
        voice: legacy_voice(&entry.capabilities),
        min_version: safe_adapter.and_then(|definition| definition.min_version.clone()),
    }
```

(Insert the new line right before the closing `}` of the struct literal, after `voice:`.)

`crates/orkworksd/src/harness/store.rs:322-348`, function `legacy_patch`, builds a `HarnessPatch` field-by-field for the same migration path. Legacy entries never carry a version requirement, so this is always `None`:

```rust
        capacity: None,
        session_signals: None,
        integration: None,
        voice: legacy_voice_patch(&entry.capabilities, &baseline.capabilities),
        min_version: None,
    }
```

(Insert the new line right before the closing `}`, after `voice:`.)

`crates/orkworksd/src/http/integration_handlers.rs:341-358` (inside the existing test `detected_tool_stays_absent_when_the_command_is_not_on_path`) builds a `HarnessPatch` field-by-field to override `claude-code`'s launch command. Add the same `None`:

```rust
                        default_model: None,
                        resume: None,
                        models: None,
                        peon: None,
                        capacity: None,
                        session_signals: None,
                        integration: None,
                        voice: None,
                        min_version: None,
                    },
```

(Insert the new line right before the closing `}` of the `HarnessPatch { ... }` literal, after `voice: None,`.)

Skipping any of these three produces `error[E0063]: missing field `min_version`` — a crate-wide compile failure, not a scoped one, since `cargo test` compiles the whole lib regardless of which module's tests you're targeting.

- [ ] **Step 8: Run the test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml min_version_round_trips`
Expected: PASS. Also run the full `definition.rs` test module to make sure nothing else broke:

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd harness::definition::`
Expected: all PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/orkworksd/src/harness/definition.rs crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/store.rs
git commit -m "Add min_version capability field to harness definitions (#227)

Declarative, per-harness minimum-version requirement, following the
existing peon/capacity optional-capability pattern. Codex's builtin
entry declares 0.114.0 (its hooks framework's minimum version); no
other harness declares one yet. Nothing consumes this field until
the version probe (next commit) is wired in."
```

---

### Task 2: Version probe and parser in `detect.rs`

**Files:**
- Modify: `crates/orkworksd/src/harness/detect.rs` (top-of-file imports, new functions, tests module)

- [ ] **Step 1: Write the failing tests**

Add near the top of `crates/orkworksd/src/harness/detect.rs`, after the existing `use` lines:

```rust
use std::time::Duration;
```

Add these tests to the `#[cfg(test)] mod tests` block at the bottom of the file (after the existing `windows_candidate_names_*` tests):

```rust
    #[test]
    fn parse_version_token_finds_a_three_component_version() {
        assert_eq!(parse_version_token("codex-cli 0.114.2\n"), Some((0, 114, 2)));
    }

    #[test]
    fn parse_version_token_accepts_a_missing_patch_component() {
        assert_eq!(parse_version_token("gemini version 2.9\n"), Some((2, 9, 0)));
    }

    #[test]
    fn parse_version_token_picks_the_first_numeric_token_in_a_noisy_banner() {
        // Known, accepted limitation (documented in the design spec): this is
        // a generic heuristic, not a real parser, so an unrelated leading
        // numeric token (a bundled dependency version here) wins over the
        // actual tool version later in the same banner.
        assert_eq!(
            parse_version_token("built against libfoo 6.2.1, tool 0.114.2"),
            Some((6, 2, 1))
        );
    }

    #[test]
    fn parse_version_token_returns_none_for_output_with_no_numeric_token() {
        assert_eq!(parse_version_token("no version information available"), None);
    }

    #[test]
    fn parse_version_token_returns_none_for_empty_output() {
        assert_eq!(parse_version_token(""), None);
    }

    #[tokio::test]
    async fn probe_tool_version_returns_combined_stdout_and_stderr() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fake-tool");
        std::fs::write(&bin, "#!/bin/sh\necho 'fake-tool 1.2.3'\n").unwrap();
        make_test_executable(&bin);

        let output = probe_tool_version(&bin).await.expect("should run");
        assert_eq!(parse_version_token(&output), Some((1, 2, 3)));
    }

    #[tokio::test]
    async fn probe_tool_version_returns_empty_text_for_a_silent_nonzero_exit() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fake-tool");
        std::fs::write(&bin, "#!/bin/sh\nexit 1\n").unwrap();
        make_test_executable(&bin);

        // .output() succeeds regardless of exit code — it only fails if the
        // process can't be spawned at all. A silent nonzero exit is a
        // parse-failure case for the *caller*, not a spawn failure here.
        let output = probe_tool_version(&bin).await;
        assert_eq!(output, Some(String::new()));
        assert_eq!(parse_version_token(&output.unwrap()), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_tool_version_kills_a_hanging_binary_when_the_timeout_fires() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("hangs-forever");
        let pidfile = dir.path().join("child.pid");
        // `exec` replaces the shell process image with `sleep`, so there is
        // exactly one process for kill_on_drop's SIGKILL to reach — without
        // `exec`, `sh` would fork `sleep` as a *grandchild* that
        // kill_on_drop's signal to the immediate child (`sh`) would never
        // reach, leaking an orphaned `sleep` even with the fix in place.
        std::fs::write(
            &bin,
            format!("#!/bin/sh\necho $$ > {}\nexec sleep 30\n", pidfile.display()),
        )
        .unwrap();
        make_test_executable(&bin);

        let start = std::time::Instant::now();
        let result = probe_tool_version(&bin).await;
        assert!(result.is_none(), "a timed-out probe must not return output");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "must not wait past the 3s timeout"
        );

        let pid: i32 = std::fs::read_to_string(&pidfile)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // SIGKILL delivery/reaping is asynchronous — tokio reaps the killed
        // child via a background task on the *same* current-thread runtime
        // this test runs on, so the poll must yield with `tokio::time::sleep`
        // rather than block the thread with `std::thread::sleep`; blocking
        // here would starve that reaping task and make this loop spin until
        // timeout even though the kill succeeded.
        for _ in 0..20 {
            if unsafe { libc::kill(pid, 0) } != 0 {
                return; // ESRCH: process no longer exists.
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("child process {pid} is still alive after kill_on_drop should have killed it");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd harness::detect::`
Expected: compile failure — `parse_version_token` and `probe_tool_version` don't exist yet.

- [ ] **Step 3: Implement `parse_version_token`**

Add to `crates/orkworksd/src/harness/detect.rs`, above the `#[cfg(test)]` module:

```rust
/// Extracts the first `major.minor[.patch]`-shaped token from arbitrary CLI
/// output. A generic heuristic, not a real version parser — see the design
/// spec for the accepted limitations (e.g. a noisy banner with an unrelated
/// leading numeric token). A missing patch component defaults to 0.
pub(crate) fn parse_version_token(text: &str) -> Option<(u64, u64, u64)> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    for token in tokens {
        let parts: Vec<&str> = token.split('.').filter(|part| !part.is_empty()).collect();
        if parts.len() < 2 || parts.len() > 3 {
            continue;
        }
        let Some(numbers) = parts
            .iter()
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<u64>>>()
        else {
            continue;
        };
        return Some((numbers[0], numbers[1], numbers.get(2).copied().unwrap_or(0)));
    }
    None
}
```

- [ ] **Step 4: Implement `probe_tool_version`**

Add directly above `parse_version_token`:

```rust
/// Runs `<executable> --version`, bounded to 3 seconds, and returns the
/// combined stdout+stderr text (some CLIs print version info to stderr;
/// non-UTF8 bytes are lossily converted rather than treated as an error).
/// Returns `None` on any spawn error or timeout. `.output()` drains both
/// streams concurrently, so combining them here doesn't risk a
/// pipe-buffer deadlock.
pub(crate) async fn probe_tool_version(executable: &Path) -> Option<String> {
    let mut command = tokio::process::Command::new(executable);
    command.arg("--version").kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(3), command.output())
        .await
        .ok()?
        .ok()?;
    Some(
        String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr),
    )
}
```

`kill_on_drop(true)` matters specifically because of the timeout: when `tokio::time::timeout` fires, the `command.output()` future is dropped mid-flight without ever completing, and without this flag the spawned child would be orphaned rather than killed.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd harness::detect::`
Expected: all PASS, including the two pre-existing test groups (`windows_candidate_names_*`, `probe_installed_tool` tests) — this task must not change `probe_installed_tool` at all.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/harness/detect.rs
git commit -m "Add async version probe and parser to detect.rs (#227)

probe_tool_version spawns <command> --version with a 3s timeout and
kill_on_drop to avoid orphaning a hanging binary. parse_version_token
is a generic major.minor[.patch] extractor with documented, accepted
limitations rather than a real version-output parser. Neither is
wired into any call site yet — that's the next commit."
```

---

### Task 3: Wire the probe into `run_integration_action`, fixing the lock/await ordering

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs` — `run_integration_action` and its three callers (locate by the function/handler names and the code shown below, not by line number — Task 1's edits elsewhere don't shift this file, but exact current line numbers are still easy to get stale), tests module

This is the task the independent design review flagged as the real risk: `run_integration_action` today holds a `MutexGuard` (`state.workspace.lock()`) and an `RwLockReadGuard` (`state.harness_catalog.read()`) across its entire body. Both are `!Send`. Awaiting the version probe anywhere in that body while either guard is alive would either fail to compile (axum requires `Send` handler futures) or, if it somehow did compile, serialize every other workspace-touching request behind the probe's timeout. The fix reorders the function so no guard is alive across the one `.await` point, while preserving today's exact error-priority order (a request with both a missing workspace *and* an unknown harness ID still reports "no workspace" first, not "not found" — that ordering is preserved by re-checking the workspace after the probe rather than moving the harness lookup ahead of the workspace check).

- [ ] **Step 1: Write the failing test for version gating end-to-end**

Add to the `tests` module in `crates/orkworksd/src/http/integration_handlers.rs`, near `init_git_workspace_with_claude_settings_ignored`:

```rust
    fn init_git_workspace_with_copilot_settings_ignored(workspace: &std::path::Path) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(
            workspace.join(".gitignore"),
            ".github/copilot/settings.local.json\n",
        )
        .unwrap();
    }
```

Then add these two tests (near `detected_tool_reflects_probe_result_for_a_resolvable_command`). Use `copilot`, not `gemini`: discovered while running this step, `gemini.rs`'s own `ToolHookContract` declares `activation: IntegrationActivation::Unknown` even in its fully-installed, fully-detected steady state (its coverage is `Limited` by pre-existing design) — so a "fully active" assertion against gemini can never pass, for reasons unrelated to `min_version`. Copilot and Claude both declare `activation: IntegrationActivation::Active`; copilot is used here to keep this test independent of the Claude-code-specific tests already in this file.

```rust
    #[tokio::test]
    async fn min_version_gating_marks_a_below_threshold_binary_as_needing_trust() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "needs_trust");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "unsupported_tool_version"));
    }

    #[tokio::test]
    async fn min_version_gating_leaves_an_above_threshold_binary_fully_active() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        // Active (as opposed to Absent/Disabled) only applies once the hook
        // is actually Installed — status_from_document's activation match
        // only reaches self.contract.activation via the `registration ==
        // Installed` arm, so an install must happen first, exactly like the
        // pre-existing Claude reference test this one is modeled on
        // (detected_tool_reflects_probe_result_for_a_resolvable_command).
        let install_response =
            install_integration(State(state.clone()), AxumPath("copilot".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd http::integration_handlers::tests::min_version_gating`
Expected: both tests FAIL (not compile-fail — `HarnessPatch.min_version` and the detect.rs functions already exist from Tasks 1-2). They fail because `run_integration_action` never calls `probe_tool_version` yet, so `compatible` stays hardcoded `true` regardless of the declared `min_version` — the below-threshold test expects `"needs_trust"` but gets `"active"`.

- [ ] **Step 3: Reorder and async-ify `run_integration_action`**

Replace the whole `run_integration_action` function in `crates/orkworksd/src/http/integration_handlers.rs` (match it by its signature and body shown below, not by line number):

```rust
async fn run_integration_action(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(&ResolvedHarness, &IntegrationContext<'_>) -> Result<
        crate::harness::integration::IntegrationStatus,
        IntegrationError,
    >,
) -> axum::response::Response {
    // Checked first (and dropped immediately) to preserve today's exact
    // error-priority order: a request against a missing workspace reports
    // NoWorkspace even if harness_id is also unknown. Re-checked again below
    // after the version probe, since a lock held only long enough to check
    // "does a workspace exist" leaves a window (however small) for a
    // concurrent request to clear it before this request re-acquires the
    // lock for real use.
    {
        let ws_guard = state.workspace.lock().unwrap();
        if ws_guard.is_none() {
            return integration_error_response(IntegrationError::NoWorkspace);
        }
    }

    let harness: ResolvedHarness = {
        let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
        let Some(harness) = registry.get(harness_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: format!("unknown harness id \"{harness_id}\"") }),
            )
                .into_response();
        };
        harness.clone()
    };

    // No lock guard is held from here through the `.await` below — this is
    // the fix for the bug the original draft of this design had: both
    // guards above are `!Send`, and the workspace mutex must not be held
    // for the probe's full timeout, which other workspace-touching requests
    // would otherwise queue behind.
    let probed_tool = crate::harness::detect::probe_installed_tool(&harness.launch_command());
    let detected_tool = match (probed_tool, harness.definition.min_version.as_ref()) {
        (Some(mut tool), Some(requirement)) => {
            match crate::harness::detect::probe_tool_version(&tool.executable).await {
                Some(output) => match crate::harness::detect::parse_version_token(&output) {
                    Some(parsed) => {
                        tool.compatible = parsed >= requirement.min;
                        tool.version = Some(output);
                    }
                    None => {
                        tool.compatible = false;
                        tool.version = None;
                    }
                },
                None => {
                    tool.compatible = false;
                    tool.version = None;
                }
            }
            Some(tool)
        }
        (probed_tool, _) => probed_tool,
    };

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return integration_error_response(IntegrationError::NoWorkspace);
    };

    let reporter_assets = match reporter_assets() {
        Ok(resolver) => resolver,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error })).into_response();
        }
    };

    let orkworks_root = match dirs::home_dir() {
        Some(home) => home.join(".orkworks"),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "couldn't resolve home directory".into() }),
            )
                .into_response();
        }
    };

    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled: true,
        detected_tool: detected_tool.as_ref(),
        reporter_assets: &reporter_assets,
    };

    match action(&harness, &ctx) {
        Ok(status) => Json(status).into_response(),
        Err(error) => integration_error_response(error),
    }
}
```

- [ ] **Step 4: Await the now-async function at its three call sites**

Still in `crates/orkworksd/src/http/integration_handlers.rs`, update each of the three handlers immediately below `run_integration_action` to add `.await`:

```rust
pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_status(ctx)).await
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_install(ctx)).await
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_uninstall(ctx)).await
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd http::integration_handlers::`
Expected: all PASS — the two new tests from Step 1, and every pre-existing test in this file (`status_reports_absent_for_a_fresh_workspace`, `install_then_status_reports_installed`, `install_then_uninstall_reports_absent`, `status_without_a_workspace_returns_conflict`, `status_for_an_unknown_harness_id_returns_not_found`, `install_rejects_malformed_existing_settings_file`, `detected_tool_reflects_probe_result_for_a_resolvable_command`, `detected_tool_stays_absent_when_the_command_is_not_on_path`) must be unaffected — they exercise harnesses with no `min_version` declared, so the probe branch never triggers for them.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs
git commit -m "Wire version probe into run_integration_action (#227)

run_integration_action becomes async and is reordered so no lock
guard (state.workspace's MutexGuard, state.harness_catalog's
RwLockReadGuard) is held across the version probe's await point —
both are !Send, and even setting that aside, holding the workspace
mutex for the probe's timeout would have serialized unrelated
requests behind it. The harness is cloned out of the registry before
the probe runs; the workspace lock is re-acquired afterward,
preserving today's error-priority order (missing-workspace still
wins over unknown-harness-id when both are true).

Closes the loop on #227: JsonHookHandler's NeedsTrust/
unsupported_tool_version branch (harness/integrations/mod.rs,
status_from_document) was dead code until now — this is the first
real producer of DetectedTool.compatible."
```

- [ ] **Step 7: Write and run the concurrency regression test**

This test is written *after* the implementation, not before — unlike the tests in Steps 1-2, it isn't driving new behavior; it's pinning a property (no lock held across the await) that only becomes meaningful once that behavior exists. Written before Step 3's implementation, a "hangs forever" fake binary would never actually be invoked with `--version` (nothing called `probe_tool_version` yet), so the test would pass for the wrong reason — not a real RED state.

Add to the same `tests` module:

```rust
    #[tokio::test]
    async fn a_slow_version_probe_does_not_block_a_concurrent_workspace_request() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        // `exec` matters here exactly as it does in detect.rs's kill-on-
        // timeout test: without it, `sh` forks `sleep` as a grandchild that
        // kill_on_drop's signal never reaches, leaking it independently of
        // (and in addition to) whatever detect.rs's own test covers — that
        // test only exercises its own spawned binary, not this one.
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into())).await.into_response()
        });
        // Give the slow request a head start into its probe before firing
        // the concurrent one.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let start = std::time::Instant::now();
        let concurrent_response =
            get_integration_status(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        let elapsed = start.elapsed();

        assert_eq!(concurrent_response.status(), StatusCode::OK);
        // 2s of margin below the probe's 3s timeout: generous enough to
        // absorb scheduling jitter on a loaded CI runner while still being a
        // meaningful signal that the concurrent request didn't queue behind
        // the slow one. If this proves flaky in practice, widen the margin
        // rather than add retry logic — a single fixed threshold is enough
        // signal for what this test is checking.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "a concurrent request must not wait behind the slow probe's 3s timeout, took {elapsed:?}"
        );

        // Clean up: let the slow request finish so the test doesn't leak a
        // background task past its own scope.
        let slow_response = slow_request.await.unwrap();
        assert_eq!(slow_response.status(), StatusCode::OK);
    }
```

The code above uses `std::time::Duration::...` fully-qualified rather than a bare `Duration`, so no new `use` import is needed in the `tests` module.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd http::integration_handlers::tests::a_slow_version_probe`
Expected: PASS, completing in a small fraction of the slow request's ~3s timeout. If this test hangs or takes >3s, the lock-reordering fix in Step 3 has a bug — a guard is still being held across the await.

- [ ] **Step 8: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs
git commit -m "Add concurrency regression test for the lock-reordering fix (#227)

Pins that a slow/hanging version probe on one request does not block
a concurrent request that touches the same workspace mutex. Written
after the implementation rather than before: before Step 3 existed,
a hanging fake binary was never actually invoked with --version, so
this test would have passed for the wrong reason."
```

---

### Task 4: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests PASS, including the full `harness::` and `http::` modules and everything else in the crate (this change touches shared types like `HarnessDefinition`/`HarnessPatch` and a widely-called function, so a full run — not just the touched modules — is the real bar here).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets -- -D warnings`
Expected: no warnings. Pay particular attention to anything clippy flags in the reordered `run_integration_action` (e.g. needless clone lints) — if clippy suggests removing the `harness.clone()`, don't: it's load-bearing for the lock-lifetime fix, not incidental.

- [ ] **Step 3: Confirm no unrelated files changed**

Run: `git status --porcelain=v1`
Expected: clean (everything from Tasks 1-3 already committed) or only the files this plan named.

At this point the branch is ready for the standard OrkWorks finishing steps (not part of this plan): push, open a PR, run `/code-review` (required for any change under `crates/orkworksd/`, per `AGENTS.md`), and run the doc-currency/worktree-currency checks before wrap-up.
