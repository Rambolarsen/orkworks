import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const preload = await readFile(new URL("../electron/preload.ts", import.meta.url), "utf8");
const windowTypes = await readFile(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

test("preload and renderer contracts expose the same four actions and subscription", () => {
  for (const name of ["getUpdateStatus", "checkForUpdates", "downloadUpdate", "requestUpdateInstall", "onUpdateStatus"]) {
    assert.match(preload, new RegExp(name));
    assert.match(windowTypes, new RegExp(name));
  }
});

test("development preload methods stay IPC-inert", () => {
  assert.match(preload, /process\.defaultApp/);
  assert.match(preload, /state:\s*"unavailable"/);
  assert.match(preload, /reason:\s*"development"/);
});

test("packaged preload methods use the matching IPC channels and removable status listener", () => {
  for (const channel of ["get-update-status", "check-for-updates", "download-update", "request-update-install"]) {
    assert.match(preload, new RegExp(`ipcRenderer\\.invoke\\(\\"${channel}\\"\\)`));
  }
  assert.match(preload, /ipcRenderer\.on\("update-status", handler\)/);
  assert.match(preload, /ipcRenderer\.removeListener\("update-status", handler\)/);
});

test("renderer contract defines UpdateStatus independently from Electron main", () => {
  assert.match(windowTypes, /export type UpdateStatus\s*=/);
  assert.doesNotMatch(windowTypes, /electron\/updateService/);
});
