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
| `forgetSession(id)` | TS API client | Call the forget endpoint |
| `saveRetention(cfg)` | TS IPC | Persist retention settings via Electron main process |
| Trash icon | SessionListPanel | Delete button for remembered/killed sessions |
| Retention section | SettingsModal | Max sessions count + max age days inputs |
| `RetentionSettings` type | appSettingsTypes.ts | TypeScript interface for retention config |

### Data flow

```
User configures retention in SettingsModal
  -> saved to Electron settings.json (retention key)
  -> pushed to Rust sidecar via POST /settings/retention

User clicks trash icon on remembered session
  -> DELETE /sessions/:id/forget
  -> files removed from .orkworks/
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
- Looks up session by ID in the in-memory sessions map
- Rejects if `memoryState` is `Live` → `409 Conflict` with `{"error": "Cannot forget a live session. Kill it first."}`
- Session not found → `404 Not Found` with `{"error": "Session not found"}`
- Calls `metadata_store.delete_session(id)` — removes `.orkworks/sessions/<id>.json`
- Calls `metadata_store.delete_events(id)` — removes `.orkworks/events/<id>.ndjson` and `.orkworks/events/<id>.terminal`
- Returns `200 OK`
- Idempotent: if files are already gone, still returns `200 OK`

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

### `MetadataStore` new methods

```rust
fn delete_session(&self, id: &str) -> io::Result<()>
```
Removes `sessions/<id>.json` via `std::fs::remove_file`. Returns `Ok(())` if file doesn't exist.

```rust
fn delete_events(&self, id: &str) -> io::Result<()>
```
Removes `events/<id>.ndjson` and `events/<id>.terminal` via `std::fs::remove_file`. Returns `Ok(())` if files don't exist.

### Background cleanup task

Spawned in `main()` after the HTTP server starts. A `tokio::spawn` loop:

```rust
async fn cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;
        let config = state.retention_config.read().await;
        if config.max_sessions == 0 && config.max_age_days == 0 { continue; }

        let Ok(sessions) = state.metadata_store.read_all_sessions() else { continue; };

        // Collect non-live sessions, sorted oldest first
        let mut candidates: Vec<_> = sessions.iter()
            .filter(|s| !state.is_live(&s.id))
            .collect();
        candidates.sort_by_key(|s| s.last_activity());

        let now = Utc::now();

        // Age check: delete sessions older than max_age_days
        if config.max_age_days > 0 {
            let cutoff = now - chrono::Duration::days(config.max_age_days as i64);
            let expired: Vec<_> = candidates.iter()
                .filter(|s| s.last_activity_date() < cutoff)
                .map(|s| s.id.clone())
                .collect();
            for id in &expired {
                let _ = state.metadata_store.delete_session(id);
                let _ = state.metadata_store.delete_events(id);
            }
            candidates.retain(|s| !expired.contains(&s.id));
        }

        // Count check: delete oldest until within limit
        if config.max_sessions > 0 && candidates.len() > config.max_sessions {
            let to_delete = candidates.len() - config.max_sessions;
            for s in candidates.iter().take(to_delete) {
                let _ = state.metadata_store.delete_session(&s.id);
                let _ = state.metadata_store.delete_events(&s.id);
            }
        }
    }
}
```

- Errors deleting individual files are logged, not fatal — cleanup continues
- If `read_all_sessions()` fails, the cycle is skipped and retried next interval
- Live sessions are never touched — gated by checking the in-memory sessions HashMap

## TypeScript / Frontend

### `api.ts`

```typescript
export async function forgetSession(id: string): Promise<boolean> {
  const res = await fetch(`${port}/sessions/${id}/forget`, { method: "DELETE" });
  return res.ok;
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
  await forgetSession(id);
  if (activeSessionId === id) setActiveSessionId(null);
  await refreshSessions();
}, [activeSessionId, refreshSessions]);
```

After settings are saved, push retention config to the sidecar via `POST /settings/retention`.

### `SessionListPanel.tsx`

- New prop: `onForgetSession: (id: string) => void`
- Trash icon (🗑 or ✕) rendered next to sessions where `memoryState !== "live"`
- Muted gray color to distinguish from the kill button (which remains on live sessions only)
- Calls `onForgetSession(s.id)` with `e.stopPropagation()`

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

Expose `saveRetention(settings: RetentionSettings)` via IPC invoke.

### `electron/main.ts`

On app startup (after settings read) and on `save-retention` IPC, push retention config to the sidecar via HTTP `POST /settings/retention`.

## Edge Cases & Error Handling

### Forget endpoint
- **Live session:** `409 Conflict`
- **Not found:** `404 Not Found`
- **File deletion fails** (permissions, disk): `500 Internal Server Error`, logged to stderr
- **Idempotent:** if files already gone, `200 OK`

### Background cleanup
- File deletion errors logged, not fatal to the cleanup cycle
- `read_all_sessions()` failure: skip cycle, retry next interval
- Live sessions never touched
- Retention config defaults disabled (0/0) — no surprise deletions

### UI
- Trash icon hidden for live sessions
- After forget: session removed from list, activeSessionId cleared if applicable
- Retention inputs clamp to 0..999
- Sidecar unreachable: retention config still persisted locally; pushed on next connection

### Settings persistence
- Retention defaults ship disabled — opt-in
- Settings survive app restart via `settings.json`
- Sidecar starts with empty config, receives push from Electron on startup

## Testing

| Area | Type | What |
|------|------|------|
| Rust `MetadataStore` | Unit | `delete_session`, `delete_events` with temp dir fixtures |
| Rust handler | Integration | Create → kill → forget → verify files gone |
| Rust handler | Integration | Forget live session → `409` |
| Rust handler | Integration | Forget unknown ID → `404` |
| Rust cleanup task | Unit | Mock clock, verify count/age enforcement logic |
| TS `forgetSession` | Unit | Mock fetch, verify correct URL and method |
| TS SettingsModal | Unit | Retention inputs render, validate, clamp |