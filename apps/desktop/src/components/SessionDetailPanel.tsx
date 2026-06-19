import { useEffect, useState } from "react";
import type { SessionInfo } from "../api";
import { sessionAttentionStatus } from "../sessionSort";
import {
  attentionLabel,
  attentionTone,
  memoryStateLabel,
  relativeTime,
  resumeActionLabel,
  sourceLabel,
  sourceWithConfidence,
} from "../labels";
import EmptyState from "./EmptyState";

interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onResumeSession: (id: string) => void;
}

function SessionDetailPanel({ sessions, activeSessionId, onResumeSession }: SessionDetailPanelProps) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const interval = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const active = sessions.find((s) => s.id === activeSessionId);

  if (!active) {
    return <EmptyState message="Select a session to see details." />;
  }

  const attn = sessionAttentionStatus(active);
  const tone = attentionTone(attn);
  const canResume = active.memoryState === "resumable" && active.resumeStrategy !== "none";
  const resumeText = resumeActionLabel(active.resumeStrategy);
  const folder = active.cwd.split("/").pop() || active.cwd;
  const sourceTag = active.metadataSource ?? undefined;

  return (
    <div className="session-detail">
      {active.summary && (
        <div className="session-detail-section">
          <div className="session-detail-label">Task</div>
          <div className="session-detail-value">{active.summary}</div>
        </div>
      )}

      <div className="session-detail-section">
        <div className="session-detail-label">Status</div>
        <div className="session-detail-value">
          <span className="session-detail-dot" data-attention={tone} aria-hidden="true" />
          {attentionLabel(attn)}
        </div>
      </div>

      <div className="session-detail-section">
        <div className="session-detail-label">Directory</div>
        <div className="session-detail-value">{folder}</div>
      </div>

      {active.branch && (
        <div className="session-detail-section">
          <div className="session-detail-label">Git</div>
          <div className="session-detail-value">
            {active.branch}
            {active.isWorktree && (
              <span className="git-worktree-tag">worktree</span>
            )}
          </div>
          <div className="git-state-row">
            <span className="git-state" data-state={active.dirty ? "dirty" : "clean"}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles !== undefined && active.changedFiles > 0 && (
              <span className="git-changed">{active.changedFiles} files changed</span>
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

      <div className="session-detail-section">
        <div className="session-detail-label">Memory</div>
        <div className="session-detail-value">
          {memoryStateLabel(active.memoryState)} · {resumeActionLabel(active.resumeStrategy)}
        </div>
      </div>

      {sourceTag && (
        <div className="session-detail-section">
          <div className="session-detail-label">Source</div>
          <span className="source-badge" data-source={sourceLabel(sourceTag).toLowerCase()}>
            {sourceWithConfidence(sourceTag, active.metadataConfidence)}
          </span>
        </div>
      )}

      {active.peonLastInference && (
        <div className="session-detail-section">
          <div className="session-detail-label">Peon</div>
          <span className="session-detail-value peon-value">
            Observed {relativeTime(active.peonLastInference, now) || active.peonLastInference}
          </span>
        </div>
      )}

      <button
        className="session-resume-button"
        type="button"
        disabled={!canResume}
        onClick={() => onResumeSession(active.id)}
        title={resumeText}
      >
        {resumeText}
      </button>
    </div>
  );
}

export default SessionDetailPanel;
