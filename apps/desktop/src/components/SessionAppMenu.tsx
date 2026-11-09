import { useEffect, useId, useRef, useState } from "react";
import { ChevronDown, ExternalLink } from "lucide-react";
import HarnessIcon from "./HarnessIcon";
import type { SessionInfo } from "../api";

type Availability = { label: string; available: boolean; reason?: string; installed?: boolean };

export default function SessionAppMenu({ workspacePath, session, disabled }: { workspacePath: string; session: SessionInfo; disabled: boolean }) {
  const sessionId = session.id;
  const [open, setOpen] = useState(false);
  const [availability, setAvailability] = useState<Availability | null>(null);
  const [folderAvailability, setFolderAvailability] = useState<Availability | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const intent = useRef(0);
  const panelId = useId();

  useEffect(() => {
    setAvailability(null);
    setFolderAvailability(null);
  }, [disabled, workspacePath, sessionId, session.harnessId, session.resume?.harnessSessionId, session.lifecycle, session.cwd, session.worktreeBinding?.root]);

  useEffect(() => {
    if (disabled) return;
    let current = true;
    setError(null);
    for (const destination of ["native", "vscode"] as const) {
      void window.orkworks.inspectSessionApp(workspacePath, sessionId, destination).then(value => {
        if (current) (destination === "native" ? setAvailability : setFolderAvailability)(value);
      }).catch(reason => {
        if (current) setError(reason instanceof Error ? reason.message : "Couldn't check the desktop application.");
      });
    }
    return () => { current = false; };
  }, [open, disabled, workspacePath, sessionId, session.harnessId, session.resume?.harnessSessionId, session.lifecycle, session.cwd, session.worktreeBinding?.root]);

  useEffect(() => {
    if (!open || disabled) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => { document.removeEventListener("pointerdown", outside); };
  }, [open, disabled, workspacePath, sessionId]);
  useEffect(() => () => { intent.current++; }, []);

  async function launch(destination: "native" | "vscode" = "native") {
    const request = ++intent.current;
    setOpening(true);
    setError(null);
    try {
      await window.orkworks.openSessionApp(workspacePath, sessionId, destination);
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
      disabled={disabled} onClick={() => setOpen(value => !value)}><ExternalLink size={14} aria-hidden="true" />Open in…<ChevronDown size={13} aria-hidden="true" /></button>
    {open && <div id={panelId} className="session-app-popover" role="group" aria-label="Open session in application">
      <button type="button" data-session-app-open disabled={!availability?.available || opening || disabled} onClick={() => void launch()}>
        <span aria-hidden="true"><HarnessIcon tool={session.harnessId} size={16} /></span>
        {opening ? "Opening…" : availability ? `Open in ${availability.label}` : "Checking application…"}
      </button>
      {availability?.reason && <p>{availability.reason}</p>}
      {folderAvailability?.installed && <>
        <button type="button" data-session-app-vscode disabled={!folderAvailability.available || opening || disabled} onClick={() => void launch("vscode")}>
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor">
            <path d="M17.5 2 6.2 10.2 2.8 7.7 1 8.7v6.6l1.8 1 3.4-2.5L17.5 22l5.5-2.7V4.7L17.5 2ZM4.8 12 2.8 13.5v-3L4.8 12Zm12.7-5.1v10.2L9.5 12l8-5.1Z" />
          </svg>
          <span className="session-app-destination">Visual Studio Code<small>Open project folder</small></span>
        </button>
        {folderAvailability.reason && <p>{folderAvailability.reason}</p>}
      </>}
      {error && <p role="alert">{error}</p>}
    </div>}
  </div>;
}
