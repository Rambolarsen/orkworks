import test from "node:test";
import assert from "node:assert/strict";
import {
  STARTUP_FAILURE_FALLBACK,
  STARTUP_FAILURE_MESSAGES,
  startupFailureMessage,
  validateStartupFailure,
} from "../src/startupDiagnostics.ts";

test("validates a bounded startup failure and resolves only local catalog text", () => {
  const failure = validateStartupFailure({
    code: "codex_help_timeout",
    operation: "resume",
    occurredAt: "2026-11-07T12:00:00.000Z",
    message: "<script>secret</script>",
  });
  assert.deepEqual(failure, {
    code: "codex_help_timeout",
    operation: "resume",
    occurredAt: "2026-11-07T12:00:00.000Z",
  });
  assert.equal(startupFailureMessage(failure), "The Codex session isolation check timed out. Check that the configured executable responds to --help, then retry.");
  assert.equal(STARTUP_FAILURE_MESSAGES.codex_help_timeout, startupFailureMessage(failure));
});

test("unknown, malformed, invalid-date, and oversized startup failures are absent", () => {
  assert.equal(validateStartupFailure({ code: "unknown", operation: "launch", occurredAt: "2026-11-07T12:00:00.000Z" }), undefined);
  assert.equal(validateStartupFailure({ code: "process_spawn_failed", operation: "launch", occurredAt: "yesterday" }), undefined);
  assert.equal(validateStartupFailure({ code: "process_spawn_failed", operation: "other", occurredAt: "2026-11-07T12:00:00.000Z" }), undefined);
  assert.equal(validateStartupFailure({ code: "process_spawn_failed", operation: "launch", occurredAt: "2026-11-07T12:00:00.000Z", extra: "x".repeat(250) }), undefined);
  assert.equal(STARTUP_FAILURE_FALLBACK, "Session startup failed. Check sidecar diagnostics before retrying.");
  assert.equal(startupFailureMessage({ code: "future_code", operation: "resume", occurredAt: "2026-11-07T12:00:00.000Z" }), STARTUP_FAILURE_FALLBACK);
});
