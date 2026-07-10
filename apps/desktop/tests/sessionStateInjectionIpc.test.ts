import test from "node:test";
import assert from "node:assert/strict";

import {
  debugInjectionUrl,
  parseSessionStateInjectionPayload,
} from "../electron/sessionStateInjectionIpc.ts";

test("rejects malformed debug injection payloads", () => {
  assert.throws(() => parseSessionStateInjectionPayload(null), /invalid/i);
  assert.throws(
    () => parseSessionStateInjectionPayload({ sessionId: "id" }),
    /invalid/i,
  );
  assert.throws(
    () =>
      parseSessionStateInjectionPayload({
        sessionId: "",
        injectionId: "running-capped",
      }),
    /invalid/i,
  );
});

test("encodes session ids in debug injection URLs", () => {
  assert.equal(
    debugInjectionUrl(4312, "session/with spaces"),
    "http://127.0.0.1:4312/sessions/session%2Fwith%20spaces/debug-injection",
  );
});
