import type { SessionInfo } from "./api.ts";
import { sessionAttentionStatus } from "./sessionSort.ts";

/**
 * Unread — "something changed since you last looked" — is orthogonal to
 * attention tone: tone says *what* a session needs, unread says *have you
 * seen it*. A live session becomes unread when it finishes a working turn
 * while it is not selected; selecting it clears the flag.
 *
 * The signature is the normalized attention status, not raw activity — an
 * actively working session emits output every poll, and flagging that would
 * make unread permanent noise instead of a signal.
 *
 * In-memory only: unread resets on app restart.
 */
export interface UnreadState {
  signatures: ReadonlyMap<string, string>;
  unreadIds: ReadonlySet<string>;
  acknowledgedIds: ReadonlySet<string>;
}

export const EMPTY_UNREAD_STATE: UnreadState = {
  signatures: new Map(),
  unreadIds: new Set(),
  acknowledgedIds: new Set(),
};

const WORKING_RESULTS = new Set(["idle", "needs_you", "blocked", "failed", "capped"]);

export function trackUnread(
  prev: UnreadState,
  sessions: readonly SessionInfo[],
  activeSessionId: string | null,
): UnreadState {
  const signatures = new Map<string, string>();
  const unreadIds = new Set<string>();
  const acknowledgedIds = new Set(prev.acknowledgedIds);
  const sessionIds = new Set(sessions.map((session) => session.id));
  for (const id of acknowledgedIds) {
    if (!sessionIds.has(id)) acknowledgedIds.delete(id);
  }
  for (const s of sessions) {
    // "working" while still spawning the PTY isn't a real working turn —
    // give it its own signature so settling to alive/idle right after
    // never reads as an unread result.
    const sig = s.lifecycle === "creating" ? "creating" : sessionAttentionStatus(s);
    signatures.set(s.id, sig);
    if (sig === "working" || sig === "creating") acknowledgedIds.delete(s.id);
    if (s.id === activeSessionId) continue; // being looked at right now
    const prevSig = prev.signatures.get(s.id);
    const becameResult = prevSig === "working" && WORKING_RESULTS.has(sig);
    if (s.lifecycle === "alive" && (becameResult || prev.unreadIds.has(s.id))) {
      unreadIds.add(s.id);
    }
    if (becameResult) acknowledgedIds.delete(s.id);
  }
  // Sessions are re-fetched every couple of seconds; returning the same
  // object when nothing changed lets React setState bail out instead of
  // re-rendering the whole panel tree per poll.
  if (
    mapsEqual(prev.signatures, signatures) &&
    setsEqual(prev.unreadIds, unreadIds) &&
    setsEqual(prev.acknowledgedIds, acknowledgedIds)
  ) {
    return prev;
  }
  return { signatures, unreadIds, acknowledgedIds };
}

function mapsEqual(a: ReadonlyMap<string, string>, b: ReadonlyMap<string, string>): boolean {
  if (a.size !== b.size) return false;
  for (const [k, v] of b) if (a.get(k) !== v) return false;
  return true;
}

function setsEqual(a: ReadonlySet<string>, b: ReadonlySet<string>): boolean {
  if (a.size !== b.size) return false;
  for (const v of b) if (!a.has(v)) return false;
  return true;
}

export function clearUnread(state: UnreadState, id: string): UnreadState {
  if (!state.unreadIds.has(id)) return state;
  const unreadIds = new Set(state.unreadIds);
  unreadIds.delete(id);
  return { ...state, unreadIds };
}

export function acknowledgeSession(state: UnreadState, id: string): UnreadState {
  const unreadIds = new Set(state.unreadIds);
  unreadIds.delete(id);
  if (state.acknowledgedIds.has(id) && unreadIds.size === state.unreadIds.size) return state;
  const acknowledgedIds = new Set(state.acknowledgedIds);
  acknowledgedIds.add(id);
  return { ...state, unreadIds, acknowledgedIds };
}
