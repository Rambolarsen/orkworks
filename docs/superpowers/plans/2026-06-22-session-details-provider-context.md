# Session Details Provider Context Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move provider visibility out of the main Dockview Providers surface and into read-only session details (`Provider`, `Model`, `State`), while keeping app-wide provider editing in Settings and preserving the Peon fallback backend.

**Architecture:** Keep provider fallback/configuration in the existing Electron settings + Rust `ProviderManager` flow, but persist the winning provider context onto session metadata so `/sessions` can expose session-specific provider/runtime fields. In the renderer, add a small read-only provider context block to `SessionDetailPanel`, extract the existing provider editor into a Settings-only component, and demote the Dockview `capacity` surface away from Providers.

**Tech Stack:** React, TypeScript, Dockview, Electron preload/main-process settings, Rust/Axum sidecar, serde, Node built-in test runner, Cargo tests.

---

## File Map

- Create: `docs/adr/0016-session-details-provider-context.md`
- Create: `apps/desktop/src/sessionProviderContext.ts`
- Create: `apps/desktop/src/components/ProviderSettingsSection.tsx`
- Create: `apps/desktop/tests/sessionProviderContext.test.ts`
- Modify: `docs/adr/0015-provider-ops-peon-fallback.md`
- Modify: `docs/adr/README.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/architecture.md`
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/components/CapacityPanel.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/electron/menuTemplate.ts`
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

### Task 1: Record The UI Decision And Update Product Docs

**Files:**
- Create: `docs/adr/0016-session-details-provider-context.md`
- Modify: `docs/adr/0015-provider-ops-peon-fallback.md`
- Modify: `docs/adr/README.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Verify the existing docs still describe Providers as a primary panel**

Run: `rtk rg -n "Providers panel|visible label: Providers|capacity slot now renders the visible Providers panel|Provider defaults, overrides" README.md AGENTS.md docs/adr docs/agents/architecture.md`

Expected: matches in `README.md`, `AGENTS.md`, `docs/agents/architecture.md`, and ADR 0015.

- [ ] **Step 2: Write the superseding ADR**

```md
# Session details provider context

- Status: accepted
- Supersedes: 0015 (UI surface only)
- Date: 2026-06-22

## Context

Provider fallback remains necessary for Peon reliability, but a dedicated Providers panel overstates how important provider management is to the main user workflow. OrkWorks is session-centric and should expose only the provider context relevant to the selected session.

## Decision

Show session-specific provider context in read-only `Details` fields:

- `Provider`
- `Model`
- `State`

Keep provider editing app-wide in `Settings`. Remove Providers as a primary Dockview surface. The backend fallback system from ADR 0015 remains in place.

## Consequences

- Session details become the only always-relevant provider surface in the main window.
- Provider editing remains available without breaking the read-only interaction model in `Details`.
- ADR 0015's backend fallback decision stands, but its primary-panel UI decision is superseded.
```

- [ ] **Step 3: Mark ADR 0015 as superseded and update the ADR index**

```md
- Status: superseded by 0016
```

```md
| [0015](./0015-provider-ops-peon-fallback.md) | Provider ops panel and app-wide Peon fallback | superseded |
| [0016](./0016-session-details-provider-context.md) | Session details provider context | accepted |
```

- [ ] **Step 4: Update the product docs to match the new UI**

```md
- Session details show read-only `Provider`, `Model`, and `State` for the selected session.
- Provider editing remains app-wide in Settings.
- The Dockview `capacity` slot is no longer the visible Providers surface.
```

Apply that wording consistently in:

- `README.md`
- `AGENTS.md`
- `docs/agents/architecture.md`

- [ ] **Step 5: Re-run the doc search**

Run: `rtk rg -n "visible label: Providers|capacity slot now renders the visible Providers panel|Open Providers Panel" README.md AGENTS.md docs/adr docs/agents/architecture.md apps/desktop/src/components/SettingsModal.tsx`

Expected: no stale matches for the old primary-panel wording.

- [ ] **Step 6: Commit the ADR/doc update**

```bash
rtk git add docs/adr/0016-session-details-provider-context.md docs/adr/0015-provider-ops-peon-fallback.md docs/adr/README.md README.md AGENTS.md docs/agents/architecture.md
rtk git commit -m "docs: move provider context into session details"
```

### Task 2: Persist Session-Specific Provider Context In The Rust Backend

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Add failing Rust tests for provider observation persistence**

```rust
#[test]
fn merge_peon_inference_persists_provider_context() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    store.write_session(&test_metadata("provider-context"));

    let inf = crate::peon::PeonInference {
        observed_status: Some("working".into()),
        phase: None,
        summary: Some("still working".into()),
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        confidence: 0.9,
        detected_harness: None,
        detected_model: None,
        harness_session_id: None,
    };

    let provider = crate::providers::ProviderObservation {
        provider_id: "claude-code".into(),
        provider_label: "Claude Code".into(),
        provider_model: Some("sonnet".into()),
        provider_state: "healthy".into(),
    };

    store.merge_peon_inference("provider-context", &inf, "later", Some(&provider));

    let meta = store.read_session("provider-context").unwrap();
    assert_eq!(meta.provider_id.as_deref(), Some("claude-code"));
    assert_eq!(meta.provider_label.as_deref(), Some("Claude Code"));
    assert_eq!(meta.provider_model.as_deref(), Some("sonnet"));
    assert_eq!(meta.provider_state.as_deref(), Some("healthy"));
}
```

```rust
#[test]
fn session_info_serializes_provider_fields() {
    let info = SessionInfo {
        id: "test".into(),
        label: "Test".into(),
        harness: None,
        model: None,
        status: "running".into(),
        cwd: "/tmp".into(),
        created_at: "now".into(),
        observed_status: Some("waiting_for_input".into()),
        summary: Some("Needs approval".into()),
        next_action: Some("Choose an option".into()),
        needs_user_input: Some(true),
        detected_question: Some("Proceed?".into()),
        suggested_options: Some(vec!["yes".into(), "no".into()]),
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: Some("process".into()),
        metadata_confidence: Some(1.0),
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        provider: Some("Claude Code".into()),
        provider_model: Some("sonnet".into()),
        provider_state: Some("healthy".into()),
        memory_state: MemoryState::Live,
        resume_strategy: harness::ResumeStrategy::None,
        resume: None,
        resumed_from: None,
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"provider\":\"Claude Code\""));
    assert!(json.contains("\"providerModel\":\"sonnet\""));
    assert!(json.contains("\"providerState\":\"healthy\""));
}
```

- [ ] **Step 2: Run the targeted Rust tests and confirm they fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml merge_peon_inference_persists_provider_context`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_info_serializes_provider_fields`

Expected: FAIL because `ProviderObservation`, the new metadata fields, and the new `merge_peon_inference` signature do not exist yet.

- [ ] **Step 3: Add provider observation data to the provider runner and metadata model**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderObservation {
    #[serde(rename = "providerId")]
    pub provider_id: String,
    #[serde(rename = "providerLabel")]
    pub provider_label: String,
    #[serde(rename = "providerModel")]
    pub provider_model: Option<String>,
    #[serde(rename = "providerState")]
    pub provider_state: String,
}

pub struct ProviderRunResult {
    pub inference: Option<peon::PeonInference>,
    pub winning_provider_id: Option<String>,
    pub observation: Option<ProviderObservation>,
    pub attempts: Vec<AttemptRecord>,
    pub runtime: HashMap<String, ProviderRuntimeEntry>,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    #[serde(rename = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(rename = "providerLabel", skip_serializing_if = "Option::is_none")]
    pub provider_label: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    pub provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<String>,
    // existing fields stay in place
}
```

```rust
pub fn merge_peon_inference(
    &self,
    id: &str,
    inf: &crate::peon::PeonInference,
    timestamp: &str,
    provider: Option<&crate::providers::ProviderObservation>,
) {
    // existing inference merge
    meta.provider_id = provider.map(|p| p.provider_id.clone());
    meta.provider_label = provider.map(|p| p.provider_label.clone());
    meta.provider_model = provider.and_then(|p| p.provider_model.clone());
    meta.provider_state = provider.map(|p| p.provider_state.clone());
    // existing write + append_event
}
```

- [ ] **Step 4: Thread the new provider context into `/sessions`**

```rust
#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    provider_state: Option<String>,
    // existing fields
}
```

```rust
provider: meta.and_then(|m| m.provider_label.clone()),
provider_model: meta.and_then(|m| m.provider_model.clone()),
provider_state: meta.and_then(|m| m.provider_state.clone()),
```

```rust
let provider_result = state_clone.providers.run_inference(providers::PeonScope::Session, &output_snapshot);
if let Some(ref inf) = provider_result.inference {
    ws.metadata.merge_peon_inference(&id, inf, &now_iso, provider_result.observation.as_ref());
}
```

- [ ] **Step 5: Re-run the Rust tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml merge_peon_inference_persists_provider_context`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_info_serializes_provider_fields`

Expected: PASS.

- [ ] **Step 6: Commit the backend contract change**

```bash
rtk git add crates/orkworksd/src/providers.rs crates/orkworksd/src/metadata.rs crates/orkworksd/src/main.rs
rtk git commit -m "feat: persist provider context on sessions"
```

### Task 3: Add Read-Only Provider Fields To Session Details

**Files:**
- Create: `apps/desktop/src/sessionProviderContext.ts`
- Create: `apps/desktop/tests/sessionProviderContext.test.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`

- [ ] **Step 1: Write the failing TypeScript test for provider-field display values**

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { sessionProviderContext } from "../src/sessionProviderContext.ts";
import type { SessionInfo } from "../src/api.ts";

function sampleSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "s1",
    label: "Claude Code",
    status: "running",
    cwd: "/tmp/repo",
    created_at: "2026-06-22T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    ...overrides,
  };
}

test("sessionProviderContext uses session-scoped provider values", () => {
  assert.deepEqual(
    sessionProviderContext(sampleSession({
      provider: "Claude Code",
      providerModel: "sonnet",
      providerState: "healthy",
    })),
    { provider: "Claude Code", model: "sonnet", state: "healthy" },
  );
});

test("sessionProviderContext falls back to read-only unresolved values", () => {
  assert.deepEqual(
    sessionProviderContext(sampleSession()),
    { provider: "—", model: "—", state: "unknown" },
  );
});
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/sessionProviderContext.test.ts`

Expected: FAIL because `sessionProviderContext.ts` and the new `SessionInfo` provider fields do not exist yet.

- [ ] **Step 3: Add the API fields and the display helper**

```ts
export interface SessionInfo {
  provider?: string;
  providerModel?: string;
  providerState?: ProviderEffectiveState;
  // existing fields
}
```

```ts
import type { SessionInfo } from "./api.ts";

export function sessionProviderContext(session: SessionInfo): {
  provider: string;
  model: string;
  state: string;
} {
  return {
    provider: session.provider ?? "—",
    model: session.providerModel ?? "—",
    state: session.providerState ?? "unknown",
  };
}
```

- [ ] **Step 4: Render the fields in `SessionDetailPanel`**

```tsx
const providerContext = sessionProviderContext(active);
```

```tsx
<div className="session-detail-section">
  <div className="session-detail-label">Provider</div>
  <div className="session-detail-value">{providerContext.provider}</div>
</div>

<div className="session-detail-section">
  <div className="session-detail-label">Model</div>
  <div className="session-detail-value">{providerContext.model}</div>
</div>

<div className="session-detail-section">
  <div className="session-detail-label">State</div>
  <div className="session-detail-value">{providerContext.state}</div>
</div>
```

- [ ] **Step 5: Re-run the TypeScript test**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/sessionProviderContext.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit the session detail provider fields**

```bash
rtk git add apps/desktop/src/api.ts apps/desktop/src/sessionProviderContext.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/sessionProviderContext.test.ts
rtk git commit -m "feat: show provider context in session details"
```

### Task 4: Move Provider Editing Into Settings And Demote The Dockview Surface

**Files:**
- Create: `apps/desktop/src/components/ProviderSettingsSection.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/components/CapacityPanel.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/electron/menuTemplate.ts`
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Write failing renderer/source tests for the Settings migration**

```ts
test("SettingsModal renders provider editing inline instead of an open-panel button", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /ProviderSettingsSection/);
  assert.doesNotMatch(source, /Open Providers Panel/);
  assert.doesNotMatch(source, /onOpenProviders/);
});

test("Dockview keeps capacity as a non-provider surface", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");
  assert.match(source, /capacity.*Capacity/);
  assert.doesNotMatch(source, /capacity.*Providers/);
});
```

- [ ] **Step 2: Run the failing renderer tests**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/dockview.test.ts tests/providersPanel.test.ts`

Expected: FAIL because the Settings modal still references `onOpenProviders` and Dockview still titles the panel `Providers`.

- [ ] **Step 3: Extract the provider editor into a Settings-only component**

```tsx
interface ProviderSettingsSectionProps {
  providerSettings: ProviderSettings | null;
  providerRuntime: ProviderRuntimeResponse | null;
  onSaveProviderSettings: (providers: ProviderSettings) => Promise<void>;
}

export default function ProviderSettingsSection({
  providerSettings,
  providerRuntime,
  onSaveProviderSettings,
}: ProviderSettingsSectionProps) {
  if (!providerSettings) {
    return <div className="settings-section-copy">Loading provider settings…</div>;
  }

  const viewModel = buildProviderViewModel(providerSettings, providerRuntime);
  // move the existing reorder / clear-override controls here unchanged
}
```

```tsx
<div className="settings-section">
  <h3>Providers</h3>
  <p className="settings-section-copy">
    App-wide defaults, overrides, fallback order, and Peon provider models live here.
  </p>
  <ProviderSettingsSection
    providerSettings={initialSettings.providers}
    providerRuntime={providerRuntime}
    onSaveProviderSettings={onSaveProviderSettings}
  />
</div>
```

- [ ] **Step 4: Simplify Dockview and App wiring**

```tsx
// App.tsx
<DockviewApp
  backendStatus={backendStatus}
  workspace={workspace}
  sessions={sessions}
  activeSessionId={activeSessionId}
  resumeTick={resumeTick}
  onSelectSession={handleSelectSession}
  onCreateSession={handleCreateSession}
  onKillSession={handleKillSession}
  onForgetSession={handleForgetSession}
  onResumeSession={handleResumeSession}
  onFocusTerminal={handleFocusTerminal}
  onOpenWorkspace={handleOpenWorkspace}
  dockviewApiRef={dockviewApiRef}
/>
```

```tsx
// SettingsModal invocation
<SettingsModal
  initialSettings={settings}
  providerRuntime={providerRuntime}
  onSaveProviderSettings={saveProviderSettings}
  onClose={() => setSettingsOpen(false)}
  onSaved={setSettings}
/>
```

```tsx
// DockviewApp.tsx
capacity: { component: "capacity", title: "Capacity", position: { referencePanel: "terminal", direction: "right" } },
recommendations: { component: "recommendations", title: "Recommendations", position: { referencePanel: "terminal", direction: "right" } },
```

```tsx
// CapacityPanel.tsx
export default function CapacityPanel() {
  return <EmptyState message="Capacity insights are not implemented yet." />;
}
```

- [ ] **Step 5: Update the menu label and renderer/source tests**

```ts
const panelTitles = {
  sessions: "Sessions",
  detail: "Detail",
  terminal: "Terminal",
  capacity: "Capacity",
  recommendations: "Recommendations",
} as const;
```

```ts
test("ProviderSettingsSection keeps provider editing out of Details", () => {
  const source = readFileSync(new URL("../src/components/ProviderSettingsSection.tsx", import.meta.url), "utf8");
  assert.match(source, /Move up/);
  assert.match(source, /Clear override/);
  assert.match(source, /Last error/);
});
```

- [ ] **Step 6: Re-run the renderer tests**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/dockview.test.ts tests/providersPanel.test.ts tests/sessionProviderContext.test.ts`

Expected: PASS.

- [ ] **Step 7: Commit the Settings migration and Dockview cleanup**

```bash
rtk git add apps/desktop/src/components/ProviderSettingsSection.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/components/CapacityPanel.tsx apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/App.tsx apps/desktop/electron/menuTemplate.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/dockview.test.ts
rtk git commit -m "feat: move provider controls into settings"
```

### Task 5: Verify The Whole Slice And Run The Repo Doc Check

**Files:**
- Modify: none

- [ ] **Step 1: Run the focused desktop test suite**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/dockview.test.ts tests/providersPanel.test.ts tests/sessionProviderContext.test.ts tests/providerSettingsSync.test.ts tests/electronSettingsMemory.test.ts`

Expected: PASS.

- [ ] **Step 2: Run the focused Rust tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 3: Run the repo doc currency hook**

Run: `rtk bash .claude/hooks/doc-check.sh`

Expected: no remaining flagged docs for this change set.

- [ ] **Step 4: Check the final diff**

Run: `rtk git status --short`

Expected: only the intended ADR/doc, Rust, and desktop UI/test files remain modified.

## Self-Review

- Spec coverage: the plan covers the approved `Details` fields (`Provider`, `Model`, `State`), keeps `Details` read-only, keeps editing in Settings, and removes Providers as a primary concept in the main window.
- Placeholder scan: no `TODO`/`TBD` markers remain; every task names exact files, commands, and concrete code changes.
- Type consistency: backend uses `provider`, `providerModel`, and `providerState` in the `/sessions` API; frontend uses the same names in `SessionInfo` and the `sessionProviderContext` helper.
