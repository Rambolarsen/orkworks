export type ProviderId = "opencode" | "claude-code" | "codex" | "aider" | "copilot" | "ollama";
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled" | "checking_capacity";

export interface PeonSelection {
  provider: ProviderId;
  model: string;
  ollamaBaseUrl?: string;
}

export interface ProviderSettingsEntry {
  id: ProviderId;
  model: string | null;
  enabled: boolean;
  fallbackOrder: number;
  defaultState: ProviderCapacityState;
  overrideState: ProviderCapacityState | null;
}

export interface ProviderSettings {
  version: 1 | 2;
  revision: number;
  peonSelection?: PeonSelection | null;
  peonModel: string | null;
  ollamaBaseUrl: string;
  providers: ProviderSettingsEntry[];
}

export interface ProviderApplyStatus {
  appliedRevision: number | null;
  appliedAt: string | null;
  lastApplyError: string | null;
}
