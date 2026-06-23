import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "../appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import ProviderSettingsSection from "./ProviderSettingsSection";

type HotkeyAction = keyof HotkeySettings;

interface SettingsModalProps {
  initialSettings: AppSettings;
  harnesses: HarnessConfig[];
  activeHarnessIds: string[];
  providerRuntime: ProviderRuntimeResponse | null;
  onClose: () => void;
  onSaved: (settings: AppSettings) => void;
  onSaveActiveHarnesses: (ids: string[]) => Promise<void>;
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

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export default function SettingsModal({ initialSettings, harnesses, activeHarnessIds, providerRuntime, onClose, onSaved, onSaveActiveHarnesses }: SettingsModalProps) {
  const modalRef = useRef<HTMLElement>(null);
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
  const [peonModelDraft, setPeonModelDraft] = useState<string | null>(initialSettings.providers.peonModel);
  const [activeDraft, setActiveDraft] = useState<string[]>(activeHarnessIds);
  const [activeSaveStatus, setActiveSaveStatus] = useState<string | null>(null);

  useLayoutEffect(() => {
    const modal = modalRef.current;
    if (!modal) return;

    const focusable = modal.querySelectorAll<HTMLElement>(FOCUSABLE);
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (first) first.focus();

    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Tab") return;

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    }

    modal.addEventListener("keydown", onKeyDown);
    return () => modal.removeEventListener("keydown", onKeyDown);
  }, []);

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

  useEffect(() => {
    setPeonModelDraft(providerDraft.peonModel);
  }, [providerDraft.peonModel]);

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

  async function savePeonModel(model: string | null) {
    setProviderSaveStatus(null);
    const next = { ...providerDraft, peonModel: model };
    setProviderDraft(next);
    await persistProviderSettings(next);
  }

  function toggleHarness(id: string) {
    setActiveDraft((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }

  async function saveActiveHarnessesHandler() {
    setActiveSaveStatus(null);
    try {
      await onSaveActiveHarnesses(activeDraft);
      setActiveSaveStatus("Saved");
    } catch {
      setActiveSaveStatus("Couldn't save active providers.");
    }
  }

  async function persistProviderSettings(settings: ProviderSettings) {
    try {
      const result = await window.orkworks.saveProviderSettings(settings);
      setProviderDraft(result.settings.providers);
      onSaved(result.settings);
      setProviderSaveStatus("Saved");
    } catch {
      setProviderSaveStatus("Couldn't save provider settings.");
    }
  }

  return (
    <div className="settings-backdrop" role="presentation">
      <section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title" ref={modalRef}>
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
          <h3>Active Providers</h3>
          <p className="settings-section-copy">
            Select which providers are available in this workspace. Shell is always available.
          </p>

          <div className="settings-config-list">
            {harnesses
              .filter((h) => h.id !== "generic-shell")
              .sort((a, b) => a.name.localeCompare(b.name))
              .map((h) => (
                <label key={h.id} className="settings-config-item">
                  <input
                    type="checkbox"
                    checked={activeDraft.includes(h.id)}
                    onChange={() => toggleHarness(h.id)}
                  />
                  <span>{h.name}</span>
                </label>
              ))}
          </div>

          <div className="settings-config-footer">
            <button type="button" onClick={saveActiveHarnessesHandler}>Save</button>
            {activeSaveStatus && (
              <span className={`settings-config-status ${activeSaveStatus === "Saved" ? "settings-config-status--ok" : ""}`}>
                {activeSaveStatus}
              </span>
            )}
          </div>
        </div>

        <div className="settings-section">
          <h3>Provider Settings</h3>
          <p className="settings-section-copy">
            Configure per-provider fallback order, state overrides, and peon model.
          </p>

          <div className="provider-list">
            <div className="provider-card">
              <div className="provider-label">Peon Model</div>
              <input
                className="provider-model-select"
                type="text"
                list="peon-model-suggestions"
                placeholder="(none — let provider decide)"
                value={peonModelDraft ?? ""}
                onChange={(e) => setPeonModelDraft(e.target.value.trim() || null)}
                onBlur={() => {
                  if (peonModelDraft !== providerDraft.peonModel) {
                    savePeonModel(peonModelDraft);
                  }
                }}
              />
              <datalist id="peon-model-suggestions">
                {[...new Set(Object.values(providerModels).flat())].sort().map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </div>

            <ProviderSettingsSection
              providerSettings={providerDraft}
              providerRuntime={providerRuntime}
              onSaveProviderSettings={persistProviderSettings}
            />
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
