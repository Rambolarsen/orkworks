import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme } from "electron";
import { spawn, type ChildProcess } from "child_process";
import { existsSync } from "fs";
import * as path from "path";
import { getDevRepoRoot, getDevSidecarPath, getPackagedSidecarPath } from "./paths";
import { readWorkspaceMemory, rememberWorkspacePath } from "./workspaceMemory";
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";
import type { AppSettings } from "./settingsMemory";
import { DEFAULT_HOTKEYS, DEFAULT_RETENTION, normalizeProviderSettings, normalizeRetention, readSettings, settingsWithHotkeys, validateHotkeys, writeSettings } from "./settingsMemory";
import { pushProviderSettings } from "./providerSettingsSync";
import type { ProviderSettings } from "./providerTypes";
import { buildMenuTemplate } from "./menuTemplate";

app.setName("OrkWorks");

let mainWindow: BrowserWindow | null = null;
let sidecarProcess: ChildProcess | null = null;
let backendPort: number | null = null;
let portResolve: ((port: number) => void) | null = null;
let portPromise = new Promise<number>((resolve) => {
  portResolve = resolve;
});

let workspacePath: string | null = null;
let menuPanelItems: Record<string, Electron.MenuItem> = {};
let currentSettings: AppSettings | null = null;
let providerModels: Map<string, string[]> = new Map();
let providerLabels: Record<string, string> = {};
let hotkeyCaptureActive = false;
const menuPanelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"];

function rendererSettings(settings: AppSettings): AppSettings & { defaultHotkeys: typeof DEFAULT_HOTKEYS } {
  return {
    ...settings,
    defaultHotkeys: { ...DEFAULT_HOTKEYS },
  };
}

function createMenu(settings: AppSettings): Electron.Menu {
  const template = buildMenuTemplate({
    appName: app.name,
    platform: process.platform,
    settings,
    isHotkeyCaptureActive: () => hotkeyCaptureActive,
    sendCommand: (command) => {
      mainWindow?.webContents.send("orkworks:menu-command", command);
    },
  });
  return Menu.buildFromTemplate(template);
}

function applyMenu(menu: Electron.Menu): void {
  const previousPanelChecked: Record<string, boolean> = {};
  for (const id of menuPanelIds) {
    const item = menuPanelItems[id];
    if (item) previousPanelChecked[id] = item.checked;
  }

  Menu.setApplicationMenu(menu);

  menuPanelItems = {};
  for (const id of menuPanelIds) {
    const item = menu.getMenuItemById(id);
    if (item) {
      if (id in previousPanelChecked) item.checked = previousPanelChecked[id];
      menuPanelItems[id] = item;
    }
  }
}

function getSidecarPath(): string {
  if (app.isPackaged) {
    return getPackagedSidecarPath(process.resourcesPath, process.platform);
  }
  return getDevSidecarPath(__dirname);
}

function startSidecar(cwdOverride?: string): void {
  const binaryPath = getSidecarPath();
  const sidecarCwd = cwdOverride ?? (app.isPackaged ? app.getPath("home") : getDevRepoRoot(__dirname));
  console.log(`[main] starting sidecar: ${binaryPath}`);
  console.log(`[main] sidecar cwd: ${sidecarCwd}`);

  sidecarProcess = spawn(binaryPath, [], {
    cwd: sidecarCwd,
    stdio: ["ignore", "pipe", "pipe"],
  });

  sidecarProcess.stdout?.on("data", (data: Buffer) => {
    const line = data.toString().trim();
    console.log(`[orkworksd] ${line}`);
    const match = line.match(/ORKWORKSD_PORT=(\d+)/);
    if (match) {
      backendPort = parseInt(match[1], 10);
      console.log(`[main] sidecar ready on port ${backendPort}`);
      if (portResolve) {
        portResolve(backendPort);
        portResolve = null;
      }
    }
  });

  sidecarProcess.stderr?.on("data", (data: Buffer) => {
    console.error(`[orkworksd:err] ${data.toString().trim()}`);
  });

  sidecarProcess.on("exit", (code) => {
    console.log(`[main] sidecar exited with code ${code}`);
    sidecarProcess = null;
  });
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 500,
    title: "OrkWorks",
    icon: path.join(__dirname, "../build", process.platform === "win32" ? "icon.ico" : "icon.png"),
    ...(process.platform === "darwin" && { titleBarStyle: "hiddenInset" as const }),
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, "preload.js"),
    },
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }

  mainWindow.on("closed", () => {
    mainWindow = null;
  });
}

function updateDockIcon(): void {
  const dark = nativeTheme.shouldUseDarkColors;
  if (app.dock) {
    const iconName = dark ? "icon-dark.png" : "icon.png";
    app.dock.setIcon(path.join(__dirname, "../build", iconName));
  } else if (process.platform === "win32" && mainWindow) {
    const iconName = dark ? "icon-dark.ico" : "icon.ico";
    mainWindow.setIcon(path.join(__dirname, "../build", iconName));
  }
}

app.whenReady().then(() => {
  updateDockIcon();
  nativeTheme.on("updated", updateDockIcon);

  const appMemory = readWorkspaceMemory(app.getPath("userData"));
  const initialWorkspacePath =
    appMemory.lastWorkspacePath && existsSync(appMemory.lastWorkspacePath)
      ? appMemory.lastWorkspacePath
      : null;
  currentSettings = readSettings(app.getPath("userData"));

  ipcMain.handle("get-backend-url", async () => {
    const port = await portPromise;
    return `http://127.0.0.1:${port}`;
  });

  ipcMain.handle("get-layout", async () => {
    return readLayoutMemory(app.getPath("userData"));
  });

  ipcMain.handle("save-layout", async (_event, json: string) => {
    writeLayoutMemory(app.getPath("userData"), json);
  });

  ipcMain.handle("get-initial-workspace", async () => {
    if (!initialWorkspacePath) return null;
    const port = await portPromise;
    const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: initialWorkspacePath }),
    });
    if (!resp.ok) return null;
    return resp.json();
  });

  ipcMain.handle("get-settings", async () => {
    currentSettings = readSettings(app.getPath("userData"));
    return rendererSettings(currentSettings);
  });

  ipcMain.handle("save-hotkeys", async (_event, hotkeys: unknown) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const nextSettings = settingsWithHotkeys(baseSettings, hotkeys);

    const validation = validateHotkeys(nextSettings.hotkeys);
    if (!validation.ok) {
      return { ok: false, errors: validation.errors };
    }

    const nextMenu = createMenu(nextSettings);
    writeSettings(app.getPath("userData"), nextSettings);
    currentSettings = nextSettings;
    applyMenu(nextMenu);

    return { ok: true, settings: rendererSettings(currentSettings) };
  });

  ipcMain.handle("save-retention", async (_event, retention: unknown) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const nextSettings: AppSettings = {
      ...baseSettings,
      version: 1,
      retention: normalizeRetention(retention),
    };
    writeSettings(app.getPath("userData"), nextSettings);
    currentSettings = nextSettings;

    try {
      const port = await portPromise;
      await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(nextSettings.retention),
      });
    } catch {
      console.warn("[main] failed to push retention to sidecar (will retry on next save)");
    }

    return { ok: true };
  });

  ipcMain.handle("save-provider-settings", async (_event, providers: ProviderSettings) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const nextSettings: AppSettings = {
      ...baseSettings,
      version: 1,
      providers: normalizeProviderSettings({
        ...providers,
        revision: Math.max(baseSettings.providers.revision + 1, providers.revision),
      }),
    };

    writeSettings(app.getPath("userData"), nextSettings);
    currentSettings = nextSettings;

    const port = await portPromise;
    await pushProviderSettings(`http://127.0.0.1:${port}`, nextSettings.providers);

    providerModels.delete("ollama");

    return { ok: true, settings: rendererSettings(currentSettings) };
  });

  ipcMain.handle("get-provider-models", async (_event, providerId: string) => {
    if (providerModels.has(providerId)) {
      return { models: providerModels.get(providerId)! };
    }
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/providers/${providerId}/models`);
      if (resp.ok) {
        const data = await resp.json() as { models: string[] };
        providerModels.set(providerId, data.models);
        return { models: data.models };
      }
    } catch {
      // Fall through to empty
    }
    return { models: [] };
  });

  ipcMain.handle("get-provider-labels", async () => {
    if (Object.keys(providerLabels).length > 0) {
      return { labels: { ...providerLabels } };
    }
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/providers`);
      if (resp.ok) {
        const data = await resp.json() as { providers: Array<{ id: string; label: string }> };
        const labels: Record<string, string> = {};
        for (const entry of data.providers) {
          labels[entry.id] = entry.label;
        }
        providerLabels = labels;
        return { labels: { ...labels } };
      }
    } catch {
      // Fall through to empty
    }
    return { labels: {} };
  });

  ipcMain.handle("get-claude-code-hook-status", async () => {
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/workspace/attention-hook/status`);
      if (resp.status === 409) {
        return { installed: false, error: "Open a workspace first." };
      }
      if (resp.ok) {
        return await resp.json() as { installed: boolean; error?: string };
      }
    } catch {
      // Fall through to unknown status
    }
    return { installed: false, error: "Couldn't reach the OrkWorks sidecar." };
  });

  ipcMain.handle("install-claude-code-hook", async () => {
    try {
      const port = await portPromise;
      const resp = await fetch(`http://127.0.0.1:${port}/workspace/attention-hook/install`, { method: "POST" });
      if (resp.status === 409) {
        return { installed: false, error: "Open a workspace first." };
      }
      const body = await resp.json() as { installed?: boolean; error?: string };
      if (resp.ok) {
        return { installed: Boolean(body.installed), error: undefined };
      }
      return { installed: false, error: body.error ?? "Couldn't install the hook." };
    } catch {
      return { installed: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  });

  ipcMain.handle("open-workspace", async () => {
    const result = await dialog.showOpenDialog({
      properties: ["openDirectory"],
      title: "Select Workspace",
    });
    if (result.canceled || result.filePaths.length === 0) return null;

    const dirPath = result.filePaths[0];
    workspacePath = dirPath;

    rememberWorkspacePath(app.getPath("userData"), dirPath);

    if (sidecarProcess) {
      sidecarProcess.kill();
      sidecarProcess = null;
    }
    backendPort = null;
    portPromise = new Promise<number>((resolve) => {
      portResolve = resolve;
    });

    sidecarProcess = spawn(getSidecarPath(), [], {
      cwd: dirPath,
      stdio: ["ignore", "pipe", "pipe"],
    });

    sidecarProcess.stdout?.on("data", (data: Buffer) => {
      const line = data.toString().trim();
      console.log(`[orkworksd] ${line}`);
      const match = line.match(/ORKWORKSD_PORT=(\d+)/);
      if (match) {
        backendPort = parseInt(match[1], 10);
        console.log(`[main] sidecar ready on port ${backendPort}`);
        if (portResolve) {
          portResolve(backendPort);
          portResolve = null;
        }
      }
    });

    sidecarProcess.stderr?.on("data", (data: Buffer) => {
      console.error(`[orkworksd:err] ${data.toString().trim()}`);
    });

    sidecarProcess.on("exit", (code) => {
      console.log(`[main] sidecar exited with code ${code}`);
      sidecarProcess = null;
    });

    const port = await portPromise;

    const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: dirPath }),
    });

    if (!resp.ok) return null;

    try {
      const retention = currentSettings?.retention ?? DEFAULT_RETENTION;
      await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(retention),
      });
    } catch {
      // Non-fatal: sidecar will use defaults until next save-retention
    }

    await syncSavedProviderSettings();

    return resp.json();
  });

  ipcMain.on("orkworks:panel-visibility", (_event, data: { panelId: string; visible: boolean }) => {
    const item = menuPanelItems[data.panelId];
    if (item) item.checked = data.visible;
  });

  ipcMain.on("orkworks:hotkey-capture-active", (_event, active: boolean) => {
    const nextActive = Boolean(active);
    if (hotkeyCaptureActive === nextActive) return;

    hotkeyCaptureActive = nextActive;
    currentSettings = currentSettings ?? readSettings(app.getPath("userData"));
    applyMenu(createMenu(currentSettings));
  });

  startSidecar(initialWorkspacePath ?? undefined);
  createWindow();
  applyMenu(createMenu(currentSettings));

  portPromise.then(async (port) => {
    try {
      const retention = currentSettings?.retention ?? DEFAULT_RETENTION;
      await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(retention),
      });
    } catch {
      // Sidecar may not be ready yet; will be pushed on next save-retention
    }
    await syncSavedProviderSettings();
  });

  async function syncSavedProviderSettings(): Promise<void> {
    const settings = currentSettings ?? readSettings(app.getPath("userData"));
    const port = await portPromise;
    const result = await pushProviderSettings(`http://127.0.0.1:${port}`, settings.providers);
    if (result.lastApplyError) {
      console.warn(`[main] failed to push provider settings: ${result.lastApplyError}`);
    }
  }

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

function killSidecar(): void {
  if (sidecarProcess) {
    sidecarProcess.kill();
    sidecarProcess = null;
  }
}

app.on("before-quit", killSidecar);

process.on("SIGTERM", () => {
  killSidecar();
  app.quit();
});

process.on("SIGINT", () => {
  killSidecar();
  app.quit();
});
