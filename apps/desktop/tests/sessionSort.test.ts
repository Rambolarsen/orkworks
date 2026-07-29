import test from "node:test";
import assert from "node:assert/strict";

import {
  mergeSessionsById,
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
} from "../src/sessionSort.ts";
import type { SessionInfo } from "../src/api.ts";

function session(
  id: string,
  lifecycle: NonNullable<SessionInfo["lifecycle"]>,
  attention?: SessionInfo["attention"],
): SessionInfo {
  return {
    id, label: id, status: "running", lifecycle, attention,
    cwd: "/tmp", created_at: "now", memoryState: lifecycle === "alive" ? "live" : "remembered",
    resumeStrategy: "none",
  };
}

test("needsAttention recognizes only actionable alive attention", () => {
  assert.equal(needsAttention("needs_you"), true);
  assert.equal(needsAttention("blocked"), true);
  assert.equal(needsAttention("failed"), true);
  assert.equal(needsAttention("working"), false);
  assert.equal(needsAttention("idle"), false);
  assert.equal(needsAttention("capped"), false);
});

test("alive sessions use attention and dead sessions are neutral", () => {
  assert.equal(sessionAttentionStatus(session("working", "alive", "working")), "working");
  assert.equal(sessionAttentionStatus(session("idle", "alive")), "idle");
  assert.equal(sessionAttentionStatus(session("dead", "dead", "blocked")), "neutral");
});

test("a session spawning or tearing down its PTY reads working", () => {
  assert.equal(sessionAttentionStatus(session("creating", "creating")), "working");
  assert.equal(sessionAttentionStatus(session("stopping", "stopping")), "working");
});

test("a session reads idle the instant it goes alive, even before the harness reports in", () => {
  assert.equal(sessionAttentionStatus(session("just-alive", "alive")), "idle");
});

test("sortSessions ranks actionable alive sessions before working, idle, and dead", () => {
  const ordered = sortSessions([
    session("dead", "dead"),
    session("idle", "alive", "idle"),
    session("working", "alive", "working"),
    session("failed", "alive", "failed"),
    session("needs-you", "alive", "needs_you"),
  ]);
  assert.deepEqual(ordered.map((item) => item.id), ["needs-you", "failed", "working", "idle", "dead"]);
});

test("sortSessions breaks attention ties by most recent activity, not label", () => {
  const stale = { ...session("4Seems the branch", "alive", "idle"), lastActivityAt: "2026-07-28T08:00:00.000Z" };
  const recent = { ...session("keep going", "alive", "idle"), lastActivityAt: "2026-07-28T21:00:00.000Z" };

  const ordered = sortSessions([stale, recent]);

  assert.deepEqual(ordered.map((item) => item.id), ["keep going", "4Seems the branch"]);
});

test("sortSessions uses recent output when it is newer than meaningful activity", () => {
  const stale = { ...session("stale", "alive", "idle"), lastActivityAt: "2026-07-28T21:00:00.000Z" };
  const recentOutput = {
    ...session("recent-output", "alive", "idle"),
    lastActivityAt: "2026-07-28T08:00:00.000Z",
    lastOutputAt: "2026-07-28T22:00:00.000Z",
  };

  assert.deepEqual(sortSessions([stale, recentOutput]).map((item) => item.id), ["recent-output", "stale"]);
});

test("mergeSessionsById keeps one row when a creation response repeats a polled session", () => {
  const existing = session("existing", "alive");
  const polledNew = session("new", "alive");
  const createdNew = { ...polledNew, label: "created-new" };

  const merged = mergeSessionsById([existing, polledNew], [createdNew]);

  assert.deepEqual(merged.map((item) => item.id), ["new", "existing"]);
  assert.strictEqual(merged.find((item) => item.id === "new"), createdNew);
});
