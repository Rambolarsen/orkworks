import { Fragment, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { GitBranch } from "lucide-react";
import type { SessionInfo } from "../api";
import { sessionProviderContext } from "../sessionProviderContext";
import { sessionAttentionStatus } from "../sessionSort";
import {
  attentionLabel,
  attentionTone,
  detailActionZone,
  memoryStateLabel,
  relativeTime,
  situationHeadline,
  situationTail,
  sourceLabel,
  sourceWithConfidence,
  VOCAB,
} from "../labels";
import { pushToast } from "../feedback";
import EmptyState from "./EmptyState";
import StatusIndicator from "./StatusIndicator";
import ResumeChooser from "./ResumeChooser";

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
  const sourceTag = active.metadataSource;
  const providerContext = sessionProviderContext(active);
  const folder = active.cwd.split("/").pop() || active.cwd;
  const headline = situationHeadline(active);
  const tail = situationTail(active);
  const actionZone = detailActionZone(active, tone);
  const badgeText =
    attn === "capped" && active.usageLimitResetHint
      ? `Capped · ${active.usageLimitResetHint}`
      : attentionLabel(attn);

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
      {/* Surface 1 — situation hero: distilled "what's going on", never restating the row. */}
      <div className="detail-situation" data-attention={tone}>
        <div className="detail-situation-top">
          <span className="detail-badge" data-attention={tone}>
            <StatusIndicator tone={tone} label={attentionLabel(attn)} />
            {badgeText}
          </span>
          <span className="detail-situation-time">{relativeTime(active.peonLastInference, now) || relativeTime(active.created_at, now)}</span>
        </div>
        <div className="detail-headline">{headline}</div>
        {tail && (
          <div className="detail-tail" data-attention={tone}>{tail}</div>
        )}
        {active.conflictWarning && (
          <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
        )}
      </div>

      {/* Surface 2 — action zone: the one app-only move, never a duplicate of the terminal. */}
      {(active.recommendation || actionZone.kind !== "none") && (
        <div className="detail-actions">
          {active.recommendation && <div className="recommendation-text">{active.recommendation}</div>}

          {actionZone.kind === "cue" && (
            <div className="detail-cue" data-attention={tone}>
              <span className="detail-cue-arrow">&rarr;</span>
              {actionZone.text}
            </div>
          )}

          {actionZone.kind === "buttons" && (
            <div className="detail-button-row">
              {!!active.changedFiles && (
                <button
                  className="detail-button detail-button--primary"
                  type="button"
                  onClick={() => pushToast("info", VOCAB.diffReviewComingSoon)}
                >
                  {VOCAB.reviewDiffAction} (+{active.changedFiles})
                </button>
              )}
              <button
                className="detail-button detail-button--ghost"
                type="button"
                onClick={() => pushToast("info", VOCAB.markHandledComingSoon)}
              >
                {VOCAB.markHandledAction}
              </button>
            </div>
          )}

          {actionZone.kind === "resume" && (
            <>
              {/* Every option resumes via the same call for now — the backend doesn't accept a
                  strategy yet, so the choice is cosmetic until #97 lands. */}
              <ResumeChooser options={actionZone.options} onSelect={() => onResumeSession(active.id)} />
              {actionZone.note && <div className="detail-resume-note">{actionZone.note}</div>}
            </>
          )}
        </div>
      )}

      {/* Surface 3 — facts (demoted): everything the row and terminal don't say. */}
      <div className="detail-facts">
        <div className="detail-facts-grid">
          <div className="detail-fact">
            <div className="session-detail-label">Directory</div>
            <div className="session-detail-value" title={active.cwd}>{folder}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Provider state</div>
            <div className="session-detail-value">{providerContext.providerState}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Coding tool</div>
            <div className="session-detail-value">{providerContext.codingTool}</div>
          </div>
          <div className="detail-fact">
            <div className="session-detail-label">Model</div>
            <div className="session-detail-value">
              {providerContext.model}
              <span className="session-detail-value-sub">{providerContext.modelProvider}</span>
            </div>
          </div>
        </div>

        {active.branch && (
          <div className="detail-fact-git">
            <span className="git-branch-chip">
              <GitBranch size={11} />
              {active.branch}
            </span>
            {active.isWorktree && <span className="git-worktree-tag">worktree</span>}
            <span className="git-state" data-state={active.dirty ? "dirty" : "clean"}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles !== undefined && active.changedFiles > 0 && (
              <span className="git-changed">+{active.changedFiles} files</span>
            )}
          </div>
        )}
      </div>

      {/* Surface 4 — provenance footer. */}
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
