import { peonErrorFromBody } from "./peonSelectionTransaction.ts";

export type SidecarFetchImpl = (input: string, init?: RequestInit) => Promise<Response>;

export function sidecarUrl(port: number, path?: string): string {
  const suffix = path ? "/" + path : "";
  return `http://127.0.0.1:${port}${suffix}`;
}

export type SidecarSendOptions = {
  method?: RequestInit["method"];
  jsonBody?: unknown;
  token?: string;
  signal?: AbortSignal;
  timeoutMs?: number;
  fetchImpl?: SidecarFetchImpl;
};

export function errorFromBody(body: unknown, fallback: string): string {
  return body && typeof body === "object" && typeof (body as { error?: unknown }).error === "string"
    ? (body as { error: string }).error
    : fallback;
}

export async function parseErrorBody(response: Response, fallback: string): Promise<string> {
  const body: unknown = await response.json().catch(() => ({}));
  return errorFromBody(body, fallback);
}

export async function parsePeonError(response: Response, fallback: string): Promise<Error> {
  const body: unknown = await response.json().catch(() => ({ error: undefined }));
  return peonErrorFromBody(body, fallback);
}

export function sidecarSend(port: number, path: string, options: SidecarSendOptions = {}): Promise<Response> {
  const { method, jsonBody, token, signal, timeoutMs, fetchImpl = fetch } = options;
  let composedSignal = signal;
  if (timeoutMs !== undefined) {
    const timeoutSignal = AbortSignal.timeout(timeoutMs);
    composedSignal = signal ? AbortSignal.any([signal, timeoutSignal]) : timeoutSignal;
  }
  const headers: Record<string, string> = {};
  if (jsonBody !== undefined) headers["Content-Type"] = "application/json";
  if (token !== undefined) headers["x-orkworks-open-plan-token"] = token;
  return fetchImpl(sidecarUrl(port, path), {
    ...(method ? { method } : {}),
    ...(jsonBody !== undefined ? { body: JSON.stringify(jsonBody) } : {}),
    ...(Object.keys(headers).length > 0 ? { headers } : {}),
    ...(composedSignal ? { signal: composedSignal } : {}),
  });
}
