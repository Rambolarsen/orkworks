import type { ProviderCapacityState, ProviderEffectiveState, ProviderSettings, ProviderSettingsEntry } from "./providerTypes.ts";
import type { ProviderRuntimeResponse } from "./api.ts";

export function deriveEffectiveState(
  entry: Pick<ProviderSettingsEntry, "enabled" | "defaultState" | "overrideState">,
): ProviderEffectiveState {
  if (!entry.enabled) return "disabled";
  return entry.overrideState ?? entry.defaultState;
}

export function sortProviderEntries(entries: ProviderSettingsEntry[]): ProviderSettingsEntry[] {
  return [...entries].sort((a, b) => a.fallbackOrder - b.fallbackOrder || a.id.localeCompare(b.id));
}

export function isAppliedRevisionStale(settings: ProviderSettings, runtime: ProviderRuntimeResponse | null): boolean {
  if (!runtime) return true;
  return runtime.appliedRevision !== settings.revision;
}

export interface ProviderRow {
  id: string;
  label: string;
  enabled: boolean;
  fallbackOrder: number;
  effectiveState: ProviderEffectiveState;
  defaultState: ProviderCapacityState;
  overrideState: ProviderCapacityState | null;
  lastErrorSummary: string | null;
  resetHint: string | null;
}

export interface ProviderViewModel {
  rows: ProviderRow[];
  isStale: boolean;
  summary: {
    currentProviderLabel: string | null;
  };
}

export function buildProviderViewModel(
  settings: ProviderSettings,
  runtime: ProviderRuntimeResponse | null,
  winningProviderId?: string,
): ProviderViewModel {
  const sorted = sortProviderEntries(settings.providers);
  const isStale = isAppliedRevisionStale(settings, runtime);

  const runtimeMap = new Map(
    (runtime?.providers ?? []).map((entry) => [entry.id, entry]),
  );

  const rows: ProviderRow[] = sorted.map((entry) => {
    const rt = runtimeMap.get(entry.id);
    return {
      id: entry.id,
      label: rt?.label ?? entry.id,
      enabled: entry.enabled,
      fallbackOrder: entry.fallbackOrder,
      effectiveState: deriveEffectiveState(entry),
      defaultState: entry.defaultState,
      overrideState: entry.overrideState,
      lastErrorSummary: rt?.runtime.lastErrorSummary ?? null,
      resetHint: rt?.runtime.resetHint ?? null,
    };
  });

  const currentProviderLabel = winningProviderId
    ? (runtimeMap.get(winningProviderId)?.label ?? null)
    : null;

  return {
    rows,
    isStale,
    summary: {
      currentProviderLabel,
    },
  };
}
