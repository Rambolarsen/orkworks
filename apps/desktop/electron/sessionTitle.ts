export type SessionTitleRequest = {
  sessionId: string;
  workspaceIdentity: string;
} & ({ action: "read" | "automatic"; title?: never } | { action: "set"; title: string });
export interface SessionTitleResult { title: string; automatic: boolean }

export function validateSessionTitleRequest(input: unknown): asserts input is SessionTitleRequest {
  if (!input || typeof input !== "object") throw new Error("Invalid title request.");
  const value = input as Record<string, unknown>;
  const control = /[\u0000-\u001f\u007f-\u009f]/;
  if (typeof value.sessionId !== "string" || !/^[a-zA-Z0-9_-]{1,256}$/.test(value.sessionId)
    || typeof value.workspaceIdentity !== "string" || !value.workspaceIdentity || value.workspaceIdentity.length > 4096
    || control.test(value.workspaceIdentity)) throw new Error("Invalid title request.");
  if ((value.action === "read" || value.action === "automatic") && value.title === undefined) return;
  if (value.action === "set" && typeof value.title === "string" && value.title.trim()
    && Array.from(value.title.trim()).length <= 100 && !control.test(value.title)) return;
  throw new Error("Invalid title request.");
}

export async function requestSessionTitle(baseUrl: string, input: unknown, token: string, fetchImpl: typeof fetch, signal: AbortSignal): Promise<SessionTitleResult> {
  validateSessionTitleRequest(input);
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(input.sessionId)}/title`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
    body: JSON.stringify({ workspaceIdentity: input.workspaceIdentity, action: input.action, ...(input.action === "set" ? { title: input.title } : {}) }),
    signal,
  });
  if (!response.ok) throw new Error((await response.text()).trim() || "Could not update session title.");
  const result: unknown = await response.json();
  if (!result || typeof result !== "object" || typeof (result as SessionTitleResult).title !== "string"
    || typeof (result as SessionTitleResult).automatic !== "boolean") throw new Error("Invalid session title response.");
  return result as SessionTitleResult;
}
