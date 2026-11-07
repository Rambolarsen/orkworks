import type { SessionInfo } from "./api.ts";
import { startupFailureMessage } from "./startupDiagnostics.ts";

/**
 * The New Session dialog's create response now reflects the pre-spawn
 * "creating" record (issue #302) — the actual spawn/failure transition
 * happens asynchronously, observed only via the existing poll. A dialog
 * create used to surface a spawn failure as a synchronous 500, caught
 * directly in the dialog's own error handler; now that failure shows up
 * later, as this dialog-created id resolving to status "error" on some
 * future poll. Track ids added via the dialog so that transition, and only
 * that transition, still fires the "couldn't start" toast — an unrelated
 * failure much later in an established session must never misfire it.
 */
export function trackPendingCreate(prev: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(prev);
  next.add(id);
  return next;
}

export interface PendingCreateResolution {
  ids: ReadonlySet<string>;
  erroredIds: readonly string[];
  erroredSessions: readonly SessionInfo[];
}

export function resolvePendingCreates(
  prev: ReadonlySet<string>,
  sessions: readonly SessionInfo[],
): PendingCreateResolution {
  if (prev.size === 0) return { ids: prev, erroredIds: [], erroredSessions: [] };
  const sessionById = new Map(sessions.map((s) => [s.id, s]));
  const ids = new Set<string>();
  const erroredIds: string[] = [];
  const erroredSessions: SessionInfo[] = [];
  for (const id of prev) {
    const session = sessionById.get(id);
    const status = session?.status;
    if (session && startupFailureMessage(session.startupFailure)) {
      erroredIds.push(id);
      erroredSessions.push(session);
      continue;
    }
    if (status === "creating") {
      ids.add(id);
      continue;
    }
    // Dropped silently for "running" (the normal case) and for a missing
    // status (the session vanished from the poll, e.g. deleted mid-create) —
    // only "error" gets a toast.
    if (status === "error") {
      erroredIds.push(id);
      if (session) erroredSessions.push(session);
    }
  }
  return { ids, erroredIds, erroredSessions };
}
