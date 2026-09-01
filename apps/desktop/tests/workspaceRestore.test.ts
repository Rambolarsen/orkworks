import assert from "node:assert/strict";
import test from "node:test";

import { parseWorkspaceRestoreResponse } from "../electron/workspaceRestore.ts";

function jsonResponse(status: number, body?: unknown): Response {
  return new Response(body === undefined ? undefined : JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

test("a successful /workspace response maps into the lifecycle workspace shape", async () => {
  const workspace = await parseWorkspaceRestoreResponse(jsonResponse(200, {
    path: "/repo",
    repo_root: "/repo",
    branch: "main",
    dirty: false,
    lastActiveSessionId: "s1",
    activeHarnessIds: ["claude"],
  }));

  assert.deepEqual(workspace, {
    path: "/repo",
    repo_root: "/repo",
    branch: "main",
    dirty: false,
    lastActiveSessionId: "s1",
    activeHarnessIds: ["claude"],
  });
});

test("missing fields in a successful response fall back to the lifecycle defaults", async () => {
  const workspace = await parseWorkspaceRestoreResponse(jsonResponse(200, { path: "/repo" }));

  assert.deepEqual(workspace, {
    path: "/repo",
    repo_root: null,
    branch: null,
    dirty: null,
    lastActiveSessionId: null,
    activeHarnessIds: [],
  });
});

test("a client error from a healthy sidecar means no workspace, not a backend failure", async () => {
  assert.equal(await parseWorkspaceRestoreResponse(jsonResponse(400, "unknown path")), null);
  assert.equal(await parseWorkspaceRestoreResponse(jsonResponse(404, "not found")), null);
});

test("a server error still surfaces as a backend restoration failure", async () => {
  await assert.rejects(
    parseWorkspaceRestoreResponse(jsonResponse(500, "poisoned lock")),
    /Workspace restoration failed: 500/,
  );
});
