import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { getSessionPlanContent, requestSessionPlanReview } from "../electron/planOpener.ts";

test("reads plan content through the authenticated sidecar endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const content = await getSessionPlanContent(
    "http://127.0.0.1:4444",
    "session 1",
    "private-token",
    async (url, init) => {
      requests.push({ url: url.toString(), init });
      return new Response(JSON.stringify({ content: "# plan" }));
    },
  );

  assert.equal(content, "# plan");
  assert.deepEqual(requests, [{
    url: "http://127.0.0.1:4444/sessions/session%201/plan-content",
    init: { headers: { "x-orkworks-open-plan-token": "private-token" } },
  }]);
});

test("does not accept a malformed plan-content response", async () => {
  await assert.rejects(
    getSessionPlanContent("http://127.0.0.1:4444", "s", "token", async () => new Response(JSON.stringify({ path: "/secret" }))),
    /Couldn’t read this plan/,
  );
});

test("submits a review request through the authenticated sidecar endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  await requestSessionPlanReview("http://127.0.0.1:4444", "session 1", "private-token", async (url, init) => {
    requests.push({ url: url.toString(), init });
    return new Response(null, { status: 204 });
  });
  assert.deepEqual(requests, [{
    url: "http://127.0.0.1:4444/sessions/session%201/request-plan-review",
    init: { method: "POST", headers: { "x-orkworks-open-plan-token": "private-token" } },
  }]);
});

test("starts the sidecar with the plan token and retains the restored workspace", async () => {
  const mainSource = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");

  assert.match(mainSource, /env: \{ \.\.\.process\.env, ORKWORKS_OPEN_PLAN_TOKEN: openPlanToken \}/);
  assert.match(mainSource, /workspacePath = initialWorkspacePath;/);
  assert.match(mainSource, /openPlanToken = randomBytes\(32\)\.toString\("hex"\);/);
});
