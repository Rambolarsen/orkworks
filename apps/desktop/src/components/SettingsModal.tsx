import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings } from "../appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, OllamaVerificationResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import { normalizeActiveHarnessIds, selectableHarnesses } from "../newSessionDialogState";
import ProviderSettingsSection from "./ProviderSettingsSection";
import HarnessIntegrationSection from "./HarnessIntegrationSection";
import HarnessDetectionStatus from "./HarnessDetectionStatus";
import HarnessIcon from "./HarnessIcon";
import Toggle from "./Toggle";
import Button from "./Button";
import Input from "./Input";
import { createSettingsController } from "../settingsController";

// The controller delegates to the existing window.orkworks.verifyOllama and
// window.orkworks.saveProviderSettings IPC methods; the modal never bypasses it.

type HotkeyAction = keyof HotkeySettings;
type OllamaVerificationViewState =
  | { phase: "idle" }
  | { phase: "checking"; requestedBaseUrl: string }
  | { phase: "done"; result: OllamaVerificationResponse };

type SettingsSection = "tools" | "providers" | "hotkeys" | "retention" | "debug";

const NAV_ITEMS: Array<{ key: SettingsSection; label: string }> = [
  { key: "tools", label: "Coding tools" },
  { key: "providers", label: "Model providers" },
  { key: "hotkeys", label: "Hotkeys" },
  { key: "retention", label: "Session retention" },
  { key: "debug", label: "Debug" },
];

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
const INTEGRATION_HARNESS_IDS = ["claude-code", "gemini", "copilot", "codex", "opencode"];

export default function SettingsModal({ initialSettings, harnesses, activeHarnessIds, providerRuntime, onClose, onSaved, onSaveActiveHarnesses }: SettingsModalProps) {
  const modalRef = useRef<HTMLElement>(null);
  const settingsControllerRef = useRef<ReturnType<typeof createSettingsController> | null>(null);
  if (!settingsControllerRef.current) {
    settingsControllerRef.current = createSettingsController(window.orkworks, initialSettings);
  }
  const settingsController = settingsControllerRef.current;
  const defaultHotkeys = initialSettings.defaultHotkeys;
  const [activeSection, setActiveSection] = useState<SettingsSection>("tools");
  const [draft, setDraft] = useState<HotkeySettings>(initialSettings.hotkeys);
  const [capturing, setCapturing] = useState<HotkeyAction | null>(null);
  const [errors, setErrors] = useState<Partial<Record<HotkeyAction, string[]>>>({});
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [retention, setRetention] = useState<RetentionSettings>(initialSettings.retention);
  const [retentionSaveStatus, setRetentionSaveStatus] = useState<string | null>(null);
  const [debugSettings, setDebugSettings] = useState<DebugSettings>(initialSettings.debug);
  const [debugSaveStatus, setDebugSaveStatus] = useState<string | null>(null);
  const [providerDraft, setProviderDraft] = useState<ProviderSettings>(initialSettings.providers);
  const [providerModels, setProviderModels] = useState<Record<string, string[]>>({});
  const [providerSaveStatus, setProviderSaveStatus] = useState<string | null>(null);
  const [peonModelDraft, setPeonModelDraft] = useState<string | null>(initialSettings.providers.peonModel);
  const [ollamaBaseUrlDraft, setOllamaBaseUrlDraft] = useState<string>(initialSettings.providers.ollamaBaseUrl);
  const [ollamaVerification, setOllamaVerification] = useState<OllamaVerificationViewState>({ phase: "idle" });
  const verifyRequestRef = useRef(0);
  const [activeDraft, setActiveDraft] = useState<string[]>(() =>
    normalizeActiveHarnessIds(harnesses, activeHarnessIds),
  );
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

  useEffect(() => {
    setOllamaBaseUrlDraft(providerDraft.ollamaBaseUrl);
  }, [providerDraft.ollamaBaseUrl]);

  function normalizeBaseUrlDraft(baseUrl: string): string {
    const trimmed = baseUrl.trim().replace(/\/+$/, "");
    try {
      const parsed = new URL(trimmed);
      if (parsed.pathname !== "/" || parsed.search || parsed.hash) {
        return trimmed;
      }
      return parsed.origin;
    } catch {
      return trimmed;
    }
  }

  function failedVerificationResult(baseUrl: string, diagnostic: string): OllamaVerificationResponse {
    return {
      ok: false,
      normalizedBaseUrl: normalizeBaseUrlDraft(baseUrl),
      status: "failed",
      reasonCode: "invalid_url",
      httpStatus: null,
      models: [],
      excludedModels: [],
      diagnostic,
    };
  }

  async function verifyOllamaDraft(baseUrl: string) {
    const requestId = ++verifyRequestRef.current;
    const normalizedDraft = normalizeBaseUrlDraft(baseUrl);
    setOllamaVerification({ phase: "checking", requestedBaseUrl: normalizedDraft });
    try {
      const result = await settingsController.verifyOllama(baseUrl);
      if (requestId !== verifyRequestRef.current) return;
      setOllamaVerification({ phase: "done", result });
    } catch (error) {
      if (requestId !== verifyRequestRef.current) return;
      setOllamaVerification({
        phase: "done",
        result: failedVerificationResult(
          baseUrl,
          error instanceof Error ? error.message : "Couldn't verify Ollama.",
        ),
      });
    }
  }

  function renderOllamaVerificationStatus() {
    if (ollamaVerification.phase === "idle") {
      return <span className="provider-verify-status">No verification result yet.</span>;
    }
    if (ollamaVerification.phase === "checking") {
      return <span className="provider-verify-status">Checking {ollamaVerification.requestedBaseUrl || "Ollama"}…</span>;
    }
    const { result } = ollamaVerification;
    if (result.status === "connected") {
      return (
        <span className="provider-verify-status provider-verify-status--ok">
          Connected to {result.normalizedBaseUrl}.
        </span>
      );
    }
    if (result.status === "connected_empty") {
      return (
        <span className="provider-verify-status">
          Connected to {result.normalizedBaseUrl}, but no Peon candidate models were found.
        </span>
      );
    }
    return (
      <span className="provider-verify-status provider-verify-status--error">
        {result.diagnostic ?? `Couldn't verify ${result.normalizedBaseUrl}.`}
      </span>
    );
  }

  const candidateModels =
    ollamaVerification.phase === "done" && ollamaVerification.result.ok
      ? ollamaVerification.result.models
      : [];

  async function saveRetention(rt: RetentionSettings) {
    setRetentionSaveStatus(null);
    setRetention(rt);
    settingsController.updateDraft("retention", rt);
    setRetentionSaveStatus("Pending save");
  }

  async function saveDebugSettings(debug: DebugSettings) {
    setDebugSaveStatus(null);
    setDebugSettings(debug);
    settingsController.updateDraft("debug", debug);
    setDebugSaveStatus("Pending save");
  }

  async function save() {
    setSaving(true);
    setErrors({});
    setSaveError(null);
    try {
      settingsController.updateDraft("hotkeys", draft);
      const result = await settingsController.commit();
      if (result.ok) {
        const retentionPending = Boolean(result.retentionApplyStatus?.lastApplyError);
        const providerPending = Boolean(result.providerApplyStatus?.lastApplyError);
        if (retentionPending) setRetentionSaveStatus("Saved locally; sidecar pending");
        if (providerPending) setProviderSaveStatus("Saved locally; sidecar pending");
        if (retentionPending || providerPending) return;
        onSaved(result.settings);
        onClose();
      } else {
        setSaveError(`Couldn't save ${result.failedDomain} settings.`);
      }
    } catch {
      setSaveError("Settings could not be saved. The active shortcuts were not changed.");
    } finally {
      setSaving(false);
    }
  }

  async function savePeonModel(model: string | null) {
    setProviderSaveStatus(null);
    const nextBaseUrl =
      ollamaVerification.phase === "done" && ollamaVerification.result.ok
        ? ollamaVerification.result.normalizedBaseUrl
        : normalizeBaseUrlDraft(ollamaBaseUrlDraft);
    const next = { ...providerDraft, peonModel: model, ollamaBaseUrl: nextBaseUrl };
    setProviderDraft(next);
    settingsController.updateDraft("providers", next);
  }

  function toggleHarness(id: string) {
    setActiveDraft((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }

  async function saveActiveHarnessesHandler() {
    setActiveSaveStatus(null);
    try {
      const normalizedActiveDraft = normalizeActiveHarnessIds(harnesses, activeDraft);
      await onSaveActiveHarnesses(normalizedActiveDraft);
      setActiveDraft(normalizedActiveDraft);
      setActiveSaveStatus("Saved");
    } catch {
      setActiveSaveStatus("Couldn't save active coding tools.");
    }
  }

  function persistProviderSettings(settings: ProviderSettings) {
    setProviderDraft(settings);
    settingsController.updateDraft("providers", settings);
    setProviderSaveStatus("Pending save");
  }

  return (
    <div className="settings-backdrop" role="presentation">
      <section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title" ref={modalRef}>
        <header className="settings-modal-header">
          <div>
            <h2 id="settings-title">Settings</h2>
            <p>Configure OrkWorks desktop preferences.</p>
          </div>
          <button className="settings-icon-button" type="button" onClick={() => { settingsController.discard(); onClose(); }} aria-label="Close settings">
            ×
          </button>
        </header>

        <div className="settings-body">
          <nav className="settings-nav">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.key}
                type="button"
                className={`settings-nav-button${activeSection === item.key ? " settings-nav-button--active" : ""}`}
                onClick={() => setActiveSection(item.key)}
              >
                {item.label}
              </button>
            ))}
          </nav>

          <div className="settings-content">
            {activeSection === "tools" && (
              <div className="settings-section">
                <h3>Active coding tools</h3>
                <p className="settings-section-copy">
                  Select which coding tools are available in this workspace. Shell is always available.
                </p>

                <div className="settings-config-list">
                  {selectableHarnesses(harnesses)
                    .filter((h) => h.id !== "generic-shell")
                    .sort((a, b) => a.name.localeCompare(b.name))
                    .map((h) => (
                      <div key={h.id} className="settings-config-item-row">
                        <div className="settings-config-item-header">
                          <div className="settings-config-item">
                            <HarnessIcon tool={h.name} size={16} />
                            <span>{h.name}</span>
                            <HarnessDetectionStatus harnessId={h.id} />
                          </div>
                          <Toggle checked={activeDraft.includes(h.id)} onChange={() => toggleHarness(h.id)} />
                        </div>
                        {INTEGRATION_HARNESS_IDS.includes(h.id) && activeDraft.includes(h.id) && (
                          <HarnessIntegrationSection harnessId={h.id} harnessName={h.name} harness={h} />
                        )}
                      </div>
                    ))}
                </div>

                <div className="settings-config-footer">
                  <Button variant="secondary" size="sm" onClick={saveActiveHarnessesHandler}>Save</Button>
                  {activeSaveStatus && (
                    <span className={`settings-config-status ${activeSaveStatus === "Saved" ? "settings-config-status--ok" : ""}`}>
                      {activeSaveStatus}
                    </span>
                  )}
                </div>
              </div>
            )}

            {activeSection === "debug" && (
              <div className="settings-section">
                <h3>Debug</h3>
                <p className="settings-section-copy">
                  Reveal internal metadata in session details when you need to debug session state.
                </p>

                <Toggle
                  checked={debugSettings.showSessionIds}
                  onChange={() => {
                    const next = { showSessionIds: !debugSettings.showSessionIds, rendererHealthLogMs: debugSettings.rendererHealthLogMs };
                    setDebugSettings(next);
                    saveDebugSettings(next);
                  }}
                  label="Show debug metadata"
                />

                {debugSaveStatus && (
                  <div className={`retention-status ${debugSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
                    {debugSaveStatus}
                  </div>
                )}
              </div>
            )}

            {activeSection === "providers" && (
              <div className="settings-section">
                <h3>Model providers</h3>
                <p className="settings-section-copy">
                  Configure model provider fallback order, state overrides, and Peon model.
                </p>

                <div className="provider-list">
                  <div className="provider-card">
                    <div className="provider-label">Peon model</div>
                    <input
                      className="provider-model-select"
                      type="text"
                      list="peon-model-suggestions"
                      placeholder="(none - let model provider decide)"
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

                  <div className="provider-card">
                    <div className="provider-label">Ollama base URL</div>
                    <input
                      className="provider-model-select"
                      type="text"
                      placeholder="http://127.0.0.1:11434"
                      value={ollamaBaseUrlDraft}
                      onChange={(e) => {
                        verifyRequestRef.current++;
                        setOllamaVerification({ phase: "idle" });
                        setOllamaBaseUrlDraft(e.target.value.trim());
                      }}
                      onBlur={() => {
                        const normalized = normalizeBaseUrlDraft(ollamaBaseUrlDraft);
                        if (normalized !== providerDraft.ollamaBaseUrl && (normalized.startsWith("http://") || normalized.startsWith("https://"))) {
                          const next = { ...providerDraft, ollamaBaseUrl: normalized };
                          persistProviderSettings(next);
                        }
                      }}
                    />
                  </div>

                  <div className="provider-card">
                    <div className="provider-label">Ollama verification</div>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={ollamaVerification.phase === "checking"}
                      onClick={() => verifyOllamaDraft(ollamaBaseUrlDraft)}
                    >
                      {ollamaVerification.phase === "checking" ? "Verifying…" : "Verify Ollama"}
                    </Button>
                    <div role="status" aria-live="polite">
                      {renderOllamaVerificationStatus()}
                    </div>
                    <ul className="ollama-candidate-list">
                      {candidateModels.map((model) => (
                        <li key={model}>
                          <span className={model === peonModelDraft ? "selected-model" : undefined}>{model}</span>
                          <Button
                            variant="ghost"
                            size="sm"
                            ariaLabel={`Use ${model} for Peon`}
                            onClick={() => {
                              setPeonModelDraft(model);
                              void savePeonModel(model);
                            }}
                          >
                            Use this model
                          </Button>
                        </li>
                      ))}
                    </ul>
                  </div>

                  <ProviderSettingsSection
                    providerSettings={providerDraft}
                    providerRuntime={providerRuntime}
                  />
                </div>

                {providerSaveStatus && (
                  <div className={`retention-status ${providerSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
                    {providerSaveStatus}
                  </div>
                )}
              </div>
            )}

            {activeSection === "hotkeys" && (
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
                      <Button variant="ghost" size="sm" onClick={() => setCapturing(row.action)}>Edit</Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          settingsController.resetHotkey(row.action);
                          setDraft((current) => ({ ...current, [row.action]: defaultHotkeys[row.action] }));
                        }}
                      >
                        Reset
                      </Button>
                    </div>
                  ))}
                </div>
                {saveError && <div className="settings-save-error">{saveError}</div>}
              </div>
            )}

            {activeSection === "retention" && (
              <div className="settings-section">
                <h3>Session Retention</h3>
                <p className="settings-section-copy">
                  Live sessions are never auto-deleted. Changes take effect within 5 minutes.
                </p>

                <div className="retention-list">
                  <div className="retention-row">
                    <div className="retention-label">Max sessions to keep</div>
                    <Input
                      type="number"
                      min={0}
                      max={999}
                      style={{ width: 72 }}
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
                    <Input
                      type="number"
                      min={0}
                      max={999}
                      style={{ width: 72 }}
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
            )}
          </div>
        </div>

        <footer className="settings-modal-footer">
          <Button variant="secondary" size="sm" onClick={() => {
            settingsController.updateDraft("hotkeys", { ...defaultHotkeys });
            setDraft({ ...defaultHotkeys });
          }}>Restore defaults</Button>
          <span className="settings-footer-spacer" />
          <Button variant="ghost" size="sm" onClick={() => { settingsController.discard(); onClose(); }}>Cancel</Button>
          <Button variant="primary" size="sm" disabled={saving} onClick={save}>
            {saving ? "Saving..." : "Save"}
          </Button>
        </footer>
      </section>
    </div>
  );
}

function isBareKey(event: KeyboardEvent): boolean {
  return !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
}
