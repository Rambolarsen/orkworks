import type { SessionInfo } from "./api.ts";

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
}

export function resolvePendingCreates(
  prev: ReadonlySet<string>,
  sessions: readonly SessionInfo[],
): PendingCreateResolution {
  if (prev.size === 0) return { ids: prev, erroredIds: [] };
  const statusById = new Map(sessions.map((s) => [s.id, s.status]));
  const ids = new Set<string>();
  const erroredIds: string[] = [];
  for (const id of prev) {
    const status = statusById.get(id);
    if (status === "creating") {
      ids.add(id);
      continue;
    }
    // Dropped silently for "running" (the normal case) and for a missing
    // status (the session vanished from the poll, e.g. deleted mid-create) —
    // only "error" gets a toast.
    if (status === "error") {
      erroredIds.push(id);
    }
  }
  return { ids, erroredIds };
}
