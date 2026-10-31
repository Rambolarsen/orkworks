import { useEffect, useRef, useState } from "react";
import { Pencil } from "lucide-react";
import type { SessionTitleRequest } from "../sessionTitleTypes";

export default function SessionTitleEditor({ sessionId, title, workspaceIdentity, onUpdated }: {
  sessionId: string;
  title: string;
  workspaceIdentity: string;
  onUpdated?: () => void;
}) {
  const [display, setDisplay] = useState(title);
  const [draft, setDraft] = useState(title);
  const [editing, setEditing] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const mounted = useRef(true);
  const busy = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => { setDisplay(title); }, [title]);
  const valid = draft.trim().length > 0 && Array.from(draft.trim()).length <= 100 && !/[\u0000-\u001f\u007f-\u009f]/.test(draft);

  async function save(action: "set" | "automatic") {
    if (busy.current || (action === "set" && !valid)) return;
    busy.current = true;
    setPending(true);
    setError("");
    const request: SessionTitleRequest = action === "set"
      ? { sessionId, workspaceIdentity, action, title: draft.trim() }
      : { sessionId, workspaceIdentity, action };
    try {
      const result = await window.orkworks.sessionTitle(request);
      if (!mounted.current) return;
      setDisplay(result.title);
      setEditing(false);
      onUpdated?.();
    } catch (error) {
      if (mounted.current) setError(error instanceof Error ? error.message : "Could not update session title.");
    } finally {
      busy.current = false;
      if (mounted.current) setPending(false);
    }
  }

  return <div className="session-title-editor">
    <div className="session-detail-label">Title</div>
    {editing ? <form onSubmit={event => { event.preventDefault(); void save("set"); }}
      onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); if (!pending) setEditing(false); } }}>
      <input aria-label="Session title" value={draft} onChange={event => setDraft(event.target.value)} autoFocus disabled={pending} />
      <div className="session-title-actions">
        <button type="submit" disabled={pending || !valid}>{pending ? "Saving…" : "Save"}</button>
        <button type="button" disabled={pending} onClick={() => setEditing(false)}>Cancel</button>
        <button type="button" disabled={pending} onClick={() => { void save("automatic"); }}>Use automatic title</button>
      </div>
      <p className="session-title-help">Up to 100 characters. Your title stays until you use automatic naming.</p>
      {error && <p role="alert">{error}</p>}
    </form> : <div className="session-title-current">
      <span>{display}</span>
      <button type="button" aria-label="Rename session" title="Rename session" onClick={() => { setDraft(display); setError(""); setEditing(true); }}><Pencil size={14} aria-hidden="true" /></button>
    </div>}
  </div>;
}
