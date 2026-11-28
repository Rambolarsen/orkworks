---
type: Implementation Plan
title: Auto-archive, favorites, and staged session retention implementation
description: Deliver OrganizationStore v2, Favorites, fail-closed staged retention settings, idle-active warnings, and updated workspace GC.
tags: [orkworks, sessions, archive, retention, implementation]
status: draft
workflow_status: proposed
---

# Auto-archive, favorites, and staged session retention implementation

Design: [Auto-archive, favorites, and staged session retention](../specs/2026-11-28-auto-archive-favorites-retention-design.md).
Revises retention section of: [Active, Closed and Archived sessions](../specs/2026-10-18-session-archive.md).
Companion design (lands second on this `OrganizationStore` v2 foundation):
[Opt-in worktree launch for new agent sessions](../specs/2026-11-28-new-session-worktree-design.md).
Tracking issue: [#2279](https://github.com/Rambolarsen/orkworks/issues/2279).

Pass condition: OrkWorks replaces legacy `maxSessions` / `maxAgeDays` deletion
with a Settings-controlled, enabled-by-default staged lifecycle
(`Active (idle)` / `Closed` -> `Archived` -> `Deleted`), protects **Favorite**
sessions and pre-existing v1 archived sessions from automatic cleanup, warns
with a sleep-safe grace window before closing idle Active sessions, and gates
sidecar retention until Electron pushes authoritative settings.

## Boundaries and investigated evidence

Use the existing Rust sidecar, `OrganizationStore`, `SessionApplication`,
`runtime/retention.rs`, `workspace_gc.rs`, Electron settings persistence, and
renderer Sessions/Settings surfaces. Do not add external dependencies, native
coding-tool archive mutations, count-based (`maxSessions`) caps, or changes to
the dormant SQLite `session_store.rs` schema (deferred to `#1244`, while keeping
existing compiled SQLite tests passing).

Investigated codebase constraints that shape the deliverables:

- `SessionMetadata.last_activity` is not updated by raw PTY output or arbitrary
  typing, whereas `SessionMetadata.last_output_at` fires on every non-empty PTY
  byte (including TUI clock/status-bar redraws). Stage 1B therefore uses an
  in-memory monotonic `last_interactive_at` on `SessionHandle` updated on user
  terminal input and `last_activity` advances, ignoring raw `last_output_at`.
- Live Active sessions never survive sidecar restart (reconciled to ended on
  restart), and `OrganizationStore` writes `fsync` + bump `revision` (hashed
  into `sessionActionToken`). Active idle warning timestamps therefore live in
  memory on `SessionHandle`, not in `session-state/<id>.json`.
- `orkworksd` currently starts `retention_cleanup_task` immediately using
  `RetentionConfig::default()` while Electron pushes `POST /settings/retention`
  fire-and-forget. With default-on retention, `orkworksd` must fail closed
  (`retention_applied = false`) until Electron's retried startup push succeeds.
- `OrganizationStore` v2 reserves the optional `managedWorktree` field (default
  `null`) so the companion worktree-launch plan adds no second schema migration.

## Delivery 1: governance prerequisites, `OrganizationStore` v2, Favorites, and `workspace_gc`

Files: new ADR under `docs/adr/` (+ `docs/adr/index.md`); `specs/orkworks-mvp.md`;
`docs/superpowers/specs/2026-10-18-session-archive.md`;
`crates/orkworksd/src/session_archive.rs`; `crates/orkworksd/src/session_types.rs`;
`crates/orkworksd/src/session_projection.rs`; `crates/orkworksd/src/session_application.rs`;
`crates/orkworksd/src/http/session_handlers.rs`; `crates/orkworksd/src/runtime/workspace_gc.rs`.

- [ ] Open the GitHub tracking issue, record the next available ADR for staged
  retention and favorites, update `specs/orkworks-mvp.md` and the Retention
  section of `2026-10-18-session-archive.md`, and index the ADR before runtime
  code.
- [ ] Write failing `OrganizationStore` v2 unit tests covering: v1 record
  compatibility, automatic read-promotion of v1 `state == "archived"` records
  to `favorited: true` + `archivedAt: changedAt`, atomic v2 writes within the
  4 KiB bound, `favorited` persistence across Close/Archive/Restore/Resume,
  `restoredAt` stamping on Restore, `archivedAt = now` reset when unfavoriting
  an `Archived` session, reserved `managedWorktree: null` round-tripping, and
  fail-closed handling of corrupt/unknown-version records.
- [ ] Implement `OrganizationStore` v2 and project `favorited` and `archivedAt`
  in `OrganizationProjection` / `SessionInfo` while keeping `sessionActionToken`
  bound to `(instance_id, id, created_at, revision, state, runtime_key)`.
- [ ] Add failing application/API tests and implement Favorite / Unfavorite
  commands under per-session `OrganizationStore::reserve` and
  `sessionActionToken` validation (clearing any active in-memory idle warning on
  favorite, resetting `archivedAt = now` when unfavoriting an `Archived`
  session, and preserving existing Close/Archive/Restore/Delete fencing).
- [ ] Update `dir_has_retention_protected_records` and `workspace_gc.rs` with
  regression tests confirming protection of `favorited: true`, non-removed
  `managedWorktree`, legacy v1 `archived`, and corrupt/unknown-version records,
  while allowing clean non-favorited v2 `closed` / `archived` directories for
  verifiably deleted workspaces to be collected.
- [ ] Run targeted Rust tests (`cargo test --manifest-path crates/orkworksd/Cargo.toml session_archive workspace_gc`) and confirm all pass.

Interface: `OrganizationStore` writes `version: 2` records containing
`favorited`, `archivedAt`, `restoredAt`, and optional `managedWorktree`; session
actions accept Favorite / Unfavorite with the row's `sessionActionToken` and
return updated projected organization state or a stale-token conflict.

## Delivery 2: fail-closed retention sync, interactive activity tracking, and staged background lifecycle

Files: `crates/orkworksd/src/main.rs`; `crates/orkworksd/src/http/retention_handlers.rs`;
`crates/orkworksd/src/runtime/retention.rs`; `crates/orkworksd/src/runtime/session_runtime.rs`;
`crates/orkworksd/src/runtime/terminal_runtime.rs`; `crates/orkworksd/src/session_types.rs`;
`crates/orkworksd/src/session_projection.rs`; `apps/desktop/src/appSettingsTypes.ts`;
`apps/desktop/electron/settingsMemory.ts`; `apps/desktop/electron/workspaceRuntime.ts`;
`apps/desktop/electron/backendRestoration.ts`; `apps/desktop/electron/main.ts`.

- [ ] Replace `RetentionConfig` and `POST /settings/retention` with
  `{ version: 2, autoArchiveAfterDays, autoArchiveActiveSessions, deleteArchivedAfterDays }`
  (validating `0..=999` bounds). Start `orkworksd` with `retention_applied =
  false` so `retention_cleanup_once` is a no-op until settings are explicitly
  pushed.
- [ ] Add in-memory `last_interactive_at`, `auto_archive_warned_at`, and
  `auto_archive_scheduled_at` to `SessionHandle`; update `last_interactive_at`
  on terminal user input writes and `last_activity` advances (never on raw
  `last_output_at`), clear warnings immediately on user input, `working`
  transition, favorite, close, or retention-settings change, and project
  `autoArchiveWarning: { warnedAt, scheduledAt }`.
- [ ] Write deterministic clock-injected failing tests for `retention.rs`
  covering: fail-closed no-op before `POST /settings/retention`; Stage 1A
  `Closed -> Archived` using `max(last_activity, restoredAt)`; Stage 1B idle
  `Active` warning issuance (ignoring raw `last_output_at` redraws, honoring
  terminal input, excluding `working` / `creating` / `stopping` / `finalizing` /
  `favorited` / orchestration-owned sessions); sleep-safe
  `max(..., now + leadTime)` scheduling AND wall-clock sleep-gap detection
  (`now - last_retention_tick_at > 15m`) renewing existing active warning
  deadlines before expiration (including a dedicated test where sleep begins
  after warning issuance and spans the original deadline); Close-first archival
  after grace expiry with locks released across awaits and refusal on Close
  timeout; Stage 2
  `Archived -> Deleted` using `archivedAt` while skipping `favorited`,
  non-removed `managedWorktree`, pending, and corrupt records.
- [ ] Implement the staged retention loop in `runtime/retention.rs` (removing
  legacy `max_sessions` / `max_age_days` selection while keeping observation
  anti-spam trim intact and preserving an invocation seam for Plan 2's
  archive-time managed-worktree cleanup).
- [ ] Update Electron `settingsMemory.ts` (`DEFAULT_RETENTION` v2 defaults
  `7 / true / 30` and legacy `settings.json` migration preserving explicit
  `maxAgeDays > 0` as `deleteArchivedAfterDays`) and harden
  `workspaceRuntime.ts` / `backendRestoration.ts` to retry
  `POST /settings/retention` with bounded backoff until acknowledged.
- [ ] Run targeted Rust retention/runtime tests and Electron settings/restoration
  tests; confirm all pass.

## Delivery 3: desktop UI for Settings, Favorites, and idle-Active warnings

Files: `apps/desktop/src/api.ts`; `apps/desktop/src/workspaceSessionController.ts`;
`apps/desktop/src/components/SettingsModal.tsx`; `apps/desktop/src/components/SessionListPanel.tsx`;
`apps/desktop/src/components/SessionContextMenu.tsx`; `apps/desktop/src/components/SessionDetailPanel.tsx`;
`apps/desktop/src/components/ApplicationShell.tsx`; `apps/desktop/src/App.tsx`;
`apps/desktop/src/App.css`; `docs/user/sessions.md`.

- [ ] Replace the **Session retention** section in `SettingsModal.tsx` with the
  three staged controls (`autoArchiveAfterDays`, `autoArchiveActiveSessions`,
  `deleteArchivedAfterDays`) and inline protection note for Favorites and dirty
  worktrees.
- [ ] Add **Favorite / Unfavorite** actions to `SessionContextMenu.tsx` and
  `SessionDetailPanel.tsx`, a subtle star indicator on favorited rows, and
  **Favorites** in the existing State filter (`SessionListPanel.tsx`) showing
  all favorited sessions across `Active`, `Closed`, and `Archived`.
- [ ] Surface `autoArchiveWarning` in `SessionListPanel.tsx` (subtle row
  indicator), `SessionDetailPanel.tsx` and Terminal header (non-blocking banner
  with one-click **Favorite (keep)** and **Archive now**), and emit at most one
  desktop OS notification per `(sessionId, warnedAt)` when desktop
  notifications are enabled.
- [ ] Update Restore helper copy (fresh auto-archive window) and Delete
  confirmation copy (calling out when deleting a Favorited session).
- [ ] Add desktop unit/component tests and an Electron shell workflow fixture
  covering Settings defaults/migration, cross-state Favorites filter/toggle,
  warning banner + notification deduplication, and Delete confirmation copy;
  update `docs/user/sessions.md`.

## Delivery 4: verify, review, and land

- [ ] Run `bash scripts/verify-repo.sh`, `bash scripts/doc-check.sh`, and
  `bash .claude/hooks/worktree-check.sh`; inspect any warnings against the diff.
- [ ] Complete diff-scoped `/code-review medium` (justified by schema migration,
  retention lifecycle, and Close-first concurrency changes) and address all
  findings before merge.
- [ ] Push branch, open PR, complete `babysitting-pull-requests` checks/comments
  before authorized merge, and clean up the owned worktree via
  `bash scripts/finish-pr.sh <PR>`.

## Planning review

Disposition: **Ready** (for owner review and execution once the tracking GitHub
issue is created and Delivery 1's ADR/MVP update lands first).

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3 | Known sidecar, Electron settings, and renderer owners; no new external dependencies; Plan 2 depends on this `OrganizationStore` v2 foundation |
| Blast radius | 4 | Changes default-on retention, `OrganizationStore` schema, `workspace_gc`, and Sessions/Settings UI; mitigated by fail-closed startup gating and v1 archive-to-favorite promotion |
| State changes | 3 | Additive `OrganizationStore` v2 fields and `settings.json` retention v2 migration; v1 readers fail closed on v2 files without deleting them |
| Reversibility | 3 | Retention can be disabled (`0`) immediately in Settings; code rollback preserves v2 files as protected under v1 readers (documented in release notes) |
| Uncertainty | 2 | Existing `session_archive.rs`, `runtime/retention.rs`, `workspace_gc.rs`, and `settingsMemory.ts` were inspected; deterministic clock and Close-first tests cover all async edges |

Total: 15. Key failure treatments: sidecar never runs default-on retention
before `POST /settings/retention` succeeds; pre-existing v1 archived sessions
auto-promote to `favorited: true`; unfavoriting an archived session resets
`archivedAt = now`; Active sessions never close without a sleep-safe grace
window after warning; corrupt records and dirty worktrees stay protected.
