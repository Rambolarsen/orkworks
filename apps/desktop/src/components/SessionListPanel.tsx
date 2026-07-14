import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { Trash2 } from "lucide-react";
import type { SessionInfo, WorkspaceInfo } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import { sessionAttentionStatus } from "../sessionSort";
import { harnessDisplayName, sessionCodingTool } from "../sessionProviderContext";
import { groupSessions } from "../sessionGroups";
import { VOCAB, attentionLabel, attentionTone, isLoudTone, relativeTime } from "../labels";
import EmptyState from "./EmptyState";
import StatusIndicator from "./StatusIndicator";
import HarnessIcon from "./HarnessIcon";

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  unreadIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
  onForgetSession: (id: string) => void;
  onFocusTerminal: () => void;
  onOpenWorkspace: () => void;
}

function lastActivity(s: SessionInfo, now: Date): string {
  return relativeTime(s.peonLastInference, now) || relativeTime(s.created_at, now);
}

function SessionListPanel({
  workspace,
  sessions,
  activeSessionId,
  unreadIds,
  harnesses,
  onSelectSession,
  onKillSession,
  onForgetSession,
  onFocusTerminal,
  onOpenWorkspace,
}: SessionListPanelProps) {
  const listRef = useRef<HTMLUListElement | null>(null);
  const itemRefs = useRef<Map<string, HTMLLIElement>>(new Map());

  useEffect(() => {
    if (!activeSessionId) return;
    const el = itemRefs.current.get(activeSessionId);
    el?.scrollIntoView({ block: "nearest" });
  }, [activeSessionId]);

  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const interval = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const grouped = useMemo(() => groupSessions(sessions, now), [sessions, now]);

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
      {sessions.length === 0 ? (
        <EmptyState message="No sessions yet. Press ⌘N to start one." />
      ) : (
        <ul
          id="sessions-list"
          ref={listRef}
          className="session-list"
          role="listbox"
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
                const transitional = s.lifecycle === "creating" || s.lifecycle === "stopping";
                const tone = transitional ? "working" : attentionTone(attn);
                const remembered = s.lifecycle === "dead";
                const canKill = s.lifecycle === "alive";
                // Unread is "changed since you looked" — a remembered session
                // can't change under you, so the signal is suppressed there.
                const unread = unreadIds.has(s.id) && !remembered;
                const loud = isLoudTone(tone);
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
                    role="option"
                    aria-selected={s.id === activeSessionId}
                    data-attention={tone}
                    onClick={() => handleSelect(s.id)}
                  >
                    <div className="session-row-leading">
                      <div className="session-row-primary">
                        <StatusIndicator
                          tone={tone}
                          label={transitional ? "" : attentionLabel(attn)}
                          variant={unread ? "unread" : "status"}
                        />
                        <span className="session-row-label">{s.label}</span>
                      </div>
                    </div>
                    <div className="session-row-secondary">
                      {loud && (
                        <span className="session-row-attention" data-attention={tone}>
                          {attentionLabel(attn)}
                        </span>
                      )}
                      <div className="session-row-meta">
                        <span className="session-row-tool" aria-hidden={!tool}>
                          {tool && <HarnessIcon tool={tool} />}
                        </span>
                        <span className="session-row-time">{lastActivity(s, now)}</span>
                      </div>
                      <div className="session-row-actions">
                        {canKill && (
                          <button
                            className="session-row-kill"
                            type="button"
                            aria-label="Kill session"
                            onClick={(e) => {
                              e.stopPropagation();
                              onKillSession(s.id);
                            }}
                          >
                            &times;
                          </button>
                        )}
                        {remembered && (
                          <button
                            className="session-row-forget"
                            type="button"
                            aria-label="Delete session"
                            onClick={(e) => {
                              e.stopPropagation();
                              if (window.confirm("Permanently delete this session? The session record, events, and saved terminal scrollback cannot be restored.")) {
                                onForgetSession(s.id);
                              }
                            }}
                          >
                            <Trash2 size={12} />
                          </button>
                        )}
                      </div>
                    </div>
                  </li>
                );
              })}
            </Fragment>
          ))}
        </ul>
      )}
    </div>
  );
}

export default SessionListPanel;
