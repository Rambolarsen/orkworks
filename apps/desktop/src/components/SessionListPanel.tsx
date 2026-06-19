import { Fragment, useEffect, useMemo, useRef } from "react";
import type { SessionInfo, WorkspaceInfo } from "../api";
import { needsAttention, sessionAttentionStatus } from "../sessionSort";
import { attentionBorderColor, sourceColor } from "./legacyColors";
import { VOCAB } from "../labels";
import EmptyState from "./EmptyState";

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
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

function SessionListPanel({
  workspace,
  sessions,
  activeSessionId,
  onSelectSession,
  onKillSession,
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

  const grouped = useMemo(() => {
    const now = new Date();
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
            return (
              <li
                key={s.id}
                ref={(el) => {
                  if (el) itemRefs.current.set(s.id, el);
                  else itemRefs.current.delete(s.id);
                }}
                className={[
                  "session-item",
                  s.id === activeSessionId ? "session-item--active" : "",
                  s.memoryState !== "live" ? "session-item--remembered" : "",
                  s.memoryState === "resumable" ? "session-item--resumable" : "",
                ].filter(Boolean).join(" ")}
                style={{ borderLeft: `3px solid ${attentionBorderColor(attn)}` }}
                onClick={() => handleSelect(s.id)}
              >
                <div className="session-item-main">
                  <div className="session-item-info">
                    <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                      {needsAttention(attn) && (
                        <span className="session-item-alert" title="Needs attention">&#x26A0;</span>
                      )}
                      <span className="session-item-label">{s.label}</span>
                    </div>
                    <span className="session-item-meta">
                      {attn} &middot; {s.cwd.split("/").pop() || s.cwd}
                    </span>
                    {s.metadataSource && (
                      <span
                        className="session-item-badge"
                        style={{
                          background: sourceColor(s.metadataSource) + "22",
                          color: sourceColor(s.metadataSource),
                        }}
                      >
                        {s.metadataSource} &middot; {Math.round((s.metadataConfidence ?? 1) * 100)}%
                      </span>
                    )}
                    {s.memoryState !== "live" && (
                      <span className="session-memory-badge">
                        {s.memoryState === "resumable" ? "resumable" : "remembered"}
                      </span>
                    )}
                  </div>
                </div>
                {s.memoryState === "live" && (
                  <button
                    className="session-kill-button"
                    type="button"
                    title="Kill session"
                    onClick={(e) => {
                      e.stopPropagation();
                      onKillSession(s.id);
                    }}
                  >
                    &times;
                  </button>
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
