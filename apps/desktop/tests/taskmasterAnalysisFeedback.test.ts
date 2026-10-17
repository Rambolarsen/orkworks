import { test } from "node:test";
import assert from "node:assert/strict";
import { analysisStatusAdvanced, formatAnalysisDuration, type TaskmasterRunStatus } from "../src/taskmasterAnalysisFeedback.ts";

const previous: TaskmasterRunStatus = {
  workspacePath: "/a", activeAttempt: null,
  latestOutcome: { state: "succeeded", startedAt: "2026-10-17T10:00:00Z", completedAt: "2026-10-17T10:01:00Z", trigger: "manual", provider: "codex", model: "gpt-x", errorSummary: null },
};

test("request acknowledgement survives an unchanged old outcome, then yields to active or fast terminal status", () => {
  assert.equal(analysisStatusAdvanced(previous, previous), false);
  assert.equal(analysisStatusAdvanced(null, previous), false);
  assert.equal(analysisStatusAdvanced({ ...previous, workspacePath: "/b" }, previous), false);
  assert.equal(analysisStatusAdvanced({ ...previous, activeAttempt: { id: 2, state: "queued", queuedAt: "2026-10-17T11:00:00Z", startedAt: null, trigger: "manual", provider: "codex", model: "gpt-x" } }, previous), true);
  for (const state of ["succeeded", "failed", "interrupted"] as const) {
    assert.equal(analysisStatusAdvanced({ ...previous, latestOutcome: { ...previous.latestOutcome!, state, completedAt: "2026-10-17T11:01:00Z" } }, previous), true);
  }
});

test("only a new cached or skipped disposition replaces the pending request", () => {
  const cached = { ...previous, latestManualRequest: { id: 3, requestedAt: "q", completedAt: "c", disposition: "cached" as const, reason: "unchanged_inputs" as const } };
  assert.equal(analysisStatusAdvanced(cached, previous), true);
  assert.equal(analysisStatusAdvanced(cached, cached), false);
  assert.equal(analysisStatusAdvanced({ ...cached, latestManualRequest: { ...cached.latestManualRequest, id: 4, disposition: "skipped", reason: "unavailable" } }, cached), true);
});

test("elapsed time is readable, clamps clock skew, and tolerates missing timestamps", () => {
  const now = Date.parse("2026-10-17T10:02:05Z");
  assert.equal(formatAnalysisDuration("2026-10-17T10:00:00Z", now), "2m 5s");
  assert.equal(formatAnalysisDuration("2026-10-17T10:02:00Z", now), "5s");
  assert.equal(formatAnalysisDuration("2026-10-17T11:00:00Z", now), "0s");
  assert.equal(formatAnalysisDuration("invalid", now), null);
});

// Exercise the real status surface with server state; no mocked presentation.
const { build } = await import("esbuild");
const { createRequire } = await import("node:module");
const { createElement } = await import("react");
const { renderToStaticMarkup } = await import("react-dom/server");
const compiled = await build({ entryPoints: [new URL("../src/components/TaskmasterAnalysisStatus.tsx", import.meta.url).pathname], bundle: true, write: false, platform: "node", format: "cjs", packages: "external" });
const module = { exports: {} as Record<string, unknown> };
new Function("require", "module", "exports", compiled.outputFiles[0].text)(createRequire(import.meta.url), module, module.exports);
const Status = module.exports.default as (props: { status: TaskmasterRunStatus | null; requesting: boolean; pending: boolean; error?: string }) => import("react").ReactElement;
const render = (status: TaskmasterRunStatus | null, props = {}) => renderToStaticMarkup(createElement(Status, { status, requesting: false, pending: false, ...props }));

test("pending analysis hides the previous success, and completion shows authoritative empty counts", () => {
  const pending = render(previous, { pending: true });
  assert.match(pending, /Brain analysis requested/);
  assert.doesNotMatch(pending, /complete|Finished|codex/);
  const complete = render({ ...previous, latestOutcome: { ...previous.latestOutcome!, proposalCounts: { created: 0, suppressed: 0 } } });
  assert.match(complete, /Brain analysis complete/);
  assert.match(complete, /No proposals were created/);
  assert.doesNotMatch(complete, /requested|2026-10-17T/);
});

test("running status takes precedence over the old outcome and exposes elapsed time", () => {
  const markup = render({ ...previous, activeAttempt: { id: 2, state: "running", queuedAt: new Date().toISOString(), startedAt: new Date().toISOString(), trigger: "manual", provider: "codex", model: "gpt-running" } });
  assert.match(markup, /Analyzing workspace/);
  assert.match(markup, /Elapsed 0s/);
  assert.match(markup, /gpt-running/);
  assert.doesNotMatch(markup, /Brain analysis complete|Finished/);
});

test("status polling failures are visible even while waiting for admission", () => {
  const markup = render(null, { pending: true, error: "Sidecar disconnected" });
  assert.match(markup, /Sidecar disconnected/);
  assert.match(markup, /Analysis status unavailable/);
});

test("failed and interrupted runs show their failure, without presenting an empty success", () => {
  for (const state of ["failed", "interrupted"] as const) {
    const markup = render({ ...previous, latestOutcome: { ...previous.latestOutcome!, state, errorSummary: "Provider disconnected", proposalCounts: { created: 0, suppressed: 0 } } });
    assert.match(markup, /Provider disconnected/);
    assert.doesNotMatch(markup, /No proposals were created|complete/);
  }
});


test("a newer actual outcome supersedes an older manual disposition, while a newer skip remains visible", () => {
  const request = { id: 3, requestedAt: "2026-10-17T09:00:00Z", completedAt: "2026-10-17T09:00:01Z", disposition: "skipped" as const, reason: "unavailable" as const };
  assert.match(render({ ...previous, latestManualRequest: request }), /Brain analysis complete/);
  const newer = render({ ...previous, latestManualRequest: { ...request, completedAt: "2026-10-17T11:00:00Z" } });
  assert.match(newer, /Analysis did not start/);
  assert.doesNotMatch(newer, /Brain analysis complete/);
});
