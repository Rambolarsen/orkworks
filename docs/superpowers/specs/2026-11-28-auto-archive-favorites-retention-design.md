---
type: Design
title: Auto-archive, favorites, and staged session retention
description: Proposed staged session retention lifecycle replacing legacy max-sessions/max-age deletion with default-on auto-archive, active-session idle warnings, favorites, and archived deletion.
tags: [orkworks, sessions, archive, retention, lifecycle]
status: stable
workflow_status: approved
---

# Auto-archive, favorites, and staged session retention

Status: owner approved on 2026-11-28; runtime implementation has not started and
remains gated on the linked
[implementation plan](../plans/2026-11-28-auto-archive-favorites-retention.md),
tracking issue, ADR, and updates to the
[session archive design](2026-10-18-session-archive.md) and
[MVP specification](../../../specs/orkworks-mvp.md).

This design pairs with
[Opt-in worktree launch for new agent sessions](2026-11-28-new-session-worktree-design.md).
**Sequencing:** land this design's `OrganizationStore` v2, Favorites, and staged
retention first (or as Phase 1 of a joint rollout) so managed-worktree launch
builds directly on `OrganizationStore` v2 and archive-time cleanup without
wiring temporary rules into legacy retention.

## Intent and problem statement

Today OrkWorks' retention and archive features pull in opposite directions:

- **Retention** (`maxSessions` / `maxAgeDays` in Settings) defaults to `0`/`0`
  (disabled) and, when enabled, jumps straight from `Closed` to permanent
  deletion.
- **Archive** is manual-only and acts as "exempt from deletion forever." Because
  users rarely archive sessions by hand, finished sessions pile up in the
  default list—and cleanup tied to archiving (such as removing clean
  OrkWorks-created worktrees) rarely runs unless archiving happens
  automatically.

This design **replaces** the legacy `maxSessions` / `maxAgeDays` retention
system with one unified, **enabled-by-default**, Settings-controlled staged
lifecycle:

```text
Active (idle) ─[advance warning + grace]─► Close ─► Archived ─[archived age]─► Deleted
Closed        ─[inactivity age]───────────────────► Archived ─[archived age]─► Deleted
```

An explicit **Favorite** flag takes over the "keep forever" role so `Archived`
means what users expect: aged out of the active workspace list, still readable
for a grace period, and eventually cleaned up unless protected.

- **Goals**
  - Keep the default Sessions list tidy automatically with sensible, on-by-default
    staged retention controlled in OrkWorks Settings.
  - Include genuinely idle **Active** sessions in auto-archive, with a clear
    advance warning/notification and grace window before closing them.
  - Give users an explicit **Favorite** toggle that exempts a session from
    warnings, auto-archive, and auto-delete.
  - Protect existing users on upgrade so previously manually-archived sessions
    are never unexpectedly deleted.
  - Integrate cleanly with managed worktrees: auto-archiving triggers clean
    worktree removal, while flagged dirty worktrees are protected from
    auto-deletion.
- **Non-goals**
  - Count-based (`maxSessions`) deletion caps (replaced by age-based stages).
  - Deleting a coding tool's native saved conversations on disk.
  - Auto-archiving or interrupting sessions that are actively working.
  - Updating the dormant, gated SQLite `session_store.rs` layer in this slice
    (deferred to `#1244`; existing compiled SQLite tests stay green).
  - Syncing favorites or archive state across independent OrkWorks instances.
- **Anti-goals**
  - Running default-on retention in the sidecar before authoritative user
    settings have been synced from Electron (which could delete sessions when a
    user explicitly set retention to `0`).
  - Silently killing a live terminal without advance warning (including right
    after a laptop wakes from multi-day sleep).
  - Deleting sessions a user manually archived under the old "keep forever"
    contract during upgrade, or silently deleting an archived session 5 minutes
    after a user unfavorites it.
  - Deleting an archived session that still holds a flagged dirty managed
    worktree, or deleting corrupt/unreadable organization records.

## Lifecycle model and favorites

### States and the `favorited` flag

Keep the three existing user-facing session states—**Active**, **Closed**,
**Archived**—and add an orthogonal boolean **`favorited`** (default `false`) in
`OrganizationStore`.

| Protection / attribute | Exempt from idle-Active warning | Exempt from auto-archive | Exempt from auto-delete |
| --- | --- | --- | --- |
| `favorited: true` | Yes | Yes | Yes |
| `managedWorktree.cleanupStatus` in `{"retained_dirty", "removal_failed"}` | N/A (already archived) | N/A | Yes |
| Unreadable / corrupt organization record | Yes | Yes | Yes |
| Live runtime actively `working` (or `creating` / `stopping` / `finalizing`) | Yes | Yes | Yes |
| Active orchestration run / reservation | Yes | Yes | Yes |

**Favorite behavior:**

- Users can star/unstar any `Active`, `Closed`, or `Archived` session from the
  row context menu, a subtle star control in Details, or row indicator.
- Toggling `favorited` uses the existing `sessionActionToken` stale-action check
  and atomic `OrganizationStore` write.
- `favorited: true` persists across manual `Close`, `Archive`, `Restore`, and
  `Resume`.
- Favoriting a session that currently has an active auto-archive warning
  immediately clears its in-memory warning.
- **Unfavoriting an already-`Archived` session resets `archivedAt = now`**,
  granting a fresh full `deleteArchivedAfterDays` window so unstarring an old
  archived session (including any upgraded v1 archived session) is never
  silently destructive on the next 5-minute cleanup pass.
- Explicit **manual** Close, Archive, and Delete still work on Favorited
  sessions; manual Delete confirmation explicitly calls out that the session is
  favorited before proceeding.

### Restore behavior

Restoring an `Archived` session to `Closed`:

- Clears `archivedAt` and sets `restoredAt = now` (recorded in
  `OrganizationStore` so producer metadata snapshots do not have to be
  rewritten).
- Computes inactivity age for auto-archive from
  `max(SessionMetadata.last_activity, restoredAt)` so a restored older session
  is **not** immediately re-archived on the next background pass; it gets a
  fresh full `autoArchiveAfterDays` window from the moment the user restored it.
- Preserves `favorited`.

## Staged retention policy and settings

Replace `RetentionSettings { maxSessions, maxAgeDays }` in Settings →
**Session retention** with:

| Setting | Type | Default | Range / Meaning |
| --- | --- | --- | --- |
| `autoArchiveAfterDays` | integer | `7` | `0..=999` (`0` disables auto-archive for both Closed and Active sessions) |
| `autoArchiveActiveSessions` | boolean | `true` | When `true` and `autoArchiveAfterDays > 0`, idle Active sessions also auto-archive after an advance warning |
| `deleteArchivedAfterDays` | integer | `30` | `0..=999` (`0` disables automatic deletion of Archived sessions; measured from `archivedAt`) |

### Fail-closed startup settings synchronization

Because retention is now enabled by default, the sidecar must **never** run
autonomous default-on retention before receiving authoritative user settings
(otherwise a transient startup push failure could archive or delete sessions for
a user who explicitly configured `0`):

- Electron `DEFAULT_RETENTION` defaults to `{ version: 2, autoArchiveAfterDays:
  7, autoArchiveActiveSessions: true, deleteArchivedAfterDays: 30 }`.
- `orkworksd` starts with `retention_applied = false` and skips retention ticks
  until `POST /settings/retention` succeeds at least once for the running
  sidecar.
- Electron's workspace-readiness restoration pushes `POST /settings/retention`
  with bounded retry/backoff until acknowledged (replacing the current
  fire-and-forget secondary push), and keeps the existing saved/pending apply
  indicator in `SettingsModal.tsx`.
- Settings UI displays a concise note explaining that **Favorited** sessions and
  archived sessions with **dirty worktrees** are never automatically archived or
  deleted.

### Inactivity timestamps

OrkWorks currently separates `SessionMetadata.last_activity` (updated on status,
attention, metadata, and lifecycle transitions) from `SessionMetadata.last_output_at`
(updated on every non-empty PTY byte, including TUI status-bar/clock redraws and
cursor blinks). To avoid both false archival of active plain shells and defeated
idle detection on TUI harnesses:

- **Stage 1A (`Closed` → `Archived`):** Measure inactivity from
  `max(SessionMetadata.last_activity, OrganizationStore.restoredAt)`.
- **Stage 1B (`Active` idle detection and warning cancellation):** Maintain a
  lightweight, in-memory monotonic `last_interactive_at` timestamp on
  `SessionHandle` (initialized at launch/resume from `now`, updated on terminal
  user input writes and whenever `last_activity` advances). Measure Active
  inactivity from `max(SessionMetadata.last_activity, handle.last_interactive_at)`
  and require `observed_status != "working"`. Deliberately ignore raw
  `last_output_at` so periodic TUI redraws or cursor blinks cannot defeat idle
  detection, while any active user typing in a plain shell or coding tool
  immediately counts as activity without PTY hot-path disk writes.

### Stage 1A: Closed → Archived

A `Closed` session is eligible for auto-archive when all of the following hold:

1. `retention_applied == true` and `autoArchiveAfterDays > 0`.
2. `session_state == Closed`, `favorited == false`, no pending reservation, and
   runtime is confirmed closed (`lifecycle == "dead"`, `lifecycle_phase ==
   "ended"`, not in `session_pids`).
3. `now - max(last_activity, restoredAt) >= autoArchiveAfterDays`.

Execution acquires the per-session `OrganizationStore::reserve`, revalidates
identity and eligibility, runs managed-worktree cleanup if present (per the
[worktree launch design](2026-11-28-new-session-worktree-design.md)), and
atomically writes `state = "archived"` with `archivedAt = now`.

### Stage 1B: Idle Active → Advance Warning → Close → Archived

An `Active` session is **never** silently closed on first sight of old
inactivity. Warning state lives **in-memory on `SessionHandle`** (never written
to `session-state/<id>.json`) because live Active sessions do not survive
sidecar restart anyway and keeping warning state in memory avoids PTY hot-path
`fsync`s and `sessionActionToken` revision churn.

1. **Idle qualification:** A live session qualifies as idle only when all of the
   following hold:
   - `retention_applied == true`, `autoArchiveAfterDays > 0`, and
     `autoArchiveActiveSessions == true`.
   - `favorited == false` and no pending state reservation or active
     orchestration ownership.
   - Live lifecycle is steady-state running (not `creating`, `stopping`, or
     `finalizing`) and `observed_status != "working"`.
   - `inactivity = now - max(last_activity, handle.last_interactive_at)`.
2. **Warning lead time:**
   - Lead time is `min(24 hours, floor(autoArchiveAfterDays * 24 hours / 4))`
     (24 hours for `autoArchiveAfterDays >= 4`, 12 hours for 2 days, 6 hours for
     1 day).
   - Once `inactivity >= autoArchiveAfterDays - leadTime` and no warning is
     currently active on `SessionHandle`, record in memory:
     - `auto_archive_warned_at = now`
     - `auto_archive_scheduled_at = max(last_interactive_at_or_activity + autoArchiveAfterDays, now + leadTime)`
   - **Sleep / wake safety invariant:** Computing `auto_archive_scheduled_at`
     with `max(..., now + leadTime)` guarantees that if the user's machine slept
     for two weeks while OrkWorks stayed running, an idle Active session still
     gets a full `leadTime` grace window after wake and visible notification
     before OrkWorks closes it.
3. **Warning presentation:**
   - Project `autoArchiveWarning: { warnedAt, scheduledAt }` onto `SessionInfo`
     from `SessionHandle`.
   - Sessions list shows a subtle warning badge/tooltip on the row; Details and
     the Terminal header show a non-blocking banner ("Idle — closes and archives
     in `24h`", using the remaining time) with one-click **Favorite (keep)** and
     **Archive now** actions.
   - If desktop notifications are enabled, emit **at most one** OS notification
     per `(sessionId, warnedAt)` cycle (deduplicated so the 5-minute retention
     loop never repeats it).
4. **Reset / cancellation:**
   - Any user terminal input write, transition to `working`, advance of
     `last_activity`, toggling `favorited = true`, retention settings change
     that disables or extends the window, or session close immediately clears
     `SessionHandle`'s in-memory warning fields (zero disk I/O).
5. **Grace-period expiration and Close-first execution:**
   - On a background pass where `now >= auto_archive_scheduled_at`, re-verify
     the session is still idle, un-favorited, and matches its `createdAt` +
     `auto_archive_warned_at`.
   - Execute the existing bounded **Close-first** protocol from
     [2026-10-18-session-archive.md](2026-10-18-session-archive.md#L22-L25)
     (releasing `workspace`/`sessions` mutexes across awaits, 15-second cleanup
     bound).
   - If Close fails or times out, do **not** archive and record a diagnostic
     without retrying in a tight loop.
   - If Close succeeds, run managed-worktree cleanup (if applicable) and commit
     `state = "archived"` with `archivedAt = now`.

### Stage 2: Archived → Deleted

An `Archived` session is eligible for permanent deletion when all of the
following hold:

1. `retention_applied == true` and `deleteArchivedAfterDays > 0`.
2. `session_state == Archived`, `favorited == false`, no pending reservation,
   and runtime is confirmed closed.
3. `managedWorktree.cleanupStatus` is absent or `"removed"` (never delete while
   `"active"`, `"retained_dirty"`, or `"removal_failed"`).
4. `now - archivedAt >= deleteArchivedAfterDays`.

Execution uses the existing `delete_session_evidence` path (with workspace lease
and Windows cleanup-namespace guards), clears Peon/runtime tracking, and removes
the organization record.

## Persistence, migration, and workspace GC

### `OrganizationStore` v2 record

Extend `session-state/<id>.json` (preserving the 4 KiB bound, atomic
temp-file + fsync + directory-sync publication, and fail-closed handling of
corrupt files):

```json
{
  "version": 2,
  "sessionId": "<id>",
  "createdAt": "<rfc3339>",
  "state": "archived",
  "favorited": false,
  "archivedAt": "<rfc3339 or null>",
  "restoredAt": "<rfc3339 or null>",
  "managedWorktree": null,
  "revision": 3,
  "changedAt": "<rfc3339>"
}
```

- Valid `version: 1` records remain readable without error.
- **Upgrade migration of `version: 1` archived records:** A `version: 1` record
  with `state == "archived"` was explicitly archived by the user under the old
  contract where Archived meant "exempt from retention deletion." On read (and
  persisted as `version: 2` on next write), treat any `v1` `state == "archived"`
  record as `favorited: true` with `archivedAt = changedAt`. That guarantees
  upgrading to default-on auto-delete never deletes a previously
  manually-archived session.
- **Upgrade migration of `settings.json`:**
  - Introduce `retention.version = 2` alongside `autoArchiveAfterDays`,
    `autoArchiveActiveSessions`, and `deleteArchivedAfterDays`.
  - When loading a legacy `settings.json` lacking `retention.version`:
    - Replace legacy `maxSessions` / `maxAgeDays` with the new v2 defaults
      (`autoArchiveAfterDays: 7`, `autoArchiveActiveSessions: true`,
      `deleteArchivedAfterDays: 30`), except if the user had explicitly set a
      custom `maxAgeDays > 0`, preserve that number as
      `deleteArchivedAfterDays` so their chosen deletion horizon is respected.
    - Persist the migrated v2 shape on the next settings write.

### Startup workspace GC alignment

Update `dir_has_retention_protected_records`
([session_archive.rs](../../../crates/orkworksd/src/session_archive.rs#L61)) and
[workspace_gc.rs](../../../crates/orkworksd/src/runtime/workspace_gc.rs#L112):

- Protect any workspace metadata directory under `~/.orkworks/workspaces/<hash>/`
  that contains:
  - any `favorited: true` record,
  - any `managedWorktree.cleanupStatus` in `{"active", "retained_dirty", "removal_failed"}`,
  - any `version: 1` `state == "archived"` record (preserving pre-upgrade
    protection),
  - any corrupt, oversized, invalid-name, or unknown-version record.
- Ordinary non-favorited `v2` `closed` or clean `archived` records for a
  verifiably deleted workspace source no longer block startup workspace GC.

## UI changes summary

- **Sessions list (`SessionListPanel.tsx` / `SessionContextMenu.tsx`):**
  - Add **Favorite / Unfavorite** action in the context menu and Details, plus a
    subtle star indicator on favorited rows.
  - Add **Favorites** as a State filter choice alongside `All unarchived`,
    `Active`, `Closed`, and `Archived`. Selecting **Favorites** shows **all**
    favorited sessions across `Active`, `Closed`, and `Archived` (so upgraded
    v1 archived sessions appear there immediately without switching filters).
    This keeps the existing single-enum State filter without adding tabs or
    violating the single-central-context shell contract.
  - Show a subtle pending-archive indicator on idle Active rows inside their
    grace window, and a banner in Details/Terminal header with **Favorite
    (keep)** and **Archive now**.
  - Update the Restore helper copy in `SessionContextMenu.tsx` to reflect the
    new staged lifecycle (restoring moves back to Closed with a fresh
    auto-archive window).
- **Settings → Session retention (`SettingsModal.tsx`):**
  - Replace **Max sessions to keep** and **Auto-delete sessions older than
    (days)** with:
    1. **Auto-archive inactive sessions after (days)** (default `7`, `0 = off`)
    2. **Include idle active sessions** toggle (default on, noting the advance
       warning/grace window)
    3. **Delete archived sessions after (days)** (default `30`, `0 = off`)
  - Show a brief inline summary noting that Favorited sessions and sessions with
    dirty managed worktrees are protected.

## Decisions made by the designer (for owner review)

- **Defaults:** `autoArchiveAfterDays = 7`, `autoArchiveActiveSessions = true`,
  `deleteArchivedAfterDays = 30`, with an automatic advance warning lead time of
  `min(24h, autoArchiveAfterDays * 24h / 4)`.
- **Fail-closed sidecar startup gating:** `orkworksd` waits for Electron's
  retried `POST /settings/retention` push before running default-on retention so
  users who disabled retention (`0`) are never affected by a missed startup sync.
- **In-memory Active warning state & interactive-activity tracking:** Keep
  `autoArchiveWarnedAt`/`autoArchiveScheduledAt` and `last_interactive_at` in
  memory on `SessionHandle` (ignoring raw `last_output_at` cursor/clock redraws)
  to avoid PTY hot-path disk writes and `sessionActionToken` churn.
- **Pre-existing v1 archived sessions auto-promoted to `favorited: true`** so
  enabling staged auto-delete by default never deletes sessions users previously
  archived for safekeeping.
- **Sleep-safe grace period:** `autoArchiveScheduledAt` is always at least
  `now + leadTime` when the warning is first issued, preventing surprise closes
  immediately after multi-day machine sleep.
- **Archived deletion measured from `archivedAt`**, not `lastActivity`; both
  **Restore** (`restoredAt`) and **Unfavoriting an Archived session**
  (`archivedAt = now`) grant a fresh full window.
- **Legacy `maxSessions` count cap removed** and dormant SQLite `session_store`
  schema updates deferred to `#1244`.

## Left to the builder

- Exact `POST /sessions/:id/favorite` (or extended organization-state command)
  route payload and response shape, reusing `sessionActionToken` checks.
- Exact background scan interval (reusing the existing 5-minute retention loop
  is sufficient; warning countdown rendering in the UI uses `scheduledAt`).
- Exact Electron startup retry cadence/backoff for `POST /settings/retention`.
- Exact OS notification wording and icon placement in `SessionListPanel.tsx` /
  `SessionDetailPanel.tsx` / `SettingsModal.tsx`.

## Compatibility, recovery, and verification

- **Downgrade safety:** An older v1 binary reading a v2 `session-state/<id>.json`
  rejects `version != 1` as unreadable, which fails closed and protects the
  record from automatic deletion rather than misinterpreting it. Document this
  limit in release notes alongside existing retention downgrade guidance.
- **Concurrency and recovery:** Background auto-archive and auto-delete must
  never hold `workspace` or `sessions` mutexes across Close-first awaits, must
  recheck workspace identity and `createdAt` before every mutation, and must
  preserve per-session serialization with manual actions and Resume.
- **Approval and implementation prerequisites:**
  1. Owner approval of this written design.
  2. Update [2026-10-18-session-archive.md](2026-10-18-session-archive.md)
     (superseding its Retention section),
     [specs/orkworks-mvp.md](../../../specs/orkworks-mvp.md), and an ADR
     recording the staged retention & favorite state contract.
  3. GitHub issue tracking implementation.
- **Required verification before PR handoff:**
  - Rust unit and integration tests: fail-closed startup gating before
    `POST /settings/retention`, v1-to-v2 archive-to-favorite migration,
    `settings.json` legacy migration, Closed → Archived after threshold,
    Restore clock reset, Unfavorite-on-Archived `archivedAt` clock reset, idle
    Active warning issuance (ignoring raw `last_output_at` redraws while
    honoring terminal input) + sleep-safe grace window + reset on
    input/working/favorite/settings-change + Close-first archival + timeout
    refusal, Archived → Deleted after `archivedAt` threshold, Favorite exemption
    across all stages, dirty-worktree auto-delete protection, `workspace_gc`
    protection rules, and corrupt-record protection.
  - Desktop tests: new Retention settings UI, default values, startup retry on
    retention sync, Favorite toggle and cross-state Favorites filter, idle
    Active warning banner + notification deduplication, and updated
    Restore/Delete messaging.
  - Full `bash scripts/verify-repo.sh`, `bash scripts/doc-check.sh`, and
    current-diff `/code-review low`.
