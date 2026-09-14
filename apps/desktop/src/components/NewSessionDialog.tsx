import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { HarnessConfig, CreateSessionOptions, IntegrationStatusResult } from "../harnessTypes";
import type { ProviderModelsResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { getHarnessDetectionStatus } from "../harnessDetection";
import { canStartNewSession, detectedSelectableHarnesses, syncDraftWithHarnesses, type NewSessionDraft } from "../newSessionDialogState";

interface NewSessionDialogProps {
  harnesses: HarnessConfig[];
  providerRuntime: ProviderRuntimeResponse | null;
  onConfirm: (opts: CreateSessionOptions) => void;
  onCancel: () => void;
}

function harnessLabel(name: string, state: string | undefined): string {
  if (!state || state === "healthy" || state === "unknown") return name;
  return `${name} (${state})`;
}

const LS_HARNESS_KEY = "orkworks-new-session-harnessId";
const LS_MODEL_KEY = "orkworks-new-session-model";

function getSavedDraft(): NewSessionDraft | null {
  const savedHarnessId = localStorage.getItem(LS_HARNESS_KEY);
  const savedModel = localStorage.getItem(LS_MODEL_KEY);

  if (!savedHarnessId) return null;
  return {
    harnessId: savedHarnessId,
    model: savedModel ?? "",
  };
}

function resolveInitialDraft(harnesses: HarnessConfig[]) {
  const savedDraft = getSavedDraft();
  if (savedDraft && harnesses.some((harness) => harness.id === savedDraft.harnessId && !harness.retired)) {
    return savedDraft;
  }
  return syncDraftWithHarnesses(
    { harnessId: "", model: "" },
    harnesses,
  );
}

export default function NewSessionDialog({ harnesses, providerRuntime, onConfirm, onCancel }: NewSessionDialogProps) {
  const harnessesKey = useMemo(
    () => JSON.stringify(harnesses.map((harness) => [harness.id, harness.name, harness.retired, harness.launch, harness.integration])),
    [harnesses],
  );
  const [detectionStatuses, setDetectionStatuses] = useState<Record<string, IntegrationStatusResult | undefined>>({});
  const [detectionComplete, setDetectionComplete] = useState(false);
  const detectedHarnessIds = useMemo(() => new Set([
    "generic-shell",
    ...Object.entries(detectionStatuses)
      .filter(([, result]) => result?.ok === true && result.status.toolDetected)
      .map(([harnessId]) => harnessId),
  ]), [detectionStatuses]);
  const selectable = useMemo(
    () => detectedSelectableHarnesses(harnesses, detectedHarnessIds),
    [detectedHarnessIds, harnessesKey],
  );
  const [draft, setDraft] = useState(() => resolveInitialDraft(harnesses));
  const [initialPrompt, setInitialPrompt] = useState("");
  const [models, setModels] = useState<string[]>([]);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const harnessSelectRef = useRef<HTMLSelectElement>(null);
  const confirmationGeneration = useRef(0);
  const handleCancel = useCallback(() => {
    confirmationGeneration.current += 1;
    onCancel();
  }, [onCancel]);
  const pendingHarness = !detectionComplete && draft.harnessId
    ? harnesses.find((harness) => harness.id === draft.harnessId && !harness.retired && !selectable.some((entry) => entry.id === harness.id))
    : undefined;

  useEffect(() => {
    let cancelled = false;
    setDetectionComplete(false);
    setDetectionStatuses({});
    void Promise.all(
      harnesses
        .filter((harness) => harness.id !== "generic-shell" && !harness.retired)
        .map(async (harness) => {
          try {
            return [harness.id, await getHarnessDetectionStatus(harness)] as const;
          } catch (error) {
            return [harness.id, {
              ok: false as const,
              error: error instanceof Error ? error.message : "Coding tool detection unavailable.",
            }] as const;
          }
        }),
    ).then((entries) => {
      if (!cancelled) {
        setDetectionStatuses(Object.fromEntries(entries));
        setDetectionComplete(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [harnessesKey]);

  useEffect(() => {
    harnessSelectRef.current?.focus();
  }, []);

  useEffect(() => {
    function onDocKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        handleCancel();
      }
    }
    document.addEventListener("keydown", onDocKeyDown);
    return () => document.removeEventListener("keydown", onDocKeyDown);
  }, [handleCancel]);

  useEffect(() => {
    if (!detectionComplete) return;
    setDraft((current) => selectable.length === 0
      ? { harnessId: "", model: "" }
      : syncDraftWithHarnesses(current, selectable, getSavedDraft()));
  }, [detectionComplete, selectable]);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const resp: ProviderModelsResponse = await window.orkworks.getProviderModels(draft.harnessId);
        if (!cancelled) setModels(resp.models);
      } catch {
        if (!cancelled) setModels([]);
      }
    }
    if (draft.harnessId) load();
    else setModels([]);
    return () => { cancelled = true; };
  }, [draft.harnessId]);

  function handleHarnessChange(id: string) {
    const h = selectable.find((h) => h.id === id);
    setDraft({
      harnessId: id,
      model: h?.defaultModel ?? "",
    });
  }

  const handleConfirm = useCallback(async () => {
    if (confirmBusy) return;
    if (!canStartNewSession(selectable, draft.harnessId, detectedHarnessIds)) return;
    const selectedHarness = selectable.find((harness) => harness.id === draft.harnessId);
    const generation = confirmationGeneration.current;
    setConfirmBusy(true);
    try {
      if (selectedHarness && selectedHarness.id !== "generic-shell") {
        try {
          const freshStatus = await getHarnessDetectionStatus(selectedHarness);
          if (generation !== confirmationGeneration.current) return;
          setDetectionStatuses((current) => ({ ...current, [selectedHarness.id]: freshStatus }));
          if (freshStatus.ok !== true || !freshStatus.status.toolDetected) return;
        } catch (error) {
          if (generation !== confirmationGeneration.current) return;
          setDetectionStatuses((current) => ({
            ...current,
            [selectedHarness.id]: {
              ok: false,
              error: error instanceof Error ? error.message : "Coding tool detection unavailable.",
            },
          }));
          return;
        }
      }
      if (generation !== confirmationGeneration.current) return;
      const harnessId = draft.harnessId || undefined;
      const model = draft.model.trim() || undefined;
      if (harnessId) localStorage.setItem(LS_HARNESS_KEY, harnessId);
      if (model) localStorage.setItem(LS_MODEL_KEY, model);
      else localStorage.removeItem(LS_MODEL_KEY);
      onConfirm({ harnessId, model, initialPrompt: initialPrompt.trim() || undefined });
    } finally {
      setConfirmBusy(false);
    }
  }, [confirmBusy, detectedHarnessIds, draft.harnessId, draft.model, initialPrompt, onConfirm, selectable]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      if (e.target instanceof HTMLTextAreaElement) return;
      if (canStartNewSession(selectable, draft.harnessId, detectedHarnessIds)) {
        e.preventDefault();
        handleConfirm();
      }
      return;
    }
    if (e.key === "Tab") {
      const container = e.currentTarget as HTMLElement;
      const focusable = container.querySelectorAll<HTMLElement>(
        "select:not([disabled]), input:not([disabled]), textarea:not([disabled]), button:not([disabled])"
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  return (
    <div className="new-session-backdrop" role="presentation" onKeyDown={handleKeyDown}>
      <section className="new-session-dialog" role="dialog" aria-modal="true" aria-labelledby="new-session-title">
        <header className="new-session-header">
          <h2 id="new-session-title">New agent session</h2>
        </header>

        <div className="new-session-body">
          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-harness">Coding tool</label>
            <select
              ref={harnessSelectRef}
              id="nsd-harness"
              className="new-session-select"
              value={draft.harnessId}
              onChange={(e) => handleHarnessChange(e.target.value)}
              disabled={confirmBusy || (selectable.length === 0 && !pendingHarness)}
            >
              {selectable.length === 0 && !pendingHarness ? (
                <option value="">Default shell</option>
              ) : (
                <>
                {pendingHarness && (
                  <option value={pendingHarness.id} disabled>
                    {pendingHarness.name} (checking availability…)
                  </option>
                )}
                {selectable.map((h) => {
                  const state = providerRuntime?.providers.find((p) => p.id === h.id)?.effectiveState;
                  return <option key={h.id} value={h.id}>{harnessLabel(h.name, state)}</option>;
                })}
                </>
              )}
            </select>
          </div>

          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-model">Model</label>
            <input
              key={draft.harnessId}
              id="nsd-model"
              className="new-session-input"
              type="text"
              list="nsd-model-suggestions"
              defaultValue={draft.model}
              onChange={(e) => setDraft((current) => ({ ...current, model: e.target.value }))}
              placeholder="default"
            />
            <datalist id="nsd-model-suggestions">
              {models.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </div>

          <div className="new-session-row new-session-row--prompt">
            <label className="new-session-label" htmlFor="nsd-prompt">Initial prompt</label>
            <textarea
              id="nsd-prompt"
              className="new-session-textarea"
              value={initialPrompt}
              onChange={(e) => setInitialPrompt(e.target.value)}
              placeholder="Optional - sent when the agent session starts"
              rows={3}
            />
          </div>
        </div>

        <footer className="new-session-footer">
          <button type="button" className="new-session-cancel" onClick={handleCancel}>Cancel</button>
          <button
            type="button"
            className="new-session-confirm"
            onClick={handleConfirm}
            disabled={confirmBusy || !canStartNewSession(selectable, draft.harnessId, detectedHarnessIds)}
          >
            Start
          </button>
        </footer>
      </section>
    </div>
  );
}
