import { useEffect, useState } from "react";
import type { HarnessConfig, CreateSessionOptions } from "../harnessTypes";
import { canStartNewSession, syncDraftWithHarnesses } from "../newSessionDialogState";

interface NewSessionDialogProps {
  harnesses: HarnessConfig[];
  onConfirm: (opts: CreateSessionOptions) => void;
  onCancel: () => void;
}

export default function NewSessionDialog({ harnesses, onConfirm, onCancel }: NewSessionDialogProps) {
  const defaultHarness = harnesses[0] ?? null;
  const [draft, setDraft] = useState(() => ({
    harnessId: defaultHarness?.id ?? "",
    model: defaultHarness?.defaultModel ?? "",
  }));
  const [initialPrompt, setInitialPrompt] = useState("");

  useEffect(() => {
    setDraft((current) => syncDraftWithHarnesses(current, harnesses));
  }, [harnesses]);

  function handleHarnessChange(id: string) {
    const h = harnesses.find((h) => h.id === id);
    setDraft({
      harnessId: id,
      model: h?.defaultModel ?? "",
    });
  }

  function handleConfirm() {
    onConfirm({
      harnessId: draft.harnessId || undefined,
      model: draft.model.trim() || undefined,
      initialPrompt: initialPrompt.trim() || undefined,
    });
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }

  return (
    <div className="new-session-backdrop" role="presentation" onKeyDown={handleKeyDown}>
      <section className="new-session-dialog" role="dialog" aria-modal="true" aria-labelledby="new-session-title">
        <header className="new-session-header">
          <h2 id="new-session-title">New Session</h2>
        </header>

        <div className="new-session-body">
          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-harness">Harness</label>
            <select
              id="nsd-harness"
              className="new-session-select"
              value={draft.harnessId}
              onChange={(e) => handleHarnessChange(e.target.value)}
              disabled={harnesses.length === 0}
            >
              {harnesses.length === 0 ? (
                <option value="">Default shell</option>
              ) : (
                harnesses.map((h) => (
                  <option key={h.id} value={h.id}>{h.name}</option>
                ))
              )}
            </select>
          </div>

          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-model">Model</label>
            <input
              id="nsd-model"
              className="new-session-input"
              type="text"
              value={draft.model}
              onChange={(e) => setDraft((current) => ({ ...current, model: e.target.value }))}
              placeholder="default"
            />
          </div>

          <div className="new-session-row new-session-row--prompt">
            <label className="new-session-label" htmlFor="nsd-prompt">Initial prompt</label>
            <textarea
              id="nsd-prompt"
              className="new-session-textarea"
              value={initialPrompt}
              onChange={(e) => setInitialPrompt(e.target.value)}
              placeholder="Optional — sent to the harness on start"
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
