import { effectiveLifecyclePhase, type SessionInfo } from "./api.ts";
import { AttentionState } from "./domain/session.ts";

const ATTENTION_PRIORITY: Record<AttentionState, number> = {
  [AttentionState.WaitingForInput]: 0,
  [AttentionState.CheckingCapacity]: 1,
  [AttentionState.Capped]: 2,
  [AttentionState.Blocked]: 3,
  [AttentionState.Failed]: 4,
  [AttentionState.Done]: 5,
  [AttentionState.Stale]: 6,
  [AttentionState.Working]: 7,
  [AttentionState.Idle]: 8,
  [AttentionState.Neutral]: 99,
};

export function needsAttention(status: AttentionState): boolean {
  return (
    status === AttentionState.Blocked ||
    status === AttentionState.Failed ||
    status === AttentionState.WaitingForInput
  );
}

export function sessionAttentionStatus(session: SessionInfo): AttentionState {
  if (session.capacityCheckPending) return AttentionState.CheckingCapacity;
  if (session.atUsageLimit) return AttentionState.Capped;
  if (effectiveLifecyclePhase(session.status, session.lifecyclePhase) === "active") {
    return (session.observedStatus as AttentionState | undefined) ?? AttentionState.Idle;
  }
  return (session.finalObservedStatus as AttentionState | undefined) ?? AttentionState.Neutral;
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
