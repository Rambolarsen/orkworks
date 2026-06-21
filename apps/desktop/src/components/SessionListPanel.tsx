import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { Trash2 } from "lucide-react";
import type { SessionInfo, WorkspaceInfo } from "../api";
import { sessionAttentionStatus } from "../sessionSort";
import {
  VOCAB,
  attentionLabel,
  attentionTone,
  memoryStateLabel,
  relativeTime,
  sourceWithConfidence,
} from "../labels";
import EmptyState from "./EmptyState";

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
  onForgetSession: (id: string) => void;
  onFocusTerminal: () => void;
  onOpenWorkspace: () => void;
}

type GroupKey = "today" | "week" | "earlier";

const GROUP_LABELS: Record<GroupKey, string> = {
  today: "Today",
  week: "This week",
  earlier: "Earlier",
};

function groupForSession(s: SessionInfo, now: Date): GroupKey {
  const created = new Date(s.created_at);
  if (Number.isNaN(created.getTime())) return "earlier";
  const sameDay =
    created.getFullYear() === now.getFullYear() &&
    created.getMonth() === now.getMonth() &&
    created.getDate() === now.getDate();
  if (sameDay) return "today";
  const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
  if (now.getTime() - created.getTime() < sevenDaysMs) return "week";
  return "earlier";
}

function lastActivity(s: SessionInfo, now: Date): string {
  return relativeTime(s.peonLastInference, now) || relativeTime(s.created_at, now);
}

function SessionListPanel({
  workspace,
  sessions,
  activeSessionId,
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

  const grouped = useMemo(() => {
    const buckets: Record<GroupKey, SessionInfo[]> = {
      today: [],
      week: [],
      earlier: [],
    };
    for (const s of sessions) {
      buckets[groupForSession(s, now)].push(s);
    }
    return (["today", "week", "earlier"] as GroupKey[])
      .filter((k) => buckets[k].length > 0)
      .map((k) => ({ key: k, label: GROUP_LABELS[k], items: buckets[k] }));
  }, [sessions]);

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
                const tone = attentionTone(attn);
                const folder = s.cwd.split("/").pop() || s.cwd;
                const dirtyText = s.dirty && s.changedFiles ? ` · ${s.changedFiles} files` : "";
                const action = s.summary || s.nextAction;
                return (
                  <li
                    key={s.id}
                    ref={(el) => {
                      if (el) itemRefs.current.set(s.id, el);
                      else itemRefs.current.delete(s.id);
                    }}
                    className={[
                      "session-row",
                      s.memoryState !== "live" ? "session-row--remembered" : "",
                    ].filter(Boolean).join(" ")}
                    role="option"
                    aria-selected={s.id === activeSessionId}
                    data-attention={tone}
                    onClick={() => handleSelect(s.id)}
                  >
                    <div className="session-row-primary">
                      {tone !== "neutral" && (
                        <span className="session-row-dot" aria-hidden="true" />
                      )}
                      <span className="session-row-label">{s.label}</span>
                    </div>
                    <div className="session-row-meta">
                      <span className="session-row-time">{lastActivity(s, now)}</span>
                      {s.memoryState === "live" && (
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
                      {s.memoryState !== "live" && (
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
                    <div className="session-row-secondary">
                      {attentionLabel(attn)} · {folder}{dirtyText}
                    </div>
                    {action && (
                      <div className="session-row-action">{action}</div>
                    )}
                    {s.metadataSource && (
                      <div className="session-row-source">
                        {sourceWithConfidence(s.metadataSource, s.metadataConfidence)}
                      </div>
                    )}
                    {s.memoryState !== "live" && (
                      <div className="session-row-memory">
                        {memoryStateLabel(s.memoryState)}
                      </div>
                    )}
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
