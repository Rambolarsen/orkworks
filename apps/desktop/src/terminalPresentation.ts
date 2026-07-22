import type { SessionLifecycle } from "./api";

export function renderTerminalPresentation<T>(
  lifecycle: SessionLifecycle | undefined,
  interactive: () => T,
  historical: () => T,
): T {
  return lifecycle === "dead" ? historical() : interactive();
}
