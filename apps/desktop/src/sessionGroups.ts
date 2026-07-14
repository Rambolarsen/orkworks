import type { SessionInfo } from "./api";

export type GroupKey = "today" | "week" | "earlier";

export const GROUP_LABELS: Record<GroupKey, string> = {
  today: "Today",
  week: "This week",
  earlier: "Earlier",
};

export function groupForSession(s: SessionInfo, now: Date): GroupKey {
  const lastActivity = s.lastActivityAt ? new Date(s.lastActivityAt) : undefined;
  const groupingTime = lastActivity && !Number.isNaN(lastActivity.getTime())
    ? lastActivity
    : new Date(s.created_at);
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
