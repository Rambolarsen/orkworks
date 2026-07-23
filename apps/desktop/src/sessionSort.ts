import type { SessionInfo } from "./api.ts";

const ATTENTION_PRIORITY: Record<string, number> = {
  needs_you: 0,
  blocked: 1,
  capped: 2,
  failed: 3,
  working: 5,
  idle: 6,
  neutral: 99,
};

export function needsAttention(status: string): boolean {
  return (
    status === "blocked" ||
    status === "failed" ||
    status === "needs_you"
  );
}

export function sessionAttentionStatus(session: SessionInfo): string {
  // Spawning a PTY isn't the harness doing anything yet — read as idle, not working.
  if (session.lifecycle === "creating") return "idle";
  if (session.lifecycle !== "alive") return "neutral";
  return session.attention ?? "idle";
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const la = a.lifecycle === "alive" ? 0 : 1;
    const lb = b.lifecycle === "alive" ? 0 : 1;
    if (la !== lb) return la - lb;
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}

export function mergeSessionsById(
  existing: readonly SessionInfo[],
  incoming: readonly SessionInfo[],
): SessionInfo[] {
  const sessions = new Map(existing.map((session) => [session.id, session]));
  for (const session of incoming) {
    sessions.set(session.id, session);
  }
  return sortSessions([...sessions.values()]);
}
