import type { SessionInfo, SessionLifecycle } from "./api";

export function renderTerminalPresentation<T>(
  lifecycle: SessionLifecycle | undefined,
  interactive: () => T,
  historical: () => T,
): T {
  return lifecycle === "dead" ? historical() : interactive();
}

/** The interval between a session being created and the harness actually spawning (issue #302). */
export function isSessionStarting(session: Pick<SessionInfo, "status">): boolean {
  return session.status === "creating";
}

export interface TerminalInteractivity {
  disableStdin: boolean;
  cursorBlink: boolean;
}

/** Keystrokes and the blinking cursor read as "ready" — suppress both until the harness process has actually spawned, or once the session has ended. */
export function computeTerminalInteractivity({
  starting,
  ended,
}: {
  starting: boolean;
  ended: boolean;
}): TerminalInteractivity {
  const disableStdin = starting || ended;
  return { disableStdin, cursorBlink: !disableStdin };
}
