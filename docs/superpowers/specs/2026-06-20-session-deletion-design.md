# Session Deletion & Retention Policy

**Date:** 2026-06-20
**Status:** draft

## Overview

Add permanent session deletion (removing `.orkworks/` metadata files from disk) and a configurable auto-expiry retention policy that runs as a background task in the Rust sidecar.

## Motivation

The existing `DELETE /sessions/:id` endpoint kills a live session (stops PTY, marks status as `killed`) but preserves metadata on disk. Killed and ended sessions accumulate in `.orkworks/sessions/` and `.orkworks/events/` indefinitely. Users need a way to:
1. Manually delete individual remembered/killed sessions permanently from disk
2. Configure a retention policy (max session count + max age) that auto-cleans old sessions

## Architecture

### New components

| Component | Layer | Purpose |
|-----------|-------|---------|
| `DELETE /sessions/:id/forget` | Rust sidecar | Permanently delete session metadata + events + terminal files from disk |
| `POST /settings/retention` | Rust sidecar | Receive retention policy from Electron main process |
| Background cleanup task | Rust sidecar | Periodic tokio task (every 5 min) that enforces retention policy |
| `forgetSession(baseUrl, id)` | TS API client | Call the forget endpoint |
| Canonical settings save IPC | TS/Electron IPC | Persist hotkey and retention settings together via Electron main process |
| Trash icon | SessionListPanel | Delete button for remembered/killed sessions |
| Retention section | SettingsModal | Max sessions count + max age days inputs |
| `RetentionSettings` type | appSettingsTypes.ts | TypeScript interface for retention config |

### Data flow

```
User configures retention in SettingsModal
  -> saved to Electron settings.json (retention key)
  -> pushed to Rust sidecar via POST /settings/retention

User clicks trash icon on remembered session
  -> confirmation prompt explains permanent metadata/scrollback deletion
  -> DELETE /sessions/:id/forget
  -> files removed from .orkworks/
  -> workspace last-active memory cleared if it pointed at the deleted session
  -> session list refreshed

Background cleanup task (every 5 min)
  -> reads retention config from AppState
  -> lists all sessions from metadata store
  -> deletes non-live sessions exceeding count/age limits
```

### Non-goals

- Batch multi-select delete UI
- Per-workspace retention overrides
- Archive/export before deletion
- Undo/restore deleted sessions

## Rust Sidecar

### `DELETE /sessions/:id/forget`

Handler in `main.rs`:
- Looks up session metadata by ID in the active workspace metadata store
- Checks the in-memory sessions map only to determine whether the session still has a live running PTY/process
- Rejects if the session is actively live/running → `409 Conflict` with `{"error": "Cannot forget a live session. Kill it first."}`
- Session not found → `404 Not Found` with `{"error": "Session not found"}`
- Killed/ended/error sessions that still have handles in the in-memory map are allowed if their status is not live/running
- Calls `metadata_store.delete_session(id)` — removes `.orkworks/sessions/<id>.json`
- Calls `metadata_store.delete_events(id)` — removes `.orkworks/events/<id>.ndjson` and `.orkworks/events/<id>.terminal`
- Clears Peon in-memory debounce/inference state for the session ID
- Clears `.orkworks/workspace.json.lastActiveSessionId` if it points at the forgotten session
- Returns `200 OK`
- Idempotent: if files are already gone, still returns `200 OK`

Important: the in-memory session map is not the source of truth for remembered sessions. Remembered/resumable sessions are reconstructed from `.orkworks/sessions/*.json` in `GET /sessions`, so `forget` must support IDs that are absent from the in-memory map.

### `POST /settings/retention`

Request body:
```json
{
  "maxSessions": 50,
  "maxAgeDays": 30
}
```

- `maxSessions`: 0 = disabled (keep unlimited)
- `maxAgeDays`: 0 = disabled (never expire by age)

Stored in `AppState` behind a `tokio::sync::RwLock<RetentionConfig>`. Returns `200 OK`.

Electron owns durable settings persistence. The Rust sidecar treats retention config as runtime state and must receive it:
- when the initial sidecar port becomes ready
- after every sidecar restart or workspace switch
- after the user saves settings

### `MetadataStore` new methods

```rust
fn delete_session(&self, id: &str) -> io::Result<()>
```
Removes `sessions/<id>.json` via `std::fs::remove_file`. Returns `Ok(())` if file doesn't exist.

```rust
fn delete_events(&self, id: &str) -> io::Result<()>
```
Removes `events/<id>.ndjson` and `events/<id>.terminal` via `std::fs::remove_file`. Returns `Ok(())` if files don't exist.

```rust
fn clear_last_active_session_if_matches(&self, id: &str) -> io::Result<()>
```
Reads `workspace.json`; if `lastActiveSessionId == id`, rewrites workspace memory with no active session while preserving other workspace memory fields.

`read_all_sessions()` should return enough error information for cleanup to distinguish "no sessions directory" from unreadable/corrupt state. If it remains a `Vec<SessionMetadata>` API, the cleanup task cannot truthfully log or skip a failed cycle.

### Background cleanup task

Spawned in `main()` after the HTTP server starts. A `tokio::spawn` loop:

```rust
async fn cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;
        let config = state.retention_config.read().await;
        if config.max_sessions == 0 && config.max_age_days == 0 { continue; }

        let Some(metadata_store) = state.metadata_store_for_active_workspace() else { continue; };
        let Ok(sessions) = metadata_store.read_all_sessions() else { continue; };

        // Collect non-live sessions, sorted oldest first
        let mut candidates: Vec<_> = sessions.iter()
            .filter(|s| !state.is_live(&s.id))
            .collect();
        candidates.sort_by_key(|s| s.last_activity_timestamp());

        let now = Utc::now();

        // Age check: delete sessions older than max_age_days
        if config.max_age_days > 0 {
            let cutoff = now - chrono::Duration::days(config.max_age_days as i64);
            let expired: Vec<_> = candidates.iter()
                .filter(|s| s.last_activity_timestamp() < cutoff)
                .map(|s| s.id.clone())
                .collect();
            for id in &expired {
                let _ = metadata_store.delete_session(id);
                let _ = metadata_store.delete_events(id);
                let _ = metadata_store.clear_last_active_session_if_matches(id);
            }
            candidates.retain(|s| !expired.contains(&s.id));
        }

        // Count check: delete oldest until within limit
        if config.max_sessions > 0 && candidates.len() > config.max_sessions {
            let to_delete = candidates.len() - config.max_sessions;
            for s in candidates.iter().take(to_delete) {
                let _ = metadata_store.delete_session(&s.id);
                let _ = metadata_store.delete_events(&s.id);
                let _ = metadata_store.clear_last_active_session_if_matches(&s.id);
            }
        }
    }
}
```

- Errors deleting individual files are logged, not fatal — cleanup continues
- If `read_all_sessions()` fails, the cycle is skipped and retried next interval
- Live sessions are never touched — gated by checking live/running status in the in-memory sessions HashMap, not by mere map membership
- Cleanup runs against the currently active workspace metadata store. If no workspace is active, it does nothing.

## TypeScript / Frontend

### `api.ts`

```typescript
export async function forgetSession(baseUrl: string, id: string): Promise<void> {
  const res = await fetch(`${baseUrl}/sessions/${id}/forget`, { method: "DELETE" });
  if (!res.ok) throw new Error(`forget session failed: ${res.status}`);
}
```

### `appSettingsTypes.ts`

```typescript
export interface RetentionSettings {
  maxSessions: number;  // 0 = disabled
  maxAgeDays: number;   // 0 = disabled
}
```

Added to `AppSettings` alongside `version`, `hotkeys`, `defaultHotkeys`.

### `App.tsx`

New handler:
```typescript
const handleForgetSession = useCallback(async (id: string) => {
  try {
    const baseUrl = await window.orkworks.getBackendUrl();
    await forgetSession(baseUrl, id);
    disposeTerminal(id);
    if (activeSessionId === id) setActiveSessionId(null);
    await refreshSessions();
  } catch {
    pushToast("error", "Couldn't delete session.");
  }
}, [activeSessionId, refreshSessions]);
```

After settings are saved, push retention config to the sidecar via `POST /settings/retention`.

### `SessionListPanel.tsx`

- New prop: `onForgetSession: (id: string) => void`
- Trash icon (🗑 or ✕) rendered next to sessions where `memoryState !== "live"`
- Muted gray color to distinguish from the kill button (which remains on live sessions only)
- Calls `onForgetSession(s.id)` with `e.stopPropagation()` only after a confirmation prompt
- Confirmation text must make clear that the session record, events, and saved terminal scrollback are permanently deleted

### `SettingsModal.tsx`

Below the hotkey list, separated by a divider:
- "Session retention" heading
- Two number inputs:
  - "Max sessions to keep" (0 = unlimited, default 0)
  - "Auto-delete sessions older than (days)" (0 = never, default 0)
- Inputs clamped to 0..999 range
- Help text: "Live sessions are never auto-deleted. Changes take effect immediately."
- Saved to `settings.json` under a `retention` key via IPC

### `electron/settingsMemory.ts`

Extend `DEFAULT_SETTINGS`:
```typescript
retention: { maxSessions: 0, maxAgeDays: 0 }
```

### `electron/preload.ts`

No separate retention-only persistence path is required. Settings should continue to flow through one canonical settings save path so hotkey and retention edits cannot race or overwrite each other.

### `electron/main.ts`

On app startup (after settings read), after every sidecar restart/workspace switch, and after settings save, push retention config to the sidecar via HTTP `POST /settings/retention`.

If a sidecar push fails, keep the setting persisted locally and retry the next time a sidecar port becomes ready.

## Edge Cases & Error Handling

### Forget endpoint
- **Live session:** `409 Conflict`
- **Not found:** `404 Not Found`
- **File deletion fails** (permissions, disk): `500 Internal Server Error`, logged to stderr
- **Idempotent:** if files already gone, `200 OK`
- **Remembered session absent from in-memory map:** allowed if session metadata exists
- **Killed/ended handle still in in-memory map:** allowed if status is not live/running
- **Deleted active session:** clears active-session workspace memory and active renderer selection

### Background cleanup
- File deletion errors logged, not fatal to the cleanup cycle
- `read_all_sessions()` failure: skip cycle, retry next interval
- Live sessions never touched
- Deleted sessions are also cleared from workspace last-active memory when applicable
- Retention config defaults disabled (0/0) — no surprise deletions

### UI
- Manual deletion requires confirmation because there is no undo/restore
- Trash icon hidden for live sessions
- After forget: session removed from list, activeSessionId cleared if applicable
- Failed forget: session remains selected/listed and an error toast is shown
- Retention inputs clamp to 0..999
- Sidecar unreachable: retention config still persisted locally; pushed on next connection

### Settings persistence
- Retention defaults ship disabled — opt-in
- Settings survive app restart via `settings.json`
- Sidecar starts with empty config, receives push from Electron on startup
- Sidecar receives the persisted config again after workspace switches because those restart the sidecar process

## Testing

| Area | Type | What |
|------|------|------|
| Rust `MetadataStore` | Unit | `delete_session`, `delete_events` with temp dir fixtures |
| Rust `MetadataStore` | Unit | `clear_last_active_session_if_matches` preserves unrelated workspace memory |
| Rust handler | Integration | Create → kill → forget → verify files gone |
| Rust handler | Integration | Forget remembered session absent from in-memory map → `200` and files gone |
| Rust handler | Integration | Forget live session → `409` |
| Rust handler | Integration | Forget unknown ID → `404` |
| Rust handler | Integration | Forget active remembered session clears workspace `lastActiveSessionId` |
| Rust cleanup task | Unit | Mock clock, verify count/age enforcement logic |
| Rust cleanup task | Unit | Killed/ended sessions still present in map can expire, live/running sessions cannot |
| Electron main | Unit | Retention config is pushed after sidecar startup and workspace-switch sidecar restart |
| TS `forgetSession` | Unit | Mock fetch, verify correct URL and method |
| TS `forgetSession` | Unit | Non-OK response rejects so UI can show an error |
| TS SettingsModal | Unit | Retention inputs render, validate, clamp |
| TS SessionListPanel | Unit | Forget button is hidden for live sessions and requires confirmation for remembered sessions |
