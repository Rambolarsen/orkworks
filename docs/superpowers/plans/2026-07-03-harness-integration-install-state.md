# Harness Integration Install State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add generic harness integration status/install/uninstall support so Settings can show `Enabled`, `Detected`, and `Installed`, with ownership-safe uninstall for OrkWorks-managed integration and a Claude Code migration off the one-off hook button.

**Architecture:** Keep the harness inventory API (`GET /harnesses`) unchanged and add a small backend integration-status layer for built-in harnesses only. Use a workspace-scoped manifest for Claude ownership tracking, expose new REST endpoints from the sidecar, and have the renderer fetch them directly via `getBackendUrl()` rather than adding new Electron IPC surface.

**Tech Stack:** Rust (`axum`, `serde`, existing sidecar modules), TypeScript/React renderer, Electron preload bridge only for backend URL, Node built-in test runner, Cargo tests.

## Global Constraints

- Built-in harness rows must always remain visible.
- Uninstall removes only OrkWorks-managed integration and must never uninstall the external CLI.
- Built-ins without an OrkWorks-managed integration must report `installed.state = unsupported`.
- `enabled` is workspace-scoped, `detected` is app-environment scoped, and workspace-local `installed` state must degrade cleanly when no workspace is open.
- Legacy Claude hook installs without ownership proof must surface as `conflict` with `ownership = unknown`, with uninstall blocked.
- Install/uninstall must be idempotent success operations when the requested end state is already satisfied.
- Keep the backend dispatch minimal: a small internal match/enum table is preferred over a heavyweight plugin framework.

---

## File Map

- Create: `crates/orkworksd/src/harness_integration.rs`
  Responsibility: integration status enums/structs, manifest read/write helpers, per-built-in status/install/uninstall dispatch, command detection helpers.
- Modify: `crates/orkworksd/src/metadata.rs`
  Responsibility: add a workspace-scoped path helper for the harness integration manifest file.
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
  Responsibility: add `GET /harnesses/integration-status`, `POST /harnesses/:id/install`, and `POST /harnesses/:id/uninstall`.
- Modify: `crates/orkworksd/src/http/hook_handlers.rs`
  Responsibility: keep/reuse Claude hook file helpers, add uninstall-safe removal helpers, remove the old Claude-only HTTP handlers.
- Modify: `crates/orkworksd/src/main.rs`
  Responsibility: register the new harness integration routes and stop exposing the old Claude-only install/status endpoints.
- Modify: `apps/desktop/src/harnessTypes.ts`
  Responsibility: replace `AttentionHookStatusResponse` with generic harness integration types.
- Modify: `apps/desktop/src/api.ts`
  Responsibility: add fetch wrappers for harness integration status/install/uninstall.
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
  Responsibility: replace the Claude-only hook UI with generic harness rows/cards and refresh logic.
- Modify: `apps/desktop/src/App.tsx`
  Responsibility: no new backend model here; keep passing harness list and active harness ids into `SettingsModal`.
- Modify: `apps/desktop/electron/preload.ts`
  Responsibility: remove the now-unused Claude-specific hook IPC methods.
- Modify: `apps/desktop/electron/main.ts`
  Responsibility: remove the now-unused `get-claude-code-hook-status` and `install-claude-code-hook` IPC handlers.
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
  Responsibility: remove the deleted Claude-only methods from the renderer bridge typing.
- Modify: `apps/desktop/tests/providersPanel.test.ts`
  Responsibility: replace Claude-only source assertions with generic harness integration UI assertions.
- Modify: `apps/desktop/tests/api.test.ts`
  Responsibility: add type-level coverage for the new harness integration response shapes.
- Modify: `crates/orkworksd/src/http/hook_handlers.rs` tests and `crates/orkworksd/src/http/harness_handlers.rs` tests
  Responsibility: cover status/install/uninstall, legacy Claude conflict state, unsupported harnesses, and idempotent repeats.
- Create: `apps/desktop/tests/architectureDoc.test.ts`
  Responsibility: guard the documented endpoint surface so the Claude-only route names do not linger.
- Modify: `docs/agents/architecture.md`
  Responsibility: document the new endpoints and removal of the Claude-only install/status surface.

### Task 1: Backend Status Model And Integration Status Endpoint

**Files:**
- Create: `crates/orkworksd/src/harness_integration.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/harness_integration.rs`
- Test: `crates/orkworksd/src/http/harness_handlers.rs`

**Interfaces:**
- Consumes:
  - `crate::harness_registry::HarnessConfig`
  - `crate::metadata::MetadataStore`
  - `crate::AppState`
- Produces:
  - `pub(crate) enum HarnessDetectedState { Detected, NotDetected, Unknown }`
  - `pub(crate) enum HarnessInstalledState { NotInstalled, Installed, Partial, Outdated, Conflict, Unsupported, Unknown }`
  - `pub(crate) enum HarnessOwnership { Owned, Unowned, Unknown }`
  - `pub(crate) enum HarnessIntegrationScope { Workspace, AppEnvironment }`
  - `pub(crate) struct HarnessIntegrationStatusResponse`
  - `pub(crate) fn list_integration_statuses(state: &AppState) -> Vec<HarnessIntegrationStatusResponse>`
  - `pub(crate) async fn list_harness_integration_status(State<Arc<AppState>>) -> impl IntoResponse`

- [ ] **Step 1: Write the failing backend tests for status typing, unsupported built-ins, and no-workspace behavior**

```rust
#[test]
fn reports_unsupported_for_builtins_without_installable_integration() {
    let harnesses = builtin_harness_configs();
    let statuses = super::list_integration_statuses_for_test(None, &harnesses);

    let codex = statuses.iter().find(|s| s.harness_id == "codex").unwrap();
    assert_eq!(codex.installed.state, HarnessInstalledState::Unsupported);
    assert_eq!(codex.actions.can_install, false);
    assert_eq!(codex.actions.can_uninstall, false);
}

#[test]
fn reports_unknown_workspace_state_when_no_workspace_is_open() {
    let harnesses = builtin_harness_configs();
    let statuses = super::list_integration_statuses_for_test(None, &harnesses);

    let claude = statuses.iter().find(|s| s.harness_id == "claude-code").unwrap();
    assert_eq!(claude.detected.scope, HarnessIntegrationScope::AppEnvironment);
    assert_eq!(claude.installed.state, HarnessInstalledState::Unknown);
    assert!(claude.installed.detail.contains("Open a workspace"));
    assert_eq!(claude.actions.can_install, false);
}

#[tokio::test]
async fn integration_status_endpoint_returns_rows_for_all_harnesses() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    *state.harnesses.write().await = builtin_harness_configs();

    let response = list_harness_integration_status(State(state)).await.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

- [ ] **Step 2: Run the Rust tests to verify the status API is missing**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: FAIL with unresolved symbols such as `list_integration_statuses_for_test`, `HarnessInstalledState`, or the missing `/harnesses/integration-status` handler.

- [ ] **Step 3: Add the status enums, response structs, manifest path helper, and read-only status endpoint**

```rust
// crates/orkworksd/src/metadata.rs
pub fn harness_integrations_path(&self) -> PathBuf {
    self.root.join("harness-integrations.json")
}
```

```rust
// crates/orkworksd/src/harness_integration.rs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessDetectedState {
    Detected,
    NotDetected,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessInstalledState {
    NotInstalled,
    Installed,
    Partial,
    Outdated,
    Conflict,
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessOwnership {
    Owned,
    Unowned,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessIntegrationScope {
    Workspace,
    AppEnvironment,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessDetectedStatus {
    pub(crate) state: HarnessDetectedState,
    pub(crate) scope: HarnessIntegrationScope,
    #[serde(rename = "resolvedPath", skip_serializing_if = "Option::is_none")]
    pub(crate) resolved_path: Option<String>,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessInstalledStatus {
    pub(crate) state: HarnessInstalledState,
    pub(crate) scope: HarnessIntegrationScope,
    pub(crate) ownership: HarnessOwnership,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessIntegrationActions {
    #[serde(rename = "canInstall")]
    pub(crate) can_install: bool,
    #[serde(rename = "canUninstall")]
    pub(crate) can_uninstall: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessIntegrationStatusResponse {
    #[serde(rename = "harnessId")]
    pub(crate) harness_id: String,
    pub(crate) enabled: Option<serde_json::Value>,
    pub(crate) detected: HarnessDetectedStatus,
    pub(crate) installed: HarnessInstalledStatus,
    pub(crate) actions: HarnessIntegrationActions,
}
```

```rust
// crates/orkworksd/src/http/harness_handlers.rs
pub(crate) async fn list_harness_integration_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let statuses = crate::harness_integration::list_integration_statuses(&state);
    Json(statuses)
}
```

```rust
// crates/orkworksd/src/main.rs
mod harness_integration;

.route("/harnesses/integration-status", get(list_harness_integration_status))
```

- [ ] **Step 4: Run the focused Rust tests until they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit the status-model slice**

```bash
git add crates/orkworksd/src/metadata.rs \
  crates/orkworksd/src/harness_integration.rs \
  crates/orkworksd/src/http/harness_handlers.rs \
  crates/orkworksd/src/main.rs
git commit -m "feat: add harness integration status model"
```

### Task 2: Claude Install/Uninstall Backend With Workspace Manifest Ownership

**Files:**
- Modify: `crates/orkworksd/src/harness_integration.rs`
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `crates/orkworksd/src/http/hook_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/http/hook_handlers.rs`
- Test: `crates/orkworksd/src/http/harness_handlers.rs`

**Interfaces:**
- Consumes:
  - `read_settings_local`, `settings_local_path`, `ensure_stable_claude_hook_script`
  - `MetadataStore::harness_integrations_path()`
- Produces:
  - `pub(crate) struct HarnessIntegrationManifest`
  - `pub(crate) async fn install_harness_integration(State<Arc<AppState>>, Path<String>) -> impl IntoResponse`
  - `pub(crate) async fn uninstall_harness_integration(State<Arc<AppState>>, Path<String>) -> impl IntoResponse`
  - `pub(crate) fn remove_claude_hook_entry(settings: &mut Value, command: &str) -> Result<bool, String>`

- [ ] **Step 1: Write the failing backend tests for owned install, idempotent repeat, blocked legacy uninstall, and successful uninstall**

```rust
#[tokio::test]
async fn install_claude_integration_writes_manifest_and_reports_owned() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    *state.harnesses.write().await = builtin_harness_configs();

    let response = install_harness_integration(State(state.clone()), Path("claude-code".into()))
        .await
        .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let statuses = crate::harness_integration::list_integration_statuses(&state);
    let claude = statuses.iter().find(|s| s.harness_id == "claude-code").unwrap();
    assert_eq!(claude.installed.state, HarnessInstalledState::Installed);
    assert_eq!(claude.installed.ownership, HarnessOwnership::Owned);
}

#[tokio::test]
async fn uninstall_blocks_legacy_claude_hook_without_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    *state.harnesses.write().await = builtin_harness_configs();

    let claude_dir = dir.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(
        claude_dir.join("settings.local.json"),
        r#"{ "hooks": { "Notification": [ { "hooks": [ { "type": "command", "command": "\"/tmp/report-claude-session-from-hook.sh\"" } ] } ] } }"#,
    ).unwrap();

    let response = uninstall_harness_integration(State(state), Path("claude-code".into()))
        .await
        .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn repeated_uninstall_returns_success_with_unchanged_status() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    *state.harnesses.write().await = builtin_harness_configs();

    let response = uninstall_harness_integration(State(state), Path("claude-code".into()))
        .await
        .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

- [ ] **Step 2: Run the Rust tests to verify install/uninstall logic is still missing**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: FAIL with missing `install_harness_integration` / `uninstall_harness_integration` handlers, missing manifest helpers, or incorrect legacy status behavior.

- [ ] **Step 3: Implement manifest-backed Claude install/uninstall and remove the old Claude-only HTTP endpoints**

```rust
// crates/orkworksd/src/harness_integration.rs
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HarnessIntegrationManifest {
    pub(crate) version: u32,
    pub(crate) entries: Vec<HarnessIntegrationManifestEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct HarnessIntegrationManifestEntry {
    #[serde(rename = "harnessId")]
    pub(crate) harness_id: String,
    pub(crate) kind: String,
    #[serde(rename = "settingsPath")]
    pub(crate) settings_path: String,
    pub(crate) command: String,
}
```

```rust
// crates/orkworksd/src/http/hook_handlers.rs
pub(crate) fn remove_claude_hook_entry(settings: &mut Value, command: &str) -> Result<bool, String> {
    let notification = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("Notification"))
        .and_then(|n| n.as_array_mut())
        .ok_or_else(|| "\"hooks.Notification\" is not an array".to_string())?;

    let before = notification.len();
    notification.retain(|entry| {
        let hooks = entry.get("hooks").and_then(|h| h.as_array());
        !hooks
            .into_iter()
            .flatten()
            .any(|hook| hook.get("command").and_then(|c| c.as_str()) == Some(command))
    });
    Ok(before != notification.len())
}
```

```rust
// crates/orkworksd/src/http/harness_handlers.rs
pub(crate) async fn install_harness_integration(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    crate::harness_integration::install(&state, &id).into_response()
}

pub(crate) async fn uninstall_harness_integration(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    crate::harness_integration::uninstall(&state, &id).into_response()
}
```

```rust
// crates/orkworksd/src/main.rs
.route("/harnesses/:id/install", post(install_harness_integration))
.route("/harnesses/:id/uninstall", post(uninstall_harness_integration))
// Remove:
// .route("/workspace/attention-hook/status", get(get_attention_hook_status))
// .route("/workspace/attention-hook/install", post(install_attention_hook))
```

- [ ] **Step 4: Run the focused Rust tests until they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit the install/uninstall backend slice**

```bash
git add crates/orkworksd/src/harness_integration.rs \
  crates/orkworksd/src/http/harness_handlers.rs \
  crates/orkworksd/src/http/hook_handlers.rs \
  crates/orkworksd/src/main.rs
git commit -m "feat: add harness integration install and uninstall"
```

### Task 3: Renderer Types, REST Wiring, And Settings UI Migration

**Files:**
- Modify: `apps/desktop/src/harnessTypes.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/providersPanel.test.ts`

**Interfaces:**
- Consumes:
  - `window.orkworks.getBackendUrl(): Promise<string>`
  - `GET /harnesses`
  - `GET /harnesses/integration-status`
  - `POST /harnesses/:id/install`
  - `POST /harnesses/:id/uninstall`
- Produces:
  - `export interface HarnessIntegrationStatusResponse`
  - `export async function getHarnessIntegrationStatuses(baseUrl: string): Promise<HarnessIntegrationStatusResponse[]>`
  - `export async function installHarnessIntegration(baseUrl: string, id: string): Promise<HarnessIntegrationStatusResponse>`
  - `export async function uninstallHarnessIntegration(baseUrl: string, id: string): Promise<HarnessIntegrationStatusResponse>`

- [ ] **Step 1: Write the failing type/source tests for generic harness integration UI and removal of Claude-only bridge methods**

```ts
test("harnessTypes declares the generic harness integration status shape", () => {
  const source = readFileSync(new URL("../src/harnessTypes.ts", import.meta.url), "utf8");
  assert.match(source, /export interface HarnessIntegrationStatusResponse/);
  assert.match(source, /ownership: "owned" \| "unowned" \| "unknown"/);
  assert.match(source, /state: "not_installed" \| "installed" \| "partial" \| "outdated" \| "conflict" \| "unsupported" \| "unknown"/);
});

test("SettingsModal uses generic harness integration status instead of Claude-only hook IPC", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /getHarnessIntegrationStatuses/);
  assert.match(source, /installHarnessIntegration/);
  assert.match(source, /uninstallHarnessIntegration/);
  assert.doesNotMatch(source, /getClaudeCodeHookStatus/);
  assert.doesNotMatch(source, /installClaudeCodeHook/);
});

test("preload no longer exposes Claude-only hook installers", () => {
  const source = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  assert.doesNotMatch(source, /getClaudeCodeHookStatus/);
  assert.doesNotMatch(source, /installClaudeCodeHook/);
});
```

- [ ] **Step 2: Run the frontend tests to verify the generic renderer API is missing**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/providersPanel.test.ts`

Expected: FAIL because `HarnessIntegrationStatusResponse`, generic fetch wrappers, and the updated `SettingsModal` source assertions do not exist yet.

- [ ] **Step 3: Add generic renderer types/fetchers, remove the old Claude-only bridge methods, and render the richer Settings rows**

```ts
// apps/desktop/src/harnessTypes.ts
export interface HarnessIntegrationStatusResponse {
  harnessId: string;
  enabled?: {
    state: "enabled" | "disabled" | "unavailable";
    scope: "workspace";
    detail?: string;
  };
  detected: {
    state: "detected" | "not_detected" | "unknown";
    scope: "app_environment";
    resolvedPath?: string;
    detail: string;
  };
  installed: {
    state: "not_installed" | "installed" | "partial" | "outdated" | "conflict" | "unsupported" | "unknown";
    scope: "workspace" | "app_environment";
    ownership: "owned" | "unowned" | "unknown";
    detail: string;
  };
  actions: {
    canInstall: boolean;
    canUninstall: boolean;
  };
}
```

```ts
// apps/desktop/src/api.ts
export async function getHarnessIntegrationStatuses(baseUrl: string): Promise<HarnessIntegrationStatusResponse[]> {
  const resp = await fetch(`${baseUrl}/harnesses/integration-status`);
  if (!resp.ok) throw new Error(`get harness integration statuses failed: ${resp.status}`);
  return resp.json();
}

export async function installHarnessIntegration(baseUrl: string, id: string): Promise<HarnessIntegrationStatusResponse> {
  const resp = await fetch(`${baseUrl}/harnesses/${id}/install`, { method: "POST" });
  if (!resp.ok) throw new Error(`install harness integration failed: ${resp.status}`);
  return resp.json();
}

export async function uninstallHarnessIntegration(baseUrl: string, id: string): Promise<HarnessIntegrationStatusResponse> {
  const resp = await fetch(`${baseUrl}/harnesses/${id}/uninstall`, { method: "POST" });
  if (!resp.ok) throw new Error(`uninstall harness integration failed: ${resp.status}`);
  return resp.json();
}
```

```tsx
// apps/desktop/src/components/SettingsModal.tsx
const [integrationStatuses, setIntegrationStatuses] = useState<Record<string, HarnessIntegrationStatusResponse>>({});

useEffect(() => {
  let cancelled = false;
  async function load() {
    const baseUrl = await window.orkworks.getBackendUrl();
    const rows = await getHarnessIntegrationStatuses(baseUrl);
    if (!cancelled) {
      setIntegrationStatuses(Object.fromEntries(rows.map((row) => [row.harnessId, row])));
    }
  }
  load().catch(() => setActiveSaveStatus("Couldn't load coding tool integration status."));
  return () => { cancelled = true; };
}, [harnesses]);
```

- [ ] **Step 4: Run the frontend tests until they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/providersPanel.test.ts`

Expected: PASS

- [ ] **Step 5: Commit the renderer migration**

```bash
git add apps/desktop/src/harnessTypes.ts \
  apps/desktop/src/api.ts \
  apps/desktop/src/components/SettingsModal.tsx \
  apps/desktop/electron/preload.ts \
  apps/desktop/electron/main.ts \
  apps/desktop/src/orkworksWindow.d.ts \
  apps/desktop/tests/api.test.ts \
  apps/desktop/tests/providersPanel.test.ts
git commit -m "feat: show harness integration status in settings"
```

### Task 4: Documentation, Full Verification, And Final Cleanup

**Files:**
- Create: `apps/desktop/tests/architectureDoc.test.ts`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/superpowers/specs/2026-07-03-harness-integration-install-state-design.md`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `crates/orkworksd/src/http/harness_handlers.rs`
- Test: `crates/orkworksd/src/http/hook_handlers.rs`

**Interfaces:**
- Consumes:
  - final route list from `crates/orkworksd/src/main.rs`
  - final renderer/backend fetch path from `apps/desktop/src/api.ts`
- Produces:
  - updated architecture documentation that names the new harness integration endpoints
  - final verification evidence for desktop tests, Rust tests, and doc currency

- [ ] **Step 1: Write the failing doc/source test expectations for the new route names**

```ts
test("architecture doc names the harness integration status/install/uninstall endpoints", () => {
  const source = readFileSync(new URL("../../../docs/agents/architecture.md", import.meta.url), "utf8");
  assert.match(source, /GET \/harnesses\/integration-status/);
  assert.match(source, /POST \/harnesses\/:id\/install/);
  assert.match(source, /POST \/harnesses\/:id\/uninstall/);
  assert.doesNotMatch(source, /workspace\/attention-hook\/status/);
});
```

- [ ] **Step 2: Run the final mixed verification before doc updates**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/architectureDoc.test.ts`

Expected: FAIL because `docs/agents/architecture.md` still names the old Claude-only route surface or does not yet mention the new harness integration endpoints.

- [ ] **Step 3: Update the architecture doc and finalize the spec wording cleanup**

```md
Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, `POST /sessions/:id/attention`, `GET /sessions/:id/terminal-output`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `GET /harnesses`, `GET /harnesses/integration-status`, `POST /harnesses/:id/install`, `POST /harnesses/:id/uninstall`, and `WS /sessions/:id/terminal`.
```

```md
The renderer no longer uses Claude-specific install/status IPC for hook setup. Settings fetches generic harness integration status from the sidecar and triggers install/uninstall through the harness REST endpoints, while the preload bridge remains limited to backend discovery and Electron-only functionality.
```

- [ ] **Step 4: Run the full repo verification and doc currency check**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: PASS

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: PASS

Run: `bash .claude/hooks/doc-check.sh`
Expected: either no output or only informational lines with no remaining required doc updates.

- [ ] **Step 5: Commit docs and verification adjustments**

```bash
git add docs/agents/architecture.md \
  docs/superpowers/specs/2026-07-03-harness-integration-install-state-design.md
git commit -m "docs: document harness integration status flow"
```

## Self-Review

### Spec Coverage

- `enabled` / `detected` / `installed` split: covered by Task 1 types and Task 3 UI.
- `unsupported` for non-installable built-ins: covered by Task 1 status logic and tests.
- ownership-safe uninstall: covered by Task 2 manifest-backed Claude install/uninstall and blocked legacy uninstall tests.
- no-workspace mixed-scope behavior: covered by Task 1 endpoint behavior and tests.
- generic settings UI replacing Claude-only button: covered by Task 3.
- docs update for endpoint surface: covered by Task 4.

No uncovered spec requirement remains.

### Placeholder Scan

- No `TODO`, `TBD`, or “implement later” placeholders remain.
- Every task lists exact files, commands, and named interfaces.
- The only intentionally open implementation choice is Claude manifest shape vs inline marker; this plan resolves it by using a workspace-scoped manifest, so later tasks do not depend on an unresolved branch.

### Type Consistency

- Backend names are consistent across tasks: `HarnessIntegrationStatusResponse`, `HarnessInstalledState`, `HarnessOwnership`, `install_harness_integration`, `uninstall_harness_integration`.
- Frontend names are consistent across tasks: `HarnessIntegrationStatusResponse`, `getHarnessIntegrationStatuses`, `installHarnessIntegration`, `uninstallHarnessIntegration`.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-03-harness-integration-install-state.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
