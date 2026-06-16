import type { SessionInfo } from "../api";

export const PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  failed: 2,
  running: 3,
  creating: 4,
  idle: 5,
};

export function needsAttention(status: string): boolean {
  return status === "blocked" || status === "failed" || status === "waiting_for_input";
}

export function sessionAttentionStatus(session: SessionInfo): string {
  return session.observedStatus ?? session.status;
}

export function isLive(status: string): boolean {
  return status === "running" || status === "creating";
}

export function borderColor(status: string): string {
  if (status === "running" || status === "creating") return "#4ec94e";
  if (status === "blocked" || status === "waiting_for_input") return "#d4d44e";
  if (status === "failed") return "#cc4444";
  return "#666";
}

export function sourceColor(source: string | undefined): string {
  if (source === "agent") return "#4ec94e";
  if (source === "peon") return "#57c7ff";
  return "#858585";
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const pa = PRIORITY[sessionAttentionStatus(a)] ?? 9;
    const pb = PRIORITY[sessionAttentionStatus(b)] ?? 9;
    if (pa !== pb) return pa - pb;
    return a.label < b.label ? -1 : a.label > b.label ? 1 : 0;
  });
}
