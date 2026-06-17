import test from "node:test";
import assert from "node:assert/strict";

import {
  attentionBorderColor,
  borderColor,
  isLive,
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
  sourceColor,
  statusDotColor,
} from "../src/components/RightSidebarHelpers.ts";
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

test("isLive returns true for running and creating", () => {
  assert.equal(isLive("running"), true);
  assert.equal(isLive("creating"), true);
  assert.equal(isLive("ended"), false);
  assert.equal(isLive("killed"), false);
});

test("attentionBorderColor returns correct colors", () => {
  assert.equal(attentionBorderColor("waiting_for_input"), "#cc4444");
  assert.equal(attentionBorderColor("failed"), "#cc4444");
  assert.equal(attentionBorderColor("blocked"), "#d4d44e");
  assert.equal(attentionBorderColor("done"), "#4ec94e");
  assert.equal(attentionBorderColor("stale"), "#4a4a4a");
  assert.equal(attentionBorderColor("idle"), "#4a4a4a");
  assert.equal(attentionBorderColor("working"), "#3c3c3c");
});

test("borderColor delegates to attentionBorderColor for compatibility", () => {
  assert.equal(borderColor("waiting_for_input"), "#cc4444");
  assert.equal(borderColor("blocked"), "#d4d44e");
  assert.equal(borderColor("done"), "#4ec94e");
  assert.equal(borderColor("running"), "#3c3c3c");
});

test("statusDotColor returns correct colors per attention status", () => {
  assert.equal(statusDotColor("waiting_for_input"), "#cc4444");
  assert.equal(statusDotColor("failed"), "#cc4444");
  assert.equal(statusDotColor("blocked"), "#d4d44e");
  assert.equal(statusDotColor("done"), "#4ec94e");
  assert.equal(statusDotColor("stale"), "#666");
  assert.equal(statusDotColor("idle"), "#666");
  assert.equal(statusDotColor("working"), "#4ec94e");
  assert.equal(statusDotColor("running"), "#4ec94e");
  assert.equal(statusDotColor("creating"), "#4ec94e");
  assert.equal(statusDotColor("unknown-status"), "#666");
});

test("sourceColor returns agent/peon colors or default", () => {
  assert.equal(sourceColor("agent"), "#4ec94e");
  assert.equal(sourceColor("peon"), "#57c7ff");
  assert.equal(sourceColor("process"), "#858585");
  assert.equal(sourceColor(undefined), "#858585");
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
