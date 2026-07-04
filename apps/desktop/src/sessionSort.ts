import type { SessionInfo } from "./api";

export const ATTENTION_PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  checking_capacity: 2,
  capped: 2,
  failed: 3,
  done: 4,
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
  return (
    status === "blocked" ||
    status === "failed" ||
    status === "waiting_for_input"
  );
}

export function sessionAttentionStatus(session: SessionInfo): string {
  if (session.capacityCheckPending) return "checking_capacity";
  if (session.atUsageLimit) return "capped";
  const lifecyclePhase = session.lifecyclePhase
    ?? (session.status === "creating"
      ? "creating"
      : session.status === "running"
        ? "active"
        : "ended");
  if (lifecyclePhase === "active") {
    return session.observedStatus ?? session.status;
  }
  return session.finalObservedStatus ?? session.status;
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const la = a.memoryState === "live" ? 0 : 1;
    const lb = b.memoryState === "live" ? 0 : 1;
    if (la !== lb) return la - lb;
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}
