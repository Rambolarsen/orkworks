export type ProviderId = string;
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

export type PeonSelectionSaveResult =
  | { ok: true; settings: import("./appSettingsTypes").AppSettings }
  | { ok: false; error: string };

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

export interface ProviderDefinition {
  id: ProviderId;
  label: string;
  harnessId?: string;
  origin?: "builtin" | "override" | "custom" | "standalone";
}

export function normalizeProviderSettings(
  settings: ProviderSettings,
  definitions: readonly ProviderDefinition[],
): ProviderSettings {
  const definitionsById = new Map(definitions.map((definition) => [definition.id, definition]));
  const defaults = new Map<string, ProviderSettingsEntry>([
    ["opencode", { id: "opencode", model: null, enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null }],
    ["claude-code", { id: "claude-code", model: null, enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null }],
    ["codex", { id: "codex", model: null, enabled: true, fallbackOrder: 2, defaultState: "unknown", overrideState: null }],
    ["aider", { id: "aider", model: null, enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null }],
    ["copilot", { id: "copilot", model: null, enabled: true, fallbackOrder: 4, defaultState: "unknown", overrideState: null }],
    ["ollama", { id: "ollama", model: null, enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null }],
  ]);
  const entries = new Map<string, ProviderSettingsEntry>();

  for (const entry of settings.providers) {
    if (!definitionsById.has(entry.id) || entries.has(entry.id)) continue;
    entries.set(entry.id, { ...entry });
  }

  let nextFallbackOrder = Math.max(-1, ...Array.from(entries.values()).map((entry) => entry.fallbackOrder)) + 1;
  for (const definition of definitions) {
    if (entries.has(definition.id)) continue;
    const defaultEntry = defaults.get(definition.id);
    entries.set(definition.id, {
      id: definition.id,
      ...(defaultEntry === undefined && definition.harnessId ? { harnessId: definition.harnessId } : {}),
      ...(defaultEntry === undefined && definition.origin ? { origin: definition.origin } : {}),
      model: null,
      enabled: defaultEntry?.enabled ?? true,
      fallbackOrder: defaultEntry?.fallbackOrder ?? nextFallbackOrder++,
      defaultState: defaultEntry?.defaultState ?? "unknown",
      overrideState: null,
    });
  }

  const providers = Array.from(entries.values())
    .sort((left, right) => left.fallbackOrder - right.fallbackOrder || left.id.localeCompare(right.id))
    .map((entry, index) => ({ ...entry, fallbackOrder: index }));
  const peonSelection = settings.peonSelection && entries.has(settings.peonSelection.provider)
    ? { ...settings.peonSelection }
    : null;
  return { ...settings, peonSelection, providers };
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
