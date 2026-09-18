import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme, shell } from "electron";
import { spawn } from "child_process";
import { createHash, randomBytes } from "crypto";
import { existsSync, readFileSync } from "fs";
import type { AppUpdater, NsisUpdater, UpdateInfo } from "electron-updater";
import { KnowledgeUpdates, synchronizeKnowledge } from "./knowledgeUpdates";
import { taskmasterRequest } from "./taskmasterSettings";
import { approveInferenceAdapter, readInferenceTrust, revokeInferenceAdapter, type TrustContext } from "./inferenceTrust";
import * as path from "path";
import { pathToFileURL } from "url";
import { getDevSidecarPath, getPackagedSidecarPath } from "./paths";
import { accessibleWorkspaceDirectoryPath, canonicalWorkspacePath, readWorkspaceMemory, rememberWorkspacePath, forgetWorkspacePath, type WorkspaceMemoryDiagnostic } from "./workspaceMemory";
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";
import type { AppSettings } from "./settingsMemory";
import { DEFAULT_HOTKEYS, DEFAULT_RETENTION, loadSettingsForStartup, normalizeDebugSettings, normalizeProviderSettings, normalizeRetention, providerDefinitionsForStoredSettings, readSettings, settingsWithHotkeys, settingsWithPeonSelection, validateHotkeys, writeSettings } from "./settingsMemory";
import { providerSettingsSyncError, pushProviderSettings } from "./providerSettingsSync";
import type { PeonAppliedState, PeonProviderVerificationResponse, PeonSelection, ProviderApplyStatus, ProviderDefinition, ProviderId, ProviderModelOption, ProviderSettings } from "./providerTypes";
import { createPeonSelectionTransaction, normalizePeonSelectionInput, peonErrorFromBody, type PeonSelectionTransaction } from "./peonSelectionTransaction";
import { buildMenuTemplate } from "./menuTemplate";
import { getSessionPlanContent, requestSessionPlanReview, selectTerminalPlan } from "./planOpener";
import { configureExternalLinks, openExternalLink } from "./externalLinks";
import { createSidecarLifecycle, type SidecarLifecycle, type SidecarProcess, type SidecarState } from "./sidecarLifecycle";
import { channelForVersion, createUpdateService, type UpdateCandidate, type UpdateEngine, type UpdateEngineEvent, type UpdateService } from "./updateService";
import { createBackendRestorationCoordinator, WorkspaceRestorationFailure, type BackendRestorationCoordinator } from "./backendRestoration";
import { buildWorkspaceRestoreRequest, parseWorkspaceRestoreResponse } from "./workspaceRestore";
import { workspaceHistoryPath } from "./workspaceRestore";
import type { BackendLifecycleEvent, BackendLifecycleWorkspace, BackendRetryResult, InitialWorkspaceSnapshot, WorkspaceHistoryDiagnostic } from "./backendLifecycleEvent";
import { createWorkspaceSwitchCoordinator, WorkspaceSwitchError, type WorkspaceSwitchCoordinator, type WorkspaceSwitchEvent } from "./workspaceSwitchCoordinator";
import { sanitizeBackendLifecycleFailure } from "./backendLifecycleFailure";
import { rendererConsoleDiagnostic, rendererConsoleLevel, rendererOrigin, sanitizeRendererDiagnosticMessage } from "./rendererDiagnostic";
import { recoveryDocumentUrl } from "./rendererRecoveryDocument";
import { createRecoveryDocumentGuard, isSupersededNavigation } from "./rendererRecoveryState";
import {
  enableHarnessImmediate,
  isStale,
  saveActiveHarnessesWithIntegrations,
  type ActiveHarnessSaveResult,
  type ElectronHarnessConfig,
  type GroupedIntegrationStatus,
  type GroupedIntegrationStatusResult,
  type IntegrationKey,
  type IntegrationRevisionExpectation,
  type IntegrationStatus,
  type IntegrationStatusResult,
  type PlannedIntegrationMutation,
} from "./activeHarnessIntegration";

import { getWindowChromeOptions } from "./windowChrome";

app.setName("OrkWorks");

let mainWindow: BrowserWindow | null = null;
let sidecarLifecycle: SidecarLifecycle | null = null;
let backendRestoration: BackendRestorationCoordinator<BackendLifecycleWorkspace> | null = null;
let updateService: UpdateService | null = null;
let workspaceSwitchCoordinator: WorkspaceSwitchCoordinator<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic> | null = null;
let quitInProgress = false;
let quitBypass = false;
let workspacePath: string | null = null;
let workspaceDisplayPath: string | null = null;
let pendingWorkspaceDisplayPath: string | null = null;
let menuPanelItems: Record<string, Electron.MenuItem> = {};
let currentSettings: AppSettings | null = null;
let providerModels: Map<string, string[]> = new Map();
const peonModelCatalogCache = new Map<string, { models: ProviderModelOption[]; observedAt: string }>();
let providerLabels: Record<string, string> = {};
let hotkeyCaptureActive = false;
let openPlanToken = "";
let settingsWriteQueue: Promise<void> = Promise.resolve();
const menuPanelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"];

type ElectronUpdateInfo = UpdateInfo & {
  tag?: string;
  downloadedFile?: string;
};

async function listSessions(baseUrl: string): Promise<Array<{ lifecycle?: string }>> {
  const response = await fetch(`${baseUrl}/sessions`);
  if (!response.ok) throw new Error(`list sessions failed: ${response.status}`);
  const sessions: unknown = await response.json();
  if (!Array.isArray(sessions)) throw new Error("list sessions failed: malformed response");
  return sessions as Array<{ lifecycle?: string }>;
}

function updateCandidate(info: ElectronUpdateInfo, channel: "latest" | "nightly", metadataUrl: string | null): UpdateCandidate {
  const tag = info.tag;
  const metadataFile = process.platform === "darwin" ? `${channel}-mac.yml` : `${channel}.yml`;
  if (channelForVersion(info.version) !== channel || tag !== `v${info.version}`
    || metadataUrl !== `https://github.com/Rambolarsen/orkworks/releases/download/${encodeURIComponent(tag)}/${metadataFile}`) {
    throw new Error("Update metadata does not match the requested channel, version, and release tag.");
  }
  const payload = info.files?.find(({ url }) => url.endsWith(process.platform === "darwin" ? ".zip" : ".exe"));
  if (!payload || !/^[A-Za-z0-9+/]{86}==$/.test(payload.sha512)) {
    throw new Error("Update metadata is missing a valid platform payload checksum.");
  }
  // UpdateDownloadedEvent extends UpdateInfo with a local completion path.
  // Hash the same release metadata at check, completion, and revalidation.
  const { downloadedFile: _downloadedFile, ...releaseMetadata } = info;
  const metadataDigest = createHash("sha256").update(JSON.stringify(releaseMetadata)).digest("hex");
  const releaseNotes = Array.isArray(info.releaseNotes)
    ? info.releaseNotes.map(({ note }) => note).filter((note): note is string => note !== null).join("\n\n") || null
    : info.releaseNotes ?? null;
  return {
    identity: {
      channel,
      version: info.version,
      tag,
      metadataUrl,
      metadataDigest: `sha256:${metadataDigest}`,
      payloadDigest: `sha512:${payload.sha512}`,
    },
    releaseNotes,
    publishedAt: info.releaseDate || null,
  };
}

function sameUpdateCandidate(left: UpdateCandidate, right: UpdateCandidate): boolean {
  return left.identity.channel === right.identity.channel
    && left.identity.version === right.identity.version
    && left.identity.tag === right.identity.tag
    && left.identity.metadataUrl === right.identity.metadataUrl
    && left.identity.metadataDigest === right.identity.metadataDigest
    && left.identity.payloadDigest === right.identity.payloadDigest;
}

function createElectronUpdateEngine(autoUpdater: AppUpdater): {
  engine: UpdateEngine;
  verifyCandidate(candidate: UpdateCandidate): Promise<boolean>;
} {
  const updaterSettings = {
    autoDownload: false,
    autoInstallOnAppQuit: false,
    allowDowngrade: false,
  };
  Object.assign(autoUpdater, updaterSettings);
  autoUpdater.setFeedURL({ provider: "github", owner: "Rambolarsen", repo: "orkworks" });
  autoUpdater.requestHeaders = { "Cache-Control": "no-cache" };
  autoUpdater.disableWebInstaller = true;

  const listeners = new Set<(event: UpdateEngineEvent) => void>();
  let channel: "latest" | "nightly" = "latest";
  let downloadedCandidate: UpdateCandidate | null = null;
  let metadataUrl: string | null = null;
  let signatureVerified = false;
  // Observe the public NSIS verifier without replacing or bypassing its verdict.
  // Cached downloads that skip it cannot establish verification in this process.
  const windowsUpdater = autoUpdater as AppUpdater & Partial<Pick<NsisUpdater, "verifyUpdateCodeSignature">>;
  if (process.platform === "win32" && typeof windowsUpdater.verifyUpdateCodeSignature === "function") {
    let verifyingPublishers: string[] | null = null;
    let verificationWarning = false;
    const logger = autoUpdater.logger;
    autoUpdater.logger = {
      info: (message) => logger?.info(message),
      error: (message) => logger?.error(message),
      debug: (message) => logger?.debug?.(message),
      warn: (message) => {
        // The release pipeline pins the certificate SimpleName (CN). This exact
        // pinned-verifier message means a successful match, not skipped checks.
        if (verifyingPublishers !== null && !verifyingPublishers.some((publisher) => message ===
          `Signature validated using only CN ${publisher}. Please add your full Distinguished Name (DN) to publisherNames configuration`)) {
          verificationWarning = true;
        }
        logger?.warn(message);
      },
    };
    const verify = windowsUpdater.verifyUpdateCodeSignature.bind(autoUpdater);
    windowsUpdater.verifyUpdateCodeSignature = async (publishers, file) => {
      signatureVerified = false;
      if (!publishers.length || publishers.some((publisher) => !publisher.trim())) {
        return "Update signature verification requires an expected publisher.";
      }
      verificationWarning = false;
      verifyingPublishers = publishers;
      try {
        const failure = await verify(publishers, file);
        // The pinned verifier can warn and return null after skipping validation
        // (e.g. unsupported PowerShell or missing path). Only the explicit
        // successful SimpleName message above is exempt from failing closed.
        signatureVerified = failure === null && !verificationWarning;
        return failure ?? (signatureVerified ? null : "Windows signature verification could not be established without warnings.");
      } finally {
        verifyingPublishers = null;
      }
    };
  }
  // GitHubProvider falls back to latest metadata on *any* prerelease metadata
  // error. Cancel that request in the updater's own public Electron session.
  autoUpdater.netSession.webRequest.onBeforeRequest({
    urls: ["https://github.com/Rambolarsen/orkworks/releases/download/*"],
  }, ({ url }, callback) => {
    const parsed = new URL(url);
    if (!parsed.pathname.endsWith(".yml")) return callback({});
    const expected = process.platform === "darwin" ? `${channel}-mac.yml` : `${channel}.yml`;
    const allowed = parsed.pathname.endsWith(`/${expected}`);
    if (allowed) metadataUrl = url;
    callback({ cancel: !allowed });
  });
  let operationQueue: Promise<void> = Promise.resolve();
  const emit = (event: UpdateEngineEvent): void => {
    for (const listener of listeners) listener(event);
  };
  const serialize = <T>(operation: () => Promise<T>): Promise<T> => {
    const result = operationQueue.then(operation, operation);
    operationQueue = result.then(() => undefined, () => undefined);
    return result;
  };

  const engine: UpdateEngine = {
    // NSIS schedules quit before async spawn failure is known and exposes no
    // supported latch reset/retry handshake. Block before any sidecar effects.
    installationUnavailableReason: process.platform === "win32"
      ? "Windows installation is unavailable: installer failure cannot be recovered safely through the public updater API. Install a signed release manually."
      : "Installation is unavailable: native verification cannot be completed safely before shutdown. Install a signed release manually.",
    get autoDownload() { return autoUpdater.autoDownload; },
    set autoDownload(value) { autoUpdater.autoDownload = value; },
    get autoInstallOnAppQuit() { return autoUpdater.autoInstallOnAppQuit; },
    set autoInstallOnAppQuit(value) { autoUpdater.autoInstallOnAppQuit = value; },
    get allowDowngrade() { return autoUpdater.allowDowngrade; },
    set allowDowngrade(value) { autoUpdater.allowDowngrade = value; },
    get allowPrerelease() { return autoUpdater.allowPrerelease; },
    set allowPrerelease(value) { autoUpdater.allowPrerelease = value; },
    get channel() { return channel; },
    set channel(value) {
      channel = value;
      autoUpdater.channel = value;
      autoUpdater.allowDowngrade = false;
    },
    onEvent(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    checkForUpdates(operationId) {
      return serialize(async () => {
        metadataUrl = null;
        const operation = { id: operationId, channel, finished: false };
        const cleanup = () => {
          autoUpdater.removeListener("update-available", onAvailable);
          autoUpdater.removeListener("update-not-available", onNotAvailable);
          autoUpdater.removeListener("error", onError);
        };
        const finish = (event: UpdateEngineEvent) => {
          if (operation.finished) return;
          operation.finished = true;
          cleanup();
          emit(event);
        };
        const onAvailable = (info: ElectronUpdateInfo) => {
          if (operation.finished) return;
          try {
            finish({ type: "update-available", operationId: operation.id,
              candidate: updateCandidate(info, operation.channel, metadataUrl) });
          } catch (error) {
            onError(error instanceof Error ? error : new Error(String(error)));
          }
        };
        const onNotAvailable = () => finish({ type: "update-not-available", operationId: operation.id });
        const onError = (error: Error) => finish({
          type: "error",
          operation: "check",
          operationId: operation.id,
          message: error.message,
        });
        autoUpdater.on("update-available", onAvailable);
        autoUpdater.on("update-not-available", onNotAvailable);
        autoUpdater.on("error", onError);
        try {
          await autoUpdater.checkForUpdates();
        } catch (error) {
          onError(error instanceof Error ? error : new Error(String(error)));
          throw error;
        } finally {
          operation.finished = true;
          cleanup();
        }
      });
    },
    downloadUpdate(operationId) {
      return serialize(async () => {
        downloadedCandidate = null;
        signatureVerified = false;
        let completedCandidate: UpdateCandidate | null = null;
        const operation = { id: operationId, channel, finished: false };
        const cleanup = () => {
          autoUpdater.removeListener("download-progress", onProgress);
          autoUpdater.removeListener("update-downloaded", onDownloaded);
          autoUpdater.removeListener("error", onError);
        };
        const finish = (event: UpdateEngineEvent) => {
          if (operation.finished) return;
          operation.finished = true;
          cleanup();
          emit(event);
        };
        const onProgress = (progress: { percent: number; transferred: number; total: number }) => {
          if (operation.finished) return;
          emit({
            type: "download-progress",
            operationId: operation.id,
            percent: progress.percent,
            transferred: progress.transferred,
            total: progress.total,
          });
        };
        const onDownloaded = (info: ElectronUpdateInfo) => {
          if (!operation.finished) completedCandidate = updateCandidate(info, operation.channel, metadataUrl);
        };
        const onError = (error: Error) => finish({
          type: "error",
          operation: "download",
          operationId: operation.id,
          message: error.message,
        });
        autoUpdater.on("download-progress", onProgress);
        autoUpdater.on("update-downloaded", onDownloaded);
        autoUpdater.on("error", onError);
        try {
          await autoUpdater.downloadUpdate();
          if (!operation.finished && completedCandidate !== null) {
            downloadedCandidate = completedCandidate;
            finish({ type: "update-downloaded", operationId: operation.id, candidate: completedCandidate });
          }
        } catch (error) {
          onError(error instanceof Error ? error : new Error(String(error)));
          throw error;
        } finally {
          operation.finished = true;
          cleanup();
        }
      });
    },
    async quitAndInstall() {
      // Also fail closed if a caller bypasses the service's availability guard.
      throw new Error(engine.installationUnavailableReason!);
    },
  };

  return {
    engine,
    verifyCandidate: (candidate) => serialize(async () => {
      if (process.platform !== "win32" || !signatureVerified || downloadedCandidate === null
        || !sameUpdateCandidate(candidate, downloadedCandidate)) return false;
      metadataUrl = null;
      const current = await autoUpdater.checkForUpdates();
      return current !== null && sameUpdateCandidate(candidate,
        updateCandidate(current.updateInfo, channel, metadataUrl));
    }),
  };
}

function registerUpdateIpc(service: UpdateService): void {
  const updateSubscriptions = new Map<number, () => void>();
  ipcMain.handle("get-update-status", () => service.getStatus());
  ipcMain.handle("check-for-updates", () => service.check());
  ipcMain.handle("download-update", () => service.download());
  ipcMain.handle("request-update-install", () => service.requestInstall());
  ipcMain.on("subscribe-update-status", (event) => {
    const senderId = event.sender.id;
    updateSubscriptions.get(senderId)?.();
    const unsubscribe = service.subscribe((status) => event.sender.send("update-status", status));
    updateSubscriptions.set(senderId, unsubscribe);
    event.sender.once("destroyed", () => {
      if (updateSubscriptions.get(senderId) !== unsubscribe) return;
      updateSubscriptions.delete(senderId);
      unsubscribe();
    });
  });
}

function rendererSettings(settings: AppSettings): AppSettings & { defaultHotkeys: typeof DEFAULT_HOTKEYS } {
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
    // ERR_ABORTED marks a navigation superseded by another in-flight load
    // (dev-server reload, a racing loadURL, the recovery page's own retry) —
    // not a genuine failure, so the recovery document must not load.
    if (isSupersededNavigation(errorCode)) return;
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

function toWorkspaceHistoryDiagnostic(
  diagnostic: WorkspaceMemoryDiagnostic | null,
): WorkspaceHistoryDiagnostic | null {
  return diagnostic ? { code: diagnostic.code, message: diagnostic.message } : null;
}

app.whenReady().then(async () => {
  updateDockIcon();
  nativeTheme.on("updated", updateDockIcon);

  const appMemory = readWorkspaceMemory(app.getPath("userData"));
  if (appMemory.diagnostic) {
    console.warn("[main] workspace history diagnostic", appMemory.diagnostic.message);
  }
  const initialHistoryDiagnostic = toWorkspaceHistoryDiagnostic(appMemory.diagnostic);
  let currentHistoryDiagnostic = initialHistoryDiagnostic;
  workspacePath = null;
  currentSettings = loadSettingsForStartup(app.getPath("userData"));

  let latestBackendLifecycle: BackendLifecycleEvent = { state: "picker" };
  let lastBackendFailure = "The OrkWorks sidecar is unavailable.";
  let appliedPeonState: PeonAppliedState | null = null;
  let backendGeneration = 0;
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
  async function readTaskmasterSettings(payload?: unknown): Promise<Record<string, unknown>> {
    const generation = backendGeneration;
    const port = await restoration.getReadiness();
    if (generation !== backendGeneration) throw new Error("Workspace changed; reopen Recommendations settings");
    const result = await taskmasterRequest(port, openPlanToken, "settings", payload);
    if (generation !== backendGeneration) throw new Error("Workspace changed; reopen Recommendations settings");
    const settings = result.settings as { automaticKnowledgeUpdates?: boolean } | undefined;
    knowledgeUpdates.setEnabled(settings?.automaticKnowledgeUpdates !== false);
    return { ...result, knowledgeUpdate: knowledgeUpdates.status() };
  }
  const STALE_BACKEND_GENERATION_MESSAGE =
    "The workspace changed before this request could run. Reload the current workspace and retry.";
  const generationBoundRequestControllers = new Set<AbortController>();

  function cancelGenerationBoundRequests(): void {
    for (const controller of generationBoundRequestControllers) controller.abort();
    generationBoundRequestControllers.clear();
  }

  function assertCurrentReadyBackendGeneration(generation: number): void {
    if (generation !== backendGeneration || latestBackendLifecycle.state !== "ready") {
      throw new Error(STALE_BACKEND_GENERATION_MESSAGE);
    }
  }

  async function withReadyBackendGeneration<T>(
    operation: (port: number, token: string, signal: AbortSignal) => Promise<T>,
  ): Promise<T> {
    if (latestBackendLifecycle.state !== "ready") {
      throw new Error("Workspace transition is in progress");
    }
    const generation = backendGeneration;
    const port = await restoration.getReadiness();
    assertCurrentReadyBackendGeneration(generation);
    const token = openPlanToken;
    const controller = new AbortController();
    generationBoundRequestControllers.add(controller);
    try {
      const result = await operation(port, token, controller.signal);
      assertCurrentReadyBackendGeneration(generation);
      return result;
    } finally {
      generationBoundRequestControllers.delete(controller);
    }
  }

  function taskmasterRecommendationPath(id: string, action: "dismiss" | "accept"): string {
    if (!id) throw new Error("Invalid recommendation ID.");
    return `taskmaster/recommendations/${encodeURIComponent(id)}/${action}`;
  }

  async function taskmasterMutationRequest(
    port: number,
    token: string,
    resource: string,
    payload: unknown,
    signal: AbortSignal,
  ): Promise<unknown> {
    const response = await fetch(`http://127.0.0.1:${port}/${resource}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
      body: JSON.stringify(payload),
      signal: AbortSignal.any([signal, AbortSignal.timeout(15_000)]),
    });
    const body: unknown = await response.json().catch(() => ({}));
    if (!response.ok) {
      const message = body && typeof body === "object" && typeof (body as { error?: unknown }).error === "string"
        ? (body as { error: string }).error
        : `Taskmaster request failed (${response.status})`;
      throw new Error(message);
    }
    return body;
  }

  function normalizeRecommendationAcceptOptions(value: unknown): { sessionId: string; prompt?: string } {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      throw new Error("Invalid recommendation handoff.");
    }
    const input = value as { sessionId?: unknown; prompt?: unknown };
    if (typeof input.sessionId !== "string" || !input.sessionId) {
      throw new Error("Invalid recommendation handoff session.");
    }
    if (input.prompt !== undefined && typeof input.prompt !== "string") {
      throw new Error("Invalid recommendation handoff prompt.");
    }
    return {
      sessionId: input.sessionId,
      ...(input.prompt === undefined ? {} : { prompt: input.prompt }),
    };
  }

  function normalizeDebugAttention(value: unknown): string {
    if (value === "working" || value === "idle" || value === "needs_you"
      || value === "blocked" || value === "failed" || value === "capped") return value;
    throw new Error("Invalid debug attention.");
  }

  let knowledgeSync: Promise<void> | null = null;
  async function inferenceTrustContext(): Promise<TrustContext> {
    const generation = backendGeneration;
    const port = await restoration.getReadiness();
    if (generation !== backendGeneration) throw new Error("Sidecar changed. Refresh executable approvals.");
    return { port, token: openPlanToken, isCurrent: () => generation === backendGeneration,
      confirm: async (detail) => {
        if (!mainWindow || mainWindow.isDestroyed()) return false;
        const result = await dialog.showMessageBox(mainWindow, {
          type: "warning", title: "Approve custom inference executable?", message: "Trust this executable with background inference context?", detail,
          buttons: ["Cancel", "Approve executable"], defaultId: 0, cancelId: 0, noLink: true,
        });
        return result.response === 1;
      },
    };
  }
  function refreshKnowledge(): Promise<void> {
    if (knowledgeSync) return knowledgeSync;
    const requestedGeneration = backendGeneration;
    knowledgeSync = (async () => {
      const generation = backendGeneration;
      const port = await restoration.getReadiness();
      const token = openPlanToken;
      const stillCurrent = () => generation === backendGeneration && token === openPlanToken;
      if (!stillCurrent()) return;
      const status = await taskmasterRequest(port, token, "settings");
      const settings = status.settings as { automaticKnowledgeUpdates?: boolean };
      knowledgeUpdates.setEnabled(settings.automaticKnowledgeUpdates !== false);
      await synchronizeKnowledge(knowledgeUpdates,
        (bundle) => taskmasterRequest(port, token, "knowledge", bundle), stillCurrent);
    })().catch((error: unknown) => {
      console.warn("[taskmaster] knowledge unavailable:", error instanceof Error ? error.message : "unknown error");
    }).finally(() => {
      knowledgeSync = null;
      if (requestedGeneration !== backendGeneration) void refreshKnowledge();
    });
    return knowledgeSync;
  }
  const knowledgeTimer = setInterval(() => { void refreshKnowledge(); }, 6 * 60 * 60 * 1000);
  knowledgeTimer.unref();
  app.once("before-quit", () => { clearInterval(knowledgeTimer); knowledgeUpdates.setEnabled(false); });
  let activeHarnessRevision = 0;
  // Main's copy of the persisted active-harness selection, kept in sync with
  // the sidecar: seeded from workspace restoration and updated only after a
  // confirmed PUT succeeds (never in the stale-skip branch below, which
  // returns without writing). Per-row integration reconcile reads this to
  // decide install/repair vs uninstall without trusting renderer state.
  let persistedActiveHarnessIds: string[] = [];

  function publishBackendLifecycle(event: BackendLifecycleEvent): void {
    latestBackendLifecycle = event;
    mainWindow?.webContents.send("orkworks:backend-lifecycle", event);
  }

  function rememberRestoredWorkspace(workspace: BackendLifecycleWorkspace | null): WorkspaceHistoryDiagnostic | null {
    const restoredPath = workspaceHistoryPath(workspacePath, workspace);
    if (!restoredPath) return currentHistoryDiagnostic;
    const canonicalPath = canonicalWorkspacePath(restoredPath);
    if (!canonicalPath) {
      currentHistoryDiagnostic = {
        code: "history_write_failed",
        message: "Workspace history could not be saved; the ready workspace was kept.",
      };
      console.warn("[main] workspace history was not updated", "workspace path could not be canonicalized");
      return currentHistoryDiagnostic;
    }
    try {
      const result = rememberWorkspacePath(app.getPath("userData"), canonicalPath);
      const diagnostic = result.diagnostic;
      currentHistoryDiagnostic = toWorkspaceHistoryDiagnostic(diagnostic);
      if (currentHistoryDiagnostic) {
        console.warn("[main] workspace history was not updated", diagnostic?.message);
      }
    } catch (error) {
      currentHistoryDiagnostic = {
        code: "history_write_failed",
        message: "Workspace history could not be saved; the ready workspace was kept.",
      };
      console.warn("[main] workspace history was not updated", error instanceof Error ? error.message : "unknown error");
    }
    return currentHistoryDiagnostic;
  }

  async function restoreWorkspace(port: number, signal: AbortSignal): Promise<BackendLifecycleWorkspace | null> {
    if (!workspacePath) return null;

    const displayPath = workspaceDisplayPath ?? workspacePath;
    const response = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(buildWorkspaceRestoreRequest(displayPath, workspacePath)),
      signal,
    });
    const restoreResult = await parseWorkspaceRestoreResponse(response);
    signal.throwIfAborted();
    if (!restoreResult.ok) {
      const rejectedPath = workspacePath;
      console.warn(`[main] remembered workspace path was rejected by the sidecar: ${rejectedPath}`);
      if (restoreResult.removeFromHistory) {
        try {
          const result = forgetWorkspacePath(app.getPath("userData"), rejectedPath);
          const diagnostic = result.diagnostic;
          currentHistoryDiagnostic = toWorkspaceHistoryDiagnostic(diagnostic);
          if (currentHistoryDiagnostic) {
            console.warn("[main] rejected workspace could not be removed from history", diagnostic?.message);
          }
        } catch (error) {
          console.warn("[main] rejected workspace could not be removed from history", error instanceof Error ? error.message : "unknown error");
        }
      }
      workspacePath = null;
      if (restoreResult.failureCode === "destination_conflict") {
        throw new WorkspaceSwitchError("destination_conflict", "Workspace is already owned by another sidecar.");
      }
      throw new WorkspaceRestorationFailure(restoreResult.status);
    }
    return restoreResult.workspace;
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

  async function parsePeonError(response: Response, fallback: string): Promise<Error> {
    const body = await response.json().catch(() => ({ error: undefined })) as unknown;
    return peonErrorFromBody(body, fallback);
  }

  async function persistActiveHarnesses(
    ids: string[],
    expectedActiveHarnessRevision: number,
  ): Promise<
    | { ok: true; activeHarnessRevision: number }
    | { ok: false; error: string; code?: string }
  > {
    const guard = { workspacePath, generation: backendGeneration, activeHarnessRevision };
    const port = await restoration.getReadiness();
    if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
      // Workspace switched mid-await: readiness now resolves to a different
      // sidecar than the one this save started against. Skip the write
      // rather than persisting the old workspace's selection into the new
      // one's backend; saveActiveHarnessesWithIntegrations independently
      // re-checks the guard right after this resolves and reports
      // stale_workspace, so this result is discarded either way.
      return { ok: true, activeHarnessRevision: expectedActiveHarnessRevision };
    }
    const response = await fetch(`http://127.0.0.1:${port}/workspace/active-harnesses`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ activeHarnessIds: ids, expectedActiveHarnessRevision }),
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
      // The workspace can switch while this PUT is in flight; the pre-fetch
      // guard check cannot catch that. If the response arrives against a
      // switched workspace, discard it the same way as the pre-fetch
      // stale-skip above — saveActiveHarnessesWithIntegrations re-checks the
      // guard and reports stale_workspace, so this result is discarded —
      // without letting the old workspace's selection or revision clobber
      // the state the new workspace's onReady seeded (and that per-row
      // integration reconcile reads back).
      if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
        return { ok: true, activeHarnessRevision: expectedActiveHarnessRevision };
      }
      activeHarnessRevision = savedActiveHarnessRevision;
      persistedActiveHarnessIds = ids;
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
  }

  async function fetchHarnessesForSave(): Promise<{ documentRevision: string | null; harnesses: ElectronHarnessConfig[] }> {
    const port = await restoration.getReadiness();
    const response = await fetch(`http://127.0.0.1:${port}/harnesses`);
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

  function persistedOllamaBaseUrl(): string | undefined {
    return currentSettings?.providers.peonSelection?.ollamaBaseUrl
      ?? currentSettings?.providers.ollamaBaseUrl;
  }

  async function syncSavedProviderSettings(port: number, signal: AbortSignal): Promise<void> {
    return enqueueSettingsWrite(async () => {
      const settings = currentSettings ?? readSettings(app.getPath("userData"));
      const providerResponse = await fetch(`http://127.0.0.1:${port}/providers`, { signal });
      if (!providerResponse.ok) throw new Error(`provider catalog sync failed: ${providerResponse.status}`);
      const providerBody = await providerResponse.json() as {
        providers?: unknown;
      };
      if (!Array.isArray(providerBody.providers)) throw new Error("provider catalog sync returned malformed data");
      const definitions: ProviderDefinition[] = providerBody.providers.map((entry) => {
        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
          throw new Error("provider catalog sync returned malformed entry");
        }
        const raw = entry as Record<string, unknown>;
        if (typeof raw.id !== "string" || typeof raw.label !== "string") {
          throw new Error("provider catalog sync returned malformed identity");
        }
        return {
          id: raw.id,
          label: raw.label,
          ...(typeof raw.harnessId === "string" ? { harnessId: raw.harnessId } : {}),
          ...(raw.origin === "builtin" || raw.origin === "override" || raw.origin === "custom" || raw.origin === "standalone"
            ? { origin: raw.origin }
            : {}),
        };
      });
      const normalizedProviders = normalizeProviderSettings(settings.providers, definitions);
      const nextSettings: AppSettings = { ...settings, providers: normalizedProviders };
      if (JSON.stringify(settings.providers) !== JSON.stringify(normalizedProviders)) {
        writeSettings(app.getPath("userData"), nextSettings);
        currentSettings = nextSettings;
      }
      const abortableFetch: typeof fetch = (input, init) => fetch(input, { ...init, signal });
      const result = await pushProviderSettings(
        `http://127.0.0.1:${port}`,
        normalizedProviders,
        abortableFetch,
      );
      signal.throwIfAborted();
      const syncError = providerSettingsSyncError(result);
      if (syncError) throw syncError;
      providerModels.clear();
    });
  }

  let persistedPeonRestoreController: AbortController | null = null;

  function cancelPersistedPeonSelectionRestore(): void {
    const controller = persistedPeonRestoreController;
    persistedPeonRestoreController = null;
    controller?.abort();
  }

  function restorePersistedPeonSelection(port: number): void {
    cancelPersistedPeonSelectionRestore();
    const selection = currentSettings?.providers.peonSelection;
    if (!selection) return;
    const generation = backendGeneration;
    const controller = new AbortController();
    persistedPeonRestoreController = controller;
    const isCurrentReady = (): boolean => !controller.signal.aborted
      && generation === backendGeneration
      && latestBackendLifecycle.state === "ready";
    void peonTransaction.syncPersistedSelection(selection, controller.signal, port)
      .then((applied) => {
        if (!isCurrentReady()) return;
        appliedPeonState = applied;
      })
      .catch((error: unknown) => {
        if (!isCurrentReady()) return;
        console.warn(`[main] failed to restore Peon selection: ${error instanceof Error ? error.message : "unknown error"}`);
      })
      .finally(() => {
        if (persistedPeonRestoreController === controller) persistedPeonRestoreController = null;
      });
  }

  const peonTransaction: PeonSelectionTransaction = createPeonSelectionTransaction({
    discover: async (provider, ollamaBaseUrl) => {
      const port = await restoration.getReadiness();
      const response = await fetch(`http://127.0.0.1:${port}/settings/providers/${encodeURIComponent(provider)}/models`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(ollamaBaseUrl ? { baseUrl: ollamaBaseUrl } : {}),
      });
      if (!response.ok) throw new Error("model discovery failed");
      return (await response.json() as { models: string[] }).models;
    },
    verify: async ({ provider, ollamaBaseUrl, generation, readyPort, signal }) => {
      const port = readyPort ?? await restoration.getReadiness();
      const body: { provider: string; generation: number; ollamaBaseUrl?: string } = { provider, generation };
      if (provider === "ollama") body.ollamaBaseUrl = ollamaBaseUrl ?? persistedOllamaBaseUrl();
      const response = await fetch(`http://127.0.0.1:${port}/settings/peon/provider/verify`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        signal,
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
    },
    apply: async ({ selection, generation, readyPort, signal, skipTest }) => {
      const port = readyPort ?? await restoration.getReadiness();
      const response = await fetch(`http://127.0.0.1:${port}/settings/peon/test-and-apply`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(skipTest ? { selection, generation, skipTest } : { selection, generation }),
        signal,
      });
      if (!response.ok) throw await parsePeonError(response, "Couldn't apply the Peon provider.");
      return await response.json();
    },
    getApplied: async (signal) => {
      const port = await restoration.getReadiness();
      const response = await fetch(`http://127.0.0.1:${port}/settings/peon/applied`, { signal });
      if (!response.ok) throw await parsePeonError(response, "Couldn't read the applied Peon provider.");
      return await response.json();
    },
  });

  const restoration = createBackendRestorationCoordinator<BackendLifecycleWorkspace>({
    setTimeout: (callback, delayMs) => setTimeout(callback, delayMs),
    clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
    onReady: (port, workspace) => {
      activeHarnessRevision = workspace?.activeHarnessRevision ?? 0;
      persistedActiveHarnessIds = workspace?.activeHarnessIds ?? [];
    },
    onFailure: (error) => {
      logBackendLifecycleFailure("restoration", error);
      if (workspaceSwitchCoordinator?.getState() === "opening") return;
      lastBackendFailure = sanitizeBackendLifecycleFailure(error);
      publishBackendLifecycle({ state: "failed", message: lastBackendFailure });
    },
    onStepFailure: (step, error) => {
      logBackendLifecycleFailure(`restoration:${step}`, error);
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
        // A failed generation is reported to the coordinator; recovery stays
        // explicit through retry-backend so replacement remains serialized.
        restoration.fail(new Error(lastBackendFailure));
      },
      onUnexpectedExit: (message) => {
        cancelPersistedPeonSelectionRestore();
        cancelGenerationBoundRequests();
        backendGeneration += 1;
        const failureMessage = sanitizeBackendLifecycleFailure(message);
        lastBackendFailure = failureMessage;
        restoration.cancel(new Error(failureMessage));
        workspaceSwitchCoordinator?.markUnresolved({
          code: "cleanup_failed",
          message: failureMessage,
        });
      },
      onState: (state: SidecarState) => {
        if (state === "starting") {
          cancelPersistedPeonSelectionRestore();
          cancelGenerationBoundRequests();
          backendGeneration += 1;
          restoration.beginGeneration();
        }
      },
    },
  });

  function publishWorkspaceSwitchEvent(event: WorkspaceSwitchEvent<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic>): void {
    if (event.state === "picker") {
      publishBackendLifecycle(event.failure ? { state: "picker", failure: event.failure } : { state: "picker" });
    } else if (event.state === "opening" || event.state === "closing") {
      publishBackendLifecycle({ state: event.state });
    } else if (event.state === "unresolved") {
      publishBackendLifecycle({ state: "unresolved", failure: event.failure });
    } else {
      publishBackendLifecycle({
        state: "ready",
        port: event.port,
        workspace: event.workspace,
        historyDiagnostic: event.historyDiagnostic,
      });
      restorePersistedPeonSelection(event.port);
      void refreshKnowledge();
    }
  }

  workspaceSwitchCoordinator = createWorkspaceSwitchCoordinator<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic>({
    initialWorkspacePath: null,
    validateDestination: (candidate) => {
      const identity = accessibleWorkspaceDirectoryPath(candidate);
      if (identity) pendingWorkspaceDisplayPath = candidate;
      return identity;
    },
    setWorkspacePath: (nextPath) => {
      workspacePath = nextPath;
      if (nextPath) {
        workspaceDisplayPath = pendingWorkspaceDisplayPath ?? nextPath;
        pendingWorkspaceDisplayPath = null;
      } else {
        workspaceDisplayPath = null;
      }
    },
    onCloseAdmission: () => {
      cancelPersistedPeonSelectionRestore();
      cancelGenerationBoundRequests();
      backendGeneration += 1;
    },
    closeCurrentRuntime: async () => {
      restoration.cancel(new Error("Workspace is closing"));
      await sidecarLifecycle?.stop();
    },
    startRuntime: async (nextPath, generation) => {
      workspacePath = nextPath;
      workspaceDisplayPath = pendingWorkspaceDisplayPath ?? nextPath;
      let lifecycleReadiness: Promise<number>;
      try {
        lifecycleReadiness = sidecarLifecycle!.start(nextPath);
        void lifecycleReadiness.catch(() => {});
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : "The sidecar did not become ready.";
        throw new WorkspaceSwitchError("readiness_failed", message);
      }
      const port = await lifecycleReadiness.catch((error: unknown) => {
        const message = error instanceof Error ? error.message : "The sidecar did not become ready.";
        throw new WorkspaceSwitchError("readiness_failed", message);
      });

      let restoredPort: number;
      try {
        restoredPort = await restoration.getReadiness();
      } catch (error: unknown) {
        if (error instanceof WorkspaceSwitchError) throw error;
        if (error instanceof WorkspaceRestorationFailure) {
          throw new WorkspaceSwitchError("restoration_failed", error.message);
        }
        const message = error instanceof Error ? error.message : "The destination workspace could not be restored.";
        throw new WorkspaceSwitchError("restoration_failed", message);
      }
      if (!workspaceSwitchCoordinator?.isCurrentGeneration(generation)) {
        throw new WorkspaceSwitchError("restoration_failed", "Workspace opening was superseded.");
      }
      const workspace = restoration.getRestoredWorkspace();
      if (!workspace) {
        throw new WorkspaceSwitchError("restoration_failed", "The destination workspace was not restored.");
      }
      return { port: restoredPort || port, workspace };
    },
    cleanupAttemptedRuntime: async () => {
      restoration.cancel(new Error("Attempted workspace runtime is being cleaned up"));
      workspacePath = null;
      workspaceDisplayPath = null;
      pendingWorkspaceDisplayPath = null;
      await sidecarLifecycle?.stop();
    },
    rememberWorkspace: (_path, workspace) => rememberRestoredWorkspace(workspace),
    publish: publishWorkspaceSwitchEvent,
  });

  ipcMain.handle("get-backend-lifecycle", () => {
    return latestBackendLifecycle;
  });

  ipcMain.handle("get-backend-url", async () => {
    const port = await restoration.getReadiness();
    return `http://127.0.0.1:${port}`;
  });

  ipcMain.handle("dismiss-taskmaster-recommendation", async (_event, id: unknown, reason: unknown) => {
    if (typeof id !== "string" || !id) throw new Error("Invalid recommendation ID.");
    if (reason !== undefined && typeof reason !== "string") throw new Error("Invalid dismissal reason.");
    await withReadyBackendGeneration(async (port, token, signal) => {
      await taskmasterMutationRequest(
        port,
        token,
        taskmasterRecommendationPath(id, "dismiss"),
        reason === undefined ? {} : { reason },
        signal,
      );
    });
  });

  ipcMain.handle("accept-taskmaster-recommendation", async (_event, id: unknown, options: unknown) => {
    if (typeof id !== "string" || !id) throw new Error("Invalid recommendation ID.");
    const normalized = normalizeRecommendationAcceptOptions(options);
    return withReadyBackendGeneration((port, token, signal) => taskmasterMutationRequest(
      port,
      token,
      taskmasterRecommendationPath(id, "accept"),
      normalized,
      signal,
    ));
  });

  ipcMain.handle("apply-debug-attention", async (_event, id: unknown, attention: unknown, message: unknown) => {
    if (typeof id !== "string" || !id) throw new Error("Invalid session ID.");
    const normalizedAttention = normalizeDebugAttention(attention);
    if (message !== undefined && typeof message !== "string") throw new Error("Invalid debug attention message.");
    await withReadyBackendGeneration(async (port, token, signal) => {
      const response = await fetch(`http://127.0.0.1:${port}/sessions/${encodeURIComponent(id)}/debug-injection`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
        body: JSON.stringify({ attention: normalizedAttention, message }),
        signal: AbortSignal.any([signal, AbortSignal.timeout(15_000)]),
      });
      if (!response.ok) throw new Error(`apply debug attention failed: ${response.status}`);
    });
  });

  ipcMain.handle("retry-backend", async (): Promise<BackendRetryResult> => {
    if (!workspaceSwitchCoordinator) throw new Error("Workspace lifecycle is unavailable");
    const result = await workspaceSwitchCoordinator.retry();
    if (result.ok) return { ok: true, state: "ready" };
    if (result.state === "unresolved") return { ok: false, state: "unresolved", failure: result.failure };
    if (!result.ok && result.failure.code === "invalid_destination") {
      publishBackendLifecycle({ state: "picker" });
    }
    return { ok: false, state: "picker", failure: result.failure };
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

  ipcMain.handle("get-initial-workspace", async (): Promise<InitialWorkspaceSnapshot> => ({
    // Workspace history is a picker hint, not proof that this process owns the
    // remembered directory after a restart. Native process-tree ownership is
    // not proven on every supported platform yet, so startup remains in the
    // picker until the user explicitly chooses a destination.
    workspace: null,
    historyDiagnostic: currentHistoryDiagnostic,
  }));

  ipcMain.handle("get-settings", async () => {
    currentSettings = readSettings(app.getPath("userData"));
    return rendererSettings(currentSettings);
  });
  ipcMain.handle("get-taskmaster-settings", () => readTaskmasterSettings());
  ipcMain.handle("get-inference-trust", async () => readInferenceTrust(await inferenceTrustContext()));
  ipcMain.handle("approve-inference-adapter", (_event, request: unknown) => enqueueSettingsWrite(async () => approveInferenceAdapter(request, await inferenceTrustContext())));
  ipcMain.handle("revoke-inference-adapter", (_event, request: unknown) => enqueueSettingsWrite(async () => revokeInferenceAdapter(request, await inferenceTrustContext())));
  ipcMain.handle("save-taskmaster-settings", (_event, payload: unknown) => enqueueSettingsWrite(async () => {
    const saved = await readTaskmasterSettings(payload);
    void refreshKnowledge();
    return saved;
  }));

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

  ipcMain.handle("save-provider-settings", async (_event, providers: ProviderSettings) => {
    let previousOllamaBaseUrl: string | undefined;
    const nextSettings = await enqueueSettingsWrite(() => {
      const baseSettings = readSettings(app.getPath("userData"));
      previousOllamaBaseUrl = baseSettings.providers.ollamaBaseUrl;
      const nextSettings: AppSettings = {
        ...baseSettings,
        version: 1,
        providers: normalizeProviderSettings({
          ...providers,
          revision: Math.max(baseSettings.providers.revision + 1, providers.revision),
        }, providerDefinitionsForStoredSettings(providers)),
      };
      writeSettings(app.getPath("userData"), nextSettings);
      currentSettings = nextSettings;
      return nextSettings;
    });

    const port = await restoration.getReadiness();
    const providerApplyStatus = await pushProviderSettings(`http://127.0.0.1:${port}`, nextSettings.providers);

    if (previousOllamaBaseUrl !== nextSettings.providers.ollamaBaseUrl) {
      providerModels.delete(providerModelCacheKey("ollama", previousOllamaBaseUrl));
      providerModels.delete(providerModelCacheKey("ollama", nextSettings.providers.ollamaBaseUrl));
    }

    return { ok: true, settings: rendererSettings(nextSettings), providerApplyStatus };
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

  ipcMain.handle("save-peon-selection", async (_event, value: unknown) => {
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
      return { models: providerModels.get(cacheKey)! };
    }
    try {
      const port = await restoration.getReadiness();
      const response = await fetch(`http://127.0.0.1:${port}/settings/providers/${encodeURIComponent(providerId)}/models`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(ollamaBaseUrl ? { baseUrl: ollamaBaseUrl } : {}),
      });
      if (!response.ok) throw new Error("model discovery failed");
      const models = (await response.json() as { models: string[] }).models;
      providerModels.set(cacheKey, models);
      return { models };
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
    return withReadyBackendGeneration((port, token, signal) =>
      getSessionPlanContent(`http://127.0.0.1:${port}`, sessionId, token, fetch, signal));
  });
  ipcMain.handle("request-plan-review", async (_event, sessionId: unknown) => {
    if (typeof sessionId !== "string" || !sessionId) throw new Error("Invalid session ID.");
    await withReadyBackendGeneration((port, token, signal) =>
      requestSessionPlanReview(`http://127.0.0.1:${port}`, sessionId, token, fetch, signal));
  });
  ipcMain.handle("select-terminal-plan", async (_event, sessionId: unknown, printedPath: unknown) => {
    if (typeof sessionId !== "string" || !sessionId || typeof printedPath !== "string" || !printedPath) throw new Error("Invalid plan selection.");
    await withReadyBackendGeneration((port, token, signal) =>
      selectTerminalPlan(`http://127.0.0.1:${port}`, sessionId, printedPath, token, fetch, signal));
  });

  const integrationActionLabels: Record<"status" | "install" | "repair" | "uninstall", string> = {
    status: "check the integration status",
    install: "install the integration",
    repair: "repair the integration",
    uninstall: "uninstall the integration",
  };

  const STALE_WORKSPACE_FALLBACK_MESSAGE =
    "The workspace changed before this integration request could run. Reload the current workspace and retry.";

  async function callIntegrationRoute(
    harnessId: unknown,
    action: "status" | "install" | "uninstall",
  ): Promise<{ ok: true; status: unknown } | { ok: false; error: string }> {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    // Same inner guard as persistActiveHarnesses: restoration.getReadiness()
    // may resolve to a different sidecar than the one this call started
    // against if the workspace switched mid-await. Fail closed rather than
    // sending the request to — for install/uninstall, mutating hook files
    // in — the wrong workspace. The save orchestrator independently
    // re-checks staleness after this resolves and reports stale_workspace,
    // so this failure result is superseded there either way.
    const guard = { workspacePath, generation: backendGeneration, activeHarnessRevision };
    try {
      const port = await restoration.getReadiness();
      if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
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

  async function callGroupedIntegrationRoute(
    key: IntegrationKey,
    action: "status" | "install" | "repair" | "uninstall",
    expected?: IntegrationRevisionExpectation,
  ): Promise<
    | { ok: true; group: GroupedIntegrationStatus }
    | { ok: false; error: string; code?: string }
  > {
    if (!key.adapterId || !key.targetId) throw new Error("Invalid integration key.");
    const guard = { workspacePath, generation: backendGeneration, activeHarnessRevision };
    try {
      const port = await restoration.getReadiness();
      if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE, code: "workspace_changed" };
      }
      const mutation = action !== "status";
      const resp = await fetch(
        `http://127.0.0.1:${port}/workspace/integrations/${encodeURIComponent(key.adapterId)}/${encodeURIComponent(key.targetId)}/${action}`,
        mutation
          ? {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(expected),
          }
          : { method: "GET" },
      );
      if (resp.ok) {
        return { ok: true, group: await resp.json() as GroupedIntegrationStatus };
      }
      const body = await resp.json().catch(() => ({ error: undefined, code: undefined })) as {
        error?: string;
        code?: string;
      };
      return {
        ok: false,
        error: body.error ?? `Couldn't ${integrationActionLabels[action]}.`,
        ...(body.code ? { code: body.code } : {}),
      };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  function toIntegrationStatusResult(
    response: Awaited<ReturnType<typeof callIntegrationRoute>>,
  ): IntegrationStatusResult {
    return response.ok
      ? { ok: true, status: response.status as IntegrationStatus }
      : { ok: false, error: response.error };
  }

  function toGroupedIntegrationStatusResult(
    response: Awaited<ReturnType<typeof callGroupedIntegrationRoute>>,
  ): GroupedIntegrationStatusResult {
    return response.ok
      ? response
      : { ok: false, error: response.error, ...(response.code ? { code: response.code } : {}) };
  }

  ipcMain.handle("get-harness-integration-status", async (_event, harnessId: unknown) =>
    callIntegrationRoute(harnessId, "status"));

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
      captureWorkspaceGuard: () => ({ workspacePath, generation: backendGeneration, activeHarnessRevision }),
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
        captureWorkspaceGuard: () => ({ workspacePath, generation: backendGeneration, activeHarnessRevision }),
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

  async function parseErrorBody(resp: Response, fallback: string): Promise<string> {
    const body = await resp.json().catch(() => ({ error: undefined }));
    return (body as { error?: string }).error ?? fallback;
  }

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
      const guard = { workspacePath, generation: backendGeneration, activeHarnessRevision };
      const port = await restoration.getReadiness();
      const snapshot = await fetchHarnessesForSave();
      if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
      const harness = snapshot.harnesses.find((candidate) => candidate.id === harnessId);
      if (!harness) return { ok: false, error: "Coding tool was not found." };
      if (!snapshot.documentRevision) return { ok: false, error: "Coding tool configuration has no revision." };
      const isCustom = harness.origin === "custom";
      const definition = isCustom ? editableCustomDefinition(harness) : undefined;
      if (definition && typeof definition.launch === "object" && definition.launch !== null) {
        (definition.launch as { command?: string }).command = commandPath.trim();
      }
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: isCustom ? "CustomReplace" : "BuiltinPatch",
          ...(isCustom
            ? { definition }
            : { patch: { launch: { command: commandPath.trim() } } }),
          expectedRevision: snapshot.documentRevision,
        }),
      });
      if (resp.ok) {
        const updated = effectiveHarnessFromMutationResponse(await resp.json());
        return updated ? { ok: true, harness: updated } : { ok: false, error: "Malformed harness update response." };
      }
      return { ok: false, error: await parseErrorBody(resp, "Couldn't set the custom path.") };
    } catch {
      return { ok: false, error: "Couldn't reach the OrkWorks sidecar." };
    }
  }

  async function clearHarnessCommandOverride(harnessId: unknown) {
    if (typeof harnessId !== "string" || !harnessId) throw new Error("Invalid harness ID.");
    try {
      const guard = { workspacePath, generation: backendGeneration, activeHarnessRevision };
      const port = await restoration.getReadiness();
      const snapshot = await fetchHarnessesForSave();
      if (isStale(guard, { workspacePath, generation: backendGeneration, activeHarnessRevision })) {
        return { ok: false, error: STALE_WORKSPACE_FALLBACK_MESSAGE };
      }
      const harness = snapshot.harnesses.find((candidate) => candidate.id === harnessId);
      if (!harness) return { ok: false, error: "Coding tool was not found." };
      if (harness.origin === "custom") {
        return { ok: false, error: "Custom harness commands are edited in the configuration JSON." };
      }
      const resp = await fetch(`http://127.0.0.1:${port}/harnesses/${encodeURIComponent(harnessId)}`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ expectedRevision: snapshot.documentRevision }),
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
    if (!workspaceSwitchCoordinator) throw new Error("Workspace lifecycle is unavailable");
    const result = await workspaceSwitchCoordinator.pickWorkspace(async () => {
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
      querySessions: async () => {
        const port = await restoration.getReadiness();
        const baseUrl = `http://127.0.0.1:${port}`;
        const sessions = await listSessions(baseUrl);
        return sessions.filter((session) => session.lifecycle === "alive");
      },
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
      stopSidecar: async () => {
        if (!sidecarLifecycle) throw new Error("Backend lifecycle is unavailable");
        await sidecarLifecycle.stopAndWait(10_000);
      },
      restartSidecar: async () => {
        if (!sidecarLifecycle) throw new Error("Backend lifecycle is unavailable. Restart OrkWorks.");
        const restartCwd = workspacePath ?? app.getPath("home");
        try {
          const lifecycleReadiness = sidecarLifecycle.start(restartCwd);
          await lifecycleReadiness;
          await restoration.getReadiness();
        } catch {
          throw new Error("The sidecar could not be restarted. Restart OrkWorks to recover.");
        }
      },
      runInstall: async (install) => {
        if (!workspaceSwitchCoordinator) throw new Error("Workspace lifecycle is unavailable");
        await workspaceSwitchCoordinator.runUpdate(async () => install());
      },
    });
    registerUpdateIpc(updateService);
  }

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
  updateService = null;
  backendRestoration?.dispose();
  backendRestoration = null;
  sidecarLifecycle?.dispose();
  sidecarLifecycle = null;
  workspaceSwitchCoordinator = null;
}

function requestQuit(): void {
  void (workspaceSwitchCoordinator?.quit() ?? Promise.resolve({ ok: true as const, state: "picker" as const, generation: 0 }))
    .then((result) => {
      if (!result.ok) {
        // The coordinator has already published the unresolved diagnostic. Keep
        // the app and retry path alive so a later quit can try cleanup again.
        quitInProgress = false;
        return;
      }
      killSidecar();
      quitBypass = true;
      app.quit();
    })
    .catch(() => {
      // An unexpected coordinator rejection is also unsafe to finalize. Keep
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
