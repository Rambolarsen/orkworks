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
  assert.match(mainSource, /sidecarLifecycle\.start\(dirPath\)/);
});

test("Electron main restores workspace and settings before publishing ready", () => {
  assert.match(mainSource, /async function restoreBackend\(port: number, generation: number\)/);
  assert.match(mainSource, /await restoreWorkspace\(port, generation\)[\s\S]*await applyRetentionSettings\(port, generation\)[\s\S]*await syncSavedProviderSettings\(port, generation\)[\s\S]*publishBackendLifecycle\(\{ state: "ready", port \}\)/);
  assert.match(mainSource, /if \(!isCurrentBackendGeneration\(generation, port\)\) return/);
});

test("backend readiness and retry use the lifecycle controller", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-backend-url", async \(\) => \{\s*const port = await currentBackendReadiness/);
  assert.match(mainSource, /ipcMain\.handle\("retry-backend", async \(\) => \{[\s\S]*sidecarLifecycle\.retry\(\)/);
  assert.doesNotMatch(mainSource, /new Promise<number>\(\(resolve\) => \{\s*portResolve/);
});

test("initial workspace restoration handles rejected readiness", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /try \{[\s\S]*await currentBackendReadiness;[\s\S]*\} catch \{[\s\S]*return null;/);
});

test("preload validates and forwards the lifecycle contract", () => {
  assert.match(preloadSource, /type BackendLifecycleEvent\s*=/);
  assert.match(preloadSource, /state: "starting" \| "retrying"/);
  assert.match(preloadSource, /state: "ready"; port: number/);
  assert.match(preloadSource, /state: "failed" \| "exhausted"; message: string/);
  assert.match(preloadSource, /isBackendLifecycleEvent\(data\)/);
  assert.match(preloadSource, /onBackendLifecycle:/);
  assert.match(preloadSource, /retryBackend:/);
});

test("renderer declarations expose the same lifecycle contract", () => {
  assert.match(rendererTypes, /export type BackendLifecycleEvent\s*=/);
  assert.match(rendererTypes, /state: "starting" \| "retrying"/);
  assert.match(rendererTypes, /state: "ready"; port: number/);
  assert.match(rendererTypes, /state: "failed" \| "exhausted"; message: string/);
  assert.match(rendererTypes, /onBackendLifecycle: \(callback: \(event: BackendLifecycleEvent\) => void\) => \(\) => void/);
  assert.match(rendererTypes, /retryBackend: \(\) => Promise<void>/);
});
