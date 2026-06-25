import test from "node:test";
import assert from "node:assert/strict";

import { sessionProviderContext } from "../src/sessionProviderContext.ts";
import type { SessionInfo } from "../src/api.ts";

function sampleSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "session-1",
    label: "Test session",
    harness: "OpenCode",
    provider: "OpenRouter",
    model: "deepseek/deepseek-reasoner",
    providerState: "healthy",
    status: "running",
    cwd: "/tmp/project",
    created_at: "2026-06-25T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    ...overrides,
  };
}

test("sessionProviderContext separates coding tool, model provider, and model", () => {
  const context = sessionProviderContext(sampleSession());

  assert.equal(context.codingTool, "OpenCode");
  assert.equal(context.modelProvider, "OpenRouter");
  assert.equal(context.model, "deepseek/deepseek-reasoner");
  assert.equal(context.providerState, "healthy");
});

test("sessionProviderContext leaves model provider unknown when unavailable", () => {
  const context = sessionProviderContext(sampleSession({ provider: undefined }));

  assert.equal(context.codingTool, "OpenCode");
  assert.equal(context.modelProvider, "Unknown");
  assert.equal(context.model, "deepseek/deepseek-reasoner");
});
