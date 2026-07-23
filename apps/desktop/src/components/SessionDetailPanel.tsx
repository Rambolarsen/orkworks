import { Fragment, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { GitBranch } from "lucide-react";
import type { SessionAttention, SessionInfo } from "../api";
import { sessionProviderContext } from "../sessionProviderContext";
import { sessionAttentionStatus } from "../sessionSort";
import {
  attentionLabel,
  attentionTone,
  detailActionZone,
  lifecyclePhaseLabel,
  memoryStateLabel,
  minDelay,
  nextRelativeTimeRefreshMs,
  relativeTime,
  situationHeadline,
  situationTail,
  sourceWithConfidence,
  VOCAB,
  workPhaseLabel,
} from "../labels";
import { pushToast } from "../feedback";
import DetailField from "./DetailField";
import EmptyState from "./EmptyState";
import SourceBadge from "./SourceBadge";
import StatusIndicator from "./StatusIndicator";
import ResumeChooser from "./ResumeChooser";

const DEBUG_ATTENTION_OPTIONS: SessionAttention[] = ["working", "idle", "needs_you", "blocked", "failed", "capped"];

interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onResumeSession: (id: string) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  showDebugMetadata: boolean;
}

function SessionDetailPanel({ sessions, activeSessionId, onResumeSession, onApplyDebugAttention, showDebugMetadata }: SessionDetailPanelProps) {
  const [now, setNow] = useState(() => new Date());
  const [debugAttention, setDebugAttention] = useState<SessionAttention>("working");
  const [debugMessage, setDebugMessage] = useState("");
  const active = sessions.find((s) => s.id === activeSessionId);

  useEffect(() => {
    if (!active) return;
    let nextRefresh = nextRelativeTimeRefreshMs(active.peonLastInference, now);
    nextRefresh = minDelay(nextRefresh, nextRelativeTimeRefreshMs(active.created_at, now));
    if (nextRefresh === null) return;
    const timeout = window.setTimeout(() => setNow(new Date()), nextRefresh);
    return () => window.clearTimeout(timeout);
  }, [active, now]);

  if (!active) {
    return <EmptyState message="Select an agent session to see details." />;
  }

  const attn = sessionAttentionStatus(active);
  const tone = attentionTone(attn);
  const sourceTag = active.metadataSource;
  const providerContext = sessionProviderContext(active);
  const folder = active.cwd.split("/").pop() || active.cwd;
  const headline = situationHeadline(active);
  const tail = situationTail(active, tone);
  const actionZone = detailActionZone(active, tone);
  const badgeText =
    attn === "capped" && active.usageLimitResetHint
      ? `Capped · ${active.usageLimitResetHint}`
      : attentionLabel(attn);

  const provenanceItems: ReactNode[] = [];
  if (sourceTag) {
    provenanceItems.push(
      <SourceBadge key="source" source={sourceTag}>
        {sourceWithConfidence(sourceTag, active.metadataConfidence)}
      </SourceBadge>,
    );
  }
  if (active.peonLastInference) {
    provenanceItems.push(
      <span key="peon" className="peon-value">
        Observed {relativeTime(active.peonLastInference, now) || active.peonLastInference}
      </span>,
    );
  }
  if (showDebugMetadata && active.finalObservedStatus) {
    provenanceItems.push(
      <span key="final-attention" className="peon-value">
        Final attention: {attentionLabel(active.finalObservedStatus)}
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

          {actionZone.kind === "plan" && (
            <button
              className="detail-button detail-button--primary"
              type="button"
              onClick={() => void window.orkworks.openPlan(active.id).catch((error: unknown) => {
                pushToast("error", error instanceof Error ? error.message : "Couldn't open plan.");
              })}
            >
              Open plan
            </button>
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
          <DetailField className="detail-fact" label="Directory" valueTitle={active.cwd}>
            {folder}
          </DetailField>
          <DetailField className="detail-fact" label="Provider state">
            {providerContext.providerState}
          </DetailField>
          <DetailField className="detail-fact" label="Coding tool">
            {providerContext.codingTool}
          </DetailField>
          <DetailField className="detail-fact" label="Model">
            {providerContext.model}
            <span className="session-detail-value-sub">{providerContext.modelProvider}</span>
          </DetailField>
          {showDebugMetadata && (
            <>
              <DetailField className="detail-fact" label="Work phase">
                {workPhaseLabel(active.workPhase)}
              </DetailField>
              <DetailField className="detail-fact" label="Lifecycle">
                {lifecyclePhaseLabel(active.lifecyclePhase)}
              </DetailField>
              <DetailField className="detail-fact" label="OrkWorks session ID">
                {active.id}
              </DetailField>
              <DetailField className="detail-fact" label="Harness session ID">
                {active.resume?.harnessSessionId ?? "Not captured"}
              </DetailField>
              {active.lifecycle === "alive" && (
                <DetailField className="detail-fact" label="Debug attention injection">
                  <div className="debug-injection">
                    <select
                      className="debug-injection-select"
                      value={debugAttention}
                      onChange={(e) => setDebugAttention(e.target.value as SessionAttention)}
                    >
                      {DEBUG_ATTENTION_OPTIONS.map((value) => (
                        <option key={value} value={value}>{attentionLabel(value)}</option>
                      ))}
                    </select>
                    {debugAttention === "capped" && (
                      <input
                        type="text"
                        className="debug-injection-message"
                        placeholder="Reset hint (optional)"
                        value={debugMessage}
                        onChange={(e) => setDebugMessage(e.target.value)}
                      />
                    )}
                    <button
                      type="button"
                      className="debug-injection-apply"
                      onClick={() => onApplyDebugAttention(active.id, debugAttention, debugMessage.trim() || undefined)}
                    >
                      Inject
                    </button>
                  </div>
                </DetailField>
              )}
            </>
          )}
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
