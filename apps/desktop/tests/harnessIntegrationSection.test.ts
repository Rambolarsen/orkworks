import test from "node:test";
import assert from "node:assert/strict";

import {
  isAttentionSignal,
  shouldShowInstalledConfirmation,
} from "../src/harnessIntegrationPresentation.ts";

test("isAttentionSignal is false for codex, whose hook only reports a session ID (issue #271)", () => {
  assert.equal(isAttentionSignal("codex"), false);
});

test("isAttentionSignal is false for opencode, whose session.created hook only reports a session ID (issue #110)", () => {
  assert.equal(isAttentionSignal("opencode"), false);
});

test("isAttentionSignal is true for harnesses whose hook reports needs-input attention", () => {
  assert.equal(isAttentionSignal("claude-code"), true);
  assert.equal(isAttentionSignal("gemini"), true);
  assert.equal(isAttentionSignal("copilot"), true);
});

test("unsupported tool versions suppress installed confirmation", () => {
  assert.equal(
    shouldShowInstalledConfirmation([{ code: "unsupported_tool_version", message: "unsupported" }]),
    false,
  );
  assert.equal(shouldShowInstalledConfirmation([]), true);
});
