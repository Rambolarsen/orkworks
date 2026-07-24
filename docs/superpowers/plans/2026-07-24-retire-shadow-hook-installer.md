# Retire the Shadow Hook Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the unsafe, live `crates/orkworksd/src/http/hook_handlers.rs` Claude-only hook installer with the existing-but-unwired ADR-0026 integration stack (`harness/integration.rs`'s `JsonHookHandler`/`ValidatedWorkspaceTarget`/`ConfigFileTransaction`), exposed through new generic, harness-parameterized HTTP routes, and author the reporter script the new system depends on (which does not exist yet).

**Architecture:** Three new routes (`GET/POST/POST /workspace/integrations/:harness_id/{status,install,uninstall}`) replace the two Claude-only routes. A new `crates/orkworksd/scripts/report-harness-event.sh` is invoked by the installed hook, branching on the `--marker` argument's harness suffix to preserve Claude's session-ID capture while staying generic. The renderer's Settings UI cuts over from a boolean `installed` flag to the richer `IntegrationStatus` shape, gaining an Uninstall button it never had. Gemini/Copilot UI and a Windows `.ps1` script are out of scope (tracked as issues #217 and #218).

**Tech Stack:** Rust (axum, serde_json), TypeScript/React (Electron preload/main, SettingsModal.tsx), POSIX shell.

---

## Before you start

This branch touches `apps/desktop/src`, `apps/desktop/electron`, and `crates/orkworksd` — per `AGENTS.md` it requires a branch + PR (not direct-to-`main`), and a `/code-review` pass before merge (this qualifies for at least the lightweight tier; consider medium given it touches a security-relevant workspace-mutation seam and the IPC/Electron boundary).

Use the `starting-work` skill to set up your branch/worktree. Branch name: `retire-shadow-hook-installer`.

```bash
git worktree add ../orkworks-retire-shadow-hook-installer -b retire-shadow-hook-installer
cd ../orkworks-retire-shadow-hook-installer
cd apps/desktop && pnpm install && cd ../..
```

All file paths below are relative to the repo root of that worktree.

---

### Task 1: Author `report-harness-event.sh`

**Files:**
- Create: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs` (add tests to the existing `#[cfg(test)] mod tests` block)

The new integration system (`ReporterPlatform::asset_name()`, `harness/integrations/mod.rs:40-45`) expects to reconcile and invoke a script named `report-harness-event.sh`, called as `report-harness-event.sh --marker <marker>` with the hook's JSON payload on stdin. This script does not exist anywhere in the repo today. It must: always POST the generic `waiting_for_input` attention signal, and additionally POST Claude's `harness-session` capture only when the `--marker` value's suffix is `claude-code`.

- [ ] **Step 1: Create a placeholder script so the crate still compiles**

`include_str!` is evaluated at compile time, so the file must exist before we can write a test against its contents.

```bash
cat > crates/orkworksd/scripts/report-harness-event.sh << 'EOF'
#!/usr/bin/env bash
set -u
EOF
chmod +x crates/orkworksd/scripts/report-harness-event.sh
```

- [ ] **Step 2: Write the failing tests**

Open `crates/orkworksd/src/harness/integrations/mod.rs`, find the `#[cfg(test)] mod tests {` block, and add these four tests near the top of the module (right after the `use super::*;` / existing `use` lines inside the test module):

```rust
    #[test]
    fn report_harness_event_always_posts_generic_attention() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/attention"));
        assert!(script.contains("\"status\":\"waiting_for_input\""));
    }

    #[test]
    fn report_harness_event_captures_claude_session_id_only_for_claude_marker() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("claude-code"));
        assert!(script.contains("session_id"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"claude_hook\""));
    }

    #[test]
    fn report_harness_event_bounds_every_curl_with_a_timeout() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        let max_time_count = script.matches("--max-time").count();
        assert_eq!(
            max_time_count, 2,
            "both possible curl calls must cap their own runtime so a stuck orkworksd cannot \
             hang the harness's own hook mechanism"
        );
    }

    #[test]
    fn report_harness_event_parses_the_marker_flag() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("--marker"));
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml report_harness_event -- --nocapture`
Expected: all 4 tests FAIL — the two-line placeholder (`#!/usr/bin/env bash` / `set -u`) contains none of `ORKWORKS_SESSION_ID`, `claude-code`, `--max-time`, or `--marker`.

- [ ] **Step 4: Write the real script**

```bash
cat > crates/orkworksd/scripts/report-harness-event.sh << 'EOF'
#!/usr/bin/env bash
set -u

marker=""
while [ $# -gt 0 ]; do
  case "$1" in
    --marker)
      if [ $# -ge 2 ]; then
        marker="$2"
        shift 2
      else
        # No value follows; drop the flag alone so the loop still terminates.
        shift 1
      fi
      ;;
    *)
      shift
      ;;
  esac
done

observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"
payload="$(cat || true)"

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ]; then
  attention_payload="$(python3 -c 'import json,sys; print(json.dumps({"status":"waiting_for_input","observedAt":sys.argv[1]}))' "$observed_at")"
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d "$attention_payload" >/dev/null || true
fi

case "$marker" in
  *:claude-code)
    claude_session_id="$(
      printf '%s' "$payload" |
        python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("session_id") or "")' 2>/dev/null ||
        true
    )"
    if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ -n "$claude_session_id" ]; then
      escaped_session_id=$(printf '%s' "$claude_session_id" | sed 's/[\\"]/\\&/g')
      session_payload=$(printf '{"harnessSessionId":"%s","source":"claude_hook","confidence":0.98}' "$escaped_session_id")
      curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
        -H "Content-Type: application/json" \
        -d "$session_payload" >/dev/null || true
    fi
    ;;
esac
EOF
chmod +x crates/orkworksd/scripts/report-harness-event.sh
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml report_harness_event -- --nocapture`
Expected: PASS (4 passed)

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/scripts/report-harness-event.sh crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "feat(harness): author report-harness-event.sh reporter script"
```

---

### Task 2: `ResolvedHarness::integration_install` / `integration_uninstall`

**Files:**
- Modify: `crates/orkworksd/src/harness/registry.rs:102-118` (existing `integration_status`), add two new methods after it
- Test: `crates/orkworksd/src/harness/registry.rs` (existing `#[cfg(test)] mod tests` block)

`ResolvedHarness::integration_status` already exists and falls back to `generic_shell_status` for harnesses with no `integration` binding (e.g. `generic-shell`). `install`/`uninstall` need the same shape.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `crates/orkworksd/src/harness/registry.rs` (after the existing `use` lines):

```rust
    fn test_reporter_assets() -> (tempfile::TempDir, tempfile::TempDir, crate::harness::integration::ReporterAssetResolver) {
        use crate::harness::integrations::ReporterPlatform;

        let assets = tempfile::tempdir().unwrap();
        std::fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        std::fs::write(
            assets.path().join(ReporterPlatform::WindowsPowerShell.asset_name()),
            "# noop\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let resolver = crate::harness::integration::ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        (assets, stable, resolver)
    }

    #[test]
    fn integration_install_and_uninstall_round_trip_for_claude_code() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let user = HarnessUserDocument::default();
        let resolved = resolve_document(&builtins, &user).unwrap();
        let claude = resolved.get("claude-code").unwrap();

        let workspace = tempfile::tempdir().unwrap();
        // Claude's handler is a JsonHookHandler, which refuses to touch a
        // config file outside a Git workspace where the target is ignored
        // (see `ValidatedWorkspaceTarget::require_local_or_ignored_untracked`
        // in harness/integration.rs) — without this, install/uninstall below
        // return `UnsafeTarget { code: "not_git_workspace" }` and `.unwrap()` panics.
        git2::Repository::init(workspace.path()).unwrap();
        std::fs::write(
            workspace.path().join(".gitignore"),
            ".claude/settings.local.json\n",
        )
        .unwrap();
        let (_assets, _stable, reporter_assets) = test_reporter_assets();
        let orkworks_root = tempfile::tempdir().unwrap();
        let ctx = crate::harness::integration::IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: orkworks_root.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter_assets,
        };

        let installed = claude.integration_install(&ctx).unwrap();
        assert_eq!(
            installed.registration,
            crate::harness::integration::IntegrationRegistration::Installed
        );

        let uninstalled = claude.integration_uninstall(&ctx).unwrap();
        assert_eq!(
            uninstalled.registration,
            crate::harness::integration::IntegrationRegistration::Absent
        );
    }

    #[test]
    fn integration_install_on_a_harness_with_no_binding_is_a_no_op() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let user = HarnessUserDocument::default();
        let resolved = resolve_document(&builtins, &user).unwrap();
        let shell = resolved.get("generic-shell").unwrap();

        let workspace = tempfile::tempdir().unwrap();
        let (_assets, _stable, reporter_assets) = test_reporter_assets();
        let orkworks_root = tempfile::tempdir().unwrap();
        let ctx = crate::harness::integration::IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: orkworks_root.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter_assets,
        };

        let status = shell.integration_install(&ctx).unwrap();
        assert_eq!(
            status.registration,
            crate::harness::integration::IntegrationRegistration::Unsupported
        );
        let status = shell.integration_uninstall(&ctx).unwrap();
        assert_eq!(
            status.registration,
            crate::harness::integration::IntegrationRegistration::Unsupported
        );
    }
```

Confirmed: `generic_shell_status` (`harness/integrations/mod.rs:325-346`) returns `IntegrationRegistration::Unsupported`, matching the assertion above.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml integration_install_and_uninstall integration_install_on_a_harness -- --nocapture`
Expected: FAIL with "no method named `integration_install` found"

- [ ] **Step 3: Add the two methods**

In `crates/orkworksd/src/harness/registry.rs`, find the existing method (around line 102-118):

```rust
    #[allow(dead_code)] // Read by generic integration routes in Task 8.
    pub(crate) fn integration_status(
        &self,
        ctx: &crate::harness::integration::IntegrationContext<'_>,
    ) -> Result<
        crate::harness::integration::IntegrationStatus,
        crate::harness::integration::IntegrationError,
    > {
        match &self.definition.integration {
            Some(binding) => crate::harness::integration::handler(binding).status(ctx),
            None => Ok(crate::harness::integrations::generic_shell_status(
                ctx.workspace,
                ctx.enabled,
                ctx.detected_tool.is_some(),
            )),
        }
    }
```

Replace it with (dropping the now-stale `#[allow(dead_code)]` — this is wired in Task 3 — and adding the two new methods):

```rust
    pub(crate) fn integration_status(
        &self,
        ctx: &crate::harness::integration::IntegrationContext<'_>,
    ) -> Result<
        crate::harness::integration::IntegrationStatus,
        crate::harness::integration::IntegrationError,
    > {
        match &self.definition.integration {
            Some(binding) => crate::harness::integration::handler(binding).status(ctx),
            None => Ok(crate::harness::integrations::generic_shell_status(
                ctx.workspace,
                ctx.enabled,
                ctx.detected_tool.is_some(),
            )),
        }
    }

    pub(crate) fn integration_install(
        &self,
        ctx: &crate::harness::integration::IntegrationContext<'_>,
    ) -> Result<
        crate::harness::integration::IntegrationStatus,
        crate::harness::integration::IntegrationError,
    > {
        match &self.definition.integration {
            Some(binding) => crate::harness::integration::handler(binding).install(ctx),
            None => Ok(crate::harness::integrations::generic_shell_status(
                ctx.workspace,
                ctx.enabled,
                ctx.detected_tool.is_some(),
            )),
        }
    }

    pub(crate) fn integration_uninstall(
        &self,
        ctx: &crate::harness::integration::IntegrationContext<'_>,
    ) -> Result<
        crate::harness::integration::IntegrationStatus,
        crate::harness::integration::IntegrationError,
    > {
        match &self.definition.integration {
            Some(binding) => crate::harness::integration::handler(binding).uninstall(ctx),
            None => Ok(crate::harness::integrations::generic_shell_status(
                ctx.workspace,
                ctx.enabled,
                ctx.detected_tool.is_some(),
            )),
        }
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml integration_install_and_uninstall integration_install_on_a_harness -- --nocapture`
Expected: PASS (2 passed)

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/harness/registry.rs
git commit -m "feat(harness): add ResolvedHarness::integration_install/uninstall"
```

---

### Task 3: New `http/integration_handlers.rs`

**Files:**
- Create: `crates/orkworksd/src/http/integration_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs:3-7` (register the new module, remove `hook_handlers`)

This is the replacement for `hook_handlers.rs`: three generic routes, error-code mapping, and — for the first time in production code — a real `ReporterAssetResolver` (mirroring the packaged-vs-dev script location logic the old `hook_handlers.rs::claude_hook_script_path` used, generalized to a directory).

- [ ] **Step 1: Write the failing tests**

Create `crates/orkworksd/src/http/integration_handlers.rs` with just the imports and a `#[cfg(test)]` module (no handlers yet), so the tests fail to compile against missing functions:

```rust
use crate::harness::integration::{IntegrationContext, IntegrationError, ReporterAssetResolver};
use crate::harness::registry::ResolvedHarness;
use crate::http::ErrorResponse;
use crate::AppState;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_app_state_with_workspace, FakeHome};
    use serde_json::Value;

    // Claude's handler is a JsonHookHandler: `require_local_or_ignored_untracked`
    // (harness/integration.rs) refuses to read or write its config file unless
    // the workspace is a Git repository with that file gitignored. Every test
    // below that reaches Claude's handler must set this up first, or it hits
    // `UnsafeTarget { code: "not_git_workspace" }` (mapped to 400) instead of
    // the behavior the test name describes — `status()` swallows that into an
    // `"error"` registration, and `install`/`uninstall` return it as an error.
    fn init_git_workspace_with_claude_settings_ignored(workspace: &std::path::Path) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(workspace.join(".gitignore"), ".claude/settings.local.json\n").unwrap();
    }

    #[tokio::test]
    async fn status_reports_absent_for_a_fresh_workspace() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
    }

    #[tokio::test]
    async fn install_then_status_reports_installed() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response = install_integration(State(state.clone()), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let status_response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(status_response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "installed");
    }

    #[tokio::test]
    async fn install_then_uninstall_reports_absent() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response = install_integration(State(state.clone()), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);
        let uninstall_response = uninstall_integration(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(uninstall_response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(uninstall_response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
    }

    #[tokio::test]
    async fn status_without_a_workspace_returns_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        *state.workspace.lock().unwrap() = None;

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn status_for_an_unknown_harness_id_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("not-a-real-harness".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn install_rejects_malformed_existing_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.local.json"), "not json").unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = install_integration(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml --tests 2>&1 | head -30`
Expected: compile FAIL with "cannot find function `get_integration_status`" (and similarly for `install_integration`/`uninstall_integration`)

- [ ] **Step 3: Implement the handlers**

Append to `crates/orkworksd/src/http/integration_handlers.rs` (above the `#[cfg(test)]` block):

```rust
fn resolve_scripts_source_dir(exe_dir: Option<PathBuf>, manifest_dir: &Path) -> PathBuf {
    if let Some(dir) = exe_dir {
        let packaged = dir.join("scripts");
        if packaged.is_dir() {
            return packaged;
        }
    }
    manifest_dir.join("scripts")
}

fn scripts_source_dir() -> PathBuf {
    resolve_scripts_source_dir(
        std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn stable_hook_scripts_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("hook-scripts"))
}

fn reporter_assets() -> Result<ReporterAssetResolver, String> {
    let stable_dir = stable_hook_scripts_dir()
        .ok_or_else(|| "couldn't resolve home directory for the reporter scripts".to_string())?;
    Ok(ReporterAssetResolver { source_dir: scripts_source_dir(), stable_dir })
}

fn integration_error_response(error: IntegrationError) -> axum::response::Response {
    let status = match &error {
        IntegrationError::NoWorkspace
        | IntegrationError::RevisionChanged
        | IntegrationError::OwnershipAmbiguous => StatusCode::CONFLICT,
        IntegrationError::UnsafeTarget { .. } | IntegrationError::InvalidConfig(_) => {
            StatusCode::BAD_REQUEST
        }
        IntegrationError::LaunchConflict | IntegrationError::Io(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    let message = match &error {
        IntegrationError::NoWorkspace => "Open a workspace first.".to_string(),
        IntegrationError::UnsafeTarget { message, .. } => message.clone(),
        IntegrationError::InvalidConfig(message) => message.clone(),
        IntegrationError::OwnershipAmbiguous => {
            "This integration's config entry doesn't match what OrkWorks installed; resolve it manually.".to_string()
        }
        IntegrationError::LaunchConflict => "Unexpected launch conflict.".to_string(),
        IntegrationError::RevisionChanged => "Configuration changed; retry the request.".to_string(),
        IntegrationError::Io(error) => error.to_string(),
    };
    (status, Json(ErrorResponse { error: message })).into_response()
}

fn run_integration_action(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(&ResolvedHarness, &IntegrationContext<'_>) -> Result<
        crate::harness::integration::IntegrationStatus,
        IntegrationError,
    >,
) -> axum::response::Response {
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return integration_error_response(IntegrationError::NoWorkspace);
    };

    let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
    let Some(harness) = registry.get(harness_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("unknown harness id \"{harness_id}\"") }),
        )
            .into_response();
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
        detected_tool: None,
        reporter_assets: &reporter_assets,
    };

    match action(harness, &ctx) {
        Ok(status) => Json(status).into_response(),
        Err(error) => integration_error_response(error),
    }
}

pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_status(ctx))
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_install(ctx))
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_uninstall(ctx))
}
```

- [ ] **Step 4: Register the module**

In `crates/orkworksd/src/http/mod.rs`, replace:

```rust
pub(crate) mod harness_handlers;
pub(crate) mod hook_handlers;
pub(crate) mod provider_handlers;
```

with:

```rust
pub(crate) mod harness_handlers;
pub(crate) mod integration_handlers;
pub(crate) mod provider_handlers;
```

This will not compile yet — `main.rs` still imports `hook_handlers`. That's fixed in Task 4; for now just confirm `cargo check` fails only on the `main.rs` import, not inside `integration_handlers.rs` itself:

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml --tests 2>&1 | head -40`
Expected: errors reference `crate::http::hook_handlers` in `main.rs`, not errors inside `integration_handlers.rs`

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs crates/orkworksd/src/http/mod.rs
git commit -m "feat(harness): add generic /workspace/integrations/:harness_id HTTP handlers"
```

---

### Task 4: Wire routes, delete `hook_handlers.rs`

**Files:**
- Modify: `crates/orkworksd/src/main.rs:31-44` (imports), `:190-197` (production route registration), `:473-480` (test-only `test_router` route registration)
- Delete: `crates/orkworksd/src/http/hook_handlers.rs`

- [ ] **Step 1: Update imports in `main.rs`**

Replace:

```rust
use crate::http::harness_handlers::{
    create_harness, delete_harness, list_harnesses, update_harness,
};
use crate::http::hook_handlers::{get_attention_hook_status, install_attention_hook};
```

with:

```rust
use crate::http::harness_handlers::{
    create_harness, delete_harness, list_harnesses, update_harness,
};
use crate::http::integration_handlers::{
    get_integration_status, install_integration, uninstall_integration,
};
```

- [ ] **Step 2: Update route registration**

Replace:

```rust
        .route(
            "/workspace/attention-hook/status",
            get(get_attention_hook_status),
        )
        .route(
            "/workspace/attention-hook/install",
            post(install_attention_hook),
        )
```

with:

```rust
        .route(
            "/workspace/integrations/:harness_id/status",
            get(get_integration_status),
        )
        .route(
            "/workspace/integrations/:harness_id/install",
            post(install_integration),
        )
        .route(
            "/workspace/integrations/:harness_id/uninstall",
            post(uninstall_integration),
        )
```

- [ ] **Step 3: Update the duplicate route table in `main.rs`'s own test module**

`main.rs` has a second, hand-maintained route table — `fn test_router(state: Arc<AppState>) -> Router` inside its `#[cfg(test)] mod tests` block (around line 459) — used by the HTTP-level tests in that same module (e.g. `session_routes_remain_registered_with_current_methods_and_paths`). It references `get_attention_hook_status`/`install_attention_hook` via the `use super::*;` import at the top of `mod tests`, so once Step 1's import is replaced, this table stops compiling unless it's updated too.

Replace (inside `test_router`, around line 473-480):

```rust
            .route(
                "/workspace/attention-hook/status",
                get(get_attention_hook_status),
            )
            .route(
                "/workspace/attention-hook/install",
                post(install_attention_hook),
            )
```

with:

```rust
            .route(
                "/workspace/integrations/:harness_id/status",
                get(get_integration_status),
            )
            .route(
                "/workspace/integrations/:harness_id/install",
                post(install_integration),
            )
            .route(
                "/workspace/integrations/:harness_id/uninstall",
                post(uninstall_integration),
            )
```

- [ ] **Step 4: Delete the old handler file**

```bash
rm crates/orkworksd/src/http/hook_handlers.rs
```

- [ ] **Step 5: Run the full test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -40`
Expected: PASS, with the count of passing tests roughly unchanged (14 old `hook_handlers.rs` tests removed, replaced by Task 1's 4 tests + Task 2's 2 tests + Task 3's 6 tests — a net decrease of 2, consolidated behind the shared `JsonHookHandler`/`ConfigFileTransaction` machinery that already has its own coverage in `harness/integrations/mod.rs`)

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/main.rs
git rm crates/orkworksd/src/http/hook_handlers.rs
git commit -m "refactor(harness): retire hook_handlers.rs in favor of generic integration routes"
```

---

### Task 5: Renderer types

**Files:**
- Modify: `apps/desktop/src/harnessTypes.ts`

- [ ] **Step 1: Replace `AttentionHookStatusResponse` with `IntegrationStatus` types**

Replace the current tail of `apps/desktop/src/harnessTypes.ts`:

```ts
export interface AttentionHookStatusResponse {
  installed: boolean;
  error?: string;
}
```

with:

```ts
export type IntegrationRegistration = "unsupported" | "absent" | "installed" | "drifted" | "error";
export type IntegrationOwnership = "none" | "ork_works" | "ambiguous";
export type IntegrationActivation = "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
export type IntegrationCoverage = "full" | "limited" | "none";

export interface IntegrationDiagnostic {
  code: string;
  message: string;
  action?: string;
}

export interface IntegrationConfirmation {
  toolName: string;
  workspaceLabel: string;
  coverageSummary: string;
  relativePaths: string[];
  executableCodeWarning: boolean;
}

export interface IntegrationStatus {
  harnessId: string;
  enabled: boolean;
  toolDetected: boolean;
  registration: IntegrationRegistration;
  ownership: IntegrationOwnership;
  activation: IntegrationActivation;
  coverage: IntegrationCoverage;
  diagnostics: IntegrationDiagnostic[];
  confirmation: IntegrationConfirmation | null;
}

export type IntegrationStatusResult =
  | { ok: true; status: IntegrationStatus }
  | { ok: false; error: string };
```

These mirror the Rust types exactly: `IntegrationStatus` (`crates/orkworksd/src/harness/integration.rs:377-389`), its enums (`:341-375`), `IntegrationDiagnostic` (`:391-397`), `IntegrationConfirmation` (`:399-407`) — all `#[serde(rename_all = "camelCase")]`/`"snake_case"`. `IntegrationStatusResult` is a renderer-only envelope (not a Rust type) matching this codebase's `SaveHotkeysResult`-style `{ ok, ... }` pattern for reporting IPC-relay failures (network errors, non-2xx responses) uniformly.

Note on `IntegrationOwnership::OrkWorks`: serde's `snake_case` variant-rename rule inserts an underscore before every internal uppercase letter rather than detecting word boundaries, so this specific variant serializes as `"ork_works"`, not `"orkworks"`. Verified directly: a standalone `#[derive(Serialize)] #[serde(rename_all = "snake_case")] enum { OrkWorks }` serializes `OrkWorks` to `"ork_works"`. The other enums above only have single-word or already-conventional multi-word variants (e.g. `NeedsTrust` → `"needs_trust"`, `NotApplicable` → `"not_applicable"`) where this doesn't bite.

- [ ] **Step 2: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit 2>&1 | head -30`
Expected: errors only in `orkworksWindow.d.ts` and `SettingsModal.tsx` (both still reference `AttentionHookStatusResponse`/old preload methods) — fixed in the next two tasks. No errors inside `harnessTypes.ts` itself.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/harnessTypes.ts
git commit -m "feat(desktop): add IntegrationStatus renderer types"
```

---

### Task 6: Preload + window type bridge

**Files:**
- Modify: `apps/desktop/electron/preload.ts:18-19`
- Modify: `apps/desktop/src/orkworksWindow.d.ts:4,23-24`

- [ ] **Step 1: Update `preload.ts`**

Replace:

```ts
  getClaudeCodeHookStatus: (): Promise<unknown> => ipcRenderer.invoke("get-claude-code-hook-status"),
  installClaudeCodeHook: (): Promise<unknown> => ipcRenderer.invoke("install-claude-code-hook"),
```

with:

```ts
  getHarnessIntegrationStatus: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("get-harness-integration-status", harnessId),
  installHarnessIntegration: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("install-harness-integration", harnessId),
  uninstallHarnessIntegration: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("uninstall-harness-integration", harnessId),
```

- [ ] **Step 2: Update `orkworksWindow.d.ts`**

Replace the import line:

```ts
import type { AttentionHookStatusResponse } from "./harnessTypes";
```

with:

```ts
import type { IntegrationStatusResult } from "./harnessTypes";
```

Replace:

```ts
      getClaudeCodeHookStatus: () => Promise<AttentionHookStatusResponse>;
      installClaudeCodeHook: () => Promise<AttentionHookStatusResponse>;
```

with:

```ts
      getHarnessIntegrationStatus: (harnessId: string) => Promise<IntegrationStatusResult>;
      installHarnessIntegration: (harnessId: string) => Promise<IntegrationStatusResult>;
      uninstallHarnessIntegration: (harnessId: string) => Promise<IntegrationStatusResult>;
```

- [ ] **Step 3: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit 2>&1 | head -30`
Expected: errors only in `electron/main.ts` (still has the old `ipcMain.handle` names, unused now) and `SettingsModal.tsx` — fixed in the next two tasks.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts
git commit -m "feat(desktop): expose harness integration IPC bridge in preload"
```

---

### Task 7: Electron main process IPC handlers

**Files:**
- Modify: `apps/desktop/electron/main.ts:337-368`

- [ ] **Step 1: Replace the two old `ipcMain.handle` blocks**

Replace:

```ts
  ipcMain.handle("get-claude-code-hook-status", async () => {
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/workspace/attention-hook/status`);
      if (resp.status === 409) {
        return { installed: false, error: "Open a workspace first." };
      }
      if (resp.ok) {
        return await resp.json() as { installed: boolean; error?: string };
      }
    } catch {
      // Fall through to unknown status
    }
    return { installed: false, error: "Couldn't reach the OrkWorks sidecar." };
  });

  ipcMain.handle("install-claude-code-hook", async () => {
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/workspace/attention-hook/install`, { method: "POST" });
      if (resp.status === 409) {
        return { installed: false, error: "Open a workspace first." };
      }
      const body = await resp.json() as { installed?: boolean; error?: string };
      if (resp.ok) {
        return { installed: Boolean(body.installed), error: undefined };
      }
      return { installed: false, error: body.error ?? "Couldn't install the hook." };
    } catch {
      return { installed: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  });
```

with:

```ts
  async function callIntegrationRoute(harnessId: unknown, action: "status" | "install" | "uninstall") {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      const port = await portPromise;
      const method = action === "status" ? "GET" : "POST";
      const resp = await fetch(
        `http://127.0.0.1:${port}/workspace/integrations/${encodeURIComponent(harnessId)}/${action}`,
        { method },
      );
      if (resp.ok) {
        return { ok: true, status: await resp.json() };
      }
      const body = await resp.json().catch(() => ({ error: undefined }));
      return { ok: false, error: (body as { error?: string }).error ?? `Couldn't ${action} the integration.` };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  ipcMain.handle("get-harness-integration-status", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "status"));

  ipcMain.handle("install-harness-integration", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "install"));

  ipcMain.handle("uninstall-harness-integration", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "uninstall"));
```

- [ ] **Step 2: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit 2>&1 | head -30`
Expected: errors only in `SettingsModal.tsx` (fixed in the next task)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/electron/main.ts
git commit -m "feat(desktop): relay harness integration status/install/uninstall through Electron main"
```

---

### Task 8: `SettingsModal.tsx` UI cutover

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx:6,58-60,127-152,374-391`
- Modify: `apps/desktop/tests/providersPanel.test.ts:85-92`

- [ ] **Step 1: Update the type import**

Replace:

```ts
import type { HarnessConfig, AttentionHookStatusResponse } from "../harnessTypes";
```

with:

```ts
import type { HarnessConfig, IntegrationStatusResult } from "../harnessTypes";
```

- [ ] **Step 2: Replace the Claude hook state**

Replace:

```ts
  const [claudeHookStatus, setClaudeHookStatus] = useState<AttentionHookStatusResponse | null>(null);
  const [claudeHookInstalling, setClaudeHookInstalling] = useState(false);
```

with:

```ts
  const [claudeIntegration, setClaudeIntegration] = useState<IntegrationStatusResult | null>(null);
  const [claudeIntegrationBusy, setClaudeIntegrationBusy] = useState(false);
```

- [ ] **Step 3: Replace the status-fetch effect and install handler**

Replace:

```ts
  useEffect(() => {
    if (!hasClaudeCodeHarness) return;
    let cancelled = false;
    window.orkworks.getClaudeCodeHookStatus().then((status) => {
      if (!cancelled) setClaudeHookStatus(status);
    });
    return () => {
      cancelled = true;
    };
  }, [hasClaudeCodeHarness]);

  async function installClaudeHookHandler() {
    const confirmed = window.confirm(
      "This will add a Notification hook entry to .claude/settings.local.json in this workspace, " +
      "so OrkWorks can detect when Claude Code is waiting for input. Continue?"
    );
    if (!confirmed) return;

    setClaudeHookInstalling(true);
    try {
      const status = await window.orkworks.installClaudeCodeHook();
      setClaudeHookStatus(status);
    } finally {
      setClaudeHookInstalling(false);
    }
  }
```

with:

```ts
  useEffect(() => {
    if (!hasClaudeCodeHarness) return;
    let cancelled = false;
    window.orkworks.getHarnessIntegrationStatus("claude-code").then((result) => {
      if (!cancelled) setClaudeIntegration(result);
    });
    return () => {
      cancelled = true;
    };
  }, [hasClaudeCodeHarness]);

  async function installClaudeIntegrationHandler() {
    setClaudeIntegrationBusy(true);
    try {
      setClaudeIntegration(await window.orkworks.installHarnessIntegration("claude-code"));
    } finally {
      setClaudeIntegrationBusy(false);
    }
  }

  async function uninstallClaudeIntegrationHandler() {
    setClaudeIntegrationBusy(true);
    try {
      setClaudeIntegration(await window.orkworks.uninstallHarnessIntegration("claude-code"));
    } finally {
      setClaudeIntegrationBusy(false);
    }
  }
```

- [ ] **Step 4: Replace the render block**

Replace:

```tsx
                  {h.id === "claude-code" && activeDraft.includes(h.id) && (
                    <div className="settings-config-item-actions">
                      {claudeHookStatus === null && (
                        <span className="settings-config-status">checking attention hook…</span>
                      )}
                      {claudeHookStatus?.installed && (
                        <span className="settings-config-status settings-config-status--ok">✓ Attention hook installed</span>
                      )}
                      {claudeHookStatus && !claudeHookStatus.installed && (
                        <button type="button" onClick={installClaudeHookHandler} disabled={claudeHookInstalling}>
                          {claudeHookInstalling ? "Installing…" : "Install attention hook"}
                        </button>
                      )}
                      {claudeHookStatus?.error && (
                        <span className="settings-config-status">{claudeHookStatus.error}</span>
                      )}
                    </div>
                  )}
```

with:

```tsx
                  {h.id === "claude-code" && activeDraft.includes(h.id) && (
                    <div className="settings-config-item-actions">
                      {claudeIntegration === null && (
                        <span className="settings-config-status">checking Claude Code integration…</span>
                      )}
                      {claudeIntegration && !claudeIntegration.ok && (
                        <span className="settings-config-status">{claudeIntegration.error}</span>
                      )}
                      {claudeIntegration?.ok && claudeIntegration.status.registration === "installed" && (
                        <>
                          <span className="settings-config-status settings-config-status--ok">✓ Notification hook installed</span>
                          <button type="button" onClick={uninstallClaudeIntegrationHandler} disabled={claudeIntegrationBusy}>
                            {claudeIntegrationBusy ? "Removing…" : "Uninstall"}
                          </button>
                        </>
                      )}
                      {claudeIntegration?.ok &&
                        (claudeIntegration.status.registration === "absent" ||
                          claudeIntegration.status.registration === "drifted") && (
                          <>
                            {claudeIntegration.status.confirmation && (
                              <p className="settings-section-copy">
                                Installing will add a Notification hook to{" "}
                                {claudeIntegration.status.confirmation.relativePaths.join(", ")} in this
                                workspace ({claudeIntegration.status.confirmation.coverageSummary}).
                              </p>
                            )}
                            <button type="button" onClick={installClaudeIntegrationHandler} disabled={claudeIntegrationBusy}>
                              {claudeIntegrationBusy
                                ? "Installing…"
                                : claudeIntegration.status.registration === "drifted"
                                  ? "Reinstall"
                                  : "Install attention hook"}
                            </button>
                          </>
                        )}
                      {claudeIntegration?.ok && claudeIntegration.status.registration === "unsupported" && (
                        <span className="settings-config-status">
                          Attention hook isn't supported for this coding tool.
                        </span>
                      )}
                      {claudeIntegration?.ok && claudeIntegration.status.diagnostics.length > 0 && (
                        <span className="settings-config-status">
                          {claudeIntegration.status.diagnostics[0].message}
                        </span>
                      )}
                    </div>
                  )}
```

- [ ] **Step 5: Update the existing frontend regression test**

`apps/desktop/tests/providersPanel.test.ts` has a source-text-matching test (around line 85-92) that still asserts on the old API names and the removed `window.confirm` gate — it will fail against the cut-over `SettingsModal.tsx` unless updated. Replace:

```ts
test("SettingsModal offers a per-harness attention hook install affordance when enabled but not installed", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /getClaudeCodeHookStatus/);
  assert.match(source, /installClaudeCodeHook/);
  assert.match(source, /h\.id === "claude-code" && activeDraft\.includes\(h\.id\)/);
  assert.match(source, /Install attention hook/);
  assert.match(source, /window\.confirm/);
});
```

with:

```ts
test("SettingsModal offers a per-harness attention hook install affordance when enabled but not installed", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /getHarnessIntegrationStatus/);
  assert.match(source, /installHarnessIntegration/);
  assert.match(source, /uninstallHarnessIntegration/);
  assert.match(source, /h\.id === "claude-code" && activeDraft\.includes\(h\.id\)/);
  assert.match(source, /Install attention hook/);
});
```

(The `window.confirm` assertion is dropped, not replaced, per decision #6: installation is single-click with inline informational text, no second confirm gate.)

- [ ] **Step 6: Type-check and run the frontend test suite**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: no type errors, all tests pass

- [ ] **Step 7: Manual verification**

Run: `cd apps/desktop && pnpm dev`

Open a workspace, open Settings, select Claude Code as an active coding tool, and verify:
1. Status shows "checking Claude Code integration…" briefly, then either "Install attention hook" or "✓ Notification hook installed"
2. Clicking "Install attention hook" adds a `Notification` hook entry to `.claude/settings.local.json` in the opened workspace, with a command referencing `~/.orkworks/hook-scripts/report-harness-event.sh`, and the button becomes "Uninstall"
3. Clicking "Uninstall" removes that entry and the button reverts to "Install attention hook"

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/tests/providersPanel.test.ts
git commit -m "feat(desktop): cut Settings Claude Code hook UI over to the integration status API"
```

---

### Task 9: Update architecture docs

**Files:**
- Modify: `docs/agents/architecture.md:34,42,57`

- [ ] **Step 1: Update the endpoint list (line 34)**

In the "Key endpoints" sentence, replace:

```
`GET /workspace/attention-hook/status`, `POST /workspace/attention-hook/install`,
```

with:

```
`GET /workspace/integrations/:harness_id/status`, `POST /workspace/integrations/:harness_id/install`, `POST /workspace/integrations/:harness_id/uninstall`,
```

- [ ] **Step 2: Update the explanatory paragraph (line 42)**

Replace:

```
`GET /workspace/attention-hook/status` and `POST /workspace/attention-hook/install` back the explicit Settings affordance for Claude Code's Notification hook. Installation merges an idempotent entry into `.claude/settings.local.json` (never `settings.json`) and never runs at session spawn; see [ADR 0019](../adr/0019-attention-signal-endpoint-opt-in-hook-install.md). The reporter script is copied to `~/.orkworks/hook-scripts/` on install, so installed commands remain stable across app updates and AppImage mount changes.
```

with:

```
`GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}` back the explicit Settings affordance for a harness's own notification/hook integration (Claude Code today; Gemini and Copilot handlers exist but have no UI yet — see issue #217). Installation merges an idempotent, ownership-marked entry into the tool's own config file (e.g. `.claude/settings.local.json`, never `settings.json`) and never runs at session spawn; see [ADR 0026](../adr/0026-resolved-harness-capability-registry.md). The shared reporter script (`report-harness-event.sh`, POSIX only — Windows tracked as issue #218) is copied to `~/.orkworks/hook-scripts/` on install, so installed commands remain stable across app updates and AppImage mount changes.
```

- [ ] **Step 3: Update the module list (line 57)**

Replace:

```
  - `hook_handlers.rs` — Claude Code attention hook install/status (`GET /workspace/attention-hook/status`, `POST /workspace/attention-hook/install`), reporter script path resolution
```

with:

```
  - `integration_handlers.rs` — generic harness integration install/status/uninstall (`GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}`), reporter script path resolution
```

- [ ] **Step 4: Commit**

```bash
git add docs/agents/architecture.md
git commit -m "docs: describe the generic harness integration routes"
```

---

### Task 10: Full verification and PR

**Files:** none (verification only)

- [ ] **Step 1: Run the full Rust test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -20`
Expected: PASS, 0 failures

- [ ] **Step 2: Run the Rust linter**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings 2>&1 | tail -40`
Expected: PASS, no warnings

- [ ] **Step 3: Run the frontend type-check and tests**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: PASS

- [ ] **Step 4: Confirm no remaining references to the old system**

Run: `grep -rn "attention-hook\|hook_handlers\|AttentionHookStatusResponse\|getClaudeCodeHookStatus\|installClaudeCodeHook\|get-claude-code-hook-status\|install-claude-code-hook" crates/orkworksd/src apps/desktop/src apps/desktop/electron apps/desktop/tests docs/agents/architecture.md`
Expected: no output (this also covers `apps/desktop/tests`, which Task 8 Step 5 already updated — this scan is a backstop, not the only place that update happens)

- [ ] **Step 5: Run the doc-currency and worktree-currency checks**

Run: `bash .claude/hooks/doc-check.sh`
Run: `bash .claude/hooks/worktree-check.sh`
Address anything flagged before proceeding.

- [ ] **Step 6: Push and open the PR**

```bash
git push -u origin retire-shadow-hook-installer
gh pr create --title "Retire the shadow hook installer" --body "$(cat <<'EOF'
## Summary
- Replaces the unsafe, live Claude-only hook installer (`http/hook_handlers.rs`) with the existing-but-unwired ADR-0026 integration stack, exposed via generic `GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}` routes.
- Authors `report-harness-event.sh`, the reporter script the new system depends on (did not exist before this change), preserving Claude's session-ID capture behavior.
- Adds an Uninstall button to Settings, which the old system never had.

## Follow-ups
- #217 — Gemini/Copilot Settings UI (backend already supports it)
- #218 — Windows `report-harness-event.ps1`

## Test plan
- [ ] `cargo test --manifest-path crates/orkworksd/Cargo.toml`
- [ ] `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings`
- [ ] `cd apps/desktop && npx tsc --noEmit`
- [ ] Manual: install/uninstall the Claude Code hook via Settings in a real workspace, verify `.claude/settings.local.json` contents
EOF
)"
```

- [ ] **Step 7: Request review**

Per `AGENTS.md`, run `/code-review` (medium effort — this touches a security-relevant workspace-mutation seam and the IPC/Electron boundary) before merge, and address findings or note why each is intentional in the PR description.

---

## Self-review notes

- **Spec coverage:** all 7 grilling-session decision points are covered — script authoring (Task 1), generic routes + registry methods (Tasks 2-3), error mapping (Task 3), rip-and-replace deletion (Task 4), renderer cutover with Uninstall button and single-click confirmation-as-text (Tasks 5-8), no migration code (not implemented anywhere, as agreed), doc updates (Task 9).
- **Type consistency:** `IntegrationStatusResult`/`IntegrationStatus` names are used identically across Tasks 5-8; `integration_install`/`integration_uninstall`/`integration_status` names are used identically across Tasks 2-4; route paths (`/workspace/integrations/:harness_id/{status,install,uninstall}`) are used identically across Tasks 3, 4, 7, 9.
- **Verified:** Task 2's `generic_shell_status` registration-variant assertion (`Unsupported`) was checked directly against `harness/integrations/mod.rs:325-346` while writing this plan — no open questions remain.

## Post-authoring correctness pass (2026-07-24)

An independent review against the live repo (not just this plan's own claims) found and fixed several defects before any task ran:

- **Every JsonHookHandler test needed a Git workspace.** Claude's handler enforces `require_local_or_ignored_untracked` (`harness/integration.rs`), which requires the workspace to be a Git repo with the target file gitignored. Task 2's `integration_install_and_uninstall_round_trip_for_claude_code` and three of Task 3's HTTP tests (`status_reports_absent_for_a_fresh_workspace`, `install_then_status_reports_installed`, `install_then_uninstall_reports_absent`) originally used a bare `tempfile::tempdir()` with no `git2::Repository::init` — `install`/`uninstall` would have returned `UnsafeTarget{"not_git_workspace"}` (panicking `.unwrap()` in Task 2, wrong HTTP status in Task 3) instead of the behavior under test. Fixed by adding git init + `.gitignore` setup to each. Task 3's `install_rejects_malformed_existing_settings_file` test had the same gap — it was accidentally passing because `not_git_workspace` and `invalid_config` both map to 400, so it wasn't actually exercising the malformed-JSON path its name claims; fixed the same way.
- **`main.rs` has a second, hand-maintained route table** (`fn test_router` inside its own `#[cfg(test)] mod tests`, around line 473) that duplicates the production router for HTTP-level tests. The original Task 4 only updated the production router; `test_router` would have failed to compile once the top-level `hook_handlers` import was replaced. Added Step 3 to Task 4 to update it too.
- **`IntegrationOwnership::OrkWorks` serializes to `"ork_works"`, not `"orkworks"`.** Serde's `snake_case` rename rule inserts an underscore before every internal capital rather than detecting word boundaries; verified empirically with a standalone `cargo run`. Fixed the `IntegrationOwnership` TS union in Task 5.
- **`crates/orkworksd/src/http/hook_handlers.rs` has 14 tests, not 13** — fixed the count in Task 4's "expected" note.
- **`apps/desktop/tests/providersPanel.test.ts` was untouched by the plan** but source-text-matches `getClaudeCodeHookStatus`/`installClaudeCodeHook`/`window.confirm` against `SettingsModal.tsx`; all three vanish in Task 8's cutover. Added a step to Task 8 to update that test, and widened Task 10's final grep to scan `apps/desktop/tests` too.
- The `--marker` argument parser in Task 1's script used `shift 2` unconditionally; if `--marker` were ever invoked as the last argument with no value, bash leaves the positional parameters unchanged on an out-of-range `shift`, so the loop would spin forever on the same `$1`. Fixed to check `$#` before shifting.

Everything else — file paths, line ranges, Rust field/type names (`IntegrationContext`, `ReporterAssetResolver`, `AppState.workspace`/`.harness_catalog`, `ResolvedHarnessRegistry::get`), the other serde renames, the TS/Electron before-blocks in Tasks 5-8, and ADR 0019's already-correct "superseded by ADR 0026" status — matched the live repo as written.

**Double-check live, don't just trust this plan:** run `cargo test --manifest-path crates/orkworksd/Cargo.toml` after Task 3 specifically (before Task 4 deletes the old file) to confirm the git-workspace fixture setup above actually produces the registrations each test expects — the JsonHookHandler control flow (`load` → `require_local_or_ignored_untracked` → `ConfigFileTransaction`) has enough branches that it's worth seeing green output rather than trusting this review's trace-through.
