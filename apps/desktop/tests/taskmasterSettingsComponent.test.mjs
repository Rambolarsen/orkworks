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
async function fixture(t, { state = "execution_inactive", missing = false, effort, trust = false } = {}) {
  const settings = { enabled: true, selection: { provider: "custom", model: " vendor/opaque model ", reasoningEffort: effort }, contextLevel: "workflow_context", excludedPaths: [], dailyEvaluationLimit: 8, minIntervalMinutes: 60, automaticKnowledgeUpdates: true, workspaceOverrides: {} };
  const status = { settings, effectiveSettings: settings, providers: missing ? [] : [{ id: "custom", label: "Custom", state, models: [], supportsReasoningEffort: false }], remainingEvaluations: 8, analysisStatus: state, knowledgeVersion: null, lastEvaluatedAt: null, lastError: null, workspacePath: null, knowledgeUpdate: { version: null, lastSuccessfulUpdate: null, lastError: null } };
  let discovery = 0;
  const saves = [];
  let approved = false;
  const previousWindow = globalThis.window;
  globalThis.window = { orkworks: {
    getTaskmasterSettings: async () => structuredClone(status),
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
  const render = () => { stateIndex = 0; effectIndex = 0; return module.exports.default(); };
  const settle = async () => { while (effects.length) effects.shift()(); await new Promise(setImmediate); };
  render(); await settle();
  return { render, settle, saves, discovery: () => discovery };
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
