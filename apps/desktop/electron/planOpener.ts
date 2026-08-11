export async function getSessionPlanContent(
  baseUrl: string,
  sessionId: string,
  token: string,
  fetchImpl: typeof fetch,
): Promise<string> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/plan-content`, {
    headers: { "x-orkworks-open-plan-token": token },
  });
  if (!response.ok) throw new Error("Couldn’t read this plan. It may have moved or is no longer available.");
  const body = await response.json() as { content?: unknown };
  if (typeof body.content !== "string") throw new Error("Couldn’t read this plan.");
  return body.content;
}

export async function requestSessionPlanReview(baseUrl: string, sessionId: string, token: string, fetchImpl: typeof fetch): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/request-plan-review`, { method: "POST", headers: { "x-orkworks-open-plan-token": token } });
  if (!response.ok) throw new Error("Couldn’t ask this agent to review the plan.");
}

export async function selectTerminalPlan(baseUrl: string, sessionId: string, printedPath: string, token: string, fetchImpl: typeof fetch): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/select-terminal-plan`, { method: "POST", headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token }, body: JSON.stringify({ printedPath }) });
  if (!response.ok) throw new Error("Couldn’t open this plan. It may have moved or be outside this repository’s worktrees.");
}
