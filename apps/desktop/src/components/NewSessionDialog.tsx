import { useCallback, useEffect, useRef, useState } from "react";
import type { HarnessConfig, CreateSessionOptions } from "../harnessTypes";
import type { ProviderModelsResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { canStartNewSession, syncDraftWithHarnesses } from "../newSessionDialogState";

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

function resolveInitialDraft(harnesses: HarnessConfig[]) {
  const defaultHarness = harnesses[0] ?? null;
  const savedHarnessId = localStorage.getItem(LS_HARNESS_KEY);
  const savedModel = localStorage.getItem(LS_MODEL_KEY);

  if (savedHarnessId && harnesses.some((h) => h.id === savedHarnessId)) {
    return {
      harnessId: savedHarnessId,
      model: savedModel ?? "",
    };
  }

  return {
    harnessId: defaultHarness?.id ?? "",
    model: defaultHarness?.defaultModel ?? "",
  };
}

export default function NewSessionDialog({ harnesses, providerRuntime, onConfirm, onCancel }: NewSessionDialogProps) {
  const [draft, setDraft] = useState(() => resolveInitialDraft(harnesses));
  const [initialPrompt, setInitialPrompt] = useState("");
  const [models, setModels] = useState<string[]>([]);
  const harnessSelectRef = useRef<HTMLSelectElement>(null);

  useEffect(() => {
    harnessSelectRef.current?.focus();
  }, []);

  useEffect(() => {
    function onDocKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    document.addEventListener("keydown", onDocKeyDown);
    return () => document.removeEventListener("keydown", onDocKeyDown);
  }, [onCancel]);

  useEffect(() => {
    setDraft((current) => syncDraftWithHarnesses(current, harnesses));
  }, [harnesses]);

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
    const h = harnesses.find((h) => h.id === id);
    setDraft({
      harnessId: id,
      model: h?.defaultModel ?? "",
    });
  }

  const handleConfirm = useCallback(() => {
    const harnessId = draft.harnessId || undefined;
    const model = draft.model.trim() || undefined;
    if (harnessId) localStorage.setItem(LS_HARNESS_KEY, harnessId);
    if (model) localStorage.setItem(LS_MODEL_KEY, model);
    else localStorage.removeItem(LS_MODEL_KEY);
    onConfirm({ harnessId, model, initialPrompt: initialPrompt.trim() || undefined });
  }, [draft.harnessId, draft.model, initialPrompt, onConfirm]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      if (e.target instanceof HTMLTextAreaElement) return;
      if (canStartNewSession(harnesses, draft.harnessId)) {
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
          <h2 id="new-session-title">New Session</h2>
        </header>

        <div className="new-session-body">
          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-harness">Provider</label>
            <select
              ref={harnessSelectRef}
              id="nsd-harness"
              className="new-session-select"
              value={draft.harnessId}
              onChange={(e) => handleHarnessChange(e.target.value)}
              disabled={harnesses.length === 0}
            >
              {harnesses.length === 0 ? (
                <option value="">Default shell</option>
              ) : (
                harnesses.map((h) => {
                  const state = providerRuntime?.providers.find((p) => p.id === h.id)?.effectiveState;
                  return <option key={h.id} value={h.id}>{harnessLabel(h.name, state)}</option>;
                })
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
              placeholder="Optional — sent to the provider on start"
              rows={3}
            />
          </div>
        </div>

        <footer className="new-session-footer">
          <button type="button" className="new-session-cancel" onClick={onCancel}>Cancel</button>
          <button
            type="button"
            className="new-session-confirm"
            onClick={handleConfirm}
            disabled={!canStartNewSession(harnesses, draft.harnessId)}
          >
            Start
          </button>
        </footer>
      </section>
    </div>
  );
}
