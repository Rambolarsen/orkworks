export type ProviderId = "opencode" | "claude-code" | "codex" | "aider" | "copilot" | "ollama";
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled" | "checking_capacity";

export interface PeonSelection {
  provider: ProviderId;
  model: string;
  ollamaBaseUrl?: string;
}

export interface PeonProviderVerificationResponse {
  ok: boolean;
  provider: ProviderId;
  capabilities: {
    connectivity: boolean;
    modelDiscovery: boolean;
    providerDefault: boolean;
    testInference: boolean;
  };
  models: string[];
  ollamaBaseUrl: string | null;
  generation: number;
}

export interface PeonAppliedState {
  provider: string | null;
  model: string | null;
  ollamaBaseUrl: string | null;
  appliedAt: string | null;
  connectionRevision: number;
}

export interface PeonSelectionSaveResult {
  ok: true;
  settings: import("./appSettingsTypes").AppSettings;
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

export type RetentionApplyStatus = ProviderApplyStatus;

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
