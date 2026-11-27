import { installTaskmasterIpc } from "./taskmasterIpc";
import { homedir } from "node:os";
import { discoverSkills, readSkillDetail } from "./skillInventory";
import type { SkillInventory } from "./skillTypes";
import { SkillInstaller, findApm, inspectApm, apmExecutablePath } from "./skillInstall";
import { realpath } from "node:fs/promises";
import { readGitChanges, validateGitChangesRequest } from "./gitChanges";
import { requestSessionTitle, validateSessionTitleRequest } from "./sessionTitle";
import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme, Notification, shell } from "electron";
import { installSessionNotificationIpc } from "./sessionNotificationIpc";
import { spawn, execFile } from "child_process";
import { stat } from "node:fs/promises";
import { inspectSessionApp, openSessionApp, sessionAppInvocation, type SessionAppDependencies } from "./sessionAppOpener";
import { normalizeImageCreation, submitImageSession } from "./sessionImages";
import { randomBytes } from "crypto";
import { existsSync, readFileSync } from "fs";
import { KnowledgeUpdates, synchronizeKnowledge } from "./knowledgeUpdates";
import { taskmasterRequest } from "./taskmasterSettings";
import * as path from "path";
import { pathToFileURL } from "url";
import { getDevSidecarPath, getDevUserDataPath, getPackagedSidecarPath } from "./paths";
import { accessibleWorkspaceDirectoryPath, canonicalWorkspacePath, readWorkspaceMemory, rememberWorkspacePath, pinWorkspacePath, unpinWorkspacePath, type WorkspaceMemoryDiagnostic } from "./workspaceMemory";
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";
import { createShellLayoutMemory } from "./shellLayoutMemory";
import { confirmShellMemoryRebuild } from "./shellMemoryRebuild";
import { createWorkspaceNavigationMemory, forgetRememberedWorkspaceWithNavigation, type LastCentralSurface } from "./workspaceNavigationMemory";
import type { AppSettings, RendererSettings } from "./settingsMemory";
import { DEFAULT_HOTKEYS, loadSettingsForStartup, normalizeDebugSettings, normalizeRetention, readSettings, settingsWithHotkeys, settingsWithPeonSelection, validateHotkeys, writeSettings } from "./settingsMemory";
import type { PeonAppliedState, PeonProviderVerificationResponse, PeonSelectionSaveResult, ProviderApplyStatus, ProviderId, ProviderModelOption } from "./providerTypes";
import { createPeonSelectionTransaction, normalizePeonSelectionInput, type PeonSelectionTransaction } from "./peonSelectionTransaction";
import { buildMenuTemplate } from "./menuTemplate";
import { getSessionPlanContent, requestSessionPlanReview, selectTerminalPlan, useLatestPlanDocument, resetSavedPlanDocuments, validatePlanRecoveryId, validatePlanDocumentKind, validatePlanSelectionId } from "./planOpener";
import { configureExternalLinks, openExternalLink } from "./externalLinks";
import type { SidecarProcess } from "./sidecarLifecycle";
import { sidecarSend, sidecarUrl, parseErrorBody, parsePeonError, errorFromBody, setSidecarRendererToken } from "./sidecarClient";
import { createUpdateService, type UpdateService } from "./updateService";
import { createElectronUpdateEngine, registerUpdateIpc } from "./electronUpdateEngine";
import type { BackendRetryResult, WorkspaceHistoryDiagnostic } from "./backendLifecycleEvent";
import { createWorkspaceRuntime, StaleBackendGenerationError, type WorkspaceRuntime } from "./workspaceRuntime";
import { rendererConsoleDiagnostic, rendererConsoleLevel, rendererOrigin, sanitizeRendererDiagnosticMessage } from "./rendererDiagnostic";
import { recoveryDocumentUrl } from "./rendererRecoveryDocument";
import { createRecoveryDocumentGuard, isSupersededNavigation } from "./rendererRecoveryState";
import {
  enableHarnessImmediate,
  saveActiveHarnessesWithIntegrations,
  type ActiveHarnessSaveResult,
  type ElectronHarnessConfig,
  type GroupedIntegrationStatus,
  type GroupedIntegrationStatusResult,
  type IntegrationKey,
  type IntegrationRevisionExpectation,
  type PlannedIntegrationMutation,
} from "./activeHarnessIntegration";

import { getWindowChromeOptions } from "./windowChrome";

app.setName("OrkWorks");
if (process.platform === "win32" && app.isPackaged) app.setAppUserModelId("ai.orkworks.desktop");

// A dev checkout running with ORKWORKS_DEV_PORT (the documented way to run
// `pnpm dev` while another checkout holds port 5173) gets userData scoped to
// this repo root and port, so two parallel dev instances never share workspace
// memory or settings; a rejected remembered-workspace restore in one instance
// must not delete the other instance's persisted state via forgetWorkspacePath.
if (!app.isPackaged && (process.env.ORKWORKS_DEV_PORT ?? "").trim() !== "") {
  app.setPath(
    "userData",
    getDevUserDataPath(__dirname, app.getPath("userData"), (process.env.ORKWORKS_DEV_PORT ?? "").trim()),
  );
}

let mainWindow: BrowserWindow | null = null;
let workspaceRuntime: WorkspaceRuntime | null = null;
let updateService: UpdateService | null = null;
let quitInProgress = false;
let quitBypass = false;
let menuPanelItems: Record<string, Electron.MenuItem> = {};
let currentSettings: AppSettings | null = null;
let providerModels = new Map<string, { models: string[]; modelOptions?: import("./providerTypes").ProviderModelOption[] }>();
const peonModelCatalogCache = new Map<string, { models: ProviderModelOption[]; observedAt: string }>();
let hotkeyCaptureActive = false;
// Lower-privilege credential for the renderer and ordinary main-process calls
// (ADR 0117). It lives for this app run; openPlanToken stays main-only.
const rendererToken = randomBytes(32).toString("hex");
setSidecarRendererToken(rendererToken);
let settingsWriteQueue: Promise<void> = Promise.resolve();
const menuPanelIds = ["sessions", "detail", "terminal", "recommendations"];

async function discoverProviderModels(
  port: number,
  providerId: string,
  ollamaBaseUrl?: string,
  signal?: AbortSignal,
): Promise<{ models: string[]; modelOptions?: import("./providerTypes").ProviderModelOption[] }> {
  const response = await sidecarSend(port, `settings/providers/${encodeURIComponent(providerId)}/models`, {
    method: "POST",
    jsonBody: ollamaBaseUrl ? { baseUrl: ollamaBaseUrl } : {},
    ...(signal ? { signal } : {}),
  });
  if (!response.ok) throw new Error("model discovery failed");
  return await response.json() as { models: string[]; modelOptions?: import("./providerTypes").ProviderModelOption[] };
}

async function listSessions(port: number, signal?: AbortSignal): Promise<Array<{ lifecycle?: string }>> {
  const response = await sidecarSend(port, "sessions", signal ? { signal } : {});
  if (!response.ok) throw new Error(`list sessions failed: ${response.status}`);
  const sessions: unknown = await response.json();
  if (!Array.isArray(sessions)) throw new Error("list sessions failed: malformed response");
  return sessions as Array<{ lifecycle?: string }>;
}

function rendererSettings(settings: AppSettings): RendererSettings {
  return {
    ...settings,
    defaultHotkeys: { ...DEFAULT_HOTKEYS },
  };
}

function enqueueSettingsWrite<T>(operation: () => T | Promise<T>): Promise<T> {
  const result = settingsWriteQueue.then(operation, operation);
  settingsWriteQueue = result.then(() => undefined, () => undefined);
  return result;
}

function providerModelCacheKey(providerId: string, ollamaBaseUrl?: string): string {
  return providerId === "ollama" ? `${providerId}:${ollamaBaseUrl ?? ""}` : providerId;
}

function createMenu(settings: AppSettings): Electron.Menu {
  const template = buildMenuTemplate({
    appName: app.name,
    openExternal: url => openExternalLink(url, shell.openExternal),
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

function createWindow(clearSessionNotifications: () => void): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 500,
    title: "OrkWorks",
    icon: path.join(__dirname, "../build", process.platform === "win32" ? "icon.ico" : "icon.png"),
    ...getWindowChromeOptions(process.platform),
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, "preload.js"),
      additionalArguments: [`--orkworks-packaged=${app.isPackaged}`],
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
    if (!isMainFrame) return;
    if (!_isInPlace) clearSessionNotifications();
    recoveryDocumentGuard.beginOriginalDocumentNavigation(url);
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
    // ERR_ABORTED marks a navigation superseded by another in-flight load
    // (dev-server reload, a racing loadURL, the recovery page's own retry) —
    // not a genuine failure, so the recovery document must not load.
    if (isSupersededNavigation(errorCode)) return;
    loadRecoveryDocument();
  });

  mainWindow.webContents.on("render-process-gone", (_event, details) => {
    clearSessionNotifications();
    console.error("[main] renderer diagnostic", {
      type: "render-process-gone",
      reason: details.reason,
      exitCode: details.exitCode,
    });
    loadRecoveryDocument();
  });

  mainWindow.webContents.on("console-message", ({ level: severity, sourceId, lineNumber }) => {
    console.warn("[main] renderer diagnostic", {
      ...rendererConsoleDiagnostic(rendererConsoleLevel(severity), sourceId, lineNumber),
    });
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(originalUrl);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }

  mainWindow.on("closed", () => {
    clearSessionNotifications();
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

function toWorkspaceHistoryDiagnostic(
  diagnostic: WorkspaceMemoryDiagnostic | null,
): WorkspaceHistoryDiagnostic | null {
  return diagnostic ? { code: diagnostic.code, message: diagnostic.message } : null;
}

interface WorkspaceHistorySnapshot {
  pinned: string[];
  recent: string[];
  diagnostic: WorkspaceHistoryDiagnostic | null;
}

function toWorkspaceHistorySnapshot(memory: ReturnType<typeof readWorkspaceMemory>): WorkspaceHistorySnapshot {
  return {
    pinned: memory.pinnedWorkspacePaths,
    recent: memory.recentWorkspacePaths,
    diagnostic: toWorkspaceHistoryDiagnostic(memory.diagnostic),
  };
}

app.whenReady().then(async () => {
  updateDockIcon();
  nativeTheme.on("updated", updateDockIcon);

  const appMemory = readWorkspaceMemory(app.getPath("userData"));
  if (appMemory.diagnostic) {
    console.warn("[main] workspace history diagnostic", appMemory.diagnostic.message);
  }
  const initialHistoryDiagnostic = toWorkspaceHistoryDiagnostic(appMemory.diagnostic);
  currentSettings = loadSettingsForStartup(app.getPath("userData"));
  const shellLayoutMemory = createShellLayoutMemory(app.getPath("userData"));
  const workspaceNavigationMemory = createWorkspaceNavigationMemory(app.getPath("userData"));

  let appliedPeonState: PeonAppliedState | null = null;
  const knowledgeResources = app.isPackaged
    ? path.join(process.resourcesPath, "knowledge")
    : path.join(__dirname, "../resources/knowledge");
  const knowledgeKeyPath = path.join(knowledgeResources, "public-key.pem");
  const knowledgeUpdates = new KnowledgeUpdates({
    directory: path.join(app.getPath("userData"), "knowledge"),
    starterPath: path.join(knowledgeResources, "starter.json"),
    publicKey: existsSync(knowledgeKeyPath) ? readFileSync(knowledgeKeyPath, "utf8") : "",
    feedUrl: "https://rambolarsen.github.io/brain/orkworks-knowledge/manifest.json",
  });

  function normalizeDebugAttention(value: unknown): string {
    if (value === "working" || value === "idle" || value === "needs_you"
      || value === "blocked" || value === "failed" || value === "capped") return value;
    throw new Error("Invalid debug attention.");
  }

  function persistedOllamaBaseUrl(): string | undefined {
    return currentSettings?.providers.peonSelection?.ollamaBaseUrl
      ?? currentSettings?.providers.ollamaBaseUrl;
  }

  const clearSessionNotifications = installSessionNotificationIpc(
    () => mainWindow,
    () => workspaceRuntime?.getLifecycle().state === "ready"
      && currentSettings?.desktopSessionNotifications === true,
    { ipc: ipcMain, isSupported: () => Notification.isSupported(), create: options => new Notification(options) },
  );

  const runtime = createWorkspaceRuntime({
    generateToken: () => {
      const openPlanToken = randomBytes(32).toString("hex");
      return openPlanToken;
    },
    spawnSidecar: (cwd, openPlanToken): SidecarProcess => {
      const binaryPath = getSidecarPath();
      console.log(`[main] starting sidecar: ${binaryPath}`);
      console.log(`[main] sidecar cwd: ${cwd}`);
      const child = spawn(binaryPath, [], {
        cwd,
        stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, ORKWORKS_OPEN_PLAN_TOKEN: openPlanToken, ORKWORKS_RENDERER_TOKEN: rendererToken },
      });
      child.stderr?.on("data", (data: Buffer) => {
        console.error(`[orkworksd:err] ${data.toString().trim()}`);
      });
      return child as SidecarProcess;
    },
    validateDestination: (candidate) => accessibleWorkspaceDirectoryPath(candidate),
    canonicalizePath: (candidate) => canonicalWorkspacePath(candidate),
    rememberWorkspacePath: (canonicalPath) => {
      const result = rememberWorkspacePath(app.getPath("userData"), canonicalPath);
      return { diagnostic: toWorkspaceHistoryDiagnostic(result.diagnostic) };
    },
    forgetWorkspacePath: async (rejectedPath) => {
      const result = await forgetRememberedWorkspaceWithNavigation(
        app.getPath("userData"),
        rejectedPath,
        workspaceNavigationMemory,
      );
      return {
        historyDiagnostic: toWorkspaceHistoryDiagnostic(result.history.diagnostic),
        navigationDiagnostic: result.navigation && !result.navigation.ok ? result.navigation.diagnostic : null,
      };
    },
    getSettings: () => currentSettings ?? readSettings(app.getPath("userData")),
    writeSettings: (nextSettings) => {
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
    },
    enqueueSettingsWrite,
    clearProviderModelsCache: () => {
      providerModels.clear();
    },
    initialHistoryDiagnostic,
    onLifecycleChange: (event) => {
      clearSessionNotifications();
      mainWindow?.webContents.send("orkworks:backend-lifecycle", event);
    },
    onReadySideEffects: ({ port, signal, isCurrent }) => {
      restorePersistedPeonSelection(port, signal, isCurrent);
      void refreshKnowledge();
    },
  });
  workspaceRuntime = runtime;

  let knowledgeSync: Promise<void> | null = null;
  function refreshKnowledge(): Promise<void> {
    if (knowledgeSync) return knowledgeSync;
    if (runtime.getLifecycle().state !== "ready") return Promise.resolve();
    const requestedGeneration = runtime.getGeneration();
    knowledgeSync = runtime.runReady(async ({ port, token, isCurrent }) => {
      const status = await taskmasterRequest(port, token, "settings");
      if (!isCurrent()) return;
      const settings = status.settings as { automaticKnowledgeUpdates?: boolean };
      knowledgeUpdates.setEnabled(settings.automaticKnowledgeUpdates !== false);
      await synchronizeKnowledge(
        knowledgeUpdates,
        (activation) => taskmasterRequest(port, token, "knowledge", activation),
        isCurrent,
      );
    }).catch((error: unknown) => {
      if (error instanceof StaleBackendGenerationError) return;
      console.warn("[taskmaster] knowledge unavailable:", error instanceof Error ? error.message : "unknown error");
    }).finally(() => {
      knowledgeSync = null;
      if (requestedGeneration !== runtime.getGeneration() && runtime.getLifecycle().state === "ready") {
        void refreshKnowledge();
      }
    });
    return knowledgeSync;
  }
  const knowledgeTimer = setInterval(() => { void refreshKnowledge(); }, 6 * 60 * 60 * 1000);
  knowledgeTimer.unref();
  app.once("before-quit", () => { clearInterval(knowledgeTimer); knowledgeUpdates.setEnabled(false); });

  async function fetchHarnessesFromSidecar(
    port: number,
    signal: AbortSignal,
  ): Promise<{ documentRevision: string | null; harnesses: ElectronHarnessConfig[] }> {
    const response = await sidecarSend(port, "harnesses", { signal });
    if (!response.ok) throw new Error(await parseErrorBody(response, "Couldn't load coding tool definitions."));
    const data = await response.json() as {
      documentRevision?: unknown;
      harnesses?: Array<{
        definition?: ElectronHarnessConfig;
        origin?: ElectronHarnessConfig["origin"];
        compatibility?: {
          profile?: string | null;
          sessionSignals?: unknown;
          integration?: unknown;
        };
      }>;
    };
    if (!Array.isArray(data.harnesses)) throw new Error("Malformed harness list response.");
    if (data.documentRevision !== null && typeof data.documentRevision !== "string" && data.documentRevision !== undefined) {
      throw new Error("Malformed harness list revision.");
    }
    const harnesses = data.harnesses.map((entry) => {
      if (!entry.definition || !entry.origin) throw new Error("Malformed harness list entry.");
      return {
        ...entry.definition,
        origin: entry.origin,
        profile: entry.compatibility?.profile ?? null,
        sessionSignals: entry.compatibility?.sessionSignals ?? entry.definition.sessionSignals,
        integration: entry.compatibility?.integration ?? entry.definition.integration,
      };
    });
    return { documentRevision: data.documentRevision === undefined ? null : data.documentRevision, harnesses };
  }

  function fetchHarnessesForSave(): Promise<{ documentRevision: string | null; harnesses: ElectronHarnessConfig[] }> {
    return runtime.runReady(({ port, signal }) => fetchHarnessesFromSidecar(port, signal));
  }

  async function persistActiveHarnesses(
    ids: string[],
    expectedActiveHarnessRevision: number,
  ): Promise<
    | { ok: true; activeHarnessRevision: number }
    | { ok: false; error: string; code?: string }
  > {
    try {
      return await runtime.runReady(async ({ port, generation, workspacePath: capturedPath, signal }) => {
        const response = await sidecarSend(port, "workspace/active-harnesses", {
          method: "PUT",
          jsonBody: { activeHarnessIds: ids, expectedActiveHarnessRevision },
          signal,
        });
        if (response.ok) {
          const body = await response.json() as { activeHarnessRevision?: unknown };
          const savedActiveHarnessRevision = body.activeHarnessRevision;
          if (
            typeof savedActiveHarnessRevision !== "number" ||
            !Number.isInteger(savedActiveHarnessRevision) ||
            savedActiveHarnessRevision < 0
          ) {
            return { ok: false, error: "The sidecar returned an invalid active harness revision." };
          }
          // The workspace can switch while this PUT is in flight. Discard a stale
          // response without overwriting the revision seeded by the new workspace.
          // Callers re-check their guards and report stale_workspace.
          if (!runtime.setActiveHarnessRevision(generation, capturedPath, savedActiveHarnessRevision)) {
            return { ok: true, activeHarnessRevision: expectedActiveHarnessRevision };
          }
          return { ok: true, activeHarnessRevision: savedActiveHarnessRevision };
        }
        const body = await response.json().catch(() => ({ error: undefined, code: undefined })) as {
          error?: string;
          code?: string;
        };
        return {
          ok: false,
          error: body.error ?? "Couldn't save active coding tools.",
          ...(body.code ? { code: body.code } : {}),
        };
      });
    } catch (error) {
      if (error instanceof StaleBackendGenerationError) {
        return { ok: true, activeHarnessRevision: expectedActiveHarnessRevision };
      }
      throw error;
    }
  }

  function restorePersistedPeonSelection(port: number, signal: AbortSignal, isCurrent: () => boolean): void {
    const selection = currentSettings?.providers.peonSelection;
    if (!selection) return;
    void peonTransaction.syncPersistedSelection(selection, signal, port)
      .then((applied) => {
        if (!isCurrent()) return;
        appliedPeonState = applied;
      })
      .catch((error: unknown) => {
        if (!isCurrent()) return;
        console.warn(`[main] failed to restore Peon selection: ${error instanceof Error ? error.message : "unknown error"}`);
      });
  }

  function runPeonOperation<T>(
    readyPort: number | undefined,
    externalSignal: AbortSignal | undefined,
    operation: (port: number, signal: AbortSignal) => Promise<T>,
  ): Promise<T> {
    if (readyPort !== undefined) {
      const signal = externalSignal ?? new AbortController().signal;
      return operation(readyPort, signal);
    }
    return runtime.runReady(({ port, signal }) => {
      const combinedSignal = externalSignal ? AbortSignal.any([externalSignal, signal]) : signal;
      return operation(port, combinedSignal);
    });
  }

  const peonTransaction: PeonSelectionTransaction = createPeonSelectionTransaction({
    discover: (provider, ollamaBaseUrl) => runtime.runReady(async ({ port, signal }) =>
      (await discoverProviderModels(port, provider, ollamaBaseUrl, signal)).models),
    verify: ({ provider, ollamaBaseUrl, generation, readyPort, signal }) =>
      runPeonOperation(readyPort, signal, async (port, requestSignal) => {
        const body: { provider: string; generation: number; ollamaBaseUrl?: string } = { provider, generation };
        if (provider === "ollama") body.ollamaBaseUrl = ollamaBaseUrl ?? persistedOllamaBaseUrl();
        const response = await sidecarSend(port, "settings/peon/provider/verify", {
          method: "POST",
          jsonBody: body,
          signal: requestSignal,
        });
        if (!response.ok) throw await parsePeonError(response, "Couldn't verify the Peon provider.");
        const result = await response.json() as PeonProviderVerificationResponse;
        const cacheKey = providerModelCacheKey(provider, provider === "ollama" ? (result.ollamaBaseUrl ?? ollamaBaseUrl) : undefined);
        const modelOptions = result.modelOptions ?? result.models.map((id) => ({ id, displayName: id, reasoningEfforts: [], defaultReasoningEffort: null }));
        if (result.ok && modelOptions.length > 0) {
          peonModelCatalogCache.set(cacheKey, { models: modelOptions, observedAt: new Date().toISOString() });
        } else if (result.ok && result.capabilities.modelDiscovery) {
          const cached = peonModelCatalogCache.get(cacheKey);
          if (cached) return { ...result, models: cached.models.map((model) => model.id), modelOptions: cached.models, catalogStale: true, catalogObservedAt: cached.observedAt };
        }
        return result;
      }),
    apply: ({ selection, generation, readyPort, signal, skipTest }) =>
      runPeonOperation(readyPort, signal, async (port, requestSignal) => {
        const response = await sidecarSend(port, "settings/peon/test-and-apply", {
          method: "POST",
          jsonBody: skipTest ? { selection, generation, skipTest } : { selection, generation },
          signal: requestSignal,
        });
        if (!response.ok) throw await parsePeonError(response, "Couldn't apply the Peon provider.");
        return await response.json();
      }),
    getApplied: (signal) =>
      runPeonOperation(undefined, signal, async (port, requestSignal) => {
        const response = await sidecarSend(port, "settings/peon/applied", { signal: requestSignal });
        if (!response.ok) throw await parsePeonError(response, "Couldn't read the applied Peon provider.");
        return await response.json();
      }),
  });

  ipcMain.handle("get-backend-lifecycle", () => runtime.getLifecycle());

  ipcMain.on("get-backend-token-sync", (event) => {
    // Only the application window's top-level document may hold the credential.
    event.returnValue = mainWindow !== null && event.senderFrame === mainWindow.webContents.mainFrame ? rendererToken : "";
  });

  ipcMain.handle("get-backend-url", () => runtime.getBackendUrl());

  ipcMain.handle("create-session-with-images", async (_event, expectedBackendUrl: unknown, value: unknown) => {
    const request = normalizeImageCreation(value);
    return runtime.runReady(
      ({ port, token, signal }) => submitImageSession(port, token, request, signal),
      {
        expectedWorkspaceIdentity: request.expectedWorkspacePath,
        expectedBackendUrl: typeof expectedBackendUrl === "string" ? expectedBackendUrl : "",
      },
    );
  });

  ipcMain.handle("apply-debug-attention", async (_event, id: unknown, attention: unknown, message: unknown) => {
    if (typeof id !== "string" || !id) throw new Error("Invalid session ID.");
    const normalizedAttention = normalizeDebugAttention(attention);
    if (message !== undefined && typeof message !== "string") throw new Error("Invalid debug attention message.");
    await runtime.runReady(async ({ port, token, signal }) => {
      const response = await sidecarSend(port, `sessions/${encodeURIComponent(id)}/debug-injection`, {
        method: "POST",
        jsonBody: { attention: normalizedAttention, message },
        token,
        signal,
        timeoutMs: 15_000,
      });
      if (!response.ok) throw new Error(`apply debug attention failed: ${response.status}`);
    });
  });

  ipcMain.handle("retry-backend", (): Promise<BackendRetryResult> => runtime.retry());

  function sessionAppRequest(expectedWorkspace: unknown, id: unknown, open: boolean, destination: unknown) {
    if (typeof expectedWorkspace !== "string") throw new StaleBackendGenerationError();
    return runtime.runReady(async ({ port, signal }) => {
      const read = async (resource: string) => {
        const response = await sidecarSend(port, resource, { signal, timeoutMs: 10_000 });
        if (!response.ok) throw new Error("Couldn't read the current session's desktop destination.");
        return response.json();
      };
      const deps: SessionAppDependencies = {
        platform: process.platform, arch: process.arch, signal,
        readSessions: () => read("sessions"), readHarnesses: () => read("harnesses"),
        applicationForProtocol: async url => app.getApplicationNameForProtocol(url),
        checkDirectory: async cwd => { if (!(await stat(cwd)).isDirectory()) throw new Error("The session directory is unavailable."); },
        openExternal: url => shell.openExternal(url),
        run: (command, args, cwd) => new Promise<string>((resolve, reject) => {
          signal.throwIfAborted();
          const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith("ORKWORKS_")));
          const invocation = sessionAppInvocation(process.platform, command, args, process.env.ComSpec);
          execFile(invocation.file, invocation.args, { cwd, env: { ...env, ...invocation.env }, signal, timeout: 15_000, maxBuffer: 256 * 1024, windowsHide: true }, (error, stdout) => {
            if (error) reject(new Error("Couldn't open Claude. Check that Claude Code and Claude Desktop are installed, you are signed in with a subscription, and the session is closed."));
            else resolve(stdout);
          });
        }),
      };
      return open ? openSessionApp(id, deps, destination) : inspectSessionApp(id, deps, destination);
    }, { expectedWorkspaceIdentity: expectedWorkspace });
  }
  ipcMain.handle("inspect-session-app", (_event, expectedWorkspace: unknown, id: unknown, destination: unknown) => sessionAppRequest(expectedWorkspace, id, false, destination));
  ipcMain.handle("open-session-app", (_event, expectedWorkspace: unknown, id: unknown, destination: unknown) => sessionAppRequest(expectedWorkspace, id, true, destination));

  let skillInventory: SkillInventory | null = null;
  let inventoryGeneration = "";
  const selectedSkillSources = new Set<string>();
  let apmSettingsRevision = 0;
  const apmOverride = () => readSettings(app.getPath("userData")).apmExecutablePath ?? null;
  const skillInstaller = new SkillInstaller({
    home: homedir(), env: process.env, workspace: () => runtime.getWorkspacePath(),
    workspaceRevision: () => runtime.getWorkspaceRevision(),
    executable: () => findApm(process.env, apmOverride()),
    configurationRevision: () => `${apmSettingsRevision}/${apmOverride()}`,
    confirm: async preview => {
      const response = await dialog.showMessageBox(mainWindow!, {
        type: "question", title: "Install skill", buttons: ["Cancel", "Install skill"], defaultId: 0, cancelId: 0,
        message: `Install this ${preview.scope} skill?`,
        detail: `${preview.scope === "personal" ? "Personal skills may be available across projects." : "Only the selected workspace/worktree is targeted."}\n\nSource: ${preview.source}\nDestination: ${preview.destination}\nCoding tool target: Codex\n\n${preview.summary}\n\nAffected paths/categories:\n${preview.paths.join("\n")}\n\nAPM runs as your user and is not sandboxed by OrkWorks.`,
      });
      return response.response === 1;
    },
  });
  ipcMain.handle("get-skill-inventory", async () => {
    const generation = runtime.getWorkspaceRevision();
    const inventory = await discoverSkills({ workspace: runtime.getWorkspacePath(), home: homedir(), env: process.env });
    if (generation !== runtime.getWorkspaceRevision()) throw new Error("Workspace changed. Refresh skills.");
    const revision = apmSettingsRevision;
    inventory.apm = await inspectApm(process.env, apmOverride());
    if (revision !== apmSettingsRevision || generation !== runtime.getWorkspaceRevision()) throw new Error("Configuration changed. Refresh skills.");
    skillInventory = inventory; inventoryGeneration = generation;
    return inventory;
  });
  ipcMain.handle("save-apm-executable", (_event, value: unknown) => enqueueSettingsWrite(() => {
    const override = apmExecutablePath(value);
    const base = readSettings(app.getPath("userData"));
    const next = { ...base, apmExecutablePath: override };
    writeSettings(app.getPath("userData"), next);
    currentSettings = next;
    apmSettingsRevision++;
  }));
  ipcMain.handle("read-skill-detail", async (_event, id: unknown) => {
    if (!skillInventory || inventoryGeneration !== runtime.getWorkspaceRevision()) throw new Error("Refresh skills before opening details.");
    return readSkillDetail(skillInventory, id);
  });
  ipcMain.handle("pick-skill-source", async () => {
    const selection = await dialog.showOpenDialog(mainWindow!, { title: "Choose a skill directory containing SKILL.md", properties: ["openDirectory"] });
    if (selection.canceled || !selection.filePaths[0]) return null;
    const source = await realpath(selection.filePaths[0]);
    if (selectedSkillSources.size >= 32) selectedSkillSources.clear();
    selectedSkillSources.add(source);
    return source;
  });
  ipcMain.handle("preview-skill-add", (_event, input: unknown) => {
    if (!input || typeof input !== "object") throw new Error("Invalid skill request.");
    const request = input as { source?: unknown; scope?: unknown; expectedWorkspace?: unknown };
    if (typeof request.source !== "string") throw new Error("Choose a source.");
    if (path.isAbsolute(request.source) && !selectedSkillSources.has(request.source)) throw new Error("Select the local skill through Choose folder first.");
    if (request.scope === "workspace" && request.expectedWorkspace !== runtime.getWorkspacePath()) throw new Error("Workspace changed. Preview again.");
    return skillInstaller.preview(input);
  });
  ipcMain.handle("install-skill-add", (_event, id: unknown) => skillInstaller.install(id));

  ipcMain.handle("open-external-link", (_event, url: unknown) => {
    openExternalLink(url, shell.openExternal);
  });

  ipcMain.handle("get-layout", async () => {
    return readLayoutMemory(app.getPath("userData"));
  });

  ipcMain.handle("save-layout", async (_event, json: string) => {
    writeLayoutMemory(app.getPath("userData"), json);
  });

  ipcMain.handle("get-shell-layout", () => shellLayoutMemory.read());
  ipcMain.handle("save-shell-layout", (_event, preferences: unknown) => shellLayoutMemory.save(preferences));
  ipcMain.handle("reset-shell-layout", () => shellLayoutMemory.reset());
  ipcMain.handle("rebuild-shell-layout", async () => confirmShellMemoryRebuild("layout", mainWindow,
    (owner, options) => dialog.showMessageBox(owner, options), () => shellLayoutMemory.rebuild(true)));

  ipcMain.handle("get-workspace-navigation", () => {
    const { workspaceIdentity } = runtime.getReadyNavigationIdentity();
    const snapshot = workspaceNavigationMemory.read();
    return {
      lastCentralSurface: snapshot.entries.find((entry) => entry.workspaceIdentity === workspaceIdentity)?.lastCentralSurface ?? null,
      revision: snapshot.revision,
      diagnostic: snapshot.diagnostic,
    };
  });
  ipcMain.handle("complete-workspace-navigation", async (_event, surface: unknown) => {
    if (surface !== "terminal" && surface !== "review") throw new Error("Invalid central surface.");
    const { workspaceIdentity, generation, isCurrent } = runtime.getReadyNavigationIdentity();
    return workspaceNavigationMemory.complete(workspaceIdentity, generation, isCurrent, surface as LastCentralSurface);
  });
  ipcMain.handle("delete-workspace-navigation", async () => {
    const { workspaceIdentity, generation, isCurrent } = runtime.getReadyNavigationIdentity();
    return workspaceNavigationMemory.delete(workspaceIdentity, generation, isCurrent);
  });
  ipcMain.handle("rebuild-workspace-navigation", async () => confirmShellMemoryRebuild("navigation", mainWindow,
    (owner, options) => dialog.showMessageBox(owner, options), () => workspaceNavigationMemory.rebuild(true)));

  ipcMain.handle("get-initial-workspace", async () => runtime.getInitialWorkspaceSnapshot());

  ipcMain.handle("save-desktop-session-notifications", async (event, enabled: unknown) => {
    if (!mainWindow || mainWindow.isDestroyed() || event.sender !== mainWindow.webContents
        || event.senderFrame !== mainWindow.webContents.mainFrame || typeof enabled !== "boolean") {
      throw new Error("Invalid desktop notifications setting.");
    }
    await enqueueSettingsWrite(() => {
      const base = readSettings(app.getPath("userData"));
      const nextSettings = { ...base, desktopSessionNotifications: enabled };
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
      if (!enabled) clearSessionNotifications();
    });
  });
  ipcMain.handle("get-settings", async () => {
    currentSettings = readSettings(app.getPath("userData"));
    return rendererSettings(currentSettings);
  });
  installTaskmasterIpc({
    handle: (name, handler) => ipcMain.handle(name, handler),
    withReady: (operation) => runtime.runReady(({ port, token, signal }) => operation(port, token, signal)),
    workspacePath: () => runtime.getWorkspacePath(),
    getGeneration: () => runtime.getGeneration(),
    getReadiness: () => runtime.getReadiness(),
    getToken: () => runtime.getOpenPlanToken(),
    setKnowledgeUpdatesEnabled: (enabled) => knowledgeUpdates.setEnabled(enabled),
    getKnowledgeUpdateStatus: () => knowledgeUpdates.status(),
    refreshKnowledge: () => { void refreshKnowledge(); },
    enqueueSettingsWrite,
    confirmInferenceApproval: async (detail) => {
      if (!mainWindow || mainWindow.isDestroyed()) return false;
      const result = await dialog.showMessageBox(mainWindow, {
        type: "warning", title: "Approve custom inference executable?", message: "Trust this executable with background inference context?", detail,
        buttons: ["Cancel", "Approve executable"], defaultId: 0, cancelId: 0, noLink: true,
      });
      return result.response === 1;
    },
  });

  ipcMain.handle("save-hotkeys", async (_event, hotkeys: unknown) => {
    const { nextSettings, nextMenu } = await enqueueSettingsWrite(() => {
      const baseSettings = readSettings(app.getPath("userData"));
      const nextSettings = settingsWithHotkeys(baseSettings, hotkeys);
      const validation = validateHotkeys(nextSettings.hotkeys);
      if (!validation.ok) return { nextSettings: null, nextMenu: null, errors: validation.errors };
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
      return { nextSettings, nextMenu: createMenu(nextSettings), errors: null };
    });
    if (!nextSettings || !nextMenu) {
      const validation = await enqueueSettingsWrite(() => {
        const candidate = settingsWithHotkeys(readSettings(app.getPath("userData")), hotkeys);
        return validateHotkeys(candidate.hotkeys);
      });
      return { ok: false, errors: validation.errors };
    }
    applyMenu(nextMenu);

    return { ok: true, settings: rendererSettings(nextSettings) };
  });

  ipcMain.handle("save-retention", async (_event, retention: unknown) => {
    const nextSettings = await enqueueSettingsWrite(() => {
      const baseSettings = readSettings(app.getPath("userData"));
      const nextSettings: AppSettings = {
        ...baseSettings,
        version: 1,
        retention: normalizeRetention(retention),
      };
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
      return nextSettings;
    });

    let retentionApplyStatus: ProviderApplyStatus = {
      appliedRevision: null,
      appliedAt: null,
      lastApplyError: null,
    };
    try {
      const response = await runtime.runReady(({ port, signal }) => sidecarSend(port, "settings/retention", {
        method: "POST",
        jsonBody: nextSettings.retention,
        signal,
      }));
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
    const nextSettings = await enqueueSettingsWrite(() => {
      const baseSettings = readSettings(app.getPath("userData"));
      const nextSettings: AppSettings = {
        ...baseSettings,
        version: 1,
        debug: normalizeDebugSettings(debug),
      };
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
      return nextSettings;
    });
    return { ok: true, settings: rendererSettings(nextSettings) };
  });

  // peonSelectionMatchesAppliedState is enforced by peonSelectionTransaction.
  ipcMain.handle("verify-peon-provider", async (_event, provider: unknown, ollamaBaseUrl: unknown) => {
    if (typeof provider !== "string" || !provider.trim()) throw new Error("Invalid Peon provider.");
    if (ollamaBaseUrl !== undefined && typeof ollamaBaseUrl !== "string") {
      throw new Error("Invalid Ollama base URL.");
    }
    return peonTransaction.verify(
      provider.trim() as ProviderId,
      ollamaBaseUrl as string | undefined,
    );
  });

  ipcMain.handle("test-and-apply-peon-provider", async (_event, value: unknown) => {
    const selection = normalizePeonSelectionInput(value, persistedOllamaBaseUrl());
    appliedPeonState = await peonTransaction.apply(selection);
    return appliedPeonState;
  });

  ipcMain.handle("get-applied-peon-provider", async () => {
    appliedPeonState = await peonTransaction.getApplied();
    return appliedPeonState;
  });

  ipcMain.handle("save-peon-selection", async (_event, value: unknown): Promise<PeonSelectionSaveResult> => {
    const selection = normalizePeonSelectionInput(value, persistedOllamaBaseUrl());
    const result = await peonTransaction.save(selection, async () => {
      await enqueueSettingsWrite(() => {
        const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
        const nextSettings = settingsWithPeonSelection(baseSettings, selection);
        writeSettings(app.getPath("userData"), nextSettings);
        currentSettings = nextSettings;
      });
    });
    if (!result.ok) return result;
    return { ok: true, settings: rendererSettings(currentSettings ?? readSettings(app.getPath("userData"))) };
  });

  // Compatibility IPC for existing renderer consumers. Discovery is
  // connectivity/model-listing only; it never runs Peon inference or mutates
  // the staged Apply transaction.
  ipcMain.handle("get-provider-models", async (_event, providerId: string) => {
    const ollamaBaseUrl = providerId === "ollama" ? persistedOllamaBaseUrl() : undefined;
    const cacheKey = providerModelCacheKey(providerId, ollamaBaseUrl);
    if (providerModels.has(cacheKey)) {
      return providerModels.get(cacheKey)!;
    }
    try {
      return await runtime.runReady(async ({ port, isCurrent }) => {
        const catalog = await discoverProviderModels(port, providerId, ollamaBaseUrl);
        if (isCurrent()) providerModels.set(cacheKey, catalog);
        return catalog;
      });
    } catch {
      // Fall through to empty
    }
    return { models: [] };
  });

  ipcMain.handle("read-git-changes", async (_event, request: unknown) => {
    validateGitChangesRequest(request);
    return runtime.runReady(({ port, token, signal }) =>
      readGitChanges(sidecarUrl(port), request, token, fetch, signal));
  });
  ipcMain.handle("session-title", async (_event, request: unknown) => {
    validateSessionTitleRequest(request);
    return runtime.runReady(({ port, token, signal }) =>
      requestSessionTitle(sidecarUrl(port), request, token, fetch, signal));
  });
  ipcMain.handle("get-plan-content", async (_event, sessionId: unknown, kind?: unknown, selectionId?: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    validatePlanDocumentKind(kind);
    validatePlanSelectionId(selectionId);
    return runtime.runReady(({ port, token, signal }) =>
      getSessionPlanContent(sidecarUrl(port), sessionId, token, fetch, signal, kind, selectionId));
  });
  ipcMain.handle("request-plan-review", async (_event, sessionId: unknown, kind?: unknown, selectionId?: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    validatePlanDocumentKind(kind);
    validatePlanSelectionId(selectionId);
    await runtime.runReady(({ port, token, signal }) =>
      requestSessionPlanReview(sidecarUrl(port), sessionId, token, fetch, signal, kind, selectionId));
  });
  ipcMain.handle("use-latest-plan-document", async (_event, sessionId: unknown, kind: unknown, selectionId?: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    validatePlanDocumentKind(kind);
    if (kind === undefined) throw new Error("A document kind is required.");
    validatePlanSelectionId(selectionId);
    await runtime.runReady(({ port, token, signal }) =>
      useLatestPlanDocument(sidecarUrl(port), sessionId, kind, token, fetch, signal, selectionId));
  });
  ipcMain.handle("reset-saved-plan-documents", async (_event, sessionId: unknown, recoveryId: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    validatePlanRecoveryId(recoveryId);
    await runtime.runReady(({ port, token, signal }) =>
      resetSavedPlanDocuments(sidecarUrl(port), sessionId, recoveryId, token, fetch, signal));
  });
  ipcMain.handle("select-terminal-plan", async (_event, sessionId: unknown, printedPath: unknown) => {
    if (typeof sessionId !== "string" || !sessionId || typeof printedPath !== "string" || !printedPath) throw new Error("Invalid plan selection.");
    return runtime.runReady(({ port, token, signal }) =>
      selectTerminalPlan(sidecarUrl(port), sessionId, printedPath, token, fetch, signal));
  });

  const integrationActionLabels: Record<"status" | "install" | "repair" | "uninstall", string> = {
    status: "check the integration status",
    install: "install the integration",
    repair: "repair the integration",
    uninstall: "uninstall the integration",
  };

  const STALE_WORKSPACE_FALLBACK_MESSAGE =
    "The workspace changed before this integration request could run. Reload the current workspace and retry.";

  async function getHarnessIntegrationStatus(harnessId: unknown): Promise<{ ok: true; status: unknown } | { ok: false; error: string }> {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    // Keep status results bound to the workspace and generation that requested them.
    try {
      return await runtime.runReady(async ({ port, signal }) => {
        const resp = await sidecarSend(
          port,
          `workspace/integrations/${encodeURIComponent(harnessId)}/status`,
          { signal },
        );
        if (resp.ok) {
          return { ok: true as const, status: await resp.json() };
        }
        const body = await resp.json().catch(() => ({ error: undefined }));
        return { ok: false as const, error: errorFromBody(body, "Couldn't check the integration status.") };
      }, { staleMessage: STALE_WORKSPACE_FALLBACK_MESSAGE });
    } catch (error) {
      if (error instanceof StaleBackendGenerationError) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  async function callGroupedIntegrationRoute(
    key: IntegrationKey,
    action: "status" | "install" | "repair" | "uninstall",
    expected?: IntegrationRevisionExpectation,
  ): Promise<
    | { ok: true; group: GroupedIntegrationStatus }
    | { ok: false; error: string; code?: string }
  > {
    if (!key.adapterId || !key.targetId) throw new Error("Invalid integration key.");
    try {
      return await runtime.runReady(async ({ port, signal }) => {
        const mutation = action !== "status";
        const resp = await sidecarSend(
          port,
          `workspace/integrations/${encodeURIComponent(key.adapterId)}/${encodeURIComponent(key.targetId)}/${action}`,
          mutation ? { method: "POST", jsonBody: expected, signal } : { signal },
        );
        if (resp.ok) {
          return { ok: true as const, group: await resp.json() as GroupedIntegrationStatus };
        }
        const body = await resp.json().catch(() => ({ error: undefined, code: undefined })) as {
          error?: string;
          code?: string;
        };
        return {
          ok: false as const,
          error: body.error ?? `Couldn't ${integrationActionLabels[action]}.`,
          ...(body.code ? { code: body.code } : {}),
        };
      }, { staleMessage: STALE_WORKSPACE_FALLBACK_MESSAGE });
    } catch (error) {
      if (error instanceof StaleBackendGenerationError) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE, code: "workspace_changed" };
      }
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  function toGroupedIntegrationStatusResult(
    response: Awaited<ReturnType<typeof callGroupedIntegrationRoute>>,
  ): GroupedIntegrationStatusResult {
    return response.ok
      ? response
      : { ok: false, error: response.error, ...(response.code ? { code: response.code } : {}) };
  }

  ipcMain.handle("get-harness-integration-status", async (_event, harnessId: unknown) =>
    getHarnessIntegrationStatus(harnessId));

  ipcMain.handle(
    "get-grouped-harness-integration-status",
    async (_event, adapterId: unknown, targetId: unknown): Promise<GroupedIntegrationStatusResult> => {
      if (typeof adapterId !== "string" || !adapterId || typeof targetId !== "string" || !targetId) {
        throw new Error("Invalid integration key.");
      }
      return toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(
        { adapterId, targetId },
        "status",
      ));
    },
  );

  // install/uninstall intentionally have no direct IPC channel: hook-mutating
  // routes are reachable only through a confirmed orchestrator — the batched
  // save ("save-active-harnesses-with-integrations") or the per-row reconcile
  // ("reconcile-harness-integration", which runs the same plan/confirm/mutate
  // pipeline for one integration key) — never from the renderer alone.

  async function confirmMutations(planned: PlannedIntegrationMutation[]): Promise<boolean> {
    if (!mainWindow) return false;

    const hasExecutableCodeWarning = planned.some((entry) => entry.confirmation?.executableCodeWarning);
    const lines = planned.map((entry) => {
      const label = entry.operation === "uninstall" ? "Remove" : entry.operation === "repair" ? "Repair" : "Install";
      const consumerNames = entry.consumerHarnessNames.join(", ");
      const toolName = entry.confirmation?.toolName ?? consumerNames;
      const paths = entry.confirmation?.relativePaths.length ? ` (${entry.confirmation.relativePaths.join(", ")})` : "";
      return `• ${label} ${toolName} [${consumerNames}]${paths}`;
    });
    const detail = [
      lines.join("\n"),
      hasExecutableCodeWarning
        ? "\nOne or more of these integrations installs a hook file that OrkWorks executes automatically."
        : "",
    ].filter(Boolean).join("\n");

    const { response } = await dialog.showMessageBox(mainWindow, {
      type: hasExecutableCodeWarning ? "warning" : "question",
      buttons: ["Cancel", "Confirm"],
      defaultId: 0,
      cancelId: 0,
      title: "Update coding tool integrations",
      message: "OrkWorks will change hook files for the following coding tools:",
      detail,
    });

    return response === 1;
  }

  ipcMain.handle("save-active-harnesses-with-integrations", async (_event, ids: unknown): Promise<ActiveHarnessSaveResult> => {
    if (!Array.isArray(ids) || ids.some((id) => typeof id !== "string" || !id)) {
      throw new Error("Invalid active harness IDs.");
    }

    return saveActiveHarnessesWithIntegrations(ids, {
      captureWorkspaceGuard: () => runtime.getWorkspaceGuard(),
      persistActiveHarnesses,
      listHarnesses: fetchHarnessesForSave,
      getGroupedIntegrationStatus: async (key) =>
        toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "status")),
      installGroupedIntegration: async (key, expected) =>
        toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "install", expected)),
      repairGroupedIntegration: async (key, expected) =>
        toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "repair", expected)),
      confirmMutations,
      uninstallGroupedIntegration: async (key, expected) =>
        toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "uninstall", expected)),
    });
  });

  ipcMain.handle(
    "enable-harness-integration-immediate",
    async (_event, ids: unknown, adapterId: unknown, targetId: unknown): Promise<ActiveHarnessSaveResult> => {
      if (!Array.isArray(ids) || ids.some((id) => typeof id !== "string" || !id)) {
        throw new Error("Invalid active harness IDs.");
      }
      if (typeof adapterId !== "string" || !adapterId || typeof targetId !== "string" || !targetId) {
        throw new Error("Invalid integration key.");
      }

      return enableHarnessImmediate(ids, { adapterId, targetId }, {
        captureWorkspaceGuard: () => runtime.getWorkspaceGuard(),
        persistActiveHarnesses,
        listHarnesses: fetchHarnessesForSave,
        getGroupedIntegrationStatus: async (key) =>
          toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "status")),
        installGroupedIntegration: async (key, expected) =>
          toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "install", expected)),
        repairGroupedIntegration: async (key, expected) =>
          toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "repair", expected)),
        uninstallGroupedIntegration: async (key, expected) =>
          toGroupedIntegrationStatusResult(await callGroupedIntegrationRoute(key, "uninstall", expected)),
        confirmMutations,
      });
    },
  );

  function editableCustomDefinition(harness: ElectronHarnessConfig): Record<string, unknown> {
    const {
      origin: _origin,
      profile: _profile,
      sessionSignals: _sessionSignals,
      integration: _integration,
      ...definition
    } = harness as ElectronHarnessConfig & Record<string, unknown>;
    return definition;
  }

  function effectiveHarnessFromMutationResponse(body: unknown): ElectronHarnessConfig | null {
    if (!body || typeof body !== "object") return null;
    const entry = (body as { harness?: unknown }).harness;
    if (!entry || typeof entry !== "object") return null;
    const definition = (entry as { definition?: unknown }).definition;
    return (definition && typeof definition === "object" ? definition : entry) as ElectronHarnessConfig;
  }

  // PUT/DELETE /harnesses/:id (crates/orkworksd/src/http/harness_handlers.rs)
  // Both controls read the current revision first. Built-ins send a narrow
  // BuiltinPatch; custom harnesses send a complete editable definition while
  // deliberately omitting derived compatibility bindings.
  async function setHarnessCommandOverride(harnessId: unknown, commandPath: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    if (typeof commandPath !== "string" || !commandPath.trim()) throw new Error("Invalid command path.");
    try {
      return await runtime.runReady(async ({ port, signal }) => {
        const snapshot = await fetchHarnessesFromSidecar(port, signal);
        const harness = snapshot.harnesses.find((candidate) => candidate.id === harnessId);
        if (!harness) return { ok: false, error: "Coding tool was not found." };
        if (!snapshot.documentRevision) return { ok: false, error: "Coding tool configuration has no revision." };
        const isCustom = harness.origin === "custom";
        const definition = isCustom ? editableCustomDefinition(harness) : undefined;
        if (definition && typeof definition.launch === "object" && definition.launch !== null) {
          (definition.launch as { command?: string }).command = commandPath.trim();
        }
        const resp = await sidecarSend(port, `harnesses/${encodeURIComponent(harnessId)}`, {
          method: "PUT",
          jsonBody: {
            kind: isCustom ? "CustomReplace" : "BuiltinPatch",
            ...(isCustom
              ? { definition }
              : { patch: { launch: { command: commandPath.trim() } } }),
            expectedRevision: snapshot.documentRevision,
          },
          signal,
        });
        if (resp.ok) {
          const updated = effectiveHarnessFromMutationResponse(await resp.json());
          return updated ? { ok: true, harness: updated } : { ok: false, error: "Malformed harness update response." };
        }
        return { ok: false, error: await parseErrorBody(resp, "Couldn't set the custom path.") };
      }, { staleMessage: STALE_WORKSPACE_FALLBACK_MESSAGE });
    } catch (error) {
      if (error instanceof StaleBackendGenerationError) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  async function clearHarnessCommandOverride(harnessId: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      return await runtime.runReady(async ({ port, signal }) => {
        const snapshot = await fetchHarnessesFromSidecar(port, signal);
        const harness = snapshot.harnesses.find((candidate) => candidate.id === harnessId);
        if (!harness) return { ok: false, error: "Coding tool was not found." };
        if (harness.origin === "custom") {
          return { ok: false, error: "Custom harness commands are edited in the configuration JSON." };
        }
        const resp = await sidecarSend(port, `harnesses/${encodeURIComponent(harnessId)}`, {
          method: "DELETE",
          jsonBody: { expectedRevision: snapshot.documentRevision },
          signal,
        });
        if (resp.ok) {
          return { ok: true };
        }
        return { ok: false, error: await parseErrorBody(resp, "Couldn't clear the custom path.") };
      }, { staleMessage: STALE_WORKSPACE_FALLBACK_MESSAGE });
    } catch (error) {
      if (error instanceof StaleBackendGenerationError) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  ipcMain.handle("set-harness-command-override", async (_event, harnessId: unknown, commandPath: unknown) =>
    setHarnessCommandOverride(harnessId, commandPath));

  ipcMain.handle("clear-harness-command-override", async (_event, harnessId: unknown) =>
    clearHarnessCommandOverride(harnessId));

  ipcMain.handle("open-workspace", async () => {
    const result = await runtime.pickWorkspace(async () => {
      const selection = await dialog.showOpenDialog({
        properties: ["openDirectory"],
        title: "Select Workspace",
      });
      if (selection.canceled || selection.filePaths.length === 0) return null;
      return selection.filePaths[0];
    });
    if (!result || !result.ok) return null;
    return result.workspace;
  });

  ipcMain.handle("get-workspace-history", () =>
    toWorkspaceHistorySnapshot(readWorkspaceMemory(app.getPath("userData"))));

  ipcMain.handle("pin-workspace-path", (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    return toWorkspaceHistorySnapshot(pinWorkspacePath(app.getPath("userData"), path));
  });

  ipcMain.handle("unpin-workspace-path", (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    return toWorkspaceHistorySnapshot(unpinWorkspacePath(app.getPath("userData"), path));
  });

  ipcMain.handle("forget-workspace-path", async (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    const result = await forgetRememberedWorkspaceWithNavigation(app.getPath("userData"), path, workspaceNavigationMemory,
      () => confirmShellMemoryRebuild("navigation", mainWindow,
        (owner, options) => dialog.showMessageBox(owner, options), () => workspaceNavigationMemory.rebuild(true)));
    if (result.cancelled) return toWorkspaceHistorySnapshot(result.history);
    if (result.navigation && !result.navigation.ok) {
      throw new Error("Workspace navigation state could not be cleared. Try removing the workspace shortcut again.");
    }
    return toWorkspaceHistorySnapshot(result.history);
  });

  ipcMain.handle("open-remembered-workspace", async (_event, path: unknown) => {
    if (typeof path !== "string") return null;
    if (runtime.getWorkspacePath() === path) return null;
    const result = await runtime.openWorkspace(path);
    // Some failure paths inside switchWorkspaceInternal (e.g. invalid_destination,
    // hit when a remembered path has since been moved/deleted) return a failure
    // without ever publishing a workspace-switch event, so the renderer's
    // workspaceSwitchDiagnostic banner would never appear. Throwing here routes
    // every failure through the renderer's existing catch-and-toast handling in
    // handleOpenRememberedWorkspace instead of silently no-oping.
    if (!result.ok) {
      // Pre-sidecar validation failures (missing/inaccessible directories)
      // never reach restoreWorkspace's removeFromHistory handling, so forget
      // the stale shortcut here before surfacing the error.
      if (result.failure.code === "invalid_destination") {
        const forgotten = await forgetRememberedWorkspaceWithNavigation(app.getPath("userData"), path, workspaceNavigationMemory);
        if (forgotten.navigation && !forgotten.navigation.ok) {
          console.warn("[main] invalid destination navigation could not be removed", forgotten.navigation.diagnostic);
        }
      }
      throw new Error(result.failure.message);
    }
    return result.workspace;
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

  if (app.isPackaged) {
    const { autoUpdater } = await import("electron-updater");
    const adapter = createElectronUpdateEngine(autoUpdater);
    updateService = createUpdateService({
      isPackaged: true,
      platform: process.platform,
      currentVersion: app.getVersion(),
      createEngine: () => adapter.engine,
      now: () => new Date().toISOString(),
      querySessions: () => runtime.runReady(async ({ port, signal }) => {
        const sessions = await listSessions(port, signal);
        return sessions.filter((session) => session.lifecycle === "alive");
      }),
      verifyCandidate: adapter.verifyCandidate,
      confirmInstall: async ({ liveSessionCount }) => {
        const detail = liveSessionCount === null
          ? "OrkWorks could not confirm whether live terminal sessions are running. Restarting may interrupt them."
          : liveSessionCount === 0
            ? "No live terminal sessions were found."
            : `Restarting will interrupt ${liveSessionCount} live terminal session${liveSessionCount === 1 ? "" : "s"}.`;
        const options = {
          type: "warning" as const,
          title: "Restart and install update?",
          message: "Restart OrkWorks and install the downloaded update?",
          detail,
          buttons: ["Cancel", "Restart and install"],
          defaultId: 0,
          cancelId: 0,
          noLink: true,
        };
        const result = mainWindow && !mainWindow.isDestroyed()
          ? await dialog.showMessageBox(mainWindow, options)
          : await dialog.showMessageBox(options);
        return result.response === 1;
      },
      stopSidecar: () => runtime.stopSidecarForUpdate(10_000),
      restartSidecar: () => runtime.restartSidecarAfterUpdate(app.getPath("home")),
      runInstall: (install) => runtime.runUpdate(async () => { await install(); }),
    });
    registerUpdateIpc(updateService, ipcMain);
  }

  createWindow(clearSessionNotifications);
  applyMenu(createMenu(currentSettings));

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow(clearSessionNotifications);
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

function killSidecar(): void {
  updateService = null;
  workspaceRuntime?.dispose();
  workspaceRuntime = null;
}

function requestQuit(): void {
  void (workspaceRuntime?.quit() ?? Promise.resolve({ ok: true as const, state: "picker" as const, generation: 0 }))
    .then((result) => {
      if (!result.ok) {
        // The runtime has already published the unresolved diagnostic. Keep
        // the app and retry path alive so a later quit can try cleanup again.
        quitInProgress = false;
        return;
      }
      killSidecar();
      quitBypass = true;
      app.quit();
    })
    .catch(() => {
      // An unexpected runtime rejection is also unsafe to finalize. Keep
      // the app alive; the lifecycle event remains the source of truth.
      quitInProgress = false;
    });
}

app.on("before-quit", (event) => {
  if (quitBypass) {
    quitBypass = false;
    return;
  }
  event.preventDefault();
  if (quitInProgress) return;
  quitInProgress = true;
  requestQuit();
});

process.on("SIGTERM", () => {
  if (!quitInProgress) {
    quitInProgress = true;
    requestQuit();
  }
});

process.on("SIGINT", () => {
  if (!quitInProgress) {
    quitInProgress = true;
    requestQuit();
  }
});
