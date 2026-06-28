import test from "node:test";
import assert from "node:assert/strict";

import { groupForSession, groupSessions } from "../src/sessionGroups.ts";
import type { SessionInfo } from "../src/api.ts";

function session(id: string, created_at: string): SessionInfo {
  return {
    id,
    label: id,
    status: "running",
    cwd: "/tmp",
    created_at,
    memoryState: "live",
    resumeStrategy: "none",
  };
}

test("groupForSession buckets a session created earlier today as 'today'", () => {
  const now = new Date("2026-06-28T18:00:00Z");
  assert.equal(groupForSession(session("a", "2026-06-28T01:00:00Z"), now), "today");
});

test("groupForSession buckets a session from 3 days ago as 'week'", () => {
  const now = new Date("2026-06-28T18:00:00Z");
  assert.equal(groupForSession(session("a", "2026-06-25T18:00:00Z"), now), "week");
});

test("groupForSession buckets a session from 10 days ago as 'earlier'", () => {
  const now = new Date("2026-06-28T18:00:00Z");
  assert.equal(groupForSession(session("a", "2026-06-18T18:00:00Z"), now), "earlier");
});

test("groupForSession buckets an unparseable created_at as 'earlier'", () => {
  const now = new Date("2026-06-28T18:00:00Z");
  assert.equal(groupForSession(session("a", "not-a-date"), now), "earlier");
});

test("groupForSession reclassifies the same session as 'today' or 'earlier' depending on now, not just sessions", () => {
  const createdYesterday = session("a", "2026-06-27T23:00:00Z");
  const lateThatNight = new Date("2026-06-27T23:30:00Z");
  const nextAfternoon = new Date("2026-06-28T14:00:00Z");

  assert.equal(groupForSession(createdYesterday, lateThatNight), "today");
  assert.equal(groupForSession(createdYesterday, nextAfternoon), "week");
});

test("groupSessions omits empty buckets and orders today, week, earlier", () => {
  const now = new Date("2026-06-28T18:00:00Z");
  const sessions = [
    session("today-1", "2026-06-28T01:00:00Z"),
    session("old-1", "2026-06-01T00:00:00Z"),
  ];

  const groups = groupSessions(sessions, now);

  assert.deepEqual(groups.map((g) => g.key), ["today", "earlier"]);
  assert.deepEqual(groups.map((g) => g.label), ["Today", "Earlier"]);
  assert.deepEqual(groups[0].items.map((s) => s.id), ["today-1"]);
  assert.deepEqual(groups[1].items.map((s) => s.id), ["old-1"]);
});
