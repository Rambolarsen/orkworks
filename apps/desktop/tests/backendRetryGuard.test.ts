import assert from "node:assert/strict";
import test from "node:test";

import { createBackendRetryGuard } from "../src/backendRetryGuard.ts";

test("each begin() call issues a token that is current until a later begin() supersedes it", () => {
  const guard = createBackendRetryGuard();

  const first = guard.begin();
  assert.equal(guard.isCurrent(first), true);

  const second = guard.begin();
  assert.equal(guard.isCurrent(first), false, "first token is superseded once a second retry begins");
  assert.equal(guard.isCurrent(second), true);
});

test("a superseded retry's rejection must not clobber a later retry's success", () => {
  // Reproduces issue #356: double-clicking Retry races the first click's
  // rejection against the second click's success. The guard must let the
  // renderer tell which retry's outcome is still the current one.
  const guard = createBackendRetryGuard();

  const firstToken = guard.begin();
  const secondToken = guard.begin();

  // The second retry (the real one) succeeds first.
  assert.equal(guard.isCurrent(secondToken), true);

  // The first retry's stale rejection arrives afterward and must be ignored.
  assert.equal(guard.isCurrent(firstToken), false);
});
