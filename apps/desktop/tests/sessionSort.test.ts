import test from "node:test";
import assert from "node:assert/strict";

import {
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
} from "../src/sessionSort.ts";
import type { SessionInfo } from "../src/api.ts";

test("needsAttention returns true for blocked, failed, waiting_for_input", () => {
  assert.equal(needsAttention("blocked"), true);
  assert.equal(needsAttention("failed"), true);
  assert.equal(needsAttention("waiting_for_input"), true);
});

test("needsAttention returns false for running, creating, ended", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("creating"), false);
  assert.equal(needsAttention("ended"), false);
});

test("sortSessions sorts by design attention priority then label", () => {
  const sessions: SessionInfo[] = [
    { id: "running", label: "H running", status: "running", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "blocked", label: "B blocked", status: "running", observedStatus: "blocked", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "idle", label: "G idle", status: "running", observedStatus: "idle", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "done", label: "D done", status: "running", observedStatus: "done", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "failed", label: "C failed", status: "running", observedStatus: "failed", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "stale", label: "E stale", status: "running", observedStatus: "stale", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "working", label: "F working", status: "running", observedStatus: "working", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "waiting", label: "A waiting", status: "running", observedStatus: "waiting_for_input", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
  ];

  assert.deepEqual(sortSessions(sessions).map((s) => s.id), [
    "waiting",
    "blocked",
    "failed",
    "done",
    "stale",
    "working",
    "idle",
    "running",
  ]);
});

test("sessionAttentionStatus prefers observed status over lifecycle status", () => {
  const session: SessionInfo = {
    id: "1",
    label: "Running session needing input",
    status: "running",
    observedStatus: "waiting_for_input",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "live",
    resumeStrategy: "none",
  };

  assert.equal(sessionAttentionStatus(session), "waiting_for_input");
  assert.equal(needsAttention(sessionAttentionStatus(session)), true);
});

test("sessionAttentionStatus falls back to lifecycle status when no observed", () => {
  const session: SessionInfo = {
    id: "1", label: "test", status: "running", cwd: "/tmp", created_at: "now",
    memoryState: "live", resumeStrategy: "none",
  };
  assert.equal(sessionAttentionStatus(session), "running");
});

test("sessionAttentionStatus prefers checking_capacity over capped while pending", () => {
  const session: SessionInfo = {
    id: "pending",
    label: "Pending",
    status: "running",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "live",
    resumeStrategy: "none",
    capacityCheckPending: true,
    atUsageLimit: true,
  };

  assert.equal(sessionAttentionStatus(session), "checking_capacity");
});
