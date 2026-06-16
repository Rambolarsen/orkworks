import type { SessionInfo } from "../api";
import {
  needsAttention,
  isLive,
  borderColor,
  sourceColor,
  sortSessions,
} from "./RightSidebarHelpers.ts";

interface RightSidebarProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
}

function RightSidebar({ sessions, activeSessionId, onSelectSession }: RightSidebarProps) {
  const sorted = sortSessions(sessions);
  const live = sorted.filter((s) => isLive(s.status));
  const done = sorted.filter((s) => !isLive(s.status));

  if (sessions.length === 0) return null;

  return (
    <div className="overview-list">
      {live.length > 0 && (
        <div className="overview-group">
          <div className="overview-group-header">
            Working &middot; {live.length}
          </div>
          {live.map((s) => (
            <div
              key={s.id}
              className={`overview-card ${s.id === activeSessionId ? "overview-card--active" : ""}`}
              style={{ borderLeftColor: borderColor(s.status) }}
              onClick={() => onSelectSession(s.id)}
            >
              <div className="overview-card-main">
                {needsAttention(s.status) && (
                  <span className="overview-alert" title="Needs attention">&#x26A0;</span>
                )}
                <span className="overview-card-label">{s.label}</span>
              </div>
              <div className="overview-card-meta">
                {s.status}
              </div>
              {s.metadataSource && (
                <span
                  className="overview-card-badge"
                  style={{ background: sourceColor(s.metadataSource) + "22", color: sourceColor(s.metadataSource) }}
                >
                  {s.metadataSource} &middot; {Math.round((s.metadataConfidence ?? 1) * 100)}%
                </span>
              )}
            </div>
          ))}
        </div>
      )}
      {done.length > 0 && (
        <div className="overview-group">
          <div className="overview-group-header overview-group-header--done">
            Done &middot; {done.length}
          </div>
          {done.map((s) => (
            <div
              key={s.id}
              className={`overview-card overview-card--done ${s.id === activeSessionId ? "overview-card--active" : ""}`}
              style={{ borderLeftColor: borderColor(s.status) }}
              onClick={() => onSelectSession(s.id)}
            >
              <div className="overview-card-main">
                <span className="overview-card-label">{s.label}</span>
              </div>
              <div className="overview-card-meta">
                {s.status}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default RightSidebar;
