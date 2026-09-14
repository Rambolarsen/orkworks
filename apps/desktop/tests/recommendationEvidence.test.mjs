import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import * as React from "react";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/components/RecommendationEvidence.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});
const evidence = (id) => ({ observationId: id, sessionId: `session-${id}`, description: `Observation ${id}`, source: "agent", observedAt: "2026-09-14T10:00:00Z", problemArea: "Testing", evidence: `Full evidence ${id}` });
const family = (id) => ({ id, title: `Family ${id}`, updatedAt: "2026-09-14T10:00:00Z", rollupMemberIds: [], evidence: [evidence(id)] });
const parent = { ...family("parent"), rollupMemberIds: ["a", "b"] };

async function fixture(t, respond = async (id) => family(id)) {
  const requests = [], selected = [], values = [], deps = [], cleanups = [], effects = [];
  let index = 0, effectIndex = 0, props = { recommendation: parent, onSelectSession: (id) => selected.push(id) };
  const oldWindow = globalThis.window, oldFetch = globalThis.fetch;
  globalThis.window = { orkworks: { getBackendUrl: async () => "http://sidecar" } };
  globalThis.fetch = async (url) => {
    const id = decodeURIComponent(url.split("/").at(-1));
    requests.push(id);
    return { ok: true, json: async () => respond(id) };
  };
  const hooks = { ...React,
    useState(initial) {
      const i = index++;
      if (!(i in values)) values[i] = typeof initial === "function" ? initial() : initial;
      return [values[i], (next) => { values[i] = typeof next === "function" ? next(values[i]) : next; }];
    },
    useRef(initial) { return hooks.useState({ current: initial })[0]; },
    useEffect(callback, nextDeps) {
      const i = effectIndex++;
      if (!deps[i] || nextDeps.some((dep, n) => !Object.is(dep, deps[i][n]))) {
        effects.push(() => { cleanups[i]?.(); cleanups[i] = callback(); });
      }
      deps[i] = nextDeps;
    },
  };
  const module = { exports: {} };
  new Function("require", "module", "exports", compiled.outputFiles[0].text)(
    (id) => id === "react" ? hooks : require(id), module, module.exports,
  );
  const render = (nextProps) => {
    if (nextProps) props = { ...props, ...nextProps };
    index = 0; effectIndex = 0;
    return module.exports.default(props);
  };
  const settle = async () => { while (effects.length) effects.shift()(); await new Promise(setImmediate); };
  const unmount = () => cleanups.forEach((cleanup) => cleanup?.());
  t.after(() => { unmount(); globalThis.window = oldWindow; globalThis.fetch = oldFetch; });
  const toggle = async (open) => {
    nodes(render()).find((node) => node.type === "details").props.onToggle({ currentTarget: { open } });
    render(); await settle();
  };
  render(); await settle();
  return { render, settle, toggle, unmount, requests, selected };
}
function nodes(tree) {
  if (Array.isArray(tree)) return tree.flatMap(nodes);
  if (!tree || typeof tree !== "object") return [];
  if (typeof tree.type === "function") return nodes(tree.type(tree.props));
  return [tree, ...nodes(tree.props?.children)];
}
function text(tree) {
  if (Array.isArray(tree)) return tree.map(text).join("");
  if (tree && typeof tree === "object") return text(typeof tree.type === "function" ? tree.type(tree.props) : tree.props?.children);
  return tree == null || typeof tree === "boolean" ? "" : String(tree);
}

test("expansion lazily fetches complete evidence grouped by family with provenance", async (t) => {
  const view = await fixture(t, async (id) => id === "a"
    ? { ...family(id), evidence: Array.from({ length: 70 }, (_, index) => evidence(`a-${index}`)) }
    : family(id));
  assert.deepEqual(view.requests, []);
  await view.toggle(true);
  assert.deepEqual(view.requests, ["a", "b"]);
  const groups = nodes(view.render()).filter((node) => node.type === "section");
  assert.equal(groups.length, 2);
  assert.match(text(groups[0]), /Family a.*Full evidence a-69/);
  assert.match(text(groups[1]), /Family b.*Full evidence b/);
  for (const group of groups) assert.match(text(group), /agent · 2026-09-14T10:00:00Z.*Problem area · Testing/);
  nodes(groups[1]).find((node) => node.type === "button").props.onClick();
  assert.deepEqual(view.selected, ["session-b"]);
  await view.toggle(false); await view.toggle(true);
  assert.deepEqual(view.requests, ["a", "b"]);
});

test("member failure retains successful families and retries only missing details", async (t) => {
  let fail = true;
  const view = await fixture(t, async (id) => {
    if (id === "b" && fail) throw new Error("Unavailable");
    return family(id);
  });
  await view.toggle(true);
  assert.match(text(view.render()), /Full evidence a/);
  assert.match(text(view.render()), /Couldn't load/);
  fail = false;
  nodes(view.render()).find((node) => node.type === "button" && text(node) === "Retry").props.onClick();
  view.render(); await view.settle();
  assert.deepEqual(view.requests, ["a", "b", "b"]);
  assert.match(text(view.render()), /Full evidence b/);
  assert.doesNotMatch(text(view.render()), /Couldn't load/);
});

test("changed parents discard pending details while the new family request succeeds", async (t) => {
  let release;
  const pending = new Promise((resolve) => { release = resolve; });
  const view = await fixture(t, async (id) => {
    if (id === "a" || id === "b") await pending;
    return family(id);
  });
  await view.toggle(true);
  view.render({ recommendation: { ...family("next"), rollupMemberIds: ["c", "d"] } });
  await view.settle();
  release(); await view.settle();
  assert.match(text(view.render()), /Full evidence c/);
  assert.doesNotMatch(text(view.render()), /Full evidence [ab]/);
});

test("unmounting during a workspace transition discards both pending successes and errors", async (t) => {
  let release;
  const pending = new Promise((resolve) => { release = resolve; });
  const view = await fixture(t, async (id) => {
    await pending;
    if (id === "b") throw new Error("Old workspace unavailable");
    return family(id);
  });
  await view.toggle(true);
  view.unmount();
  release(); await view.settle();
  assert.doesNotMatch(text(view.render()), /Full evidence [ab]|Couldn't load/);
});

test("ordinary exact family evidence remains local and never requests member details", async (t) => {
  const view = await fixture(t);
  view.render({ recommendation: family("exact") }); await view.settle();
  await view.toggle(true);
  assert.deepEqual(view.requests, []);
  assert.match(text(view.render()), /Full evidence exact/);
});

test("changing only the workspace identity discards cached family details", async (t) => {
  let activeWorkspace = "first";
  const view = await fixture(t, async (id) => ({ ...family(id), title: `${activeWorkspace} family ${id}` }));
  view.render({ recommendation: { ...parent, workspaceId: "first" } }); await view.settle();
  await view.toggle(true);
  assert.match(text(view.render()), /first family a/);
  activeWorkspace = "second";
  view.render({ recommendation: { ...parent, workspaceId: "second" } }); await view.settle();
  assert.deepEqual(view.requests, ["a", "b", "a", "b"]);
  assert.match(text(view.render()), /second family a/);
  assert.doesNotMatch(text(view.render()), /first family/);
});
