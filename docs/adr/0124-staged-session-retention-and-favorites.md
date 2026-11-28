---
type: Architecture Decision
title: Staged session retention, favorites, and OrganizationStore v2
description: Replace legacy max-sessions/max-age deletion with fail-closed default-on staged retention, explicit favorites, sleep-safe idle-active warnings, and OrganizationStore v2.
tags: [orkworks, architecture, sessions, archive, retention]
---

# Staged session retention, favorites, and OrganizationStore v2

- Status: accepted
- Deciders: user
- Date: 2026-11-28

## Context

Under [ADR 0088](0088-session-archive-state.md), retention defaulted to disabled
(`maxSessions = 0`, `maxAgeDays = 0`), skipped `Archived` sessions indefinitely,
and jumped directly from `Closed` to permanent deletion when enabled. Because
archiving was manual-only, finished sessions accumulated in the default list and
archive-time cleanup could not run reliably.

[Issue #2279](https://github.com/Rambolarsen/orkworks/issues/2279) tracks the
owner-approved
[staged session retention, favorites, and auto-archive design](../superpowers/specs/2026-11-28-auto-archive-favorites-retention-design.md)
(PR #2266), which replaces legacy count/age deletion with an enabled-by-default
staged lifecycle (`Active (idle)` / `Closed` -> `Archived` -> `Deleted`) and
assigns explicit retention protection to a dedicated `favorited` flag.

## Decision

1. **Extend `OrganizationStore` (`session-state/<id>.json`) to version 2.**
   Keep the 4 KiB bound, atomic write-and-sync publication, identity binding to
   `(sessionId, createdAt)`, and fail-closed handling of unreadable/unknown
   records. Add `favorited: bool` (default `false`), `archivedAt`, `restoredAt`,
   and the reserved optional `managedWorktree` field (default `null`, consumed
   by the companion opt-in worktree launch design).
   - Valid `version: 1` records remain readable. Any `version: 1` record with
     `state == "archived"` is automatically promoted on read (and persisted as
     `version: 2` on next write) to `favorited: true` with
     `archivedAt = changedAt`, ensuring upgrading users never lose previously
     archived sessions to default-on archived deletion.
   - Restoring an `Archived` session clears `archivedAt`, stamps
     `restoredAt = now`, and preserves `favorited`.
   - Unfavoriting an already-`Archived` session resets `archivedAt = now` so it
     receives a fresh full `deleteArchivedAfterDays` grace window.
   - Favorite/Unfavorite mutations serialize under `OrganizationStore::reserve`
     and validate the row's `sessionActionToken`.

2. **Replace legacy `maxSessions` / `maxAgeDays` with fail-closed staged
   retention settings (v2).**
   - Defaults: `autoArchiveAfterDays = 7`, `autoArchiveActiveSessions = true`,
     `deleteArchivedAfterDays = 30` (each day setting bounded to `0..=999`,
     where `0` disables that stage).
   - Legacy `settings.json` payloads lacking `retention.version` migrate to v2
     defaults, preserving an explicit `maxAgeDays > 0` as
     `deleteArchivedAfterDays`.
   - `orkworksd` starts with `retention_applied = false` and skips automatic
     retention until Electron's retried `POST /settings/retention` push succeeds
     at least once, preventing default-on cleanup from running before user
     settings are synced.

3. **Keep idle-Active warning state and interactive activity in memory on
   `SessionHandle`.**
   - Track monotonic `last_interactive_at` on `SessionHandle` from terminal user
     input and `last_activity` advances, deliberately ignoring raw
     `last_output_at` redraws so TUI clocks/cursors cannot defeat idle detection
     and PTY hot paths incur zero disk writes.
   - Issue an advance warning with lead time
     `min(24h, floor(autoArchiveAfterDays * 24h / 4))` and schedule
     `auto_archive_scheduled_at = max(inactivity_origin + autoArchiveAfterDays, now + leadTime)`.
   - Detect host sleep/suspension jumps (`now - last_retention_tick_at > 15m`)
     and renew every active warning deadline to at least `now + leadTime` before
     evaluating expiration, ensuring a full post-wake grace window even when
     sleep began after warning issuance.
   - Clear in-memory warnings immediately on user input, `working` transition,
     `last_activity` advance, favoriting, close, or retention-settings change.
   - After grace expiration, execute the existing bounded Close-first protocol
     without holding `workspace` or `sessions` mutexes across awaits; abort
     archival on Close failure or timeout.

4. **Align Stage 2 deletion and startup `workspace_gc` protection.**
   - Stage 2 deletes `Archived` sessions when `now - archivedAt >= deleteArchivedAfterDays`,
     skipping `favorited: true`, non-removed `managedWorktree`
     (`active`, `retained_dirty`, `removal_failed`), pending reservations, and
     corrupt/unknown records.
   - Startup `workspace_gc` protects workspace metadata directories containing
     any `favorited: true` record, any non-removed `managedWorktree`, any
     unmigrated `version: 1` `state == "archived"` record, or any
     corrupt/oversized/unknown-version record; clean non-favorited v2 `closed`
     or `archived` directories for verifiably deleted workspaces may be
     collected.

## Consequences

- Ordinary finished and long-idle sessions age out of the default list
  automatically while remaining readable in **Archived** for 30 days by default.
- Favorited sessions and previously archived v1 sessions are protected across
  all automatic stages and workspace GC, and can be browsed together through the
  cross-state **Favorites** State filter.
- Older v1 binaries reject v2 `session-state/<id>.json` records as unreadable
  and fail closed, protecting those records from automatic deletion on
  downgrade.
- Dormant SQLite `session_store.rs` schema updates are deferred to `#1244`,
  while existing compiled SQLite tests remain passing.
- The companion
  [opt-in worktree launch design](../superpowers/specs/2026-11-28-new-session-worktree-design.md)
  builds directly on `OrganizationStore` v2 and the Stage 1 archive hook without
  a second schema migration.
