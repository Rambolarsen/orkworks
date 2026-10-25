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
async function fixture(t, { active = ["copilot"], detection, save, groupedStatus, section = "tools", settings = {}, saveRetention, verifyPeon } = {}) {
  const tools = [harness("codex", "Codex"), harness("copilot", "Copilot"), harness("plain", "Plain", null)];
  const calls = [];
  const savedSettings = [];
  const verifications = [];
  const previousWindow = globalThis.window;
  globalThis.window = { addEventListener() {}, removeEventListener() {}, orkworks: {
    getHarnessIntegrationStatus: async (id) => {
      calls.push(["detect", id]);
      return detection ? detection(id) : detected(id);
    },
    getGroupedHarnessIntegrationStatus: async (adapterId, targetId) => groupedStatus
      ? groupedStatus(adapterId, targetId)
      : { ok: true, group: { key: { adapterId, targetId }, consumers: [], status: detected(adapterId).status } },
    saveRetention: async (draft) => {
      calls.push(["retention", draft]);
      return saveRetention ? saveRetention(draft) : { ok: true };
    },
    saveHotkeys: async (draft) => {
      calls.push(["hotkeys", draft]);
      return { ok: true, settings: { ...props.initialSettings, hotkeys: structuredClone(draft) } };
    },
    getAppliedPeonProvider: async () => ({ provider: null, model: null, reasoningEffort: null, ollamaBaseUrl: null, appliedAt: null, connectionRevision: 0 }),
    verifyPeonProvider: async (provider, url) => {
      verifications.push([provider, url]);
      if (verifyPeon) return verifyPeon(provider, url);
      return { ok: false, provider, capabilities: { connectivity: false, modelDiscovery: false, providerDefault: false, testInference: false }, models: [], ollamaBaseUrl: null, generation: 0 };
    },
  } };
  const hotkeys = { newSession: "Ctrl+N", toggleSessionsPanel: "Ctrl+1", toggleDetailPanel: "Ctrl+2", toggleTerminalPanel: "Ctrl+3", toggleCapacityPanel: "Ctrl+4", toggleRecommendationsPanel: "Ctrl+5", resetLayout: null };
  const props = {
    initialSection: section,
    initialSettings: { version: 1, hotkeys, defaultHotkeys: hotkeys, retention: { maxSessions: 50, maxAgeDays: 30 }, debug: { showSessionIds: false, rendererHealthLogMs: 0 }, providers: { version: 2, revision: 0, peonModel: null, ollamaBaseUrl: "http://localhost:11434", providers: [] }, ...settings },
    currentWorkspacePath: "/workspace", updateStatus: null, updateCurrentVersion: null, updateChannel: null,
    onCheckForUpdates() {}, onDownloadUpdate() {}, harnesses: tools, documentRevision: null,
    onRefreshHarnesses: async () => ({ documentRevision: null, harnesses: tools }),
    activeHarnessIds: active, providerRuntime: null, onSectionChange() {}, onClose() {}, onSaved(settings) { savedSettings.push(settings); },
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
  const selectSection = (label) => nodes(render()).find((node) => node.props?.className?.startsWith("settings-nav-button") && text(node) === label).props.onClick();
  return { render, settle, calls, savedSettings, verifications, toggle, saveButton, close, unmount, selectSection, initialSettings: props.initialSettings };
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

test("restoring equivalent hotkey defaults ignores property insertion order", async (t) => {
  const defaults = { resetLayout: null, toggleRecommendationsPanel: "Ctrl+5", toggleCapacityPanel: "Ctrl+4", toggleTerminalPanel: "Ctrl+3", toggleDetailPanel: "Ctrl+2", toggleSessionsPanel: "Ctrl+1", newSession: "Ctrl+N" };
  const view = await fixture(t, { section: "hotkeys", settings: { defaultHotkeys: defaults } });
  const button = (label) => nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === label);
  button("Restore defaults").props.onClick();
  assert.equal(view.saveButton().props.disabled, true);
  assert.equal(button("Cancel").props.disabled, true);
  assert.deepEqual(view.calls, []);
});

for (const action of ["newSession", "toggleSessionsPanel", "toggleDetailPanel", "toggleTerminalPanel", "toggleCapacityPanel", "toggleRecommendationsPanel", "resetLayout"]) {
  test(`hotkey ${action} changes can be discarded and saved`, async (t) => {
    const defaults = { newSession: "Ctrl+N", toggleSessionsPanel: "Ctrl+1", toggleDetailPanel: "Ctrl+2", toggleTerminalPanel: "Ctrl+3", toggleCapacityPanel: "Ctrl+4", toggleRecommendationsPanel: "Ctrl+5", resetLayout: null };
    const saved = { ...defaults, [action]: "Ctrl+Shift+9" };
    const view = await fixture(t, { section: "hotkeys", settings: { hotkeys: saved, defaultHotkeys: defaults } });
    const button = (label) => nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === label);
    assert.equal(view.saveButton().props.disabled, true);
    button("Restore defaults").props.onClick();
    assert.equal(view.saveButton().props.disabled, false);
    button("Cancel").props.onClick();
    assert.equal(view.saveButton().props.disabled, true);
    assert.deepEqual(view.calls, []);
    button("Restore defaults").props.onClick();
    view.saveButton().props.onClick();
    await view.settle();
    assert.deepEqual(view.calls, [["hotkeys", defaults]]);
    assert.equal(view.saveButton().props.disabled, true);
    assert.equal(button("Cancel").props.disabled, true);
    assert.match(text(view.render()), /Saved/);
  });
}


for (const [field, label] of [["maxSessions", "Max sessions to keep"], ["maxAgeDays", "Auto-delete sessions older than (days)"]]) {
  test(`retention ${field} keeps draft edits local and saves bounded values on blur`, async (t) => {
    const view = await fixture(t, { section: "retention" });
    const input = () => nodes(view.render()).filter((node) => node.props?.className === "retention-row")
      .filter((row) => text(row).includes(label)).flatMap(nodes).find((node) => node.props?.type === "number");
    const initial = input().props.value;
    input().props.onChange({ target: { value: "75" } });
    assert.equal(input().props.value, 75);
    assert.deepEqual(view.calls, []);
    view.close();
    assert.equal(input().props.value, initial);
    for (const [raw, expected] of [["75", 75], ["-4", 0], ["1200", 999], ["3.8", 3], ["", 0]]) {
      input().props.onChange({ target: { value: raw } });
      input().props.onBlur({ target: { value: raw } });
      await view.settle();
      const expectedRetention = { maxSessions: 50, maxAgeDays: 30, [field]: expected };
      assert.deepEqual(view.calls.at(-1), ["retention", expectedRetention]);
      assert.deepEqual(view.savedSettings.at(-1).retention, expectedRetention);
      assert.match(text(view.render()), /Saved/);
      input().props.onChange({ target: { value: "22" } });
      view.close();
      assert.equal(input().props.value, expected);
      assert.doesNotMatch(text(view.render()), /Saved/);
    }
  });
}

for (const [name, saveRetention, status, committed] of [
  ["pending sidecar application", async () => ({ ok: true, retentionApplyStatus: { lastApplyError: "offline" } }), "Saved locally; sidecar pending", true],
  ["rejected persistence", async () => ({ ok: false }), "Session retention could not be saved.", false],
  ["transport failure", async () => { throw new Error("offline"); }, "Session retention could not be saved.", false],
]) {
  test(`retention preserves ${name} status and committed snapshot`, async (t) => {
    const view = await fixture(t, { section: "retention", saveRetention });
    const input = () => nodes(view.render()).find((node) => node.props?.type === "number");
    input().props.onBlur({ target: { value: "75" } });
    await view.settle();
    assert.ok(text(view.render()).includes(status));
    assert.equal(input().props.value, 75);
    assert.equal(view.savedSettings.length, committed ? 1 : 0);
    view.close();
    assert.equal(input().props.value, committed ? 75 : 50);
    assert.ok(!text(view.render()).includes(status));
  });
}

test("closing discards unsaved hotkeys and clears their saved status", async (t) => {
  const defaults = { newSession: "Ctrl+Shift+N", toggleSessionsPanel: "Ctrl+1", toggleDetailPanel: "Ctrl+2", toggleTerminalPanel: "Ctrl+3", toggleCapacityPanel: "Ctrl+4", toggleRecommendationsPanel: "Ctrl+5", resetLayout: null };
  const view = await fixture(t, { section: "hotkeys", settings: { defaultHotkeys: defaults } });
  const button = (label) => nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === label);
  button("Restore defaults").props.onClick();
  view.close();
  assert.equal(view.saveButton().props.disabled, true);
  assert.deepEqual(view.calls, []);
  button("Restore defaults").props.onClick();
  view.saveButton().props.onClick();
  await view.settle();
  assert.match(text(view.render()), /Saved/);
  view.close();
  assert.doesNotMatch(text(view.render()), /Saved/);
  assert.equal(view.saveButton().props.disabled, true);
});

for (const [name, providers, selection] of [
  ["legacy", { peonModel: "legacy-model", ollamaBaseUrl: "http://localhost:11434", providers: [] }, ["ollama", "http://localhost:11434"]],
  ["explicit", { peonSelection: { provider: "codex", model: "chosen-model" }, peonModel: "legacy-model", providers: [] }, ["codex", undefined]],
]) {
  test(`provider ${name} selection is consistent when opening and discarding`, async (t) => {
    const view = await fixture(t, { section: "providers", settings: { providers } });
    const selects = () => nodes(view.render()).filter((node) => node.type === "select");
    assert.deepEqual(view.verifications, [selection]);
    assert.equal(selects()[0].props.value, selection[0]);
    assert.equal(selects()[1].props.value, providers.peonSelection?.model ?? providers.peonModel);
    selects()[0].props.onChange({ target: { value: "other-provider" } });
    await view.settle();
    view.close();
    assert.equal(selects()[0].props.value, selection[0]);
    assert.equal(selects()[1].props.value, providers.peonSelection?.model ?? providers.peonModel);
  });
}


test("older retention success cannot replace a newer committed value", async (t) => {
  const first = deferred(), second = deferred();
  let count = 0;
  const view = await fixture(t, { section: "retention", saveRetention: () => ++count === 1 ? first.promise : second.promise });
  const input = () => nodes(view.render()).find((node) => node.props?.type === "number");
  input().props.onBlur({ target: { value: "60" } });
  input().props.onBlur({ target: { value: "70" } });
  second.resolve({ ok: true });
  await view.settle();
  first.resolve({ ok: true });
  await view.settle();
  assert.equal(view.savedSettings.at(-1).retention.maxSessions, 70);
  view.close();
  assert.equal(input().props.value, 70);
});

for (const outcome of ["failure", "rejection", "success"]) {
  test(`older retention ${outcome} cannot replace the current save status`, async (t) => {
    const first = deferred(), second = deferred();
    let count = 0;
    const view = await fixture(t, { section: "retention", saveRetention: () => ++count === 1 ? first.promise : second.promise });
    const input = () => nodes(view.render()).find((node) => node.props?.type === "number");
    input().props.onBlur({ target: { value: "60" } });
    input().props.onBlur({ target: { value: "70" } });
    if (outcome === "rejection") first.reject(new Error("old failure"));
    else first.resolve({ ok: outcome === "success" });
    await view.settle();
    assert.doesNotMatch(text(view.render()), /Saved|could not be saved/);
    second.resolve({ ok: true });
    await view.settle();
    assert.match(text(view.render()), /Saved/);
  });
}

test("an earlier successful retention save remains committed when its successor fails", async (t) => {
  const first = deferred(), second = deferred();
  let count = 0;
  const view = await fixture(t, { section: "retention", saveRetention: () => ++count === 1 ? first.promise : second.promise });
  const input = () => nodes(view.render()).find((node) => node.props?.type === "number");
  input().props.onBlur({ target: { value: "60" } });
  input().props.onBlur({ target: { value: "70" } });
  second.resolve({ ok: false });
  await view.settle();
  first.resolve({ ok: true });
  await view.settle();
  assert.match(text(view.render()), /could not be saved/);
  view.close();
  assert.equal(input().props.value, 60);
});

test("a delayed Debug response preserves a newer retention commit", async (t) => {
  const debug = deferred();
  const view = await fixture(t, { section: "debug" });
  window.orkworks.saveDebugSettings = () => debug.promise;
  nodes(view.render()).find((node) => node.props?.label === "Show debug metadata").props.onChange();
  view.selectSection("Session retention");
  nodes(view.render()).find((node) => node.props?.type === "number").props.onBlur({ target: { value: "75" } });
  await view.settle();
  debug.resolve({ ok: true, settings: { ...view.initialSettings, debug: { showSessionIds: true, rendererHealthLogMs: 0 } } });
  await view.settle();
  assert.equal(view.savedSettings.at(-1).retention.maxSessions, 75);
  assert.equal(view.savedSettings.at(-1).debug.showSessionIds, true);
});

for (const outcome of ["success", "rejection"]) {
  test(`older Debug ${outcome} cannot overwrite a newer toggle`, async (t) => {
    const first = deferred(), second = deferred();
    const view = await fixture(t, { section: "debug" });
    let count = 0;
    window.orkworks.saveDebugSettings = () => ++count === 1 ? first.promise : second.promise;
    const toggle = () => nodes(view.render()).find((node) => node.props?.label === "Show debug metadata");
    toggle().props.onChange();
    toggle().props.onChange();
    second.resolve({ ok: true, settings: view.initialSettings });
    await view.settle();
    if (outcome === "rejection") first.reject(new Error("old failure"));
    else first.resolve({ ok: true, settings: { ...view.initialSettings, debug: { showSessionIds: true, rendererHealthLogMs: 0 } } });
    await view.settle();
    assert.equal(toggle().props.checked, false);
    assert.equal(view.savedSettings.at(-1).debug.showSessionIds, false);
    assert.doesNotMatch(text(view.render()), /could not be saved/);
  });
}

test("a Hotkey save commits without replacing edits made while it was pending", async (t) => {
  const saving = deferred();
  const hotkeys = { newSession: "Ctrl+Shift+N", toggleSessionsPanel: "Ctrl+Shift+1", toggleDetailPanel: "Ctrl+2", toggleTerminalPanel: "Ctrl+3", toggleCapacityPanel: "Ctrl+4", toggleRecommendationsPanel: "Ctrl+5", resetLayout: null };
  const view = await fixture(t, { section: "hotkeys", settings: { hotkeys } });
  window.orkworks.saveHotkeys = () => saving.promise;
  const button = (label) => nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === label);
  button("Restore defaults").props.onClick();
  view.saveButton().props.onClick();
  button("Cancel").props.onClick();
  button("Reset").props.onClick();
  saving.resolve({ ok: true, settings: { ...view.initialSettings, hotkeys: view.initialSettings.defaultHotkeys } });
  await view.settle();
  assert.equal(view.savedSettings.at(-1).hotkeys.newSession, "Ctrl+N");
  assert.equal(view.saveButton().props.disabled, false);
  assert.match(text(view.render()), /Ctrl\+Shift\+1/);
  assert.doesNotMatch(text(view.render()), /Saved/);
});

for (const section of ["retention", "debug", "hotkeys"]) {
  for (const ending of ["close", "unmount"]) {
    for (const outcome of ["success", "rejection"]) {
      test(`${section} ignores save ${outcome} after ${ending}`, async (t) => {
        const saving = deferred();
        const view = await fixture(t, { section, saveRetention: () => saving.promise });
        if (section === "retention") nodes(view.render()).find((node) => node.props?.type === "number").props.onBlur({ target: { value: "75" } });
        if (section === "debug") {
          window.orkworks.saveDebugSettings = () => saving.promise;
          nodes(view.render()).find((node) => node.props?.label === "Show debug metadata").props.onChange();
        }
        if (section === "hotkeys") {
          window.orkworks.saveHotkeys = () => saving.promise;
          nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === "Restore defaults").props.onClick();
          view.saveButton().props.onClick();
        }
        view[ending]();
        if (outcome === "rejection") saving.reject(new Error("late failure"));
        else saving.resolve(section === "retention" ? { ok: true } : { ok: true, settings: view.initialSettings });
        await view.settle();
        assert.deepEqual(view.savedSettings, []);
        assert.doesNotMatch(text(view.render()), /Saved|could not be saved/);
      });
    }
  }
}


async function peonFixture(t) {
  const selection = { provider: "ollama", model: "chosen-model", ollamaBaseUrl: "http://localhost:11434" };
  const view = await fixture(t, {
    section: "providers",
    settings: { providers: { version: 2, revision: 0, peonSelection: selection, providers: [], ollamaBaseUrl: selection.ollamaBaseUrl } },
    verifyPeon: async (provider, url) => ({ ok: true, provider, models: ["chosen-model"], capabilities: { connectivity: true, modelDiscovery: true }, ollamaBaseUrl: url }),
  });
  const button = (label) => nodes(view.render()).find((node) => node.type?.name === "Button" && text(node) === label);
  window.orkworks.testAndApplyPeonProvider = async () => ({ ...selection, reasoningEffort: null, appliedAt: "now", connectionRevision: 1 });
  return { ...view, button };
}

for (const operation of ["Apply", "Save"]) {
  for (const ending of ["close", "unmount"]) {
    for (const outcome of ["success", "rejection"]) {
      test(`Peon ${operation} ignores ${outcome} after ${ending}`, async (t) => {
        const view = await peonFixture(t);
        const pending = deferred();
        if (operation === "Save") {
          view.button("Apply").props.onClick();
          await view.settle();
          window.orkworks.savePeonSelection = () => pending.promise;
        } else window.orkworks.testAndApplyPeonProvider = () => pending.promise;
        view.button(operation).props.onClick();
        view[ending]();
        const afterEnding = text(view.render());
        if (outcome === "rejection") pending.reject(new Error("late mutation failure"));
        else pending.resolve(operation === "Save"
          ? { ok: true, settings: view.initialSettings }
          : { provider: "ollama", model: "chosen-model", ollamaBaseUrl: "http://localhost:11434", appliedAt: "now", connectionRevision: 1 });
        await view.settle();
        assert.deepEqual(view.savedSettings, []);
        assert.equal(text(view.render()), afterEnding);
      });
    }
  }
}

for (const operation of ["Apply", "Save"]) {
  test(`an old Peon ${operation} cannot release a newer verification's busy state`, async (t) => {
    const view = await peonFixture(t);
    const mutation = deferred(), verification = deferred();
    if (operation === "Save") {
      view.button("Apply").props.onClick();
      await view.settle();
      window.orkworks.savePeonSelection = () => mutation.promise;
    } else window.orkworks.testAndApplyPeonProvider = () => mutation.promise;
    view.button(operation).props.onClick();
    window.orkworks.verifyPeonProvider = () => verification.promise;
    nodes(view.render()).find((node) => node.type === "select").props.onChange({ target: { value: "codex" } });
    mutation.reject(new Error("old mutation failure"));
    await view.settle();
    assert.equal(view.button("Refresh models").props.disabled, true);
    assert.doesNotMatch(text(view.render()), /old mutation failure/);
    verification.resolve({ ok: false, provider: "codex", models: [] });
    await view.settle();
    assert.equal(view.button("Refresh models").props.disabled, false);
  });
}

test("a delayed Peon Save response preserves a newer retention commit", async (t) => {
  const view = await peonFixture(t);
  view.button("Apply").props.onClick();
  await view.settle();
  const saving = deferred();
  window.orkworks.savePeonSelection = () => saving.promise;
  view.button("Save").props.onClick();
  view.selectSection("Session retention");
  nodes(view.render()).find((node) => node.props?.type === "number").props.onBlur({ target: { value: "75" } });
  await view.settle();
  saving.resolve({ ok: true, settings: view.initialSettings });
  await view.settle();
  assert.equal(view.savedSettings.at(-1).retention.maxSessions, 75);
});
