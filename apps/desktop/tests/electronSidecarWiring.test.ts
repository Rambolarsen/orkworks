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
  assert.doesNotMatch(mainSource, /switchWorkspace\(initialSidecarCwd\)/);
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

test("workspace startup keeps sidecar readiness and restoration failures distinct", () => {
  const startRuntime = mainSource.indexOf("startRuntime: async (nextPath, generation) => {");
  const cleanup = mainSource.indexOf("\n    cleanupAttemptedRuntime:", startRuntime);
  assert.ok(startRuntime >= 0 && cleanup > startRuntime, "workspace startup hook not found");
  const startup = mainSource.slice(startRuntime, cleanup);
  const sidecarAwait = startup.indexOf("await lifecycleReadiness.catch");
  const restorationAwait = startup.indexOf("await restoration.getReadiness");
  assert.ok(sidecarAwait >= 0 && restorationAwait > sidecarAwait, "restoration must follow sidecar readiness");
  assert.match(startup.slice(0, restorationAwait), /WorkspaceSwitchError\("readiness_failed"/);
  assert.match(startup.slice(restorationAwait), /WorkspaceSwitchError\("restoration_failed"/);
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

test("persisted Peon restoration is cancelled and admitted by the current ready backend", () => {
  const restoreStart = mainSource.indexOf("function restorePersistedPeonSelection");
  const restoreEnd = mainSource.indexOf("\n  const peonTransaction", restoreStart);
  assert.ok(restoreStart >= 0 && restoreEnd > restoreStart, "persisted Peon restore helper not found");
  const restore = mainSource.slice(restoreStart, restoreEnd);
  assert.match(restore, /const generation = backendGeneration/);
  assert.match(restore, /new AbortController\(\)/);
  assert.match(restore, /syncPersistedSelection\(selection, controller\.signal, port\)/);
  assert.match(restore, /generation === backendGeneration/);
  assert.match(restore, /latestBackendLifecycle\.state === "ready"/);
  const assignment = restore.indexOf("appliedPeonState = applied");
  const admission = restore.indexOf("if (!isCurrentReady())");
  assert.ok(admission >= 0 && assignment > admission, "stale Peon restore must be admitted before state mutation");

  assert.match(mainSource, /function cancelPersistedPeonSelectionRestore/);
  for (const hook of [
    "onCloseAdmission: () => {",
    "onUnexpectedExit: (message) => {",
    'if (state === "starting") {',
  ]) {
    const hookStart = mainSource.indexOf(hook);
    assert.ok(hookStart >= 0, `${hook} hook not found`);
    const nextBlock = mainSource.slice(hookStart, hookStart + 180);
    assert.match(nextBlock, /cancelPersistedPeonSelectionRestore\(\)/);
  }
});

test("Electron main logs raw lifecycle failures but publishes only stable copy", () => {
  assert.match(mainSource, /sanitizeBackendLifecycleFailure/);
  assert.match(mainSource, /console\.error\(/);
  assert.match(mainSource, /lastBackendFailure = sanitizeBackendLifecycleFailure/);
  assert.doesNotMatch(mainSource, /publishBackendLifecycle\(\{ state: "failed", message: error\.message \}\)/);
});

test("a stale remembered workspace path degrades to no-workspace, not a backend failure", () => {
  assert.match(mainSource, /import \{ buildWorkspaceRestoreRequest, parseWorkspaceRestoreResponse \} from "\.\/workspaceRestore";/);
  assert.match(mainSource, /const restoreResult = await parseWorkspaceRestoreResponse\(response\);/);
  assert.match(mainSource, /if \(!restoreResult\.ok\) \{[\s\S]*if \(restoreResult\.removeFromHistory\)/);
  assert.doesNotMatch(mainSource, /throw new Error\(`Workspace restoration failed: \$\{response\.status\}`\)/);
});

test("a remembered 400/404 restoration rejects readiness and the coordinator cleans the attempted sidecar", () => {
  assert.match(mainSource, /throw new WorkspaceRestorationFailure\(restoreResult\.status/);
  assert.match(mainSource, /cleanupAttemptedRuntime: async \(\) => \{[\s\S]*restoration\.cancel[\s\S]*sidecarLifecycle\?\.stop\(\)/);
  assert.match(mainSource, /state: "picker"/);
});

test("a remembered 409 restoration becomes a typed destination conflict", () => {
  assert.match(mainSource, /if \(restoreResult\.failureCode === "destination_conflict"\) \{[\s\S]*new WorkspaceSwitchError\("destination_conflict"/);
});

test("startup keeps remembered workspaces in the picker until explicit selection", () => {
  assert.match(mainSource, /const appMemory = readWorkspaceMemory\(app\.getPath\("userData"\)\);/);
  assert.match(mainSource, /workspaceSwitchCoordinator = createWorkspaceSwitchCoordinator[\s\S]*initialWorkspacePath: null/);
  assert.doesNotMatch(mainSource, /appMemory\.lastWorkspacePath/);
  assert.doesNotMatch(mainSource, /accessibleWorkspaceDirectoryPath\(appMemory\.lastWorkspacePath\)/);
  assert.doesNotMatch(mainSource, /const initialSidecarCwd = initialWorkspacePath;/);
  assert.doesNotMatch(mainSource, /workspaceSwitchCoordinator\.switchWorkspace\(initialSidecarCwd\)/);
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
  const requestEnd = mainSource.indexOf('app.on("before-quit"', requestStart);
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
  const requestEnd = mainSource.indexOf('app.on("before-quit"', requestStart);
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

test("initial workspace snapshots stay empty until explicit selection", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /workspace: null/);
  assert.match(handler, /historyDiagnostic: currentHistoryDiagnostic/);
  assert.doesNotMatch(handler, /restoration\.getReadiness\(\)/);
});

test("picker snapshots preserve the current history diagnostic", () => {
  const start = mainSource.indexOf('ipcMain.handle("get-initial-workspace"');
  const end = mainSource.indexOf('\n  });', start);
  assert.notEqual(start, -1);
  assert.notEqual(end, -1);
  const handler = mainSource.slice(start, end);
  assert.match(handler, /workspace: null/);
  assert.match(handler, /historyDiagnostic: currentHistoryDiagnostic/);
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

test("backend generation is invalidated at close admission before delayed cleanup", () => {
  const admissionStart = mainSource.indexOf("onCloseAdmission: () => {");
  const admissionEnd = mainSource.indexOf("\n    },", admissionStart);
  const closeStart = mainSource.indexOf("closeCurrentRuntime: async () => {");
  const closeEnd = mainSource.indexOf("\n    },", closeStart);
  assert.ok(admissionStart >= 0 && admissionEnd > admissionStart, "close admission hook not found");
  assert.ok(closeStart >= 0 && closeEnd > closeStart, "close runtime hook not found");
  assert.match(mainSource.slice(admissionStart, admissionEnd), /backendGeneration \+= 1/);
  assert.match(mainSource.slice(closeStart, closeEnd), /restoration\.cancel\([\s\S]*await sidecarLifecycle\?\.stop\(\)/);
  assert.ok(admissionStart < closeStart, "generation must change before cleanup is awaited");
});

test("generation-bound IPC owns Taskmaster mutations and debug attention injection", () => {
  assert.match(mainSource, /function withReadyBackendGeneration/);
  assert.match(mainSource, /ipcMain\.handle\("dismiss-taskmaster-recommendation"/);
  assert.match(mainSource, /ipcMain\.handle\("accept-taskmaster-recommendation"/);
  assert.match(mainSource, /ipcMain\.handle\("apply-debug-attention"/);
  for (const handlerName of [
    "dismiss-taskmaster-recommendation",
    "accept-taskmaster-recommendation",
    "apply-debug-attention",
  ]) {
    const handlerStart = mainSource.indexOf(`ipcMain.handle("${handlerName}"`);
    const handlerEnd = mainSource.indexOf("\n  });", handlerStart);
    assert.ok(handlerStart >= 0 && handlerEnd > handlerStart, `${handlerName} handler not found`);
    assert.match(mainSource.slice(handlerStart, handlerEnd), /withReadyBackendGeneration/);
  }
  const helperStart = mainSource.indexOf("async function withReadyBackendGeneration");
  const helperEnd = mainSource.indexOf("\n  ipcMain.handle(\"get-backend-lifecycle\"", helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, "generation-bound helper not found");
  const helper = mainSource.slice(helperStart, helperEnd);
  assert.match(helper, /latestBackendLifecycle\.state !== "ready"/);
  assert.match(helper, /generation !== backendGeneration/);
  assert.match(helper, /await operation\(/);
  const operationIndex = helper.indexOf("const result = await operation");
  assert.ok(operationIndex >= 0, "generation-bound operation is not awaited");
  assert.ok(
    helper.indexOf("assertCurrentReadyBackendGeneration(generation)", operationIndex) > operationIndex,
    "stale generation must be checked after the mutation settles",
  );
});

test("debug attention aborts its fetch when the backend generation changes", () => {
  const handlerStart = mainSource.indexOf('ipcMain.handle("apply-debug-attention"');
  const handlerEnd = mainSource.indexOf("\n  });", handlerStart);
  assert.ok(handlerStart >= 0 && handlerEnd > handlerStart, "debug attention handler not found");
  const handler = mainSource.slice(handlerStart, handlerEnd);
  assert.match(handler, /withReadyBackendGeneration\(async \(port, token, signal\)/);
  assert.match(handler, /signal: AbortSignal\.any\(\[signal, AbortSignal\.timeout\(15_000\)\]\)/);
});

test("Taskmaster mutations abort their fetch when the backend generation changes", () => {
  const helperStart = mainSource.indexOf("async function taskmasterMutationRequest");
  const helperEnd = mainSource.indexOf("\n  function normalizeRecommendationAcceptOptions", helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, "Taskmaster mutation helper not found");
  const helper = mainSource.slice(helperStart, helperEnd);
  assert.match(helper, /payload: unknown,\s*signal: AbortSignal/);
  assert.match(helper, /signal: AbortSignal\.any\(\[signal, AbortSignal\.timeout\(15_000\)\]\)/);

  for (const handlerName of [
    "dismiss-taskmaster-recommendation",
    "accept-taskmaster-recommendation",
  ]) {
    const handlerStart = mainSource.indexOf(`ipcMain.handle("${handlerName}"`);
    const handlerEnd = mainSource.indexOf("\n  });", handlerStart);
    assert.ok(handlerStart >= 0 && handlerEnd > handlerStart, `${handlerName} handler not found`);
    const handler = mainSource.slice(handlerStart, handlerEnd);
    assert.match(handler, /withReadyBackendGeneration\((?:async )?\(port, token, signal\)/);
    assert.match(handler, /taskmasterMutationRequest\([\s\S]*signal/);
  }
});

test("plan IPC operations use the ready generation admission guard", () => {
  for (const handlerName of ["get-plan-content", "request-plan-review", "select-terminal-plan"]) {
    const handlerStart = mainSource.indexOf(`ipcMain.handle("${handlerName}"`);
    const handlerEnd = mainSource.indexOf("\n  });", handlerStart);
    assert.ok(handlerStart >= 0 && handlerEnd > handlerStart, `${handlerName} handler not found`);
    assert.match(mainSource.slice(handlerStart, handlerEnd), /withReadyBackendGeneration/);
  }
});

test("in-flight plan IPC requests are cancelled when their backend generation closes", () => {
  assert.match(mainSource, /const generationBoundRequestControllers = new Set<AbortController>\(\)/);
  assert.match(mainSource, /function cancelGenerationBoundRequests\(\)[\s\S]*controller\.abort\(\)/);
  const helperStart = mainSource.indexOf("async function withReadyBackendGeneration");
  const helperEnd = mainSource.indexOf("\n  function taskmasterRecommendationPath", helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, "generation-bound helper not found");
  const helper = mainSource.slice(helperStart, helperEnd);
  assert.match(helper, /new AbortController\(\)/);
  assert.match(helper, /generationBoundRequestControllers\.add\(controller\)/);
  assert.match(helper, /operation\(port, token, controller\.signal\)/);
  assert.match(helper, /generationBoundRequestControllers\.delete\(controller\)/);
  const closeAdmission = mainSource.indexOf("onCloseAdmission: () => {");
  assert.ok(closeAdmission >= 0);
  assert.match(mainSource.slice(closeAdmission, closeAdmission + 220), /cancelGenerationBoundRequests\(\)/);
});

test("preload and renderer contracts expose only generation-bound mutation bridges", () => {
  assert.match(preloadSource, /dismissTaskmasterRecommendation: \(id: string, reason\?: string\)/);
  assert.match(preloadSource, /acceptTaskmasterRecommendation: \(id: string, options: TaskmasterAcceptOptions\)/);
  assert.match(preloadSource, /applyDebugAttention: \(id: string, attention: string, message\?: string\)/);
  assert.match(rendererTypes, /dismissTaskmasterRecommendation: \(id: string, reason\?: string\) => Promise<void>/);
  assert.match(rendererTypes, /acceptTaskmasterRecommendation: \(id: string, options: AcceptRecommendationOptions\)/);
  assert.match(rendererTypes, /applyDebugAttention: \(id: string, attention: SessionAttention, message\?: string\) => Promise<void>/);
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

test("open-remembered-workspace forgets stale paths that fail pre-sidecar validation", () => {
  const start = mainSource.indexOf('ipcMain.handle("open-remembered-workspace"');
  const end = mainSource.indexOf("\n  });", start);
  assert.ok(start >= 0 && end > start, "open-remembered-workspace handler not found");
  const handler = mainSource.slice(start, end);
  assert.match(handler, /result\.failure\.code === "invalid_destination"/);
  assert.match(handler, /forgetWorkspacePath\(app\.getPath\("userData"\), path\)/);
  assert.match(handler, /throw new Error\(result\.failure\.message\)/);
});

test("workspace history IPC channels are wired through main, preload, and the renderer contract", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-workspace-history", \(\) =>/);
  assert.match(mainSource, /ipcMain\.handle\("pin-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("unpin-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("forget-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("open-remembered-workspace", async \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /workspaceSwitchCoordinator\.getCurrentWorkspacePath\(\) === path\) return null;/);
  assert.match(mainSource, /await workspaceSwitchCoordinator\.switchWorkspace\(path\);/);

  assert.match(preloadSource, /getWorkspaceHistory: \(\): Promise<unknown> => ipcRenderer\.invoke\("get-workspace-history"\)/);
  assert.match(preloadSource, /pinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("pin-workspace-path", path\)/);
  assert.match(preloadSource, /unpinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("unpin-workspace-path", path\)/);
  assert.match(preloadSource, /forgetWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("forget-workspace-path", path\)/);
  assert.match(preloadSource, /openRememberedWorkspace: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("open-remembered-workspace", path\)/);

  assert.match(rendererTypes, /getWorkspaceHistory: \(\) => Promise<WorkspaceHistorySnapshot>;/);
  assert.match(rendererTypes, /openRememberedWorkspace: \(path: string\) => Promise<WorkspaceInfo \| null>;/);
  assert.match(rendererTypes, /"pin_limit_reached"/);
});

test("App wires the titlebar workspace-history dropdown to the remembered-open handler", () => {
  assert.match(appSource, /import \{ WorkspaceHistoryDropdown \} from "\.\/components\/WorkspaceHistoryDropdown";/);
  assert.equal(appSource.match(/<WorkspaceHistoryDropdown/g)?.length, 1);
  assert.match(appSource, /onOpenPath=\{handleOpenRememberedWorkspace\}/);
  assert.match(appSource, /onOpenOtherFolder=\{handleOpenWorkspace\}/);
  assert.match(appSource, /await window\.orkworks\.openRememberedWorkspace\(path\);/);
});

test("App renders the workspace-history list directly in the no-workspace picker screen", () => {
  assert.match(appSource, /import \{ WorkspaceHistoryList \} from "\.\/components\/WorkspaceHistoryList";/);
  assert.match(appSource, /!workspace && \(\s*<div className="workspace-picker-screen">/);
  assert.match(appSource, /<WorkspaceHistoryList\s+currentWorkspacePath=\{null\}/);
});
