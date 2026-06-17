# Panel Persist & Restore — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist Dockview panel layout to Electron userData so custom arrangements survive app restarts, and add a View menu to toggle individual panels and reset to default layout.

**Architecture:** Electron main process stores `layout.json` in userData via a new `layoutMemory.ts` (mirroring `workspaceMemory.ts`). Two new IPC handlers expose get/save. Renderer persists Dockview `toJSON()`/`fromJSON()` via preload bridge. A ViewMenu dropdown in the titlebar toggles panel visibility and offers "Reset Layout".

**Tech Stack:** TypeScript, React, Electron (IPC/contextBridge), Dockview v6.6.1

---

### Task 1: Create `electron/layoutMemory.ts`

**Files:**
- Create: `apps/desktop/electron/layoutMemory.ts`

- [ ] **Step 1: Write the module**

Implements the spec requirements: async write, module-level promise-chain queue (serializes concurrent writes from the renderer), temp-file + atomic rename (crash-mid-write falls back to the prior good layout), 1 MB sanity cap, truthy-string validation.

```typescript
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const fileName = "layout.json";
const tempFileName = "layout.json.tmp";
const MAX_LAYOUT_BYTES = 1024 * 1024;

function layoutMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

function tempLayoutPath(userDataPath: string): string {
  return join(userDataPath, tempFileName);
}

export function readLayoutMemory(userDataPath: string): string | null {
  const path = layoutMemoryPath(userDataPath);
  if (!existsSync(path)) {
    return null;
  }
  try {
    const raw = readFileSync(path, "utf8");
    JSON.parse(raw);
    return raw;
  } catch {
    console.warn("[layoutMemory] corrupt layout.json, ignoring");
    return null;
  }
}

let writeQueue: Promise<void> = Promise.resolve();

export function writeLayoutMemory(userDataPath: string, json: string): Promise<void> {
  if (typeof json !== "string" || json.length === 0) {
    console.warn("[layoutMemory] refusing to write empty layout");
    return writeQueue;
  }
  if (json.length > MAX_LAYOUT_BYTES) {
    console.warn(`[layoutMemory] refusing to write layout >${MAX_LAYOUT_BYTES} bytes`);
    return writeQueue;
  }
  writeQueue = writeQueue.then(() => {
    try {
      mkdirSync(userDataPath, { recursive: true });
      const tmp = tempLayoutPath(userDataPath);
      writeFileSync(tmp, json);
      renameSync(tmp, layoutMemoryPath(userDataPath));
    } catch (e) {
      console.warn("[layoutMemory] failed to write layout.json", e);
    }
  });
  return writeQueue;
}
```

- [ ] **Step 2: Verify the file compiles**

```bash
npx tsc --noEmit apps/desktop/electron/layoutMemory.ts
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/electron/layoutMemory.ts
git commit -m "feat: add layoutMemory module for panel layout persistence"
```

---

### Task 2: Add IPC handlers in `electron/main.ts`

**Files:**
- Modify: `apps/desktop/electron/main.ts`

- [ ] **Step 1: Add import and IPC handlers**

In `main.ts`, add the import for `layoutMemory` (after line 6):

```typescript
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";
```

Then add two new IPC handlers inside the `app.whenReady().then(() => { ... })` block, after the existing `get-backend-url` handler (after line 96):

```typescript
  ipcMain.handle("get-layout", async () => {
    return readLayoutMemory(app.getPath("userData"));
  });

  ipcMain.handle("save-layout", async (_event, json: string) => {
    await writeLayoutMemory(app.getPath("userData"), json);
  });
```

- [ ] **Step 2: Verify with TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/electron/main.ts
git commit -m "feat: add get-layout and save-layout IPC handlers"
```

---

### Task 3: Update `electron/preload.ts`

**Files:**
- Modify: `apps/desktop/electron/preload.ts`

- [ ] **Step 1: Add layout methods**

```typescript
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
});
```

- [ ] **Step 2: Verify with TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/electron/preload.ts
git commit -m "feat: expose getLayout and saveLayout via preload bridge"
```

---

### Task 4: Update `App.tsx` window.orkworks type and add ref + ViewMenu wiring

**Files:**
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Add layout methods to the global type declaration and add DockviewApi import**

In `App.tsx`, update the `declare global` block (lines 14-22) to include the new methods:

```typescript
import { useCallback, useEffect, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import DockviewApp from "./components/DockviewApp";
import { sortSessions } from "./components/RightSidebarHelpers";
import {
  type SessionInfo,
  type WorkspaceInfo,
  createSession,
  listSessions,
  deleteSession,
  resumeSession,
  setActiveWorkspaceSession,
} from "./api";
import ViewMenu from "./components/ViewMenu";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
    };
  }
}
```

- [ ] **Step 2: Add `dockviewApiRef` and wire ViewMenu into the titlebar**

In the `App` function, add after the existing `useState` lines (after line 28):

```typescript
  const dockviewApiRef = useRef<DockviewApi | null>(null);
```

Then update the JSX return block (lines 160-182) to include ViewMenu in the titlebar and pass the ref to DockviewApp:

```tsx
  return (
    <div className="app-shell">
      <div className="titlebar">
        <span className="titlebar-text">OrkWorks</span>
        <div className="titlebar-right">
          <ViewMenu dockviewApiRef={dockviewApiRef} />
          <span
            className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
          >
            {backendStatus}
          </span>
        </div>
      </div>
      <DockviewApp
        backendStatus={backendStatus}
        workspace={workspace}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onOpenWorkspace={handleOpenWorkspace}
        onSelectSession={handleSelectSession}
        onCreateSession={handleCreateSession}
        onKillSession={handleKillSession}
        onResumeSession={handleResumeSession}
        dockviewApiRef={dockviewApiRef}
      />
    </div>
  );
```

- [ ] **Step 3: Verify with TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: errors about missing `dockviewApiRef` prop on `DockviewApp` and missing `ViewMenu` component — this is expected until Task 5 and Task 6 are done.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat: wire ViewMenu into titlebar and add dockviewApiRef"
```

---

### Task 5: Refactor `DockviewApp.tsx` — remove guard, add persist/restore, export PANEL_DEFAULTS

**Files:**
- Modify: `apps/desktop/src/components/DockviewApp.tsx`

- [ ] **Step 1: Add imports, LAYOUT_VERSION, PANEL_DEFAULTS (array), and shared helpers**

Replace the existing imports (lines 1-8) with:

```typescript
import { createContext, useContext, useEffect, useRef } from "react";
import { DockviewReact, type DockviewReadyEvent, type DockviewApi } from "dockview-react";
import type { SessionInfo, WorkspaceInfo } from "../api";
import SessionListPanel from "./SessionListPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import CapacityPanel from "./CapacityPanel";
import RecommendationsPanel from "./RecommendationsPanel";
```

After `const COMPONENTS = { ... };` (after line 77), add the layout version constant, the ordered `PANEL_DEFAULTS` array (the spec requires an array because Reset Layout depends on insertion order — `sessions` must be added first as the root anchor), and the shared `buildDefaultLayout` / `clearAllPanels` helpers reused by `ViewMenu`:

```typescript
export const LAYOUT_VERSION = 1;

export interface PanelDefault {
  id: string;
  component: string;
  title: string;
  position?: { referencePanel: string; direction: "below" | "right" };
}

export const PANEL_DEFAULTS: readonly PanelDefault[] = [
  { id: "sessions",        component: "sessions",        title: "Sessions" },
  { id: "detail",          component: "detail",          title: "Detail",          position: { referencePanel: "sessions", direction: "below" } },
  { id: "terminal",        component: "terminal",        title: "Terminal",        position: { referencePanel: "sessions", direction: "right" } },
  { id: "capacity",        component: "capacity",        title: "Capacity",        position: { referencePanel: "terminal", direction: "right" } },
  { id: "recommendations", component: "recommendations", title: "Recommendations", position: { referencePanel: "capacity", direction: "below" } },
] as const;

export function buildDefaultLayout(api: DockviewApi): void {
  for (const def of PANEL_DEFAULTS) {
    api.addPanel({
      id: def.id,
      component: def.component,
      title: def.title,
      ...(def.position ? { position: def.position } : {}),
    });
  }
}

export function clearAllPanels(api: DockviewApi): void {
  const maybeClear = (api as unknown as { clear?: () => void }).clear;
  if (typeof maybeClear === "function") {
    maybeClear.call(api);
    return;
  }
  for (const panel of [...api.panels]) {
    panel.api.close();
  }
}
```

- [ ] **Step 2: Add `dockviewApiRef` to the props interface**

Update the `DockviewAppData` interface (lines 10-20) to include the ref:

```typescript
interface DockviewAppData {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onOpenWorkspace: () => void;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  dockviewApiRef: React.MutableRefObject<DockviewApi | null>;
}
```

- [ ] **Step 3: Rewrite the `DockviewApp` function body**

Replace the existing function body (lines 79-129) with the version below. Key behaviors required by the spec:

- `initializedRef` is keyed to the `event.api` instance, not a boolean — Dockview may swap the api across remounts, and we want to re-initialize if it does.
- `restoringRef` suppresses `onDidLayoutChange` events emitted *during* restore so we don't immediately re-save the layout we just loaded.
- Saved blobs are wrapped as `{ version: LAYOUT_VERSION, layout }`. On load, an unknown `version` (or a parse / `fromJSON` failure) falls through to `buildDefaultLayout`.
- `useEffect` cleanup flushes a pending debounce save synchronously on unmount so a close-then-quit within 500 ms does not lose the last change.

```typescript
function DockviewApp(props: DockviewAppData) {
  const { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession, dockviewApiRef } = props;

  const ctxValue: DockviewAppData = { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession, dockviewApiRef };

  const initializedRef = useRef<DockviewApi | null>(null);
  const restoringRef = useRef(false);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const apiRef = useRef<DockviewApi | null>(null);

  useEffect(() => {
    return () => {
      if (saveTimerRef.current) {
        clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
        const api = apiRef.current;
        if (api) {
          window.orkworks.saveLayout(
            JSON.stringify({ version: LAYOUT_VERSION, layout: api.toJSON() }),
          );
        }
      }
    };
  }, []);

  function scheduleSave(api: DockviewApi) {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      saveTimerRef.current = null;
      window.orkworks.saveLayout(
        JSON.stringify({ version: LAYOUT_VERSION, layout: api.toJSON() }),
      );
    }, 500);
  }

  return (
    <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
      <DockviewContext.Provider value={ctxValue}>
        <DockviewReact
          components={COMPONENTS}
          className="orkworks-dockview"
          onReady={(event: DockviewReadyEvent) => {
            const api = event.api;
            if (initializedRef.current === api) return;
            initializedRef.current = api;
            apiRef.current = api;
            dockviewApiRef.current = api;

            restoringRef.current = true;

            window.orkworks.getLayout().then((raw) => {
              let restored = false;
              if (raw) {
                try {
                  const wrapper = JSON.parse(raw) as { version?: number; layout?: unknown };
                  if (wrapper && wrapper.version === LAYOUT_VERSION && wrapper.layout) {
                    api.fromJSON(wrapper.layout as Parameters<DockviewApi["fromJSON"]>[0]);
                    restored = true;
                  } else {
                    console.warn("[DockviewApp] saved layout missing or wrong version, using default");
                  }
                } catch (e) {
                  console.warn("[DockviewApp] failed to restore layout, using default", e);
                }
              }
              if (!restored) buildDefaultLayout(api);
              queueMicrotask(() => {
                restoringRef.current = false;
              });
            });

            api.onDidLayoutChange(() => {
              if (restoringRef.current) return;
              scheduleSave(api);
            });
          }}
        />
      </DockviewContext.Provider>
    </div>
  );
}
```

- [ ] **Step 4: Verify with TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors (ViewMenu import in App.tsx may still fail until Task 6).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/DockviewApp.tsx
git commit -m "feat: add layout persist/restore and PANEL_DEFAULTS to DockviewApp"
```

---

### Task 6: Create `ViewMenu.tsx` component

**Files:**
- Create: `apps/desktop/src/components/ViewMenu.tsx`

- [ ] **Step 1: Write the component**

Notes on the spec-mandated behaviors below:
- `PANEL_DEFAULTS` is iterated as an array (the ordering invariant is `sessions` first).
- Toggling a closed panel: prefer the panel's documented `referencePanel`; if that panel is also closed, fall back to anchoring on `sessions`; if `sessions` is also absent, omit `position` so Dockview places the panel in a new root group.
- Reset Layout delegates to the shared `clearAllPanels` + `buildDefaultLayout` helpers exported from `DockviewApp.tsx`, so the ordering invariant and the `api.clear()` fallback live in exactly one place.

```typescript
import { useEffect, useRef, useState, type MutableRefObject } from "react";
import type { DockviewApi } from "dockview-react";
import { PANEL_DEFAULTS, buildDefaultLayout, clearAllPanels } from "./DockviewApp";

interface ViewMenuProps {
  dockviewApiRef: MutableRefObject<DockviewApi | null>;
}

export default function ViewMenu({ dockviewApiRef }: ViewMenuProps) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [open]);

  const api = dockviewApiRef.current;

  function togglePanel(id: string) {
    if (!api) return;
    const panel = api.getPanel(id);
    if (panel) {
      panel.api.close();
    } else {
      const def = PANEL_DEFAULTS.find((d) => d.id === id);
      if (!def) return;
      const options: {
        id: string;
        component: string;
        title: string;
        position?: { referencePanel: string; direction: "below" | "right" };
      } = {
        id: def.id,
        component: def.component,
        title: def.title,
      };
      if (def.position) {
        if (api.getPanel(def.position.referencePanel)) {
          options.position = def.position;
        } else if (def.id !== "sessions" && api.getPanel("sessions")) {
          options.position = { referencePanel: "sessions", direction: "below" };
        }
        // else: omit position — Dockview places the panel in a new root group
      }
      api.addPanel(options);
    }
    setOpen(false);
  }

  function resetLayout() {
    if (!api) return;
    clearAllPanels(api);
    buildDefaultLayout(api);
    setOpen(false);
  }

  return (
    <div className="view-menu" ref={menuRef}>
      <button
        className="view-menu-trigger"
        onClick={() => setOpen(!open)}
      >
        View ▾
      </button>
      {open && (
        <ul className="view-menu-dropdown">
          {PANEL_DEFAULTS.map((def) => {
            const visible = api ? api.getPanel(def.id) != null : false;
            return (
              <li
                key={def.id}
                className={visible ? "view-menu-item checked" : "view-menu-item"}
                onClick={() => togglePanel(def.id)}
              >
                {def.title}
              </li>
            );
          })}
          <li className="view-menu-separator"><hr /></li>
          <li className="view-menu-item view-menu-reset" onClick={resetLayout}>
            Reset Layout
          </li>
        </ul>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify with TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/ViewMenu.tsx
git commit -m "feat: add ViewMenu component for panel toggles and layout reset"
```

---

### Task 7: Add CSS styling for ViewMenu and titlebar

**Files:**
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Add `.titlebar-right` and ViewMenu styles**

After the existing `.status-badge.warn` block (after line 56 or wherever `.status-badge.ok` ends), append:

```css
.titlebar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.view-menu {
  position: relative;
}

.view-menu-trigger {
  font-size: 12px;
  padding: 1px 8px;
  border-radius: 4px;
  background: transparent;
  color: #cccccc;
  border: 1px solid transparent;
  cursor: pointer;
  -webkit-app-region: no-drag;
}

.view-menu-trigger:hover {
  background: #3c3c3c;
  border-color: #555;
}

.view-menu-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 4px;
  min-width: 180px;
  background: #252526;
  border: 1px solid #3c3c3c;
  border-radius: 6px;
  padding: 4px 0;
  list-style: none;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
}

.view-menu-item {
  padding: 6px 12px;
  font-size: 12px;
  color: #cccccc;
  cursor: pointer;
}

.view-menu-item:hover {
  background: #094771;
}

.view-menu-item.checked::before {
  content: "✓ ";
  color: #cccccc;
}

.view-menu-separator {
  padding: 0;
  margin: 4px 0;
}

.view-menu-separator hr {
  border: none;
  border-top: 1px solid #3c3c3c;
  margin: 0;
}

.view-menu-reset {
  color: #9cdcfe;
}
```

- [ ] **Step 2: Verify the build compiles**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/App.css
git commit -m "feat: add ViewMenu and titlebar-right styles"
```

---

### Task 8: Verification — TypeScript check and test

**Files:**
- None (verification only)

- [ ] **Step 1: Full TypeScript type-check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: zero errors.

- [ ] **Step 2: Run existing tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all existing tests pass.

- [ ] **Step 3: Dev build**

```bash
cd apps/desktop && pnpm build
```

Expected: build succeeds.

- [ ] **Step 4: Manual smoke test in Electron (required for UI changes)**

CLAUDE.md requires exercising UI changes in the app before declaring done. Launch the dev app and verify each path from the design's Data Flow section:

```bash
cd apps/desktop && pnpm dev
```

Verify, in order:

1. **First launch** — no `layout.json` exists. Default 5-panel layout appears. ViewMenu shows all five entries checked.
2. **Persist drag** — drag a panel tab to a new position; quit; relaunch. The custom arrangement restores.
3. **Toggle close** — open ViewMenu, click "Recommendations". Panel closes, ViewMenu checkmark clears.
4. **Toggle re-open with reference present** — re-open ViewMenu, click "Recommendations". Panel reappears next to Capacity.
5. **Toggle re-open with reference missing** — close both Capacity and Recommendations, then re-open Recommendations from ViewMenu. Panel anchors against Sessions (fallback). Then close Sessions too and re-open Recommendations — panel appears in a new root group (no `position`).
6. **Reset Layout** — drag, close, and reorder until the layout is non-default. Click "Reset Layout". All five panels return to the default arrangement.
7. **Crash-safe persistence** — quit while a debounced save is pending (close a panel and immediately quit). Relaunch. The close is preserved (unmount flush worked).
8. **Corrupt-file fallback** — quit, manually corrupt `layout.json` in the userData directory (e.g. write `not json`), relaunch. Default layout appears; console shows the corruption warning; a fresh `layout.json` is written on the next layout change.
9. **Version-mismatch fallback** — manually edit `layout.json` to `{"version":999,"layout":{}}`, relaunch. Default layout appears; console shows the version-mismatch warning.

If any step fails, fix before proceeding.

- [ ] **Step 5: Commit any remaining changes**

```bash
git status
```

If clean, no commit needed. Otherwise stage and commit.

---

### Task 9: Update architecture docs

**Files:**
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Update the preload bridge section**

In `docs/agents/architecture.md`, update the preload section (lines 17-18) to include the new methods:

```text
`electron/preload.ts`, which exposes `window.orkworks` with `getBackendUrl()`, `getInitialWorkspace()`, `openWorkspace()`, `getLayout()`, and `saveLayout()`.
```

- [ ] **Step 2: Update the panel layout section**

In the Dockview panel layout section (lines 39-43), add a note about the ViewMenu, layout persistence, and the `PANEL_DEFAULTS` ordering invariant:

```text
A `ViewMenu` component in the titlebar (rendered as a sibling of the Dockview root, so it stays reachable even if the saved layout is corrupt) provides per-panel visibility toggles and a "Reset Layout" action. Panel layouts persist to Electron userData via `layout.json` and restore on startup via Dockview's `toJSON()`/`fromJSON()` serialization, wrapped as `{ version, layout }` so future shape changes can be invalidated by bumping `LAYOUT_VERSION`. The default panel ordering is defined as an array (`PANEL_DEFAULTS`) shared between startup, Reset Layout, and ViewMenu toggles; `sessions` must be added first as the root anchor.
```

- [ ] **Step 3: Add `electron/layoutMemory.ts` to the main-process module list**

The architecture doc tracks new preload-exposed API surfaces. In the section that lists main-process modules alongside `electron/workspaceMemory.ts`, add an entry for `electron/layoutMemory.ts` describing its responsibility (read/write `layout.json` from Electron userData with a serialized write queue and atomic rename).

- [ ] **Step 4: Commit**

```bash
git add docs/agents/architecture.md
git commit -m "docs: update architecture for panel persist/restore and ViewMenu"
```

---

### Task 10: Run doc-check script

**Files:**
- None (hook script)

- [ ] **Step 1: Run the doc check hook**

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no flagged files that still need updating. If any are flagged, address them.

- [ ] **Step 2: Final commit if any doc changes**

```bash
git status
```
