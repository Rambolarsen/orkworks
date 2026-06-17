import { app, BrowserWindow, dialog, ipcMain, Menu } from "electron";
import { spawn, type ChildProcess } from "child_process";
import { existsSync } from "fs";
import * as path from "path";
import { getDevRepoRoot, getDevSidecarPath } from "./paths";
import { readWorkspaceMemory, rememberWorkspacePath } from "./workspaceMemory";
import { readLayoutMemory, writeLayoutMemory } from "./layoutMemory";

let mainWindow: BrowserWindow | null = null;
let sidecarProcess: ChildProcess | null = null;
let backendPort: number | null = null;
let portResolve: ((port: number) => void) | null = null;
let portPromise = new Promise<number>((resolve) => {
  portResolve = resolve;
});

let menuPanelItems: Record<string, Electron.MenuItem> = {};

function buildMenu(): void {
  const panelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"];
  const panelTitles: Record<string, string> = {
    sessions: "Sessions",
    detail: "Detail",
    terminal: "Terminal",
    capacity: "Capacity",
    recommendations: "Recommendations",
  };

  const panelItems: Electron.MenuItemConstructorOptions[] = panelIds.map((id) => ({
    id,
    label: panelTitles[id],
    type: "checkbox",
    checked: true,
    click: () => {
      mainWindow?.webContents.send("orkworks:menu-command", { action: "toggle", panelId: id });
    },
  }));

  const viewSubmenu: Electron.MenuItemConstructorOptions[] = [
    ...panelItems,
    { type: "separator" },
    {
      label: "Reset Layout",
      click: () => {
        mainWindow?.webContents.send("orkworks:menu-command", { action: "reset-layout" });
      },
    },
    { type: "separator" },
    { role: "reload" },
    { role: "forceReload" },
    { role: "toggleDevTools" },
    { type: "separator" },
    { role: "resetZoom" },
    { role: "zoomIn" },
    { role: "zoomOut" },
    { type: "separator" },
    { role: "togglefullscreen" },
  ];

  const template: Electron.MenuItemConstructorOptions[] = [
    {
      label: app.name,
      submenu: [
        { role: "about" },
        { type: "separator" },
        { role: "services" },
        { type: "separator" },
        { role: "hide" },
        { role: "hideOthers" },
        { role: "unhide" },
        { type: "separator" },
        { role: "quit" },
      ],
    },
    {
      label: "File",
      submenu: [{ role: "close" }],
    },
    {
      label: "Edit",
      submenu: [
        { role: "undo" },
        { role: "redo" },
        { type: "separator" },
        { role: "cut" },
        { role: "copy" },
        { role: "paste" },
        { role: "selectAll" },
      ],
    },
    {
      label: "View",
      submenu: viewSubmenu,
    },
    {
      label: "Window",
      submenu: [
        { role: "minimize" },
        { role: "zoom" },
        ...(process.platform === "darwin"
          ? [{ type: "separator" as const }, { role: "front" as const }]
          : [{ role: "close" as const }]),
      ],
    },
    {
      role: "help",
      submenu: [
        {
          label: "Learn More",
          click: () => {
            /* placeholder */
          },
        },
      ],
    },
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);

  menuPanelItems = {};
  for (const id of panelIds) {
    const item = menu.getMenuItemById(id);
    if (item) menuPanelItems[id] = item;
  }
}

function getSidecarPath(): string {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, "orkworksd");
  }
  return getDevSidecarPath(__dirname);
}

function startSidecar(cwdOverride?: string): void {
  const binaryPath = getSidecarPath();
  const sidecarCwd = cwdOverride ?? (app.isPackaged ? app.getPath("home") : getDevRepoRoot(__dirname));
  console.log(`[main] starting sidecar: ${binaryPath}`);
  console.log(`[main] sidecar cwd: ${sidecarCwd}`);

  sidecarProcess = spawn(binaryPath, [], {
    cwd: sidecarCwd,
    stdio: ["ignore", "pipe", "pipe"],
  });

  sidecarProcess.stdout?.on("data", (data: Buffer) => {
    const line = data.toString().trim();
    console.log(`[orkworksd] ${line}`);
    const match = line.match(/ORKWORKSD_PORT=(\d+)/);
    if (match) {
      backendPort = parseInt(match[1], 10);
      console.log(`[main] sidecar ready on port ${backendPort}`);
      if (portResolve) {
        portResolve(backendPort);
        portResolve = null;
      }
    }
  });

  sidecarProcess.stderr?.on("data", (data: Buffer) => {
    console.error(`[orkworksd:err] ${data.toString().trim()}`);
  });

  sidecarProcess.on("exit", (code) => {
    console.log(`[main] sidecar exited with code ${code}`);
    sidecarProcess = null;
  });
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 500,
    title: "OrkWorks",
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, "preload.js"),
    },
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }

  mainWindow.on("closed", () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  const appMemory = readWorkspaceMemory(app.getPath("userData"));
  const initialWorkspacePath =
    appMemory.lastWorkspacePath && existsSync(appMemory.lastWorkspacePath)
      ? appMemory.lastWorkspacePath
      : null;

  ipcMain.handle("get-backend-url", async () => {
    const port = await portPromise;
    return `http://127.0.0.1:${port}`;
  });

  ipcMain.handle("get-layout", async () => {
    return readLayoutMemory(app.getPath("userData"));
  });

  ipcMain.handle("save-layout", async (_event, json: string) => {
    writeLayoutMemory(app.getPath("userData"), json);
  });

  ipcMain.handle("get-initial-workspace", async () => {
    if (!initialWorkspacePath) return null;
    const port = await portPromise;
    const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: initialWorkspacePath }),
    });
    if (!resp.ok) return null;
    return resp.json();
  });

  ipcMain.handle("open-workspace", async () => {
    const result = await dialog.showOpenDialog({
      properties: ["openDirectory"],
      title: "Select Workspace",
    });
    if (result.canceled || result.filePaths.length === 0) return null;

    const dirPath = result.filePaths[0];
    workspacePath = dirPath;

    rememberWorkspacePath(app.getPath("userData"), dirPath);

    if (sidecarProcess) {
      sidecarProcess.kill();
      sidecarProcess = null;
    }
    backendPort = null;
    portPromise = new Promise<number>((resolve) => {
      portResolve = resolve;
    });

    sidecarProcess = spawn(getSidecarPath(), [], {
      cwd: dirPath,
      stdio: ["ignore", "pipe", "pipe"],
    });

    sidecarProcess.stdout?.on("data", (data: Buffer) => {
      const line = data.toString().trim();
      console.log(`[orkworksd] ${line}`);
      const match = line.match(/ORKWORKSD_PORT=(\d+)/);
      if (match) {
        backendPort = parseInt(match[1], 10);
        console.log(`[main] sidecar ready on port ${backendPort}`);
        if (portResolve) {
          portResolve(backendPort);
          portResolve = null;
        }
      }
    });

    sidecarProcess.stderr?.on("data", (data: Buffer) => {
      console.error(`[orkworksd:err] ${data.toString().trim()}`);
    });

    sidecarProcess.on("exit", (code) => {
      console.log(`[main] sidecar exited with code ${code}`);
      sidecarProcess = null;
    });

    const port = await portPromise;

    const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: dirPath }),
    });

    if (!resp.ok) return null;
    return resp.json();
  });

  ipcMain.on("orkworks:panel-visibility", (_event, data: { panelId: string; visible: boolean }) => {
    const item = menuPanelItems[data.panelId];
    if (item) item.checked = data.visible;
  });

  startSidecar(initialWorkspacePath ?? undefined);
  createWindow();
  buildMenu();

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
  if (sidecarProcess) {
    sidecarProcess.kill();
    sidecarProcess = null;
  }
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
