import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
  getLayout: (): Promise<string | null> => ipcRenderer.invoke("get-layout"),
  saveLayout: (json: string): Promise<void> => ipcRenderer.invoke("save-layout", json),
});
