import test from "node:test";
import assert from "node:assert/strict";

import {
  needsAttention,
  isLive,
  borderColor,
  sourceColor,
  sortSessions,
  sessionAttentionStatus,
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

test("borderColor returns correct colors per status", () => {
  assert.equal(borderColor("running"), "#4ec94e");
  assert.equal(borderColor("creating"), "#4ec94e");
  assert.equal(borderColor("blocked"), "#d4d44e");
  assert.equal(borderColor("waiting_for_input"), "#d4d44e");
  assert.equal(borderColor("failed"), "#cc4444");
  assert.equal(borderColor("ended"), "#666");
});

test("sourceColor returns agent/peon colors or default", () => {
  assert.equal(sourceColor("agent"), "#4ec94e");
  assert.equal(sourceColor("peon"), "#57c7ff");
  assert.equal(sourceColor("process"), "#858585");
  assert.equal(sourceColor(undefined), "#858585");
});

test("sortSessions sorts by priority then label", () => {
  const sessions: SessionInfo[] = [
    { id: "1", label: "B session", status: "running", cwd: "/tmp", created_at: "now" },
    { id: "2", label: "A session", status: "blocked", cwd: "/tmp", created_at: "now" },
    { id: "3", label: "C session", status: "running", cwd: "/tmp", created_at: "now" },
    { id: "4", label: "D session", status: "failed", cwd: "/tmp", created_at: "now" },
  ];
  const sorted = sortSessions(sessions);
  assert.equal(sorted[0].id, "2"); // blocked first
  assert.equal(sorted[1].id, "4"); // failed second
  assert.equal(sorted[2].id, "1"); // running, label "B session"
  assert.equal(sorted[3].id, "3"); // running, label "C session"
});

test("sessionAttentionStatus prefers observed status over lifecycle status", () => {
  const session: SessionInfo = {
    id: "1",
    label: "Running session needing input",
    status: "running",
    observedStatus: "waiting_for_input",
    cwd: "/tmp",
    created_at: "now",
  };

  assert.equal(sessionAttentionStatus(session), "waiting_for_input");
  assert.equal(needsAttention(sessionAttentionStatus(session)), true);
});

test("sortSessions uses observed status for attention priority", () => {
  const sessions: SessionInfo[] = [
    { id: "1", label: "Active", status: "running", cwd: "/tmp", created_at: "now" },
    {
      id: "2",
      label: "Needs input",
      status: "running",
      observedStatus: "waiting_for_input",
      cwd: "/tmp",
      created_at: "now",
    },
  ];

  const sorted = sortSessions(sessions);
  assert.equal(sorted[0].id, "2");
});
