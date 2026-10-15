import { test } from "node:test";
import assert from "node:assert/strict";

import {
  errorFromBody,
  parseErrorBody,
  parsePeonError,
  sidecarSend,
  sidecarUrl,
  type SidecarFetchImpl,
} from "../electron/sidecarClient.ts";

function fetchRecorder(responses: Array<Response | Error>): {
  fetchImpl: SidecarFetchImpl;
  calls: Array<{
    input: string;
    method: string;
    headers: Record<string, string>;
    body: unknown;
    signal: RequestInit["signal"];
  }>;
} {
  let next = 0;
  const calls: Array<{
    input: string;
    method: string;
    headers: Record<string, string>;
    body: unknown;
    signal: RequestInit["signal"];
  }> = [];
  const fetchImpl = async (input: string, init?: RequestInit) => {
    const response = responses[next++] ?? new Response("{}", { status: 200 });
    if (response instanceof Error) throw response;
    const headers: Record<string, string> = {};
    new Headers(init?.headers).forEach((value, key) => { headers[key] = value; });
    calls.push({
      input,
      method: init?.method ?? "GET",
      headers,
      body: init?.body,
      signal: init?.signal ?? null,
    });
    return response;
  };
  return { fetchImpl, calls };
}

test("sidecarUrl binds the port and optional path", () => {
  assert.equal(sidecarUrl(1234), "http://127.0.0.1:1234");
  assert.equal(sidecarUrl(4321, "sessions"), "http://127.0.0.1:4321/sessions");
});

test("sidecarSend defaults to GET and adds headers only for supplied values", async () => {
  const { fetchImpl, calls } = fetchRecorder([new Response("{}")]);
  const response = await sidecarSend(1234, "harnesses", { fetchImpl });
  assert.equal(response.status, 200);
  assert.deepEqual(calls[0], {
    input: "http://127.0.0.1:1234/harnesses",
    method: "GET",
    headers: {},
    body: undefined,
    signal: null,
  });
});

test("sidecarSend serializes JSON bodies into typed headers", async () => {
  const { fetchImpl, calls } = fetchRecorder([new Response("{}")]);
  await sidecarSend(2134, "workspace", {
    fetchImpl,
    method: "POST",
    jsonBody: { active: true },
    token: "t0k3n",
  });
  assert.equal(calls[0]?.method, "POST");
  assert.equal(calls[0]?.body, JSON.stringify({ active: true }));
  assert.deepEqual(calls[0]?.headers, {
    "content-type": "application/json",
    "x-orkworks-open-plan-token": "t0k3n",
  });
});

test("sidecarSend composes the request signal with the timeout", async () => {
  const { fetchImpl, calls } = fetchRecorder([new Response("{}")]);
  const controller = new AbortController();
  await sidecarSend(1234, "providers", {
    fetchImpl,
    signal: controller.signal,
    timeoutMs: 5_000,
  });
  const composed = calls[0]?.signal;
  assert.ok(composed instanceof AbortSignal);
  assert.notEqual(composed, controller.signal);
  assert.equal(composed.aborted, false);
  controller.abort();
  assert.equal(composed.aborted, true);
});

test("sidecarSend fires the timeout on its own", async () => {
  const { fetchImpl, calls } = fetchRecorder([new Response("{}")]);
  await sidecarSend(1234, "providers", { fetchImpl, timeoutMs: 20 });
  const composed = calls[0]?.signal;
  assert.ok(composed);
  const deadline = Date.now() + 2_000;
  while (!composed.aborted && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  assert.equal(composed.aborted, true);
});

test("parseErrorBody prefers the sidecar error field", async () => {
  const response = new Response(JSON.stringify({ error: "No such provider." }), { status: 500 });
  assert.equal(await parseErrorBody(response, "fallback"), "No such provider.");
});

test("parseErrorBody falls back when the body is unreadable", async () => {
  const response = new Response("not json", { status: 503 });
  assert.equal(await parseErrorBody(response, "Managed Service unreachable"), "Managed Service unreachable");
});

test("parsePeonError resolves the sidecar error body into a thrown error", async () => {
  const response = new Response(JSON.stringify({ error: "Generation mismatch." }), { status: 409 });
  const error = await parsePeonError(response, "Couldn't verify the Peon provider.");
  assert.equal(error.message, "Generation mismatch.");
});

test("parsePeonError falls back to the managed message for an unreadable body", async () => {
  const response = new Response("not json", { status: 503 });
  const error = await parsePeonError(response, "Couldn't verify the Peon provider.");
  assert.equal(error.message, "Couldn't verify the Peon provider.");
});

test("errorFromBody reads a pre-parsed error body", () => {
  assert.equal(errorFromBody({ error: "Overlay not found." }, "fallback"), "Overlay not found.");
  assert.equal(errorFromBody({ code: "nope" }, "fallback"), "fallback");
  assert.equal(errorFromBody(null, "fallback"), "fallback");
});
