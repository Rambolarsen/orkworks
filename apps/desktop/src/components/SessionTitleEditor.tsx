import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Pencil } from "lucide-react";
import type { SessionTitleRequest } from "../sessionTitleTypes";

export default function SessionTitleEditor({ sessionId, title, workspaceIdentity, onUpdated, onTaskStarted, onDialogOpenChange }: {
  sessionId: string;
  title: string;
  workspaceIdentity: string;
  onUpdated?: () => void;
  onTaskStarted?: () => void;
  onDialogOpenChange?: (open: boolean) => void;
}) {
  const [display, setDisplay] = useState(title);
  const [draft, setDraft] = useState(title);
  const [editing, setEditing] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const mounted = useRef(true);
  const busy = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const headingId = useId();
  const titleHelpId = useId();
  const taskMode = !!onTaskStarted;
  useLayoutEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useLayoutEffect(() => {
    const modal = dialog.current;
    if (!editing || !modal) return;
    modal.showModal();
    onDialogOpenChange?.(true);
    return () => {
      if (modal.open) modal.close();
      onDialogOpenChange?.(false);
    };
  }, [editing, onDialogOpenChange]);
  useEffect(() => { setDisplay(title); }, [title]);
  const valid = draft.trim().length > 0 && Array.from(draft.trim()).length <= 100 && !/[\u0000-\u001f\u007f-\u009f]/.test(draft);

  function cancel() {
    if (busy.current) return;
    dialog.current?.close();
    setEditing(false);
    trigger.current?.focus();
  }

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
      dialog.current?.close();
      setEditing(false);
      onUpdated?.();
      onTaskStarted?.();
    } catch (error) {
      if (mounted.current) setError(error instanceof Error ? error.message : "Could not update session title.");
    } finally {
      busy.current = false;
      if (mounted.current) setPending(false);
    }
  }

  const form = <form onSubmit={event => { event.preventDefault(); void save("set"); }}
      onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); cancel(); } }}>
      <input aria-label={taskMode ? "Task title" : "Session title"}
        aria-describedby={taskMode ? titleHelpId : undefined}
        aria-invalid={taskMode && !!draft && !valid ? true : undefined} value={draft} onChange={event => setDraft(event.target.value)} autoFocus disabled={pending} />
      <div className="session-title-actions">
        <button type="submit" disabled={pending || !valid}>{pending ? "Saving…" : taskMode ? "Save title" : "Save"}</button>
        <button type="button" disabled={pending} onClick={cancel}>Cancel</button>
        {!taskMode && <button type="button" disabled={pending} onClick={() => { void save("automatic"); }}>Use automatic title</button>}
      </div>
      {!taskMode && <p className="session-title-help">Up to 100 characters. Your title stays until you use automatic naming.</p>}
      {error && <p role="alert">{error}</p>}
    </form>;

  if (taskMode) return <>
    <button ref={trigger} type="button" data-start-new-task onClick={() => {
      setDraft("");
      setError("");
      setEditing(true);
    }}>Start new task</button>
    {editing && createPortal(<dialog ref={dialog} role="dialog" className="start-task-dialog" data-start-task-dialog
      aria-labelledby={headingId} onCancel={event => { event.preventDefault(); cancel(); }}
      onKeyDown={event => event.stopPropagation()}>
      <h2 id={headingId}>Start new task</h2>
      <p id={titleHelpId}>Enter a title of 1–100 characters without control characters, then enter your instructions in the terminal.</p>
      {form}
    </dialog>, document.body)}
  </>;

  return <div className="session-title-editor">
    <div className="session-detail-label">Title</div>
    {editing ? form : <div className="session-title-current">
      <span>{display}</span>
      <button type="button" aria-label="Rename session" title="Rename session" onClick={() => { setDraft(display); setError(""); setEditing(true); }}><Pencil size={14} aria-hidden="true" /></button>
    </div>}
  </div>;
}
