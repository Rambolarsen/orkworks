# Panel Persist & Restore — Design

**Date:** 2026-06-17  
**Status:** approved  

## Problem

Users can close Dockview panels via the built-in tab `×` button, but there is no mechanism to restore them. The `onReady` handler in `DockviewApp.tsx` that creates the 5-panel default layout is guarded by a `useRef` that prevents it from ever running again. Closing a panel requires restarting the app to recover it.

Additionally, custom panel arrangements (drag-and-drop reordering) are lost on every restart because no layout persistence exists.

## Solution

Full layout persistence to Electron `userData` via Dockview's `toJSON()`/`fromJSON()` serialization, plus a View menu in the titlebar that lets users toggle individual panel visibility and reset to the default layout.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Renderer process                                   │
│                                                     │
│  App.tsx                                            │
│  ├─ .titlebar div (sibling of DockviewApp)          │
│  │   └─ <ViewMenu> button                           │
│  └─ DockviewApp.tsx                                 │
│         ├─ DockviewReact (5 panels)                 │
│         ├─ onReady:                                  │
│         │   ├─ getLayout() → fromJSON() or default │
│         │   └─ onDidLayoutChange → debounce →       │
│         │       saveLayout(toJSON())                │
│         └─ DockviewApi ref (shared via context)     │
│                                                     │
│  ViewMenu.tsx                                       │
│  ├─ Reads api.getPanel(id) for checkmarks           │
│  ├─ Toggle: api.addPanel(id) / panel.api.close()    │
│  └─ Reset: api.clear() + rebuild default layout     │
│                                                     │
│  window.orkworks.getLayout() / saveLayout(json)     │
└───────────────────────┬─────────────────────────────┘
                        │ IPC (contextBridge)
┌───────────────────────┴─────────────────────────────┐
│  Main process                                       │
│                                                     │
│  electron/main.ts                                   │
│  ├─ ipcMain.handle("get-layout", ...)               │
│  └─ ipcMain.handle("save-layout", ...)              │
│                                                     │
│  electron/layoutMemory.ts (new)                     │
│  ├─ readLayoutMemory(userData): DockviewJSON | null │
│  └─ writeLayoutMemory(userData, json): void         │
│                                                     │
│  layout.json        (in Electron userData)          │
└─────────────────────────────────────────────────────┘
```

### Main process (`electron/main.ts`)

Two new IPC handlers, following the pattern established by the existing `get-backend-url` and `get-initial-workspace` handlers:

```ts
ipcMain.handle("get-layout", async () => {
  return readLayoutMemory(app.getPath("userData"));
});

ipcMain.handle("save-layout", async (_event, json: string) => {
  await writeLayoutMemory(app.getPath("userData"), json);
});
```

The `save-layout` handler awaits `writeLayoutMemory` to serialize concurrent writes — see below.

### New file: `electron/layoutMemory.ts`

Mirrors `workspaceMemory.ts` pattern. Reads/writes `layout.json` from the Electron `userData` directory.

```
export function readLayoutMemory(userDataPath: string): string | null
export function writeLayoutMemory(userDataPath: string, json: string): Promise<void>
```

Behavior:
- Missing file → `readLayoutMemory` returns `null`.
- Corrupt JSON → catches, `console.warn`s, returns `null`.
- Creates parent directory as needed.
- Validates that the input is a truthy string before writing, rejects strings larger than 1 MB (sanity cap against a runaway `toJSON()` corrupting the file), and `console.warn`s on rejection.
- **Write serialization:** `writeLayoutMemory` is `async` and uses a module-level write queue (a single in-flight promise chained on each call) so two rapid `invoke`s from the renderer cannot interleave. Each write goes to a temp file (`layout.json.tmp`) and is renamed over `layout.json` to make crash-mid-write fall back to the previous good layout rather than a half-written one.
- Write failures (disk full, permission) are `console.warn`ed in main; the renderer treats the IPC call as fire-and-forget.

### Preload (`electron/preload.ts`)

Two new methods on `window.orkworks`:

```ts
getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
```

### App.tsx type declaration

The global `window.orkworks` interface declaration in `App.tsx` gains two new methods.

### DockviewApp.tsx changes

**Keep an `initializedRef` guard scoped to the api instance.** The existing `onReadyRef` is renamed and re-purposed: React 18 Strict Mode fires `onReady` twice in dev, and Dockview reuses the api instance across remounts. Without a guard, we would call `fromJSON` (or rebuild defaults) twice and stack two `onDidLayoutChange` subscriptions. The guard ensures the initial restore and the subscription happen exactly once per api instance.

The `onReady` callback now does:

1. If `initializedRef.current === event.api`, return. Otherwise set `initializedRef.current = event.api` and store the api on `dockviewApiRef.current` for the ViewMenu.
2. Set `restoringRef.current = true` (see below).
3. Calls `window.orkworks.getLayout()`:
   - If a wrapper object is returned and `wrapper.version === LAYOUT_VERSION`: `event.api.fromJSON(wrapper.layout)` inside a try/catch. On throw, log a warning and fall through to the default-build branch.
   - If `null`, an unknown version, or a `fromJSON` failure: run `buildDefaultLayout(event.api)` (the existing 5-panel logic, extracted to a helper iterating `PANEL_DEFAULTS` in order).
4. Clear `restoringRef.current` on the next microtask (`queueMicrotask` or `Promise.resolve().then(...)`) so any layout-change events emitted *during* restore are swallowed.
5. Subscribes to `event.api.onDidLayoutChange`:
   - Early-return if `restoringRef.current` is true (suppresses the re-save of the layout we just loaded).
   - Otherwise debounced at 500ms via a `useRef<ReturnType<typeof setTimeout>>` timer.
   - On fire: `window.orkworks.saveLayout(JSON.stringify({ version: LAYOUT_VERSION, layout: event.api.toJSON() }))`.
6. On component unmount: if the debounce timer is pending, clear it and issue a final synchronous `saveLayout` so a close-then-quit within 500ms does not lose the last change.

**Layout version constant.** A top-level `const LAYOUT_VERSION = 1;` accompanies the persisted wrapper `{ version, layout }`. Bumping this constant invalidates older saved layouts (they fall back to defaults) and is the documented mechanism for handling future Dockview JSON shape changes.

**Exports `PANEL_DEFAULTS`** — an ordered array of default panel descriptors used by both the startup default layout and the ViewMenu reset/toggle. An array is used (not an object) because Reset Layout depends on insertion order — `sessions` must be added first as root before any other panel can reference it. Object property order is technically preserved in modern JS but is fragile to refactors; an array makes the ordering invariant explicit and reviewable.

```ts
export const PANEL_DEFAULTS = [
  { id: "sessions",        component: "sessions",        title: "Sessions" },
  { id: "detail",          component: "detail",          title: "Detail",          position: { referencePanel: "sessions", direction: "below" as const } },
  { id: "terminal",        component: "terminal",        title: "Terminal",        position: { referencePanel: "sessions", direction: "right" as const } },
  { id: "capacity",        component: "capacity",        title: "Capacity",        position: { referencePanel: "terminal", direction: "right" as const } },
  { id: "recommendations", component: "recommendations", title: "Recommendations", position: { referencePanel: "capacity", direction: "below" as const } },
] as const;
```

**Provides `DockviewApi` to the ViewMenu** — either through the existing `DockviewContext` or a separate ref lifted to `App.tsx`.

**The titlebar (and ViewMenu) live outside the Dockview root.** `App.tsx:162` renders `.titlebar` as a sibling of `<DockviewApp>`, so a corrupt saved layout can never hide the View menu. The user always has a path to "Reset Layout" regardless of dockview state.

### ViewMenu component (`src/components/ViewMenu.tsx`)

A dropdown button in the titlebar, rendered inside the existing `.titlebar` div in `App.tsx`.

**State:** None. Reads live panel state from the Dockview API on each render (menu open).

**Rendering:**
- Trigger: `<button class="view-menu-trigger">View ▾</button>`
- Dropdown: `<ul class="view-menu-dropdown">` with absolute positioning
- Each panel: `<li>` with class `checked` or empty based on `api.getPanel(id) !== undefined`
- Checkmark: `✓` text character (no emoji)
- Click handler:
  1. `const panel = api.getPanel(id)`
  2. If panel exists: `panel.api.close()`
  3. If not: `api.addPanel({ id, component, title, ...position })`
- Separator: `<li class="view-menu-separator"><hr /></li>`
- Reset: `<li class="view-menu-reset">Reset Layout</li>` — calls `api.clear()` then rebuilds all 5 panels from `PANEL_DEFAULTS` in the default order (sessions first as root, then positioned relative to sessions or terminal)

**Edge case — reference panel missing:** When adding a panel whose `referencePanel` is also absent (e.g., adding "capacity" when "terminal" was also closed), fall back to anchoring against `sessions` (the universal root anchor that is always restored first by Reset Layout and is the only panel without a `referencePanel`). If `sessions` itself is also absent, omit `position` entirely and let Dockview place the panel in a new root group. The next `saveLayout` will persist whatever the user does next.

**Dropdown close:** Click outside the dropdown or press Escape closes it. Managed via a `useEffect` that adds a `mousedown` listener on `document` and an Escape `keydown` listener.

### CSS additions (`App.css`)

New selectors under the existing `.titlebar` block:
- `.view-menu-trigger` — styled like `.status-badge`, cursor pointer
- `.view-menu-dropdown` — dark background, border, z-index above dockview panels, min-width 180px
- `.view-menu-dropdown li` — padding, hover highlight
- `.view-menu-dropdown li.checked::before` — `content: "✓ "`
- `.view-menu-separator hr` — thin border, margin
- `.view-menu-reset` — slightly different color to distinguish action

---

## Data Flow

### Path 1: Startup restore

```
App mounts
  → DockviewReact fires onReady
    → if initializedRef.current === event.api: return  // strict-mode guard
    → initializedRef.current = event.api
    → dockviewApiRef.current = event.api
    → restoringRef.current = true
    → wrapper = await window.orkworks.getLayout()
    → if wrapper && wrapper.version === LAYOUT_VERSION:
        try { event.api.fromJSON(wrapper.layout) }
        catch { buildDefaultLayout(event.api) }   // iterate PANEL_DEFAULTS in order
      else:
        buildDefaultLayout(event.api)
    → queueMicrotask(() => { restoringRef.current = false })
    → event.api.onDidLayoutChange(() => {
        if (restoringRef.current) return           // swallow restore-time events
        clearTimeout(timer)
        timer = setTimeout(() => {
          window.orkworks.saveLayout(JSON.stringify({
            version: LAYOUT_VERSION,
            layout: event.api.toJSON(),
          }))
        }, 500)
      })

// useEffect cleanup on unmount:
//   if (timer pending) clearTimeout(timer)
//                       + final synchronous saveLayout(...) flush
```

### Path 2: View menu toggle

```
User clicks View → "Recommendations" (unchecked)
  → panel = dockviewApi.getPanel("recommendations")
  → panel is null:
      dockviewApi.addPanel({
        id: "recommendations",
        component: "recommendations",
        title: "Recommendations",
        position: { referencePanel: "capacity", direction: "below" }
      })
  → onDidLayoutChange fires → debounce save
```

### Path 3: View menu toggle (close)

```
User clicks View → "Capacity" (checked)
  → panel = dockviewApi.getPanel("capacity")
  → panel.api.close()
  → onDidLayoutChange fires → debounce save
```

### Path 4: Reset Layout

```
User clicks View → "Reset Layout"
  → clearAllPanels(dockviewApi)               // see note below
  → for each descriptor in PANEL_DEFAULTS:    // array, fixed order
      dockviewApi.addPanel(descriptor)
  → onDidLayoutChange fires → debounce save
```

**`clearAllPanels` implementation note:** The design originally proposed `api.clear()`. Confirm at implementation time that `clear()` is exposed on the public `DockviewApi` in v6.6.1. If not, fall back to iterating `api.panels` and calling `panel.api.close()` on each before re-adding from `PANEL_DEFAULTS`. Either way the public observable behavior is identical.

### Path 5: Drag-and-drop persistence

```
User drags a panel tab to a new group/position
  → onDidLayoutChange fires
  → debounce 500ms
  → window.orkworks.saveLayout(JSON.stringify(api.toJSON()))
  → layout.json written to userData
```

---

## Error Handling

| Scenario | Behavior |
|---|---|
| `layout.json` missing | `getLayout()` returns `null` → default layout built |
| `layout.json` corrupt JSON | `readLayoutMemory` catches, logs warning, returns `null` → default layout |
| Saved layout has unknown `version` | Treated as `null` → default layout built (mechanism for handling future Dockview JSON shape changes) |
| `fromJSON()` throws on bad data | try/catch, log warning, fall back to default layout |
| `saveLayout()` fails (disk full, etc.) | `console.warn` in main; renderer treats as fire-and-forget, UI not blocked |
| Concurrent `saveLayout` IPC calls | Serialized in `layoutMemory.ts` via a module-level promise chain + temp-file rename, so writes cannot interleave or leave a half-written file |
| Reference panel missing on addPanel | Fall back to anchoring on `sessions`; if `sessions` is also absent, omit `position` |
| Layout-change events emitted *during* restore | `restoringRef` guard suppresses the immediate re-save of the just-loaded layout |
| Debounced save pending at unmount/quit | Flushed synchronously on component unmount so a close-then-quit within 500ms does not lose the last change |
| Double `onReady` fire (React Strict Mode) | `initializedRef` guard keyed to `event.api`: initial restore and `onDidLayoutChange` subscription happen exactly once per api instance |
| Corrupt layout hides the View menu | Impossible: the titlebar (and ViewMenu) render outside the Dockview root, so Reset Layout is always reachable |

---

## Files Changed

| File | Change |
|---|---|
| `electron/layoutMemory.ts` | **New.** Read/write `layout.json` from Electron userData |
| `electron/main.ts` | Add `get-layout` and `save-layout` IPC handlers |
| `electron/preload.ts` | Expose `getLayout()` and `saveLayout()` on `window.orkworks` |
| `apps/desktop/src/App.tsx` | Update `window.orkworks` type, add ViewMenu to titlebar |
| `apps/desktop/src/components/DockviewApp.tsx` | Remove `onReadyRef`, add persist/restore logic, export `DockviewApi` ref, define `PANEL_DEFAULTS` |
| `apps/desktop/src/components/ViewMenu.tsx` | **New.** View dropdown with panel toggles and reset |
| `apps/desktop/src/App.css` | Add ViewMenu styles |

## Files NOT Changed

| File | Reason |
|---|---|
| `crates/orkworksd/` | No backend changes needed — layout is purely UI state |
| `.orkworks/` protocol | Layout is app-local, not workspace-specific |
| `docs/agents/architecture.md` | Update trigger applies — see below |

---

## Architecture Doc Update Trigger

Update `docs/agents/architecture.md` as part of this work. Specifically:

- Panel layout section: document the persist/restore lifecycle, the `PANEL_DEFAULTS` ordering invariant, and the `ViewMenu` titlebar entry point.
- Preload section: document the new `getLayout()` and `saveLayout()` methods on `window.orkworks`.
- Electron main-process module list: add `electron/layoutMemory.ts` alongside `electron/workspaceMemory.ts` (a new preload-exposed API surface is part of what the architecture doc tracks).

---

## Spec Self-Review

- **Placeholder scan:** No TBDs or TODOs.
- **Internal consistency:** All file paths match actual repo structure. `fromJSON`/`toJSON` types align with Dockview v6.6.1 API.
- **Scope check:** Single focused feature — panel persist + restore + View menu. No UI refactoring, no workspace-specific layouts, no backend changes.
- **Ambiguity check:** All edge cases (missing file, corrupt JSON, missing reference panel, double onReady) have explicit behavior defined.
