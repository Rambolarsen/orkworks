import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import test from "node:test";
import type {
  BackendLifecycleEvent,
  BackendLifecycleWorkspace,
  WorkspaceHistoryDiagnostic,
} from "../electron/backendLifecycleEvent.ts";
import { DEFAULT_SETTINGS, type AppSettings } from "../electron/settingsMemory.ts";
import type { SidecarProcess } from "../electron/sidecarLifecycle.ts";
import {
  BACKEND_UNAVAILABLE_MESSAGE,
  createWorkspaceRuntime,
  StaleBackendGenerationError,
  type WorkspaceRuntimeOptions,
  type WorkspaceRuntimeSideEffectsContext,
} from "../electron/workspaceRuntime.ts";

const mainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const taskmasterIpcSource = readFileSync(new URL("../electron/taskmasterIpc.ts", import.meta.url), "utf8");

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

class FakeSidecarProcess extends EventEmitter implements SidecarProcess {
  stdout = new EventEmitter();
  stderr = new EventEmitter();
  pid: number;
  killed = false;
  exited = false;
  autoExitOnKill: boolean;
  onKill?: () => void;

  constructor(pid: number, autoExitOnKill = true) {
    super();
    this.pid = pid;
    this.autoExitOnKill = autoExitOnKill;
  }

  emitReady(port: number): void {
    this.stdout.emit("data", Buffer.from(`ORKWORKSD_PORT=${port}\n`));
  }

  emitExit(code: number | null = 0, signal: NodeJS.Signals | null = null): void {
    if (this.exited) return;
    this.exited = true;
    this.emit("exit", code, signal);
  }

  kill(signal?: NodeJS.Signals | number): boolean {
    this.killed = true;
    this.onKill?.();
    if (this.autoExitOnKill && !this.exited) {
      queueMicrotask(() => this.emitExit(0, typeof signal === "string" ? signal : null));
    }
    return true;
  }
}

function makeWorkspace(workspacePath: string, activeHarnessRevision = 2): BackendLifecycleWorkspace {
  return {
    path: workspacePath,
    workspaceIdentity: workspacePath,
    repo_root: null,
    branch: null,
    dirty: false,
    lastActiveSessionId: null,
    activeHarnessIds: [],
    activeHarnessRevision,
  };
}

interface HarnessOptions {
  initialHistoryDiagnostic?: WorkspaceHistoryDiagnostic | null;
  validateDestination?: (candidate: string) => string | null;
  canonicalizePath?: (candidate: string) => string | null;
  rememberWorkspacePath?: WorkspaceRuntimeOptions["rememberWorkspacePath"];
  forgetWorkspacePath?: WorkspaceRuntimeOptions["forgetWorkspacePath"];
  workspaceHandler?: (port: number, body: { path?: string; workspacePath?: string }, signal?: AbortSignal) => Promise<Response>;
  retentionHandler?: (port: number, signal?: AbortSignal) => Promise<Response>;
  providersHandler?: (port: number, signal?: AbortSignal) => Promise<Response>;
  autoExitOnKill?: boolean;
  cleanupTimeoutMs?: number;
  spawnError?: Error;
}

function createHarness(options: HarnessOptions = {}) {
  let settings: AppSettings = structuredClone(DEFAULT_SETTINGS);
  const processes: FakeSidecarProcess[] = [];
  const spawned: Array<{ cwd: string; token: string }> = [];
  const lifecycleEvents: BackendLifecycleEvent[] = [];
  const sideEffects: WorkspaceRuntimeSideEffectsContext[] = [];
  const remembered: string[] = [];
  const forgotten: string[] = [];
  const logs: Array<{ level: string; args: unknown[] }> = [];
  let tokenCounter = 0;

  const runtime = createWorkspaceRuntime({
    generateToken: () => `plan-token-${++tokenCounter}`,
    spawnSidecar: (cwd, token) => {
      spawned.push({ cwd, token });
      if (options.spawnError) throw options.spawnError;
      const proc = new FakeSidecarProcess(7000 + processes.length, options.autoExitOnKill ?? true);
      processes.push(proc);
      return proc;
    },
    sidecarSend: async (port, endpoint, init) => {
      if (endpoint === "workspace") {
        const body = (init?.jsonBody ?? {}) as { path?: string; workspacePath?: string };
        if (options.workspaceHandler) return options.workspaceHandler(port, body, init?.signal);
        return new Response(JSON.stringify(makeWorkspace(body.workspacePath ?? body.path ?? "/ws")), { status: 200 });
      }
      if (endpoint === "settings/retention") {
        if (options.retentionHandler) return options.retentionHandler(port, init?.signal);
        return new Response(null, { status: 204 });
      }
      if (endpoint === "providers") {
        if (options.providersHandler) return options.providersHandler(port, init?.signal);
        return new Response(JSON.stringify({ providers: [] }), { status: 200 });
      }
      return new Response(null, { status: 404 });
    },
    fetch: async () => new Response(JSON.stringify({ ok: true }), { status: 200 }),
    validateDestination: options.validateDestination ?? ((candidate) => candidate),
    canonicalizePath: options.canonicalizePath ?? ((candidate) => candidate),
    rememberWorkspacePath: options.rememberWorkspacePath ?? ((canonicalPath) => {
      remembered.push(canonicalPath);
      return { diagnostic: null };
    }),
    forgetWorkspacePath: options.forgetWorkspacePath ?? (async (rejectedPath) => {
      forgotten.push(rejectedPath);
      return { historyDiagnostic: null, navigationDiagnostic: null };
    }),
    getSettings: () => settings,
    writeSettings: (next) => { settings = next; },
    enqueueSettingsWrite: (operation) => Promise.resolve(operation()),
    clearProviderModelsCache: () => {},
    initialHistoryDiagnostic: options.initialHistoryDiagnostic ?? null,
    onLifecycleChange: (event) => lifecycleEvents.push(event),
    onReadySideEffects: (ctx) => sideEffects.push(ctx),
    timers: {
      setTimeout: (cb, ms) => setTimeout(cb, ms),
      clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
      now: () => Date.now(),
      ...(options.cleanupTimeoutMs === undefined ? {} : { cleanupTimeoutMs: options.cleanupTimeoutMs }),
    },
    logger: {
      log: (...args) => logs.push({ level: "log", args }),
      warn: (...args) => logs.push({ level: "warn", args }),
      error: (...args) => logs.push({ level: "error", args }),
    },
  });

  async function openReady(workspacePath: string, port: number): Promise<FakeSidecarProcess> {
    const index = processes.length;
    const pending = runtime.openWorkspace(workspacePath);
    await settle();
    const proc = processes[index];
    assert.ok(proc, `expected sidecar process for ${workspacePath}`);
    proc.emitReady(port);
    const result = await pending;
    assert.equal(result.ok, true);
    return proc;
  }

  return {
    runtime,
    processes,
    spawned,
    lifecycleEvents,
    sideEffects,
    remembered,
    forgotten,
    logs,
    openReady,
  };
}

test("startup stays in picker with no sidecar spawned and preserves initial history diagnostic", async () => {
  const diagnostic: WorkspaceHistoryDiagnostic = {
    code: "corrupt_history",
    message: "Workspace history could not be loaded.",
  };
  const harness = createHarness({ initialHistoryDiagnostic: diagnostic });

  assert.deepEqual(harness.runtime.getLifecycle(), { state: "picker" });
  assert.deepEqual(harness.runtime.getInitialWorkspaceSnapshot(), {
    workspace: null,
    historyDiagnostic: diagnostic,
  });
  assert.equal(harness.spawned.length, 0);

  // Retrying from the stable picker must not turn picker into a ready null-workspace sidecar.
  const retry = await harness.runtime.retry();
  assert.equal(retry.ok, false);
  assert.equal(retry.state, "picker");
  assert.equal(harness.spawned.length, 0);
  harness.runtime.dispose();
});

test("openWorkspace restores workspace, persists history, publishes ready, and fires side effects", async () => {
  const harness = createHarness();
  const subscriberEvents: BackendLifecycleEvent[] = [];
  const unsubscribe = harness.runtime.subscribeLifecycle((event) => subscriberEvents.push(event));

  await harness.openReady("/repo/alpha", 4301);

  assert.deepEqual(harness.spawned, [{ cwd: "/repo/alpha", token: "plan-token-1" }]);
  assert.deepEqual(harness.remembered, ["/repo/alpha"]);
  assert.equal(harness.runtime.getWorkspacePath(), "/repo/alpha");
  assert.equal(await harness.runtime.getBackendUrl(), "http://127.0.0.1:4301");
  assert.equal(harness.runtime.getOpenPlanToken(), "plan-token-1");
  assert.equal(harness.runtime.getActiveHarnessRevision(), 2);

  const lifecycle = harness.runtime.getLifecycle();
  assert.equal(lifecycle.state, "ready");
  assert.equal(lifecycle.port, 4301);
  assert.equal(lifecycle.workspace?.workspaceIdentity, "/repo/alpha");
  assert.equal(lifecycle.historyDiagnostic, null);
  assert.deepEqual(harness.lifecycleEvents.map((e) => e.state), ["opening", "ready"]);
  assert.deepEqual(subscriberEvents.map((e) => e.state), ["opening", "ready"]);

  assert.equal(harness.sideEffects.length, 1);
  assert.equal(harness.sideEffects[0].port, 4301);
  assert.equal(harness.sideEffects[0].signal.aborted, false);
  assert.equal(harness.sideEffects[0].isCurrent(), true);

  unsubscribe();
  harness.runtime.dispose();
});

test("pickWorkspace cancels cleanly on null selection and opens selected path", async () => {
  const harness = createHarness();

  const cancelled = await harness.runtime.pickWorkspace(async () => null);
  assert.equal(cancelled, null);
  assert.equal(harness.spawned.length, 0);
  assert.deepEqual(harness.runtime.getLifecycle(), { state: "picker" });

  const picking = harness.runtime.pickWorkspace(async () => "/repo/picked");
  await settle();
  harness.processes[0].emitReady(4302);
  const picked = await picking;
  assert.ok(picked && picked.ok);
  assert.equal(harness.runtime.getWorkspacePath(), "/repo/picked");
  harness.runtime.dispose();
});

test("readiness failure, 404 restoration failure, and 409 destination conflict remain distinct", async () => {
  // 1. Sidecar spawn failure before listening -> readiness_failed, returns cleanly to picker
  {
    const harness = createHarness({ spawnError: new Error("spawn ENOENT") });
    const result = await harness.runtime.openWorkspace("/repo/crash-boot");
    assert.equal(result.ok, false);
    if (!result.ok) {
      assert.equal(result.failure.code, "readiness_failed");
    }
    assert.equal(harness.runtime.getLifecycle().state, "picker");
    assert.deepEqual(harness.forgotten, []);
    harness.runtime.dispose();
  }

  // 2. Sidecar returns 404 on /workspace -> restoration_failed + forget from history + cleanup attempted sidecar
  {
    const harness = createHarness({
      workspaceHandler: async () => new Response(JSON.stringify({ error: "missing" }), { status: 404 }),
    });
    const opening = harness.runtime.openWorkspace("/repo/missing");
    await settle();
    harness.processes[0].emitReady(4310);
    const result = await opening;
    assert.equal(result.ok, false);
    if (!result.ok) {
      assert.equal(result.failure.code, "restoration_failed");
    }
    assert.deepEqual(harness.forgotten, ["/repo/missing"]);
    assert.equal(harness.processes[0].killed, true);
    assert.equal(harness.runtime.getWorkspacePath(), null);
    assert.equal(harness.runtime.getLifecycle().state, "picker");
    harness.runtime.dispose();
  }

  // 3. Sidecar returns 409 on /workspace -> typed destination_conflict without forgetting history
  {
    const harness = createHarness({
      workspaceHandler: async () => new Response(JSON.stringify({ error: "already owned" }), { status: 409 }),
    });
    const opening = harness.runtime.openWorkspace("/repo/conflict");
    await settle();
    harness.processes[0].emitReady(4311);
    const result = await opening;
    assert.equal(result.ok, false);
    if (!result.ok) {
      assert.equal(result.failure.code, "destination_conflict");
    }
    assert.deepEqual(harness.forgotten, []);
    assert.equal(harness.processes[0].killed, true);
    assert.equal(harness.runtime.getLifecycle().state, "picker");
    harness.runtime.dispose();
  }
});

test("secondary restoration step failures are logged without failing readiness", async () => {
  const harness = createHarness({
    retentionHandler: async () => new Response("retention down", { status: 500 }),
    providersHandler: async () => new Response("providers down", { status: 503 }),
  });

  await harness.openReady("/repo/secondary", 4320);

  assert.equal(harness.runtime.getLifecycle().state, "ready");
  assert.ok(
    harness.logs.some((entry) => entry.level === "error" && entry.args.includes("restoration:provider")),
    "provider sync step failure should be logged",
  );
  harness.runtime.dispose();
});

test("canonicalization or history write failures surface a safe diagnostic without rolling back ready", async () => {
  // Canonicalization failure
  {
    const harness = createHarness({ canonicalizePath: () => null });
    await harness.openReady("/repo/uncanonical", 4330);
    const lifecycle = harness.runtime.getLifecycle();
    assert.equal(lifecycle.state, "ready");
    assert.deepEqual(lifecycle.historyDiagnostic, {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
    assert.equal(harness.runtime.getWorkspacePath(), "/repo/uncanonical");
    assert.deepEqual(
      harness.runtime.getInitialWorkspaceSnapshot().historyDiagnostic,
      lifecycle.historyDiagnostic,
    );
    harness.runtime.dispose();
  }

  // History persistence error
  {
    const harness = createHarness({
      rememberWorkspacePath: () => ({
        diagnostic: { code: "history_write_failed", message: "Disk full" },
      }),
    });
    await harness.openReady("/repo/disk-full", 4331);
    const lifecycle = harness.runtime.getLifecycle();
    assert.equal(lifecycle.state, "ready");
    assert.deepEqual(lifecycle.historyDiagnostic, {
      code: "history_write_failed",
      message: "Disk full",
    });
    assert.equal(harness.runtime.getWorkspacePath(), "/repo/disk-full");
    harness.runtime.dispose();
  }
});

test("unexpected sidecar exit invalidates generation, aborts in-flight work, marks unresolved, and never auto-relaunches", async () => {
  const harness = createHarness();
  const proc = await harness.openReady("/repo/active", 4340);

  const generationBefore = harness.runtime.getGeneration();
  const readyEffect = harness.sideEffects[0];
  let requestSignal: AbortSignal | undefined;
  const inflight = deferred<string>();
  const request = harness.runtime.runReady(async ({ signal }) => {
    requestSignal = signal;
    return inflight.promise;
  });
  await settle();
  assert.equal(requestSignal?.aborted, false);
  assert.equal(readyEffect.signal.aborted, false);

  proc.emitExit(137);
  await settle();

  assert.equal(harness.runtime.getGeneration(), generationBefore + 1);
  assert.equal(requestSignal?.aborted, true);
  assert.equal(readyEffect.signal.aborted, true);
  assert.equal(readyEffect.isCurrent(), false);
  assert.equal(harness.spawned.length, 1, "sidecar must never auto-relaunch after unexpected exit");

  const lifecycle = harness.runtime.getLifecycle();
  assert.equal(lifecycle.state, "unresolved");
  assert.equal(lifecycle.failure.message, BACKEND_UNAVAILABLE_MESSAGE);

  inflight.resolve("late");
  await assert.rejects(request, StaleBackendGenerationError);
  await assert.rejects(harness.runtime.runReady(async () => "nope"), /Workspace transition is in progress/);
  harness.runtime.dispose();
});

test("generation is invalidated and in-flight requests abort at close admission before delayed cleanup finishes", async () => {
  const harness = createHarness({ autoExitOnKill: false });
  const proc = await harness.openReady("/repo/closing", 4350);

  const generationBefore = harness.runtime.getGeneration();
  const readyEffect = harness.sideEffects[0];
  let requestSignal: AbortSignal | undefined;
  const inflight = deferred<string>();
  const request = harness.runtime.runReady(async ({ signal }) => {
    requestSignal = signal;
    return inflight.promise;
  });
  await settle();

  const closing = harness.runtime.close();
  await settle();
  // Before the sidecar process has exited, close admission must already have fenced generation and aborted work.
  assert.equal(proc.exited, false);
  assert.equal(harness.runtime.getGeneration(), generationBefore + 1);
  assert.equal(requestSignal?.aborted, true);
  assert.equal(readyEffect.signal.aborted, true);
  assert.equal(readyEffect.isCurrent(), false);

  proc.emitExit(0);
  const closeResult = await closing;
  assert.equal(closeResult.ok, true);
  assert.equal(closeResult.state, "picker");

  inflight.resolve("late");
  await assert.rejects(request, StaleBackendGenerationError);
  harness.runtime.dispose();
});

test("quit and retry preserve sticky unresolved diagnostics when cleanup times out, while clean quit finalizes", async () => {
  // Sticky unresolved after cleanup timeout (ADR 0060).
  {
    const harness = createHarness({ autoExitOnKill: false, cleanupTimeoutMs: 25 });
    await harness.openReady("/repo/stuck", 4360);

    const quitResult = await harness.runtime.quit();
    assert.equal(quitResult.ok, false);
    assert.equal(quitResult.state, "unresolved");
    assert.equal(harness.runtime.getLifecycle().state, "unresolved");

    // Retry while unresolved stays fail-closed without spawning a replacement sidecar.
    const retryFailed = await harness.runtime.retry();
    assert.equal(retryFailed.ok, false);
    assert.equal(retryFailed.state, "unresolved");
    assert.equal(harness.spawned.length, 1);

    // Sticky cleanup failure also keeps a subsequent quit unresolved.
    const secondQuit = await harness.runtime.quit();
    assert.equal(secondQuit.ok, false);
    assert.equal(secondQuit.state, "unresolved");
    harness.runtime.dispose();
  }

  // Clean shutdown finalizes quit to picker.
  {
    const harness = createHarness({ autoExitOnKill: true });
    await harness.openReady("/repo/clean-quit", 4361);
    const finalQuit = await harness.runtime.quit();
    assert.equal(finalQuit.ok, true);
    assert.equal(finalQuit.state, "picker");
    harness.runtime.dispose();
  }
});

test("retry degrades invalid_destination back to picker cleanly after a non-sticky failure", async () => {
  let valid = true;
  const harness = createHarness({
    validateDestination: (candidate) => (valid ? candidate : null),
    spawnError: new Error("spawn ENOENT"),
  });
  const firstResult = await harness.runtime.openWorkspace("/repo/vanish");
  assert.equal(firstResult.ok, false);
  assert.equal(harness.runtime.getLifecycle().state, "picker");

  valid = false;
  const retry = await harness.runtime.retry();
  assert.equal(retry.ok, false);
  assert.equal(retry.state, "picker");
  assert.equal(retry.failure.code, "invalid_destination");
  assert.deepEqual(harness.runtime.getLifecycle(), { state: "picker" });
  harness.runtime.dispose();
});

test("navigation identity and activeHarnessRevision mutations are fenced to the active ready generation", async () => {
  const harness = createHarness();
  assert.throws(() => harness.runtime.getReadyNavigationIdentity(), /ready workspace/i);

  await harness.openReady("/repo/one", 4380);
  const nav = harness.runtime.getReadyNavigationIdentity();
  assert.equal(nav.workspaceIdentity, "/repo/one");
  assert.equal(nav.isCurrent(), true);

  const guard1 = harness.runtime.getWorkspaceGuard();
  assert.equal(guard1.workspacePath, "/repo/one");
  assert.equal(guard1.activeHarnessRevision, 2);
  assert.equal(harness.runtime.setActiveHarnessRevision(guard1.generation, "/repo/one", 5), true);
  assert.equal(harness.runtime.getActiveHarnessRevision(), 5);

  const switching = harness.runtime.openWorkspace("/repo/two");
  await settle();
  assert.equal(nav.isCurrent(), false);
  assert.equal(harness.runtime.setActiveHarnessRevision(guard1.generation, "/repo/one", 99), false);

  harness.processes[1].emitReady(4381);
  assert.equal((await switching).ok, true);
  assert.equal(harness.runtime.getActiveHarnessRevision(), 2);
  assert.equal(harness.runtime.setActiveHarnessRevision(guard1.generation, "/repo/one", 99), false);
  assert.equal(harness.runtime.getActiveHarnessRevision(), 2);
  harness.runtime.dispose();
});

// Cross-boundary IPC & renderer contract checks (main <-> preload <-> renderer types <-> App).

test("main delegates workspace lifecycle to WorkspaceRuntime without duplicate spawn or legacy channels", () => {
  assert.equal(mainSource.match(/createWorkspaceRuntime\(/g)?.length, 1);
  assert.equal(mainSource.match(/\bspawn\(/g)?.length, 1);
  assert.equal(mainSource.match(/ORKWORKS_OPEN_PLAN_TOKEN/g)?.length, 1);
  assert.doesNotMatch(mainSource, /createSidecarLifecycle\(/);
  assert.doesNotMatch(mainSource, /createBackendRestorationCoordinator\(/);
  assert.doesNotMatch(mainSource, /createWorkspaceSwitchCoordinator\(/);
  assert.doesNotMatch(mainSource, /withReadyBackendGeneration/);
  assert.doesNotMatch(mainSource, /function requestQuit[\s\S]*\.finally\(/);
});

test("shell rebuild IPC handlers gate writes on distinct native confirmations across preload and renderer", () => {
  assert.match(mainSource, /ipcMain\.handle\("rebuild-shell-layout", async \(\) => confirmShellMemoryRebuild\("layout",/);
  assert.match(mainSource, /ipcMain\.handle\("rebuild-workspace-navigation", async \(\) => confirmShellMemoryRebuild\("navigation",/);
  assert.match(preloadSource, /rebuildShellLayout: \(\): Promise<ShellMemoryRebuildResult> => ipcRenderer\.invoke\("rebuild-shell-layout"\)/);
  assert.match(preloadSource, /rebuildWorkspaceNavigation: \(\): Promise<ShellMemoryRebuildResult> => ipcRenderer\.invoke\("rebuild-workspace-navigation"\)/);
  assert.match(rendererTypes, /rebuildShellLayout: \(\) => Promise<ShellMemoryRebuildResult>/);
  assert.match(rendererTypes, /rebuildWorkspaceNavigation: \(\) => Promise<ShellMemoryRebuildResult>/);
  assert.doesNotMatch(mainSource, /rebuild-shell-layout", \(_event, confirmed/);
  assert.doesNotMatch(mainSource, /rebuild-workspace-navigation", \(_event, confirmed/);
});

test("preload and renderer contracts expose consolidated advice bridges and no legacy duplicates", () => {
  assert.match(preloadSource, /dismissAdviceRecommendation: \(workspacePath: string, id: string, reason\?: string\)/);
  assert.match(preloadSource, /acceptAdviceRecommendation: \(workspacePath: string, id: string, options: TaskmasterAcceptOptions\)/);
  assert.match(preloadSource, /requestAdviceAnalysis: \(workspacePath: string\): Promise<Record<string, unknown>> => ipcRenderer\.invoke\("request-advice-analysis", workspacePath\)/);
  assert.match(preloadSource, /applyDebugAttention: \(id: string, attention: string, message\?: string\)/);
  assert.match(rendererTypes, /dismissAdviceRecommendation: \(workspacePath: string, id: string, reason\?: string\) => Promise<Record<string, unknown>>/);
  assert.match(rendererTypes, /acceptAdviceRecommendation: \(workspacePath: string, id: string, options: AcceptRecommendationOptions\)/);
  assert.match(rendererTypes, /requestAdviceAnalysis: \(workspacePath: string\) => Promise<Record<string, unknown>>/);
  assert.match(rendererTypes, /applyDebugAttention: \(id: string, attention: SessionAttention, message\?: string\) => Promise<void>/);
  assert.match(taskmasterIpcSource, /deps\.withReady\(async \(port, token, signal\)/);
  assert.match(taskmasterIpcSource, /\['request-advice-analysis', 'analyze'\]/);

  for (const legacyChannel of [
    "analyze-taskmaster",
    "dismiss-taskmaster-recommendation",
    "accept-taskmaster-recommendation",
    "get-readiness-status",
    "get-improvement-status",
    "request-improvement",
    "cancel-improvement",
    "assess-repository-readiness",
    "assess-and-suggest",
    "get-taskmaster-run-status",
    "assess-taskmaster-workflow",
  ]) {
    assert.doesNotMatch(mainSource, new RegExp(legacyChannel), `legacy IPC channel ${legacyChannel} must not remain in main.ts`);
    assert.doesNotMatch(preloadSource, new RegExp(legacyChannel), `legacy IPC channel ${legacyChannel} must not remain in preload.ts`);
  }
  for (const legacyMethod of [
    "getTaskmasterRunStatus",
    "requestTaskmasterAnalysis",
    "getReadinessStatus",
    "getImprovementStatus",
    "requestImprovement",
    "cancelImprovement",
    "requestReadinessSuggestion",
    "requestReadinessAssessment",
    "requestTaskmasterAssessment",
    "dismissTaskmasterRecommendation",
    "acceptTaskmasterRecommendation",
  ]) {
    assert.doesNotMatch(preloadSource, new RegExp(`\\b${legacyMethod}\\b`), `legacy preload method ${legacyMethod} must not remain`);
    assert.doesNotMatch(rendererTypes, new RegExp(`\\b${legacyMethod}\\b`), `legacy window method ${legacyMethod} must not remain`);
  }
});

test("preload, renderer declarations, and App agree on lifecycle, retry, and history IPC contracts", () => {
  assert.match(preloadSource, /import \{ subscribeBackendLifecycle, type BackendLifecycleEvent, type BackendRetryResult, type InitialWorkspaceSnapshot \}/);
  assert.match(preloadSource, /onBackendLifecycle:/);
  assert.match(preloadSource, /retryBackend: \(\): Promise<BackendRetryResult> =>/);
  assert.match(preloadSource, /getInitialWorkspace: \(\): Promise<InitialWorkspaceSnapshot>/);
  assert.match(preloadSource, /getWorkspaceHistory: \(\): Promise<unknown> => ipcRenderer\.invoke\("get-workspace-history"\)/);
  assert.match(preloadSource, /pinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("pin-workspace-path", path\)/);
  assert.match(preloadSource, /unpinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("unpin-workspace-path", path\)/);
  assert.match(preloadSource, /forgetWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("forget-workspace-path", path\)/);
  assert.match(preloadSource, /openRememberedWorkspace: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("open-remembered-workspace", path\)/);
  assert.match(preloadSource, /completeWorkspaceNavigation: \(surface: LastCentralSurface\)/);

  assert.match(rendererTypes, /export type BackendLifecycleEvent\s*=/);
  assert.match(rendererTypes, /state: "opening" \| "closing"/);
  assert.match(rendererTypes, /state: "unresolved"; failure: WorkspaceLifecycleFailure/);
  assert.match(rendererTypes, /state: "ready"; port: number; workspace: WorkspaceInfo \| null; historyDiagnostic: WorkspaceHistoryDiagnostic \| null/);
  assert.match(rendererTypes, /export type BackendRetryResult =/);
  assert.match(rendererTypes, /export type InitialWorkspaceSnapshot/);
  assert.match(rendererTypes, /getWorkspaceHistory: \(\) => Promise<WorkspaceHistorySnapshot>;/);
  assert.match(rendererTypes, /openRememberedWorkspace: \(path: string\) => Promise<WorkspaceInfo \| null>;/);
  assert.match(rendererTypes, /completeWorkspaceNavigation: \(surface: LastCentralSurface\)/);
  assert.match(rendererTypes, /"pin_limit_reached"/);

  assert.match(appSource, /setWorkspaceHistoryDiagnostic\(snapshot\.historyDiagnostic\)/);
  assert.match(appSource, /event\.state === "ready"[\s\S]*setWorkspaceHistoryDiagnostic\(event\.historyDiagnostic\)/);
  assert.match(appSource, /Workspace history unavailable/);
  assert.match(appSource, /await window\.orkworks\.openRememberedWorkspace\(path\);/);
  assert.equal(appSource.match(/<WorkspaceHistoryDropdown/g)?.length, 1);
  assert.match(appSource, /<WorkspaceHistoryList\s+currentWorkspacePath=\{null\}/);
});
