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
    activeHarnessRevision: 7,
  }));

  assert.deepEqual(workspace, {
    ok: true,
    workspace: {
      path: "/repo",
      repo_root: "/repo",
      branch: "main",
      dirty: false,
      lastActiveSessionId: "s1",
      activeHarnessIds: ["claude"],
      activeHarnessRevision: 7,
    },
  });
});

test("a missing active harness revision is rejected", async () => {
  await assert.rejects(
    parseWorkspaceRestoreResponse(jsonResponse(200, { path: "/repo" })),
    /Workspace restoration returned an invalid active harness revision\./,
  );
});

test("an invalid active harness revision is rejected", async () => {
  for (const activeHarnessRevision of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1, "7", null]) {
    await assert.rejects(
      parseWorkspaceRestoreResponse(jsonResponse(200, { path: "/repo", activeHarnessRevision })),
      /Workspace restoration returned an invalid active harness revision\./,
    );
  }
});

test("a lease or accessibility conflict preserves history for a later retry", async () => {
  assert.deepEqual(await parseWorkspaceRestoreResponse(jsonResponse(400, "unknown path")), {
    ok: false,
    status: 400,
    removeFromHistory: false,
  });
  assert.deepEqual(await parseWorkspaceRestoreResponse(jsonResponse(409, "lease")), {
    ok: false,
    status: 409,
    removeFromHistory: false,
  });
});

test("only a missing workspace is removed from history", async () => {
  assert.deepEqual(await parseWorkspaceRestoreResponse(jsonResponse(404, "not found")), {
    ok: false,
    status: 404,
    removeFromHistory: true,
  });
});

test("a server error still surfaces as a backend restoration failure", async () => {
  await assert.rejects(
    parseWorkspaceRestoreResponse(jsonResponse(500, "poisoned lock")),
    /Workspace restoration failed: 500/,
  );
});
