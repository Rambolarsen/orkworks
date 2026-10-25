import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/useActionsRecommendations.ts", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});

// Fast Refresh retains useState values across edits. Supply the retained state
// at the hook boundary and exercise its real render logic before effects run.
function renderRetained(snapshot, scope = "/workspace", enabled = true) {
  const hooks = {
    useState: () => [snapshot, () => {}],
    useRef: () => ({ current: null }),
    useEffect: () => {},
    useCallback: callback => callback,
  };
  const module = { exports: {} };
  new Function("require", "module", "exports", compiled.outputFiles[0].text)(
    id => id === "react" ? hooks : require(id), module, module.exports,
  );
  return module.exports.useActionsRecommendations(scope, enabled);
}

test("retained pre-shared-source state renders loading until the source publishes", () => {
  const result = renderRetained({ scope: "/workspace", status: "ready", items: [{ id: "old" }] });
  assert.equal(result.status, "loading");
  assert.deepEqual(result.items, []);
  assert.deepEqual(result.response, { recommendations: [], diagnostics: [] });
  assert.equal(result.revision, 0);
});

test("current source snapshots keep recommendations and diagnostics", () => {
  const response = { recommendations: [{ id: "current" }], diagnostics: [{ message: "partial" }] };
  const result = renderRetained({ scope: "/workspace", status: "partial", response, revision: 2 });
  assert.equal(result.status, "partial");
  assert.equal(result.response, response);
  assert.equal(result.items, response.recommendations);
  assert.equal(result.revision, 2);
});

test("disabled and changed workspaces never expose retained recommendations", () => {
  const snapshot = { scope: "/workspace", status: "ready", response: { recommendations: [{ id: "old" }], diagnostics: [] }, revision: 2 };
  for (const [scope, enabled, status] of [["/other", true, "loading"], ["/workspace", false, "unavailable"]]) {
    const result = renderRetained(snapshot, scope, enabled);
    assert.equal(result.status, status);
    assert.deepEqual(result.items, []);
  }
});
