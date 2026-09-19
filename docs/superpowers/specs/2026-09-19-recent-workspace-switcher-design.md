# Recent workspace switcher (pin / remove) — design

Status: draft, pending user review
Date: 2026-09-19
Builds on: [`specs/multi-workspace.md`](../../../specs/multi-workspace.md) (installation-scoped
workspace history), [ADR 0060](../../adr/0060-independent-workspace-instances.md) (accepted)

## Purpose

The installation-scoped workspace history introduced by ADR 0060 / PR #569 /
PR #575 already persists a recency-ordered `recentWorkspacePaths` list on disk,
but nothing in the UI ever reads it — the app only ever uses it as a single
`lastWorkspacePath` startup hint. There is no way for a user to see or switch
among previously opened workspaces without re-browsing the filesystem through
the native folder picker each time.

This feature makes that existing history visible and actionable: list
remembered workspaces, let the user switch to one directly, and let them pin
workspaces they return to often so those survive recency-based eviction and
sort to the top.

## Non-goals

This feature only changes which workspace *this* OrkWorks instance opens
next. Consistent with the non-goals already declared in
`specs/multi-workspace.md`, it does not:

- Show or affect the state of a workspace open in another OrkWorks instance.
- Add a live, cross-instance dashboard or focus switcher.
- Change the close-then-open switch lifecycle, lease/adoption behavior, or
  crash recovery in any way — it reuses that machinery unchanged.
- Auto-open or auto-resume anything; every switch remains a deliberate,
  user-initiated action gated by the existing confirmation-on-uncertain-work
  step.

## Data model change (v1 → v2)

Current on-disk shape (`apps/desktop/electron/workspaceMemory.ts`):

```json
{
  "version": 1,
  "revision": 42,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"]
}
```

New shape:

```json
{
  "version": 2,
  "revision": 43,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"],
  "pinnedWorkspacePaths": ["<canonical path>"]
}
```

Rules:

- `pinnedWorkspacePaths` is a separate list from `recentWorkspacePaths`. A
  path appears in at most one of the two lists at a time.
- Pinned entries are never evicted by the existing 20-entry /
  64 KiB `recentWorkspacePaths` cap. They get their own bound — 50 entries,
  and count toward the file's existing 64 KiB total serialized-size cap — so
  the file stays bounded even though pinning is "effectively unlimited" for
  realistic use.
- **Pinned-cap overflow needs new logic, not reuse of the existing byte-fit
  diagnostic.** The existing "a single path that cannot fit returns a
  diagnostic without a silent no-op" behavior in `updateWorkspaceMemory`
  reacts only to *byte-size* overflow on a write; it has no notion of a
  *count* cap, so a 51st pin attempt while the file still fits in 64 KiB
  would not trip it. `pinWorkspacePath` must check
  `pinnedWorkspacePaths.length >= 50` itself before attempting the write and
  return a new, distinct diagnostic (e.g. `pin_limit_reached`) without
  mutating the file, alongside the existing byte-size check for the
  (unlikely, since paths are short relative to 64 KiB / 50) case where 50
  entries themselves overflow the byte budget.
- Pinning a path already in `recentWorkspacePaths` moves it to
  `pinnedWorkspacePaths` (removed from the former). Unpinning does the
  reverse and the path re-enters `recentWorkspacePaths` at the front (as if
  just opened), rather than being lost.
- `forgetWorkspacePath` is extended to remove the path from whichever list
  currently holds it (today it only touches `recentWorkspacePaths`).
- **`rememberWorkspacePath` becomes pin-aware.** Today it unconditionally
  inserts the opened path at the front of `recentWorkspacePaths` and sets
  `lastWorkspacePath`, and it runs on every successful open (including opens
  driven by `open-remembered-workspace`, review finding: opening a pinned
  workspace would otherwise re-add it to `recentWorkspacePaths` and violate
  the "a path lives in at most one list" rule above). The fix: if the opened
  path is already in `pinnedWorkspacePaths`, `rememberWorkspacePath` updates
  `lastWorkspacePath` only and leaves both lists' membership unchanged —
  it never inserts into `recentWorkspacePaths` for an already-pinned path.
- **The v2 read-time validity invariant changes.** The current validator
  (`validStoredMemory`) requires `lastWorkspacePath === null ||
  recentWorkspacePaths.includes(lastWorkspacePath)`. Because
  `rememberWorkspacePath` can now set `lastWorkspacePath` to a path that
  lives only in `pinnedWorkspacePaths` (see above), the v2 validator must
  relax this to `lastWorkspacePath === null ||
  recentWorkspacePaths.includes(lastWorkspacePath) ||
  pinnedWorkspacePaths.includes(lastWorkspacePath)`. Without this change a
  correctly written v2 file recording a pinned last-opened workspace would
  fail its own validation and be treated as corrupt on the next launch.
- Every mutation continues to use the existing short-lived advisory lock,
  revision reread/increment, and same-directory atomic replace — no changes
  to that mechanism.

New/changed functions in `electron/workspaceMemory.ts`:

- `pinWorkspacePath(userDataPath, path)`
- `unpinWorkspacePath(userDataPath, path)`
- `forgetWorkspacePath(userDataPath, path)` — extended, not replaced
- `rememberWorkspacePath(userDataPath, path)` — extended to skip inserting
  into `recentWorkspacePaths` when `path` is already pinned (see above)
- `readWorkspaceMemory` — return type gains `pinnedWorkspacePaths`
- `validStoredMemory` (or its v2 equivalent) — relaxed `lastWorkspacePath`
  membership check described above

### ADR

This is a protocol/schema decision (new persisted field, new file version).
Per the root `AGENTS.md` rule ("Create an ADR before writing any
implementation code for a decision that shapes the architecture, stack, or
protocol"), the implementation plan's **first task**, ordered strictly
before any `workspaceMemory.ts` schema/mutator code or IPC code, must be
writing a short ADR amendment note to ADR 0060 (or a new small ADR,
whichever the amend/supersede guidance in
`docs/agents/development-workflow.md` calls for) documenting the v1 → v2
shape and the pinned/recent split. This design doc identifies the
requirement; the plan must sequence it as a blocking first step, not merely
mention it somewhere.

## IPC contract (main ↔ preload ↔ renderer)

New channels in `electron/main.ts`, mirrored independently in
`electron/preload.ts` and `src/orkworksWindow.d.ts` per the electron/src
duplication rule in `apps/desktop/AGENTS.md`:

- `get-workspace-history` → `{ pinned: string[]; recent: string[]; diagnostic: WorkspaceHistoryDiagnostic | null }`
- `pin-workspace-path(path: string)` → same shape, updated
- `unpin-workspace-path(path: string)` → same shape, updated
- `forget-workspace-path(path: string)` → same shape, updated (this channel
  may already need to be added fresh — today `forgetWorkspacePath` is only
  called internally by main.ts when a remembered path is rejected on open,
  not exposed to the renderer)
- `open-remembered-workspace(path: string)` → same return contract as
  today's `open-workspace`. Implementation: if `path` is already the active
  workspace, return it unchanged without invoking the coordinator (see
  current-workspace short-circuit in the UI section below — this is
  defense in depth, not the primary guard). Otherwise call the existing
  `workspaceSwitchCoordinator.pickWorkspace(async () => path)` — i.e. supply
  the already-known path instead of invoking `dialog.showOpenDialog`. This is
  the only lifecycle-affecting change, and it changes nothing about
  `pickWorkspace` itself: same confirmation-on-uncertain-work step, same
  generation guards, same serialized/coalesced switch behavior described in
  `specs/multi-workspace.md`'s close-then-open lifecycle.

## UI

A new `WorkspaceHistoryList` renderer component, reused in two places (per
user decision):

1. **Workspace picker screen** (no workspace currently open): rendered as
   the primary content alongside the existing "Open workspace" button, so a
   user with history sees their pinned/recent workspaces immediately instead
   of only a button that opens a folder browser.
2. **Titlebar switcher dropdown**: the existing ⇄ button in the titlebar
   (`handleOpenWorkspace` in `src/App.tsx`) currently calls
   `window.orkworks.openWorkspace()` directly, which always shows the native
   dialog. It changes to open a small dropdown containing the same
   `WorkspaceHistoryList`, plus a trailing "Open other folder…" item that
   preserves today's native-dialog behavior unchanged.

Each row: last path segment as the label, full canonical path as the
`title` tooltip (matching the existing titlebar workspace-name pattern), a
pin/unpin icon button, and a remove (✕) icon button. Pinned rows render in
a "Pinned" section above an "Recent" section; an empty pinned section is
omitted rather than shown empty. New copy (section headers, pin/unpin/
remove labels, "Open other folder…") is added to the `VOCAB` table in
`src/labels.ts` alongside the existing `openWorkspace`/`switchWorkspace`
entries, not inlined as string literals, following that file's established
"canonical vocabulary, one word per concept" convention.

**Row for the currently-open workspace:** `WorkspaceHistoryList` renders
that row as non-interactive for switching (visually marked, e.g. a
"Current" badge, no click handler wired to open) — `switchWorkspaceInternal`
in `workspaceSwitchCoordinator.ts` always tears down and relaunches the
sidecar with no short-circuit for "destination equals current workspace,"
so treating that row as clickable would cause a pointless close/reopen
cycle. Pin/unpin and remove-from-history controls remain available on that
row (removing a currently-open workspace from history only deletes the
shortcut per the existing `forgetWorkspacePath` contract; it does not close
it). As defense in depth against a stale render (e.g. a click event queued
just as the workspace changed underneath it), `open-remembered-workspace`
in `main.ts` also short-circuits and returns the current workspace
unchanged if invoked with the currently-active path, rather than relying
solely on the UI to prevent it.

Row click (not on the icon buttons, and not on the current-workspace row)
calls `window.orkworks.openRememberedWorkspace(path)` and follows the same
`isSwitchingWorkspace` / toast-on-failure pattern `handleOpenWorkspace`
already uses.

**Dropdown dismissal:** a repo search of `apps/desktop/src/` found no
existing outside-click/`role="menu"`/popover pattern to reuse, so the
titlebar dropdown implements its own minimal version: a controlled-
visibility panel anchored under the ⇄ button, closed on a document-level
`mousedown` outside the panel and on `Escape`, with no new dependency or
icon library introduced.

## Error handling

- Corrupt or unreadable history file: unchanged existing behavior — the
  `historyDiagnostic` alert ("Workspace history unavailable") continues to
  render, and `WorkspaceHistoryList` renders empty rather than guessing at
  partial data, matching the spec's "corrupt history is left untouched...
  surfaced as diagnostic" rule.
- No proactive existence/accessibility probing of every remembered path on
  render — that would be slow and race against concurrent filesystem state.
  Instead, attempting to open a since-removed/inaccessible path fails through
  the *existing* rejected-path handling in `main.ts` (which already calls
  `forgetWorkspacePath` on a rejected remembered path today for the
  single-`lastWorkspacePath` case) plus a toast, reusing that path for both
  pinned and recent entries.
- Switching while a switch is already in flight is a no-op/coalesced, per
  the existing `workspaceSwitchCoordinator` serialization — no new gating
  logic needed since `open-remembered-workspace` goes through the same
  coordinator as `open-workspace`.

## Testing

- Extend `apps/desktop/tests/electronWorkspaceMemory.test.ts`: pin/unpin
  round-trip, forget removing from either list, the v1 → v2 migration
  default, `rememberWorkspacePath` not duplicating an already-pinned path
  into `recentWorkspacePaths`, the relaxed `lastWorkspacePath` validity
  invariant covering a pinned-only last-opened path, the pinned-list
  count-cap diagnostic (distinct from the existing byte-fit diagnostic) at
  and beyond 50 entries, and lock/revision behavior for the new mutators
  (mirroring the existing concurrent-writer tests).
- New or extended renderer test for `WorkspaceHistoryList`: rendering
  pinned/recent sections, pin/unpin/remove button wiring, the picker vs.
  titlebar-dropdown placement, the current-workspace row rendering as
  non-clickable-for-switching while keeping pin/remove available, and
  dropdown dismissal on outside-click/Escape.
- Manual verification in the running app (`pnpm dev`) covering: opening from
  history, pinning, unpinning, removing, and the empty-history / corrupt-
  history states — per the UI-verification bar in the root `AGENTS.md`.

## Open items for the implementation plan

- Exact icon/asset choice for pin and remove controls (follow existing
  icon conventions in `src/App.tsx`/CSS rather than introducing a new icon
  library).
