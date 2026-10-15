export type SidecarFetchImpl = (input: string, init?: RequestInit) => Promise<Response>;

import { peonErrorFromBody } from "./peonSelectionTransaction.ts";

export function sidecarUrl(port: number, path?: string): string {
  return `http://127.0.0.1:${port}${path ? `/${path}` : ""}`;
}

export type SidecarSendOptions = {
  method?: RequestInit["method"];
  jsonBody?: unknown;
  token?: string;
  signal?: AbortSignal;
  timeoutMs?: number;
  fetchImpl?: SidecarFetchImpl;
};

function errorFromBody(body: unknown, fallback: string): string {
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

export { errorFromBody };

export function sidecarSend(port: number, path: string, options: SidecarSendOptions = {}): Promise<Response> {
  const { method, jsonBody, token, signal, timeoutMs, fetchImpl = fetch } = options;
  const headers: Record<string, string> = {};
  if (jsonBody !== undefined) headers["Content-Type"] = "application/json";
  if (token !== undefined) headers["x-orkworks-open-plan-token"] = token;
  const composedSignal = signal && timeoutMs
    ? AbortSignal.any([signal, AbortSignal.timeout(timeoutMs)])
    : signal ?? (timeoutMs ? AbortSignal.timeout(timeoutMs) : null);
  return fetchImpl(sidecarUrl(port, path), {
    ...(method ? { method } : {}),
    ...(jsonBody !== undefined ? { body: JSON.stringify(jsonBody) } : {}),
    ...(Object.keys(headers).length > 0 ? { headers } : {}),
    ...(composedSignal ? { signal: composedSignal } : {}),
  });
}
