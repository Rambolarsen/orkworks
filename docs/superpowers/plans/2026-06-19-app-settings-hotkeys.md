# App Settings Hotkeys Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add persisted app-level settings for configurable Electron menu hotkeys, with an in-app Hotkeys settings modal.

**Architecture:** The Electron main process owns settings persistence, validation, and active menu accelerators. The renderer only displays settings, captures proposed chords, and submits hotkey changes through the preload bridge. Menu construction is extracted into a testable Electron-side template module so saved accelerators drive the active app menu without duplicating shortcut behavior in renderer-only handlers.

**Tech Stack:** Electron main/preload, React 19, TypeScript, Node built-in test runner, Electron `userData` persistence.

---

## Preconditions

- Issue: `#25 App settings object and configurable hotkeys` exists and is open.
- Authoritative scope is present in `specs/orkworks-mvp.md` under `App Settings and Hotkeys`.
- Design doc: `docs/superpowers/specs/2026-06-18-app-settings-hotkeys-design.md`.
- No backend (`crates/orkworksd`) or `.orkworks/` protocol changes are part of this plan.

## File Structure

- Create `docs/adr/0013-main-process-owned-app-settings.md`: records the settings ownership and menu-source-of-truth decision.
- Modify `docs/adr/README.md`: adds ADR 0013 to the index.
- Create `apps/desktop/electron/settingsMemory.ts`: defines `AppSettings`, hotkey defaults, normalization, validation, and `settings.json` read/write.
- Create `apps/desktop/electron/menuTemplate.ts`: builds the Electron menu template from `AppSettings` and command handlers.
- Modify `apps/desktop/electron/main.ts`: loads settings, builds menu from settings, adds settings IPC, suppresses OrkWorks menu commands during capture.
- Modify `apps/desktop/electron/preload.ts`: exposes `getSettings`, `saveHotkeys`, and `setHotkeyCaptureActive`.
- Create `apps/desktop/src/appSettingsTypes.ts`: shares renderer-side settings and save-result types.
- Create `apps/desktop/src/hotkeyCapture.ts`: converts renderer keyboard events into Electron accelerator strings.
- Create `apps/desktop/src/components/SettingsModal.tsx`: renders the Hotkeys modal and edit/reset/default flows.
- Modify `apps/desktop/src/App.tsx`: adds settings state, titlebar entry point, modal wiring, and `window.orkworks` types.
- Modify `apps/desktop/src/App.css`: styles the settings button, modal, hotkey rows, capture state, and validation messages.
- Create tests:
  - `apps/desktop/tests/electronSettingsMemory.test.ts`
  - `apps/desktop/tests/electronMenuTemplate.test.ts`
  - `apps/desktop/tests/hotkeyCapture.test.ts`
  - extend `apps/desktop/tests/dockview.test.ts` for preload and modal source wiring.

---

### Task 1: Record the Architecture Decision

**Files:**
- Create: `docs/adr/0013-main-process-owned-app-settings.md`
- Modify: `docs/adr/README.md`
- Test: documentation inspection

- [ ] **Step 1: Create ADR 0013**

Add `docs/adr/0013-main-process-owned-app-settings.md` with this content:

```markdown
# Main-process-owned app settings and menu accelerators

- Status: accepted
- Deciders: user
- Date: 2026-06-19

## Context

OrkWorks already persists app-owned desktop state in Electron `userData` for workspace memory and Dockview layout. Menu accelerators are still hard-coded in the Electron main process, so they cannot be customized without code changes.

The first settings slice is configurable hotkeys for existing menu actions. Active shortcuts must remain native Electron menu accelerators rather than renderer-only keyboard handlers, because the menu is the current source of command dispatch for these actions.

## Decision

Persist a versioned `AppSettings` document in Electron `userData` as `settings.json`. The Electron main process owns loading, default merging, validation, writing, and applying app settings.

The renderer accesses settings only through narrow preload APIs:

- `getSettings()`
- `saveHotkeys(hotkeys)`
- `setHotkeyCaptureActive(active)`

The Electron application menu is built from canonical settings. Hotkey saves validate and build the next menu before writing settings, then write settings and apply the new menu as one observable operation.

## Consequences

- **Easier**: App settings have a clear owner and future settings sections can share the same persistence boundary.
- **Easier**: Native Electron menu accelerators remain the source of truth for active shortcuts.
- **Easier**: Renderer code can focus on presentation and proposed edits instead of privileged filesystem or menu state.
- **Harder**: Main-process settings changes need IPC and menu regression tests.
- **Harder**: Hotkey capture must suppress existing accelerators while recording a replacement chord.
```

- [ ] **Step 2: Update ADR index**

Add this row to `docs/adr/README.md` after ADR 0012:

```markdown
| [0013](./0013-main-process-owned-app-settings.md) | Main-process-owned app settings and menu accelerators | accepted |
```

- [ ] **Step 3: Verify ADR files**

Run:

```bash
rg -n "0013|Main-process-owned app settings" docs/adr
```

Expected: output includes the ADR file title and the README index row.

- [ ] **Step 4: Commit**

```bash
git add docs/adr/0013-main-process-owned-app-settings.md docs/adr/README.md
git commit -m "docs: record app settings ownership decision"
```

---

### Task 2: Add Settings Persistence and Validation

**Files:**
- Create: `apps/desktop/electron/settingsMemory.ts`
- Create: `apps/desktop/tests/electronSettingsMemory.test.ts`
- Test: `apps/desktop/tests/electronSettingsMemory.test.ts`

- [ ] **Step 1: Write failing persistence and validation tests**

Create `apps/desktop/tests/electronSettingsMemory.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  DEFAULT_HOTKEYS,
  readSettings,
  settingsPath,
  validateHotkeys,
  writeSettings,
} from "../electron/settingsMemory.ts";

test("settings memory returns defaults when settings.json is missing", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const settings = readSettings(dir);
    assert.equal(settings.version, 1);
    assert.deepEqual(settings.hotkeys, DEFAULT_HOTKEYS);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory falls back to defaults when settings.json is corrupt", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(settingsPath(dir), "{not json");
    const settings = readSettings(dir);
    assert.deepEqual(settings.hotkeys, DEFAULT_HOTKEYS);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory merges partial persisted hotkeys with defaults", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({ version: 1, hotkeys: { newSession: "CmdOrCtrl+Alt+N" } }),
    );

    const settings = readSettings(dir);

    assert.equal(settings.hotkeys.newSession, "CmdOrCtrl+Alt+N");
    assert.equal(settings.hotkeys.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel);
    assert.equal(settings.hotkeys.resetLayout, null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory writes canonical settings JSON", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeSettings(dir, {
      version: 1,
      hotkeys: {
        ...DEFAULT_HOTKEYS,
        newSession: "CmdOrCtrl+Alt+N",
      },
    });

    const raw = readFileSync(settingsPath(dir), "utf8");
    assert.equal(raw.endsWith("\n"), true);
    assert.equal(JSON.parse(raw).hotkeys.newSession, "CmdOrCtrl+Alt+N");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("validateHotkeys rejects duplicates", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    toggleSessionsPanel: DEFAULT_HOTKEYS.newSession,
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleSessionsPanel, ["Duplicate shortcut also used by New Session."]);
});

test("validateHotkeys rejects invalid syntax and required empty values", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    newSession: "",
    toggleDetailPanel: "CmdOrCtrl+",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.newSession, ["Shortcut is required."]);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut must include a non-modifier key."]);
});

test("validateHotkeys allows optional resetLayout to be unset", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    resetLayout: null,
  });

  assert.deepEqual(result, { ok: true, errors: {} });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts
```

Expected: FAIL with an import error for `../electron/settingsMemory.ts`.

- [ ] **Step 3: Implement settings memory**

Create `apps/desktop/electron/settingsMemory.ts`:

```ts
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface AppSettings {
  version: 1;
  hotkeys: HotkeySettings;
}

export interface HotkeySettings {
  newSession: string;
  toggleSessionsPanel: string;
  toggleDetailPanel: string;
  toggleTerminalPanel: string;
  toggleCapacityPanel: string;
  toggleRecommendationsPanel: string;
  resetLayout: string | null;
}

export type HotkeyAction = keyof HotkeySettings;

export interface HotkeyDefinition {
  action: HotkeyAction;
  label: string;
  required: boolean;
  menuAction: "new-session" | "focus" | "reset-layout";
  panelId?: string;
}

export type HotkeyValidationErrors = Partial<Record<HotkeyAction, string[]>>;

export type HotkeyValidationResult =
  | { ok: true; errors: HotkeyValidationErrors }
  | { ok: false; errors: HotkeyValidationErrors };

export const HOTKEY_DEFINITIONS: HotkeyDefinition[] = [
  { action: "newSession", label: "New Session", required: true, menuAction: "new-session" },
  { action: "toggleSessionsPanel", label: "Sessions Panel", required: true, menuAction: "focus", panelId: "sessions" },
  { action: "toggleDetailPanel", label: "Detail Panel", required: true, menuAction: "focus", panelId: "detail" },
  { action: "toggleTerminalPanel", label: "Terminal Panel", required: true, menuAction: "focus", panelId: "terminal" },
  { action: "toggleCapacityPanel", label: "Capacity Panel", required: true, menuAction: "focus", panelId: "capacity" },
  { action: "toggleRecommendationsPanel", label: "Recommendations Panel", required: true, menuAction: "focus", panelId: "recommendations" },
  { action: "resetLayout", label: "Reset Layout", required: false, menuAction: "reset-layout" },
];

export const DEFAULT_HOTKEYS: HotkeySettings = {
  newSession: "CmdOrCtrl+N",
  toggleSessionsPanel: "CmdOrCtrl+Shift+S",
  toggleDetailPanel: "CmdOrCtrl+Shift+D",
  toggleTerminalPanel: "CmdOrCtrl+Shift+T",
  toggleCapacityPanel: "CmdOrCtrl+Shift+C",
  toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
  resetLayout: null,
};

export const DEFAULT_SETTINGS: AppSettings = {
  version: 1,
  hotkeys: DEFAULT_HOTKEYS,
};

const fileName = "settings.json";
const modifierNames = new Set(["Command", "Cmd", "Control", "Ctrl", "CommandOrControl", "CmdOrCtrl", "Alt", "Option", "AltGr", "Shift", "Super", "Meta"]);
const namedKeys = new Set([
  "Plus", "Space", "Tab", "Capslock", "Numlock", "Scrolllock", "Backspace", "Delete", "Insert", "Return", "Enter", "Up", "Down", "Left", "Right",
  "Home", "End", "PageUp", "PageDown", "Escape", "Esc", "VolumeUp", "VolumeDown", "VolumeMute", "MediaNextTrack", "MediaPreviousTrack",
  "MediaStop", "MediaPlayPause", "PrintScreen",
]);

export function settingsPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function normalizeSettings(value: unknown): AppSettings {
  if (!value || typeof value !== "object") {
    return DEFAULT_SETTINGS;
  }
  const parsed = value as Partial<AppSettings>;
  return {
    version: 1,
    hotkeys: normalizeHotkeys(parsed.hotkeys),
  };
}

export function normalizeHotkeys(value: unknown): HotkeySettings {
  const source = value && typeof value === "object" ? value as Partial<HotkeySettings> : {};
  return {
    newSession: stringOrDefault(source.newSession, DEFAULT_HOTKEYS.newSession),
    toggleSessionsPanel: stringOrDefault(source.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel),
    toggleDetailPanel: stringOrDefault(source.toggleDetailPanel, DEFAULT_HOTKEYS.toggleDetailPanel),
    toggleTerminalPanel: stringOrDefault(source.toggleTerminalPanel, DEFAULT_HOTKEYS.toggleTerminalPanel),
    toggleCapacityPanel: stringOrDefault(source.toggleCapacityPanel, DEFAULT_HOTKEYS.toggleCapacityPanel),
    toggleRecommendationsPanel: stringOrDefault(source.toggleRecommendationsPanel, DEFAULT_HOTKEYS.toggleRecommendationsPanel),
    resetLayout: typeof source.resetLayout === "string" && source.resetLayout.trim().length > 0 ? source.resetLayout : null,
  };
}

export function readSettings(userDataPath: string): AppSettings {
  const path = settingsPath(userDataPath);
  if (!existsSync(path)) {
    return DEFAULT_SETTINGS;
  }
  try {
    return normalizeSettings(JSON.parse(readFileSync(path, "utf8")));
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function writeSettings(userDataPath: string, settings: AppSettings): void {
  mkdirSync(userDataPath, { recursive: true });
  writeFileSync(settingsPath(userDataPath), `${JSON.stringify(normalizeSettings(settings), null, 2)}\n`);
}

export function validateHotkeys(hotkeys: HotkeySettings): HotkeyValidationResult {
  const errors: HotkeyValidationErrors = {};
  const seen = new Map<string, HotkeyDefinition>();

  for (const definition of HOTKEY_DEFINITIONS) {
    const value = hotkeys[definition.action];
    const trimmed = typeof value === "string" ? value.trim() : "";

    if (!trimmed) {
      if (definition.required) addError(errors, definition.action, "Shortcut is required.");
      continue;
    }

    const syntaxError = acceleratorSyntaxError(trimmed);
    if (syntaxError) {
      addError(errors, definition.action, syntaxError);
      continue;
    }

    const key = trimmed.toLowerCase();
    const duplicate = seen.get(key);
    if (duplicate) {
      addError(errors, definition.action, `Duplicate shortcut also used by ${duplicate.label}.`);
    } else {
      seen.set(key, definition);
    }
  }

  return Object.keys(errors).length === 0 ? { ok: true, errors } : { ok: false, errors };
}

function stringOrDefault(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim().length > 0 ? value : fallback;
}

function addError(errors: HotkeyValidationErrors, action: HotkeyAction, message: string): void {
  errors[action] = [...(errors[action] ?? []), message];
}

function acceleratorSyntaxError(accelerator: string): string | null {
  const parts = accelerator.split("+").map((part) => part.trim()).filter(Boolean);
  if (parts.length === 0) return "Shortcut is required.";

  const keyParts = parts.filter((part) => !modifierNames.has(part));
  if (keyParts.length === 0) return "Shortcut must include a non-modifier key.";
  if (keyParts.length > 1) return "Shortcut must contain only one non-modifier key.";

  return isSupportedKey(keyParts[0]) ? null : `Unsupported key "${keyParts[0]}".`;
}

function isSupportedKey(key: string): boolean {
  if (/^[A-Z0-9]$/.test(key)) return true;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return true;
  return namedKeys.has(key);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts
```

Expected: PASS for all tests in `electronSettingsMemory.test.ts`.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/electron/settingsMemory.ts apps/desktop/tests/electronSettingsMemory.test.ts
git commit -m "feat: add app settings persistence"
```

---

### Task 3: Extract Menu Template and Wire Saved Hotkeys in Main

**Files:**
- Create: `apps/desktop/electron/menuTemplate.ts`
- Create: `apps/desktop/tests/electronMenuTemplate.test.ts`
- Modify: `apps/desktop/electron/main.ts`
- Test: `apps/desktop/tests/electronMenuTemplate.test.ts`

- [ ] **Step 1: Write failing menu template tests**

Create `apps/desktop/tests/electronMenuTemplate.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";

import { DEFAULT_SETTINGS } from "../electron/settingsMemory.ts";
import { buildMenuTemplate, findMenuItem } from "../electron/menuTemplate.ts";

test("menu template uses accelerators from settings", () => {
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "darwin",
    settings: {
      version: 1,
      hotkeys: {
        ...DEFAULT_SETTINGS.hotkeys,
        newSession: "CmdOrCtrl+Alt+N",
        toggleTerminalPanel: "CmdOrCtrl+Alt+T",
        resetLayout: "CmdOrCtrl+Alt+Backspace",
      },
    },
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "new-session")?.accelerator, "CmdOrCtrl+Alt+N");
  assert.equal(findMenuItem(template, "terminal")?.accelerator, "CmdOrCtrl+Alt+T");
  assert.equal(findMenuItem(template, "reset-layout")?.accelerator, "CmdOrCtrl+Alt+Backspace");
});

test("menu template omits optional reset layout accelerator when unset", () => {
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "darwin",
    settings: DEFAULT_SETTINGS,
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "reset-layout")?.accelerator, undefined);
});

test("menu command handlers are suppressible during hotkey capture", () => {
  const commands: Array<{ action: string; panelId?: string }> = [];
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "linux",
    settings: DEFAULT_SETTINGS,
    isHotkeyCaptureActive: () => true,
    sendCommand: (command) => commands.push(command),
  });

  findMenuItem(template, "new-session")?.click?.({} as never, {} as never, {} as never);
  findMenuItem(template, "sessions")?.click?.({} as never, {} as never, {} as never);

  assert.deepEqual(commands, []);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronMenuTemplate.test.ts
```

Expected: FAIL with an import error for `../electron/menuTemplate.ts`.

- [ ] **Step 3: Implement menu template builder**

Create `apps/desktop/electron/menuTemplate.ts`:

```ts
import type { MenuItemConstructorOptions } from "electron";
import type { AppSettings } from "./settingsMemory";

export interface MenuCommand {
  action: "new-session" | "focus" | "reset-layout";
  panelId?: string;
}

export interface BuildMenuTemplateOptions {
  appName: string;
  platform: NodeJS.Platform;
  settings: AppSettings;
  sendCommand: (command: MenuCommand) => void;
  isHotkeyCaptureActive?: () => boolean;
}

export function buildMenuTemplate(options: BuildMenuTemplateOptions): MenuItemConstructorOptions[] {
  const panelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"] as const;
  const panelTitles: Record<(typeof panelIds)[number], string> = {
    sessions: "Sessions",
    detail: "Detail",
    terminal: "Terminal",
    capacity: "Capacity",
    recommendations: "Recommendations",
  };
  const panelAccelerators: Record<(typeof panelIds)[number], string> = {
    sessions: options.settings.hotkeys.toggleSessionsPanel,
    detail: options.settings.hotkeys.toggleDetailPanel,
    terminal: options.settings.hotkeys.toggleTerminalPanel,
    capacity: options.settings.hotkeys.toggleCapacityPanel,
    recommendations: options.settings.hotkeys.toggleRecommendationsPanel,
  };

  const sendIfNotCapturing = (command: MenuCommand) => {
    if (options.isHotkeyCaptureActive?.()) return;
    options.sendCommand(command);
  };

  const panelItems: MenuItemConstructorOptions[] = panelIds.map((id) => ({
    id,
    label: panelTitles[id],
    accelerator: panelAccelerators[id],
    type: "checkbox",
    checked: true,
    click: () => sendIfNotCapturing({ action: "focus", panelId: id }),
  }));

  const resetLayoutItem: MenuItemConstructorOptions = {
    id: "reset-layout",
    label: "Reset Layout",
    click: () => sendIfNotCapturing({ action: "reset-layout" }),
  };
  if (options.settings.hotkeys.resetLayout) {
    resetLayoutItem.accelerator = options.settings.hotkeys.resetLayout;
  }

  return [
    {
      label: options.appName,
      submenu: [
        { role: "about" },
        { type: "separator" },
        { role: "services" },
        { type: "separator" },
        { role: "hide" },
        { role: "hideOthers" },
        { role: "unhide" },
        { type: "separator" },
        { role: "quit" },
      ],
    },
    {
      label: "File",
      submenu: [
        {
          id: "new-session",
          label: "New Session",
          accelerator: options.settings.hotkeys.newSession,
          click: () => sendIfNotCapturing({ action: "new-session" }),
        },
        { type: "separator" },
        { role: "close" },
      ],
    },
    {
      label: "Edit",
      submenu: [
        { role: "undo" },
        { role: "redo" },
        { type: "separator" },
        { role: "cut" },
        { role: "copy" },
        { role: "paste" },
        { role: "selectAll" },
      ],
    },
    {
      label: "View",
      submenu: [
        ...panelItems,
        { type: "separator" },
        resetLayoutItem,
        { type: "separator" },
        { role: "reload" },
        { role: "forceReload" },
        { role: "toggleDevTools" },
        { type: "separator" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
        { type: "separator" },
        { role: "togglefullscreen" },
      ],
    },
    {
      label: "Window",
      submenu: [
        { role: "minimize" },
        { role: "zoom" },
        ...(options.platform === "darwin"
          ? [{ type: "separator" as const }, { role: "front" as const }]
          : [{ role: "close" as const }]),
      ],
    },
    {
      role: "help",
      submenu: [
        {
          label: "Learn More",
          click: () => {},
        },
      ],
    },
  ];
}

export function findMenuItem(template: MenuItemConstructorOptions[], id: string): MenuItemConstructorOptions | undefined {
  for (const item of template) {
    if (item.id === id) return item;
    if (Array.isArray(item.submenu)) {
      const found = findMenuItem(item.submenu, id);
      if (found) return found;
    }
  }
  return undefined;
}
```

- [ ] **Step 4: Run menu template tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronMenuTemplate.test.ts
```

Expected: PASS.

- [ ] **Step 5: Modify main process to use settings and menu template**

In `apps/desktop/electron/main.ts`:

Add imports:

```ts
import type { AppSettings, HotkeySettings } from "./settingsMemory";
import { readSettings, validateHotkeys, writeSettings } from "./settingsMemory";
import { buildMenuTemplate } from "./menuTemplate";
```

Add state near the existing globals:

```ts
let currentSettings: AppSettings | null = null;
let hotkeyCaptureActive = false;
```

Replace the existing `buildMenu(): void` function with:

```ts
function buildMenu(settings: AppSettings): void {
  const template = buildMenuTemplate({
    appName: app.name,
    platform: process.platform,
    settings,
    isHotkeyCaptureActive: () => hotkeyCaptureActive,
    sendCommand: (command) => {
      mainWindow?.webContents.send("orkworks:menu-command", command);
    },
  });
  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);

  menuPanelItems = {};
  for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
    const item = menu.getMenuItemById(id);
    if (item) menuPanelItems[id] = item;
  }
}
```

Inside `app.whenReady().then(() => {`, after `initialWorkspacePath`, load settings:

```ts
  currentSettings = readSettings(app.getPath("userData"));
```

Add IPC handlers before `open-workspace`:

```ts
  ipcMain.handle("get-settings", async () => {
    currentSettings = readSettings(app.getPath("userData"));
    return currentSettings;
  });

  ipcMain.handle("save-hotkeys", async (_event, hotkeys: HotkeySettings) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const nextSettings: AppSettings = {
      ...baseSettings,
      version: 1,
      hotkeys,
    };

    const validation = validateHotkeys(nextSettings.hotkeys);
    if (!validation.ok) {
      return { ok: false, errors: validation.errors };
    }

    const template = buildMenuTemplate({
      appName: app.name,
      platform: process.platform,
      settings: nextSettings,
      isHotkeyCaptureActive: () => hotkeyCaptureActive,
      sendCommand: (command) => {
        mainWindow?.webContents.send("orkworks:menu-command", command);
      },
    });
    const menu = Menu.buildFromTemplate(template);

    writeSettings(app.getPath("userData"), nextSettings);
    Menu.setApplicationMenu(menu);
    currentSettings = nextSettings;

    menuPanelItems = {};
    for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
      const item = menu.getMenuItemById(id);
      if (item) menuPanelItems[id] = item;
    }

    return { ok: true, settings: nextSettings };
  });

  ipcMain.on("orkworks:hotkey-capture-active", (_event, active: boolean) => {
    hotkeyCaptureActive = active;
  });
```

Replace the startup `buildMenu();` call with:

```ts
  buildMenu(currentSettings);
```

- [ ] **Step 6: Typecheck Electron main changes**

Run:

```bash
cd apps/desktop && npx tsc -p tsconfig.node.json --noEmit
```

Expected: PASS.

- [ ] **Step 7: Run focused tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts tests/electronMenuTemplate.test.ts
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/electron/main.ts apps/desktop/electron/menuTemplate.ts apps/desktop/tests/electronMenuTemplate.test.ts
git commit -m "feat: build menu from app settings"
```

---

### Task 4: Expose Settings Through the Preload Bridge

**Files:**
- Modify: `apps/desktop/electron/preload.ts`
- Create: `apps/desktop/src/appSettingsTypes.ts`
- Modify: `apps/desktop/src/App.tsx`
- Test: source inspection in `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Add failing preload source test**

Append to `apps/desktop/tests/dockview.test.ts`:

```ts
test("preload exposes settings and hotkey capture APIs", () => {
  const source = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");

  assert.match(source, /getSettings:\s*\(\)/);
  assert.match(source, /ipcRenderer\.invoke\("get-settings"\)/);
  assert.match(source, /saveHotkeys:\s*\(hotkeys:/);
  assert.match(source, /ipcRenderer\.invoke\("save-hotkeys", hotkeys\)/);
  assert.match(source, /setHotkeyCaptureActive:\s*\(active:/);
  assert.match(source, /ipcRenderer\.send\("orkworks:hotkey-capture-active", active\)/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: FAIL because preload does not expose settings APIs yet.

- [ ] **Step 3: Update preload**

In `apps/desktop/electron/preload.ts`, add the new bridge methods inside `contextBridge.exposeInMainWorld("orkworks", { ... })`:

```ts
  getSettings: (): Promise<unknown> => ipcRenderer.invoke("get-settings"),
  saveHotkeys: (hotkeys: unknown): Promise<unknown> => ipcRenderer.invoke("save-hotkeys", hotkeys),
  setHotkeyCaptureActive: (active: boolean) => {
    ipcRenderer.send("orkworks:hotkey-capture-active", active);
  },
```

- [ ] **Step 4: Add shared renderer settings types**

Create `apps/desktop/src/appSettingsTypes.ts`:

```ts
export interface HotkeySettings {
  newSession: string;
  toggleSessionsPanel: string;
  toggleDetailPanel: string;
  toggleTerminalPanel: string;
  toggleCapacityPanel: string;
  toggleRecommendationsPanel: string;
  resetLayout: string | null;
}

export interface AppSettings {
  version: 1;
  hotkeys: HotkeySettings;
}

export type SaveHotkeysResult =
  | { ok: true; settings: AppSettings }
  | { ok: false; errors: Partial<Record<keyof HotkeySettings, string[]>> };
```

- [ ] **Step 5: Update renderer window typing**

In `apps/desktop/src/App.tsx`, add this import:

```ts
import type { AppSettings, HotkeySettings, SaveHotkeysResult } from "./appSettingsTypes";
```

Extend `Window["orkworks"]` with:

```ts
      getSettings: () => Promise<AppSettings>;
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      setHotkeyCaptureActive: (active: boolean) => void;
```

- [ ] **Step 6: Run preload source test and typecheck**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
cd apps/desktop && npx tsc --noEmit
```

Expected: both commands PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/electron/preload.ts apps/desktop/src/appSettingsTypes.ts apps/desktop/src/App.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat: expose app settings preload APIs"
```

---

### Task 5: Add Renderer Hotkey Capture Helpers

**Files:**
- Create: `apps/desktop/src/hotkeyCapture.ts`
- Create: `apps/desktop/tests/hotkeyCapture.test.ts`
- Test: `apps/desktop/tests/hotkeyCapture.test.ts`

- [ ] **Step 1: Write failing hotkey capture tests**

Create `apps/desktop/tests/hotkeyCapture.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";

import { acceleratorFromKeyboardEvent } from "../src/hotkeyCapture.ts";

function event(key: string, modifiers: Partial<Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "altKey" | "shiftKey">> = {}) {
  return {
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault() {},
    stopPropagation() {},
    ...modifiers,
  } as KeyboardEvent;
}

test("acceleratorFromKeyboardEvent normalizes command/control chords", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("n", { metaKey: true })), "CmdOrCtrl+N");
  assert.equal(acceleratorFromKeyboardEvent(event("T", { ctrlKey: true, shiftKey: true })), "CmdOrCtrl+Shift+T");
});

test("acceleratorFromKeyboardEvent ignores modifier-only keydown events", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("Shift", { shiftKey: true })), null);
  assert.equal(acceleratorFromKeyboardEvent(event("Control", { ctrlKey: true })), null);
});

test("acceleratorFromKeyboardEvent supports named keys", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("Backspace", { metaKey: true, altKey: true })), "CmdOrCtrl+Alt+Backspace");
  assert.equal(acceleratorFromKeyboardEvent(event(" ", { ctrlKey: true })), "CmdOrCtrl+Space");
  assert.equal(acceleratorFromKeyboardEvent(event("ArrowLeft", { ctrlKey: true })), "CmdOrCtrl+Left");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/hotkeyCapture.test.ts
```

Expected: FAIL with an import error for `../src/hotkeyCapture.ts`.

- [ ] **Step 3: Implement capture helper**

Create `apps/desktop/src/hotkeyCapture.ts`:

```ts
const modifierKeys = new Set(["Meta", "Control", "Alt", "Shift"]);

const keyNameMap: Record<string, string> = {
  " ": "Space",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Escape: "Esc",
};

export function acceleratorFromKeyboardEvent(event: KeyboardEvent): string | null {
  if (modifierKeys.has(event.key)) {
    return null;
  }

  const key = normalizeKey(event.key);
  if (!key) {
    return null;
  }

  const parts: string[] = [];
  if (event.metaKey || event.ctrlKey) parts.push("CmdOrCtrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey && key.length > 1) parts.push("Shift");
  if (event.shiftKey && key.length === 1 && key === key.toLowerCase()) parts.push("Shift");
  parts.push(key);

  return parts.join("+");
}

function normalizeKey(key: string): string | null {
  const mapped = keyNameMap[key] ?? key;
  if (mapped.length === 1) {
    return mapped.toUpperCase();
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(mapped)) {
    return mapped;
  }
  if (/^[A-Za-z]+$/.test(mapped)) {
    return mapped[0].toUpperCase() + mapped.slice(1);
  }
  return null;
}
```

- [ ] **Step 4: Run capture tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/hotkeyCapture.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/hotkeyCapture.ts apps/desktop/tests/hotkeyCapture.test.ts
git commit -m "feat: add hotkey capture normalization"
```

---

### Task 6: Build the Settings Modal UI

**Files:**
- Create: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Test: source inspection and TypeScript

- [ ] **Step 1: Add failing renderer wiring tests**

Append to `apps/desktop/tests/dockview.test.ts`:

```ts
test("App exposes a settings titlebar entry and renders SettingsModal", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import SettingsModal from "\.\/components\/SettingsModal"/);
  assert.match(source, /setSettingsOpen\(true\)/);
  assert.match(source, /<SettingsModal/);
});

test("SettingsModal contains hotkey edit reset default cancel and save flows", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  for (const text of ["Hotkeys", "Edit", "Reset", "Restore defaults", "Cancel", "Save"]) {
    assert.match(source, new RegExp(text));
  }
  assert.match(source, /acceleratorFromKeyboardEvent/);
  assert.match(source, /setHotkeyCaptureActive\(true\)/);
  assert.match(source, /setHotkeyCaptureActive\(false\)/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: FAIL because `SettingsModal.tsx` and app wiring do not exist yet.

- [ ] **Step 3: Create SettingsModal component**

Create `apps/desktop/src/components/SettingsModal.tsx`:

```tsx
import { useEffect, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, HotkeySettings, SaveHotkeysResult } from "../appSettingsTypes";

type HotkeyAction = keyof HotkeySettings;

interface SettingsModalProps {
  initialSettings: AppSettings;
  onClose: () => void;
  onSaved: (settings: AppSettings) => void;
}

const defaultHotkeys: HotkeySettings = {
  newSession: "CmdOrCtrl+N",
  toggleSessionsPanel: "CmdOrCtrl+Shift+S",
  toggleDetailPanel: "CmdOrCtrl+Shift+D",
  toggleTerminalPanel: "CmdOrCtrl+Shift+T",
  toggleCapacityPanel: "CmdOrCtrl+Shift+C",
  toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
  resetLayout: null,
};

const hotkeyRows: Array<{ action: HotkeyAction; label: string; optional?: boolean }> = [
  { action: "newSession", label: "New Session" },
  { action: "toggleSessionsPanel", label: "Sessions Panel" },
  { action: "toggleDetailPanel", label: "Detail Panel" },
  { action: "toggleTerminalPanel", label: "Terminal Panel" },
  { action: "toggleCapacityPanel", label: "Capacity Panel" },
  { action: "toggleRecommendationsPanel", label: "Recommendations Panel" },
  { action: "resetLayout", label: "Reset Layout", optional: true },
];

export default function SettingsModal({ initialSettings, onClose, onSaved }: SettingsModalProps) {
  const [draft, setDraft] = useState<HotkeySettings>(initialSettings.hotkeys);
  const [capturing, setCapturing] = useState<HotkeyAction | null>(null);
  const [errors, setErrors] = useState<Partial<Record<HotkeyAction, string[]>>>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!capturing) return;

    window.orkworks.setHotkeyCaptureActive(true);
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setCapturing(null);
        return;
      }
      if (event.key === "Backspace" || event.key === "Delete") {
        const row = hotkeyRows.find((item) => item.action === capturing);
        if (row?.optional) {
          setDraft((current) => ({ ...current, [capturing]: null }));
          setCapturing(null);
        }
        return;
      }

      const accelerator = acceleratorFromKeyboardEvent(event);
      if (accelerator) {
        setDraft((current) => ({ ...current, [capturing]: accelerator }));
        setCapturing(null);
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.orkworks.setHotkeyCaptureActive(false);
    };
  }, [capturing]);

  async function save() {
    setSaving(true);
    setErrors({});
    const result: SaveHotkeysResult = await window.orkworks.saveHotkeys(draft);
    setSaving(false);
    if (result.ok) {
      onSaved(result.settings);
      onClose();
    } else {
      setErrors(result.errors);
    }
  }

  return (
    <div className="settings-backdrop" role="presentation">
      <section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <header className="settings-modal-header">
          <div>
            <h2 id="settings-title">Settings</h2>
            <p>Configure OrkWorks desktop preferences.</p>
          </div>
          <button className="settings-icon-button" type="button" onClick={onClose} aria-label="Close settings">×</button>
        </header>

        <div className="settings-section">
          <h3>Hotkeys</h3>
          <p className="settings-section-copy">Changes apply after Save and update the native Electron menu.</p>

          <div className="hotkey-list">
            {hotkeyRows.map((row) => (
              <div className={`hotkey-row ${capturing === row.action ? "hotkey-row--capturing" : ""}`} key={row.action}>
                <div>
                  <div className="hotkey-label">{row.label}</div>
                  {errors[row.action]?.map((error) => (
                    <div className="hotkey-error" key={error}>{error}</div>
                  ))}
                </div>
                <kbd className="hotkey-value">
                  {capturing === row.action ? "Press shortcut..." : draft[row.action] ?? "Unset"}
                </kbd>
                <button type="button" onClick={() => setCapturing(row.action)}>Edit</button>
                <button
                  type="button"
                  onClick={() => setDraft((current) => ({ ...current, [row.action]: defaultHotkeys[row.action] }))}
                >
                  Reset
                </button>
              </div>
            ))}
          </div>
        </div>

        <footer className="settings-modal-footer">
          <button type="button" onClick={() => setDraft(defaultHotkeys)}>Restore defaults</button>
          <span className="settings-footer-spacer" />
          <button type="button" onClick={onClose}>Cancel</button>
          <button type="button" className="settings-primary-button" disabled={saving} onClick={save}>
            {saving ? "Saving..." : "Save"}
          </button>
        </footer>
      </section>
    </div>
  );
}
```

- [ ] **Step 4: Wire App titlebar and settings state**

In `apps/desktop/src/App.tsx`:

Add imports:

```ts
import SettingsModal from "./components/SettingsModal";
```

Add state inside `App()`:

```ts
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
```

Add a settings loader:

```ts
  const openSettings = useCallback(async () => {
    const loaded = await window.orkworks.getSettings();
    setSettings(loaded);
    setSettingsOpen(true);
  }, []);
```

In the titlebar, replace the single status badge with a right-side group:

```tsx
        <div className="titlebar-right">
          <button
            className="titlebar-settings-button"
            type="button"
            onClick={openSettings}
            title="Settings"
          >
            Settings
          </button>
          <span
            className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
          >
            {backendStatus}
          </span>
        </div>
```

Before the closing `</div>` of `.app-shell`, render:

```tsx
      {settingsOpen && settings && (
        <SettingsModal
          initialSettings={settings}
          onClose={() => setSettingsOpen(false)}
          onSaved={(nextSettings) => setSettings(nextSettings)}
        />
      )}
```

- [ ] **Step 5: Add CSS**

Append to `apps/desktop/src/App.css`:

```css
.titlebar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
}

.titlebar-settings-button {
  border: 1px solid #4a4a4a;
  border-radius: 4px;
  padding: 2px 8px;
  color: #d4d4d4;
  background: #252526;
  font: inherit;
  font-size: 11px;
  cursor: pointer;
}

.titlebar-settings-button:hover {
  background: #3a3a3b;
}

.settings-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
}

.settings-modal {
  width: min(720px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  overflow: auto;
  border: 1px solid #484848;
  border-radius: 10px;
  background: #211f1e;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.45);
}

.settings-modal-header,
.settings-modal-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  border-color: #363331;
}

.settings-modal-header {
  justify-content: space-between;
  border-bottom: 1px solid #363331;
}

.settings-modal-header h2,
.settings-section h3 {
  margin: 0;
  color: #f0e7dc;
}

.settings-modal-header p,
.settings-section-copy {
  margin: 4px 0 0;
  color: #9b928a;
}

.settings-icon-button {
  border: 0;
  color: #d4d4d4;
  background: transparent;
  font-size: 22px;
  cursor: pointer;
}

.settings-section {
  padding: 16px;
}

.hotkey-list {
  margin-top: 14px;
  border: 1px solid #363331;
  border-radius: 8px;
  overflow: hidden;
}

.hotkey-row {
  display: grid;
  grid-template-columns: minmax(150px, 1fr) minmax(140px, auto) auto auto;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-bottom: 1px solid #363331;
}

.hotkey-row:last-child {
  border-bottom: 0;
}

.hotkey-row--capturing {
  background: #2d3428;
}

.hotkey-label {
  color: #e6dfd7;
  font-weight: 600;
}

.hotkey-error {
  margin-top: 3px;
  color: #ff8f7a;
  font-size: 11px;
}

.hotkey-value {
  justify-self: end;
  border: 1px solid #4b4743;
  border-radius: 5px;
  padding: 4px 7px;
  color: #f0e7dc;
  background: #151413;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  font-size: 11px;
}

.hotkey-row button,
.settings-modal-footer button {
  border: 1px solid #4b4743;
  border-radius: 4px;
  padding: 5px 9px;
  color: #e6dfd7;
  background: #2a2826;
  font: inherit;
  cursor: pointer;
}

.hotkey-row button:hover,
.settings-modal-footer button:hover:not(:disabled) {
  background: #383431;
}

.settings-modal-footer {
  border-top: 1px solid #363331;
}

.settings-footer-spacer {
  flex: 1;
}

.settings-primary-button {
  border-color: #5c879e !important;
  color: #d8edf7 !important;
  background: #123241 !important;
}

.settings-primary-button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
```

- [ ] **Step 6: Run focused renderer checks**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/hotkeyCapture.test.ts
cd apps/desktop && npx tsc --noEmit
```

Expected: both commands PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css apps/desktop/src/components/SettingsModal.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat: add hotkey settings modal"
```

---

### Task 7: Full Verification and Documentation Currency

**Files:**
- Modify only if checks identify required doc updates: `README.md`, `AGENTS.md`, or relevant specs.
- Test: full desktop checks and doc-check.

- [ ] **Step 1: Run all frontend tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all tests PASS.

- [ ] **Step 2: Run Electron and renderer type checks**

Run:

```bash
cd apps/desktop && npx tsc -p tsconfig.node.json --noEmit
cd apps/desktop && npx tsc --noEmit
```

Expected: both commands PASS.

- [ ] **Step 3: Run desktop build**

Run:

```bash
cd apps/desktop && pnpm build
```

Expected: build exits 0.

- [ ] **Step 4: Run doc currency hook**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no required documentation updates are listed. If the hook flags README or AGENTS because ADR 0013 was added, update their ADR/spec lists consistently and rerun this command.

- [ ] **Step 5: Manual verification**

Run the desktop app:

```bash
cd apps/desktop && pnpm dev
```

Verify:

- Settings opens from the titlebar.
- Editing `New Session` to `CmdOrCtrl+Alt+N` updates the File menu accelerator after Save.
- Capturing `CmdOrCtrl+N` records the chord and does not create a new session.
- Duplicate assignment shows an action-specific error and does not change the active menu.
- `Reset Layout` can be left unset and can be assigned a shortcut.
- Sessions panel shortcut preserves its current focus/restore/close behavior after customization.
- Restarting the app preserves saved hotkeys.

- [ ] **Step 6: Commit verification/doc updates**

If documentation files changed in this task:

```bash
git add README.md AGENTS.md specs/orkworks-mvp.md
git commit -m "docs: update app settings documentation"
```

If no documentation files changed, skip this commit.

---

## Self-Review Checklist

- Spec coverage:
  - Persisted `settings.json`: Task 2.
  - Versioned `AppSettings` with hotkeys only: Task 2.
  - Existing shortcut scope and shipped defaults: Task 2.
  - Menu accelerators built from settings: Task 3.
  - Preload read/save APIs: Task 4.
  - Settings modal with edit/reset/default/cancel/save: Task 6.
  - Main-process validation for invalid, duplicate, and required hotkeys: Task 2 and Task 3.
  - Capture suppression: Task 3, Task 4, Task 6.
  - Sessions panel behavior preservation: Task 3 keeps command identity, Task 6 changes only accelerators.
  - Restart persistence: Task 2 and Task 7 manual verification.
- No backend or `.orkworks/` protocol files are modified.
- No new hotkey actions are added.
- ADR prerequisite is first, before implementation code.
- Plan uses TDD checkpoints with failing-test, implementation, passing-test, and commit steps.
