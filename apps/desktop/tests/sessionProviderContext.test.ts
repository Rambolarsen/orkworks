import test from "node:test";
import assert from "node:assert/strict";
import { sessionProviderContext } from "../src/sessionProviderContext.ts";
import type { SessionInfo } from "../src/api.ts";

function sampleSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "s1",
    label: "Claude Code",
    status: "running",
    cwd: "/tmp/repo",
    created_at: "2026-06-22T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    ...overrides,
  };
}

test("sessionProviderContext uses session-scoped provider values", () => {
  assert.deepEqual(
    sessionProviderContext(sampleSession({
      provider: "Claude Code",
      providerModel: "sonnet",
      providerState: "healthy",
    })),
    { provider: "Claude Code", model: "sonnet", state: "healthy" },
  );
});

test("sessionProviderContext falls back to read-only unresolved values", () => {
  assert.deepEqual(
    sessionProviderContext(sampleSession()),
    { provider: "—", model: "—", state: "unknown" },
  );
});
