import type { SessionInfo } from "../api";

export const ATTENTION_PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  failed: 2,
  done: 3,
  stale: 4,
  working: 5,
  idle: 6,
  creating: 7,
  running: 8,
  ended: 9,
  killed: 10,
  error: 11,
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

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}

export function borderColor(status: string): string {
  return attentionBorderColor(status);
}

export function statusDotColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#666";
  if (status === "working" || status === "running" || status === "creating") return "#4ec94e";
  return "#666";
}

export function attentionBorderColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#4a4a4a";
  return "#3c3c3c";
}

export function sourceColor(source: string | undefined): string {
  if (source === "agent") return "#4ec94e";
  if (source === "peon") return "#57c7ff";
  return "#858585";
}
