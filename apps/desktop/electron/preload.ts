import { contextBridge, ipcRenderer } from "electron";
import { subscribeBackendLifecycle, type BackendLifecycleEvent } from "./backendLifecycleEvent";

type IntegrationKey = {
  adapterId: string;
  targetId: string;
};

type IntegrationConsumer = {
  harnessId: string;
  harnessName: string;
};

type GroupedIntegrationStatus = {
  key: IntegrationKey;
  consumers: IntegrationConsumer[];
  status: {
    harnessId: string;
    enabled: boolean;
    toolDetected: boolean;
    registration: "unsupported" | "absent" | "installed" | "drifted" | "error";
    ownership: "none" | "ork_works" | "ambiguous";
    activation: "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
    coverage: "full" | "limited" | "none";
    diagnostics: Array<{ code: string; message: string; action?: string }>;
    confirmation: {
      toolName: string;
      workspaceLabel: string;
      coverageSummary: string;
      relativePaths: string[];
      executableCodeWarning: boolean;
    } | null;
  };
};

type GroupedIntegrationStatusResult =
  | { ok: true; group: GroupedIntegrationStatus }
  | { ok: false; error: string; code?: string };

type ActiveHarnessIntegrationResult = {
  key: IntegrationKey;
  consumerHarnessIds: string[];
  operation: "install" | "repair" | "uninstall" | "skipped";
  outcome: "succeeded" | "failed" | "unsupported" | "stale_workspace";
  registration: "unsupported" | "absent" | "installed" | "drifted" | "error";
  activation: "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
  coverage: "full" | "limited" | "none";
  diagnosticCode?: string;
  message?: string;
};

type ActiveHarnessSaveResult = {
  activeHarnesses: {
    outcome: "persisted" | "failed" | "stale_workspace";
    message?: string;
  };
  integrations: Record<string, ActiveHarnessIntegrationResult>;
};

contextBridge.exposeInMainWorld("orkworks", {
  platform: process.platform,
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  retryBackend: (): Promise<void> => ipcRenderer.invoke("retry-backend"),
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
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
  getSettings: (): Promise<unknown> => ipcRenderer.invoke("get-settings"),
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
  saveProviderSettings: (providers: unknown): Promise<{ ok: true; settings: unknown; providerApplyStatus?: { appliedRevision: number | null; appliedAt: string | null; lastApplyError: string | null } }> =>
    ipcRenderer.invoke("save-provider-settings", providers),
  verifyOllama: (baseUrl: string): Promise<unknown> => ipcRenderer.invoke("verify-ollama", baseUrl),
  getProviderModels: (providerId: string): Promise<unknown> => ipcRenderer.invoke("get-provider-models", providerId),
  getProviderLabels: (): Promise<unknown> => ipcRenderer.invoke("get-provider-labels"),
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
  openExternalLink: (url: string): Promise<void> => ipcRenderer.invoke("open-external-link", url),
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
