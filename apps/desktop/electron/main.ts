import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme, shell } from "electron";
import { spawn } from "child_process";
import { randomBytes } from "crypto";
import { existsSync } from "fs";
import * as path from "path";
import { pathToFileURL } from "url";
import { getDevRepoRoot, getDevSidecarPath, getPackagedSidecarPath } from "./paths";
import { readWorkspaceMemory, rememberWorkspacePath } from "./workspaceMemory";
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";
import type { AppSettings } from "./settingsMemory";
import { DEFAULT_HOTKEYS, DEFAULT_RETENTION, loadSettingsForStartup, normalizeDebugSettings, normalizeProviderSettings, normalizeRetention, readSettings, settingsWithHotkeys, validateHotkeys, writeSettings } from "./settingsMemory";
import { pushProviderSettings } from "./providerSettingsSync";
import type { ProviderApplyStatus, ProviderSettings } from "./providerTypes";
import { buildMenuTemplate } from "./menuTemplate";
import { getSessionPlanContent, requestSessionPlanReview, selectTerminalPlan } from "./planOpener";
import { configureExternalLinks, openExternalLink } from "./externalLinks";
import { createSidecarLifecycle, type SidecarLifecycle, type SidecarProcess, type SidecarState } from "./sidecarLifecycle";
import { createBackendRestorationCoordinator, switchWorkspaceBackend, type BackendRestorationCoordinator } from "./backendRestoration";
import type { BackendLifecycleEvent, BackendLifecycleWorkspace } from "./backendLifecycleEvent";
import { sanitizeBackendLifecycleFailure } from "./backendLifecycleFailure";
import { rendererConsoleDiagnostic, rendererOrigin, sanitizeRendererDiagnosticMessage } from "./rendererDiagnostic";
import { recoveryDocumentUrl } from "./rendererRecoveryDocument";
import { createRecoveryDocumentGuard } from "./rendererRecoveryState";

app.setName("OrkWorks");

let mainWindow: BrowserWindow | null = null;
let sidecarLifecycle: SidecarLifecycle | null = null;
let backendRestoration: BackendRestorationCoordinator<BackendLifecycleWorkspace> | null = null;
let workspacePath: string | null = null;
let menuPanelItems: Record<string, Electron.MenuItem> = {};
let currentSettings: AppSettings | null = null;
let providerModels: Map<string, string[]> = new Map();
let providerLabels: Record<string, string> = {};
let hotkeyCaptureActive = false;
let openPlanToken = "";
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

  const originalUrl = process.env.VITE_DEV_SERVER_URL
    || pathToFileURL(path.join(__dirname, "..", "dist", "index.html")).toString();
  configureExternalLinks(mainWindow.webContents, shell.openExternal, process.env.VITE_DEV_SERVER_URL, originalUrl);
  const recoveryUrl = recoveryDocumentUrl(originalUrl);

  const recoveryDocumentGuard = createRecoveryDocumentGuard(originalUrl);
  const loadRecoveryDocument = (): void => {
    if (!recoveryDocumentGuard.beginRecoveryDocumentLoad()
      || !mainWindow
      || mainWindow.isDestroyed()
      || mainWindow.webContents.isDestroyed()) return;
    void mainWindow.loadURL(recoveryUrl).catch(() => {
      recoveryDocumentGuard.recoveryDocumentLoadFailed();
    });
  };

  mainWindow.webContents.on("did-start-navigation", (_event, url, _isInPlace, isMainFrame) => {
    if (isMainFrame) recoveryDocumentGuard.beginOriginalDocumentNavigation(url);
  });

  mainWindow.webContents.on("did-finish-load", () => {
    recoveryDocumentGuard.finishOriginalDocumentLoad(mainWindow?.webContents.getURL() ?? "");
  });

  mainWindow.webContents.on("did-fail-load", (_event, errorCode, errorDescription, validatedURL, isMainFrame) => {
    if (!isMainFrame) return;
    console.error("[main] renderer diagnostic", {
      type: "did-fail-load",
      errorCode,
      reason: sanitizeRendererDiagnosticMessage(errorDescription),
      origin: rendererOrigin(validatedURL),
    });
    loadRecoveryDocument();
  });

  mainWindow.webContents.on("render-process-gone", (_event, details) => {
    console.error("[main] renderer diagnostic", {
      type: "render-process-gone",
      reason: details.reason,
      exitCode: details.exitCode,
    });
    loadRecoveryDocument();
  });

  mainWindow.webContents.on("console-message", (_event, level, _message, line, sourceId) => {
    console.warn("[main] renderer diagnostic", {
      ...rendererConsoleDiagnostic(level, sourceId, line),
    });
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(originalUrl);
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

function logBackendLifecycleFailure(scope: string, error: unknown): void {
  const detail = error instanceof Error ? error.stack ?? error.message : String(error);
  console.error("[main] backend lifecycle failure", scope, detail);
}

app.whenReady().then(() => {
  updateDockIcon();
  nativeTheme.on("updated", updateDockIcon);

  const appMemory = readWorkspaceMemory(app.getPath("userData"));
  const initialWorkspacePath =
    appMemory.lastWorkspacePath && existsSync(appMemory.lastWorkspacePath)
      ? appMemory.lastWorkspacePath
      : null;
  workspacePath = initialWorkspacePath;
  currentSettings = loadSettingsForStartup(app.getPath("userData"));

  let latestBackendLifecycle: BackendLifecycleEvent | null = null;
  let lastBackendFailure = "The OrkWorks sidecar is unavailable.";

  function publishBackendLifecycle(event: BackendLifecycleEvent): void {
    latestBackendLifecycle = event;
    mainWindow?.webContents.send("orkworks:backend-lifecycle", event);
  }

  async function restoreWorkspace(port: number, signal: AbortSignal): Promise<BackendLifecycleWorkspace | null> {
    if (!workspacePath) return null;

    const response = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: workspacePath }),
      signal,
    });
    if (!response.ok) {
      throw new Error(`Workspace restoration failed: ${response.status}`);
    }
    const rawWorkspace = await response.json() as Partial<BackendLifecycleWorkspace>;
    signal.throwIfAborted();
    return {
      path: rawWorkspace.path ?? "",
      repo_root: rawWorkspace.repo_root ?? null,
      branch: rawWorkspace.branch ?? null,
      dirty: rawWorkspace.dirty ?? null,
      lastActiveSessionId: rawWorkspace.lastActiveSessionId ?? null,
      activeHarnessIds: rawWorkspace.activeHarnessIds ?? [],
    };
  }

  async function applyRetentionSettings(port: number, signal: AbortSignal): Promise<void> {
    const retention = currentSettings?.retention ?? DEFAULT_RETENTION;
    try {
      const response = await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(retention),
        signal,
      });
      signal.throwIfAborted();
      if (!response.ok) {
        console.warn(`[main] failed to restore retention settings: ${response.status}`);
      }
    } catch (error) {
      signal.throwIfAborted();
      console.warn(`[main] failed to restore retention settings: ${error instanceof Error ? error.message : "unknown error"}`);
    }
  }

  async function syncSavedProviderSettings(port: number, signal: AbortSignal): Promise<void> {
    const settings = currentSettings ?? readSettings(app.getPath("userData"));
    const abortableFetch: typeof fetch = (input, init) => fetch(input, { ...init, signal });
    const result = await pushProviderSettings(
      `http://127.0.0.1:${port}`,
      settings.providers,
      abortableFetch,
    );
    signal.throwIfAborted();
    if (result.lastApplyError) {
      console.warn(`[main] failed to push provider settings: ${result.lastApplyError}`);
    }
  }

  const restoration = createBackendRestorationCoordinator<BackendLifecycleWorkspace>({
    setTimeout: (callback, delayMs) => setTimeout(callback, delayMs),
    clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
    onReady: (port, workspace) => {
      publishBackendLifecycle({ state: "ready", port, workspace });
    },
    onFailure: (error) => {
      logBackendLifecycleFailure("restoration", error);
      lastBackendFailure = sanitizeBackendLifecycleFailure(error);
      publishBackendLifecycle({ state: "failed", message: lastBackendFailure });
    },
  });
  backendRestoration = restoration;

  sidecarLifecycle = createSidecarLifecycle({
    spawn: (cwd): SidecarProcess => {
      const binaryPath = getSidecarPath();
      openPlanToken = randomBytes(32).toString("hex");
      console.log(`[main] starting sidecar: ${binaryPath}`);
      console.log(`[main] sidecar cwd: ${cwd}`);
      const child = spawn(binaryPath, [], {
        cwd,
        stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, ORKWORKS_OPEN_PLAN_TOKEN: openPlanToken },
      });
      child.stderr?.on("data", (data: Buffer) => {
        console.error(`[orkworksd:err] ${data.toString().trim()}`);
      });
      return child as SidecarProcess;
    },
    setTimeout: (callback, delayMs) => setTimeout(callback, delayMs),
    clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
    now: () => Date.now(),
    callbacks: {
      onReady: (port) => {
        console.log(`[main] sidecar ready on port ${port}`);
        restoration.restore(port, {
          restoreWorkspace: (signal) => restoreWorkspace(port, signal),
          applyRetentionSettings: (signal) => applyRetentionSettings(port, signal),
          syncProviderSettings: (signal) => syncSavedProviderSettings(port, signal),
        });
      },
      onUnavailable: (message) => {
        logBackendLifecycleFailure("sidecar", message);
        lastBackendFailure = sanitizeBackendLifecycleFailure(message);
        restoration.fail(new Error(lastBackendFailure));
      },
      onState: (state: SidecarState) => {
        if (state === "starting") {
          restoration.beginGeneration();
          publishBackendLifecycle({ state: "starting" });
        } else if (state === "retrying") {
          publishBackendLifecycle({ state: "retrying" });
        } else if (state === "exhausted") {
          publishBackendLifecycle({ state: "exhausted", message: lastBackendFailure });
        }
      },
    },
  });

  ipcMain.handle("get-backend-lifecycle", () => {
    return latestBackendLifecycle;
  });

  ipcMain.handle("get-backend-url", async () => {
    const port = await restoration.getReadiness();
    return `http://127.0.0.1:${port}`;
  });

  ipcMain.handle("retry-backend", async () => {
    if (!sidecarLifecycle) throw new Error("Backend lifecycle is unavailable");
    const lifecycleReadiness = sidecarLifecycle.retry();
    void lifecycleReadiness.catch(() => {});
    await restoration.getReadiness();
  });

  ipcMain.handle("open-external-link", (_event, url: unknown) => {
    openExternalLink(url, shell.openExternal);
  });

  ipcMain.handle("get-layout", async () => {
    return readLayoutMemory(app.getPath("userData"));
  });

  ipcMain.handle("save-layout", async (_event, json: string) => {
    writeLayoutMemory(app.getPath("userData"), json);
  });

  ipcMain.handle("get-initial-workspace", async () => {
    if (!initialWorkspacePath) return null;
    try {
      await restoration.getReadiness();
      return restoration.getRestoredWorkspace();
    } catch {
      return null;
    }
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

    let retentionApplyStatus: ProviderApplyStatus = {
      appliedRevision: null,
      appliedAt: null,
      lastApplyError: null,
    };
    try {
      const port = await restoration.getReadiness();
      const response = await fetch(`http://127.0.0.1:${port}/settings/retention`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(nextSettings.retention),
      });
      retentionApplyStatus = {
        appliedRevision: null,
        appliedAt: response.ok ? new Date().toISOString() : null,
        lastApplyError: response.ok ? null : `settings push failed: ${response.status}`,
      };
    } catch {
      console.warn("[main] failed to push retention to sidecar (will retry on next save)");
      retentionApplyStatus.lastApplyError = "settings push failed";
    }

    return { ok: true, retentionApplyStatus };
  });

  ipcMain.handle("save-debug-settings", async (_event, debug: unknown) => {
    const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
    const nextSettings: AppSettings = {
      ...baseSettings,
      version: 1,
      debug: normalizeDebugSettings(debug),
    };
    writeSettings(app.getPath("userData"), nextSettings);
    currentSettings = nextSettings;
    return { ok: true, settings: rendererSettings(currentSettings) };
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

    const port = await restoration.getReadiness();
    const providerApplyStatus = await pushProviderSettings(`http://127.0.0.1:${port}`, nextSettings.providers);

    providerModels.delete("ollama");

    return { ok: true, settings: rendererSettings(currentSettings), providerApplyStatus };
  });

  ipcMain.handle("verify-ollama", async (_event, baseUrl: string) => {
    const port = await restoration.getReadiness();
    const response = await fetch(`http://127.0.0.1:${port}/settings/providers/ollama/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ baseUrl }),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: "Couldn't verify Ollama." }));
      throw new Error(error.error ?? "Couldn't verify Ollama.");
    }

    return await response.json();
  });

  ipcMain.handle("get-provider-models", async (_event, providerId: string) => {
    if (providerModels.has(providerId)) {
      return { models: providerModels.get(providerId)! };
    }
    try {
      const port = await restoration.getReadiness();
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
      const port = await restoration.getReadiness();
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

  ipcMain.handle("get-plan-content", async (_event, sessionId: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    const port = await restoration.getReadiness();
    return getSessionPlanContent(`http://127.0.0.1:${port}`, sessionId, openPlanToken, fetch);
  });
  ipcMain.handle("request-plan-review", async (_event, sessionId: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    const port = await restoration.getReadiness();
    await requestSessionPlanReview(`http://127.0.0.1:${port}`, sessionId, openPlanToken, fetch);
  });
  ipcMain.handle("select-terminal-plan", async (_event, sessionId: unknown, printedPath: unknown) => {
    if (typeof sessionId !== "string" || !sessionId || typeof printedPath !== "string" || !printedPath) throw new Error("Invalid plan selection.");
    const port = await restoration.getReadiness();
    await selectTerminalPlan(`http://127.0.0.1:${port}`, sessionId, printedPath, openPlanToken, fetch);
  });

  const integrationActionLabels: Record<"status" | "install" | "uninstall", string> = {
    status: "check the integration status",
    install: "install the integration",
    uninstall: "uninstall the integration",
  };

  async function callIntegrationRoute(harnessId: unknown, action: "status" | "install" | "uninstall") {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      const port = await restoration.getReadiness();
      const method = action === "status" ? "GET" : "POST";
      const resp = await fetch(
        `http://127.0.0.1:${port}/workspace/integrations/${encodeURIComponent(harnessId)}/${action}`,
        { method },
      );
      if (resp.ok) {
        return { ok: true, status: await resp.json() };
      }
      const body = await resp.json().catch(() => ({ error: undefined }));
      return { ok: false, error: (body as { error?: string }).error ?? `Couldn't ${integrationActionLabels[action]}.` };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  ipcMain.handle("get-harness-integration-status", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "status"));

  ipcMain.handle("install-harness-integration", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "install"));

  ipcMain.handle("uninstall-harness-integration", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "uninstall"));

  async function parseErrorBody(resp: Response, fallback: string): Promise<string> {
    const body = await resp.json().catch(() => ({ error: undefined }));
    return (body as { error?: string }).error ?? fallback;
  }

  // PUT/DELETE /harnesses/:id (crates/orkworksd/src/http/harness_handlers.rs)
  // replace or remove the harness's *entire* stored override document, not
  // just the launch.command field these two functions touch. Harmless today
  // since nothing else writes a claude-code override, but if a future
  // feature adds another per-field override for this harness, Save/Clear
  // here will silently clobber it too — that endpoint would need to become
  // field-scoped (merge-on-write) before this could safely coexist with one.
  async function setHarnessCommandOverride(harnessId: unknown, commandPath: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    if (typeof commandPath !== "string" || !commandPath.trim()) throw new Error("Invalid command path.");
    try {
      const port = await restoration.getReadiness();
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "BuiltinPatch",
          patch: { launch: { command: commandPath } },
        }),
      });
      if (resp.ok) {
        return { ok: true, harness: await resp.json() };
      }
      return { ok: false, error: await parseErrorBody(resp, "Couldn't set the custom path.") };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  async function clearHarnessCommandOverride(harnessId: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      const port = await restoration.getReadiness();
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "DELETE",
      });
      if (resp.ok) {
        return { ok: true };
      }
      return { ok: false, error: await parseErrorBody(resp, "Couldn't clear the custom path.") };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  ipcMain.handle("set-harness-command-override", async (_event, harnessId: unknown, commandPath: unknown) =>
    setHarnessCommandOverride(harnessId, commandPath));

  ipcMain.handle("clear-harness-command-override", async (_event, harnessId: unknown) =>
    clearHarnessCommandOverride(harnessId));

  ipcMain.handle("open-workspace", async () => {
    const result = await dialog.showOpenDialog({
      properties: ["openDirectory"],
      title: "Select Workspace",
    });
    if (result.canceled || result.filePaths.length === 0) return null;

    const dirPath = result.filePaths[0];
    if (!sidecarLifecycle) throw new Error("Backend lifecycle is unavailable");
    const lifecycleReadiness = switchWorkspaceBackend(
      dirPath,
      (nextPath) => rememberWorkspacePath(app.getPath("userData"), nextPath),
      (nextPath) => {
        workspacePath = nextPath;
        try {
          return sidecarLifecycle!.start(nextPath);
        } catch (error) {
          const failure = error instanceof Error ? error : new Error("Backend replacement failed");
          restoration.fail(failure);
          throw failure;
        }
      },
    );
    void lifecycleReadiness.catch(() => {});
    await restoration.getReadiness();
    return restoration.getRestoredWorkspace();
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

  const initialSidecarCwd = initialWorkspacePath
    ?? (app.isPackaged ? app.getPath("home") : getDevRepoRoot(__dirname));
  const initialLifecycleReadiness = sidecarLifecycle.start(initialSidecarCwd);
  void initialLifecycleReadiness.catch(() => {});
  createWindow();
  applyMenu(createMenu(currentSettings));

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
  backendRestoration?.dispose();
  backendRestoration = null;
  sidecarLifecycle?.dispose();
  sidecarLifecycle = null;
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
