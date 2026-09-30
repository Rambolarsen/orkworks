/** Narrow privileged transport: renderer input never selects a URL or authority. */
export async function taskmasterRequest(
  port: number, token: string, resource: "settings" | "knowledge" | "inference" | "models" | "run-status", payload?: unknown,
  fetcher: (url: string, init?: RequestInit) => Promise<Response> = fetch,
  assertCurrent: () => void = () => {},
): Promise<Record<string, unknown>> {
  assertCurrent();
  if (!Number.isInteger(port) || port < 1 || port > 65535 || !token) throw new Error("Taskmaster backend unavailable");
  const body = payload === undefined ? undefined : JSON.stringify(payload);
  const limit = resource === "knowledge" ? 2 * 1024 * 1024 : resource === "inference" ? 8 * 1024 : 64 * 1024;
  if (body && Buffer.byteLength(body) > limit) throw new Error("Taskmaster request too large");
  const path = resource === "settings" ? "/settings/taskmaster"
    : resource === "run-status" ? "/taskmaster/run-status"
      : `/settings/taskmaster/${resource}`;
  const response = await fetcher(`http://127.0.0.1:${port}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
    body, signal: AbortSignal.timeout(resource === "models" ? 45_000 : 15_000),
  });
  assertCurrent();
  const responseText = await response.text();
  assertCurrent();
  const responseLimit = resource === "knowledge" ? 2 * 1024 * 1024 : resource === "models" ? 256 * 1024 : 64 * 1024;
  if (Buffer.byteLength(responseText) > responseLimit) throw new Error("Taskmaster response too large");
  let result: unknown;
  try { result = JSON.parse(responseText); }
  catch { throw new Error("Invalid Taskmaster response"); }
  if (!result || typeof result !== "object" || Array.isArray(result)) throw new Error("Invalid Taskmaster response");
  const value = result as Record<string, unknown>;
  const error = value.error;
  const message = typeof error === "string" ? error
    : error && typeof error === "object" && !Array.isArray(error) && typeof (error as Record<string, unknown>).message === "string"
      ? (error as Record<string, string>).message
      : `Taskmaster request failed (${response.status})`;
  if (!response.ok) throw new Error(message);
  return value;
}
