# Terminal Replay Size Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop dead-session terminal replay from mid-word-splitting lines by recording the PTY's size at the moment a session dies and replaying at that exact size (scaled visually to fit the panel), instead of re-wrapping to today's panel width.

**Architecture:** The sidecar persists the last known PTY `cols`/`rows` to a small sidecar file (`<id>.terminal-size`, next to the existing `<id>.terminal` file) the moment a session transitions to `killed`/`ended`/`error`, and serves it alongside the existing terminal-output replay payload. The desktop app's `HistoricalTerminal` component constructs xterm at that fixed size (no reflow) and applies a CSS `transform: scale()` to fit the panel. Sessions with no recorded size (legacy, pre-fix) keep today's fit-to-container behavior unchanged.

**Tech Stack:** Rust (axum, serde) sidecar in `crates/orkworksd`; TypeScript/React + xterm.js in `apps/desktop`.

## Global Constraints

- Follow existing code conventions exactly: `MetadataStore` methods use the private-helper-plus-public-method pattern already used for `.terminal` files (`terminal_output_path` → `append_terminal_output_records`/`read_terminal_output`).
- Do **not** add fields to `SessionMetadata` — it is built as an exhaustive struct literal at ~30 call sites (`main.rs`, `session_view.rs`, `runtime/terminal_runtime.rs`, `runtime/peon_runtime.rs`, `http/session_handlers.rs`); a sidecar file avoids touching any of them.
- No new dependencies (frontend or Rust).
- No CSS file changes — `.terminal-container .xterm { width: 100%; height: 100%; }` in `apps/desktop/src/App.css:544` stays untouched; the fixed-size replay path overrides sizing via inline styles set from TypeScript, which win over the stylesheet rule automatically.
- Legacy sessions (no recorded size) must render exactly as they do today — same `FitAddon`/`ResizeObserver` code path, unchanged.
- Every step that changes code must leave the affected test suite green before moving to the next step.
- Rust test command: `cargo test --manifest-path crates/orkworksd/Cargo.toml <test_name>` (or omit `<test_name>` to run the whole suite).
- TypeScript test command: `node --experimental-strip-types --test tests/<file>.test.ts` from `apps/desktop/` (or `tests/*.test.ts tests/*.test.mjs` for the whole suite).
- Type-check command: `npx tsc --noEmit` from `apps/desktop/`.
- All work happens in the `terminal-replay-size` worktree at `/Users/froomiebot/workspace/orkworks-terminal-replay-size` (branch `terminal-replay-size`, based on `origin/main`). Every `cd`/command below assumes that root unless stated otherwise.

---

### Task 1: `MetadataStore` terminal-size sidecar file

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs:1675-1696` (insert new methods before the closing `}` of `impl MetadataStore`, right after `trim_terminal_output`)
- Modify: `crates/orkworksd/src/metadata.rs` (insert new tests before the `terminal_output_tail_keeps_everything_under_both_budgets` test, currently at line 3689)

**Interfaces:**
- Produces: `MetadataStore::write_terminal_size(&self, id: &str, cols: u16, rows: u16)`, `MetadataStore::read_terminal_size(&self, id: &str) -> Option<(u16, u16)>` (tuple order is always `(cols, rows)`) — consumed by Task 2 (write) and Task 3 (read).

- [ ] **Step 1: Write the failing tests**

In `crates/orkworksd/src/metadata.rs`, find this exact block (currently starting at line 3689):

```rust
    #[test]
    fn terminal_output_tail_keeps_everything_under_both_budgets() {
```

Insert the following two tests immediately before it:

```rust
    #[test]
    fn terminal_size_round_trips_through_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        assert_eq!(store.read_terminal_size("no-size-yet"), None);

        store.write_terminal_size("sized-session", 120, 40);

        assert_eq!(store.read_terminal_size("sized-session"), Some((120, 40)));
    }

    #[test]
    fn terminal_size_treats_malformed_or_zero_content_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let path = store.terminal_size_path("malformed-session");
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        fs::write(&path, "not-a-size").unwrap();
        assert_eq!(store.read_terminal_size("malformed-session"), None);

        fs::write(&path, "0x40").unwrap();
        assert_eq!(store.read_terminal_size("malformed-session"), None);

        fs::write(&path, "120x0").unwrap();
        assert_eq!(store.read_terminal_size("malformed-session"), None);
    }

```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_size_round_trips_through_write_and_read`
Expected: compile error — `no method named 'write_terminal_size' found for struct 'MetadataStore'` (and similarly for `read_terminal_size`/`terminal_size_path`).

- [ ] **Step 3: Implement the sidecar methods**

In `crates/orkworksd/src/metadata.rs`, find the exact closing of `impl MetadataStore` (currently lines 1675-1696):

```rust
    pub fn trim_terminal_output(&self, id: &str, max_lines: usize) {
        let path = self.terminal_output_path(id);
        let Ok(tail) =
            read_terminal_output_tail(&path, max_lines, TERMINAL_OUTPUT_TRIM_TARGET_BYTES)
        else {
            return;
        };
        if !tail.discarded {
            return;
        }
        let content = tail
            .marker
            .into_iter()
            .chain(tail.physical)
            .flatten()
            .collect::<Vec<_>>();
        match fs::write(&path, content) {
            Ok(_) => {}
            Err(e) => warn!("failed to trim terminal output for {id}: {e}"),
        }
    }
}
```

Replace it with (adding three new methods before the closing `}`):

```rust
    pub fn trim_terminal_output(&self, id: &str, max_lines: usize) {
        let path = self.terminal_output_path(id);
        let Ok(tail) =
            read_terminal_output_tail(&path, max_lines, TERMINAL_OUTPUT_TRIM_TARGET_BYTES)
        else {
            return;
        };
        if !tail.discarded {
            return;
        }
        let content = tail
            .marker
            .into_iter()
            .chain(tail.physical)
            .flatten()
            .collect::<Vec<_>>();
        match fs::write(&path, content) {
            Ok(_) => {}
            Err(e) => warn!("failed to trim terminal output for {id}: {e}"),
        }
    }

    fn terminal_size_path(&self, id: &str) -> PathBuf {
        self.events_dir().join(format!("{}.terminal-size", id))
    }

    /// Records the PTY's last known size for a session, once, at the moment
    /// it reaches a terminal status. This is the only write path — resize
    /// events during a live session are not persisted, since replay only
    /// ever needs the final size.
    pub fn write_terminal_size(&self, id: &str, cols: u16, rows: u16) {
        if let Err(e) = fs::create_dir_all(&self.events_dir()) {
            warn!("failed to create events dir for terminal size: {e}");
            return;
        }
        let path = self.terminal_size_path(id);
        if let Err(e) = fs::write(&path, format!("{cols}x{rows}")) {
            warn!("failed to write terminal size for {id}: {e}");
        }
    }

    /// Reads back the size written by `write_terminal_size`. Returns `None`
    /// for sessions with no recorded size (legacy sessions from before this
    /// existed) and for any malformed or zero-valued content, so callers can
    /// treat both cases identically as "size unknown".
    pub fn read_terminal_size(&self, id: &str) -> Option<(u16, u16)> {
        let path = self.terminal_size_path(id);
        let content = fs::read_to_string(&path).ok()?;
        let (cols_str, rows_str) = content.trim().split_once('x')?;
        let cols: u16 = cols_str.parse().ok()?;
        let rows: u16 = rows_str.parse().ok()?;
        if cols == 0 || rows == 0 {
            return None;
        }
        Some((cols, rows))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_size_`
Expected: both `terminal_size_round_trips_through_write_and_read` and `terminal_size_treats_malformed_or_zero_content_as_absent` PASS.

- [ ] **Step 5: Run the full Rust suite to check for regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests PASS (no regressions from the new methods, which are additive-only).

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/metadata.rs
git commit -m "feat: add terminal-size sidecar to MetadataStore"
```

---

### Task 2: Persist size in `set_session_status`

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:539-663` (the `set_session_status` function)
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs` (new test in the `mod tests` block)

**Interfaces:**
- Consumes: `MetadataStore::write_terminal_size` from Task 1.
- Produces: `set_session_status` now writes `<id>.terminal-size` once, whenever it transitions a session (that still has a live in-memory handle) into `killed`/`ended`/`error`. No other behavior of `set_session_status` changes — Task 3 and later tasks read the file this writes.

- [ ] **Step 1: Write the failing test**

In `crates/orkworksd/src/runtime/terminal_runtime.rs`, inside `mod tests` (which already has `use super::*; use crate::test_support::*;` at the top, giving access to `test_app_state_with_workspace`, `test_session_info`, `test_session_metadata`, and `set_session_status` itself), find the exact end of the `set_session_status_updates_registry` test (currently ending at line 2060):

```rust
        set_session_status(&state, "test-2", "ended");
        let ended = state
            .sessions
            .lock()
            .unwrap()
            .get("test-2")
            .unwrap()
            .info
            .clone();
        assert_eq!(ended.status, "running");
        assert_eq!(ended.lifecycle_phase, "ending");
    }
```

Insert the new test immediately after that closing `}`:

```rust
    #[test]
    fn set_session_status_persists_terminal_size_on_terminal_transition() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "sized-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&test_session_metadata(
                &id,
                "Sized",
                dir.path().display().to_string(),
                "running",
                "t0",
                "t0",
            ));
        }

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(id.clone(), "Sized", "/tmp", "running", "t0"),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(40, 120),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        set_session_status(&state, &id, "ended");

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        assert_eq!(ws.metadata.read_terminal_size(&id), Some((120, 40)));
    }
```

(`SessionRuntime::detached(rows, cols)` takes rows first, cols second — this creates a runtime with `last_rows = 40`, `last_cols = 120`, so the expected sidecar content is `cols=120, rows=40`.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml set_session_status_persists_terminal_size_on_terminal_transition`
Expected: FAIL — `assert_eq!` left is `None`, right is `Some((120, 40))` (nothing writes the sidecar file yet).

- [ ] **Step 3: Thread the captured size through `set_session_status`**

In `crates/orkworksd/src/runtime/terminal_runtime.rs`, find the exact current function opening (lines 539-580):

```rust
pub(crate) fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) -> bool {
    let is_terminal = matches!(status, "killed" | "ended" | "error");
    let (handle_decision, session_resume, entered_running, entered_terminal) = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            let entered_running =
                !is_terminal && status == "running" && handle.info.status != "running";
            if is_terminal && matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended") {
                return false;
            }
            if is_terminal {
                handle.info.status = "running".to_string();
                handle.info.lifecycle_phase = "ending".to_string();
                handle.info.lifecycle = "stopping".to_string();
                handle.info.attention = None;
                handle.info.connectivity = Some(connectivity_for_status("running").to_string());
                handle.info.terminal_outcome = None;
            } else {
                handle.info.status = status.to_string();
                handle.info.lifecycle_phase = if status == "creating" {
                    "creating".to_string()
                } else {
                    "active".to_string()
                };
                handle.info.lifecycle = if status == "creating" {
                    "creating"
                } else {
                    "alive"
                }
                .to_string();
                handle.info.connectivity = Some(connectivity_for_status(status).to_string());
                handle.info.terminal_outcome = terminal_outcome_for_status(status);
            }
            handle.info.last_activity_at = Some(iso_now());
            if is_terminal {
                handle.info.observed_status = None;
            }
            (
                Some(true),
                (handle.info.resume.clone(), handle.info.resumed_from.clone()),
                entered_running,
                is_terminal,
            )
        } else {
            (None, (None, None), false, false)
        }
    };
```

Replace the tuple destructuring line, the returned tuples, and nothing else, so it reads:

```rust
pub(crate) fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) -> bool {
    let is_terminal = matches!(status, "killed" | "ended" | "error");
    let (handle_decision, session_resume, entered_running, entered_terminal, terminal_size) = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            let entered_running =
                !is_terminal && status == "running" && handle.info.status != "running";
            if is_terminal && matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended") {
                return false;
            }
            if is_terminal {
                handle.info.status = "running".to_string();
                handle.info.lifecycle_phase = "ending".to_string();
                handle.info.lifecycle = "stopping".to_string();
                handle.info.attention = None;
                handle.info.connectivity = Some(connectivity_for_status("running").to_string());
                handle.info.terminal_outcome = None;
            } else {
                handle.info.status = status.to_string();
                handle.info.lifecycle_phase = if status == "creating" {
                    "creating".to_string()
                } else {
                    "active".to_string()
                };
                handle.info.lifecycle = if status == "creating" {
                    "creating"
                } else {
                    "alive"
                }
                .to_string();
                handle.info.connectivity = Some(connectivity_for_status(status).to_string());
                handle.info.terminal_outcome = terminal_outcome_for_status(status);
            }
            handle.info.last_activity_at = Some(iso_now());
            if is_terminal {
                handle.info.observed_status = None;
            }
            let terminal_size =
                is_terminal.then(|| (handle.runtime.last_cols, handle.runtime.last_rows));
            (
                Some(true),
                (handle.info.resume.clone(), handle.info.resumed_from.clone()),
                entered_running,
                is_terminal,
                terminal_size,
            )
        } else {
            (None, (None, None), false, false, None)
        }
    };
```

Then find the workspace-write block immediately below (unchanged context, currently starting around line 592):

```rust
    let now = iso_now();
    let mut applied = handle_decision.unwrap_or(false);
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
```

Insert the sidecar write right after `if let Some(ref ws) = *ws_guard {` and before the `read_session` call:

```rust
    let now = iso_now();
    let mut applied = handle_decision.unwrap_or(false);
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some((cols, rows)) = terminal_size {
            ws.metadata.write_terminal_size(id, cols, rows);
        }
        if let Some(mut meta) = ws.metadata.read_session(id) {
```

Everything else in the function (the rest of the `if let Some(mut meta) = ...` block, the `if applied { ... }` event-append block, and the closing `applied`) stays exactly as-is.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml set_session_status_persists_terminal_size_on_terminal_transition`
Expected: PASS.

- [ ] **Step 5: Run the full Rust suite to check for regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests PASS, including the existing `set_session_status_updates_registry`, `set_session_status_seeds_peon_last_output_when_session_enters_running`, and `set_session_status_running_does_not_reset_existing_peon_last_output` tests (these construct `AppState` with `workspace: Mutex::new(None)`, so the new sidecar-write branch is simply never reached for them — confirm this stays true).

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "feat: persist recorded terminal size when a session ends"
```

---

### Task 3: Serve size from `get_terminal_output`

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs:14-16` (`TerminalOutputResponse` struct)
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs:31-49` (`get_terminal_output` handler)
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs` (new tests in the existing `mod tests` block)

**Interfaces:**
- Consumes: `MetadataStore::read_terminal_size` from Task 1.
- Produces: `GET /sessions/:id/terminal-output` now includes optional `cols`/`rows` (u16) top-level JSON fields alongside the existing `lines` array, omitted entirely when no size was recorded. Consumed by Task 4 (`apps/desktop/src/api.ts`).

- [ ] **Step 1: Write the failing tests**

In `crates/orkworksd/src/runtime/terminal_http.rs`, inside `mod tests`, find the exact end of the `get_terminal_output_reads_persisted_terminal_history_for_dead_session` test (currently lines 155-167):

```rust
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["lines"],
            serde_json::json!([{"text": "first line", "delimiter": "\r\n"}])
        );
    }

    #[tokio::test]
    async fn get_terminal_output_returns_legacy_strings_for_legacy_history() {
```

Insert the two new tests between the closing `}` of the first test and the `#[tokio::test]` of the next one, so the block reads:

```rust
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["lines"],
            serde_json::json!([{"text": "first line", "delimiter": "\r\n"}])
        );
    }

    #[tokio::test]
    async fn get_terminal_output_includes_recorded_size_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "sized-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.append_terminal_output_records(
                &session_id,
                &[metadata::TerminalOutputRecord::raw("line", "\r\n")],
            );
            ws.metadata.write_terminal_size(&session_id, 120, 40);
        }

        let payload =
            response_json(get_terminal_output(State(state), Path(session_id)).await).await;

        assert_eq!(payload["cols"], serde_json::json!(120));
        assert_eq!(payload["rows"], serde_json::json!(40));
    }

    #[tokio::test]
    async fn get_terminal_output_omits_size_when_not_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "unsized-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.append_terminal_output_records(
                &session_id,
                &[metadata::TerminalOutputRecord::raw("line", "\r\n")],
            );
        }

        let payload =
            response_json(get_terminal_output(State(state), Path(session_id)).await).await;

        assert!(payload.get("cols").is_none());
        assert!(payload.get("rows").is_none());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml get_terminal_output_includes_recorded_size_when_present get_terminal_output_omits_size_when_not_recorded`
Expected: `get_terminal_output_includes_recorded_size_when_present` FAILs (`payload["cols"]` is `Value::Null`, not `120`); `get_terminal_output_omits_size_when_not_recorded` passes trivially today (irrelevant until the field exists) but is included now so it stays pinned once the field is added.

- [ ] **Step 3: Implement**

Replace the current struct (lines 14-16):

```rust
#[derive(Serialize)]
pub(crate) struct TerminalOutputResponse {
    pub(crate) lines: Vec<metadata::TerminalOutputRecord>,
}
```

with:

```rust
#[derive(Serialize)]
pub(crate) struct TerminalOutputResponse {
    pub(crate) lines: Vec<metadata::TerminalOutputRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cols: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rows: Option<u16>,
}
```

Replace the current handler (lines 31-49):

```rust
pub(crate) async fn get_terminal_output(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    let id_clone = id.clone();
    let lines = tokio::task::spawn_blocking(move || {
        let ws_guard = state_clone.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) => ws
                .metadata
                .read_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES),
            None => Vec::new(),
        }
    })
    .await
    .unwrap_or_else(|error| {
        tracing::error!(%error, "terminal-output metadata task failed");
        Vec::new()
    });
    Json(TerminalOutputResponse { lines })
}
```

with:

```rust
pub(crate) async fn get_terminal_output(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    let id_clone = id.clone();
    let (lines, size) = tokio::task::spawn_blocking(move || {
        let ws_guard = state_clone.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) => (
                ws.metadata
                    .read_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES),
                ws.metadata.read_terminal_size(&id_clone),
            ),
            None => (Vec::new(), None),
        }
    })
    .await
    .unwrap_or_else(|error| {
        tracing::error!(%error, "terminal-output metadata task failed");
        (Vec::new(), None)
    });
    Json(TerminalOutputResponse {
        lines,
        cols: size.map(|(cols, _)| cols),
        rows: size.map(|(_, rows)| rows),
    })
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml get_terminal_output_`
Expected: all `get_terminal_output_*` tests PASS, including the two pre-existing ones (`get_terminal_output_reads_persisted_terminal_history_for_dead_session`, `get_terminal_output_returns_legacy_strings_for_legacy_history`) — they only assert on `payload["lines"]`, so the new optional fields don't affect them.

- [ ] **Step 5: Run the full Rust suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/runtime/terminal_http.rs
git commit -m "feat: serve recorded terminal size from terminal-output endpoint"
```

---

### Task 4: `api.ts` — extend `getTerminalOutput`

**Files:**
- Modify: `apps/desktop/src/api.ts:201-211`

**Interfaces:**
- Consumes: the `cols`/`rows` JSON fields from Task 3.
- Produces: `getTerminalOutput(baseUrl: string, id: string): Promise<{ lines: TerminalOutputRecord[]; cols?: number; rows?: number }>` — the `lines` field replaces the previous bare-array return; consumed by Task 6 (`terminalStore.ts`) and Task 8 (`HistoricalTerminal.tsx`, via Task 5's `loadTerminalReplay`).

There is no separate test file for `api.ts` fetch wrappers in this codebase (no network mocking is set up for them) — this task is verified by the type-check in Step 2 and by the downstream tasks that consume the new shape.

- [ ] **Step 1: Change the return shape**

In `apps/desktop/src/api.ts`, replace (lines 201-211):

```ts
export type TerminalOutputRecord = string | { text: string; delimiter: string };

export async function getTerminalOutput(
  baseUrl: string,
  id: string,
): Promise<TerminalOutputRecord[]> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/terminal-output`);
  if (!resp.ok) throw new Error(`get terminal output failed: ${resp.status}`);
  const data = await resp.json();
  return data.lines ?? [];
}
```

with:

```ts
export type TerminalOutputRecord = string | { text: string; delimiter: string };

export async function getTerminalOutput(
  baseUrl: string,
  id: string,
): Promise<{ lines: TerminalOutputRecord[]; cols?: number; rows?: number }> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/terminal-output`);
  if (!resp.ok) throw new Error(`get terminal output failed: ${resp.status}`);
  const data = await resp.json();
  return { lines: data.lines ?? [], cols: data.cols, rows: data.rows };
}
```

- [ ] **Step 2: Type-check (expect new failures in downstream files — that's the point)**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: FAILs in `src/terminalStore.ts` and `src/terminalReplay.ts`/`src/components/HistoricalTerminal.tsx` (they still expect an array). This is expected — Tasks 5, 6, and 8 fix each consumer.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/api.ts
git commit -m "feat: return recorded terminal size from getTerminalOutput"
```

(Committing here despite the type-check failure is intentional and matches this plan's task boundaries — each task is a focused, reviewable diff; the type-check is green again by the end of Task 8. If your workflow requires green-at-every-commit, squash Tasks 4-8 before merge.)

---

### Task 5: `terminalReplay.ts` — extend `loadTerminalReplay`

**Files:**
- Modify: `apps/desktop/src/terminalReplay.ts` (whole file, 33 lines)
- Modify: `apps/desktop/tests/terminalReplay.test.ts` (whole file)

**Interfaces:**
- Consumes: nothing new (still generic over `TerminalReplayRecord`).
- Produces: `loadTerminalReplay(read: () => Promise<{ lines: TerminalReplayRecord[]; cols?: number; rows?: number }>, isCurrent: () => boolean, createTerminal: (size: { cols?: number; rows?: number }) => ReplayTerminal): Promise<TerminalReplayResult>` — `createTerminal` now receives the recorded size so it can construct a fixed-size terminal. Consumed by Task 8 (`HistoricalTerminal.tsx`).

- [ ] **Step 1: Update the implementation**

Replace the full contents of `apps/desktop/src/terminalReplay.ts`:

```ts
export type TerminalReplayResult = "loaded" | "empty" | "error" | "stale";

export interface ReplayTerminal {
  write(text: string): void;
  writeln(line: string): void;
}

export type TerminalReplayRecord = string | { text: string; delimiter: string };

export function writeTerminalReplay(terminal: ReplayTerminal, records: TerminalReplayRecord[]): void {
  for (const record of records) {
    if (typeof record === "string") terminal.writeln(record);
    else terminal.write(record.text + record.delimiter);
  }
}

export async function loadTerminalReplay(
  read: () => Promise<{ lines: TerminalReplayRecord[]; cols?: number; rows?: number }>,
  isCurrent: () => boolean,
  createTerminal: (size: { cols?: number; rows?: number }) => ReplayTerminal,
): Promise<TerminalReplayResult> {
  try {
    const payload = await read();
    if (!isCurrent()) return "stale";
    if (payload.lines.length === 0) return "empty";
    const terminal = createTerminal({ cols: payload.cols, rows: payload.rows });
    writeTerminalReplay(terminal, payload.lines);
    return "loaded";
  } catch {
    return isCurrent() ? "error" : "stale";
  }
}
```

- [ ] **Step 2: Update the existing tests to match the new `read` contract**

Replace the full contents of `apps/desktop/tests/terminalReplay.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { loadTerminalReplay, writeTerminalReplay } from "../src/terminalReplay.ts";
import { renderTerminalPresentation } from "../src/terminalPresentation.ts";

test("dead session routing invokes replay instead of interactive terminal creation", () => {
  let interactive = 0;
  let historical = 0;
  const result = renderTerminalPresentation(
    "dead",
    () => { interactive += 1; return "interactive"; },
    () => { historical += 1; return "historical"; },
  );

  assert.equal(result, "historical");
  assert.equal(interactive, 0);
  assert.equal(historical, 1);
});

for (const lifecycle of ["creating", "alive", "stopping"] as const) {
  test(`${lifecycle} session routing retains interactive terminal creation`, () => {
    let interactive = 0;
    let historical = 0;
    const result = renderTerminalPresentation(
      lifecycle,
      () => { interactive += 1; return "interactive"; },
      () => { historical += 1; return "historical"; },
    );

    assert.equal(result, "interactive");
    assert.equal(interactive, 1);
    assert.equal(historical, 0);
  });
}

test("writes persisted replay when the request remains current", async () => {
  const written: string[] = [];
  let factories = 0;
  const result = await loadTerminalReplay(
    async () => ({ lines: ["first", "second"] }),
    () => true,
    () => {
      factories += 1;
      return {
        write: (text: string) => written.push(`write:${text}`),
        writeln: (line: string) => written.push(`writeln:${line}`),
      };
    },
  );

  assert.equal(result, "loaded");
  assert.equal(factories, 1);
  assert.deepEqual(written, ["writeln:first", "writeln:second"]);
});

test("writes raw replay records without adding a line ending", async () => {
  const written: string[] = [];
  const result = await loadTerminalReplay(
    async () => ({ lines: [{ text: "one", delimiter: "\r\n" }] }),
    () => true,
    () => ({
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    }),
  );

  assert.equal(result, "loaded");
  assert.deepEqual(written, ["write:one\r\n"]);
});

test("replay fallback writes raw records and preserves legacy line behavior", () => {
  const written: string[] = [];

  writeTerminalReplay(
    {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    },
    [{ text: "one", delimiter: "\r\n" }, "one"],
  );

  assert.deepEqual(written, ["write:one\r\n", "writeln:one"]);
});

test("passes the recorded size through to the terminal factory", async () => {
  const sizes: Array<{ cols?: number; rows?: number }> = [];
  const result = await loadTerminalReplay(
    async () => ({ lines: ["one"], cols: 120, rows: 40 }),
    () => true,
    (size) => {
      sizes.push(size);
      return {
        write: () => {},
        writeln: () => {},
      };
    },
  );

  assert.equal(result, "loaded");
  assert.deepEqual(sizes, [{ cols: 120, rows: 40 }]);
});

test("does not write a replay response after selection changes", async () => {
  let resolve!: (payload: { lines: string[] }) => void;
  const pending = new Promise<{ lines: string[] }>((done) => { resolve = done; });
  const written: string[] = [];
  let factories = 0;
  let current = true;
  const result = loadTerminalReplay(() => pending, () => current, () => {
    factories += 1;
    return {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    };
  });

  current = false;
  resolve({ lines: ["stale"] });

  assert.equal(await result, "stale");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});

test("reports empty and failed replay without writing", async () => {
  const written: string[] = [];
  let factories = 0;
  const create = () => {
    factories += 1;
    return {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    };
  };
  assert.equal(await loadTerminalReplay(async () => ({ lines: [] }), () => true, create), "empty");
  assert.equal(await loadTerminalReplay(async () => { throw new Error("offline"); }, () => true, create), "error");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});
```

- [ ] **Step 3: Run the tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/terminalReplay.test.ts`
Expected: all tests PASS, including the new `passes the recorded size through to the terminal factory` test.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/terminalReplay.ts apps/desktop/tests/terminalReplay.test.ts
git commit -m "feat: thread recorded terminal size through loadTerminalReplay"
```

---

### Task 6: `terminalStore.ts` — adjust the one consumer of the old shape

**Files:**
- Modify: `apps/desktop/src/terminalStore.ts:186-190`

**Interfaces:**
- Consumes: `getTerminalOutput` from Task 4.
- Produces: no change to this file's own exports; this task exists purely to keep the live-terminal WebSocket-close replay fallback compiling against the new return shape. Cols/rows are ignored here on purpose — this path replays into an already-correctly-sized live terminal (mid-session reconnect), not a dead-session replay, so there's nothing to fix here.

There is no dedicated test file for `terminalStore.ts` in this codebase (it's WebSocket/DOM-heavy and untested today) — this task is verified by the type-check.

- [ ] **Step 1: Update the call site**

In `apps/desktop/src/terminalStore.ts`, replace (lines 186-190):

```ts
      getTerminalOutput(baseUrl, id).then((records) => {
        writeTerminalReplay(term, records);
      }).catch(() => {
        /* silently ignore fetch failures */
      });
```

with:

```ts
      getTerminalOutput(baseUrl, id).then((payload) => {
        writeTerminalReplay(term, payload.lines);
      }).catch(() => {
        /* silently ignore fetch failures */
      });
```

- [ ] **Step 2: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors from `terminalStore.ts` anymore. Errors may remain in `HistoricalTerminal.tsx` until Task 8 — that's expected.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/terminalStore.ts
git commit -m "fix: match getTerminalOutput's new return shape in reconnect replay"
```

---

### Task 7: `terminalReplayScale.ts` — pure scale-factor function

**Files:**
- Create: `apps/desktop/src/terminalReplayScale.ts`
- Create: `apps/desktop/tests/terminalReplayScale.test.ts`

**Interfaces:**
- Produces: `computeReplayScale(natural: { width: number; height: number }, available: { width: number; height: number }): number` — consumed by Task 8 (`HistoricalTerminal.tsx`).

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/tests/terminalReplayScale.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { computeReplayScale } from "../src/terminalReplayScale.ts";

test("scales down when the panel is narrower than the recorded width", () => {
  const scale = computeReplayScale({ width: 1200, height: 400 }, { width: 600, height: 400 });
  assert.equal(scale, 0.5);
});

test("scales down when the panel is shorter than the recorded height", () => {
  const scale = computeReplayScale({ width: 1200, height: 400 }, { width: 1200, height: 200 });
  assert.equal(scale, 0.5);
});

test("uses the more constraining dimension when both are smaller", () => {
  const scale = computeReplayScale({ width: 1000, height: 500 }, { width: 400, height: 400 });
  assert.equal(scale, 0.4);
});

test("never scales up past 1 when the panel is larger than the recording", () => {
  const scale = computeReplayScale({ width: 800, height: 400 }, { width: 2000, height: 2000 });
  assert.equal(scale, 1);
});

test("returns 1 for a zero or negative natural size instead of dividing by zero", () => {
  assert.equal(computeReplayScale({ width: 0, height: 400 }, { width: 600, height: 400 }), 1);
  assert.equal(computeReplayScale({ width: 800, height: 0 }, { width: 600, height: 400 }), 1);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/terminalReplayScale.test.ts`
Expected: FAIL — `Cannot find module '../src/terminalReplayScale.ts'`.

- [ ] **Step 3: Implement**

Create `apps/desktop/src/terminalReplayScale.ts`:

```ts
export function computeReplayScale(
  natural: { width: number; height: number },
  available: { width: number; height: number },
): number {
  if (natural.width <= 0 || natural.height <= 0) return 1;
  return Math.min(1, available.width / natural.width, available.height / natural.height);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/terminalReplayScale.test.ts`
Expected: all 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/terminalReplayScale.ts apps/desktop/tests/terminalReplayScale.test.ts
git commit -m "feat: add pure replay scale-factor calculation"
```

---

### Task 8: `HistoricalTerminal.tsx` — render at recorded size, scaled to fit

**Files:**
- Modify: `apps/desktop/src/components/HistoricalTerminal.tsx` (whole file, 57 lines)

**Interfaces:**
- Consumes: `loadTerminalReplay` (Task 5), `getTerminalOutput` (Task 4), `computeReplayScale` (Task 7).
- Produces: no change to this component's own props (`{ sessionId: string }`) or exported default — this is the terminal leaf of the chain, nothing downstream depends on new interfaces here.

This task has no new automated test of its own (DOM/xterm rendering isn't unit-tested in this codebase — `tests/dockview.test.ts` only asserts on the file's *source text*, not its runtime behavior). Verification is: (a) the existing source-text assertions in `tests/dockview.test.ts` still pass unmodified, (b) `tsc --noEmit` is clean, (c) a manual smoke check in Task 9.

- [ ] **Step 1: Confirm the source-text constraints this file must keep satisfying**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts`
Expected: PASS (this is the baseline — re-run after Step 2 to confirm nothing broke). The relevant test, `"HistoricalTerminal loads output without opening an interactive terminal transport"`, requires the new file to still contain the literal substrings `getTerminalOutput(baseUrl, sessionId)` and `loadTerminalReplay`, and to **not** contain `WebSocket` or `ensureTerminal`.

- [ ] **Step 2: Replace the component**

Replace the full contents of `apps/desktop/src/components/HistoricalTerminal.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { getTerminalOutput } from "../api";
import { loadTerminalReplay } from "../terminalReplay";
import { computeReplayScale } from "../terminalReplayScale";
import { orkworksTerminalTheme } from "../terminalTheme";
import EmptyState from "./EmptyState";

export default function HistoricalTerminal({ sessionId }: { sessionId: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<"loading" | "empty" | "error" | "loaded">("loading");

  useEffect(() => {
    let current = true;
    let terminal: Terminal | null = null;
    let observer: ResizeObserver | null = null;

    void window.orkworks.getBackendUrl()
      .then((baseUrl) => loadTerminalReplay(
        () => getTerminalOutput(baseUrl, sessionId),
        () => current,
        ({ cols, rows }) => {
          const container = containerRef.current;
          const hasFixedSize = Boolean(cols && rows);
          terminal = new Terminal({
            theme: orkworksTerminalTheme,
            disableStdin: true,
            cursorBlink: false,
            scrollback: 2000,
            cols,
            rows,
          });
          if (!container) return terminal;
          terminal.open(container);

          if (hasFixedSize) {
            // Recorded size known: render at the exact recorded grid (no reflow), then
            // shrink the whole grid with a CSS transform to fit the panel. Everything
            // inside xterm's `.xterm` root is absolutely positioned, so `.xterm` has no
            // natural size of its own — `.xterm-screen` is what xterm sizes in pixels to
            // match cols/rows, so that's what we measure and then stamp onto `.xterm`.
            const xtermEl = container.querySelector<HTMLElement>(".xterm");
            const screenEl = container.querySelector<HTMLElement>(".xterm-screen");
            if (xtermEl && screenEl) {
              let natural: { width: number; height: number } | null = null;
              const applyScale = () => {
                if (!natural) return;
                const scale = computeReplayScale(natural, {
                  width: container.clientWidth,
                  height: container.clientHeight,
                });
                xtermEl.style.transform = `scale(${scale})`;
                xtermEl.style.transformOrigin = "top left";
              };
              const renderDisposable = terminal.onRender(() => {
                if (natural) return;
                const rect = screenEl.getBoundingClientRect();
                natural = { width: rect.width, height: rect.height };
                xtermEl.style.width = `${rect.width}px`;
                xtermEl.style.height = `${rect.height}px`;
                renderDisposable.dispose();
                applyScale();
              });
              observer = new ResizeObserver(applyScale);
              observer.observe(container);
            }
          } else {
            // Legacy sessions with no recorded size: unchanged fit-to-container behavior.
            const fitAddon = new FitAddon();
            terminal.loadAddon(fitAddon);
            try { fitAddon.fit(); } catch { /* container not measured yet */ }
            observer = new ResizeObserver(() => {
              try { fitAddon.fit(); } catch { /* container not measured yet */ }
            });
            observer.observe(container);
          }

          return terminal;
        },
      ))
      .then((result) => {
        if (!current || result === "stale") return;
        setState(result);
      })
      .catch(() => {
        if (current) setState("error");
      });

    return () => {
      current = false;
      observer?.disconnect();
      terminal?.dispose();
    };
  }, [sessionId]);

  if (state === "empty") return <EmptyState message="No saved terminal output for this session." />;
  if (state === "error") return <EmptyState message="Saved terminal output is unavailable." />;
  return <div className="terminal-shell"><div ref={containerRef} className="terminal-container" aria-label={state === "loading" ? "Loading saved terminal output" : "Saved terminal output"} /></div>;
}
```

- [ ] **Step 3: Re-run the source-text test**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts`
Expected: PASS (same as Step 1 — confirms the rewrite kept the required literal call shapes and didn't introduce `WebSocket`/`ensureTerminal`).

- [ ] **Step 4: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors anywhere in the project — this closes out the type errors seeded in Task 4.

- [ ] **Step 5: Run the full TypeScript test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/components/HistoricalTerminal.tsx
git commit -m "feat: render dead-session replay at recorded size, scaled to fit"
```

---

### Task 9: Docs, final verification, and manual smoke check

**Files:**
- Modify: `AGENTS.md` (Metadata protocol section)

**Interfaces:** none — this task only documents the new on-disk artifact and runs whole-project verification.

- [ ] **Step 1: Document the new sidecar file**

In `AGENTS.md`, find this line in the "Metadata protocol" section:

```
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
```

Insert immediately after it:

```
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size` — the PTY's `cols`x`rows` at the moment a session reaches a terminal status (`killed`/`ended`/`error`), written once; used to render dead-session terminal replay at its recorded size instead of the current panel width. Absent for sessions that ended before this file existed, and for sessions whose in-memory runtime handle was already gone at the terminal-status transition — both cases fall back to fit-to-container replay.
```

- [ ] **Step 2: Commit the doc update**

```bash
git add AGENTS.md
git commit -m "docs: document the terminal-size sidecar file"
```

- [ ] **Step 3: Full Rust verification**

Run:
```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
```
Expected: both succeed with no warnings-as-errors and no failing tests.

- [ ] **Step 4: Full desktop verification**

Run from `apps/desktop/`:
```bash
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```
Expected: both succeed.

- [ ] **Step 5: Manual smoke check**

Run `pnpm dev` from `apps/desktop/` (auto-launches the sidecar). Start a session, widen the app window so the terminal panel is wide, run a command that produces a long line (e.g. `printf 'a%.0s' {1..200}; echo`), then end the session (kill it from the UI). Narrow the app window so the detail panel is much narrower than it was when recorded, and open the ended session's terminal replay. Confirm the long line renders as one unbroken (shrunk) line instead of being character-split across multiple lines. Then repeat with a session that predates this change if one exists (or accept a freshly-created session before this build as the "legacy" case is no longer reproducible after this change ships) to confirm the fit-to-container fallback still renders without errors.

- [ ] **Step 6: Run the repo's doc-currency and worktree-currency checks**

Run:
```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```
Expected: `doc-check.sh` reports no further flagged files beyond the `AGENTS.md` update already made in Step 1 (which should no longer be flagged, having been made). `worktree-check.sh` reports the fleet-wide worktree state — act only on worktrees you own; note anything else for its owner.

---

## Self-Review Notes

- **Spec coverage:** every bullet in the design doc's Scope section maps to a task — persistence (Task 1-2), serving (Task 3), fixed-size + scaled rendering (Task 7-8), legacy fallback preserved unchanged (Task 8's `else` branch, verified by Task 8 Step 3's re-run of the source-text test), no `SessionMetadata` fields added (confirmed by Task 1 using a sidecar file), no new dependencies (none introduced anywhere in this plan).
- **Placeholder scan:** no TBD/TODO markers; every step shows complete, exact code.
- **Type consistency:** `read_terminal_size`/`write_terminal_size` use `(cols: u16, rows: u16)` parameter order and `Option<(u16, u16)>` return order consistently across Tasks 1-3; `{ lines, cols, rows }` shape is consistent across Tasks 3-8 (Rust JSON field names `cols`/`rows` match the TS `data.cols`/`data.rows` access in Task 4 with no renaming needed, both already lowercase single words).
