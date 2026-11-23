import type { SessionInfo } from "./api.ts";
import { isSessionAlive } from "./sessionRuntimeState.ts";
import { sessionAttentionStatus } from "./sessionSort.ts";

interface AttentionSnapshot {
  scope: string;
  waitingIds: ReadonlySet<string>;
}

export function observeSessionAttention(
  previous: AttentionSnapshot | null,
  scope: string | null,
  sessions: readonly SessionInfo[],
): { snapshot: AttentionSnapshot | null; newWaits: SessionInfo[] } {
  if (scope === null) return { snapshot: null, newWaits: [] };

  const waitingIds = new Set<string>();
  const newWaits: SessionInfo[] = [];
  for (const session of sessions) {
    if (!isSessionAlive(session) || sessionAttentionStatus(session) !== "needs_you" || waitingIds.has(session.id)) continue;
    waitingIds.add(session.id);
    if (previous?.scope === scope && !previous.waitingIds.has(session.id)) newWaits.push(session);
  }

  return { snapshot: { scope, waitingIds }, newWaits };
}

export function sessionAlertDelivery(
  sessionId: string,
  visibleSessionId: string | null,
  appFocused: boolean,
  desktopEnabled: boolean,
): "toast" | "desktop" | null {
  if (appFocused) return sessionId === visibleSessionId ? null : "toast";
  return desktopEnabled ? "desktop" : null;
}
