import test from "node:test";
import assert from "node:assert/strict";

import type { SessionInfo, WorkspaceInfo } from "../src/api.ts";

test("SessionInfo type accepts metadata fields", () => {
  const session: SessionInfo = {
    id: "test",
    label: "Test",
    status: "running",
    cwd: "/tmp",
    created_at: "now",
    metadataSource: "process",
    metadataConfidence: 1.0,
  };
  assert.equal(session.metadataSource, "process");
  assert.equal(session.metadataConfidence, 1.0);
});

test("WorkspaceInfo type has expected shape", () => {
  const ws: WorkspaceInfo = {
    path: "/tmp/project",
    repo_root: "/tmp/project",
    branch: "main",
    dirty: false,
  };
  assert.equal(ws.path, "/tmp/project");
  assert.equal(ws.branch, "main");
});
