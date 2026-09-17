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
  assert.match(mainSource, /workspaceSwitchCoordinator\.switchWorkspace\(initialSidecarCwd\)/);
  assert.match(mainSource, /sidecarLifecycle!\.start\(nextPath\)/);
});

test("Electron main restores workspace and settings before publishing ready", () => {
  assert.match(mainSource, /import \{ createBackendRestorationCoordinator, WorkspaceRestorationFailure/);
  assert.equal(mainSource.match(/createBackendRestorationCoordinator(?:<[^>]+>)?\(/g)?.length, 1);
  assert.match(mainSource, /restoreWorkspace: \(signal\) => restoreWorkspace\(port, signal\)/);
  assert.match(mainSource, /applyRetentionSettings: \(signal\) => applyRetentionSettings\(port, signal\)/);
  assert.match(mainSource, /syncProviderSettings: \(signal\) => syncSavedProviderSettings\(port, signal\)/);
  assert.match(mainSource, /async function restoreWorkspace\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /async function applyRetentionSettings\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /async function syncSavedProviderSettings\(port: number, signal: AbortSignal\)/);
  assert.match(mainSource, /onReady: \(port, workspace\) =>/);
  assert.match(mainSource, /state: "ready",\s*port: event\.port/);
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

test("a remembered 400/404 restoration rejects readiness and the coordinator cleans the attempted sidecar", () => {
  assert.match(mainSource, /throw new WorkspaceRestorationFailure\(restoreResult\.status/);
  assert.match(mainSource, /cleanupAttemptedRuntime: async \(\) => \{[\s\S]*restoration\.cancel[\s\S]*sidecarLifecycle\?\.stop\(\)/);
  assert.match(mainSource, /state: "picker"/);
});

test("startup validates the remembered path as an accessible directory before starting a sidecar", () => {
  assert.match(mainSource, /accessibleWorkspaceDirectoryPath\(appMemory\.lastWorkspacePath\)/);
  assert.match(mainSource, /const initialSidecarCwd = initialWorkspacePath;/);
  assert.match(mainSource, /if \(initialSidecarCwd\) void workspaceSwitchCoordinator\.switchWorkspace\(initialSidecarCwd\)/);
});

test("backend readiness and retry use the lifecycle controller", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-backend-url", async \(\) => \{\s*const port = await restoration\.getReadiness\(\)/);
  assert.match(mainSource, /ipcMain\.handle\("retry-backend", async \(\)(?:: Promise<BackendRetryResult>)? => \{[\s\S]*workspaceSwitchCoordinator\.retry\(\)/);
  assert.doesNotMatch(mainSource, /new Promise<number>\(\(resolve\) => \{\s*portResolve/);
});

test("sidecar failure recovery is explicit and stays on the serialized coordinator path", () => {
  const unavailableStart = mainSource.indexOf("onUnavailable: (message) => {");
  const unavailableEnd = mainSource.indexOf("\n      onState:", unavailableStart);
  assert.ok(unavailableStart >= 0 && unavailableEnd > unavailableStart);
  const unavailableHandler = mainSource.slice(unavailableStart, unavailableEnd);
  assert.match(unavailableHandler, /restoration\.fail\(/);
  assert.doesNotMatch(unavailableHandler, /\.retry\(/);
  assert.doesNotMatch(unavailableHandler, /sidecarLifecycle\.(start|stop|retry)\(/);

  const retryStart = mainSource.indexOf('ipcMain.handle("retry-backend"');
  const retryEnd = mainSource.indexOf("\n  });", retryStart);
  assert.ok(retryStart >= 0 && retryEnd > retryStart);
  const retryHandler = mainSource.slice(retryStart, retryEnd);
  assert.match(retryHandler, /workspaceSwitchCoordinator\.retry\(\)/);
  assert.doesNotMatch(retryHandler, /sidecarLifecycle\.retry\(\)/);
});

test("unexpected sidecar exit invalidates generation and cancels stale restoration readiness", () => {
  const start = mainSource.indexOf("onUnexpectedExit: (message) => {");
  const end = mainSource.indexOf("\n      onState:", start);
  assert.ok(start >= 0 && end > start);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /backendGeneration \+= 1/);
  assert.match(handler, /restoration\.cancel\(/);
  assert.match(handler, /workspaceSwitchCoordinator\?\.markUnresolved/);
});

test("quit does not finalize the app after unresolved workspace cleanup", () => {
  const start = mainSource.indexOf('app.on("before-quit"');
  const end = mainSource.indexOf("\n});", start);
  assert.ok(start >= 0 && end > start, "before-quit handler not found");
  const handler = mainSource.slice(start, end);
  const requestStart = mainSource.indexOf("function requestQuit");
  const requestEnd = mainSource.indexOf("\n}\n\napp.on(\"before-quit\"", requestStart);
  assert.ok(requestStart >= 0 && requestEnd > requestStart, "quit request helper not found");
  const request = mainSource.slice(requestStart, requestEnd);

  assert.match(handler, /event\.preventDefault\(\);[\s\S]*requestQuit\(\);/);
  assert.match(request, /\.then\(\(result\) => \{/);
  assert.match(request, /if \(!result\.ok\) \{[\s\S]*quitInProgress = false;[\s\S]*return;/);
  assert.match(request, /if \(!result\.ok\) \{[\s\S]*return;[\s\S]*killSidecar\(\);[\s\S]*app\.quit\(\);/);
  assert.doesNotMatch(request, /\.finally\(/);
});

test("repeated quit requests stay prevented while cleanup is pending", () => {
  const start = mainSource.indexOf('app.on("before-quit"');
  const end = mainSource.indexOf("\n});", start);
  assert.ok(start >= 0 && end > start, "before-quit handler not found");
  const handler = mainSource.slice(start, end);

  assert.match(
    handler,
    /if \(quitBypass\) \{\s*quitBypass = false;\s*return;\s*\}\s*event\.preventDefault\(\);\s*if \(quitInProgress\) return;/,
  );

  const requestStart = mainSource.indexOf("function requestQuit");
  const requestEnd = mainSource.indexOf("\n}\n\napp.on(\"before-quit\"", requestStart);
  assert.ok(requestStart >= 0 && requestEnd > requestStart, "quit request helper not found");
  const request = mainSource.slice(requestStart, requestEnd);
  assert.match(request, /quitBypass = true;\s*app\.quit\(\);/);
});

test("retry keeps cleanup-timeout unresolved diagnostics across the IPC contract", () => {
  const retryStart = mainSource.indexOf('ipcMain.handle("retry-backend"');
  const retryEnd = mainSource.indexOf("\n  });", retryStart);
  assert.ok(retryStart >= 0 && retryEnd > retryStart, "retry handler not found");
  const retryHandler = mainSource.slice(retryStart, retryEnd);

  assert.match(retryHandler, /Promise<BackendRetryResult>/);
  assert.match(retryHandler, /return \{ ok: false, state: "unresolved", failure: result\.failure \};/);
  assert.match(retryHandler, /return \{ ok: false, state: "picker", failure: result\.failure \};/);
  assert.doesNotMatch(retryHandler, /throw new Error\(result\.failure\.message\)/);
  assert.match(preloadSource, /retryBackend: \(\): Promise<BackendRetryResult> =>/);
  assert.match(rendererTypes, /export type BackendRetryResult =/);
  assert.match(rendererTypes, /retryBackend: \(\) => Promise<BackendRetryResult>/);

  const appStart = appSource.indexOf("const handleRetryBackend = useCallback");
  const appEnd = appSource.indexOf("const handleBackendUnavailable = useCallback");
  assert.ok(appStart >= 0 && appEnd > appStart, "renderer retry handler not found");
  const appHandler = appSource.slice(appStart, appEnd);
  assert.match(appHandler, /\.then\(\(result\) => \{/);
  assert.match(appHandler, /setWorkspaceSwitchDiagnostic\(result\.failure\.message\)/);
  assert.match(appHandler, /setBackendStatus\(result\.state\);/);
  assert.doesNotMatch(appHandler, /result\.state === "unresolved" \? "unresolved" : "unreachable"/);
});

test("retry cannot turn the stable picker into a ready null-workspace sidecar", () => {
  const start = mainSource.indexOf('ipcMain.handle("retry-backend"');
  const end = mainSource.indexOf('\n  });', start);
  assert.ok(start >= 0 && end > start);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /workspaceSwitchCoordinator\.retry\(\)/);
  assert.doesNotMatch(handler, /sidecarLifecycle\.retry\(\)/);
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

test("no-initial-workspace snapshots preserve the current history diagnostic", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /if \(!initialWorkspacePath\) return \{ workspace: null, historyDiagnostic: currentHistoryDiagnostic \};/);
});

test("history is persisted after restoration readiness without rolling back the ready workspace", () => {
  const rememberIndex = mainSource.indexOf("rememberWorkspace: (_path, workspace) => rememberRestoredWorkspace(workspace)");
  assert.ok(rememberIndex >= 0);
  assert.match(mainSource, /publishWorkspaceSwitchEvent\(event: WorkspaceSwitchEvent/);
  assert.match(mainSource, /state: "ready",\s*port: event\.port/);

  const start = mainSource.indexOf("function rememberRestoredWorkspace");
  const end = mainSource.indexOf("\n  async function restoreWorkspace", start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const persistence = mainSource.slice(start, end);
  assert.match(persistence, /result\.diagnostic/);
  assert.match(persistence, /workspace history was not updated/);
  assert.doesNotMatch(persistence, /workspacePath\s*=/);
});

test("workspace canonicalization failures surface a safe diagnostic while ready still publishes", () => {
  const start = mainSource.indexOf("function rememberRestoredWorkspace");
  const end = mainSource.indexOf("\n  async function restoreWorkspace", start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const persistence = mainSource.slice(start, end);
  assert.match(persistence, /if \(!canonicalPath\) \{[\s\S]*currentHistoryDiagnostic = \{[\s\S]*code: "history_write_failed"[\s\S]*message: "Workspace history could not be saved; the ready workspace was kept\."[\s\S]*\};[\s\S]*return currentHistoryDiagnostic;/);
  assert.match(mainSource, /publishBackendLifecycle\(\{[\s\S]*state: "ready",[\s\S]*historyDiagnostic: event\.historyDiagnostic/);
});

test("history diagnostics are carried through ready lifecycle state and remain visible with an active workspace", () => {
  assert.match(mainSource, /historyDiagnostic: currentHistoryDiagnostic/);
  assert.match(mainSource, /const diagnostic = result\.diagnostic;[\s\S]*currentHistoryDiagnostic = toWorkspaceHistoryDiagnostic\(diagnostic\)/);
  assert.match(appSource, /event\.state === "ready"[\s\S]*setWorkspaceHistoryDiagnostic\(event\.historyDiagnostic\)/);
  assert.match(appSource, /workspaceHistoryDiagnostic &&[\s\S]*Workspace history unavailable/);
});

test("workspace replacement is owned by the serialized coordinator", () => {
  const start = mainSource.indexOf('ipcMain.handle("open-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /workspaceSwitchCoordinator\.pickWorkspace\(/);
  assert.match(mainSource, /closeCurrentRuntime: async \(\) => \{[\s\S]*restoration\.cancel[\s\S]*await sidecarLifecycle\?\.stop\(\)/);
  assert.doesNotMatch(handler, /sidecarLifecycle\.(start|stop|retry)\(/);
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
  assert.match(preloadSource, /import \{ subscribeBackendLifecycle, type BackendLifecycleEvent, type BackendRetryResult, type InitialWorkspaceSnapshot \}/);
  assert.match(preloadSource, /subscribeBackendLifecycle\(/);
  assert.match(preloadSource, /onBackendLifecycle:/);
  assert.match(preloadSource, /retryBackend:/);
  assert.match(preloadSource, /getInitialWorkspace: \(\): Promise<InitialWorkspaceSnapshot>/);
});

test("renderer declarations expose the same lifecycle contract", () => {
  assert.match(rendererTypes, /export type BackendLifecycleEvent\s*=/);
  assert.match(rendererTypes, /state: "opening" \| "closing"/);
  assert.match(rendererTypes, /state: "unresolved"; failure: WorkspaceLifecycleFailure/);
  assert.match(rendererTypes, /failure\?: WorkspaceLifecycleFailure/);
  assert.match(rendererTypes, /state: "starting" \| "retrying"/);
  assert.match(rendererTypes, /state: "ready"; port: number; workspace: WorkspaceInfo \| null; historyDiagnostic: WorkspaceHistoryDiagnostic \| null/);
  assert.match(rendererTypes, /state: "failed" \| "exhausted"; message: string/);
  assert.match(rendererTypes, /export type WorkspaceHistoryDiagnostic/);
  assert.match(rendererTypes, /export type InitialWorkspaceSnapshot/);
  assert.match(rendererTypes, /getInitialWorkspace: \(\) => Promise<InitialWorkspaceSnapshot>/);
  assert.match(rendererTypes, /onBackendLifecycle: \(callback: \(event: BackendLifecycleEvent\) => void\) => \(\) => void/);
  assert.match(rendererTypes, /retryBackend: \(\) => Promise<BackendRetryResult>/);
});

test("the picker exposes corrupt-history diagnostics without exposing persisted internals", () => {
  assert.match(appSource, /setWorkspaceHistoryDiagnostic\(snapshot\.historyDiagnostic\)/);
  assert.match(appSource, /role="alert"/);
  assert.match(appSource, /Workspace history unavailable/);
  assert.doesNotMatch(appSource, /recentWorkspacePaths/);
});
