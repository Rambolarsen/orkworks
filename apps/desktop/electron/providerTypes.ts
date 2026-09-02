export type ProviderId = string;
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled" | "checking_capacity";

export interface ProviderDefinition {
  id: ProviderId;
  label: string;
  harnessId?: string;
  origin?: "builtin" | "override" | "custom" | "standalone";
}

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

export type PeonSelectionSaveResult =
  | { ok: true; settings: unknown }
  | { ok: false; error: string };

export function peonSelectionMatchesAppliedState(
  selection: PeonSelection,
  applied: PeonAppliedState,
): boolean {
  if (selection.provider !== applied.provider || selection.model !== applied.model) return false;
  return selection.provider === "ollama"
    ? selection.ollamaBaseUrl === applied.ollamaBaseUrl
    : applied.ollamaBaseUrl === null;
}

export interface ProviderSettingsEntry {
  id: ProviderId;
  harnessId?: string;
  origin?: "builtin" | "override" | "custom" | "standalone";
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
