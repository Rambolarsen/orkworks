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
    observedStatus: "waiting_for_input",
    summary: "Needs approval",
    nextAction: "Choose an option",
    needsUserInput: true,
    detectedQuestion: "Proceed?",
    suggestedOptions: ["yes", "no"],
    blockerDescription: "Waiting on user",
    failedCommand: "cargo test",
    failedTest: "integration",
    capacityHints: ["capped"],
    peonLastInference: "2026-06-16T12:00:00Z",
    metadataSource: "process",
    metadataConfidence: 1.0,
  };
  assert.equal(session.metadataSource, "process");
  assert.equal(session.metadataConfidence, 1.0);
  assert.equal(session.observedStatus, "waiting_for_input");
  assert.equal(session.needsUserInput, true);
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
