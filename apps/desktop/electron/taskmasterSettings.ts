/** Narrow privileged transport: renderer input never selects a URL or authority. */
export async function taskmasterRequest(
  port: number, token: string, resource: "settings" | "knowledge", payload?: unknown,
  fetcher: (url: string, init?: RequestInit) => Promise<Response> = fetch,
): Promise<Record<string, unknown>> {
  if (!Number.isInteger(port) || port < 1 || port > 65535 || !token) throw new Error("Taskmaster backend unavailable");
  const body = payload === undefined ? undefined : JSON.stringify(payload);
  if (body && Buffer.byteLength(body) > (resource === "knowledge" ? 2 * 1024 * 1024 : 64 * 1024)) throw new Error("Taskmaster request too large");
  const response = await fetcher(`http://127.0.0.1:${port}/settings/taskmaster${resource === "knowledge" ? "/knowledge" : ""}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
    body, signal: AbortSignal.timeout(15_000),
  });
  const result = await response.json() as Record<string, unknown>;
  if (!response.ok) throw new Error(typeof result.error === "string" ? result.error : `Taskmaster request failed (${response.status})`);
  return result;
}
