import assert from "node:assert/strict";
import test from "node:test";
import { requestSessionTitle } from "../electron/sessionTitle.ts";

test("title requests carry desktop authorization and exact workspace/session identity", async () => {
  let request: { url: string; init?: RequestInit } | undefined;
  const fetchImpl: typeof fetch = async (url, init) => {
    request = { url: String(url), init };
    return new Response(JSON.stringify({ title: "Login redirect repair", automatic: false }));
  };
  assert.deepEqual(await requestSessionTitle("http://127.0.0.1:7", {
    sessionId: "session-1", workspaceIdentity: "/project", action: "set", title: "Login redirect repair",
  }, "desktop-secret", fetchImpl, new AbortController().signal), { title: "Login redirect repair", automatic: false });
  assert.equal(request?.url, "http://127.0.0.1:7/sessions/session-1/title");
  assert.equal((request?.init?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "desktop-secret");
  assert.deepEqual(JSON.parse(String(request?.init?.body)), { workspaceIdentity: "/project", action: "set", title: "Login redirect repair" });
});

test("invalid title operations are rejected before transport", async () => {
  const fetchImpl: typeof fetch = async () => { throw new Error("transport reached"); };
  const base = { sessionId: "session-1", workspaceIdentity: "/project", action: "set" };
  for (const input of [null, { ...base, title: "" }, { ...base, title: "x\ny" }, { ...base, title: "x".repeat(101) },
    { ...base, sessionId: "../other", title: "Login repair" }, { ...base, action: "erase", title: "Login repair" },
    { ...base, action: "automatic", title: "Unexpected title" }]) {
    await assert.rejects(requestSessionTitle("http://127.0.0.1:7", input, "secret", fetchImpl, new AbortController().signal), /Invalid/);
  }
});

test("title transport combines a request deadline with backend cancellation", async (t) => {
  const deadline = new AbortController();
  const backend = new AbortController();
  const timeout = t.mock.method(AbortSignal, "timeout", () => deadline.signal);
  let requestSignal: AbortSignal | null | undefined;
  const fetchImpl: typeof fetch = async (_url, init) => {
    requestSignal = init?.signal;
    return new Response(JSON.stringify({ title: "Current task", automatic: false }));
  };
  await requestSessionTitle("http://127.0.0.1:7", {
    sessionId: "session-1", workspaceIdentity: "/project", action: "read",
  }, "desktop-secret", fetchImpl, backend.signal);
  assert.equal(timeout.mock.callCount(), 1);
  assert.equal(requestSignal?.aborted, false);
  deadline.abort();
  assert.equal(requestSignal?.aborted, true);
  assert.equal(backend.signal.aborted, false);
});
