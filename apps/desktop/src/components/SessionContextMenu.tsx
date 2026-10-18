import { useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { SessionAction, SessionInfo } from "../api";
import { resumeChoices } from "../labels";

export interface SessionMenuAnchor { session: SessionInfo; x: number; y: number; invoker: HTMLElement | null }
interface Props {
  anchor: SessionMenuAnchor;
  disabled: boolean;
  onDismiss: () => void;
  onAction: (action: SessionAction | "resume") => void;
}

export default function SessionContextMenu({ anchor, disabled, onDismiss, onAction }: Props) {
  const restoreWarningId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: anchor.x, top: anchor.y });
  const session = anchor.session;
  const state = session.sessionState;
  const actions: Array<{ action: SessionAction | "resume"; label: string }> = state === "active"
    ? [{ action: "close", label: "Close" }, { action: "archive", label: "Archive (close first)" }, { action: "delete", label: "Delete… (close first)" }]
    : state === "archived"
      ? [{ action: "restore", label: "Restore to Closed" }, { action: "delete", label: "Delete…" }]
      : state === "closed"
        ? [...(resumeChoices(session).some(choice => !choice.unavailable) ? [{ action: "resume" as const, label: session.resumeStrategy === "exact" ? "Resume" : "Resume options…" }] : []), { action: "archive", label: "Archive" }, { action: "delete", label: "Delete…" }]
        : [];
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const bounds = element.getBoundingClientRect();
    setPosition({ left: Math.max(8, Math.min(anchor.x, window.innerWidth - bounds.width - 8)), top: Math.max(8, Math.min(anchor.y, window.innerHeight - bounds.height - 8)) });
    (element.querySelector<HTMLButtonElement>("button:not(:disabled)") ?? element).focus();
    const outside = (event: PointerEvent) => { if (!element.contains(event.target as Node)) onDismiss(); };
    const dismiss = () => onDismiss();
    document.addEventListener("pointerdown", outside, true);
    window.addEventListener("resize", dismiss);
    return () => { document.removeEventListener("pointerdown", outside, true); window.removeEventListener("resize", dismiss); };
  }, [anchor, onDismiss]);

  return createPortal(<div ref={ref} className="session-context-menu" role="menu" aria-label={`Actions for ${session.label}`} tabIndex={-1}
    style={{ position: "fixed", ...position }} onKeyDown={event => {
      event.stopPropagation();
      if (event.key === "Escape" || event.key === "Tab") { event.preventDefault(); onDismiss(); return; }
      if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const items = [...(ref.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      if (!items.length) return;
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowUp" ? -1 : 1) + items.length) % items.length;
      items[next].focus();
    }}>
    {actions.map(({ action, label }) => <button type="button" role="menuitem" key={action} tabIndex={-1} disabled={disabled || !session.sessionActionToken}
      aria-describedby={action === "restore" ? restoreWarningId : undefined}
      onClick={() => onAction(action)}>{label}</button>)}
    {state === "archived" && <p id={restoreWarningId} className="session-menu-diagnostic">Restoring re-enables automatic cleanup. Older history may be deleted by your retention settings.</p>}
    {(!state || disabled) && <p className="session-menu-diagnostic">{session.sessionStateError ?? "Session changed or an action is running. Refresh and try again."}</p>}
  </div>, document.body);
}
