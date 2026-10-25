import { useEffect, useId, useRef, useState } from "react";
import { ChevronDown, ExternalLink } from "lucide-react";
import type { SessionInfo } from "../api";

type Availability = { label: string; available: boolean; reason?: string };

export default function SessionAppMenu({ workspacePath, session, disabled }: { workspacePath: string; session: SessionInfo; disabled: boolean }) {
  const sessionId = session.id;
  const destination = session.harnessId === "codex" ? "ChatGPT" : session.harnessId === "claude-code" ? "Claude" : null;
  const [open, setOpen] = useState(false);
  const [availability, setAvailability] = useState<Availability | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const intent = useRef(0);
  const panelId = useId();

  useEffect(() => {
    setAvailability(null);
  }, [disabled, workspacePath, sessionId, session.harnessId, session.resume?.harnessSessionId, session.lifecycle, session.cwd]);

  useEffect(() => {
    if (disabled) return;
    let current = true;
    setError(null);
    void window.orkworks.inspectSessionApp(workspacePath, sessionId).then(value => {
      if (current) setAvailability(value);
    }).catch(reason => {
      if (current) setError(reason instanceof Error ? reason.message : "Couldn't check the desktop application.");
    });
    return () => { current = false; };
  }, [open, disabled, workspacePath, sessionId, session.harnessId, session.resume?.harnessSessionId, session.lifecycle, session.cwd]);

  useEffect(() => {
    if (!open || disabled) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => { document.removeEventListener("pointerdown", outside); };
  }, [open, disabled, workspacePath, sessionId]);
  useEffect(() => () => { intent.current++; }, []);

  async function launch() {
    const request = ++intent.current;
    setOpening(true);
    setError(null);
    try {
      await window.orkworks.openSessionApp(workspacePath, sessionId);
      if (request !== intent.current) return;
      setOpen(false);
      trigger.current?.focus();
    } catch (reason) {
      if (request === intent.current) setError(reason instanceof Error ? reason.message : "Couldn't open this session.");
    } finally {
      if (request === intent.current) setOpening(false);
    }
  }

  return <div ref={root} className="session-app-menu" onKeyDown={event => {
    if (event.key !== "Escape" || !open) return;
    event.preventDefault(); event.stopPropagation();
    setOpen(false); trigger.current?.focus();
  }} onBlur={event => {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOpen(false);
  }}>
    <button ref={trigger} type="button" data-session-app-trigger aria-expanded={open} aria-controls={open ? panelId : undefined}
      disabled={disabled} onClick={() => setOpen(value => !value)}><ExternalLink size={14} aria-hidden="true" />{destination ? `Open in ${destination}` : "Open in…"}<ChevronDown size={13} aria-hidden="true" /></button>
    {open && <div id={panelId} className="session-app-popover" role="group" aria-label="Open session in application">
      <button type="button" data-session-app-open disabled={!availability?.available || opening || disabled} onClick={() => void launch()}>
        {opening ? "Opening…" : availability ? `Open in ${availability.label}` : "Checking application…"}
      </button>
      {availability?.reason && <p>{availability.reason}</p>}
      {error && <p role="alert">{error}</p>}
    </div>}
  </div>;
}
