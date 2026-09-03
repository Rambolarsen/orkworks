import type { AppSettings, HotkeySettings } from "./appSettingsTypes.ts";
import type { OllamaVerificationResponse } from "./providerTypes.ts";
import type { ActiveHarnessIntegrationResult } from "./harnessIntegrationPresentation.ts";

export type SettingsDomain = "hotkeys" | "retention" | "debug" | "providers";

export interface SettingsControllerApi {
  getSettings: () => Promise<AppSettings>;
  verifyOllama: (baseUrl: string) => Promise<OllamaVerificationResponse>;
}

export interface SettingsControllerSnapshot {
  committed: AppSettings | null;
  draft: AppSettings | null;
  verification: OllamaVerificationResponse | null;
  verificationError: unknown;
}

export function createSettingsController(api: SettingsControllerApi = window.orkworks, initialSettings?: AppSettings): {
  load(): Promise<AppSettings>;
  updateDraft(domain: SettingsDomain, value: unknown): void;
  discard(): void;
  verifyOllama(baseUrl: string): Promise<OllamaVerificationResponse>;
  resetHotkey(action: keyof HotkeySettings): void;
  snapshot(): SettingsControllerSnapshot;
} {
  let committed: AppSettings | null = initialSettings ? clone(initialSettings) : null;
  let draft: AppSettings | null = initialSettings ? clone(initialSettings) : null;
  let verification: OllamaVerificationResponse | null = null;
  let verificationError: unknown = null;
  let verificationGeneration = 0;

  function snapshot(): SettingsControllerSnapshot {
    return { committed: committed && clone(committed), draft: draft && clone(draft), verification, verificationError };
  }

  async function load(): Promise<AppSettings> {
    const loaded = await api.getSettings();
    committed = clone(loaded);
    draft = clone(loaded);
    verification = null;
    verificationError = null;
    return clone(loaded);
  }

  function updateDraft(domain: SettingsDomain, value: unknown): void {
    if (!draft) return;
    draft = { ...draft, [domain]: clone(value) } as AppSettings;
  }

  function discard(): void {
    if (!committed) return;
    draft = clone(committed);
    verification = null;
    verificationError = null;
  }

  async function verifyOllama(baseUrl: string): Promise<OllamaVerificationResponse> {
    const generation = ++verificationGeneration;
    try {
      const result = await api.verifyOllama(baseUrl);
      if (generation === verificationGeneration) {
        verification = result;
        verificationError = null;
      }
      return result;
    } catch (error) {
      if (generation === verificationGeneration) verificationError = error;
      throw error;
    }
  }

  function resetHotkey(action: keyof HotkeySettings): void {
    if (!draft) return;
    draft = { ...draft, hotkeys: { ...draft.hotkeys, [action]: draft.defaultHotkeys[action] } };
  }

  return { load, updateDraft, discard, verifyOllama, resetHotkey, snapshot };
}

export function mergeIntegrationOperationFailures(
  current: Record<string, ActiveHarnessIntegrationResult>,
  results: Record<string, ActiveHarnessIntegrationResult>,
): Record<string, ActiveHarnessIntegrationResult> {
  const next = { ...current };
  for (const [resultKey, result] of Object.entries(results)) {
    // Keep accepting the pre-grouped shape here while older renderer callers
    // drain during the IPC contract rollout. New grouped results always carry
    // consumerHarnessIds, so one result is still projected to every row.
    const consumerHarnessIds = result.consumerHarnessIds ?? [resultKey];
    for (const harnessId of consumerHarnessIds) {
      if (result.outcome === "failed") {
        next[harnessId] = result;
        continue;
      }
      if (clearsIntegrationOperationFailure(result)) {
        delete next[harnessId];
      }
    }
  }
  return next;
}

function clearsIntegrationOperationFailure(result: ActiveHarnessIntegrationResult): boolean {
  // "unsupported" also clears: if the tool fell out of eligibility, the last
  // operation's "action required" failure is no longer actionable.
  return result.outcome === "succeeded" || result.outcome === "unsupported";
}

function clone<T>(value: T): T {
  return structuredClone(value);
}
