import type { ProviderSettings } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { isAppliedRevisionStale } from "../providerPresentation";

interface ProviderSettingsSectionProps {
  providerSettings: ProviderSettings | null;
  providerRuntime: ProviderRuntimeResponse | null;
}

export default function ProviderSettingsSection({
  providerSettings,
  providerRuntime,
}: ProviderSettingsSectionProps) {
  if (!providerSettings) {
    return <div className="settings-section-copy">Loading provider settings…</div>;
  }

  const isStale = isAppliedRevisionStale(providerSettings, providerRuntime);

  return (
    <>
      {isStale && (
        <div className="providers-stale-banner">
          Saved settings revision {providerSettings.revision} is not yet applied to the sidecar.
        </div>
      )}
    </>
  );
}
