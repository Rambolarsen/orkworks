export type BackendHealthResult = "connected" | "unreachable";

export interface BackendHealthProbeDeps {
  getBackendUrl: () => Promise<string>;
  fetch: (url: string) => Promise<{ ok: boolean }>;
  delay: (ms: number) => Promise<void>;
  urlAttempts?: number;
  fetchAttempts?: number;
}

const RETRY_DELAY_MS = 500;
const DEFAULT_URL_ATTEMPTS = 30;
const DEFAULT_FETCH_ATTEMPTS = 30;

export async function probeBackendHealth(deps: BackendHealthProbeDeps): Promise<BackendHealthResult> {
  const urlAttempts = deps.urlAttempts ?? DEFAULT_URL_ATTEMPTS;
  const fetchAttempts = deps.fetchAttempts ?? DEFAULT_FETCH_ATTEMPTS;

  let baseUrl: string | null = null;
  for (let i = 0; i < urlAttempts; i++) {
    try {
      baseUrl = await deps.getBackendUrl();
      break;
    } catch {
      await deps.delay(RETRY_DELAY_MS);
    }
  }
  if (baseUrl === null) return "unreachable";

  for (let i = 0; i < fetchAttempts; i++) {
    try {
      const resp = await deps.fetch(`${baseUrl}/health`);
      if (resp.ok) return "connected";
    } catch {
      // endpoint not up yet — keep polling
    }
    if (i < fetchAttempts - 1) await deps.delay(RETRY_DELAY_MS);
  }
  return "unreachable";
}
