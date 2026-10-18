import { Fragment, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { MoreHorizontal } from "lucide-react";
import type { SessionAction, SessionInfo, WorkspaceInfo } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import { sessionAttentionStatus } from "../sessionSort";
import { harnessDisplayName, sessionCodingTool } from "../sessionProviderContext";
import { groupSessionList, nextSessionGroupRefreshMs } from "../sessionGroups";
import {
  VOCAB,
  attentionLabel,
  attentionTone,
  isLoudTone,
  lastActivity,
  lastActivityTimestamp,
  minDelay,
  nextRelativeTimeRefreshMs,
} from "../labels";
import { useStableRelativeTimeNow } from "../useStableRelativeTimeNow";
import EmptyState from "./EmptyState";
import StatusIndicator from "./StatusIndicator";
import HarnessIcon from "./HarnessIcon";
import SessionContextMenu, { type SessionMenuAnchor } from "./SessionContextMenu";

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  unreadIds: ReadonlySet<string>;
  acknowledgedIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  onSelectSession: (id: string) => void;
  actionsEnabled?: boolean;
  onSessionAction?: (id: string, action: SessionAction, token: string) => Promise<void>;
  onResumeSession?: (id: string) => void;
  onFocusTerminal: () => void;
  onOpenWorkspace: () => void;
}

function SessionListPanel({
  workspace,
  sessions,
  activeSessionId,
  unreadIds,
  acknowledgedIds,
  harnesses,
  onSelectSession,
  actionsEnabled = true,
  onSessionAction,
  onResumeSession,
  onFocusTerminal,
  onOpenWorkspace,
}: SessionListPanelProps) {
  const [filter, setFilter] = useState<"all" | "active" | "closed" | "archived">("all");
  const filterRef = useRef<HTMLSelectElement>(null);
  const actionInvoker = useRef<HTMLElement | null>(null);
  const menuRef = useRef<SessionMenuAnchor | null>(null);
  const workspaceGeneration = useRef(0);
  const [menu, setMenu] = useState<SessionMenuAnchor | null>(null);
  const [pending, setPending] = useState<ReadonlySet<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const workspaceIdentity = workspace?.workspaceIdentity ?? workspace?.path;
  useEffect(() => { workspaceGeneration.current += 1; actionInvoker.current = null; menuRef.current = null; setMenu(null); setFilter("all"); setError(null); setPending(new Set()); }, [workspaceIdentity]);
  const listRef = useRef<HTMLUListElement | null>(null);
  const itemRefs = useRef<Map<string, HTMLLIElement>>(new Map());
  useLayoutEffect(() => {
    const invoker = actionInvoker.current;
    if (!invoker) return;
    if (!invoker.isConnected) {
      if (document.activeElement === document.body) (listRef.current ?? filterRef.current)?.focus();
      actionInvoker.current = null;
    } else if (document.activeElement !== invoker && document.activeElement !== document.body) {
      actionInvoker.current = null;
    }
  });

  useEffect(() => {
    if (!activeSessionId) return;
    const el = itemRefs.current.get(activeSessionId);
    el?.scrollIntoView({ block: "nearest" });
  }, [activeSessionId]);

  const now = useStableRelativeTimeNow(useCallback((currentNow: Date) => {
    let nextRefresh = nextSessionGroupRefreshMs(sessions, currentNow);
    for (const session of sessions) {
      nextRefresh = minDelay(nextRefresh, nextRelativeTimeRefreshMs(lastActivityTimestamp(session), currentNow));
    }
    return nextRefresh;
  }, [sessions]));

  const filtered = useMemo(() => sessions.filter(session => {
    const state = session.sessionState === undefined ? (session.lifecycle === "dead" ? "closed" : "active") : session.sessionState;
    return filter === "all" ? state !== "archived" : state === filter;
  }), [sessions, filter]);
  const grouped = useMemo(() => groupSessionList(filtered, now), [filtered, now]);
  const dismissMenu = useCallback(() => {
    const current = menuRef.current;
    menuRef.current = null;
    setMenu(null);
    if (current) (current.invoker?.isConnected ? current.invoker : listRef.current)?.focus();
  }, []);
  const openMenu = (session: SessionInfo, x: number, y: number, invoker: HTMLElement | null) => {
    setError(null);
    menuRef.current = { session, x, y, invoker };
    setMenu(menuRef.current);
  };
  const runAction = async (action: SessionAction | "resume") => {
    const target = menu?.session;
    if (!target || !target.sessionActionToken || pending.has(target.id)) return;
    actionInvoker.current = menu?.invoker ?? null;
    dismissMenu();
    if (action === "resume") { onResumeSession?.(target.id); return; }
    if (action === "delete" && !window.confirm(`Permanently delete “${target.label}”? ${target.sessionState === "active" ? "The session will be closed first. " : ""}The session record, events, and saved terminal scrollback cannot be restored.`)) return;
    const generation = workspaceGeneration.current;
    setPending(current => new Set(current).add(target.id));
    try { await onSessionAction?.(target.id, action, target.sessionActionToken); }
    catch (error) { if (generation === workspaceGeneration.current) setError(error instanceof Error ? error.message : "Session action failed. Refresh and try again."); }
    finally { if (generation === workspaceGeneration.current) setPending(current => { const next = new Set(current); next.delete(target.id); return next; }); }
  };

  const orderedSessions = useMemo(
    () => grouped.flatMap((g) => g.items),
    [grouped],
  );

  if (!workspace) {
    return (
      <div className="panel-content">
        <EmptyState
          message="Open a workspace to see sessions."
          action={{ label: VOCAB.openWorkspace, onClick: onOpenWorkspace }}
        />
      </div>
    );
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLUListElement>) => {
    if (e.target !== e.currentTarget) return;
    if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
      const target = orderedSessions.find(session => session.id === activeSessionId);
      if (target) {
        e.preventDefault();
        const row = itemRefs.current.get(target.id)?.getBoundingClientRect();
        openMenu(target, row?.left ?? 0, row?.bottom ?? 0, listRef.current);
      }
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      onFocusTerminal();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    if (orderedSessions.length === 0) return;
    e.preventDefault();
    const idx = orderedSessions.findIndex((s) => s.id === activeSessionId);
    let next: number;
    if (idx === -1) {
      next = 0;
    } else if (e.key === "ArrowDown") {
      next = Math.min(orderedSessions.length - 1, idx + 1);
    } else {
      next = Math.max(0, idx - 1);
    }
    if (orderedSessions[next].id !== activeSessionId) {
      onSelectSession(orderedSessions[next].id);
    }
  };

  const handleSelect = (id: string) => {
    onSelectSession(id);
    listRef.current?.focus();
  };

  return (
    <div className="panel-content">
      <label className="session-state-filter">State
        <select ref={filterRef} aria-label="Session state" value={filter} onChange={event => { setFilter(event.target.value as typeof filter); menuRef.current = null; setMenu(null); }}>
          <option value="all">All unarchived</option><option value="active">Active</option><option value="closed">Closed</option><option value="archived">Archived</option>
        </select>
      </label>
      {error && <p role="alert" className="session-action-error">{error}</p>}
      {pending.size > 0 && <p role="status" className="session-action-progress">Finishing session action…</p>}
      {filtered.length === 0 ? (
        <EmptyState message={sessions.length ? "No sessions in this state." : "No sessions yet. Press ⌘N to start one."} />
      ) : (
        <ul
          id="sessions-list"
          ref={listRef}
          className="session-list"
          role="list"
          aria-label="Sessions"
          tabIndex={0}
          onKeyDown={handleKeyDown}
        >
          {grouped.map((group) => (
            <Fragment key={group.key}>
              <li className="session-group-header" aria-hidden="true">
                {group.label}
              </li>
              {group.items.map((s) => {
                const attn = sessionAttentionStatus(s);
                const tone = attentionTone(attn);
                const remembered = s.lifecycle === "dead";
                // Unread is "changed since you looked" — a remembered session
                // can't change under you, so the signal is suppressed there.
                const unread = unreadIds.has(s.id) && !remembered;
                // A dead session can carry a failed tone now (terminal
                // outcome "error"), but remembered rows stay dimmed — the
                // icon is the signal, never the loud treatment.
                const attention = isLoudTone(tone) && !remembered;
                const loud = attention && !acknowledgedIds.has(s.id);
                // Display name, never the raw harness id — ids leak into the
                // icon tooltip/aria-label otherwise.
                const tool = harnessDisplayName(harnesses, sessionCodingTool(s));
                return (
                  <li
                    key={s.id}
                    ref={(el) => {
                      if (el) itemRefs.current.set(s.id, el);
                      else itemRefs.current.delete(s.id);
                    }}
                    className={[
                      "session-row",
                      remembered ? "session-row--remembered" : "",
                      unread ? "session-row--unread" : "",
                      loud ? "session-row--loud" : "",
                    ].filter(Boolean).join(" ")}
                    data-session-id={s.id}
                    data-selected={s.id === activeSessionId}
                    aria-busy={pending.has(s.id)}
                    data-attention={tone}
                    onClick={() => handleSelect(s.id)}
                    onContextMenu={event => { event.preventDefault(); openMenu(s, event.clientX, event.clientY, event.currentTarget.querySelector<HTMLButtonElement>(".session-row-more")); }}
                    onKeyDown={event => {
                      if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
                        event.preventDefault(); event.stopPropagation();
                        const bounds = event.currentTarget.getBoundingClientRect();
                        openMenu(s, bounds.left, bounds.bottom, event.target as HTMLElement);
                      }
                    }}
                  >
                    <button type="button" className="session-row-select" tabIndex={-1} aria-pressed={s.id === activeSessionId}>
                    <span className="session-row-leading">
                      <span className="session-row-primary">
                        <StatusIndicator
                          tone={tone}
                          label={attentionLabel(attn)}
                          variant={unread ? "unread" : "status"}
                        />
                        <span className="session-row-label">{s.label}</span>
                      </span>
                    </span>
                    <span className="session-row-secondary">
                      {attention && (
                        <span className="session-row-attention" data-attention={tone}>
                          {attentionLabel(attn)}
                        </span>
                      )}
                      <span className="session-row-meta">
                        <span className="session-row-tool" aria-hidden={!tool}>
                          {tool && <HarnessIcon tool={tool} />}
                        </span>
                        <span className="session-row-time">{lastActivity(s, now)}</span>
                      </span>
                    </span>
                    </button>
                    <div className="session-row-actions">
                      <button type="button" className="session-row-more" aria-label={`More actions for ${s.label}`} aria-haspopup="menu" aria-expanded={menu?.session.id === s.id}
                        disabled={pending.has(s.id)}
                        onClick={event => {
                          event.stopPropagation();
                          const bounds = event.currentTarget.getBoundingClientRect();
                          openMenu(s, bounds.left, bounds.bottom, event.currentTarget);
                        }}><MoreHorizontal size={16} /></button>
                    </div>
                  </li>
                );
              })}
            </Fragment>
          ))}
        </ul>
      )}
      {menu && <SessionContextMenu anchor={menu} disabled={!actionsEnabled || !onSessionAction || pending.has(menu.session.id) || sessions.find(s => s.id === menu.session.id)?.sessionActionToken !== menu.session.sessionActionToken}
        onDismiss={dismissMenu} onAction={action => { void runAction(action); }} />}
    </div>
  );
}

export default SessionListPanel;
