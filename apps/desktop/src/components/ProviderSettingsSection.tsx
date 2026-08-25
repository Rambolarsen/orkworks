import { useEffect, useState } from "react";
import type { ProviderId, ProviderSettings } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { isAppliedRevisionStale } from "../providerPresentation";

interface ProviderSettingsSectionProps {
  providerSettings: ProviderSettings | null;
  providerRuntime: ProviderRuntimeResponse | null;
  providerModels: Record<string, string[]>;
  onProviderModelChange: (providerId: ProviderId, model: string | null) => void;
}

export default function ProviderSettingsSection({
  providerSettings,
  providerRuntime,
  providerModels,
  onProviderModelChange,
}: ProviderSettingsSectionProps) {
  const [modelDrafts, setModelDrafts] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!providerSettings) return;
    setModelDrafts(Object.fromEntries(providerSettings.providers.map((entry) => [entry.id, entry.model ?? ""])));
  }, [providerSettings]);

  if (!providerSettings) {
    return <div className="settings-section-copy">Loading model provider settings...</div>;
  }

  const isStale = isAppliedRevisionStale(providerSettings, providerRuntime);
  const labels = new Map((providerRuntime?.providers ?? []).map((entry) => [entry.id, entry.label]));

  function datalistId(providerId: string): string {
    return `provider-model-suggestions-${providerId.replace(/[^a-z0-9]+/gi, "-")}`;
  }

  return (
    <>
      {isStale && (
        <div className="providers-stale-banner">
          Saved model provider settings revision {providerSettings.revision} is not yet applied to the sidecar.
        </div>
      )}
      {providerSettings.providers.map((entry) => {
        const id = datalistId(entry.id);
        const value = modelDrafts[entry.id] ?? entry.model ?? "";
        const suggestions = providerModels[entry.id] ?? [];
        return (
          <div className="provider-card" key={entry.id}>
            <div className="provider-label">{labels.get(entry.id) ?? entry.id} Peon model</div>
            <p className="settings-section-copy">Leave blank to use the default Peon model.</p>
            <input
              className="provider-model-select"
              type="text"
              list={id}
              placeholder="Use default Peon model"
              value={value}
              onChange={(event) => setModelDrafts((current) => ({ ...current, [entry.id]: event.target.value }))}
              onBlur={() => {
                const model = value.trim() || null;
                if (model !== entry.model) onProviderModelChange(entry.id, model);
              }}
            />
            <datalist id={id}>
              {[...new Set(suggestions)].sort().map((model) => <option key={model} value={model} />)}
            </datalist>
          </div>
        );
      })}
    </>
  );
}
