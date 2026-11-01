export type PlanDocumentKind = "design" | "implementation";

export function validatePlanDocumentKind(kind: unknown): asserts kind is PlanDocumentKind | undefined {
  if (kind !== undefined && kind !== "design" && kind !== "implementation") throw new Error("Invalid document kind.");
}

export function validatePlanSelectionId(value: unknown): asserts value is string | undefined {
  if (value !== undefined && (typeof value !== "string" || !/^[a-f0-9]{64}$/.test(value))) throw new Error("Invalid document selection.");
}

function kindQuery(kind?: PlanDocumentKind, selectionId?: string): string {
  validatePlanDocumentKind(kind);
  validatePlanSelectionId(selectionId);
  if (selectionId !== undefined && kind === undefined) throw new Error("A document kind is required.");
  return kind === undefined ? "" : `?kind=${kind}${selectionId === undefined ? "" : `&selectionId=${selectionId}`}`;
}

export async function getSessionPlanContent(
  baseUrl: string,
  sessionId: string,
  token: string,
  fetchImpl: typeof fetch,
  signal: AbortSignal,
  kind?: PlanDocumentKind,
  selectionId?: string,
): Promise<string> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/plan-content${kindQuery(kind, selectionId)}`, {
    headers: { "x-orkworks-open-plan-token": token },
    signal,
  });
  if (!response.ok) throw new Error("Couldn’t read this plan. It may have moved or is no longer available.");
  const body = await response.json() as { content?: unknown };
  if (typeof body.content !== "string") throw new Error("Couldn’t read this plan.");
  return body.content;
}

export async function requestSessionPlanReview(baseUrl: string, sessionId: string, token: string, fetchImpl: typeof fetch, signal: AbortSignal, kind?: PlanDocumentKind, selectionId?: string): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/request-plan-review${kindQuery(kind, selectionId)}`, { method: "POST", headers: { "x-orkworks-open-plan-token": token }, signal });
  if (!response.ok) throw new Error("Couldn’t ask this agent to review the plan.");
}

export async function useLatestPlanDocument(baseUrl: string, sessionId: string, kind: PlanDocumentKind, token: string, fetchImpl: typeof fetch, signal: AbortSignal, selectionId?: string): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/plan-document-latest${kindQuery(kind, selectionId)}`, {
    method: "POST", headers: { "x-orkworks-open-plan-token": token }, signal,
  });
  if (!response.ok) throw new Error("Couldn’t restore the latest document.");
}

export async function selectTerminalPlan(baseUrl: string, sessionId: string, printedPath: string, token: string, fetchImpl: typeof fetch, signal: AbortSignal): Promise<{kind: PlanDocumentKind; selectionId: string} | undefined> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/select-terminal-plan`, { method: "POST", headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token }, body: JSON.stringify({ printedPath }), signal });
  if (!response.ok) throw new Error("Couldn’t open this plan. It may have moved or be outside this repository’s worktrees.");
  if (response.status === 204) return;
  const body = await response.json() as {kind?: unknown; selectionId?: unknown};
  if (body.kind == null && body.selectionId == null) return;
  validatePlanDocumentKind(body.kind);
  validatePlanSelectionId(body.selectionId);
  if (body.kind === undefined || body.selectionId === undefined) throw new Error("Invalid document selection.");
  return {kind: body.kind, selectionId: body.selectionId};
}
