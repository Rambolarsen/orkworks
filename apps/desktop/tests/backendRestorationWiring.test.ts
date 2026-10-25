import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";
import { DEFAULT_SETTINGS, normalizeProviderSettings, type AppSettings } from "../electron/settingsMemory.ts";
import { WorkspaceRestorationFailure } from "../electron/backendRestoration.ts";
import { WorkspaceSwitchError } from "../electron/workspaceSwitchCoordinator.ts";

const main = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");

// Run the production main-process functions with only external boundaries
// replaced. These assertions exercise effects, not source-pattern spelling.
function loadMainSnippet<T>(startMarker: string, endMarker: string, setup: string, exports: string, injected: Record<string, unknown>): T {
  const start = main.indexOf(startMarker);
  const end = main.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `missing main-process helper: ${startMarker}`);
  const compiled = transpileModule(`${setup}\n${main.slice(start, end)}\nmodule.exports = ${exports};`, {
    compilerOptions: { module: ModuleKind.CommonJS, target: ScriptTarget.ES2022 },
  }).outputText;
  const module = { exports: {} as T };
  new Function("module", ...Object.keys(injected), compiled)(module, ...Object.values(injected));
  return module.exports;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

const settle = () => new Promise<void>(resolve => setImmediate(resolve));

function providerReplay(options: { queued?: Promise<void>; body?: Promise<unknown>; push?: Promise<unknown> } = {}) {
  const initialSettings = structuredClone(DEFAULT_SETTINGS);
  const writes: AppSettings[] = [];
  const requests: string[] = [];
  let clears = 0;
  const replay = loadMainSnippet<{ run(port: number, signal: AbortSignal): Promise<void>; settings(): AppSettings }>(
    "async function syncSavedProviderSettings(", "\n  let persistedPeonRestoreController",
    "let currentSettings = initialSettings;",
    "{ run: syncSavedProviderSettings, settings: () => currentSettings }", {
      initialSettings, normalizeProviderSettings,
      enqueueSettingsWrite: async (operation: () => Promise<void>) => { await options.queued; return operation(); },
      app: { getPath: () => "/test-settings" },
      readSettings: () => initialSettings,
      writeSettings: (_path: string, settings: AppSettings) => writes.push(settings),
      sidecarSend: async () => { requests.push("catalog"); return { ok: true, json: async () => options.body ?? { providers: [{ id: "replacement-provider", label: "Replacement" }] } }; },
      sidecarUrl: (port: number) => `http://127.0.0.1:${port}`,
      pushProviderSettings: async () => { requests.push("push"); return options.push ?? {}; },
      providerSettingsSyncError: () => null,
      providerModels: { clear: () => { clears++; } },
    },
  );
  return { ...replay, initialSettings, writes, requests, clears: () => clears };
}

test("cancelled provider replay waiting in the settings queue has no effects", async () => {
  const queued = deferred<void>();
  const replay = providerReplay({ queued: queued.promise });
  const controller = new AbortController();
  const pending = replay.run(4001, controller.signal);
  controller.abort();
  queued.resolve();
  await assert.rejects(pending, { name: "AbortError" });
  assert.deepEqual(replay.requests, []);
  assert.deepEqual(replay.writes, []);
  assert.deepEqual(replay.settings(), replay.initialSettings);
});

test("a stale provider catalog body cannot write settings or push them after cancellation", async () => {
  const body = deferred<unknown>();
  const replay = providerReplay({ body: body.promise });
  const controller = new AbortController();
  const pending = replay.run(4001, controller.signal);
  await settle();
  controller.abort();
  body.resolve({ providers: [{ id: "old-workspace-provider", label: "Old workspace" }] });
  await assert.rejects(pending, { name: "AbortError" });
  assert.deepEqual(replay.requests, ["catalog"]);
  assert.deepEqual(replay.writes, []);
  assert.deepEqual(replay.settings(), replay.initialSettings);
  assert.equal(replay.clears(), 0);
});

test("current provider replay still normalizes, persists and pushes settings", async () => {
  const replay = providerReplay();
  await replay.run(4002, new AbortController().signal);
  assert.equal(replay.writes.length, 1);
  assert.deepEqual(replay.requests, ["catalog", "push"]);
  assert.equal(replay.clears(), 1);
  assert.equal(replay.settings().providers.providers.some(provider => provider.id === "replacement-provider"), true);
});

test("a cancelled provider push cannot clear model state for a newer generation", async () => {
  const push = deferred<unknown>();
  const replay = providerReplay({ push: push.promise });
  const controller = new AbortController();
  const pending = replay.run(4001, controller.signal);
  await settle();
  controller.abort();
  push.resolve({});
  await assert.rejects(pending, { name: "AbortError" });
  assert.equal(replay.clears(), 0);
});

for (const outcome of ["success", "failure"] as const) {
  test(`rejected-workspace history cleanup ${outcome} cannot clear a replacement path`, async () => {
    const removal = deferred<void>();
    const warnings: unknown[][] = [];
    const restore = loadMainSnippet<{
      run(port: number, signal: AbortSignal): Promise<unknown>;
      replace(): void;
      snapshot(): unknown;
    }>("async function restoreWorkspace(", "\n  async function applyRetentionSettings(",
      'let workspacePath = "/old"; let workspaceDisplayPath = null; let currentHistoryDiagnostic = null;',
      '{ run: restoreWorkspace, replace: () => { workspacePath = "/replacement"; }, snapshot: () => ({ workspacePath, currentHistoryDiagnostic }) }', {
        sidecarSend: async () => ({}),
        buildWorkspaceRestoreRequest: () => ({}),
        parseWorkspaceRestoreResponse: async () => ({ ok: false, removeFromHistory: true, failureCode: "invalid_destination", status: 400 }),
        forgetRememberedWorkspaceWithNavigation: async () => {
          await removal.promise;
          if (outcome === "failure") throw new Error("history failure");
          return { history: { diagnostic: "old diagnostic" }, navigation: null };
        },
        app: { getPath: () => "/test-history" }, workspaceNavigationMemory: {},
        toWorkspaceHistoryDiagnostic: (value: unknown) => value,
        WorkspaceRestorationFailure, WorkspaceSwitchError,
        console: { warn: (...args: unknown[]) => warnings.push(args) },
      });
    const controller = new AbortController();
    const pending = restore.run(4001, controller.signal);
    await settle();
    controller.abort();
    restore.replace();
    removal.resolve();
    await assert.rejects(pending, { name: "AbortError" });
    assert.deepEqual(restore.snapshot(), { workspacePath: "/replacement", currentHistoryDiagnostic: null });
  });
}


function generationRequests(getReadiness: () => Promise<number>) {
  return loadMainSnippet<{
    run(operation: (port: number, token: string, signal: AbortSignal) => Promise<unknown>): Promise<unknown>;
    replace(): void;
  }>("const STALE_BACKEND_GENERATION_MESSAGE", "\n  function taskmasterRecommendationPath",
    'let backendGeneration = 1; let latestBackendLifecycle = { state: "ready" }; let openPlanToken = "old-token";',
    '{ run: withReadyBackendGeneration, replace: () => { cancelGenerationBoundRequests(); backendGeneration++; openPlanToken = "new-token"; } }',
    { restoration: { getReadiness } });
}

test("backend replacement while awaiting readiness cannot dispatch an old operation", async () => {
  const readiness = deferred<number>();
  const requests = generationRequests(() => readiness.promise);
  let dispatched = false;
  const pending = requests.run(async () => { dispatched = true; });
  requests.replace();
  readiness.resolve(4001);
  await assert.rejects(pending, /workspace changed/i);
  assert.equal(dispatched, false);
});

test("backend replacement aborts old operations and fences late results while new work succeeds", async () => {
  let port = 4001;
  const requests = generationRequests(async () => port);
  const oldResult = deferred<string>();
  let oldSignal: AbortSignal | undefined;
  const pending = requests.run(async (_port, _token, signal) => { oldSignal = signal; return oldResult.promise; });
  await settle();
  requests.replace();
  port = 4002;
  assert.equal(oldSignal?.aborted, true);
  const current = await requests.run(async (readyPort, token, signal) => {
    assert.equal(readyPort, 4002);
    assert.equal(token, "new-token");
    assert.equal(signal.aborted, false);
    return "current result";
  });
  oldResult.resolve("obsolete result");
  await assert.rejects(pending, /workspace changed/i);
  assert.equal(current, "current result");
});
