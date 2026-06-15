import { app, BrowserWindow, ipcMain } from "electron";
import { spawn, type ChildProcess } from "child_process";
import * as path from "path";

let mainWindow: BrowserWindow | null = null;
let sidecarProcess: ChildProcess | null = null;
let backendPort: number | null = null;
let portResolve: ((port: number) => void) | null = null;
const portPromise = new Promise<number>((resolve) => {
  portResolve = resolve;
});

function getSidecarPath(): string {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, "orkworksd");
  }
  const repoRoot = path.resolve(__dirname, "..", "..", "..");
  return path.join(repoRoot, "crates", "orkworksd", "target", "debug", "orkworksd");
}

function startSidecar(): void {
  const binaryPath = getSidecarPath();
  console.log(`[main] starting sidecar: ${binaryPath}`);

  sidecarProcess = spawn(binaryPath, [], {
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
  ipcMain.handle("get-backend-url", async () => {
    const port = await portPromise;
    return `http://127.0.0.1:${port}`;
  });

  startSidecar();
  createWindow();

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

app.on("before-quit", () => {
  if (sidecarProcess) {
    sidecarProcess.kill();
    sidecarProcess = null;
  }
});
