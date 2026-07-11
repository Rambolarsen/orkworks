export function parseSessionStateInjectionPayload(payload: unknown): {
  sessionId: string;
  injectionId: string;
} {
  if (!payload || typeof payload !== "object") {
    throw new Error("invalid state injection payload");
  }

  const { sessionId, injectionId } = payload as Record<string, unknown>;
  if (
    typeof sessionId !== "string" ||
    !sessionId ||
    typeof injectionId !== "string" ||
    !injectionId
  ) {
    throw new Error("invalid state injection payload");
  }

  return { sessionId, injectionId };
}

export function debugInjectionUrl(port: number, sessionId: string): string {
  return `http://127.0.0.1:${port}/sessions/${encodeURIComponent(sessionId)}/debug-injection`;
}
