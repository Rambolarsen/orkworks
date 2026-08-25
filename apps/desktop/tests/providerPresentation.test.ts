import test from "node:test";
import assert from "node:assert/strict";

import { providerBadge } from "../src/providerPresentation.ts";

test("providerBadge maps each effective state to a tone and label", () => {
  assert.deepEqual(providerBadge("healthy"), { tone: "ok", label: "Healthy" });
  assert.deepEqual(providerBadge("degraded"), { tone: "warn", label: "Degraded" });
  assert.deepEqual(providerBadge("capped"), { tone: "warn", label: "Capacity reached" });
  assert.deepEqual(providerBadge("checking_capacity"), { tone: "info", label: "Checking capacity" });
  assert.deepEqual(providerBadge("disabled"), { tone: "neutral", label: "Disabled" });
  assert.deepEqual(providerBadge("unknown"), { tone: "neutral", label: "Unknown" });
});
