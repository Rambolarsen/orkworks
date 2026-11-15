// Renderer credential for the local sidecar API (ADR 0116). Electron main
// creates the token and passes it to the sidecar; the renderer reads it over
// IPC and never receives the privileged desktop token.

const TOKEN_HEADER = "x-orkworks-renderer-token";
const SOCKET_PROTOCOL = "orkworks.v1";
const SOCKET_TOKEN_PREFIX = "orkworks.token.";

let cachedToken = "";

function backendToken(): string {
  if (cachedToken) return cachedToken;
  if (typeof window === "undefined") return "";
  const token = window.orkworks?.getBackendToken?.();
  if (typeof token === "string") cachedToken = token;
  return cachedToken;
}

/** `fetch` for sidecar URLs. Adds the renderer credential when one exists. */
export function backendFetch(input: string, init?: RequestInit): Promise<Response> {
  const token = backendToken();
  if (!token) return init === undefined ? fetch(input) : fetch(input, init);
  const headers = new Headers(init?.headers);
  headers.set(TOKEN_HEADER, token);
  return fetch(input, { ...init, headers });
}

/**
 * Opens a sidecar WebSocket. Browsers cannot set handshake headers, so the
 * credential travels as an offered subprotocol and the sidecar selects
 * `orkworks.v1`.
 */
export function openBackendSocket(url: string): WebSocket {
  const token = backendToken();
  return token ? new WebSocket(url, [SOCKET_PROTOCOL, SOCKET_TOKEN_PREFIX + token]) : new WebSocket(url);
}
