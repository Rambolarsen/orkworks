import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
});
