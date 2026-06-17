import type { SessionInfo } from "../api";
import { sessionAttentionStatus, sourceColor } from "./RightSidebarHelpers.ts";

interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
}

function SessionDetailPanel({ sessions, activeSessionId }: SessionDetailPanelProps) {
  const active = sessions.find((s) => s.id === activeSessionId);

  if (!active) {
    return (
      <div style={{ padding: "12px", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
        <p className="empty-state">Select a session to see details</p>
      </div>
    );
  }

  const attn = sessionAttentionStatus(active);

  return (
    <div style={{ padding: "8px 12px", height: "100%", overflowY: "auto" }}>
      {active.summary && (
        <div className="session-detail-section">
          <div className="session-detail-label">Task</div>
          <div className="session-detail-value">{active.summary}</div>
        </div>
      )}

      <div className="session-detail-section">
        <div className="session-detail-label">Status</div>
        <div className="session-detail-value">
          <span style={{
            display: "inline-block",
            width: 8, height: 8, borderRadius: "50%",
            background: statusDotColor(attn), marginRight: 6,
            verticalAlign: "middle",
          }} />
          {attn}
        </div>
      </div>

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
            style={{
              background: sourceColor(active.metadataSource) + "22",
              color: sourceColor(active.metadataSource),
            }}
          >
            {active.metadataSource} &middot; {Math.round((active.metadataConfidence ?? 1) * 100)}%
          </span>
        </div>
      )}

      {active.peonLastInference && (
        <div className="session-detail-section">
          <div className="session-detail-label">Peon</div>
          <span className="session-detail-value" style={{ color: "#57c7ff" }}>
            observed {active.peonLastInference}
          </span>
        </div>
      )}
    </div>
  );
}

function statusDotColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#666";
  if (status === "working" || status === "running" || status === "creating") return "#4ec94e";
  return "#666";
}

export default SessionDetailPanel;
