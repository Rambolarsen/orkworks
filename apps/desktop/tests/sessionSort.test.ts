import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

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

test("sortSessions orders purely by lastActivityTimestamp descending, ignoring attention", () => {
  const olderButNeedsYou = {
    ...session("older-needs-you", "alive", "needs_you"),
    lastActivityAt: "2026-08-01T10:00:00.000Z",
  };
  const newerButIdle = {
    ...session("newer-idle", "alive", "idle"),
    lastActivityAt: "2026-08-01T11:00:00.000Z",
  };

  const ordered = sortSessions([olderButNeedsYou, newerButIdle]);

  assert.deepEqual(ordered.map((item) => item.id), ["newer-idle", "older-needs-you"]);
});

test("sortSessions orders by most recent activity when timestamps differ", () => {
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

test("mergeSessionsById returns a [list, nextLastResortAt] tuple for an initial empty list, with nextLastResortAt === now", () => {
  const now = new Date("2026-08-15T12:00:00.000Z");
  const lastResortAt = new Date("2026-08-15T11:59:55.000Z");
  const incoming = [session("a", "alive"), session("b", "alive")];

  const [merged, nextLastResortAt] = mergeSessionsById([], incoming, lastResortAt, now);

  assert.equal(merged.length, 2);
  assert.equal(nextLastResortAt, now);
});

test("mergeSessionsById drops existing rows absent from an authoritative polling snapshot", () => {
  const existing = session("existing", "alive");
  const polledNew = session("new", "alive");
  const createdNew = { ...polledNew, label: "created-new" };

  const [merged] = mergeSessionsById([existing, polledNew], [createdNew]);

  assert.deepEqual(merged.map((item) => item.id), ["new"]);
  assert.strictEqual(merged.find((item) => item.id === "new"), createdNew);
});

test("mergeSessionsById drops every existing row when the polling snapshot is empty", () => {
  const [merged] = mergeSessionsById([session("forgotten", "dead")], []);
  assert.deepEqual(merged, []);
});

test("mergeSessionsById sorts an initial polling snapshot deterministically", () => {
  const incoming = [
    session("dead", "dead"),
    session("idle", "alive", "idle"),
    session("needs-you", "alive", "needs_you"),
  ];

  const [merged] = mergeSessionsById([], incoming);
  assert.deepEqual(merged, sortSessions(incoming));
});

test("App combines a creation response with the current snapshot before merging", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /setSessions\(\s*\(?\s*(\w+)\s*\)?\s*=>\s*mergeSessionsById\(\s*\1\s*,\s*\[\s*\.\.\.\1\s*,\s*session\s*\]\s*\)\s*\);/);
});
