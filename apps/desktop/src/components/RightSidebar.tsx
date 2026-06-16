import type { SessionInfo } from "../api";
import { sourceColor } from "./RightSidebarHelpers.ts";

interface RightSidebarProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
}

function RightSidebar({ sessions, activeSessionId }: RightSidebarProps) {
  const active = sessions.find((s) => s.id === activeSessionId);

  if (!active) {
    return (
      <div className="panel-content">
        <p className="empty-state">Select a session to see details</p>
      </div>
    );
  }

  return (
    <div className="panel-content session-detail">
      <div className="session-detail-section">
        <div className="session-detail-label">Status</div>
        <div className="session-detail-value">{active.status}</div>
      </div>
      <div className="session-detail-section">
        <div className="session-detail-label">Directory</div>
        <div className="session-detail-value">{active.cwd.split("/").pop() || active.cwd}</div>
      </div>
      {active.metadataSource && (
        <div className="session-detail-section">
          <div className="session-detail-label">Source</div>
          <span
            className="overview-card-badge"
            style={{ background: sourceColor(active.metadataSource) + "22", color: sourceColor(active.metadataSource) }}
          >
            {active.metadataSource} &middot; {Math.round((active.metadataConfidence ?? 1) * 100)}%
          </span>
        </div>
      )}
    </div>
  );
}

export default RightSidebar;
