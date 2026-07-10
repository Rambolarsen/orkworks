import type { SessionInfo } from "./api";

export interface SessionStateInjectionOption {
  id: string;
  label: string;
}

export function replaceSessionAfterInjection(
  sessions: SessionInfo[],
  injected: SessionInfo,
): SessionInfo[] {
  return sessions.map((session) => (session.id === injected.id ? injected : session));
}
