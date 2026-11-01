import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { getSessionPlanContent, requestSessionPlanReview, selectTerminalPlan } from "../electron/planOpener.ts";

test("reads the exact document kind without widening path authority", async () => {
  const urls: string[] = [];
  const content = await getSessionPlanContent("http://127.0.0.1:4444", "s", "token", async (url) => {
    urls.push(url.toString());
    return new Response(JSON.stringify({ content: "# Design" }));
  }, new AbortController().signal, "design");
  assert.equal(content, "# Design");
  assert.deepEqual(urls, ["http://127.0.0.1:4444/sessions/s/plan-content?kind=design"]);
});

test("binds reads and review to the displayed opaque selection and rejects arbitrary IDs", async () => {
  const urls: string[] = [];
  const selectionId = "a".repeat(64);
  const fetcher: typeof fetch = async url => { urls.push(url.toString()); return new Response(JSON.stringify({content:"# Design"})); };
  const signal = new AbortController().signal;
  await getSessionPlanContent("http://127.0.0.1:4444", "s", "token", fetcher, signal, "design", selectionId);
  await requestSessionPlanReview("http://127.0.0.1:4444", "s", "token", fetcher, signal, "design", selectionId);
  assert.ok(urls.every(url => url.endsWith(`?kind=design&selectionId=${selectionId}`)));
  await assert.rejects(getSessionPlanContent("http://127.0.0.1:4444", "s", "token", fetcher, signal, "design", "../private"), /Invalid document selection/);
  assert.equal(urls.length, 2);
});

test("review targets the displayed implementation document", async () => {
  const urls: string[] = [];
  await requestSessionPlanReview("http://127.0.0.1:4444", "s", "token", async (url) => {
    urls.push(url.toString());
    return new Response(null, { status: 204 });
  }, new AbortController().signal, "implementation");
  assert.deepEqual(urls, ["http://127.0.0.1:4444/sessions/s/request-plan-review?kind=implementation"]);
});

test("reads plan content through the authenticated sidecar endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const signal = new AbortController().signal;
  const content = await getSessionPlanContent(
    "http://127.0.0.1:4444",
    "session 1",
    "private-token",
    async (url, init) => {
      requests.push({ url: url.toString(), init });
      return new Response(JSON.stringify({ content: "# plan" }));
    },
    signal,
  );

  assert.equal(content, "# plan");
  assert.deepEqual(requests, [{
    url: "http://127.0.0.1:4444/sessions/session%201/plan-content",
    init: { headers: { "x-orkworks-open-plan-token": "private-token" }, signal },
  }]);
});

test("does not accept a malformed plan-content response", async () => {
  await assert.rejects(
    getSessionPlanContent("http://127.0.0.1:4444", "s", "token", async () => new Response(JSON.stringify({ path: "/secret" })), new AbortController().signal),
    /Couldn’t read this plan/,
  );
});

test("submits a review request through the authenticated sidecar endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const signal = new AbortController().signal;
  await requestSessionPlanReview("http://127.0.0.1:4444", "session 1", "private-token", async (url, init) => {
    requests.push({ url: url.toString(), init });
    return new Response(null, { status: 204 });
  }, signal);
  assert.deepEqual(requests, [{
    url: "http://127.0.0.1:4444/sessions/session%201/request-plan-review",
    init: { method: "POST", headers: { "x-orkworks-open-plan-token": "private-token" }, signal },
  }]);
});

test("selects a terminal plan through the authenticated sidecar endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const signal = new AbortController().signal;
  await selectTerminalPlan("http://127.0.0.1:4444", "session 1", "specs/plan.md", "private-token", async (url, init) => {
    requests.push({ url: url.toString(), init });
    return new Response(null, { status: 204 });
  }, signal);
  assert.deepEqual(requests, [{
    url: "http://127.0.0.1:4444/sessions/session%201/select-terminal-plan",
    init: {
      method: "POST",
      headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": "private-token" },
      body: JSON.stringify({ printedPath: "specs/plan.md" }),
      signal,
    },
  }]);
});

test("starts the sidecar with the plan token without startup workspace adoption", async () => {
  const mainSource = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");

  assert.match(mainSource, /env: \{ \.\.\.process\.env, ORKWORKS_OPEN_PLAN_TOKEN: openPlanToken \}/);
  assert.doesNotMatch(mainSource, /initialSidecarCwd/);
  assert.match(mainSource, /openPlanToken = randomBytes\(32\)\.toString\("hex"\);/);
});
