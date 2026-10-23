import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { HarnessConfig, CreateSessionOptions, IntegrationStatusResult } from "../harnessTypes";
import type { ProviderModelsResponse } from "../providerTypes";
import type { ProviderRuntimeResponse } from "../api";
import { getHarnessDetectionStatus } from "../harnessDetection";
import { canStartNewSession, detectedSelectableHarnesses, syncDraftWithHarnesses, type NewSessionDraft } from "../newSessionDialogState";

import { IMAGE_ACCEPT, prepareImageBatch, releaseImages, type DraftImage } from "../sessionImages";

interface NewSessionDialogProps {
  workspacePath?: string;
  harnesses: HarnessConfig[];
  providerRuntime: ProviderRuntimeResponse | null;
  onConfirm: (opts: CreateSessionOptions) => Promise<void>;
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

export default function NewSessionDialog({ workspacePath = "", harnesses, providerRuntime, onConfirm, onCancel }: NewSessionDialogProps) {
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
  const [images, setImages] = useState<DraftImage[]>([]);
  const imagesRef = useRef<DraftImage[]>([]);
  const [imageBusy, setImageBusy] = useState(false);
  const imageBusyRef = useRef(false);
  const [imageError, setImageError] = useState("");
  const [imageTools, setImageTools] = useState<string[]>([]);
  const fileInput = useRef<HTMLInputElement>(null);
  const [models, setModels] = useState<string[]>([]);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const promptRef = useRef<HTMLTextAreaElement>(null);
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
    window.orkworks.getBackendUrl().then((url) => fetch(`${url}/session-image-context`))
      .then(async (response) => response.ok ? response.json() : null)
      .then((capability) => { if (!cancelled && capability?.version === 1 && Array.isArray(capability.supportedHarnesses)) setImageTools(capability.supportedHarnesses.filter((id: unknown): id is string => typeof id === "string")); })
      .catch(() => {});
    return () => { cancelled = true; confirmationGeneration.current += 1; releaseImages(imagesRef.current); imagesRef.current = []; };
  }, []);

  async function addImages(files: File[]) {
    if (!files.length || imageBusyRef.current || confirmBusy) return;
    imageBusyRef.current = true; setImageBusy(true); setImageError("");
    const generation = confirmationGeneration.current;
    try {
      const added = await prepareImageBatch(imagesRef.current, files);
      if (generation !== confirmationGeneration.current) { releaseImages(added); return; }
      imagesRef.current = [...imagesRef.current, ...added]; setImages(imagesRef.current);
    } catch (error) {
      if (generation === confirmationGeneration.current) setImageError(error instanceof Error ? error.message : "Cannot attach these images.");
    } finally {
      imageBusyRef.current = false;
      if (generation === confirmationGeneration.current) setImageBusy(false);
    }
  }
  const imageUnsupported = images.length > 0 && !imageTools.includes(draft.harnessId);

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
    promptRef.current?.focus();
    return () => { confirmationGeneration.current += 1; };
  }, []);

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
    if (confirmBusy || imageBusyRef.current || (imagesRef.current.length > 0 && !imageTools.includes(draft.harnessId))) return;
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
      setImageError("");
      await onConfirm({ harnessId, model, initialPrompt: initialPrompt.trim() || undefined,
        ...(imagesRef.current.length ? {imageWorkspacePath: workspacePath, initialImages: imagesRef.current.map(({name,mimeType,dataBase64}) => ({name,mimeType,dataBase64}))} : {}),
      });
    } catch (error) {
      if (generation === confirmationGeneration.current) setImageError(error instanceof Error ? error.message : "Could not start this session.");
    } finally {
      if (generation === confirmationGeneration.current) setConfirmBusy(false);
    }
  }, [confirmBusy, detectedHarnessIds, draft.harnessId, draft.model, imageTools, initialPrompt, onConfirm, selectable, workspacePath]);

  function handleKeyDown(e: React.KeyboardEvent) {
    // Buttons own Enter so keyboard attachment/removal/cancellation cannot submit.
    if (e.key === "Enter" && e.target instanceof HTMLElement && e.target.closest("button")) return;
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey && !e.nativeEvent.isComposing) {
      if (canStartNewSession(selectable, draft.harnessId, detectedHarnessIds)) {
        e.preventDefault();
        handleConfirm();
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      handleCancel();
    }
  }

  return (
    <section className="new-session-composer" aria-labelledby="new-session-title" onKeyDown={handleKeyDown}
      onDragOver={(event) => { if (event.dataTransfer.types.includes("Files")) { event.preventDefault(); event.dataTransfer.dropEffect = "copy"; } }}
      onDrop={(event) => { event.preventDefault(); if (confirmBusy || imageBusyRef.current) return; const files = Array.from(event.dataTransfer.files); if (files.length) void addImages(files); else setImageError("Drop PNG, JPEG, or WebP image files."); }}
      onPaste={(event) => {
        const files = Array.from(event.clipboardData.items).filter((item) => item.kind === "file").map((item) => item.getAsFile()).filter((file): file is File => file !== null);
        if (!files.length) return;
        event.preventDefault();
        if (confirmBusy || imageBusyRef.current) return;
        const text = event.clipboardData.getData("text/plain");
        if (text) { const prompt = promptRef.current; if (prompt && event.target === prompt) { const start = prompt.selectionStart; const end = prompt.selectionEnd; setInitialPrompt((current) => current.slice(0,start) + text + current.slice(end)); } else setInitialPrompt((current) => current + text); }
        void addImages(files);
      }}>
      <div className="new-session-composer-card">
        <header className="new-session-header">
          <h2 id="new-session-title">New agent session</h2>
        </header>

        <div className="new-session-body">
          <div className="new-session-row new-session-row--prompt">
            <label className="new-session-label" htmlFor="nsd-prompt">Initial prompt</label>
            <textarea
              ref={promptRef}
              id="nsd-prompt"
              className="new-session-textarea"
              value={initialPrompt}
              onChange={(e) => setInitialPrompt(e.target.value)}
              placeholder="Optional - sent when the agent session starts"
              rows={7}
              disabled={confirmBusy}
            />
          </div>
          <div className="new-session-images">
            <input ref={fileInput} type="file" accept={IMAGE_ACCEPT} multiple hidden aria-label="Choose image files" onChange={(event) => { const files = Array.from(event.target.files ?? []); event.target.value = ""; void addImages(files); }} />
            <button type="button" className="new-session-attach" disabled={confirmBusy || imageBusy} onClick={() => fileInput.current?.click()}>Attach images</button>
            <span className="new-session-image-hint">Paste or drop images · PNG, JPEG, WebP · 5 MiB each</span>
            {imageBusy && <span role="status">Preparing images…</span>}
            {images.length > 0 && <ul className="new-session-image-list" aria-label="Attached images">{images.map((image) => <li key={image.id}>
              <img src={image.previewUrl} alt={image.name} /><span title={image.name}>{image.name}</span>
              <button type="button" aria-label={`Remove ${image.name}`} disabled={confirmBusy || imageBusy} onClick={() => { releaseImages([image]); imagesRef.current = imagesRef.current.filter((entry) => entry.id !== image.id); setImages(imagesRef.current); }}>Remove</button>
            </li>)}</ul>}
            {imageUnsupported && <p role="status">Image context is not verified for this coding tool/version/platform. Choose a supported tool or remove the images.</p>}
            {imageError && <p className="new-session-image-error" role="alert">{imageError}</p>}
          </div>
          <div className="new-session-controls">
            <div className="new-session-row">
              <label className="new-session-label" htmlFor="nsd-harness">Coding tool</label>
              <select
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
                disabled={confirmBusy}
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
          </div>
        </div>

        <footer className="new-session-footer">
          <button type="button" className="new-session-cancel" onClick={handleCancel}>Cancel</button>
          <button
            type="button"
            className="new-session-confirm"
            onClick={handleConfirm}
            disabled={confirmBusy || imageBusy || imageUnsupported || !canStartNewSession(selectable, draft.harnessId, detectedHarnessIds)}
          >
            {confirmBusy ? "Starting…" : "Start"}
          </button>
        </footer>
      </div>
    </section>
  );
}
