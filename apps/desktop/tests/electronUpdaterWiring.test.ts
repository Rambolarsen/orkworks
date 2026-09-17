import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const main = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");

test("main gates updater construction on app.isPackaged and keeps automatic install disabled", () => {
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*createUpdateService/);
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*registerUpdateIpc/);
  assert.match(main, /autoDownload:\s*false/);
  assert.match(main, /autoInstallOnAppQuit:\s*false/);
  assert.match(main, /allowDowngrade:\s*false/);
  assert.match(main, /stopAndWait\(10_000\)/);
});

test("main fixes the updater provider and maps public updater events", () => {
  assert.match(main, /provider:\s*"github"/);
  assert.match(main, /owner:\s*"Rambolarsen"/);
  assert.match(main, /repo:\s*"orkworks"/);
  for (const eventName of ["update-available", "update-not-available", "download-progress", "update-downloaded", "error"]) {
    assert.match(main, new RegExp(`autoUpdater\\.on\\(\\"${eventName}\\"`));
  }
});

test("main registers only the narrow updater IPC contract", () => {
  for (const channel of ["get-update-status", "check-for-updates", "download-update", "request-update-install"]) {
    assert.match(main, new RegExp(`ipcMain\\.handle\\(\\"${channel}\\"`));
  }
  assert.match(main, /ipcMain\.on\("subscribe-update-status"/);
  assert.match(main, /event\.sender\.once\("destroyed", unsubscribe\)/);
});

test("main queries current sessions and restarts through the existing lifecycle", () => {
  assert.match(main, /await restoration\.getReadiness\(\)[\s\S]*listSessions\(baseUrl\)/);
  assert.match(main, /lifecycle === "alive"/);
  assert.match(main, /sidecarLifecycle\.start\(restartCwd\)/);
  assert.match(main, /Restart OrkWorks to recover/);
});

test("ordinary before-quit cleanup does not invoke the installer", () => {
  const cleanup = main.slice(main.indexOf("function killSidecar"));
  assert.doesNotMatch(cleanup, /quitAndInstall/);
});
