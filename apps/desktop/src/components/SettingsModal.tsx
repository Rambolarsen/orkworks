import { useEffect, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "../appSettingsTypes";
import type { ProviderSettings, ProviderSettingsEntry, ProviderModelsResponse } from "../providerTypes";

type HotkeyAction = keyof HotkeySettings;

interface SettingsModalProps {
  initialSettings: AppSettings;
  onClose: () => void;
  onSaved: (settings: AppSettings) => void;
}

const hotkeyRows: Array<{ action: HotkeyAction; label: string; optional?: boolean }> = [
  { action: "newSession", label: "New Session" },
  { action: "toggleSessionsPanel", label: "Sessions Panel" },
  { action: "toggleDetailPanel", label: "Detail Panel" },
  { action: "toggleTerminalPanel", label: "Terminal Panel" },
  { action: "toggleCapacityPanel", label: "Capacity Panel" },
  { action: "toggleRecommendationsPanel", label: "Recommendations Panel" },
  { action: "resetLayout", label: "Reset Layout", optional: true },
];

export default function SettingsModal({ initialSettings, onClose, onSaved }: SettingsModalProps) {
  const defaultHotkeys = initialSettings.defaultHotkeys;
  const [draft, setDraft] = useState<HotkeySettings>(initialSettings.hotkeys);
  const [capturing, setCapturing] = useState<HotkeyAction | null>(null);
  const [errors, setErrors] = useState<Partial<Record<HotkeyAction, string[]>>>({});
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [retention, setRetention] = useState<RetentionSettings>(initialSettings.retention);
  const [retentionSaveStatus, setRetentionSaveStatus] = useState<string | null>(null);
  const [providerDraft, setProviderDraft] = useState<ProviderSettings>(initialSettings.providers);
  const [providerModels, setProviderModels] = useState<Record<string, string[]>>({});
  const [providerSaveStatus, setProviderSaveStatus] = useState<string | null>(null);

  useEffect(() => {
    if (!capturing) return;

    window.orkworks.setHotkeyCaptureActive(true);
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setCapturing(null);
        return;
      }
      if ((event.key === "Backspace" || event.key === "Delete") && isBareKey(event)) {
        const row = hotkeyRows.find((item) => item.action === capturing);
        if (row?.optional) {
          setDraft((current) => ({ ...current, [capturing]: null }));
          setCapturing(null);
        }
        return;
      }

      const accelerator = acceleratorFromKeyboardEvent(event);
      if (accelerator) {
        setDraft((current) => ({ ...current, [capturing]: accelerator }));
        setCapturing(null);
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.orkworks.setHotkeyCaptureActive(false);
    };
  }, [capturing]);

  useEffect(() => {
    const ids = providerDraft.providers.map((p) => p.id);
    async function load() {
      const map: Record<string, string[]> = {};
      for (const id of ids) {
        try {
          const resp: ProviderModelsResponse = await window.orkworks.getProviderModels(id);
          map[id] = resp.models;
        } catch {
          map[id] = [];
        }
      }
      setProviderModels(map);
    }
    load();
  }, []);

  async function saveRetention(rt: RetentionSettings) {
    setRetentionSaveStatus(null);
    try {
      await window.orkworks.saveRetention(rt);
      setRetentionSaveStatus("Saved");
    } catch {
      setRetentionSaveStatus("Couldn't save retention settings.");
    }
  }

  async function save() {
    setSaving(true);
    setErrors({});
    setSaveError(null);
    try {
      const result: SaveHotkeysResult = await window.orkworks.saveHotkeys(draft);
      if (result.ok) {
        onSaved(result.settings);
        onClose();
      } else {
        setErrors(result.errors);
      }
    } catch {
      setSaveError("Settings could not be saved. The active shortcuts were not changed.");
    } finally {
      setSaving(false);
    }
  }

  async function saveProviderDraft(entry: ProviderSettingsEntry) {
    setProviderSaveStatus(null);
    const next = {
      ...providerDraft,
      providers: providerDraft.providers.map((p) =>
        p.id === entry.id ? entry : p,
      ),
    };
    setProviderDraft(next);
    try {
      await window.orkworks.saveProviderSettings(next);
      setProviderSaveStatus("Saved");
    } catch {
      setProviderSaveStatus("Couldn't save provider settings.");
    }
  }

  return (
    <div className="settings-backdrop" role="presentation">
      <section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <header className="settings-modal-header">
          <div>
            <h2 id="settings-title">Settings</h2>
            <p>Configure OrkWorks desktop preferences.</p>
          </div>
          <button className="settings-icon-button" type="button" onClick={onClose} aria-label="Close settings">
            Close
          </button>
        </header>

        <div className="settings-section">
          <h3>Providers</h3>
          <p className="settings-section-copy">
            Choose which model peon uses for each provider. Changes apply immediately.
          </p>

          <div className="provider-list">
            {[...providerDraft.providers]
              .sort((a, b) => a.fallbackOrder - b.fallbackOrder)
              .map((entry) => (
                <div className="provider-card" key={entry.id}>
                  <div className="provider-label">{entry.id === "opencode" ? "OpenCode" : entry.id === "claude-code" ? "Claude Code" : entry.id}</div>
                  <select
                    className="provider-model-select"
                    value={entry.peonModel ?? ""}
                    onChange={(e) => {
                      const val = e.target.value;
                      saveProviderDraft({ ...entry, peonModel: val || null });
                    }}
                  >
                    <option value="">default</option>
                    {(providerModels[entry.id] ?? []).map((m) => (
                      <option key={m} value={m}>{m}</option>
                    ))}
                  </select>
                </div>
              ))}
          </div>

          {providerSaveStatus && (
            <div className={`retention-status ${providerSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
              {providerSaveStatus}
            </div>
          )}
        </div>

        <div className="settings-section">
          <h3>Hotkeys</h3>
          <p className="settings-section-copy">Changes apply after Save and update the native Electron menu.</p>

          <div className="hotkey-list">
            {hotkeyRows.map((row) => (
              <div className={`hotkey-row ${capturing === row.action ? "hotkey-row--capturing" : ""}`} key={row.action}>
                <div>
                  <div className="hotkey-label">{row.label}</div>
                  {errors[row.action]?.map((error) => (
                    <div className="hotkey-error" key={error}>{error}</div>
                  ))}
                </div>
                <kbd className="hotkey-value">
                  {capturing === row.action ? "Press shortcut..." : draft[row.action] ?? "Unset"}
                </kbd>
                <button type="button" onClick={() => setCapturing(row.action)}>Edit</button>
                <button
                  type="button"
                  onClick={() => setDraft((current) => ({ ...current, [row.action]: defaultHotkeys[row.action] }))}
                >
                  Reset
                </button>
              </div>
            ))}
          </div>
          {saveError && <div className="settings-save-error">{saveError}</div>}
        </div>

        <div className="settings-section">
          <h3>Session Retention</h3>
          <p className="settings-section-copy">
            Live sessions are never auto-deleted. Changes take effect within 5 minutes.
          </p>

          <div className="retention-list">
            <div className="retention-row">
              <div className="retention-label">Max sessions to keep</div>
              <input
                className="retention-input"
                type="number"
                min={0}
                max={999}
                value={retention.maxSessions}
                onChange={(e) => {
                  const v = parseInt(e.target.value, 10);
                  if (!Number.isNaN(v)) {
                    setRetention((r) => ({ ...r, maxSessions: Math.max(0, Math.min(999, v)) }));
                  }
                }}
                onBlur={(e) => {
                  const v = parseInt(e.target.value, 10);
                  saveRetention({ ...retention, maxSessions: Number.isNaN(v) ? 0 : Math.max(0, Math.min(999, v)) });
                }}
              />
              <span className="retention-hint">0 = unlimited</span>
            </div>

            <div className="retention-row">
              <div className="retention-label">Auto-delete sessions older than (days)</div>
              <input
                className="retention-input"
                type="number"
                min={0}
                max={999}
                value={retention.maxAgeDays}
                onChange={(e) => {
                  const v = parseInt(e.target.value, 10);
                  if (!Number.isNaN(v)) {
                    setRetention((r) => ({ ...r, maxAgeDays: Math.max(0, Math.min(999, v)) }));
                  }
                }}
                onBlur={(e) => {
                  const v = parseInt(e.target.value, 10);
                  saveRetention({ ...retention, maxAgeDays: Number.isNaN(v) ? 0 : Math.max(0, Math.min(999, v)) });
                }}
              />
              <span className="retention-hint">0 = never</span>
            </div>
          </div>

          {retentionSaveStatus && (
            <div className={`retention-status ${retentionSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
              {retentionSaveStatus}
            </div>
          )}
        </div>

        <footer className="settings-modal-footer">
          <button type="button" onClick={() => setDraft({ ...defaultHotkeys })}>Restore defaults</button>
          <span className="settings-footer-spacer" />
          <button type="button" onClick={onClose}>Cancel</button>
          <button type="button" className="settings-primary-button" disabled={saving} onClick={save}>
            {saving ? "Saving..." : "Save"}
          </button>
        </footer>
      </section>
    </div>
  );
}

function isBareKey(event: KeyboardEvent): boolean {
  return !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
}
