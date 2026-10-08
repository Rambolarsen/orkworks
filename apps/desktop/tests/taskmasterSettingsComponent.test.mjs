import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import * as React from "react";

const require = createRequire(import.meta.url);
const compile = (component) => build({
  entryPoints: [new URL(`../src/components/${component}.tsx`, import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
  external: ["./InferenceTrustSettings"],
});
const compiled = await compile("TaskmasterSettings");
const compiledTrust = await compile("InferenceTrustSettings");

// Drive this component's state/effect boundary without replacing its render,
// selection, save, or discovery logic. IPC is the external dependency.
async function fixture(t, { state = "execution_inactive", missing = false, effort, trust = false, provider = "custom", workspacePath = null, knowledgeError = null, runStatus = { workspacePath: "/workspace", activeAttempt: null, latestOutcome: null }, pendingModels = null } = {}) {
  const settings = { enabled: true, selection: { provider, model: " vendor/opaque model ", reasoningEffort: effort }, contextLevel: "workflow_context", excludedPaths: [], dailyEvaluationLimit: 8, minIntervalMinutes: 60, automaticKnowledgeUpdates: true, workspaceOverrides: {} };
  const providers = ["custom", "codex", "ollama", "claude-code"].map((id) => ({ id, label: id, state: id === "custom" ? state : "ready", models: ["static-model"], supportsReasoningEffort: false }));
  let status = { settings, effectiveSettings: settings, providers: missing ? [] : providers, remainingEvaluations: 8, analysisStatus: state, knowledgeVersion: null, lastEvaluatedAt: null, workspacePath, knowledgeUpdate: { version: null, lastSuccessfulUpdate: null, lastError: knowledgeError } };
  let discovery = 0, modelRefresh = 0;
  let currentRunStatus = runStatus;
  let activeWorkspacePath = workspacePath;
  const intervalCallbacks = [];
  const saves = [];
  let approved = false;
  const previousWindow = globalThis.window;
  globalThis.window = { setInterval: (callback) => { intervalCallbacks.push(callback); return intervalCallbacks.length; }, clearInterval: () => {}, addEventListener: () => {}, removeEventListener: () => {}, orkworks: {
    getTaskmasterSettings: async () => structuredClone(status),
    getTaskmasterRunStatus: async () => structuredClone(currentRunStatus),
    refreshTaskmasterModels: async () => { modelRefresh++; return pendingModels ? await pendingModels : ["live-model"]; },
    getProviderModels: async () => { discovery++; return { models: ["discovered"] }; },
    saveTaskmasterSettings: async (value) => { saves.push(structuredClone(value)); return { ...structuredClone(status), settings: value, effectiveSettings: value }; },
    getInferenceTrust: async () => [{ id: "custom", name: "Custom", state: approved ? "approved" : "approval_required", resolvedPath: "/tools/custom", revision: { documentRevision: "a".repeat(64), generation: "0", digest: "b".repeat(64) }, definition: { command: "custom", args: [], input: "stdin", timeoutSecs: 60 } }],
    approveInferenceAdapter: async (request) => { saves.push(request); approved = true; return true; },
  } };
  t.after(() => { globalThis.window = previousWindow; });
  const values = [], dependencies = [], effects = [];
  let stateIndex = 0, effectIndex = 0;
  const hooks = { ...React,
    useRef(initial) { return hooks.useState({ current: initial })[0]; },
    useState(initial) {
      const index = stateIndex++;
      if (!(index in values)) values[index] = initial;
      return [values[index], (next) => { values[index] = typeof next === "function" ? next(values[index]) : next; }];
    },
    useEffect(callback, deps) {
      const index = effectIndex++;
      if (!dependencies[index] || deps.some((value, i) => !Object.is(value, dependencies[index][i]))) effects.push(callback);
      dependencies[index] = deps;
    },
  };
  const module = { exports: {} };
  new Function("require", "module", "exports", (trust ? compiledTrust : compiled).outputFiles[0].text)(
    (id) => id === "react" ? hooks : id === "./InferenceTrustSettings" ? { default: () => null } : require(id), module, module.exports,
  );
  const render = () => { stateIndex = 0; effectIndex = 0; return module.exports.default({ currentWorkspacePath: activeWorkspacePath }); };
  const settle = async () => { while (effects.length) effects.shift()(); await new Promise(setImmediate); };
  render(); await settle();
  return { render, settle, saves, discovery: () => discovery, modelRefresh: () => modelRefresh, setRunStatus: (value) => { currentRunStatus = value; }, setWorkspace: (path) => { status = { ...status, workspacePath: path }; activeWorkspacePath = path; }, tick: () => intervalCallbacks.at(-1)?.() };
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

test("Taskmaster renders inactive custom state without requesting model discovery", async (t) => {
  const view = await fixture(t);
  const tree = view.render(); await view.settle();
  assert.match(text(tree), /Custom background execution is not active/);
  assert.equal(view.discovery(), 0);
  assert.equal(nodes(tree).find((node) => node.type === "input" && node.props.list)?.props.value, " vendor/opaque model ");
});

test("Codex and Ollama refresh suggestions only on request and keep model entry editable", async (t) => {
  for (const provider of ["codex", "ollama"]) {
    const view = await fixture(t, { provider });
    assert.equal(view.modelRefresh(), 0, "opening Settings must not discover models");
    view.render(); await view.settle();
    const button = nodes(view.render()).find((node) => node.type === "button" && text(node).includes("Refresh model suggestions"));
    assert.ok(button, `${provider} refresh action`);
    button.props.onClick();
    await view.settle();
    const tree = view.render();
    assert.equal(view.modelRefresh(), 1);
    assert.match(text(tree), /Showing 1 live model suggestions/);
    assert.ok(nodes(tree).some((node) => node.type === "option" && node.props.value === "live-model"));
    assert.ok(nodes(tree).some((node) => node.type === "input" && node.props.list && node.props.value === " vendor/opaque model "));
  }
});

test("Claude Code and custom providers keep static suggestions without live refresh", async (t) => {
  for (const provider of ["claude-code", "custom"]) {
    const view = await fixture(t, { provider });
    assert.equal(view.modelRefresh(), 0);
    assert.doesNotMatch(text(view.render()), /Refresh model suggestions/);
    assert.ok(nodes(view.render()).some((node) => node.type === "option" && node.props.value === "static-model"));
  }
});

test("a model refresh result is discarded after the selected provider changes", async (t) => {
  let resolveModels;
  const pendingModels = new Promise((resolve) => { resolveModels = resolve; });
  const view = await fixture(t, { provider: "codex", pendingModels });
  view.render(); await view.settle();
  const refresh = nodes(view.render()).find((node) => node.type === "button" && text(node).includes("Refresh model suggestions"));
  refresh.props.onClick();
  nodes(view.render()).find((node) => node.type === "select" && node.props.value === "codex").props.onChange({ target: { value: "ollama" } });
  view.render(); await view.settle();
  resolveModels(["stale-codex-model"]);
  await new Promise(setImmediate);
  const tree = view.render();
  assert.doesNotMatch(text(tree), /stale-codex-model/);
  assert.doesNotMatch(text(tree), /Showing 1 live model suggestions/);
});

test("analysis failure detail is shown separately from provider readiness and knowledge errors", async (t) => {
  const view = await fixture(t, {
    workspacePath: "/workspace",
    knowledgeError: "knowledge signature check failed",
    runStatus: { workspacePath: "/workspace", activeAttempt: null, latestOutcome: { state: "failed", startedAt: "start", completedAt: "finish", trigger: "background", provider: "codex", model: "gpt-x", errorSummary: "context collection failed" } },
  });
  view.render(); await view.settle();
  const rendered = text(view.render());
  assert.match(rendered, /Failed background analysis · codex \/ gpt-x: context collection failed/);
  assert.match(rendered, /Knowledge update: knowledge signature check failed/);
  assert.doesNotMatch(rendered, /legacy global error/);
});

test("analysis status polling follows the selected workspace while Settings remains open", async (t) => {
  const view = await fixture(t, {
    workspacePath: "/workspace-one",
    runStatus: { workspacePath: "/workspace-one", activeAttempt: null, latestOutcome: null },
  });
  view.render(); await view.settle();
  view.setRunStatus({ workspacePath: "/workspace-one", activeAttempt: null, latestOutcome: { state: "failed", startedAt: "start", completedAt: "finish", trigger: "manual", provider: "codex", model: "old-model", errorSummary: "old workspace failure" } });
  view.tick(); await view.settle();
  assert.match(text(view.render()), /old workspace failure/);
  view.setWorkspace("/workspace-two");
  assert.doesNotMatch(text(view.render()), /old workspace failure/);
  view.setRunStatus({ workspacePath: "/workspace-two", activeAttempt: null, latestOutcome: { state: "failed", startedAt: "start", completedAt: "finish", trigger: "manual", provider: "ollama", model: "llama3", errorSummary: "provider failed" } });
  view.tick(); await view.settle();
  assert.match(text(view.render()), /Failed manual analysis · ollama \/ llama3: provider failed/);
});

test("an unavailable saved selection survives display and save unchanged", async (t) => {
  const view = await fixture(t, { missing: true });
  const tree = view.render();
  assert.match(text(tree), /custom — unavailable \(saved selection\)/);
  assert.equal(nodes(tree).find((node) => node.type === "select" && node.props.value === "custom")?.props.value, "custom");
  nodes(tree).find((node) => node.type === "button" && text(node) === "Save recommendation settings").props.onClick();
  await view.settle();
  assert.equal(view.saves.length, 1);
  assert.deepEqual(view.saves[0].selection, { provider: "custom", model: " vendor/opaque model ", reasoningEffort: undefined });
  assert.equal(view.discovery(), 0);
});

test("unsupported stored effort is rejected instead of silently dropped or saved", async (t) => {
  const view = await fixture(t, { effort: "high" });
  const tree = view.render();
  nodes(tree).find((node) => node.type === "button" && text(node) === "Save recommendation settings").props.onClick();
  await view.settle();
  assert.equal(view.saves.length, 0);
  assert.match(text(view.render()), /does not support reasoning effort/);
});

test("executable approval displays conditional execution eligibility, not inactive execution", async (t) => {
  const view = await fixture(t, { trust: true });
  assert.doesNotMatch(text(view.render()), /does not enable execution|execution inactive|remains inactive/i);
  nodes(view.render()).find((node) => node.type === "button" && text(node) === "Review and approve executable").props.onClick();
  await view.settle();
  assert.equal(view.saves.length, 1);
  assert.equal(view.saves[0].harnessId, "custom");
  const rendered = text(view.render());
  assert.match(rendered, /Approved/);
  assert.match(rendered, /selected.*background analysis/i);
  assert.doesNotMatch(rendered, /does not enable execution|execution inactive|remains inactive/i);
});


test("Taskmaster explains the verified knowledge prerequisite while preserving provider configuration", async (t) => {
  const view = await fixture(t, { state: "knowledge_unavailable", provider: "codex" });
  const tree = view.render(); await view.settle();
  assert.match(text(tree), /Verified reference knowledge required/);
  assert.ok(nodes(tree).some((node) => node.type === "input" && node.props.list));
  assert.equal(view.modelRefresh(), 0);
});
