import type { ProviderApplyStatus, ProviderSettings } from "../src/providerTypes.ts";

export async function pushProviderSettings(
  baseUrl: string,
  settings: ProviderSettings,
  fetchImpl: typeof fetch = fetch,
): Promise<ProviderApplyStatus> {
  try {
    const response = await fetchImpl(`${baseUrl}/settings/providers`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(settings),
    });

    if (!response.ok) {
      return {
        appliedRevision: null,
        appliedAt: null,
        lastApplyError: `settings push failed: ${response.status}`,
      };
    }

    const payload = (await response.json()) as ProviderApplyStatus;
    return {
      appliedRevision: payload.appliedRevision,
      appliedAt: payload.appliedAt,
      lastApplyError: payload.lastApplyError,
    };
  } catch (error) {
    return {
      appliedRevision: null,
      appliedAt: null,
      lastApplyError: error instanceof Error ? error.message : "settings push failed",
    };
  }
}
