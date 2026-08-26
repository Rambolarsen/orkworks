import assert from "node:assert/strict";
import test from "node:test";

import {
  BACKEND_UNAVAILABLE_MESSAGE,
  sanitizeBackendLifecycleFailure,
} from "../electron/backendLifecycleFailure.ts";

test("sanitizes raw lifecycle failure details into stable path-free copy", () => {
  const message = sanitizeBackendLifecycleFailure(
    new Error("spawn /Users/froomiebot/workspace/orkworks/dist/orkworksd ENOENT"),
  );

  assert.equal(message, BACKEND_UNAVAILABLE_MESSAGE);
  assert.equal(message, "The OrkWorks sidecar is unavailable.");
  assert.doesNotMatch(message, /Users|orkworksd|ENOENT/);
});
