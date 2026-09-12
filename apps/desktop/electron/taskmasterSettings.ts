/** Narrow privileged transport: renderer input never selects a URL or authority. */
export async function taskmasterRequest(
  port: number, token: string, resource: "settings" | "knowledge" | "inference", payload?: unknown,
  fetcher: (url: string, init?: RequestInit) => Promise<Response> = fetch,
  assertCurrent: () => void = () => {},
): Promise<Record<string, unknown>> {
  assertCurrent();
  if (!Number.isInteger(port) || port < 1 || port > 65535 || !token) throw new Error("Taskmaster backend unavailable");
  const body = payload === undefined ? undefined : JSON.stringify(payload);
  const limit = resource === "knowledge" ? 2 * 1024 * 1024 : resource === "inference" ? 8 * 1024 : 64 * 1024;
  if (body && Buffer.byteLength(body) > limit) throw new Error("Taskmaster request too large");
  const response = await fetcher(`http://127.0.0.1:${port}/settings/taskmaster${resource === "settings" ? "" : `/${resource}`}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
    body, signal: AbortSignal.timeout(15_000),
  });
  assertCurrent();
  const result: unknown = await response.json();
  assertCurrent();
  if (!result || typeof result !== "object" || Array.isArray(result)) throw new Error("Invalid Taskmaster response");
  const value = result as Record<string, unknown>;
  if (!response.ok) throw new Error(typeof value.error === "string" ? value.error : `Taskmaster request failed (${response.status})`);
  return value;
}
