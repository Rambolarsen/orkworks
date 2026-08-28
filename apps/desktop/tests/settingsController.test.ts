import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { AppSettings } from "../src/appSettingsTypes.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";
import { createSettingsController, type SettingsControllerApi } from "../src/settingsController.ts";
import {
  createPeonSelectionTransaction,
  normalizePeonSelectionInput,
  type PeonSelectionTransport,
} from "../electron/peonSelectionTransaction.ts";

const mainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

const settings: AppSettings = {
  version: 1,
  hotkeys: {
    newSession: "CmdOrCtrl+N",
    toggleSessionsPanel: "CmdOrCtrl+Shift+S",
    toggleDetailPanel: "CmdOrCtrl+Shift+D",
    toggleTerminalPanel: "CmdOrCtrl+Shift+T",
    toggleCapacityPanel: "CmdOrCtrl+Shift+C",
    toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
    resetLayout: null,
  },
  defaultHotkeys: {
    newSession: "CmdOrCtrl+Alt+N",
    toggleSessionsPanel: "CmdOrCtrl+Alt+S",
    toggleDetailPanel: "CmdOrCtrl+Alt+D",
    toggleTerminalPanel: "CmdOrCtrl+Alt+T",
    toggleCapacityPanel: "CmdOrCtrl+Alt+C",
    toggleRecommendationsPanel: "CmdOrCtrl+Alt+R",
    resetLayout: "CmdOrCtrl+Alt+L",
  },
  retention: { maxSessions: 4, maxAgeDays: 9 },
  debug: { showSessionIds: false, rendererHealthLogMs: 0 },
  providers: {
    version: 1,
    revision: 2,
    peonModel: "small",
    ollamaBaseUrl: "http://127.0.0.1:11434",
    providers: [],
  },
};

test("Peon bridge keeps Apply separate from durable Save and exposes applied identity", () => {
  assert.match(mainSource, /ipcMain\.handle\("verify-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("test-and-apply-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("get-applied-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("save-peon-selection"/);
  assert.match(mainSource, /peonSelectionMatchesAppliedState/);

  const applyStart = mainSource.indexOf('ipcMain.handle("test-and-apply-peon-provider"');
  const saveStart = mainSource.indexOf('ipcMain.handle("save-peon-selection"');
  assert.notEqual(applyStart, -1);
  assert.notEqual(saveStart, -1);
  assert.doesNotMatch(mainSource.slice(applyStart, saveStart), /writeSettings\(/);
  assert.match(mainSource.slice(saveStart), /writeSettings\(/);

  assert.match(preloadSource, /verifyPeonProvider:/);
  assert.match(preloadSource, /testAndApplyPeonProvider:/);
  assert.match(preloadSource, /getAppliedPeonProvider:/);
  assert.match(preloadSource, /savePeonSelection:/);
  assert.match(rendererTypes, /verifyPeonProvider:/);
  assert.match(rendererTypes, /testAndApplyPeonProvider:/);
  assert.match(rendererTypes, /getAppliedPeonProvider:/);
  assert.match(rendererTypes, /savePeonSelection:/);
  assert.ok((mainSource.match(/async function parsePeonError/g)?.length ?? 0) >= 1);

  for (const route of [
    'ipcMain.handle("save-hotkeys"',
    'ipcMain.handle("save-retention"',
    'ipcMain.handle("save-debug-settings"',
    'ipcMain.handle("save-provider-settings"',
  ]) {
    const start = mainSource.indexOf(route);
    const end = mainSource.indexOf("ipcMain.handle(", start + route.length);
    assert.match(mainSource.slice(start, end === -1 ? undefined : end), /writeSettings\(/);
  }

  const savePeonStart = mainSource.indexOf('ipcMain.handle("save-peon-selection"');
  assert.match(mainSource.slice(savePeonStart), /writeSettings\(/);
});

function transactionHarness(overrides: Partial<PeonSelectionTransport> = {}) {
  const calls: string[] = [];
  const transport: PeonSelectionTransport = {
    discover: async () => ["llama3"],
    verify: async ({ provider, ollamaBaseUrl, generation }) => {
      calls.push(`verify:${generation}`);
      return {
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["gpt-5"],
        ollamaBaseUrl: provider === "ollama" ? ollamaBaseUrl ?? "http://127.0.0.1:11434" : null,
        generation,
      };
    },
    apply: async ({ selection, generation }) => {
      calls.push(`apply:${generation}`);
      return {
        provider: selection.provider,
        model: selection.model,
        ollamaBaseUrl: selection.provider === "ollama" ? selection.ollamaBaseUrl ?? null : null,
        appliedAt: "now",
        connectionRevision: 1,
      };
    },
    getApplied: async () => ({ provider: "copilot", model: "gpt-5", ollamaBaseUrl: null, appliedAt: "now", connectionRevision: 1 }),
    ...overrides,
  };
  return { calls, transaction: createPeonSelectionTransaction(transport) };
}

test("Peon transaction requires a successful matching Apply before Save", async () => {
  const { calls, transaction } = transactionHarness();
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  let persisted = 0;
  assert.deepEqual(
    await transaction.save(selection, async () => { persisted += 1; }),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
  assert.equal(persisted, 0);

  await transaction.verify(selection.provider);
  await transaction.apply(selection);
  assert.deepEqual(await transaction.save(selection, async () => { persisted += 1; }), { ok: true });
  assert.equal(persisted, 1);
  assert.deepEqual(calls, ["verify:1", "apply:1"]);
});

test("persisted Peon synchronization passes the already-known ready port", async () => {
  const ports: unknown[] = [];
  const { transaction } = transactionHarness({
    verify: async ({ provider, ollamaBaseUrl, generation, readyPort }) => {
      ports.push(readyPort);
      return {
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["gpt-5"],
        ollamaBaseUrl: null,
        generation,
      };
    },
    apply: async ({ selection, readyPort }) => {
      ports.push(readyPort);
      return {
        provider: selection.provider,
        model: selection.model,
        ollamaBaseUrl: null,
        appliedAt: "now",
        connectionRevision: 1,
      };
    },
  });

  await transaction.syncPersistedSelection({ provider: "copilot", model: "gpt-5" }, undefined, 43123);
  assert.deepEqual(ports, [43123, 43123]);
});

test("Peon Save rejects matching sidecar state without a local successful Apply", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    getApplied: async () => ({
      provider: selection.provider,
      model: selection.model,
      ollamaBaseUrl: null,
      appliedAt: "now",
      connectionRevision: 1,
    }),
  });

  await transaction.verify(selection.provider);
  assert.deepEqual(
    await transaction.save(selection, async () => {}),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
});

test("Peon Save rejects an Apply from an older generation even when sidecar identity matches", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    getApplied: async () => ({
      provider: selection.provider,
      model: selection.model,
      ollamaBaseUrl: null,
      appliedAt: "now",
      connectionRevision: 1,
    }),
  });

  await transaction.verify(selection.provider);
  await transaction.apply(selection);
  await transaction.verify(selection.provider);
  assert.deepEqual(
    await transaction.save(selection, async () => {}),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
});

test("Peon discovery does not mutate or supersede the Apply transaction", async () => {
  let resolveOld!: (value: Awaited<ReturnType<PeonSelectionTransport["verify"]>>) => void;
  const { transaction } = transactionHarness({
    verify: ({ provider, generation }) => provider === "copilot"
      ? new Promise((resolve) => { resolveOld = resolve; })
      : Promise.resolve({
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["llama3"],
        ollamaBaseUrl: "http://custom-ollama:11434",
        generation,
      }),
  });
  const oldVerification = transaction.verify("copilot");
  const discovery = await transaction.discover("ollama", "http://custom-ollama:11434");
  assert.deepEqual(discovery, ["llama3"]);
  resolveOld({
    ok: true,
    provider: "copilot",
    capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
    models: ["gpt-5"],
    ollamaBaseUrl: null,
    generation: 1,
  });
  await oldVerification;
});

test("compatibility model discovery uses sidecar discovery without the transaction coordinator", () => {
  assert.doesNotMatch(mainSource, /providerModelDiscoveryGeneration/);
  const discoveryStart = mainSource.indexOf('ipcMain.handle("get-provider-models"');
  assert.notEqual(discoveryStart, -1);
  const discoveryRoute = mainSource.slice(discoveryStart);
  assert.match(discoveryRoute, /settings\/providers\/\$\{encodeURIComponent\(providerId\)\}\/models/);
  assert.doesNotMatch(discoveryRoute, /peonTransaction\.discover\(/);
  assert.doesNotMatch(discoveryRoute, /settings\/peon\/provider\/verify/);
});

test("provider model cache is keyed and invalidated by the Ollama base URL", () => {
  assert.match(mainSource, /providerModelCacheKey/);
  assert.match(mainSource, /providerModelCacheKey\(providerId, ollamaBaseUrl\)/);
  assert.match(mainSource, /previousOllamaBaseUrl/);
  assert.match(mainSource, /providerModels\.delete\(providerModelCacheKey\("ollama", previousOllamaBaseUrl\)\)/);
});

test("Peon selection input resolves one persisted custom Ollama URL", () => {
  assert.deepEqual(
    normalizePeonSelectionInput(
      { provider: "ollama", model: "llama3.2:3b" },
      "https://ollama.example.test:11434",
    ),
    {
      provider: "ollama",
      model: "llama3.2:3b",
      ollamaBaseUrl: "https://ollama.example.test:11434",
    },
  );
});

test("persisted Peon synchronization fails instead of allowing readiness to continue", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    apply: async () => { throw new Error("inference unavailable"); },
  });
  await assert.rejects(transaction.syncPersistedSelection(selection), /inference unavailable/);
});

function clone<T>(value: T): T {
  return structuredClone(value);
}

function apiFor(overrides: Partial<SettingsControllerApi> = {}) {
  const calls: string[] = [];
  const api: SettingsControllerApi = {
    getSettings: async () => clone(settings),
    saveHotkeys: async (value) => { calls.push("hotkeys"); return { ok: true, settings: { ...clone(settings), hotkeys: clone(value) } }; },
    saveRetention: async (value) => { calls.push("retention"); return { ok: true }; },
    saveDebugSettings: async (value) => { calls.push("debug"); return { ok: true, settings: { ...clone(settings), debug: clone(value) } }; },
    saveProviderSettings: async (value) => { calls.push("providers"); return { ok: true, settings: { ...clone(settings), providers: clone(value) } }; },
    verifyOllama: async () => ({ ok: true, normalizedBaseUrl: settings.providers.ollamaBaseUrl, status: "connected", reasonCode: "connected", httpStatus: 200, models: [], excludedModels: [], diagnostic: null }),
    ...overrides,
  };
  return { api, calls };
}

test("draft edits are isolated and discard restores the committed snapshot", async () => {
  const { api } = apiFor();
  const controller = createSettingsController(api);
  const loaded = await controller.load();
  controller.updateDraft("retention", { maxSessions: 99, maxAgeDays: 9 });
  assert.equal(loaded.retention.maxSessions, 4);
  assert.equal(controller.snapshot().draft.retention.maxSessions, 99);
  controller.discard();
  assert.deepEqual(controller.snapshot().draft, controller.snapshot().committed);
});

test("resetHotkey uses Electron-provided nullable defaults", async () => {
  const { api } = apiFor();
  const controller = createSettingsController(api);
  await controller.load();
  controller.updateDraft("hotkeys", { ...settings.hotkeys, resetLayout: "CmdOrCtrl+L" });
  controller.resetHotkey("resetLayout");
  assert.equal(controller.snapshot().draft.hotkeys.resetLayout, settings.defaultHotkeys.resetLayout);
});

test("verification is diagnostic and a late rejection cannot replace a newer result", async () => {
  let rejectOld!: (error: Error) => void;
  let resolveNew!: (value: Awaited<ReturnType<SettingsControllerApi["verifyOllama"]>>) => void;
  const { api } = apiFor({
    verifyOllama: (url) => url.includes("old")
      ? new Promise((_resolve, reject) => { rejectOld = reject; })
      : new Promise((resolve) => { resolveNew = resolve; }),
  });
  const controller = createSettingsController(api);
  await controller.load();
  const old = controller.verifyOllama("http://old");
  const newer = controller.verifyOllama("http://new");
  resolveNew({ ok: true, normalizedBaseUrl: "http://new", status: "connected", reasonCode: "connected", httpStatus: 200, models: [], excludedModels: [], diagnostic: null });
  await newer;
  rejectOld(new Error("old request failed"));
  await assert.rejects(old);
  assert.equal(controller.snapshot().verification?.normalizedBaseUrl, "http://new");
  assert.equal(controller.snapshot().draft.providers.ollamaBaseUrl, settings.providers.ollamaBaseUrl);
});

test("verification is diagnostic and a late success cannot replace a newer result", async () => {
  let resolveOld!: (value: Awaited<ReturnType<SettingsControllerApi["verifyOllama"]>>) => void;
  let resolveNew!: (value: Awaited<ReturnType<SettingsControllerApi["verifyOllama"]>>) => void;
  const { api } = apiFor({
    verifyOllama: (url) => url.includes("old")
      ? new Promise((resolve) => { resolveOld = resolve; })
      : new Promise((resolve) => { resolveNew = resolve; }),
  });
  const controller = createSettingsController(api);
  await controller.load();
  const old = controller.verifyOllama("http://old");
  const newer = controller.verifyOllama("http://new");
  resolveNew({ ok: true, normalizedBaseUrl: "http://new", status: "connected", reasonCode: "connected", httpStatus: 200, models: [], excludedModels: [], diagnostic: null });
  await newer;
  resolveOld({ ok: true, normalizedBaseUrl: "http://old", status: "connected", reasonCode: "connected", httpStatus: 200, models: [], excludedModels: [], diagnostic: null });
  await old;
  assert.equal(controller.snapshot().verification?.normalizedBaseUrl, "http://new");
});

test("commit saves changed domains in deterministic order", async () => {
  const { api, calls } = apiFor();
  const controller = createSettingsController(api);
  await controller.load();
  controller.updateDraft("providers", { ...settings.providers, peonModel: "large" });
  controller.updateDraft("debug", { ...settings.debug, showSessionIds: true });
  controller.updateDraft("retention", { ...settings.retention, maxSessions: 8 });
  controller.updateDraft("hotkeys", { ...settings.hotkeys, newSession: "CmdOrCtrl+Alt+N" });
  const result = await controller.commit();
  assert.equal(result.ok, true);
  assert.deepEqual(calls, ["hotkeys", "retention", "debug", "providers"]);
});

test("a failed domain retains the complete draft and reports that domain", async () => {
  const { api } = apiFor({ saveRetention: async () => { throw new Error("disk full"); } });
  const controller = createSettingsController(api);
  await controller.load();
  const draft = { ...settings.retention, maxSessions: 77 };
  controller.updateDraft("retention", draft);
  const result = await controller.commit();
  assert.equal(result.ok, false);
  assert.equal(result.failedDomain, "retention");
  assert.deepEqual(controller.snapshot().draft.retention, draft);
});

test("successful provider persistence preserves a stale or pending sidecar result", async () => {
  const sidecar = { appliedRevision: null, appliedAt: null, lastApplyError: "sidecar unavailable" };
  const { api } = apiFor({
    saveProviderSettings: async (value) => ({ ok: true, settings: { ...clone(settings), providers: clone(value) }, providerApplyStatus: sidecar }),
  });
  const controller = createSettingsController(api);
  await controller.load();
  controller.updateDraft("providers", { ...settings.providers, peonModel: "large" });
  const result = await controller.commit();
  assert.equal(result.ok, true);
  assert.deepEqual(result.providerApplyStatus, sidecar);
});

test("successful retention persistence preserves a stale or pending sidecar result", async () => {
  const sidecar = { appliedRevision: null, appliedAt: null, lastApplyError: "sidecar unavailable" };
  const { api } = apiFor({
    saveRetention: async () => ({ ok: true, retentionApplyStatus: sidecar }),
  });
  const controller = createSettingsController(api);
  await controller.load();
  controller.updateDraft("retention", { ...settings.retention, maxSessions: 8 });
  const result = await controller.commit();
  assert.equal(result.ok, true);
  assert.deepEqual(result.retentionApplyStatus, sidecar);
});
