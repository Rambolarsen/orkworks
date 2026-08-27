import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const mainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

test("Electron main centralizes initial and workspace sidecar startup", () => {
  assert.match(mainSource, /import \{ createSidecarLifecycle/);
  assert.equal(mainSource.match(/createSidecarLifecycle\(/g)?.length, 1);
  assert.equal(mainSource.match(/\bspawn\(/g)?.length, 1);
  assert.equal(mainSource.match(/ORKWORKS_OPEN_PLAN_TOKEN/g)?.length, 1);
  assert.match(mainSource, /sidecarLifecycle\.start\(initialSidecarCwd\)/);
  assert.match(mainSource, /sidecarLifecycle!\.start\(nextPath\)/);
});

test("Electron main restores workspace and settings before publishing ready", () => {
  assert.match(mainSource, /import \{ createBackendRestorationCoordinator, switchWorkspaceBackend/);
  assert.equal(mainSource.match(/createBackendRestorationCoordinator(?:<[^>]+>)?\(/g)?.length, 1);
  assert.match(mainSource, /restoreWorkspace: \(signal\) => restoreWorkspace\(port, signal\)/);
  assert.match(mainSource, /applyRetentionSettings: \(signal\) => applyRetentionSettings\(port, signal\)/);
  assert.match(mainSource, /syncProviderSettings: \(signal\) => syncSavedProviderSettings\(port, signal\)/);
  assert.match(mainSource, /async function restoreWorkspace\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /async function applyRetentionSettings\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /async function syncSavedProviderSettings\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /onReady: \(port, workspace\) =>/);
  assert.match(mainSource, /state: "ready", port, workspace/);
});

test("Electron main logs raw lifecycle failures but publishes only stable copy", () => {
  assert.match(mainSource, /sanitizeBackendLifecycleFailure/);
  assert.match(mainSource, /console\.error\(/);
  assert.match(mainSource, /lastBackendFailure = sanitizeBackendLifecycleFailure/);
  assert.doesNotMatch(mainSource, /publishBackendLifecycle\(\{ state: "failed", message: error\.message \}\)/);
});

test("backend readiness and retry use the lifecycle controller", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-backend-url", async \(\) => \{\s*const port = await restoration\.getReadiness\(\)/);
  assert.match(mainSource, /ipcMain\.handle\("retry-backend", async \(\) => \{[\s\S]*sidecarLifecycle\.retry\(\)/);
  assert.doesNotMatch(mainSource, /new Promise<number>\(\(resolve\) => \{\s*portResolve/);
});

test("initial workspace restoration handles rejected readiness", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /try \{[\s\S]*await restoration\.getReadiness\(\);[\s\S]*\} catch \{[\s\S]*return null;/);
});

test("workspace persistence happens before the replacement backend starts", () => {
  const start = mainSource.indexOf('ipcMain.handle("open-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /switchWorkspaceBackend\(\s*dirPath,/);
  assert.doesNotMatch(handler, /sidecarLifecycle\.stop\(\)/);
});

test("Electron main replays the latest lifecycle state to late subscribers", () => {
  assert.match(mainSource, /let latestBackendLifecycle: BackendLifecycleEvent \| null = null/);
  assert.match(mainSource, /ipcMain\.handle\("get-backend-lifecycle"/);
  assert.match(mainSource, /return latestBackendLifecycle/);
  assert.match(preloadSource, /subscribeBackendLifecycle\(/);
  assert.match(preloadSource, /ipcRenderer\.invoke\("get-backend-lifecycle"\)/);
  assert.doesNotMatch(mainSource, /event\.sender\.send\("orkworks:backend-lifecycle"/);
});

test("preload validates and forwards the lifecycle contract", () => {
  assert.match(preloadSource, /import \{ subscribeBackendLifecycle, type BackendLifecycleEvent \}/);
  assert.match(preloadSource, /subscribeBackendLifecycle\(/);
  assert.match(preloadSource, /onBackendLifecycle:/);
  assert.match(preloadSource, /retryBackend:/);
});

test("renderer declarations expose the same lifecycle contract", () => {
  assert.match(rendererTypes, /export type BackendLifecycleEvent\s*=/);
  assert.match(rendererTypes, /state: "starting" \| "retrying"/);
  assert.match(rendererTypes, /state: "ready"; port: number; workspace: WorkspaceInfo \| null/);
  assert.match(rendererTypes, /state: "failed" \| "exhausted"; message: string/);
  assert.match(rendererTypes, /onBackendLifecycle: \(callback: \(event: BackendLifecycleEvent\) => void\) => \(\) => void/);
  assert.match(rendererTypes, /retryBackend: \(\) => Promise<void>/);
});
