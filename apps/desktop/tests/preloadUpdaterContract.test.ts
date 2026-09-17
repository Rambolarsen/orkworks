import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";

const preload = await readFile(new URL("../electron/preload.ts", import.meta.url), "utf8");
const windowTypes = await readFile(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

test("preload and renderer contracts expose the same four actions and subscription", () => {
  for (const name of ["getUpdateStatus", "checkForUpdates", "downloadUpdate", "requestUpdateInstall", "onUpdateStatus"]) {
    assert.match(preload, new RegExp(name));
    assert.match(windowTypes, new RegExp(name));
  }
});

test("sandboxed development preload is explicitly flagged and stays IPC-inert", async () => {
  const main = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");
  const compiled = transpileModule(preload, {
    compilerOptions: { module: ModuleKind.CommonJS, target: ScriptTarget.ES2022 },
  }).outputText;
  for (const argv of [["electron", "--orkworks-packaged=false"], ["electron"]]) {
    let bridge: any;
    const ipcCalls: string[] = [];
    const ipcRenderer = new Proxy({}, { get: (_target, key) => () => {
      ipcCalls.push(String(key));
      return Promise.resolve({ state: "unexpected-ipc" });
    } });
    new Function("require", "exports", "process", compiled)(
      (name: string) => name === "electron"
        ? { contextBridge: { exposeInMainWorld: (_name: string, value: unknown) => { bridge = value; } }, ipcRenderer }
        : { subscribeBackendLifecycle() {} },
      {}, { platform: "win32", sandboxed: true, argv },
    );
    const expected = { state: "unavailable", reason: "development", sequence: 0 };
    for (const action of ["getUpdateStatus", "checkForUpdates", "downloadUpdate", "requestUpdateInstall"]) {
      assert.deepEqual(await bridge[action](), expected);
    }
    const statuses: unknown[] = [];
    bridge.onUpdateStatus((status: unknown) => statuses.push(status))();
    assert.deepEqual(statuses, [expected]);
    assert.deepEqual(ipcCalls, []);
  }
  assert.match(main, /additionalArguments:\s*\[`--orkworks-packaged=\$\{app\.isPackaged\}`\]/);
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
