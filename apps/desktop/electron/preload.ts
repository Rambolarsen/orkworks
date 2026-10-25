import type { GitChangesRequest, GitChangesResult } from "./gitChangesTypes";
import { contextBridge, ipcRenderer } from "electron";
import { subscribeBackendLifecycle, type BackendLifecycleEvent, type BackendRetryResult, type InitialWorkspaceSnapshot } from "./backendLifecycleEvent";
import type { UpdateStatus } from "./updateService";

import type { ActiveHarnessSaveResult, GroupedIntegrationStatusResult } from "./activeHarnessIntegration";

type TaskmasterAcceptOptions = {
  sessionId?: string;
  prompt?: string;
  packetRevision?: number;
  evidenceFingerprint?: string;
  idempotencyKey?: string;
};
type TaskmasterAssessmentRequestResponse = {
  status: "scheduled" | "cached" | "active_recommendation" | "already_running" | "unavailable";
  recommendation?: { id: string; status: string };
  message?: string;
};

type LastCentralSurface = "terminal" | "review";
type ShellPreferences = { sessionsWidth: number; inspectorWidth: number; sessionsVisible: boolean; inspectorPinned?: boolean; density: "low" | "high" };
type ShellMemoryDiagnostic = "corrupt_record" | "unsupported_version" | "lock_timeout" | "write_failed" | "restore_failed" | "stale_revision" | "stale_workspace" | "invalid_input";
type ShellMemoryResult = { ok: true } | { ok: false; diagnostic: ShellMemoryDiagnostic };
type ShellMemoryRebuildResult = ShellMemoryResult | { ok: false; diagnostic: "user_cancelled" };
type ShellLayoutSnapshot = { preferences: ShellPreferences; revision: number; diagnostic: ShellMemoryDiagnostic | null };
type WorkspaceNavigationView = { lastCentralSurface: LastCentralSurface | null; revision: number; diagnostic: ShellMemoryDiagnostic | null };

const developmentUpdateStatus: UpdateStatus = {
  state: "unavailable",
  reason: "development",
  sequence: 0,
};

const packagedUpdater = process.argv.includes("--orkworks-packaged=true");

contextBridge.exposeInMainWorld("orkworks", {
  platform: process.platform,
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  createSessionWithImages: (baseUrl: string, options: unknown): Promise<unknown> => ipcRenderer.invoke("create-session-with-images", baseUrl, options),
  retryBackend: (): Promise<BackendRetryResult> => ipcRenderer.invoke("retry-backend"),
  getUpdateStatus: (): Promise<UpdateStatus> => !packagedUpdater
    ? Promise.resolve(developmentUpdateStatus)
    : ipcRenderer.invoke("get-update-status"),
  checkForUpdates: (): Promise<UpdateStatus> => !packagedUpdater
    ? Promise.resolve(developmentUpdateStatus)
    : ipcRenderer.invoke("check-for-updates"),
  downloadUpdate: (): Promise<UpdateStatus> => !packagedUpdater
    ? Promise.resolve(developmentUpdateStatus)
    : ipcRenderer.invoke("download-update"),
  requestUpdateInstall: (): Promise<UpdateStatus> => !packagedUpdater
    ? Promise.resolve(developmentUpdateStatus)
    : ipcRenderer.invoke("request-update-install"),
  onUpdateStatus: (callback: (status: UpdateStatus) => void): (() => void) => {
    if (!packagedUpdater) {
      callback(developmentUpdateStatus);
      return () => undefined;
    }
    const handler = (_event: Electron.IpcRendererEvent, status: UpdateStatus) => callback(status);
    ipcRenderer.on("update-status", handler);
    ipcRenderer.send("subscribe-update-status");
    return () => ipcRenderer.removeListener("update-status", handler);
  },
  onBackendLifecycle: (callback: (event: BackendLifecycleEvent) => void) =>
    subscribeBackendLifecycle(
      (listener) => {
        const handler = (_event: Electron.IpcRendererEvent, data: unknown) => listener(data);
        ipcRenderer.on("orkworks:backend-lifecycle", handler);
        return () => ipcRenderer.removeListener("orkworks:backend-lifecycle", handler);
      },
      () => ipcRenderer.invoke("get-backend-lifecycle"),
      callback,
    ),
  getInitialWorkspace: (): Promise<InitialWorkspaceSnapshot> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getWorkspaceHistory: (): Promise<unknown> => ipcRenderer.invoke("get-workspace-history"),
  pinWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("pin-workspace-path", path),
  unpinWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("unpin-workspace-path", path),
  forgetWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("forget-workspace-path", path),
  openRememberedWorkspace: (path: string): Promise<unknown> => ipcRenderer.invoke("open-remembered-workspace", path),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
  getShellLayout: (): Promise<ShellLayoutSnapshot> => ipcRenderer.invoke("get-shell-layout"),
  saveShellLayout: (preferences: ShellPreferences): Promise<ShellMemoryResult> => ipcRenderer.invoke("save-shell-layout", preferences),
  resetShellLayout: (): Promise<ShellMemoryResult> => ipcRenderer.invoke("reset-shell-layout"),
  rebuildShellLayout: (): Promise<ShellMemoryRebuildResult> => ipcRenderer.invoke("rebuild-shell-layout"),
  getWorkspaceNavigation: (): Promise<WorkspaceNavigationView> => ipcRenderer.invoke("get-workspace-navigation"),
  completeWorkspaceNavigation: (surface: LastCentralSurface): Promise<ShellMemoryResult> => ipcRenderer.invoke("complete-workspace-navigation", surface),
  deleteWorkspaceNavigation: (): Promise<ShellMemoryResult> => ipcRenderer.invoke("delete-workspace-navigation"),
  rebuildWorkspaceNavigation: (): Promise<ShellMemoryRebuildResult> => ipcRenderer.invoke("rebuild-workspace-navigation"),
  getSettings: (): Promise<unknown> => ipcRenderer.invoke("get-settings"),
  getTaskmasterSettings: (): Promise<unknown> => ipcRenderer.invoke("get-taskmaster-settings"),
  getTaskmasterRunStatus: (): Promise<unknown> => ipcRenderer.invoke("get-taskmaster-run-status"),
  refreshTaskmasterModels: (provider: "codex" | "ollama", ollamaBaseUrl?: string): Promise<string[]> => ipcRenderer.invoke("refresh-taskmaster-models", provider, ollamaBaseUrl),
  requestTaskmasterAnalysis: (): Promise<unknown> => ipcRenderer.invoke("analyze-taskmaster"),
  getReadinessStatus: (): Promise<Record<string, unknown>> => ipcRenderer.invoke("get-readiness-status"),
  requestReadinessAssessment: (): Promise<{ outcome: "scheduled" | "already_running" | "unavailable"; runId: string | null; reasonCode: string | null }> => ipcRenderer.invoke("assess-repository-readiness"),
  requestTaskmasterAssessment: (): Promise<TaskmasterAssessmentRequestResponse> => ipcRenderer.invoke("assess-taskmaster-workflow"),
  dismissTaskmasterRecommendation: (id: string, reason?: string): Promise<void> =>
    ipcRenderer.invoke("dismiss-taskmaster-recommendation", id, reason),
  acceptTaskmasterRecommendation: (id: string, options: TaskmasterAcceptOptions): Promise<unknown> =>
    ipcRenderer.invoke("accept-taskmaster-recommendation", id, options),
  applyDebugAttention: (id: string, attention: string, message?: string): Promise<void> =>
    ipcRenderer.invoke("apply-debug-attention", id, attention, message),
  getInferenceTrust: (): Promise<import("./inferenceTrust").InferenceAdapterView[]> => ipcRenderer.invoke("get-inference-trust"),
  approveInferenceAdapter: (request: import("./inferenceTrust").InferenceTrustRequest): Promise<boolean> => ipcRenderer.invoke("approve-inference-adapter", request),
  revokeInferenceAdapter: (request: import("./inferenceTrust").InferenceTrustRequest): Promise<void> => ipcRenderer.invoke("revoke-inference-adapter", request),
  saveTaskmasterSettings: (settings: unknown): Promise<unknown> => ipcRenderer.invoke("save-taskmaster-settings", settings),
  verifyPeonProvider: (provider: string, ollamaBaseUrl?: string): Promise<unknown> =>
    ipcRenderer.invoke("verify-peon-provider", provider, ollamaBaseUrl),
  testAndApplyPeonProvider: (selection: unknown): Promise<unknown> =>
    ipcRenderer.invoke("test-and-apply-peon-provider", selection),
  getAppliedPeonProvider: (): Promise<unknown> => ipcRenderer.invoke("get-applied-peon-provider"),
  savePeonSelection: (selection: unknown): Promise<unknown> => ipcRenderer.invoke("save-peon-selection", selection),
  saveHotkeys: (hotkeys: unknown): Promise<unknown> => ipcRenderer.invoke("save-hotkeys", hotkeys),
  saveRetention: (retention: unknown): Promise<{ ok: boolean; retentionApplyStatus?: { appliedRevision: number | null; appliedAt: string | null; lastApplyError: string | null } }> =>
    ipcRenderer.invoke("save-retention", retention),
  saveDebugSettings: (debug: unknown): Promise<unknown> => ipcRenderer.invoke("save-debug-settings", debug),
  getProviderModels: (providerId: string): Promise<unknown> => ipcRenderer.invoke("get-provider-models", providerId),
  saveActiveHarnessesWithIntegrations: (ids: string[]): Promise<ActiveHarnessSaveResult> =>
    ipcRenderer.invoke("save-active-harnesses-with-integrations", ids),
  enableHarnessIntegrationImmediate: (ids: string[], adapterId: string, targetId: string): Promise<ActiveHarnessSaveResult> =>
    ipcRenderer.invoke("enable-harness-integration-immediate", ids, adapterId, targetId),
  getHarnessIntegrationStatus: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("get-harness-integration-status", harnessId),
  getGroupedHarnessIntegrationStatus: (adapterId: string, targetId: string): Promise<GroupedIntegrationStatusResult> =>
    ipcRenderer.invoke("get-grouped-harness-integration-status", adapterId, targetId),
  setHarnessCommandOverride: (harnessId: string, commandPath: string): Promise<unknown> =>
    ipcRenderer.invoke("set-harness-command-override", harnessId, commandPath),
  clearHarnessCommandOverride: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("clear-harness-command-override", harnessId),
  inspectSessionApp: (workspacePath: string, sessionId: string): Promise<import("./sessionAppOpener").SessionAppAvailability> => ipcRenderer.invoke("inspect-session-app", workspacePath, sessionId),
  openSessionApp: (workspacePath: string, sessionId: string): Promise<void> => ipcRenderer.invoke("open-session-app", workspacePath, sessionId),
  openExternalLink: (url: string): Promise<void> => ipcRenderer.invoke("open-external-link", url),
  readGitChanges: (request: GitChangesRequest): Promise<GitChangesResult> => ipcRenderer.invoke("read-git-changes", request),
  getPlanContent: (sessionId: string): Promise<string> => ipcRenderer.invoke("get-plan-content", sessionId),
  requestPlanReview: (sessionId: string): Promise<void> => ipcRenderer.invoke("request-plan-review", sessionId),
  selectTerminalPlan: (sessionId: string, printedPath: string): Promise<void> => ipcRenderer.invoke("select-terminal-plan", sessionId, printedPath),
  setHotkeyCaptureActive: (active: boolean) => {
    ipcRenderer.send("orkworks:hotkey-capture-active", active);
  },
  onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => {
    ipcRenderer.removeAllListeners("orkworks:menu-command");
    const handler = (_event: Electron.IpcRendererEvent, data: { action: string; panelId?: string }) => callback(data);
    ipcRenderer.on("orkworks:menu-command", handler);
    return () => {
      ipcRenderer.removeListener("orkworks:menu-command", handler);
    };
  },
  notifyPanelVisibility: (panelId: string, visible: boolean) => {
    ipcRenderer.send("orkworks:panel-visibility", { panelId, visible });
  },
});
