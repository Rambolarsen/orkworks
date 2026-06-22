import { useState } from "react";
import type { ProviderSettings } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { buildProviderViewModel } from "../providerPresentation";
import EmptyState from "./EmptyState";

interface CapacityPanelProps {
  providerSettings: ProviderSettings | null;
  providerRuntime: ProviderRuntimeResponse | null;
  onSaveProviderSettings: (providers: ProviderSettings) => Promise<void>;
}

function CapacityPanel({ providerSettings, providerRuntime, onSaveProviderSettings }: CapacityPanelProps) {
  const [saving, setSaving] = useState(false);

  if (!providerSettings) {
    return <EmptyState message="Loading providers…" />;
  }

  const ps: ProviderSettings = providerSettings;
  const viewModel = buildProviderViewModel(ps, providerRuntime);

  async function onMove(id: string, direction: "up" | "down") {
    const sorted = [...viewModel.rows];
    const idx = sorted.findIndex((r) => r.id === id);
    if (idx === -1) return;
    const swapIdx = direction === "up" ? idx - 1 : idx + 1;
    if (swapIdx < 0 || swapIdx >= sorted.length) return;

    const next = ps.providers.map((entry) => {
      if (entry.id === sorted[idx].id) return { ...entry, fallbackOrder: sorted[swapIdx].fallbackOrder };
      if (entry.id === sorted[swapIdx].id) return { ...entry, fallbackOrder: sorted[idx].fallbackOrder };
      return entry;
    });

    setSaving(true);
    try {
      await onSaveProviderSettings({ ...ps, providers: next });
    } finally {
      setSaving(false);
    }
  }

  async function onClearOverride(id: string) {
    const next = ps.providers.map((entry) =>
      entry.id === id ? { ...entry, overrideState: null } : entry,
    );
    setSaving(true);
    try {
      await onSaveProviderSettings({ ...ps, providers: next });
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="providers-panel">
      <header className="providers-summary">
        <div>
          <h2>Providers</h2>
          <p>App-wide defaults and overrides for Peon fallback.</p>
        </div>
        <div className={`providers-health providers-health--${viewModel.summary.overallStatus}`}>
          {viewModel.summary.overallStatus}
        </div>
      </header>

      {viewModel.isStale && (
        <div className="providers-stale-banner">
          Saved settings revision {providerSettings.revision} is not yet applied to the sidecar.
        </div>
      )}

      {viewModel.rows.map((row) => (
        <article key={row.id} className="provider-card">
          <header>
            <h3>{row.label}</h3>
            <span>Step {row.fallbackOrder + 1}</span>
          </header>
          <div>Default: {row.defaultState}</div>
          <div>Override: {row.overrideState ?? "none"}</div>
          <div>Effective: {row.effectiveState}</div>
          <div>Model: {row.peonModel ?? "default"}</div>
          <div>Last error: {row.lastErrorSummary ?? "none"}</div>
          <div>Reset hint: {row.resetHint ?? "none"}</div>
          <button type="button" onClick={() => onMove(row.id, "up")} disabled={saving || row.fallbackOrder === 0}>
            Move up
          </button>
          <button type="button" onClick={() => onMove(row.id, "down")} disabled={saving || row.fallbackOrder === viewModel.rows.length - 1}>
            Move down
          </button>
          <button type="button" onClick={() => onClearOverride(row.id)} disabled={saving || !row.overrideState}>
            Clear override
          </button>
        </article>
      ))}
    </section>
  );
}

export default CapacityPanel;
