import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const mainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

test("Electron main centralizes initial and workspace sidecar startup", () => {
  assert.match(mainSource, /import \{ createSidecarLifecycle/);
  assert.equal(mainSource.match(/createSidecarLifecycle\(/g)?.length, 1);
  assert.equal(mainSource.match(/\bspawn\(/g)?.length, 1);
  assert.equal(mainSource.match(/ORKWORKS_OPEN_PLAN_TOKEN/g)?.length, 1);
  assert.match(mainSource, /sidecarLifecycle\.start\(initialSidecarCwd\)/);
  assert.match(mainSource, /sidecarLifecycle!\.start\(nextPath\)/);
});

test("Electron main restores workspace and settings before publishing ready", () => {
  assert.match(mainSource, /import \{ createBackendRestorationCoordinator, switchWorkspaceBackend, WorkspaceRestorationFailure/);
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

test("secondary restoration failures are logged without failing backend readiness", () => {
  assert.match(mainSource, /onStepFailure: \(step, error\) => \{\s*logBackendLifecycleFailure\(`restoration:\$\{step\}`, error\);/);
});

test("provider startup replay is serialized with settings mutations", () => {
  const start = mainSource.indexOf("async function syncSavedProviderSettings");
  const end = mainSource.indexOf("\n  function restorePersistedPeonSelection", start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const replay = mainSource.slice(start, end);
  assert.match(replay, /return enqueueSettingsWrite\(async \(\) => \{/);
  assert.match(replay, /const settings = currentSettings \?\? readSettings\(app\.getPath\("userData"\)\);/);
});

test("Electron main logs raw lifecycle failures but publishes only stable copy", () => {
  assert.match(mainSource, /sanitizeBackendLifecycleFailure/);
  assert.match(mainSource, /console\.error\(/);
  assert.match(mainSource, /lastBackendFailure = sanitizeBackendLifecycleFailure/);
  assert.doesNotMatch(mainSource, /publishBackendLifecycle\(\{ state: "failed", message: error\.message \}\)/);
});

test("a stale remembered workspace path degrades to no-workspace, not a backend failure", () => {
  assert.match(mainSource, /import \{ parseWorkspaceRestoreResponse \} from "\.\/workspaceRestore";/);
  assert.match(mainSource, /const restoreResult = await parseWorkspaceRestoreResponse\(response\);/);
  assert.match(mainSource, /if \(!restoreResult\.ok\) \{[\s\S]*if \(restoreResult\.removeFromHistory\)/);
  assert.doesNotMatch(mainSource, /throw new Error\(`Workspace restoration failed: \$\{response\.status\}`\)/);
});

test("a remembered 400/404 restoration rejects readiness, stops the attempted sidecar, and publishes picker", () => {
  assert.match(mainSource, /throw new WorkspaceRestorationFailure\(restoreResult\.status/);
  assert.match(mainSource, /if \(error instanceof WorkspaceRestorationFailure\) \{[\s\S]*sidecarLifecycle\?\.stop\(\);[\s\S]*publishBackendLifecycle\(\{ state: "picker" \}\);/);
  assert.match(mainSource, /state: "picker"/);
});

test("startup validates the remembered path as an accessible directory before starting a sidecar", () => {
  assert.match(mainSource, /accessibleWorkspaceDirectoryPath\(appMemory\.lastWorkspacePath\)/);
  assert.match(mainSource, /const initialSidecarCwd = initialWorkspacePath;/);
  assert.match(mainSource, /if \(initialSidecarCwd\) \{[\s\S]*sidecarLifecycle\.start\(initialSidecarCwd\)/);
});

test("backend readiness and retry use the lifecycle controller", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-backend-url", async \(\) => \{\s*const port = await restoration\.getReadiness\(\)/);
  assert.match(mainSource, /ipcMain\.handle\("retry-backend", async \(\) => \{[\s\S]*sidecarLifecycle\.retry\(\)/);
  assert.doesNotMatch(mainSource, /new Promise<number>\(\(resolve\) => \{\s*portResolve/);
});

test("retry cannot turn the stable picker into a ready null-workspace sidecar", () => {
  const start = mainSource.indexOf('ipcMain.handle("retry-backend"');
  const end = mainSource.indexOf('\n  });', start);
  assert.ok(start >= 0 && end > start);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /if \(!workspacePath\) \{[\s\S]*publishBackendLifecycle\(\{ state: "picker" \}\);[\s\S]*return;/);
});

test("initial workspace restoration handles rejected readiness", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /try \{[\s\S]*await restoration\.getReadiness\(\);[\s\S]*return \{ workspace: restoration\.getRestoredWorkspace\(\), historyDiagnostic: currentHistoryDiagnostic \};/);
  assert.match(handler, /return \{ workspace: null, historyDiagnostic: currentHistoryDiagnostic \};/);
});

test("history is persisted after restoration readiness without rolling back the ready workspace", () => {
  const rememberIndex = mainSource.indexOf("rememberRestoredWorkspace(workspace)");
  const readyIndex = mainSource.indexOf('publishBackendLifecycle({ state: "ready", port, workspace, historyDiagnostic });');
  assert.ok(rememberIndex >= 0);
  assert.ok(readyIndex > rememberIndex);

  const start = mainSource.indexOf("function rememberRestoredWorkspace");
  const end = mainSource.indexOf("\n  async function restoreWorkspace", start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const persistence = mainSource.slice(start, end);
  assert.match(persistence, /result\.diagnostic/);
  assert.match(persistence, /workspace history was not updated/);
  assert.doesNotMatch(persistence, /workspacePath\s*=/);
});

test("history diagnostics are carried through ready lifecycle state and remain visible with an active workspace", () => {
  assert.match(mainSource, /historyDiagnostic: currentHistoryDiagnostic/);
  assert.match(mainSource, /const diagnostic = result\.diagnostic;[\s\S]*currentHistoryDiagnostic = toWorkspaceHistoryDiagnostic\(diagnostic\)/);
  assert.match(appSource, /event\.state === "ready"[\s\S]*setWorkspaceHistoryDiagnostic\(event\.historyDiagnostic\)/);
  assert.match(appSource, /workspaceHistoryDiagnostic &&[\s\S]*Workspace history unavailable/);
});

test("workspace replacement stages the path before starting the replacement backend", () => {
  const start = mainSource.indexOf('ipcMain.handle("open-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /switchWorkspaceBackend\(\s*dirPath,/);
  assert.match(handler, /workspacePath = nextPath/);
  assert.doesNotMatch(handler, /sidecarLifecycle\.stop\(\)/);
});

test("Electron main replays the latest lifecycle state to late subscribers", () => {
  assert.match(mainSource, /let latestBackendLifecycle: BackendLifecycleEvent = \{ state: "picker" \}/);
  assert.match(mainSource, /ipcMain\.handle\("get-backend-lifecycle"/);
  assert.match(mainSource, /return latestBackendLifecycle/);
  assert.match(preloadSource, /subscribeBackendLifecycle\(/);
  assert.match(preloadSource, /ipcRenderer\.invoke\("get-backend-lifecycle"\)/);
  assert.doesNotMatch(mainSource, /event\.sender\.send\("orkworks:backend-lifecycle"/);
});

test("preload validates and forwards the lifecycle contract", () => {
  assert.match(preloadSource, /import \{ subscribeBackendLifecycle, type BackendLifecycleEvent, type InitialWorkspaceSnapshot \}/);
  assert.match(preloadSource, /subscribeBackendLifecycle\(/);
  assert.match(preloadSource, /onBackendLifecycle:/);
  assert.match(preloadSource, /retryBackend:/);
  assert.match(preloadSource, /getInitialWorkspace: \(\): Promise<InitialWorkspaceSnapshot>/);
});

test("renderer declarations expose the same lifecycle contract", () => {
  assert.match(rendererTypes, /export type BackendLifecycleEvent\s*=/);
  assert.match(rendererTypes, /state: "starting" \| "retrying"/);
  assert.match(rendererTypes, /state: "ready"; port: number; workspace: WorkspaceInfo \| null; historyDiagnostic: WorkspaceHistoryDiagnostic \| null/);
  assert.match(rendererTypes, /state: "failed" \| "exhausted"; message: string/);
  assert.match(rendererTypes, /export type WorkspaceHistoryDiagnostic/);
  assert.match(rendererTypes, /export type InitialWorkspaceSnapshot/);
  assert.match(rendererTypes, /getInitialWorkspace: \(\) => Promise<InitialWorkspaceSnapshot>/);
  assert.match(rendererTypes, /onBackendLifecycle: \(callback: \(event: BackendLifecycleEvent\) => void\) => \(\) => void/);
  assert.match(rendererTypes, /retryBackend: \(\) => Promise<void>/);
});

test("the picker exposes corrupt-history diagnostics without exposing persisted internals", () => {
  assert.match(appSource, /setWorkspaceHistoryDiagnostic\(snapshot\.historyDiagnostic\)/);
  assert.match(appSource, /role="alert"/);
  assert.match(appSource, /Workspace history unavailable/);
  assert.doesNotMatch(appSource, /recentWorkspacePaths/);
});
