import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  platform: process.platform,
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
  getSettings: (): Promise<unknown> => ipcRenderer.invoke("get-settings"),
  saveHotkeys: (hotkeys: unknown): Promise<unknown> => ipcRenderer.invoke("save-hotkeys", hotkeys),
  saveRetention: (retention: unknown): Promise<unknown> => ipcRenderer.invoke("save-retention", retention),
  saveProviderSettings: (providers: unknown): Promise<unknown> => ipcRenderer.invoke("save-provider-settings", providers),
  getProviderModels: (providerId: string): Promise<unknown> => ipcRenderer.invoke("get-provider-models", providerId),
  getProviderLabels: (): Promise<unknown> => ipcRenderer.invoke("get-provider-labels"),
  getClaudeCodeHookStatus: (): Promise<unknown> => ipcRenderer.invoke("get-claude-code-hook-status"),
  installClaudeCodeHook: (): Promise<unknown> => ipcRenderer.invoke("install-claude-code-hook"),
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
