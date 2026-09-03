import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings } from "../appSettingsTypes";
import { normalizeProviderSettings, type ProviderDefinition, type ProviderId, type ProviderSettings, type PeonSelection, type PeonAppliedState, type PeonProviderVerificationResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { duplicateHarness, stripDerivedHarnessFields, type HarnessConfigEntry, type HarnessEditorMode, type HarnessListResponse, type HarnessMutationResponse } from "../api";
import HarnessConfigEditor from "./HarnessConfigEditor";
import type { HarnessConfig, HarnessEditorMetadata, IntegrationStatus, IntegrationStatusResult } from "../harnessTypes";
import {
  deriveIntegrationDisplayState,
  isReconcileActionable,
  type IntegrationKey,
  type ActiveHarnessIntegrationResult,
  type ActiveHarnessSaveResult,
  type IntegrationDisplayState,
} from "../harnessIntegrationPresentation";
import { normalizeActiveHarnessIds, selectableHarnesses } from "../newSessionDialogState";
import { mergeIntegrationOperationFailures } from "../settingsController";
import HarnessCommandPathControl, { looksAbsolute } from "./HarnessCommandPathControl";
import HarnessDetectionStatus from "./HarnessDetectionStatus";
import HarnessIcon from "./HarnessIcon";
import Toggle from "./Toggle";
import Button from "./Button";
import Input from "./Input";

type HotkeyAction = keyof HotkeySettings;

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
  harnesses: HarnessConfigEntry[];
  documentRevision: string | null;
  onRefreshHarnesses: () => Promise<HarnessListResponse>;
  activeHarnessIds: string[];
  providerRuntime: ProviderRuntimeResponse | null;
  onClose: () => void;
  onSaved: (settings: AppSettings) => void;
  onSaveActiveHarnesses: (ids: string[]) => Promise<ActiveHarnessSaveResult>;
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

function integrationKeyForHarness(harness: HarnessConfig): IntegrationKey | null {
  if (!harness.integration || typeof harness.integration !== "object") return null;
  const kind = (harness.integration as { kind?: unknown }).kind;
  return typeof kind === "string" && kind.length > 0
    ? { adapterId: kind, targetId: "workspace" }
    : null;
}

function jsonText(value: unknown): string {
  return `${JSON.stringify(value ?? {}, null, 2)}\n`;
}

function customHarnessStarter(): Record<string, unknown> {
  return {
    id: "my-tool",
    name: "My coding tool",
    launch: { kind: "command-template", command: "my-tool", args: [], modelPrefix: null },
    defaultModel: null,
    resume: null,
    models: null,
    peon: null,
    capacity: null,
    voice: null,
    minVersion: null,
    labelResetCommands: [],
  };
}

function editableHarnessDefinition(harness: HarnessConfigEntry): unknown {
  return stripDerivedHarnessFields(harness);
}

export default function SettingsModal({ initialSettings, harnesses, documentRevision, onRefreshHarnesses, activeHarnessIds, providerRuntime, onClose, onSaved, onSaveActiveHarnesses }: SettingsModalProps) {
  const modalRef = useRef<HTMLElement>(null);
  const savedSettingsRef = useRef<AppSettings>(clone(initialSettings));
  const defaultHotkeys = initialSettings.defaultHotkeys;
  const toolHarnesses = selectableHarnesses(harnesses)
    .filter((h) => h.id !== "generic-shell")
    .sort((a, b) => a.name.localeCompare(b.name));
  const integrationHarnessStatusKey = toolHarnesses
    .map((h) => integrationKeyForHarness(h))
    .filter((key): key is IntegrationKey => key !== null)
    .map((key) => `${key.adapterId}/${key.targetId}`)
    .filter(Boolean)
    .filter((key, index, all) => all.indexOf(key) === index)
    .join("\0");
  const [activeSection, setActiveSection] = useState<SettingsSection>("tools");
  const [draft, setDraft] = useState<HotkeySettings>(initialSettings.hotkeys);
  const [savedHotkeys, setSavedHotkeys] = useState<HotkeySettings>(initialSettings.hotkeys);
  const [capturing, setCapturing] = useState<HotkeyAction | null>(null);
  const [errors, setErrors] = useState<Partial<Record<HotkeyAction, string[]>>>({});
  const [hotkeySaveStatus, setHotkeySaveStatus] = useState<string | null>(null);
  const [retention, setRetention] = useState<RetentionSettings>(initialSettings.retention);
  const [retentionSaveStatus, setRetentionSaveStatus] = useState<string | null>(null);
  const [debugSettings, setDebugSettings] = useState<DebugSettings>(initialSettings.debug);
  const [debugSaveStatus, setDebugSaveStatus] = useState<string | null>(null);
  const [providerDraft, setProviderDraft] = useState<ProviderSettings>(initialSettings.providers);
  const [providerSaveStatus, setProviderSaveStatus] = useState<string | null>(null);
  const initialPeonSelection: PeonSelection = initialSettings.providers.peonSelection ?? {
    provider: "ollama",
    model: initialSettings.providers.peonModel ?? "",
    ollamaBaseUrl: initialSettings.providers.ollamaBaseUrl,
  };
  const [peonSelection, setPeonSelection] = useState<PeonSelection>(initialPeonSelection);
  const [peonVerification, setPeonVerification] = useState<PeonProviderVerificationResponse | null>(null);
  const [peonApplied, setPeonApplied] = useState<PeonAppliedState | null>(null);
  const [peonLocallyApplied, setPeonLocallyApplied] = useState(false);
  const [peonBusy, setPeonBusy] = useState(false);
  const [peonBusyElapsedSeconds, setPeonBusyElapsedSeconds] = useState(0);
  const [peonError, setPeonError] = useState<string | null>(null);
  const [unavailablePeonProvider, setUnavailablePeonProvider] = useState<string | null>(null);
  const [manualModelOverride, setManualModelOverride] = useState(false);
  const peonVerificationGeneration = useRef(0);
  const modalLifecycleGeneration = useRef(0);
  const toolsSaveGeneration = useRef(0);
  const reconcileGeneration = useRef(0);
  const integrationStatusRequestGeneration = useRef(0);
  const [activeDraft, setActiveDraft] = useState<string[]>(() =>
    normalizeActiveHarnessIds(harnesses, activeHarnessIds),
  );
  const [activeSaveStatus, setActiveSaveStatus] = useState<string | null>(null);
  const [toolsSaveInProgress, setToolsSaveInProgress] = useState(false);
  const [detectionGenerations, setDetectionGenerations] = useState<Record<string, number>>({});
  const [integrationStatuses, setIntegrationStatuses] = useState<Record<string, IntegrationStatusResult>>({});
  const [integrationOperationFailures, setIntegrationOperationFailures] = useState<Record<string, ActiveHarnessIntegrationResult>>({});
  const [reconcileInProgressKey, setReconcileInProgressKey] = useState<string | null>(null);
  const [integrationStatusGeneration, setIntegrationStatusGeneration] = useState(0);
  const [harnessEditor, setHarnessEditor] = useState<{
    mode: HarnessEditorMode;
    draftText: string;
    metadata: HarnessEditorMetadata;
  } | null>(null);
  const [harnessActionStatus, setHarnessActionStatus] = useState<string | null>(null);
  const [expandedCommandPaths, setExpandedCommandPaths] = useState<Record<string, boolean>>({});
  const verificationTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  function toggleCommandPathExpanded(harnessId: string) {
    setExpandedCommandPaths((current) => ({ ...current, [harnessId]: !current[harnessId] }));
  }

  function invalidateAsyncState() {
    modalLifecycleGeneration.current += 1;
    toolsSaveGeneration.current += 1;
    reconcileGeneration.current += 1;
    integrationStatusRequestGeneration.current += 1;
    peonVerificationGeneration.current += 1;
    setReconcileInProgressKey(null);
    if (verificationTimer.current) {
      clearTimeout(verificationTimer.current);
      verificationTimer.current = null;
    }
  }

  function refreshDetection(harnessId: string) {
    setDetectionGenerations((current) => ({
      ...current,
      [harnessId]: (current[harnessId] ?? 0) + 1,
    }));
    setIntegrationStatusGeneration((current) => current + 1);
    void onRefreshHarnesses().catch(() => undefined);
  }

  function refreshDetections(harnessIds: readonly string[]) {
    if (harnessIds.length === 0) return;
    setDetectionGenerations((current) => {
      const next = { ...current };
      for (const harnessId of harnessIds) {
        next[harnessId] = (next[harnessId] ?? 0) + 1;
      }
      return next;
    });
    setIntegrationStatusGeneration((current) => current + 1);
  }

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
    return () => {
      invalidateAsyncState();
    };
  }, []);

  useEffect(() => {
    if (!peonBusy) {
      setPeonBusyElapsedSeconds(0);
      return;
    }
    const startedAt = Date.now();
    const timer = setInterval(() => {
      setPeonBusyElapsedSeconds(Math.floor((Date.now() - startedAt) / 1000));
    }, 500);
    return () => clearInterval(timer);
  }, [peonBusy]);

  useEffect(() => {
    let mounted = true;
    void window.orkworks.getAppliedPeonProvider().then((applied) => {
      if (mounted) setPeonApplied(applied);
    }).catch(() => {
      if (mounted) setPeonError("Couldn't load the applied Peon configuration.");
    });
    return () => { mounted = false; };
  }, []);

  useEffect(() => {
    if (!providerRuntime || providerRuntime.providers.length === 0) return;
    const definitions: ProviderDefinition[] = providerRuntime.providers.map((provider) => ({
      id: provider.id,
      label: provider.label,
      ...(provider.harnessId ? { harnessId: provider.harnessId } : {}),
      origin: provider.origin,
    }));
    setProviderDraft((current) => normalizeProviderSettings(current, definitions));
    const selectedProvider = peonSelection.provider;
    if (!definitions.some((definition) => definition.id === selectedProvider)) {
      setUnavailablePeonProvider(selectedProvider);
      setPeonVerification(null);
      setPeonLocallyApplied(false);
    } else {
      setUnavailablePeonProvider(null);
    }
  }, [peonSelection.provider, providerRuntime]);

  useEffect(() => {
    void verifyPeonSelection(initialPeonSelection);
    // The initial selection is intentionally verified once when the modal opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let cancelled = false;
    const lifecycleGeneration = modalLifecycleGeneration.current;
    const requestGeneration = ++integrationStatusRequestGeneration.current;
    const integrationGroupKeys = integrationHarnessStatusKey === ""
      ? []
      : integrationHarnessStatusKey.split("\0");

    async function loadIntegrationStatuses() {
      if (integrationGroupKeys.length === 0) {
        if (cancelled || requestGeneration !== integrationStatusRequestGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
        setIntegrationStatuses({});
        return;
      }
      const entries = await Promise.all(
        integrationGroupKeys.map(async (groupKey) => {
          const [adapterId, targetId] = groupKey.split("/");
          try {
            return [groupKey, await window.orkworks.getGroupedHarnessIntegrationStatus(adapterId, targetId)] as const;
          } catch (error) {
            return [
              groupKey,
              { ok: false, error: error instanceof Error ? error.message : "Integration status unavailable." },
            ] as const;
          }
        }),
      );
      if (cancelled || requestGeneration !== integrationStatusRequestGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      const groupedStatuses = Object.fromEntries(entries);
      const rowStatuses = Object.fromEntries(toolHarnesses
        .map((harness) => [harness, integrationKeyForHarness(harness)] as const)
        .filter((entry): entry is [HarnessConfigEntry, IntegrationKey] => entry[1] !== null)
        .map((harness) => {
          const [harnessConfig, key] = harness;
          const groupKey = `${key.adapterId}/${key.targetId}`;
          const groupedStatus = groupedStatuses[groupKey] as {
            ok: true;
            group: { status: IntegrationStatus; consumers: Array<{ harnessId: string; harnessName: string }> };
          } | { ok: false; error: string; code?: string } | undefined;
          return [harnessConfig.id, groupedStatus?.ok
            ? {
              ok: true,
              status: {
                ...groupedStatus.group.status,
                activeConsumerCount: groupedStatus.group.consumers.length,
              },
            }
            : { ok: false, error: groupedStatus?.error ?? "Integration status unavailable." }] as const;
        })) as Record<string, IntegrationStatusResult>;
      setIntegrationStatuses(rowStatuses);
    }

    void loadIntegrationStatuses();
    return () => {
      cancelled = true;
    };
  }, [integrationHarnessStatusKey, integrationStatusGeneration]);

  function updateSavedSettings(nextSettings: AppSettings) {
    const next = clone(nextSettings);
    savedSettingsRef.current = next;
    onSaved(next);
  }

  function mergeSavedSettings(partial: Partial<AppSettings>) {
    updateSavedSettings({ ...savedSettingsRef.current, ...partial });
  }

  function discardDraftsAndClose() {
    invalidateAsyncState();
    setCapturing(null);
    setErrors({});
    setHotkeySaveStatus(null);
    setDraft(clone(savedHotkeys));
    setRetention(clone(savedSettingsRef.current.retention));
    setRetentionSaveStatus(null);
    setDebugSettings(clone(savedSettingsRef.current.debug));
    setDebugSaveStatus(null);
    setProviderDraft(clone(savedSettingsRef.current.providers));
    setProviderSaveStatus(null);
    setPeonSelection(peonSelectionFromProviders(savedSettingsRef.current.providers));
    setPeonVerification(null);
    setPeonLocallyApplied(false);
    setPeonBusy(false);
    setPeonError(null);
    setManualModelOverride(false);
    setActiveDraft(normalizeActiveHarnessIds(harnesses, activeHarnessIds));
    setActiveSaveStatus(null);
    setToolsSaveInProgress(false);
    setIntegrationStatuses({});
    setIntegrationOperationFailures({});
    setIntegrationStatusGeneration((current) => current + 1);
    onClose();
  }

  async function saveRetention(rt: RetentionSettings) {
    setRetentionSaveStatus(null);
    setRetention(rt);
    try {
      const result = await window.orkworks.saveRetention(rt);
      if (!result.ok) throw new Error("save-retention failed");
      setRetentionSaveStatus(result.retentionApplyStatus?.lastApplyError ? "Saved locally; sidecar pending" : "Saved");
      mergeSavedSettings({ retention: clone(rt) });
    } catch {
      setRetentionSaveStatus("Session retention could not be saved.");
    }
  }

  async function saveDebugSettings(debug: DebugSettings) {
    setDebugSaveStatus(null);
    setDebugSettings(debug);
    try {
      const result = await window.orkworks.saveDebugSettings(debug);
      setDebugSettings(result.settings.debug);
      setDebugSaveStatus("Saved");
      updateSavedSettings(result.settings);
    } catch {
      setDebugSaveStatus("Debug settings could not be saved.");
    }
  }

  function toggleHarness(id: string) {
    setActiveDraft((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }

  function updateIntegrationFailures(results: Record<string, ActiveHarnessIntegrationResult>) {
    setIntegrationOperationFailures((current) => mergeIntegrationOperationFailures(current, results));
  }

  async function saveActiveHarnessesHandler() {
    const lifecycleGeneration = modalLifecycleGeneration.current;
    const requestGeneration = ++toolsSaveGeneration.current;
    setActiveSaveStatus(null);
    setToolsSaveInProgress(true);
    try {
      const normalizedActiveDraft = normalizeActiveHarnessIds(harnesses, activeDraft);
      const result = await onSaveActiveHarnesses(normalizedActiveDraft);
      if (requestGeneration !== toolsSaveGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      if (result.activeHarnesses.outcome === "persisted") {
        updateIntegrationFailures(result.integrations);
        refreshDetections([
          ...new Set(Object.values(result.integrations).flatMap((operation) => operation.consumerHarnessIds)),
        ]);
        setActiveDraft(normalizedActiveDraft);
        return;
      }
      setActiveSaveStatus(result.activeHarnesses.message ?? "Couldn't save active coding tools.");
    } catch {
      if (requestGeneration !== toolsSaveGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      setActiveSaveStatus("Couldn't save active coding tools.");
    } finally {
      if (requestGeneration !== toolsSaveGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      setToolsSaveInProgress(false);
    }
  }

  async function reconcileIntegrationHandler(key: IntegrationKey) {
    const keyId = `${key.adapterId}/${key.targetId}`;
    const lifecycleGeneration = modalLifecycleGeneration.current;
    const requestGeneration = ++reconcileGeneration.current;
    setActiveSaveStatus(null);
    setReconcileInProgressKey(keyId);
    try {
      const result = await window.orkworks.reconcileHarnessIntegration(key.adapterId, key.targetId);
      if (requestGeneration !== reconcileGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      // mergeIntegrationOperationFailures neither keeps nor clears
      // stale_workspace results, so the workspace switch is surfaced as an
      // explicit status message instead of a silent no-op.
      if (result.outcome === "stale_workspace") {
        setActiveSaveStatus(result.message ?? "Workspace changed while reconciling. Reload the current workspace and retry.");
        return;
      }
      updateIntegrationFailures({ [keyId]: result });
      refreshDetections(result.consumerHarnessIds);
    } catch {
      if (requestGeneration !== reconcileGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      setActiveSaveStatus("Couldn't reconcile the integration.");
    } finally {
      if (requestGeneration !== reconcileGeneration.current || lifecycleGeneration !== modalLifecycleGeneration.current) return;
      setReconcileInProgressKey(null);
    }
  }

  async function saveHotkeysHandler() {
    setHotkeySaveStatus(null);
    setErrors({});
    try {
      const result = await window.orkworks.saveHotkeys(draft);
      if (!result.ok) {
        setErrors(result.errors);
        return;
      }
      setDraft(clone(result.settings.hotkeys));
      setSavedHotkeys(clone(result.settings.hotkeys));
      setHotkeySaveStatus("Saved");
      updateSavedSettings(result.settings);
    } catch {
      setHotkeySaveStatus("Hotkeys could not be saved.");
    }
  }

  function restoreHotkeyDefaults() {
    setDraft(clone(defaultHotkeys));
    setErrors({});
    setHotkeySaveStatus(null);
  }

  function cancelHotkeyChanges() {
    setCapturing(null);
    setDraft(clone(savedHotkeys));
    setErrors({});
    setHotkeySaveStatus(null);
  }

  function toolDisplayState(harness: HarnessConfig): IntegrationDisplayState {
    const enabled = activeDraft.includes(harness.id);
    if (harness.integration === null) {
      if (toolsSaveInProgress) {
        return {
          appearance: "in-progress",
          label: "updating",
          description: "Integration operation in progress.",
          tooltip: `OrkWorks is updating the ${harness.name} tool settings.`,
          glyph: "spinner",
        };
      }
      return enabled
        ? {
            appearance: "neutral",
            label: "no hook support",
            description: "Enabled. No OrkWorks hook support for this coding tool.",
            tooltip: `${harness.name} is enabled, but this coding tool has no OrkWorks hook capability.`,
            glyph: "neutral",
          }
        : {
            appearance: "off",
            label: "off",
            description: "Disabled. No OrkWorks integration remains.",
            tooltip: `${harness.name} is disabled and no OrkWorks-owned integration remains in this workspace.`,
            glyph: "neutral",
          };
    }

    const status = integrationStatuses[harness.id];
    if (!status) {
      return {
        appearance: "neutral",
        label: "checking status",
        description: "Checking integration status.",
        tooltip: `OrkWorks is checking the ${harness.name} integration status.`,
        glyph: "neutral",
      };
    }

    const rowKey = integrationKeyForHarness(harness);
    return deriveIntegrationDisplayState({
      harnessName: harness.name,
      enabled,
      status,
      operation: integrationOperationFailures[harness.id],
      inProgress: toolsSaveInProgress
        || (rowKey !== null && reconcileInProgressKey === `${rowKey.adapterId}/${rowKey.targetId}`),
    });
  }

  const hotkeysDirty = !deepEqual(draft, savedHotkeys);

  const peonProviders = providerDraft.providers.filter((entry) => entry.enabled).map((entry) => entry.id).filter((id, index, all) => all.indexOf(id) === index);
  const providerLabels = new Map<string, string>((providerRuntime?.providers ?? []).map((provider) => [provider.id, provider.label]));
  const selectedProviderIsAvailable = peonProviders.includes(peonSelection.provider);
  const peonApplyMatches = peonApplied?.provider === peonSelection.provider
    && peonApplied.model === peonSelection.model
    && (peonSelection.provider !== "ollama" || peonApplied.ollamaBaseUrl === peonSelection.ollamaBaseUrl)
    && peonLocallyApplied;

  function canonicalPeonSelection(selection: PeonSelection): PeonSelection {
    return { ...selection, model: selection.model.trim() };
  }

  function scheduleVerifyPeonSelection(selection: PeonSelection, immediate = false) {
    if (verificationTimer.current) clearTimeout(verificationTimer.current);
    if (immediate) {
      void verifyPeonSelection(selection);
      return;
    }
    verificationTimer.current = setTimeout(() => {
      verificationTimer.current = null;
      void verifyPeonSelection(selection);
    }, 300);
  }

  async function verifyPeonSelection(selection: PeonSelection) {
    const requestGeneration = ++peonVerificationGeneration.current;
    setPeonBusy(true);
    setPeonError(null);
    setPeonVerification(null);
    try {
      // window.orkworks.verifyOllama remains the direct Ollama-only bridge;
      // the provider-first Settings flow routes all verification through the
      // same Peon verification state machine.
      const result = await window.orkworks.verifyPeonProvider(selection.provider, selection.provider === "ollama" ? selection.ollamaBaseUrl : undefined);
      if (requestGeneration !== peonVerificationGeneration.current) return;
      setPeonVerification(result);
      if (result.ok && selection.provider === "ollama" && result.ollamaBaseUrl) {
        setPeonSelection((current) => current.provider === "ollama"
          ? { ...current, ollamaBaseUrl: result.ollamaBaseUrl! }
          : current);
      }
      if (!result.ok) setPeonError("Provider verification failed.");
    } catch (error) {
      if (requestGeneration !== peonVerificationGeneration.current) return;
      setPeonError(error instanceof Error ? error.message : "Provider verification failed.");
    } finally {
      if (requestGeneration === peonVerificationGeneration.current) setPeonBusy(false);
    }
  }

  async function applyPeonSelection() {
    if (!peonVerification?.ok || !peonSelection.model.trim()) return;
    const selection = canonicalPeonSelection(peonSelection);
    setPeonSelection(selection);
    setPeonBusy(true);
    setPeonError(null);
    try {
      const applied = await window.orkworks.testAndApplyPeonProvider(selection);
      setPeonApplied(applied);
      setPeonLocallyApplied(true);
      setProviderSaveStatus("Applied; save to persist");
    } catch (error) {
      setPeonError(error instanceof Error ? error.message : "Peon Apply failed.");
    } finally {
      setPeonBusy(false);
    }
  }

  async function savePeonSelection() {
    if (!peonApplyMatches) return;
    setPeonBusy(true);
    try {
      const result = await window.orkworks.savePeonSelection(peonSelection);
      if (!result.ok) throw new Error(result.error);
      setProviderDraft(result.settings.providers);
      setPeonLocallyApplied(false);
      setProviderSaveStatus("Saved");
      updateSavedSettings(result.settings);
    } catch (error) {
      setPeonError(error instanceof Error ? error.message : "Peon Save failed.");
    } finally {
      setPeonBusy(false);
    }
  }

  function openHarnessEditor(entry: HarnessConfigEntry) {
    const mode: HarnessEditorMode = entry.origin === "custom" ? "custom" : "override";
    setHarnessActionStatus(null);
    setHarnessEditor({
      mode,
      draftText: jsonText(mode === "override" ? entry.storedOverride : editableHarnessDefinition(entry)),
      metadata: { entry, documentRevision: documentRevision ?? entry.documentRevision ?? null },
    });
  }

  function openNewHarnessEditor() {
    setHarnessActionStatus(null);
    setHarnessEditor({
      mode: "create",
      draftText: jsonText(customHarnessStarter()),
      metadata: { documentRevision },
    });
  }

  async function duplicateHarnessForEditor(entry: HarnessConfigEntry) {
    setHarnessActionStatus("Preparing an independent copy…");
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const result = await duplicateHarness(baseUrl, entry.id);
      const duplicateEntry: HarnessConfigEntry = {
        ...entry,
        ...(result.definition as Partial<HarnessConfigEntry>),
        id: result.proposedId,
        name: result.proposedName,
        origin: "custom",
        documentRevision: result.documentRevision,
      };
      setHarnessEditor({
        mode: "create",
        draftText: jsonText(result.definition),
        metadata: {
          entry: duplicateEntry,
          documentRevision: result.documentRevision,
          duplicateSourceId: entry.id,
        },
      });
      setHarnessActionStatus(null);
    } catch (error) {
      setHarnessActionStatus(error instanceof Error ? error.message : "Couldn't duplicate coding tool.");
    }
  }

  async function handleHarnessSaved(result: HarnessMutationResponse) {
    try {
      await onRefreshHarnesses();
      setHarnessEditor(null);
      setHarnessActionStatus("Configuration saved.");
    } catch {
      setHarnessActionStatus("Configuration saved, but the coding tool list could not be refreshed.");
    }
    if (result.harness.origin === "custom") setActiveSection("tools");
  }

  async function handleHarnessDeleted() {
    try {
      await onRefreshHarnesses();
      setHarnessEditor(null);
      setHarnessActionStatus("Coding tool deleted.");
    } catch {
      setHarnessActionStatus("Coding tool deleted, but the coding tool list could not be refreshed.");
    }
  }

  async function handleHarnessRevisionConflict() {
    try {
      const refreshed = await onRefreshHarnesses();
      setHarnessEditor((current) => {
        if (!current) return current;
        const id = current.metadata.entry?.id;
        const entry = id ? refreshed.harnesses.find((candidate) => candidate.id === id) : undefined;
        return {
          ...current,
          metadata: {
            ...current.metadata,
            documentRevision: refreshed.documentRevision,
            ...(entry ? { entry } : {}),
          },
        };
      });
      setHarnessActionStatus("The latest configuration is loaded for comparison. Your draft is still here.");
    } catch {
      setHarnessActionStatus("The configuration changed elsewhere. Your draft is still here; refresh and retry.");
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
          <button className="settings-icon-button" type="button" onClick={discardDraftsAndClose} aria-label="Close settings">
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
                onClick={() => {
                  // Leaving the Hotkeys tab mid-capture would otherwise leave the
                  // window-level keydown listener armed with no visible target row,
                  // silently assigning the next keystroke typed anywhere else.
                  setCapturing(null);
                  setActiveSection(item.key);
                }}
              >
                {item.label}
              </button>
            ))}
          </nav>

          <div className="settings-content">
            {activeSection === "tools" && (
              <div className="settings-section">
                {harnessEditor ? (
                  <HarnessConfigEditor
                    mode={harnessEditor.mode}
                    draftText={harnessEditor.draftText}
                    metadata={harnessEditor.metadata}
                    onCancel={() => setHarnessEditor(null)}
                    onSaved={handleHarnessSaved}
                    onDeleted={handleHarnessDeleted}
                    onRevisionConflict={handleHarnessRevisionConflict}
                  />
                ) : (<>
                  <h3>Active coding tools</h3>
                  <p className="settings-section-copy">
                    Select which coding tools are available in this workspace. Shell is always available.
                  </p>

                  <div className="settings-config-list">
                    {toolHarnesses.map((h) => {
                      const display = toolDisplayState(h);
                      const launch = h.launch.kind === "command-template" ? h.launch : undefined;
                      const isCommandTemplate = launch !== undefined;
                      const hasCustomPath = launch !== undefined && looksAbsolute(launch.command);
                      const rowKey = integrationKeyForHarness(h);
                      const rowStatus = integrationStatuses[h.id];
                      const rowKeyId = rowKey ? `${rowKey.adapterId}/${rowKey.targetId}` : null;
                      const draftDiverged = activeDraft.includes(h.id) !== activeHarnessIds.includes(h.id);
                      // Actionability is computed from the persisted selection
                      // (what reconcile will execute against), not the draft.
                      const reconcileActionable = h.integration !== null
                        && rowKeyId !== null
                        && rowStatus !== undefined
                        && isReconcileActionable(activeHarnessIds.includes(h.id), rowStatus);

                      return (
                        <div key={h.id} className="settings-config-item-row">
                          <div className="settings-config-item-header">
                            <div className="settings-config-item">
                              <HarnessIcon tool={h.name} size={16} />
                              <span>{h.name}</span>
                              <span className={`harness-origin-badge harness-origin-badge--${h.origin}`}>{h.origin}</span>
                              {hasCustomPath && (
                                <span className="settings-config-custom-path-tell" title="This coding tool launches from a custom path.">
                                  custom path
                                </span>
                              )}
                              {/* onDetectionChanged=refreshDetection stays
                                  coupled through refreshGeneration-driven
                                  reloads and the shared refreshDetection path. */}
                              <HarnessDetectionStatus harnessId={h.id}
                                integrationKey={integrationKeyForHarness(h) ?? undefined}
                                refreshGeneration={detectionGenerations[h.id] ?? 0}
                              />
                            </div>
                            <div className="settings-config-item-header-actions">
                              {reconcileActionable && rowKey && (
                                <button
                                  type="button"
                                  onClick={() => void reconcileIntegrationHandler(rowKey)}
                                  disabled={toolsSaveInProgress || reconcileInProgressKey !== null || draftDiverged}
                                  title={draftDiverged ? "Save coding tool changes first." : undefined}
                                >
                                  {reconcileInProgressKey === rowKeyId ? "Reconciling..." : "Reconcile"}
                                </button>
                              )}
                              {h.origin !== "builtin" || h.id !== "generic-shell" ? (
                                <button type="button" onClick={() => openHarnessEditor(h)}>
                                  {h.origin === "override" ? "Edit override" : "View config"}
                                </button>
                              ) : null}
                              {h.id !== "generic-shell" && (
                                <button type="button" onClick={() => void duplicateHarnessForEditor(h)}>
                                  Duplicate
                                </button>
                              )}
                              {isCommandTemplate && (
                                <button
                                  type="button"
                                  onClick={() => toggleCommandPathExpanded(h.id)}
                                  aria-expanded={Boolean(expandedCommandPaths[h.id])}
                                >
                                  {expandedCommandPaths[h.id] ? "Hide path ▾" : "Path ▸"}
                                </button>
                              )}
                              <Toggle
                                checked={activeDraft.includes(h.id)}
                                onChange={() => toggleHarness(h.id)}
                                ariaLabel={h.name}
                                disabled={toolsSaveInProgress}
                                visualState={display.appearance}
                                statusDescription={display.description}
                                statusGlyph={display.glyph}
                                tooltip={display.tooltip}
                              />
                            </div>
                          </div>
                          {isCommandTemplate && (
                            // Stays mounted while hidden (native `hidden` attribute, not
                            // conditional rendering) so an in-progress, unsaved path edit
                            // survives collapsing the disclosure instead of being discarded.
                            <div hidden={!expandedCommandPaths[h.id]}>
                              <HarnessCommandPathControl
                                harnessId={h.id}
                                harnessName={h.name}
                                harness={h}
                                disabled={toolsSaveInProgress}
                                documentRevision={documentRevision}
                                onChanged={refreshDetection}
                              />
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>

                  <div className="settings-config-footer">
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={saveActiveHarnessesHandler}
                      disabled={toolsSaveInProgress || reconcileInProgressKey !== null}
                    >
                      {toolsSaveInProgress ? "Saving..." : "Save"}
                    </Button>
                    <Button variant="primary" size="sm" onClick={openNewHarnessEditor}>
                      Add custom coding tool
                    </Button>
                    {activeSaveStatus && (
                      <span className="settings-config-status">
                        {activeSaveStatus}
                      </span>
                    )}
                    {harnessActionStatus && (
                      <span className="settings-config-status">{harnessActionStatus}</span>
                    )}
                  </div>
                </>)}
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
                  Choose the one provider and model Peon should use. Selecting a provider verifies it before models are shown.
                </p>

                <div className="provider-list">
                  <div className="provider-card">
                    <div className="provider-label">Peon provider</div>
                    <select className="provider-model-select" value={peonSelection.provider} onChange={(event) => {
                      const provider = event.target.value as ProviderId;
                      const next = { provider, model: "", ...(provider === "ollama" ? { ollamaBaseUrl: providerDraft.ollamaBaseUrl } : {}) };
                      setPeonSelection(next);
                      setUnavailablePeonProvider(null);
                      setPeonLocallyApplied(false);
                      scheduleVerifyPeonSelection(next, true);
                    }}>
                      {!selectedProviderIsAvailable && <option value={peonSelection.provider} disabled>{peonSelection.provider} (unavailable)</option>}
                      {peonProviders.map((provider) => <option key={provider} value={provider}>{providerLabels.get(provider) ?? provider}</option>)}
                    </select>
                    {unavailablePeonProvider && (
                      <div role="alert" className="provider-verify-status">
                        The previously selected provider “{unavailablePeonProvider}” is no longer available. Choose another provider; OrkWorks will not switch automatically.
                      </div>
                    )}
                    {peonSelection.provider === "ollama" && <Input value={peonSelection.ollamaBaseUrl ?? ""} onChange={(event) => {
                      const next = { ...peonSelection, ollamaBaseUrl: event.target.value };
                      setPeonSelection(next);
                      setPeonLocallyApplied(false);
                      setPeonVerification(null);
                      setPeonError(null);
                      scheduleVerifyPeonSelection(next);
                    }} placeholder="http://127.0.0.1:11434" />}
                    <div role="status" aria-live="polite" className="provider-verify-status">{peonBusy ? <><span className="provider-verify-spinner" aria-hidden="true" />Verifying provider…<span aria-hidden="true">{peonBusyElapsedSeconds > 0 ? ` ${peonBusyElapsedSeconds}s elapsed` : ""}{peonBusyElapsedSeconds >= 15 ? " — can take up to a minute" : ""}</span></> : peonVerification?.ok ? "Provider verified." : peonError ?? "Choose a provider to verify it."}</div>
                    <div className="provider-label">Peon model</div>
                    <select className="provider-model-select" disabled={manualModelOverride || !peonVerification?.ok} value={manualModelOverride ? "" : peonSelection.model} onChange={(event) => { setPeonSelection({ ...peonSelection, model: event.target.value }); setPeonLocallyApplied(false); }}>
                      <option value="">Select a verified model</option>
                      {(peonVerification?.models ?? []).map((model) => <option key={model} value={model}>{model}</option>)}
                    </select>
                    <label><input type="checkbox" checked={manualModelOverride} onChange={(event) => { setManualModelOverride(event.target.checked); setPeonLocallyApplied(false); if (!event.target.checked) setPeonSelection({ ...peonSelection, model: "" }); }} /> Enter model manually</label>
                    {manualModelOverride && <input className="provider-model-select" type="text" value={peonSelection.model} onChange={(event) => { setPeonSelection({ ...peonSelection, model: event.target.value }); setPeonLocallyApplied(false); }} placeholder="Enter model name" />}
                    <datalist id="peon-selected-models">{(peonVerification?.models ?? []).map((model) => <option key={model} value={model} />)}</datalist>
                    <div className="provider-label">Applied Peon configuration</div>
                    <div role="status">{peonApplied ? `${peonApplied.provider} · ${peonApplied.model}${peonApplied.ollamaBaseUrl ? ` · ${peonApplied.ollamaBaseUrl}` : ""}` : "No staged configuration applied."}</div>
                    <Button variant="secondary" size="sm" disabled={peonBusy || !peonVerification?.ok || !peonSelection.model.trim()} onClick={() => void applyPeonSelection()}>Apply</Button>
                    <Button variant="primary" size="sm" disabled={peonBusy || !peonApplyMatches} onClick={() => void savePeonSelection()}>Save</Button>
                  </div>
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
                      <div className="hotkey-row-label">
                        <div className="hotkey-label">{row.label}</div>
                        {errors[row.action]?.map((error) => (
                          <div className="hotkey-error" key={error}>{error}</div>
                        ))}
                      </div>
                      <div className="hotkey-row-controls">
                        <kbd className="hotkey-value">
                          {capturing === row.action ? "Press shortcut..." : draft[row.action] ?? "Unset"}
                        </kbd>
                        <Button variant="ghost" size="sm" onClick={() => setCapturing(row.action)}>Edit</Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            setDraft((current) => ({ ...current, [row.action]: defaultHotkeys[row.action] }));
                          }}
                        >
                          Reset
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>

                <div className="settings-subsection-actions">
                  <Button variant="secondary" size="sm" onClick={restoreHotkeyDefaults}>Restore defaults</Button>
                  <Button variant="ghost" size="sm" onClick={cancelHotkeyChanges} disabled={!hotkeysDirty}>Cancel</Button>
                  <Button variant="primary" size="sm" onClick={() => void saveHotkeysHandler()} disabled={!hotkeysDirty || capturing !== null}>
                    Save
                  </Button>
                  {hotkeySaveStatus && (
                    <span className={`settings-subsection-status ${hotkeySaveStatus === "Saved" ? "settings-subsection-status--ok" : ""}`}>
                      {hotkeySaveStatus}
                    </span>
                  )}
                </div>
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
      </section>
    </div>
  );
}

function isBareKey(event: KeyboardEvent): boolean {
  return !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function peonSelectionFromProviders(providers: ProviderSettings): PeonSelection {
  return providers.peonSelection ?? {
    provider: "ollama",
    model: providers.peonModel ?? "",
    ollamaBaseUrl: providers.ollamaBaseUrl,
  };
}

function deepEqual(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}
