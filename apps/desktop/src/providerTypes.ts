export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot" | "ollama";
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled" | "checking_capacity";

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
  ollamaBaseUrl: string;
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

export interface OllamaVerificationResponse {
  ok: boolean;
  normalizedBaseUrl: string;
  status: "connected" | "connected_empty" | "failed";
  reasonCode:
    | "connected"
    | "no_models_returned"
    | "all_models_filtered"
    | "invalid_url"
    | "unreachable"
    | "timeout"
    | "http_error"
    | "parse_error";
  httpStatus: number | null;
  models: string[];
  excludedModels: string[];
  diagnostic: string | null;
}
