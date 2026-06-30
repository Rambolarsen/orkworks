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

function localDate(year: number, month: number, day: number, hours: number, minutes = 0): Date {
  return new Date(year, month - 1, day, hours, minutes);
}

function localIso(year: number, month: number, day: number, hours: number, minutes = 0): string {
  return localDate(year, month, day, hours, minutes).toISOString();
}

test("groupForSession buckets a session created earlier today as 'today'", () => {
  const now = localDate(2026, 6, 28, 18, 0);
  assert.equal(groupForSession(session("a", localIso(2026, 6, 28, 1, 0)), now), "today");
});

test("groupForSession buckets a session from 3 days ago as 'week'", () => {
  const now = localDate(2026, 6, 28, 18, 0);
  assert.equal(groupForSession(session("a", localIso(2026, 6, 25, 18, 0)), now), "week");
});

test("groupForSession buckets a session from 10 days ago as 'earlier'", () => {
  const now = localDate(2026, 6, 28, 18, 0);
  assert.equal(groupForSession(session("a", localIso(2026, 6, 18, 18, 0)), now), "earlier");
});

test("groupForSession buckets an unparseable created_at as 'earlier'", () => {
  const now = localDate(2026, 6, 28, 18, 0);
  assert.equal(groupForSession(session("a", "not-a-date"), now), "earlier");
});

test("groupForSession reclassifies the same session as 'today' or 'week' based on the local calendar day", () => {
  const createdYesterday = session("a", localIso(2026, 6, 27, 23, 0));
  const lateThatNight = localDate(2026, 6, 27, 23, 30);
  const nextAfternoon = localDate(2026, 6, 28, 14, 0);

  assert.equal(groupForSession(createdYesterday, lateThatNight), "today");
  assert.equal(groupForSession(createdYesterday, nextAfternoon), "week");
});

test("groupSessions omits empty buckets and orders today, week, earlier", () => {
  const now = localDate(2026, 6, 28, 18, 0);
  const sessions = [
    session("today-1", localIso(2026, 6, 28, 1, 0)),
    session("old-1", localIso(2026, 6, 1, 0, 0)),
  ];

  const groups = groupSessions(sessions, now);

  assert.deepEqual(groups.map((g) => g.key), ["today", "earlier"]);
  assert.deepEqual(groups.map((g) => g.label), ["Today", "Earlier"]);
  assert.deepEqual(groups[0].items.map((s) => s.id), ["today-1"]);
  assert.deepEqual(groups[1].items.map((s) => s.id), ["old-1"]);
});
