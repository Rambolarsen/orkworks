import type { SessionInfo } from "./api.ts";

/**
 * Formats remaining time until a scheduled idle-Active auto-archive deadline
 * as a concise countdown string (e.g. "24h", "6h", "45m", "<1m").
 */
export function formatAutoArchiveRemaining(scheduledAt: string, now: Date): string {
  const targetMs = Date.parse(scheduledAt);
  if (!Number.isFinite(targetMs)) return "soon";
  const remainingMs = Math.max(0, targetMs - now.getTime());
  const remainingMinutes = Math.ceil(remainingMs / 60_000);
  if (remainingMinutes <= 1) return "<1m";
  if (remainingMinutes < 60) return `${remainingMinutes}m`;
  const remainingHours = Math.ceil(remainingMinutes / 60);
  return `${remainingHours}h`;
}

/**
 * Returns the delay in milliseconds until `formatAutoArchiveRemaining` next
 * changes display value, or `null` when the deadline is invalid or elapsed.
 */
export function nextAutoArchiveCountdownRefreshMs(
  scheduledAt: string | null | undefined,
  now: Date,
): number | null {
  if (!scheduledAt) return null;
  const targetMs = Date.parse(scheduledAt);
  if (!Number.isFinite(targetMs)) return null;
  const remainingMs = targetMs - now.getTime();
  if (remainingMs <= 0) return null;
  const remainingMinutes = Math.ceil(remainingMs / 60_000);
  if (remainingMinutes <= 1) return Math.max(1_000, remainingMs);
  if (remainingMinutes <= 60) {
    return Math.max(1_000, remainingMs - (remainingMinutes - 1) * 60_000);
  }
  const remainingHours = Math.ceil(remainingMinutes / 60);
  return Math.max(1_000, remainingMs - (remainingHours - 1) * 3_600_000);
}

export function autoArchiveWarningKey(session: Pick<SessionInfo, "id" | "autoArchiveWarning">): string | null {
  const warnedAt = session.autoArchiveWarning?.warnedAt;
  if (!warnedAt) return null;
  return `${session.id}:${warnedAt}`;
}

export interface AutoArchiveNotificationState {
  scope: string;
  emittedKeys: Set<string>;
}

/**
 * Collects idle-Active auto-archive warnings that have not yet emitted a
 * desktop OS notification for the current `(scope, sessionId, warnedAt)` cycle.
 * Returns at most one notification per `(sessionId, warnedAt)` even across
 * repeated 5-minute retention ticks or session list refreshes.
 */
export function collectNewAutoArchiveWarnings(
  previous: AutoArchiveNotificationState | null,
  scope: string | null,
  sessions: readonly SessionInfo[],
): { state: AutoArchiveNotificationState | null; toNotify: SessionInfo[] } {
  if (scope === null) {
    return { state: null, toNotify: [] };
  }
  const emittedKeys = previous?.scope === scope ? new Set(previous.emittedKeys) : new Set<string>();
  const toNotify: SessionInfo[] = [];
  for (const session of sessions) {
    if (session.sessionState !== "active" || session.favorited === true) continue;
    const key = autoArchiveWarningKey(session);
    if (!key || emittedKeys.has(key)) continue;
    emittedKeys.add(key);
    toNotify.push(session);
  }
  return { state: { scope, emittedKeys }, toNotify };
}
