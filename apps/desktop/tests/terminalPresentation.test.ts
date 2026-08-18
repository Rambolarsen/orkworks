import test from "node:test";
import assert from "node:assert/strict";

import { computeTerminalInteractivity, isSessionStarting } from "../src/terminalPresentation.ts";
import type { SessionInfo } from "../src/api.ts";

/**
 * Regression coverage for issue #302: `create_session` now responds with
 * the pre-spawn "creating" record instead of waiting for the harness to
 * finish spawning. This shapes a mock exactly like that real response and
 * drives it through `isSessionStarting` — the same function
 * `TerminalPanel.tsx` calls to derive its `starting` prop — into the real
 * `computeTerminalInteractivity`, rather than asserting against a
 * synthetic `starting: true` — a regression in the derivation itself
 * (e.g. checking `lifecycle` instead of `status`) would be caught here
 * because this test exercises the actual shared function, not a
 * hand-copied re-implementation of it.
 */
function createResponse(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "new-session",
    label: "New Session",
    status: "creating",
    cwd: "/repo",
    created_at: "2026-08-17T07:00:00Z",
    ...overrides,
  };
}

test("a freshly created session (status: creating) disables stdin and stops the cursor blink", () => {
  const session = createResponse();
  const result = computeTerminalInteractivity({ starting: isSessionStarting(session), ended: false });
  assert.equal(result.disableStdin, true);
  assert.equal(result.cursorBlink, false);
});

test("once the detached spawn finishes, the same derivation re-enables the terminal", () => {
  const session = createResponse({ status: "running" });
  const result = computeTerminalInteractivity({ starting: isSessionStarting(session), ended: false });
  assert.equal(result.disableStdin, false);
  assert.equal(result.cursorBlink, true);
});

test("a spawn failure (status: error) is not treated as still-starting", () => {
  const session = createResponse({ status: "error" });
  const result = computeTerminalInteractivity({ starting: isSessionStarting(session), ended: false });
  assert.equal(result.disableStdin, false);
});
