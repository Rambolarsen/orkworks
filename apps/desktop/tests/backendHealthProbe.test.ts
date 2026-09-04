import assert from "node:assert/strict";
import test from "node:test";

import { probeBackendHealth } from "../src/backendHealthProbe.ts";

function makeDeps(overrides: Partial<Parameters<typeof probeBackendHealth>[0]> = {}) {
  const calls: { urls: string[]; delays: number[] } = { urls: [], delays: [] };
  return {
    deps: {
      getBackendUrl: async () => "http://127.0.0.1:9999",
      fetch: async () => ({ ok: true }),
      delay: async (ms: number) => {
        calls.delays.push(ms);
      },
      ...overrides,
    },
    calls,
  };
}

test("reports connected without flagging unreachable when the first getBackendUrl rejects (stale-failure window)", async () => {
  let urlAttempts = 0;
  const { deps } = makeDeps({
    getBackendUrl: async () => {
      urlAttempts += 1;
      if (urlAttempts === 1) throw new Error("Backend generation was replaced");
      return "http://127.0.0.1:9999";
    },
  });

  const result = await probeBackendHealth(deps);

  assert.equal(result, "connected");
  assert.equal(urlAttempts, 2);
});

test("reports connected when /health succeeds after the readiness window closes", async () => {
  let fetchAttempts = 0;
  const { deps } = makeDeps({
    fetch: async () => {
      fetchAttempts += 1;
      if (fetchAttempts < 3) return { ok: false };
      return { ok: true };
    },
  });

  const result = await probeBackendHealth(deps);

  assert.equal(result, "connected");
  assert.equal(fetchAttempts, 3);
});

test("reports unreachable only after every getBackendUrl attempt is exhausted", async () => {
  let urlAttempts = 0;
  const { deps, calls } = makeDeps({
    getBackendUrl: async () => {
      urlAttempts += 1;
      throw new Error("Sidecar exited before readiness");
    },
    urlAttempts: 4,
  });

  const result = await probeBackendHealth(deps);

  assert.equal(result, "unreachable");
  assert.equal(urlAttempts, 4);
  assert.deepEqual(calls.delays, [500, 500, 500]);
});

test("reports unreachable when /health never succeeds", async () => {
  let fetchAttempts = 0;
  const { deps } = makeDeps({
    fetch: async () => {
      fetchAttempts += 1;
      throw new Error("connection refused");
    },
    fetchAttempts: 5,
  });

  const result = await probeBackendHealth(deps);

  assert.equal(result, "unreachable");
  assert.equal(fetchAttempts, 5);
});

test("waits between retries instead of hammering the endpoint", async () => {
  const { deps, calls } = makeDeps({
    fetch: async () => ({ ok: false }),
    fetchAttempts: 3,
  });

  await probeBackendHealth(deps);

  assert.deepEqual(calls.delays, [500, 500]);
});
