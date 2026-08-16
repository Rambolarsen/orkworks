import type { SessionInfo } from "./api.ts";
import { lastActivityTimestamp } from "./labels.ts";

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
    const ta = Date.parse(lastActivityTimestamp(a) ?? "");
    const tb = Date.parse(lastActivityTimestamp(b) ?? "");
    if (!Number.isNaN(ta) && !Number.isNaN(tb) && ta !== tb) return tb - ta;
    const ca = Date.parse(a.created_at ?? "");
    const cb = Date.parse(b.created_at ?? "");
    if (!Number.isNaN(ca) && !Number.isNaN(cb) && ca !== cb) return cb - ca;
    return a.label.localeCompare(b.label);
  });
}

const THROTTLE_MS = 30_000;

export function mergeSessionsById(
  existing: readonly SessionInfo[],
  incoming: readonly SessionInfo[],
  lastResortAt: Date = new Date(0),
  now: Date = new Date(),
): [SessionInfo[], Date] {
  if (existing.length === 0) {
    return [sortSessions([...incoming]), now];
  }
  const existingIds = new Set(existing.map((session) => session.id));
  const incomingMap = new Map(incoming.map((session) => [session.id, session]));
  const incomingIds = new Set(incomingMap.keys());
  const idsChanged =
    existingIds.size !== incomingIds.size ||
    [...existingIds].some((id) => !incomingIds.has(id));
  const updated = existing
    .filter((session) => incomingMap.has(session.id))
    .map((session) => incomingMap.get(session.id)!);
  const added = [...incomingMap.values()].filter((session) => !existingIds.has(session.id));
  if (idsChanged || now.getTime() - lastResortAt.getTime() >= THROTTLE_MS) {
    return [sortSessions([...updated, ...added]), now];
  }
  return [updated, lastResortAt];
}
