import type { SessionInfo } from "./api.ts";

type SessionRuntimeView = Pick<SessionInfo, "lifecycle" | "sessionState">;

/** Archived history stays read-only even when producer lifecycle data regresses. */
export function isHistoricalSession(session: SessionRuntimeView): boolean {
  return session.sessionState === "archived" || session.lifecycle === "dead";
}

/** Runtime commands keep their existing alive-only restriction. */
export function isSessionAlive(session: SessionInfo | null | undefined): session is SessionInfo {
  return !!session && session.lifecycle === "alive" && !isHistoricalSession(session);
}
