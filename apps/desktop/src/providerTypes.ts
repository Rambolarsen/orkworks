export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot";
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled";

export interface ProviderSettingsEntry {
  id: ProviderId;
  enabled: boolean;
  fallbackOrder: number;
  defaultState: ProviderCapacityState;
  overrideState: ProviderCapacityState | null;
}

export interface ProviderSettings {
  version: 1;
  revision: number;
  peonModel: string | null;
  providers: ProviderSettingsEntry[];
}

export interface ProviderApplyStatus {
  appliedRevision: number | null;
  appliedAt: string | null;
  lastApplyError: string | null;
}

export interface ProviderModelsResponse {
  models: string[];
}

export interface ProviderLabelsResponse {
  labels: Record<string, string>;
}
