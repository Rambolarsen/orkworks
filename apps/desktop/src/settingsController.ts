import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes.ts";
import type { OllamaVerificationResponse, ProviderApplyStatus, ProviderSettings, RetentionApplyStatus } from "./providerTypes.ts";

export type SettingsDomain = "hotkeys" | "retention" | "debug" | "providers";

export interface SettingsControllerApi {
  getSettings: () => Promise<AppSettings>;
  saveHotkeys: (value: HotkeySettings) => Promise<SaveHotkeysResult>;
  saveRetention: (value: RetentionSettings) => Promise<{ ok: boolean; retentionApplyStatus?: RetentionApplyStatus }>;
  saveDebugSettings: (value: DebugSettings) => Promise<{ ok: true; settings: AppSettings }>;
  saveProviderSettings: (value: ProviderSettings) => Promise<{
    ok: true;
    settings: AppSettings;
    providerApplyStatus?: ProviderApplyStatus;
  }>;
  verifyOllama: (baseUrl: string) => Promise<OllamaVerificationResponse>;
}

export type SettingsCommitResult =
  | { ok: true; settings: AppSettings; providerApplyStatus?: ProviderApplyStatus; retentionApplyStatus?: RetentionApplyStatus }
  | { ok: false; failedDomain: SettingsDomain; error: unknown; settings: AppSettings };

export interface SettingsControllerSnapshot {
  committed: AppSettings | null;
  draft: AppSettings | null;
  verification: OllamaVerificationResponse | null;
  verificationError: unknown;
}

const domains: SettingsDomain[] = ["hotkeys", "retention", "debug", "providers"];

export function createSettingsController(api: SettingsControllerApi = window.orkworks, initialSettings?: AppSettings): {
  load(): Promise<AppSettings>;
  updateDraft(domain: SettingsDomain, value: unknown): void;
  discard(): void;
  verifyOllama(baseUrl: string): Promise<OllamaVerificationResponse>;
  resetHotkey(action: keyof HotkeySettings): void;
  commit(): Promise<SettingsCommitResult>;
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

  async function commit(): Promise<SettingsCommitResult> {
    if (!draft || !committed) throw new Error("Settings must be loaded before commit");
    let providerApplyStatus: ProviderApplyStatus | undefined;
    let retentionApplyStatus: RetentionApplyStatus | undefined;

    for (const domain of domains) {
      if (deepEqual(draft[domain], committed[domain])) continue;
      try {
        if (domain === "hotkeys") {
          const result = await api.saveHotkeys(draft.hotkeys);
          if (!result.ok) throw new SettingsDomainError(result.errors);
          committed = clone(result.settings);
        } else if (domain === "retention") {
          const result = await api.saveRetention(draft.retention);
          if (!result.ok) throw new SettingsDomainError(result);
          retentionApplyStatus = result.retentionApplyStatus;
          committed = { ...committed, retention: clone(draft.retention) };
        } else if (domain === "debug") {
          const result = await api.saveDebugSettings(draft.debug);
          committed = clone(result.settings);
        } else {
          const result = await api.saveProviderSettings(draft.providers);
          providerApplyStatus = result.providerApplyStatus;
          committed = clone(result.settings);
        }
        draft = { ...draft, [domain]: clone(committed[domain]) } as AppSettings;
      } catch (error) {
        return { ok: false, failedDomain: domain, error, settings: clone(committed) };
      }
    }

    return { ok: true, settings: clone(committed), providerApplyStatus, retentionApplyStatus };
  }

  return { load, updateDraft, discard, verifyOllama, resetHotkey, commit, snapshot };
}

class SettingsDomainError extends Error {
  readonly details: unknown;

  constructor(details: unknown) {
    super("Settings domain save failed");
    this.details = details;
  }
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function deepEqual(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}
