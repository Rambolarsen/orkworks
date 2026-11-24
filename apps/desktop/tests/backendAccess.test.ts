import assert from "node:assert/strict";
import test from "node:test";

type Call = { input: string; init?: RequestInit };

async function withGlobals(orkworks: unknown, run: (calls: Call[], sockets: unknown[][]) => Promise<void>): Promise<void> {
  const globals = globalThis as Record<string, unknown>;
  const saved = { window: globals.window, fetch: globals.fetch, WebSocket: globals.WebSocket };
  const calls: Call[] = [];
  const sockets: unknown[][] = [];
  globals.window = { orkworks };
  globals.fetch = async (input: string, init?: RequestInit) => { calls.push({ input, init }); return new Response("ok"); };
  globals.WebSocket = class { constructor(...args: unknown[]) { sockets.push(args); } };
  try {
    await run(calls, sockets);
  } finally {
    Object.assign(globals, saved);
  }
}

test("sidecar requests carry the renderer credential and keep caller headers", async () => {
  // A fresh module instance per test keeps the cached token isolated.
  const access = await import("../src/backendAccess.ts?with-token");
  await withGlobals({ getBackendToken: () => "renderer-secret" }, async (calls, sockets) => {
    await access.backendFetch("http://127.0.0.1:1/sessions", { method: "POST", headers: { "Content-Type": "application/json" } });
    const headers = new Headers(calls[0].init?.headers);
    assert.equal(headers.get("x-orkworks-renderer-token"), "renderer-secret");
    assert.equal(headers.get("content-type"), "application/json");
    assert.equal(calls[0].init?.method, "POST");

    access.openBackendSocket("ws://127.0.0.1:1/sessions/a/terminal");
    assert.deepEqual(sockets[0], ["ws://127.0.0.1:1/sessions/a/terminal", ["orkworks.v1", "orkworks.token.renderer-secret"]]);
  });
});

test("fixtures without a credential issue unchanged requests", async () => {
  const access = await import("../src/backendAccess.ts?without-token");
  await withGlobals({}, async (calls, sockets) => {
    await access.backendFetch("http://127.0.0.1:1/health");
    assert.deepEqual(calls[0], { input: "http://127.0.0.1:1/health", init: undefined });
    access.openBackendSocket("ws://127.0.0.1:1/workspace/updates");
    assert.deepEqual(sockets[0], ["ws://127.0.0.1:1/workspace/updates"]);
  });
});
