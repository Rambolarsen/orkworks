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
      {active.observedStatus && (
        <div className="session-detail-section">
          <div className="session-detail-label">Observed</div>
          <div className="session-detail-value">{active.observedStatus}</div>
        </div>
      )}
      {active.summary && (
        <div className="session-detail-section">
          <div className="session-detail-label">Summary</div>
          <div className="session-detail-value">{active.summary}</div>
        </div>
      )}
      {active.detectedQuestion && (
        <div className="session-detail-section">
          <div className="session-detail-label">Question</div>
          <div className="session-detail-value">{active.detectedQuestion}</div>
        </div>
      )}
      <div className="session-detail-section">
        <div className="session-detail-label">Directory</div>
        <div className="session-detail-value">{active.cwd.split("/").pop() || active.cwd}</div>
      </div>
      {active.branch && (
        <div className="session-detail-section">
          <div className="session-detail-label">Git</div>
          <div className="session-detail-value">
            {active.branch}
            {active.isWorktree && (
              <span style={{ color: "#4ec94e", marginLeft: 6, fontSize: 10 }}>worktree</span>
            )}
          </div>
          <div style={{ display: "flex", gap: 8, marginTop: 2, fontSize: 10 }}>
            <span style={{ color: active.dirty ? "#d4d44e" : "#4ec94e" }}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles !== undefined && active.changedFiles > 0 && (
              <span style={{ color: "#858585" }}>{active.changedFiles} files changed</span>
            )}
          </div>
        </div>
      )}
      {active.conflictWarning && (
        <div className="session-detail-section">
          <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
        </div>
      )}
      {active.recommendation && (
        <div className="session-detail-section">
          <div className="recommendation-text">{active.recommendation}</div>
        </div>
      )}
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
      {active.peonLastInference && (
        <div className="session-detail-section">
          <div className="session-detail-label">Peon</div>
          <span className="session-detail-value" style={{ color: '#57c7ff' }}>
            observed {active.peonLastInference}
          </span>
        </div>
      )}
    </div>
  );
}

export default RightSidebar;
