import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import type { SessionInfo, WorkspaceInfo } from "../src/api.ts";
import { forgetSession } from "../src/api.ts";

test("SessionInfo type accepts metadata fields", () => {
  const session: SessionInfo = {
    id: "test",
    label: "Test",
    status: "running",
    cwd: "/tmp",
    created_at: "now",
    observedStatus: "waiting_for_input",
    summary: "Needs approval",
    nextAction: "Choose an option",
    needsUserInput: true,
    detectedQuestion: "Proceed?",
    suggestedOptions: ["yes", "no"],
    blockerDescription: "Waiting on user",
    failedCommand: "cargo test",
    failedTest: "integration",
    capacityHints: ["capped"],
    peonLastInference: "2026-06-16T12:00:00Z",
    metadataSource: "process",
    metadataConfidence: 1.0,
    memoryState: "resumable",
    resumeStrategy: "exact",
    resume: {
      state: "available",
      preferredStrategy: "exact",
      harnessSessionId: "sess-123",
      latestFallback: true,
      lastSeenAt: "2026-06-17T12:00:00Z",
    },
    resumedFrom: "older-session",
  };
  assert.equal(session.metadataSource, "process");
  assert.equal(session.metadataConfidence, 1.0);
  assert.equal(session.observedStatus, "waiting_for_input");
  assert.equal(session.needsUserInput, true);
  assert.equal(session.memoryState, "resumable");
  assert.equal(session.resumeStrategy, "exact");
  assert.equal(session.resume?.harnessSessionId, "sess-123");
  assert.equal(session.resumedFrom, "older-session");
});

test("SessionInfo type accepts canonical terminology fields", () => {
  const session: SessionInfo = {
    id: "canonical-test",
    label: "Canonical Test",
    harnessId: "opencode",
    modelProviderId: "openrouter",
    modelId: "deepseek/deepseek-reasoner",
    status: "running",
    cwd: "/tmp/project",
    created_at: "2026-06-25T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
  };

  assert.equal(session.harnessId, "opencode");
  assert.equal(session.modelProviderId, "openrouter");
  assert.equal(session.modelId, "deepseek/deepseek-reasoner");
});

test("SessionInfo type accepts connectivity, terminalOutcome, resumeOptions, and lastActivityAt", () => {
  const session: SessionInfo = {
    id: "offline-test",
    label: "Offline Test",
    status: "ended",
    connectivity: "offline",
    terminalOutcome: "ended",
    cwd: "/tmp/project",
    created_at: "2026-06-28T09:00:00Z",
    lastActivityAt: "2026-06-28T09:05:00Z",
    memoryState: "resumable",
    resumeStrategy: "exact",
    resumeOptions: [
      {
        strategy: "exact",
        label: "Resume exact session",
        available: true,
        preferred: true,
      },
      {
        strategy: "latest_repo",
        label: "Resume latest in repo",
        available: false,
        preferred: false,
        reason: "Harness does not support repo-scoped resume",
      },
    ],
  };

  assert.equal(session.connectivity, "offline");
  assert.equal(session.terminalOutcome, "ended");
  assert.equal(session.lastActivityAt, "2026-06-28T09:05:00Z");
  assert.equal(session.resumeOptions[1].available, false);
});

test("api.ts declares canonical terminology aliases on SessionInfo", () => {
  const source = readFileSync(new URL("../src/api.ts", import.meta.url), "utf8");
  assert.match(source, /harnessId\?: string/);
  assert.match(source, /modelProviderId\?: string/);
  assert.match(source, /modelId\?: string/);
});

test("WorkspaceInfo type has expected shape", () => {
  const ws: WorkspaceInfo = {
    path: "/tmp/project",
    repo_root: "/tmp/project",
    branch: "main",
    dirty: false,
    lastActiveSessionId: "session-1",
    activeHarnessIds: [],
  };
  assert.equal(ws.path, "/tmp/project");
  assert.equal(ws.branch, "main");
  assert.equal(ws.lastActiveSessionId, "session-1");
});

test("forgetSession throws on non-ok response", async () => {
  const origFetch = globalThis.fetch;
  globalThis.fetch = (_url: string | URL | Request, _init?: RequestInit) =>
    Promise.resolve(new Response(null, { status: 409 }));
  try {
    await assert.rejects(() => forgetSession("http://localhost:0", "test-id"), /forget session failed: 409/);
  } finally {
    globalThis.fetch = origFetch;
  }
});

test("forgetSession resolves on 200", async () => {
  const origFetch = globalThis.fetch;
  globalThis.fetch = (_url: string | URL | Request, _init?: RequestInit) =>
    Promise.resolve(new Response(null, { status: 200 }));
  try {
    await assert.doesNotReject(() => forgetSession("http://localhost:0", "test-id"));
  } finally {
    globalThis.fetch = origFetch;
  }
});
