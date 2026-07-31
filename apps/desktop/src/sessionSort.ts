import type { SessionInfo } from "./api.ts";
import { lastActivityTimestamp } from "./labels.ts";

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
  // Spawning or tearing down the PTY is real activity, so "working" is fair
  // here. Once lifecycle settles to alive, nothing sets attention to
  // "working" on its own — the fallback below reads idle immediately unless
  // the harness has actually reported otherwise.
  if (session.lifecycle === "creating" || session.lifecycle === "stopping") return "working";
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
    const ta = Date.parse(lastActivityTimestamp(a) ?? "");
    const tb = Date.parse(lastActivityTimestamp(b) ?? "");
    if (!Number.isNaN(ta) && !Number.isNaN(tb) && ta !== tb) return tb - ta;
    return a.label.localeCompare(b.label);
  });
}

export function mergeSessionsById(
  existing: readonly SessionInfo[],
  incoming: readonly SessionInfo[],
  now = new Date(),
): SessionInfo[] {
  if (existing.length === 0) return sortSessions([...incoming]);

  const updates = new Map(incoming.map((session) => [session.id, session]));
  const existingIds = new Set(existing.map((session) => session.id));
  const sessions = [
    ...existing.filter((session) => updates.has(session.id)).map((session) => updates.get(session.id)!),
    ...[...updates.values()].filter((session) => !existingIds.has(session.id)),
  ];
  const alive = sessions.filter((session) => session.lifecycle === "alive");
  const nonAlive = sessions.filter((session) => session.lifecycle !== "alive");
  const top = alive[0];

  if (top && isAtLeastOneMinuteOld(top, now)) {
    const promoted = newestUpdatedAliveSession(alive, existing, updates);
    if (promoted && promoted.id !== top.id) {
      return [promoted, ...alive.filter((session) => session.id !== promoted.id), ...nonAlive];
    }
  }

  return [...alive, ...nonAlive];
}

function isAtLeastOneMinuteOld(session: SessionInfo, now: Date): boolean {
  const timestamp = Date.parse(lastActivityTimestamp(session) ?? "");
  return !Number.isNaN(timestamp) && now.getTime() - timestamp >= 60_000;
}

function newestUpdatedAliveSession(
  alive: readonly SessionInfo[],
  existing: readonly SessionInfo[],
  updates: ReadonlyMap<string, SessionInfo>,
): SessionInfo | undefined {
  const previousById = new Map(existing.map((session) => [session.id, session]));
  return alive.reduce<SessionInfo | undefined>((newest, session) => {
    const previous = previousById.get(session.id);
    const updated = updates.get(session.id);
    const timestamp = Date.parse(lastActivityTimestamp(session) ?? "");
    const previousTimestamp = Date.parse(lastActivityTimestamp(previous ?? session) ?? "");
    if (
      !updated ||
      Number.isNaN(timestamp) ||
      Number.isNaN(previousTimestamp) ||
      timestamp <= previousTimestamp ||
      (newest && timestamp <= Date.parse(lastActivityTimestamp(newest) ?? ""))
    ) {
      return newest;
    }
    return session;
  }, undefined);
}
