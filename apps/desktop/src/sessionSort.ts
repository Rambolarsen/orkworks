import { isHistoricalSession } from "./sessionRuntimeState.ts";
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
  if (isHistoricalSession(session)) return session.terminalOutcome ?? "neutral";
  if (session.lifecycle === "creating" || session.lifecycle === "stopping") return "working";
  // Dead sessions: the backend already knows how the run finished
  // ("ended" | "killed" | "error") — surface it instead of erasing it.
  // A crashed session deserves a second look; one the user killed doesn't.
  if (session.lifecycle !== "alive") return session.terminalOutcome ?? "neutral";
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
  const incomingMap = new Map<string, SessionInfo>();
  for (const session of incoming) {
    incomingMap.set(session.id, session);
  }
  const updated: SessionInfo[] = [];
  let removedAny = false;
  for (const session of existing) {
    const next = incomingMap.get(session.id);
    if (next !== undefined) {
      updated.push(next);
      incomingMap.delete(session.id);
    } else {
      removedAny = true;
    }
  }
  const idsChanged = removedAny || incomingMap.size > 0;
  if (idsChanged || now.getTime() - lastResortAt.getTime() >= THROTTLE_MS) {
    for (const added of incomingMap.values()) {
      updated.push(added);
    }
    return [sortSessions(updated), now];
  }
  return [updated, lastResortAt];
}
