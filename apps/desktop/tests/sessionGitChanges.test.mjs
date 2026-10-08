import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/components/SessionDetailPanel.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});
const component = { exports: {} };
new Function("require", "module", "exports", compiled.outputFiles[0].text)(require, component, component.exports);

function render(overrides = {}) {
  const session = {
    id: "counter", label: "Counter", status: "running", cwd: "/repo",
    created_at: "2026-10-03T10:00:00Z", memoryState: "live", resumeStrategy: "none",
    branch: "feature", dirty: true, changedFiles: 3,
    lineChanges: { additions: 124, deletions: 37 }, ...overrides,
  };
  return renderToStaticMarkup(createElement(component.exports.default, {
    sessions: [session], visibleSessionId: session.id, harnesses: [],
    showDebugMetadata: false, onResumeSession() {}, onApplyDebugAttention() {},
    onOpenSettings() {}, onReviewPlan() {},
  }));
}

test("Details shows file and live added/removed line totals", () => {
  assert.match(render(), /3 files · \+124 −37/);
});

test("Details shows clean zero totals and works before the first commit", () => {
  assert.match(render({ dirty: false, changedFiles: 0, lineChanges: { additions: 0, deletions: 0 } }), /0 files · \+0 −0/);
  assert.match(render({ branch: undefined, repoRoot: "/repo", changedFiles: 1 }), /1 file · \+124 −37/);
});

test("Details preserves file count when line statistics are unavailable", () => {
  const html = render({ lineChanges: undefined });
  assert.match(html, /3 files/);
  assert.doesNotMatch(html, /\+0|−0/);
});

test("Details uses a neutral label when a repository branch cannot be resolved", () => {
  const html = render({ branch: undefined, repoRoot: "/repo", lineChanges: undefined });
  assert.match(html, /Git repository/);
  assert.doesNotMatch(html, /No commits yet/);
});
