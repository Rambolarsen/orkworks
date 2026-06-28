import { Fragment, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { GitBranch } from "lucide-react";
import type { SessionInfo } from "../api";
import { sessionProviderContext } from "../sessionProviderContext";
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
import StatusIndicator from "./StatusIndicator";

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
    return <EmptyState message="Select an agent session to see details." />;
  }

  const attn = sessionAttentionStatus(active);
  const tone = attentionTone(attn);
  const canResume = active.memoryState === "resumable" && active.resumeStrategy !== "none";
  const resumeText = resumeActionLabel(active.resumeStrategy);
  const sourceTag = active.metadataSource;
  const providerContext = sessionProviderContext(active);
  const folder = active.cwd.split("/").pop() || active.cwd;
  const headline =
    active.detectedQuestion ||
    active.blockerDescription ||
    active.summary ||
    active.nextAction ||
    "No additional detail recorded.";

  const provenanceItems: ReactNode[] = [];
  if (sourceTag) {
    provenanceItems.push(
      <span key="source" className="source-badge" data-source={sourceLabel(sourceTag).toLowerCase()}>
        {sourceWithConfidence(sourceTag, active.metadataConfidence)}
      </span>,
    );
  }
  if (active.peonLastInference) {
    provenanceItems.push(
      <span key="peon" className="peon-value">
        Observed {relativeTime(active.peonLastInference, now) || active.peonLastInference}
      </span>,
    );
  }
  provenanceItems.push(<span key="memory">{memoryStateLabel(active.memoryState)}</span>);

  return (
    <div className="session-detail" data-attention={tone}>
      <div className="detail-situation">
        <div className="detail-eyebrow">
          <StatusIndicator tone={tone} label={attentionLabel(attn)} />
          <span>{attentionLabel(attn)}</span>
        </div>
        <div className="detail-headline">{headline}</div>
        {active.conflictWarning && (
          <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
        )}
      </div>

      <div className="detail-actions">
        {active.recommendation && <div className="recommendation-text">{active.recommendation}</div>}
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

      <div className="detail-facts">
        <div className="detail-facts-grid">
          <div className="detail-fact">
            <div className="session-detail-label">Directory</div>
            <div className="session-detail-value" title={active.cwd}>{folder}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Coding tool</div>
            <div className="session-detail-value">{providerContext.codingTool}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Model</div>
            <div className="session-detail-value">{providerContext.model}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Model provider</div>
            <div className="session-detail-value">{providerContext.modelProvider}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Provider state</div>
            <div className="session-detail-value">{providerContext.providerState}</div>
          </div>
        </div>

        {active.branch && (
          <div className="detail-fact-git">
            <GitBranch size={13} />
            <span>{active.branch}</span>
            {active.isWorktree && <span className="git-worktree-tag">worktree</span>}
            <span className="git-state" data-state={active.dirty ? "dirty" : "clean"}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles !== undefined && active.changedFiles > 0 && (
              <span className="git-changed">{active.changedFiles} files changed</span>
            )}
          </div>
        )}
      </div>

      <div className="detail-provenance">
        {provenanceItems.map((item, i) => (
          <Fragment key={i}>
            {i > 0 && <span className="detail-provenance-sep">·</span>}
            {item}
          </Fragment>
        ))}
      </div>
    </div>
  );
}

export default SessionDetailPanel;
