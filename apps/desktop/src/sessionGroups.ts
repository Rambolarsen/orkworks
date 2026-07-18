import type { SessionInfo } from "./api";
import { DAY_MS, delayUntil } from "./labels.ts";

export type GroupKey = "today" | "week" | "earlier";

export const GROUP_LABELS: Record<GroupKey, string> = {
  today: "Today",
  week: "This week",
  earlier: "Earlier",
};

function groupingTimeFor(s: SessionInfo): Date {
  const lastActivity = s.lastActivityAt ? new Date(s.lastActivityAt) : undefined;
  return lastActivity && !Number.isNaN(lastActivity.getTime())
    ? lastActivity
    : new Date(s.created_at);
}

export function groupForSession(s: SessionInfo, now: Date): GroupKey {
  const groupingTime = groupingTimeFor(s);
  if (Number.isNaN(groupingTime.getTime())) return "earlier";
  const sameDay =
    groupingTime.getFullYear() === now.getFullYear() &&
    groupingTime.getMonth() === now.getMonth() &&
    groupingTime.getDate() === now.getDate();
  if (sameDay) return "today";
  const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
  if (now.getTime() - groupingTime.getTime() < sevenDaysMs) return "week";
  return "earlier";
}

export interface SessionGroup {
  key: GroupKey;
  label: string;
  items: SessionInfo[];
}

export function groupSessions(sessions: SessionInfo[], now: Date): SessionGroup[] {
  const buckets: Record<GroupKey, SessionInfo[]> = {
    today: [],
    week: [],
    earlier: [],
  };
  for (const s of sessions) {
    buckets[groupForSession(s, now)].push(s);
  }
  return (["today", "week", "earlier"] as GroupKey[])
    .filter((k) => buckets[k].length > 0)
    .map((k) => ({ key: k, label: GROUP_LABELS[k], items: buckets[k] }));
}

export function nextSessionGroupRefreshMs(
  sessions: SessionInfo[],
  now: Date = new Date(),
): number | null {
  const nowMs = now.getTime();
  const nextMidnight = new Date(now);
  nextMidnight.setHours(24, 0, 0, 0);

  let nextDelay: number | null = null;
  for (const session of sessions) {
    const groupingTime = groupingTimeFor(session);
    if (Number.isNaN(groupingTime.getTime())) continue;

    let candidate: number | null = null;
    switch (groupForSession(session, now)) {
      case "today":
        candidate = delayUntil(nextMidnight.getTime(), nowMs);
        break;
      case "week":
        candidate = delayUntil(groupingTime.getTime() + 7 * DAY_MS, nowMs);
        break;
      case "earlier":
        break;
    }

    if (candidate === null) continue;
    nextDelay = nextDelay === null ? candidate : Math.min(nextDelay, candidate);
  }

  return nextDelay;
}
