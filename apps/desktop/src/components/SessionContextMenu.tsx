import { useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Archive, ArchiveRestore, Play, Star, StarOff, Trash2, X } from "lucide-react";
import type { SessionAction, SessionInfo } from "../api";
import { resumeChoices } from "../labels";

export interface SessionMenuAnchor { session: SessionInfo; x: number; y: number; invoker: HTMLElement | null }
interface Props {
  anchor: SessionMenuAnchor;
  disabled: boolean;
  onDismiss: () => boolean;
  onAction: (action: SessionAction | "resume") => void;
}

const actionIcons = {
  close: X,
  archive: Archive,
  restore: ArchiveRestore,
  delete: Trash2,
  favorite: Star,
  unfavorite: StarOff,
  resume: Play,
};

export default function SessionContextMenu({ anchor, disabled, onDismiss, onAction }: Props) {
  const restoreWarningId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: anchor.x, top: anchor.y });
  const session = anchor.session;
  const state = session.sessionState;
  const favoriteItem: { action: SessionAction; label: string } = session.favorited
    ? { action: "unfavorite", label: "Unfavorite" }
    : { action: "favorite", label: "Favorite" };
  const actions: Array<{ action: SessionAction | "resume"; label: string }> = state === "active"
    ? [{ action: "close", label: "Close" }, { action: "archive", label: "Archive (close first)" }, favoriteItem, { action: "delete", label: "Delete… (close first)" }]
    : state === "archived"
      ? [{ action: "restore", label: "Restore to Closed" }, favoriteItem, { action: "delete", label: "Delete…" }]
      : state === "closed"
        ? [...(resumeChoices(session).some(choice => !choice.unavailable) ? [{ action: "resume" as const, label: session.resumeStrategy === "exact" ? "Resume" : "Resume options…" }] : []), { action: "archive", label: "Archive" }, favoriteItem, { action: "delete", label: "Delete…" }]
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
  }, [anchor, disabled, onDismiss]);

  return createPortal(<div ref={ref} className="session-context-menu" role="menu" aria-label={`Actions for ${session.label}`} tabIndex={-1}
    style={{ position: "fixed", ...position }} onKeyDown={event => {
      if (event.key === "Tab") {
        // After restoring the invoker, let Sessions handle Tab or let Chromium advance.
        if (!onDismiss()) event.preventDefault();
        return;
      }
      event.stopPropagation();
      if (event.key === "Escape") { event.preventDefault(); onDismiss(); return; }
      if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const items = [...(ref.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      if (!items.length) return;
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowUp" ? -1 : 1) + items.length) % items.length;
      items[next].focus();
    }}>
    {actions.map(({ action, label }) => {
      const Icon = actionIcons[action];
      return <button type="button" role="menuitem" key={action} tabIndex={-1} disabled={disabled || !session.sessionActionToken}
      aria-describedby={action === "restore" ? restoreWarningId : undefined}
      onClick={() => onAction(action)}><Icon size={16} aria-hidden="true" /><span>{label}</span></button>;
    })}
    {state === "archived" && <p id={restoreWarningId} className="session-menu-diagnostic">Restoring moves this session back to Closed with a fresh auto-archive window and re-enables automatic cleanup.</p>}
    {(!state || disabled) && <p className="session-menu-diagnostic">{session.sessionStateError ?? "Session changed or an action is running. Refresh and try again."}</p>}
  </div>, document.body);
}
