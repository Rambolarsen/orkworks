export type PlanDocumentKind = "design" | "implementation";

export function validatePlanDocumentKind(kind: unknown): asserts kind is PlanDocumentKind | undefined {
  if (kind !== undefined && kind !== "design" && kind !== "implementation") throw new Error("Invalid document kind.");
}

function kindQuery(kind?: PlanDocumentKind): string {
  validatePlanDocumentKind(kind);
  return kind === undefined ? "" : `?kind=${kind}`;
}

export async function getSessionPlanContent(
  baseUrl: string,
  sessionId: string,
  token: string,
  fetchImpl: typeof fetch,
  signal: AbortSignal,
  kind?: PlanDocumentKind,
): Promise<string> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/plan-content${kindQuery(kind)}`, {
    headers: { "x-orkworks-open-plan-token": token },
    signal,
  });
  if (!response.ok) throw new Error("Couldn’t read this plan. It may have moved or is no longer available.");
  const body = await response.json() as { content?: unknown };
  if (typeof body.content !== "string") throw new Error("Couldn’t read this plan.");
  return body.content;
}

export async function requestSessionPlanReview(baseUrl: string, sessionId: string, token: string, fetchImpl: typeof fetch, signal: AbortSignal, kind?: PlanDocumentKind): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/request-plan-review${kindQuery(kind)}`, { method: "POST", headers: { "x-orkworks-open-plan-token": token }, signal });
  if (!response.ok) throw new Error("Couldn’t ask this agent to review the plan.");
}

export async function useLatestPlanDocument(baseUrl: string, sessionId: string, kind: PlanDocumentKind, token: string, fetchImpl: typeof fetch, signal: AbortSignal): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/plan-document-latest${kindQuery(kind)}`, {
    method: "POST", headers: { "x-orkworks-open-plan-token": token }, signal,
  });
  if (!response.ok) throw new Error("Couldn’t restore the latest document.");
}

export async function selectTerminalPlan(baseUrl: string, sessionId: string, printedPath: string, token: string, fetchImpl: typeof fetch, signal: AbortSignal): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/select-terminal-plan`, { method: "POST", headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token }, body: JSON.stringify({ printedPath }), signal });
  if (!response.ok) throw new Error("Couldn’t open this plan. It may have moved or be outside this repository’s worktrees.");
}
