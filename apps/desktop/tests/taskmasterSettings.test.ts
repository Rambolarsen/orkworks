import { test } from "node:test";
import assert from "node:assert/strict";
import { editTaskmasterScope, formatTaskmasterRunStatus, supportsTaskmasterAnalysis, type TaskmasterSettings } from "../src/taskmasterSettings.ts";
import { taskmasterRequest } from "../electron/taskmasterSettings.ts";

const settings: TaskmasterSettings = { enabled: true, selection: { provider: "ollama", model: "first" }, contextLevel: "workflow_context", excludedPaths: [], dailyEvaluationLimit: 8, minIntervalMinutes: 60, automaticKnowledgeUpdates: true, workspaceOverrides: {} };
test("Taskmaster selection allows declared inactive adapters but not unavailable providers", () => {
  for (const id of ["ollama", "codex", "claude-code", "copilot", "opencode", "aider", "custom"]) {
    for (const state of ["ready", "approval_required", "execution_inactive"] as const) {
      assert.equal(supportsTaskmasterAnalysis({ id, state }), true, `${id}: ${state}`);
    }
    for (const state of ["unavailable", "unsupported_capability"] as const) {
      assert.equal(supportsTaskmasterAnalysis({ id, state }), false, `${id}: ${state}`);
    }
  }
});
test("workspace edits cannot enlarge the app budget or replace global model selection", () => {
  const changed = editTaskmasterScope(settings, "/workspace", { selection: { provider: "claude-code", model: "second" }, dailyEvaluationLimit: 64 });
  assert.equal(changed.dailyEvaluationLimit, 8);
  assert.equal(changed.selection?.model, "first");
  assert.equal(changed.workspaceOverrides["/workspace"].selection?.model, "second");
  assert.deepEqual(settings.workspaceOverrides, {});
});
test("clearing an override restores inheritance without changing other workspaces", () => {
  const changed = editTaskmasterScope({ ...settings, workspaceOverrides: { "/a": { enabled: false }, "/b": { contextLevel: "session_observations" } } }, "/a", null);
  assert.deepEqual(changed.workspaceOverrides, { "/b": { contextLevel: "session_observations" } });
});
test("run status prioritizes active work and keeps latest failure details visible", () => {
  assert.match(formatTaskmasterRunStatus({ workspacePath: "/a", activeAttempt: { id: 1, state: "running", queuedAt: "q", startedAt: "s", trigger: "manual", provider: "codex", model: "gpt-x" }, latestOutcome: null }), /Running manual analysis · codex \/ gpt-x/);
  assert.match(formatTaskmasterRunStatus({ workspacePath: "/a", activeAttempt: null, latestOutcome: { state: "failed", startedAt: "s", completedAt: "c", trigger: "background", provider: "ollama", model: "llama", errorSummary: "offline" } }), /Failed background analysis · ollama \/ llama: offline/);
});
test("settings transport carries main-process authority only to the fixed loopback endpoint", async () => {
  let received: RequestInit | undefined;
  const result = await taskmasterRequest(4321, "test-authority", "settings", settings, async (url, init) => {
    assert.equal(url, "http://127.0.0.1:4321/settings/taskmaster");
    received = init;
    return new Response(JSON.stringify({ analysisStatus: "ready" }));
  });
  assert.equal((received?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "test-authority");
  assert.equal(received?.method, "POST");
  assert.deepEqual(result, { analysisStatus: "ready" });
});
test("model refresh uses the authenticated fixed Taskmaster discovery route", async () => {
  let received: RequestInit | undefined;
  const result = await taskmasterRequest(4321, "test-authority", "models", { provider: "ollama", ollamaBaseUrl: "http://127.0.0.1:11434" }, async (url, init) => {
    assert.equal(url, "http://127.0.0.1:4321/settings/taskmaster/models");
    received = init;
    return new Response(JSON.stringify({ models: ["llama3"] }));
  });
  assert.equal((received?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "test-authority");
  assert.equal(received?.method, "POST");
  assert.deepEqual(result, { models: ["llama3"] });
});
test("run status uses its dedicated authenticated read-only route", async () => {
  let received: RequestInit | undefined;
  await taskmasterRequest(4321, "test-authority", "run-status", undefined, async (url, init) => {
    assert.equal(url, "http://127.0.0.1:4321/taskmaster/run-status");
    received = init;
    return new Response(JSON.stringify({ activeAttempt: null, latestOutcome: null }));
  });
  assert.equal(received?.method, "GET");
  assert.equal((received?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "test-authority");
});
test("model discovery rejects oversized provider responses", async () => {
  await assert.rejects(
    taskmasterRequest(4321, "test-authority", "models", { provider: "codex" }, async () =>
      new Response(JSON.stringify({ models: ["x".repeat(300 * 1024)] }))),
    /response too large/,
  );
});
test("settings transport rejects sidecar errors without reporting a successful save", async () => {
  await assert.rejects(taskmasterRequest(4321, "test-authority", "settings", settings, async () => new Response(JSON.stringify({ error: "invalid limit" }), { status: 400 })), /invalid limit/);
  await assert.rejects(taskmasterRequest(4321, "test-authority", "models", { provider: "codex" }, async () => new Response(JSON.stringify({ error: { code: "timeout", message: "Codex app-server timed out" } }), { status: 502 })), /Codex app-server timed out/);
});
test("privileged transport rejects non-object success responses", async () => {
  for (const value of [null, [], "saved"]) {
    await assert.rejects(taskmasterRequest(4321, "test-authority", "settings", undefined,
      async () => new Response(JSON.stringify(value))), /Invalid.*response/);
  }
});
test("privileged transport keeps per-resource byte limits and rejects invalid authority before fetch", async () => {
  let calls = 0;
  const fetcher = async () => { calls++; return new Response("{}"); };
  for (const [resource, limit] of [["settings", 65536], ["knowledge", 2097152], ["inference", 8192], ["models", 65536]] as const) {
    // JSON quotes consume two bytes; multibyte text must be counted as UTF-8.
    await taskmasterRequest(4321, "token", resource, "x".repeat(limit - 2), fetcher);
    const before = calls;
    await assert.rejects(taskmasterRequest(4321, "token", resource, "é".repeat(limit / 2), fetcher), /too large/);
    assert.equal(calls, before);
  }
  const before = calls;
  for (const [port, token] of [[0, "token"], [65536, "token"], [4321, ""]] as const) {
    await assert.rejects(taskmasterRequest(port, token, "settings", undefined, fetcher), /unavailable/);
  }
  assert.equal(calls, before);
});
