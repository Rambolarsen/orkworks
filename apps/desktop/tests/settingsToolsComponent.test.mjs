import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import * as React from "react";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/components/SettingsModal.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function harness(id, name, integration = { kind: id }) {
  return {
    id, name, retired: false, origin: "builtin", profile: null,
    compatibility: { profile: null, sessionSignals: null, integration: null },
    launch: { kind: "command-template", command: id, args: [], modelPrefix: null },
    defaultModel: null, resume: null, models: null, peon: null, capacity: null,
    sessionSignals: null, integration, voice: null,
  };
}

function detected(harnessId, toolDetected = true) {
  return { ok: true, status: {
    harnessId, enabled: false, toolDetected, registration: "absent", ownership: "none",
    activation: "inactive", coverage: "none", diagnostics: [], confirmation: null,
  } };
}

function nodes(tree) {
  if (Array.isArray(tree)) return tree.flatMap(nodes);
  if (!tree || typeof tree !== "object") return [];
  return [tree, ...nodes(tree.props?.children)];
}

function text(tree) {
  if (Array.isArray(tree)) return tree.map(text).join("");
  if (tree && typeof tree === "object") return text(tree.props?.children);
  return tree == null || typeof tree === "boolean" ? "" : String(tree);
}

// Exercise the real modal's handlers, state and effects; native IPC and its
// parent-owned save callback are the external dependencies. Child components
// remain React elements, so detection is supplied through their real callback.
async function fixture(t, { active = ["copilot"], detection, save, groupedStatus } = {}) {
  const tools = [harness("codex", "Codex"), harness("copilot", "Copilot"), harness("plain", "Plain", null)];
  const calls = [];
  const previousWindow = globalThis.window;
  globalThis.window = { addEventListener() {}, removeEventListener() {}, orkworks: {
    getHarnessIntegrationStatus: async (id) => {
      calls.push(["detect", id]);
      return detection ? detection(id) : detected(id);
    },
    getGroupedHarnessIntegrationStatus: async (adapterId, targetId) => groupedStatus
      ? groupedStatus(adapterId, targetId)
      : { ok: true, group: { key: { adapterId, targetId }, consumers: [], status: detected(adapterId).status } },
    getAppliedPeonProvider: async () => ({ provider: null, model: null, reasoningEffort: null, ollamaBaseUrl: null, appliedAt: null, connectionRevision: 0 }),
    verifyPeonProvider: async () => ({ ok: false, provider: "ollama", capabilities: { connectivity: false, modelDiscovery: false, providerDefault: false, testInference: false }, models: [], ollamaBaseUrl: null, generation: 0 }),
  } };
  const hotkeys = { newSession: "Ctrl+N", toggleSessionsPanel: "Ctrl+1", toggleDetailPanel: "Ctrl+2", toggleTerminalPanel: "Ctrl+3", toggleCapacityPanel: "Ctrl+4", toggleRecommendationsPanel: "Ctrl+5", resetLayout: null };
  const props = {
    initialSettings: { version: 1, hotkeys, defaultHotkeys: hotkeys, retention: { maxSessions: 50, maxAgeDays: 30 }, debug: { showSessionIds: false, rendererHealthLogMs: 0 }, providers: { version: 2, revision: 0, peonModel: null, ollamaBaseUrl: "http://localhost:11434", providers: [] } },
    currentWorkspacePath: "/workspace", updateStatus: null, updateCurrentVersion: null, updateChannel: null,
    onCheckForUpdates() {}, onDownloadUpdate() {}, harnesses: tools, documentRevision: null,
    onRefreshHarnesses: async () => ({ documentRevision: null, harnesses: tools }),
    activeHarnessIds: active, providerRuntime: null, onSectionChange() {}, onClose() {}, onSaved() {},
    onSaveActiveHarnesses: async (ids, scope) => {
      calls.push(["save", ids, scope]);
      return save ? save(ids, scope) : { activeHarnesses: { outcome: "persisted" }, integrations: {} };
    },
  };
  const values = [], dependencies = [], cleanups = [], effects = [];
  let stateIndex = 0, effectIndex = 0;
  const useEffect = (callback, deps) => {
    const index = effectIndex++;
    if (!dependencies[index] || deps.some((value, i) => !Object.is(value, dependencies[index][i]))) {
      effects.push(() => { cleanups[index]?.(); cleanups[index] = callback(); });
    }
    dependencies[index] = deps;
  };
  const hooks = { ...React,
    useState(initial) {
      const index = stateIndex++;
      if (!(index in values)) values[index] = typeof initial === "function" ? initial() : initial;
      return [values[index], (next) => { values[index] = typeof next === "function" ? next(values[index]) : next; }];
    },
    useRef(initial) { return hooks.useState({ current: initial })[0]; },
    useCallback(callback) { return callback; },
    useEffect, useLayoutEffect: useEffect,
  };
  const module = { exports: {} };
  new Function("require", "module", "exports", compiled.outputFiles[0].text)(
    (id) => id === "react" ? hooks : require(id), module, module.exports,
  );
  const render = () => { stateIndex = 0; effectIndex = 0; return module.exports.default(props); };
  const settle = async () => {
    for (let i = 0; i < 3; i++) {
      render();
      while (effects.length) effects.shift()();
      await new Promise(setImmediate);
    }
  };
  const unmount = () => { for (const cleanup of cleanups.splice(0)) cleanup?.(); };
  t.after(() => { unmount(); globalThis.window = previousWindow; });
  await settle();
  for (const node of nodes(render()).filter((node) => node.props?.onResult)) {
    node.props.onResult(node.props.harnessId, detected(node.props.harnessId));
  }
  const toggle = (name) => nodes(render()).find((node) => node.props?.ariaLabel === name);
  const saveButton = () => nodes(render()).find((node) => node.type?.name === "Button" && ["Save", "Saving..."].includes(text(node)));
  const close = () => nodes(render()).find((node) => node.props?.onClick && node.props["aria-label"] === "Close settings").props.onClick();
  return { render, settle, calls, toggle, saveButton, close, unmount };
}

test("turning off stays draft-only; immediate enable saves the live draft and scopes reconciliation", async (t) => {
  const view = await fixture(t);
  view.toggle("Copilot").props.onChange();
  assert.equal(view.calls.length, 0);
  assert.equal(view.toggle("Copilot").props.checked, false);
  view.toggle("Codex").props.onChange();
  assert.equal(view.toggle("Codex").props.checked, true);
  await view.settle();
  assert.deepEqual(view.calls, [["detect", "codex"], ["save", ["codex"], { adapterId: "codex", targetId: "workspace" }]]);
});

test("tools without an integration stay draft-only until Save", async (t) => {
  const view = await fixture(t);
  view.toggle("Plain").props.onChange();
  await view.settle();
  assert.deepEqual(view.calls, []);
  view.saveButton().props.onClick();
  await view.settle();
  assert.deepEqual(view.calls, [["save", ["copilot", "plain"], undefined]]);
});

test("an immediate enable busies only its own row; modal Save busies all rows", async (t) => {
  const pending = deferred();
  const view = await fixture(t, { save: () => pending.promise });
  view.toggle("Codex").props.onChange();
  await view.settle();
  assert.equal(view.toggle("Codex").props.disabled, true);
  assert.equal(view.toggle("Codex").props.checked, true);
  assert.equal(view.toggle("Copilot").props.disabled, false);
  assert.equal(view.saveButton().props.disabled, false);
  const pathControls = nodes(view.render()).filter((node) => node.props?.harnessName && node.props?.documentRevision !== undefined);
  assert.equal(pathControls.find((node) => node.props.harnessId === "codex").props.disabled, true);
  assert.equal(pathControls.find((node) => node.props.harnessId === "copilot").props.disabled, false);
  view.saveButton().props.onClick();
  await view.settle();
  assert.equal(view.toggle("Copilot").props.disabled, true);
  assert.equal(view.saveButton().props.disabled, true);
  assert.equal(text(view.saveButton()), "Saving...");
  pending.resolve({ activeHarnesses: { outcome: "persisted" }, integrations: {} });
  await view.settle();
  assert.equal(view.saveButton().props.disabled, false);
});

test("a disappeared tool is unchecked without saving its selection", async (t) => {
  const view = await fixture(t, { detection: (id) => detected(id, false) });
  view.toggle("Codex").props.onChange();
  await view.settle();
  assert.deepEqual(view.calls, [["detect", "codex"]]);
  assert.equal(view.toggle("Codex").props.checked, false);
  assert.equal(view.toggle("Codex").props.disabled, true);
  assert.match(text(view.render()), /This coding tool is no longer available/);
});

test("a superseded detection cannot start a save or clear the newer operation's busy state", async (t) => {
  const detection = deferred(), saving = deferred();
  const view = await fixture(t, { detection: () => detection.promise, save: () => saving.promise });
  view.toggle("Codex").props.onChange();
  view.saveButton().props.onClick();
  await view.settle();
  detection.resolve(detected("codex"));
  await view.settle();
  assert.deepEqual(view.calls, [["detect", "codex"], ["save", ["copilot", "codex"], undefined]]);
  assert.equal(view.saveButton().props.disabled, true);
  saving.resolve({ activeHarnesses: { outcome: "persisted" }, integrations: {} });
  await view.settle();
});

for (const outcome of ["success", "failure", "rejection"]) {
  test(`a superseded save ${outcome} cannot change the newer save's state`, async (t) => {
    const oldSave = deferred(), newSave = deferred();
    let saves = 0;
    const view = await fixture(t, { save: () => ++saves === 1 ? oldSave.promise : newSave.promise });
    view.toggle("Codex").props.onChange();
    await view.settle();
    view.toggle("Copilot").props.onChange();
    view.saveButton().props.onClick();
    await view.settle();
    if (outcome === "rejection") oldSave.reject(new Error("old failure"));
    else oldSave.resolve({ activeHarnesses: { outcome: outcome === "success" ? "persisted" : "failed", message: "old failure" }, integrations: {} });
    await view.settle();
    assert.equal(view.saveButton().props.disabled, true);
    assert.equal(view.toggle("Copilot").props.checked, false);
    assert.doesNotMatch(text(view.render()), /old failure|Couldn't enable/);
    newSave.resolve({ activeHarnesses: { outcome: "failed", message: "current failure" }, integrations: {} });
    await view.settle();
    assert.equal(view.saveButton().props.disabled, false);
    assert.match(text(view.render()), /current failure/);
  });
}

for (const kind of ["modal", "tool"]) {
  test(`${kind} save failures retain the operation-specific fallback message`, async (t) => {
    const view = await fixture(t, { save: async () => { throw new Error("transport failed"); } });
    if (kind === "modal") view.saveButton().props.onClick();
    else view.toggle("Codex").props.onChange();
    await view.settle();
    assert.ok(text(view.render()).includes(kind === "modal" ? "Couldn't save active coding tools." : "Couldn't enable this coding tool."));
    assert.equal(view.saveButton().props.disabled, false);
  });
}

test("closing Settings discards drafts and ignores pending save and status results", async (t) => {
  const saving = deferred(), status = deferred();
  const view = await fixture(t, { save: () => saving.promise, groupedStatus: () => status.promise });
  view.toggle("Codex").props.onChange();
  await view.settle();
  view.close();
  saving.resolve({ activeHarnesses: { outcome: "failed", message: "late save failure" }, integrations: {} });
  status.resolve({ ok: false, error: "late status failure" });
  await view.settle();
  assert.equal(view.toggle("Codex").props.checked, false);
  assert.equal(view.toggle("Copilot").props.checked, true);
  assert.doesNotMatch(text(view.render()), /late save failure|late status failure/);
});

test("unmounting during detection prevents a later save", async (t) => {
  const detection = deferred();
  const view = await fixture(t, { detection: () => detection.promise });
  view.toggle("Codex").props.onChange();
  view.unmount();
  detection.resolve(detected("codex"));
  await new Promise(setImmediate);
  assert.deepEqual(view.calls, [["detect", "codex"]]);
});

test("persisted selections retain integration failures and refresh detection; a successful retry clears the warning", async (t) => {
  let saves = 0;
  const view = await fixture(t, { save: async () => ({
    activeHarnesses: { outcome: "persisted" },
    integrations: { "codex/workspace": {
      key: { adapterId: "codex", targetId: "workspace" }, consumerHarnessIds: ["codex"],
      operation: "install", outcome: ++saves === 1 ? "failed" : "succeeded",
      registration: "absent", activation: "inactive", coverage: "none", message: "permission denied",
    } },
  }) });
  view.toggle("Codex").props.onChange();
  await view.settle();
  assert.equal(view.toggle("Codex").props.checked, true);
  assert.match(view.toggle("Codex").props.tooltip, /permission denied/);
  const detectionGenerations = () => nodes(view.render()).filter((node) => node.props?.onResult)
    .map((node) => [node.props.harnessId, node.props.refreshGeneration]);
  assert.deepEqual(detectionGenerations(), [["codex", 1], ["copilot", 0], ["plain", 0]]);
  view.saveButton().props.onClick();
  await view.settle();
  assert.doesNotMatch(view.toggle("Codex").props.tooltip, /permission denied/);
  assert.deepEqual(detectionGenerations(), [["codex", 2], ["copilot", 0], ["plain", 0]]);
});
