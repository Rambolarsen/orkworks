import { contextBridge, ipcRenderer } from "electron";

type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number }
  | { state: "failed" | "exhausted"; message: string };

function isBackendLifecycleEvent(data: unknown): data is BackendLifecycleEvent {
  if (!data || typeof data !== "object" || Array.isArray(data)) return false;
  const event = data as Record<string, unknown>;
  if (event.state === "starting" || event.state === "retrying") return true;
  if (event.state === "ready") return typeof event.port === "number" && Number.isInteger(event.port);
  if (event.state === "failed" || event.state === "exhausted") return typeof event.message === "string";
  return false;
}

contextBridge.exposeInMainWorld("orkworks", {
  platform: process.platform,
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  retryBackend: (): Promise<void> => ipcRenderer.invoke("retry-backend"),
  onBackendLifecycle: (callback: (event: BackendLifecycleEvent) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: unknown) => {
      if (isBackendLifecycleEvent(data)) callback(data);
    };
    ipcRenderer.on("orkworks:backend-lifecycle", handler);
    return () => {
      ipcRenderer.removeListener("orkworks:backend-lifecycle", handler);
    };
  },
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
  getSettings: (): Promise<unknown> => ipcRenderer.invoke("get-settings"),
  saveHotkeys: (hotkeys: unknown): Promise<unknown> => ipcRenderer.invoke("save-hotkeys", hotkeys),
  saveRetention: (retention: unknown): Promise<{ ok: boolean; retentionApplyStatus?: { appliedRevision: number | null; appliedAt: string | null; lastApplyError: string | null } }> =>
    ipcRenderer.invoke("save-retention", retention),
  saveDebugSettings: (debug: unknown): Promise<unknown> => ipcRenderer.invoke("save-debug-settings", debug),
  saveProviderSettings: (providers: unknown): Promise<{ ok: true; settings: unknown; providerApplyStatus?: { appliedRevision: number | null; appliedAt: string | null; lastApplyError: string | null } }> =>
    ipcRenderer.invoke("save-provider-settings", providers),
  verifyOllama: (baseUrl: string): Promise<unknown> => ipcRenderer.invoke("verify-ollama", baseUrl),
  getProviderModels: (providerId: string): Promise<unknown> => ipcRenderer.invoke("get-provider-models", providerId),
  getProviderLabels: (): Promise<unknown> => ipcRenderer.invoke("get-provider-labels"),
  getHarnessIntegrationStatus: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("get-harness-integration-status", harnessId),
  installHarnessIntegration: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("install-harness-integration", harnessId),
  uninstallHarnessIntegration: (harnessId: string): Promise<unknown> =>
    ipcRenderer.invoke("uninstall-harness-integration", harnessId),
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
