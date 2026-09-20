import assert from "node:assert/strict";
import test from "node:test";

import {
  buildWorkspaceRestoreRequest,
  parseWorkspaceRestoreResponse,
  workspaceHistoryPath,
} from "../electron/workspaceRestore.ts";

function jsonResponse(status: number, body?: unknown): Response {
  return new Response(body === undefined ? undefined : JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

test("workspace restore request keeps display path separate from canonical identity", () => {
  assert.deepEqual(buildWorkspaceRestoreRequest("./repo", "/real/repo"), {
    path: "./repo",
    workspaceIdentity: "/real/repo",
  });
});

test("workspace history keeps the canonical request identity when display and canonical paths differ", () => {
  assert.equal(
    workspaceHistoryPath("/real/repo", { path: "./repo" }),
    "/real/repo",
  );
});

test("workspace restore parser exposes the sidecar identity and display path", async () => {
  const workspace = await parseWorkspaceRestoreResponse(jsonResponse(200, {
    path: "./repo",
    workspaceIdentity: "/real/repo",
    repo_root: "/real/repo",
    branch: "main",
    dirty: false,
    lastActiveSessionId: null,
    activeHarnessIds: [],
    activeHarnessRevision: 0,
  }));

  assert.equal(workspace.ok, true);
  if (workspace.ok) {
    assert.equal(workspace.workspace.path, "./repo");
    assert.equal(workspace.workspace.workspaceIdentity, "/real/repo");
  }
});

test("a successful /workspace response maps into the lifecycle workspace shape", async () => {
  const workspace = await parseWorkspaceRestoreResponse(jsonResponse(200, {
    path: "/repo",
    workspaceIdentity: "/repo",
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
      workspaceIdentity: "/repo",
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
    parseWorkspaceRestoreResponse(jsonResponse(200, { path: "/repo", workspaceIdentity: "/repo" })),
    /Workspace restoration returned an invalid active harness revision\./,
  );
});

test("an invalid active harness revision is rejected", async () => {
  for (const activeHarnessRevision of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1, "7", null]) {
    await assert.rejects(
      parseWorkspaceRestoreResponse(jsonResponse(200, {
        path: "/repo",
        workspaceIdentity: "/repo",
        activeHarnessRevision,
      })),
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
    failureCode: "destination_conflict",
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
