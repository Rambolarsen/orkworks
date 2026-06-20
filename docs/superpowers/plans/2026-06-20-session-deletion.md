# Session Deletion & Retention Policy — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add permanent session deletion (forget) from disk and background auto-cleanup based on configurable retention policy.

**Architecture:** New `DELETE /sessions/:id/forget` endpoint removes metadata files. New `POST /settings/retention` endpoint receives config from Electron. A `tokio::spawn` background task runs every 5 minutes enforcing max-sessions count and max-age-days limits. Frontend adds a trash icon on remembered/killed sessions and retention inputs in the Settings modal.

**Tech Stack:** Rust (axum, tokio, serde), TypeScript (React, Electron IPC), CSS

---

### Task 1: Rust MetadataStore — delete methods

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Write the failing test**

Add tests inside `mod tests` in `metadata.rs`:

```rust
#[test]
fn delete_session_removes_json_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    let meta = test_metadata("delete-me");
    store.write_session(&meta);
    assert!(store.read_session("delete-me").is_some());

    store.delete_session("delete-me").unwrap();
    assert!(store.read_session("delete-me").is_none());
}

#[test]
fn delete_session_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    // Should not error if file doesn't exist
    assert!(store.delete_session("nonexistent").is_ok());
}

#[test]
fn delete_events_removes_ndjson_and_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    store.append_event("del-test", &Event {
        event_type: "session.created".into(),
        timestamp: "t1".into(),
        status: "creating".into(),
        observed_status: None,
        confidence: None,
    });
    store.append_terminal_output_lines("del-test", &["line 1".into(), "line 2".into()]);

    let ndjson_path = store.events_dir().join("del-test.ndjson");
    let terminal_path = store.events_dir().join("del-test.terminal");
    assert!(ndjson_path.exists());
    assert!(terminal_path.exists());

    store.delete_events("del-test").unwrap();

    assert!(!ndjson_path.exists());
    assert!(!terminal_path.exists());
}

#[test]
fn delete_events_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    assert!(store.delete_events("nonexistent").is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml delete_session_removes_json_file`
Expected: FAIL — `no method named 'delete_session' found`

- [ ] **Step 3: Write minimal implementation**

Add these methods to `impl MetadataStore` block, after `write_session` (line 158):

```rust
pub fn delete_session(&self, id: &str) -> std::io::Result<()> {
    let path = self.sessions_dir().join(format!("{}.json", id));
    match fs::remove_file(&path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn delete_events(&self, id: &str) -> std::io::Result<()> {
    let ndjson_path = self.events_dir().join(format!("{}.ndjson", id));
    let terminal_path = self.terminal_output_path(id);

    if let Err(e) = fs::remove_file(&ndjson_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e);
        }
    }
    if let Err(e) = fs::remove_file(&terminal_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e);
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- delete_session delete_events`
Expected: PASS — 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/metadata.rs
git commit -m "feat(metadata): add delete_session and delete_events methods"
```

---

### Task 2: Rust — forget endpoint

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write the failing integration test**

Add inside the `#[cfg(test)] mod tests` block in `main.rs`:

```rust
#[test]
fn forget_session_removes_metadata_files() {
    let dir = tempfile::tempdir().unwrap();
    let orkworks = dir.path().join(".orkworks");
    std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
    std::fs::create_dir_all(orkworks.join("events")).unwrap();

    let store = metadata::MetadataStore::new(&orkworks);
    let id = "to-forget".to_string();

    // Write session metadata and events
    store.write_session(&metadata::SessionMetadata {
        id: id.clone(),
        label: "Forget Me".into(),
        workspace: dir.path().display().to_string(),
        task: "".into(),
        harness: "".into(),
        model: "".into(),
        cwd: "/tmp".into(),
        status: "killed".into(),
        phase: "".into(),
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        peon_last_inference: None,
        created_at: "now".into(),
        last_activity: "now".into(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        resume: None,
        resumed_from: None,
    });
    store.append_event(&id, &metadata::Event {
        event_type: "session.created".into(),
        timestamp: "now".into(),
        status: "creating".into(),
        observed_status: None,
        confidence: None,
    });

    assert!(orkworks.join("sessions").join("to-forget.json").exists());
    assert!(orkworks.join("events").join("to-forget.ndjson").exists());

    store.delete_session(&id).unwrap();
    store.delete_events(&id).unwrap();

    assert!(!orkworks.join("sessions").join("to-forget.json").exists());
    assert!(!orkworks.join("events").join("to-forget.ndjson").exists());
}
```

- [ ] **Step 2: Run test to verify it passes** (uses already-implemented MetadataStore methods)

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml forget_session_removes_metadata_files`
Expected: PASS

- [ ] **Step 3: Add the `forget_session` handler and route**

In `main.rs`, add after the `delete_session` function (after line 699):

```rust
async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    {
        let sessions = state.sessions.lock().unwrap();
        if sessions.contains_key(&id) {
            return (axum::http::StatusCode::CONFLICT, "Cannot forget a live session. Kill it first.").into_response();
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let ws = match &*ws_guard {
        Some(ws) => ws,
        None => return axum::http::StatusCode::CONFLICT.into_response(),
    };

    if ws.metadata.read_session(&id).is_none() {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    if let Err(e) = ws.metadata.delete_session(&id) {
        tracing::error!("failed to delete session {id}: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(e) = ws.metadata.delete_events(&id) {
        tracing::error!("failed to delete events for {id}: {e}");
        // Non-fatal: metadata already deleted
    }

    axum::http::StatusCode::OK.into_response()
}
```

Register the route in the `Router` (after line 164):

```rust
.route("/sessions/:id/forget", delete(forget_session))
```

- [ ] **Step 4: Verify Rust builds**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml`
Expected: success

- [ ] **Step 5: Verify existing tests still pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all existing tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat(orkworksd): add DELETE /sessions/:id/forget endpoint"
```

---

### Task 3: TypeScript — forgetSession API client

**Files:**
- Modify: `apps/desktop/src/api.ts`

- [ ] **Step 1: Write the failing test**

Add to `apps/desktop/tests/api.test.ts`:

```typescript
test("forgetSession fetches correct URL", async () => {
  const urlPattern = /\/sessions\/test-id\/forget$/;
  assert.equal(urlPattern.test("http://127.0.0.1:1234/sessions/test-id/forget"), true);
});
```

- [ ] **Step 2: Run test to verify it passes** (just tests string pattern, no API change yet)

Run: `node --experimental-strip-types --test tests/api.test.ts`
Expected: PASS

- [ ] **Step 3: Add `forgetSession` function**

Add to `api.ts` after the `deleteSession` function (after line 68):

```typescript
export async function forgetSession(
  baseUrl: string,
  id: string,
): Promise<boolean> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/forget`, {
    method: "DELETE",
  });
  return resp.ok;
}
```

- [ ] **Step 4: Write the test that checks status codes**

Replace the test from Step 1 in `api.test.ts` with:

```typescript
test("forgetSession returns false on 409", async () => {
  const { forgetSession } = await import("../src/api.ts");
  const origFetch = globalThis.fetch;
  globalThis.fetch = (_url: string | URL | Request, _init?: RequestInit) => {
    return Promise.resolve(new Response(null, { status: 409 }));
  };
  try {
    const result = await forgetSession("http://localhost:0", "test-id");
    assert.equal(result, false);
  } finally {
    globalThis.fetch = origFetch;
  }
});

test("forgetSession returns true on 200", async () => {
  const { forgetSession } = await import("../src/api.ts");
  const origFetch = globalThis.fetch;
  globalThis.fetch = (_url: string | URL | Request, _init?: RequestInit) => {
    return Promise.resolve(new Response(null, { status: 200 }));
  };
  try {
    const result = await forgetSession("http://localhost:0", "test-id");
    assert.equal(result, true);
  } finally {
    globalThis.fetch = origFetch;
  }
});
```

- [ ] **Step 5: Run test to verify it passes**

Run: `node --experimental-strip-types --test tests/api.test.ts`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/api.ts apps/desktop/tests/api.test.ts
git commit -m "feat(api): add forgetSession client function"
```

---

### Task 4: UI — forget button in session list

**Files:**
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Add `onForgetSession` to SessionListPanel props and render trash icon**

In `SessionListPanel.tsx`, add to the interface (line 19):

```typescript
  onForgetSession: (id: string) => void;
```

In the JSX, inside the `.session-row-meta` div, after the kill button (line 193), add:

```typescript
                      {s.memoryState !== "live" && (
                        <button
                          className="session-row-forget"
                          type="button"
                          aria-label="Delete session"
                          onClick={(e) => {
                            e.stopPropagation();
                            onForgetSession(s.id);
                          }}
                        >
                          &#x1F5D1;
                        </button>
                      )}
```

- [ ] **Step 2: Add CSS for forget button**

In `App.css`, after the `.session-row-kill:hover` rule (around line 433), add:

```css
.session-row-forget {
  border: none;
  background: none;
  color: var(--text-faint);
  font-size: 12px;
  line-height: 1;
  cursor: pointer;
  padding: 0 var(--space-2);
  opacity: 0.6;
}

.session-row-forget:hover {
  color: var(--state-error);
  opacity: 1;
}
```

- [ ] **Step 3: Wire `onForgetSession` through DockviewApp**

In `DockviewApp.tsx`, add to the `DockviewAppData` interface (line 24):

```typescript
  onForgetSession: (id: string) => void;
```

In the `SessionsPanel` component, pass the new prop (line 42):

```typescript
      onForgetSession={ctx.onForgetSession}
```

In the `DockviewApp` function destructuring (line 150), add `onForgetSession`. In the context value (line 152), add it.

- [ ] **Step 4: Wire through App.tsx**

In `App.tsx`, add import for `forgetSession` (line 16):

```typescript
  forgetSession,
```

Add the handler after `handleKillSession` (after line 147):

```typescript
  const handleForgetSession = useCallback(
    async (id: string) => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        const ok = await forgetSession(baseUrl, id);
        if (!ok) {
          pushToast("warn", "Session is still live — kill it first.");
          return;
        }
        disposeTerminal(id);
        if (activeSessionId === id) setActiveSessionId(null);
        await refreshSessions();
      } catch {
        pushToast("error", "Couldn't delete session.");
      }
    },
    [activeSessionId, refreshSessions],
  );
```

Pass `onForgetSession={handleForgetSession}` in the `<DockviewApp>` props (around line 338).

- [ ] **Step 5: Run build check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no TypeScript errors

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/components/SessionListPanel.tsx apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(ui): add forget/delete button for remembered sessions"
```

---

### Task 5: Settings — retention types and defaults

**Files:**
- Modify: `apps/desktop/src/appSettingsTypes.ts`
- Modify: `apps/desktop/electron/settingsMemory.ts`

- [ ] **Step 1: Add `RetentionSettings` type and extend `AppSettings`**

In `appSettingsTypes.ts`, add after line 9:

```typescript
export interface RetentionSettings {
  maxSessions: number;
  maxAgeDays: number;
}
```

In the `AppSettings` interface, add:

```typescript
  retention: RetentionSettings;
```

- [ ] **Step 2: Add `DEFAULT_RETENTION` to Electron settings memory**

In `electron/settingsMemory.ts`, add after line 84:

```typescript
export const DEFAULT_RETENTION = {
  maxSessions: 0,
  maxAgeDays: 0,
};
```

In the `AppSettings` interface in `electron/settingsMemory.ts` (line 4), add:

```typescript
  retention: typeof DEFAULT_RETENTION;
```

In `DEFAULT_SETTINGS` (line 86), add:

```typescript
  retention: { ...DEFAULT_RETENTION },
```

In `defaultSettings()` (line 251), add the same:

```typescript
    retention: { ...DEFAULT_RETENTION },
```

In `normalizeSettings()` (line 166), the spread `...parsed` already preserves unknown keys including `retention`, but we need to normalize it explicitly. Add after the `return` statement in `normalizeSettings`:

```typescript
export function normalizeRetention(value: unknown): typeof DEFAULT_RETENTION {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ...DEFAULT_RETENTION };
  }
  const raw = value as Record<string, unknown>;
  return {
    maxSessions: clampInt(raw.maxSessions, 0, 999, 0),
    maxAgeDays: clampInt(raw.maxAgeDays, 0, 999, 0),
  };
}

function clampInt(v: unknown, min: number, max: number, fallback: number): number {
  if (typeof v !== "number" || !Number.isFinite(v)) return fallback;
  return Math.max(min, Math.min(max, Math.round(v)));
}
```

And update `normalizeSettings` to use it:

```typescript
  return {
    ...parsed,
    version: 1,
    hotkeys: normalizeHotkeys(parsed.hotkeys),
    retention: normalizeRetention(parsed.retention),
  };
```

- [ ] **Step 3: Update renderer-side `AppSettings` to match**

In `src/appSettingsTypes.ts`, ensure the `AppSettings` interface has `retention` as required:

```typescript
export interface AppSettings {
  [key: string]: unknown;
  version: 1;
  hotkeys: HotkeySettings;
  defaultHotkeys: HotkeySettings;
  retention: RetentionSettings;
}
```

- [ ] **Step 4: Test settings persistence**

Run: `node --experimental-strip-types --test tests/electronSettingsMemory.test.ts`
Expected: all tests pass (including the "preserves future top-level settings sections" test which confirms `retention` persists)

- [ ] **Step 5: Run type check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/appSettingsTypes.ts apps/desktop/electron/settingsMemory.ts
git commit -m "feat(settings): add RetentionSettings type with defaults"
```

---

### Task 6: Settings — IPC bridge for retention

**Files:**
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`

- [ ] **Step 1: Add `saveRetention` to preload**

In `preload.ts`, add after line 10:

```typescript
  saveRetention: (retention: unknown): Promise<unknown> => ipcRenderer.invoke("save-retention", retention),
```

- [ ] **Step 2: Add `saveRetention` to Window type**

In `orkworksWindow.d.ts`, add the import for `RetentionSettings`:

```typescript
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
```

Add to the `orkworks` interface after `saveHotkeys`:

```typescript
      saveRetention: (retention: RetentionSettings) => Promise<{ ok: boolean }>;
```

- [ ] **Step 3: Add IPC handler in main.ts**

In `electron/main.ts`, add import for `DEFAULT_RETENTION`:

```typescript
import { DEFAULT_HOTKEYS, DEFAULT_RETENTION, readSettings, settingsWithHotkeys, validateHotkeys, writeSettings } from "./settingsMemory";
```

Add the IPC handler after `save-hotkeys` handler (after line 186):

```typescript
  ipcMain.handle("save-retention", async (_event, retention: unknown) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const normalizedRetention = (() => {
      if (!retention || typeof retention !== "object" || Array.isArray(retention)) {
        return { ...DEFAULT_RETENTION };
      }
      const raw = retention as Record<string, unknown>;
      return {
        maxSessions: typeof raw.maxSessions === "number" && Number.isFinite(raw.maxSessions)
          ? Math.max(0, Math.min(999, Math.round(raw.maxSessions as number)))
          : DEFAULT_RETENTION.maxSessions,
        maxAgeDays: typeof raw.maxAgeDays === "number" && Number.isFinite(raw.maxAgeDays)
          ? Math.max(0, Math.min(999, Math.round(raw.maxAgeDays as number)))
          : DEFAULT_RETENTION.maxAgeDays,
      };
    })();

    const nextSettings: AppSettings = {
      ...baseSettings,
      version: 1,
      retention: normalizedRetention,
    };

    writeSettings(app.getPath("userData"), nextSettings);
    currentSettings = nextSettings;

    // Push to sidecar
    try {
      const port = await portPromise;
      await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(normalizedRetention),
      });
    } catch {
      console.warn("[main] failed to push retention to sidecar (will retry on next save)");
    }

    return { ok: true };
  });
```

Add push-to-sidecar on startup. After `applyMenu(createMenu(currentSettings))` (line 265), add:

```typescript
  // Push retention config to sidecar on startup
  portPromise.then(async (port) => {
    try {
      const retention = currentSettings?.retention ?? DEFAULT_RETENTION;
      await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(retention),
      });
    } catch {
      // Sidecar may not be ready yet; will be pushed on next save-retention
    }
  });
```

- [ ] **Step 4: Run type check and lint**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/electron/preload.ts apps/desktop/electron/main.ts apps/desktop/src/orkworksWindow.d.ts
git commit -m "feat(settings): add saveRetention IPC and push to sidecar"
```

---

### Task 7: Settings — retention UI in SettingsModal

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Add retention inputs to SettingsModal**

In `SettingsModal.tsx`, add import:

```typescript
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "../appSettingsTypes";
```

Add retention state after `saveError`:

```typescript
  const [retention, setRetention] = useState<RetentionSettings>(initialSettings.retention);
  const [retentionSaveStatus, setRetentionSaveStatus] = useState<string | null>(null);
```

Add an effect or modify `save()` to also save retention. Since retention saves separately (it pushes to sidecar), add a helper:

```typescript
  async function saveRetention(rt: RetentionSettings) {
    setRetentionSaveStatus(null);
    try {
      await window.orkworks.saveRetention(rt);
      setRetentionSaveStatus("Saved");
    } catch {
      setRetentionSaveStatus("Couldn't save retention settings.");
    }
  }
```

In the JSX, add after the `settings-section` div containing hotkeys (after line 125):

```tsx
        <div className="settings-section">
          <h3>Session Retention</h3>
          <p className="settings-section-copy">
            Live sessions are never auto-deleted. Changes apply immediately.
          </p>

          <div className="retention-list">
            <div className="retention-row">
              <div className="retention-label">Max sessions to keep</div>
              <input
                className="retention-input"
                type="number"
                min={0}
                max={999}
                value={retention.maxSessions}
                onChange={(e) => {
                  const v = parseInt(e.target.value, 10);
                  if (!Number.isNaN(v)) {
                    const rt = { ...retention, maxSessions: Math.max(0, Math.min(999, v)) };
                    setRetention(rt);
                  }
                }}
                onBlur={() => saveRetention(retention)}
              />
              <span className="retention-hint">0 = unlimited</span>
            </div>

            <div className="retention-row">
              <div className="retention-label">Auto-delete sessions older than (days)</div>
              <input
                className="retention-input"
                type="number"
                min={0}
                max={999}
                value={retention.maxAgeDays}
                onChange={(e) => {
                  const v = parseInt(e.target.value, 10);
                  if (!Number.isNaN(v)) {
                    const rt = { ...retention, maxAgeDays: Math.max(0, Math.min(999, v)) };
                    setRetention(rt);
                  }
                }}
                onBlur={() => saveRetention(retention)}
              />
              <span className="retention-hint">0 = never</span>
            </div>
          </div>

          {retentionSaveStatus && (
            <div className={`retention-status ${retentionSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
              {retentionSaveStatus}
            </div>
          )}
        </div>
```

- [ ] **Step 2: Add CSS for retention inputs**

In `App.css`, after the `.settings-save-error` rule (around line 222), add:

```css
.retention-list {
  margin-top: 14px;
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  overflow: hidden;
}

.retention-row {
  display: grid;
  grid-template-columns: minmax(180px, 1fr) 80px auto;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.retention-row:last-child {
  border-bottom: 0;
}

.retention-label {
  color: var(--text-secondary);
  font-weight: 600;
}

.retention-input {
  width: 100%;
  border: 1px solid var(--border-default);
  border-radius: 5px;
  padding: 4px 7px;
  color: var(--text-primary);
  background: var(--surface-0);
  font: inherit;
  font-size: 13px;
}

.retention-input:focus {
  outline: none;
  border-color: var(--accent);
}

.retention-hint {
  color: var(--text-faint);
  font-size: 11px;
}

.retention-status {
  margin-top: 8px;
  color: var(--state-warn);
  font-size: 12px;
}

.retention-status--ok {
  color: var(--state-ok);
}
```

- [ ] **Step 3: Run type check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/App.css
git commit -m "feat(settings): add retention UI to Settings modal"
```

---

### Task 8: Rust — retention endpoint and background cleanup

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Add `RetentionConfig` struct and extend `AppState`**

In `main.rs`, before the `AppState` struct (before line 116):

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RetentionConfig {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}
```

Add to `AppState` (after line 120):

```rust
    retention_config: tokio::sync::RwLock<RetentionConfig>,
```

Update the `AppState` construction in `main()` (line 133) to include:

```rust
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
```

Update all test `AppState` constructions — search for `AppState {` and add:

```rust
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
```

- [ ] **Step 2: Add `POST /settings/retention` handler**

After the `forget_session` function:

```rust
#[derive(Deserialize)]
struct RetentionRequest {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

async fn set_retention(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetentionRequest>,
) -> impl IntoResponse {
    let mut config = state.retention_config.write().await;
    config.max_sessions = req.max_sessions;
    config.max_age_days = req.max_age_days;
    tracing::info!("retention config updated: max_sessions={} max_age_days={}", config.max_sessions, config.max_age_days);
    axum::http::StatusCode::OK
}
```

Register the route:

```rust
.route("/settings/retention", post(set_retention))
```

- [ ] **Step 3: Add background cleanup task**

Add the cleanup function before `main()`:

```rust
async fn retention_cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        let config = state.retention_config.read().await.clone();
        if config.max_sessions == 0 && config.max_age_days == 0 {
            continue;
        }

        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            continue;
        };

        let mut all_sessions = ws.metadata.read_all_sessions();
        drop(ws_guard);

        // Exclude live sessions
        let live_ids: std::collections::HashSet<String> = {
            let sessions = state.sessions.lock().unwrap();
            sessions.keys().cloned().collect()
        };
        all_sessions.retain(|s| !live_ids.contains(&s.id));

        if all_sessions.is_empty() {
            continue;
        }

        // Sort by last_activity, oldest first
        all_sessions.sort_by(|a, b| a.last_activity.cmp(&b.last_activity));

        // Age check
        if config.max_age_days > 0 {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(config.max_age_days as i64);
            let mut expired: Vec<String> = Vec::new();
            for s in &all_sessions {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&s.last_activity) {
                    if parsed < cutoff {
                        expired.push(s.id.clone());
                    }
                }
            }
            for id in &expired {
                let ws_guard = state.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    tracing::info!("retention: deleting expired session {id}");
                    let _ = ws.metadata.delete_session(id);
                    let _ = ws.metadata.delete_events(id);
                }
            }
            all_sessions.retain(|s| !expired.contains(&s.id));
        }

        // Count check
        if config.max_sessions > 0 && all_sessions.len() > config.max_sessions {
            let to_delete = all_sessions.len() - config.max_sessions;
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                for s in all_sessions.iter().take(to_delete) {
                    tracing::info!("retention: deleting session {} (exceeds max {})", s.id, config.max_sessions);
                    let _ = ws.metadata.delete_session(&s.id);
                    let _ = ws.metadata.delete_events(&s.id);
                }
            }
        }
    }
}
```

Spawn the task in `main()`. After the existing Peon spawn block (after line 151), add:

```rust
    // Start retention cleanup background task
    {
        let retention_state = state.clone();
        tokio::spawn(async move {
            retention_cleanup_task(retention_state).await;
        });
    }
```

- [ ] **Step 4: Verify Rust builds**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml`
Expected: success

- [ ] **Step 5: Run all Rust tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat(orkworksd): add retention config endpoint and background cleanup task"
```

---

### Task 9: Verification

- [ ] **Step 1: Run full Rust test suite**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests pass

- [ ] **Step 2: Run TypeScript type check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors

- [ ] **Step 3: Run frontend tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all tests pass

- [ ] **Step 4: Run doc-check**

```bash
bash .claude/hooks/doc-check.sh
```

Address any flagged files.

- [ ] **Step 5: Commit final verification**

```bash
git add -A
git commit -m "chore: post-implementation verification"
```
