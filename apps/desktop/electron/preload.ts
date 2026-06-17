import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
  onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => {
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
