import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { AppSettings } from "../src/appSettingsTypes.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";
import { createSettingsController, type SettingsControllerApi } from "../src/settingsController.ts";

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
