import { Fragment, useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { FileText, GitBranch, MessageCircle } from "lucide-react";
import { getSummaryLog } from "../api";
import type { SessionAttention, SessionInfo, SummaryLogEntry } from "../api";
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
import { useStableRelativeTimeNow } from "../useStableRelativeTimeNow";
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
  onReviewPlan: () => void;
  showDebugMetadata: boolean;
}

function SessionDetailPanel({ sessions, activeSessionId, onResumeSession, onApplyDebugAttention, showDebugMetadata, onReviewPlan }: SessionDetailPanelProps) {
  const [debugAttention, setDebugAttention] = useState<SessionAttention>("working");
  const [debugMessage, setDebugMessage] = useState("");
  const [reviewingSessionId, setReviewingSessionId] = useState<string | null>(null);
  const [summaryLog, setSummaryLog] = useState<SummaryLogEntry[]>([]);
  const [summaryLogSessionId, setSummaryLogSessionId] = useState<string | null>(null);
  const active = sessions.find((s) => s.id === activeSessionId);
  const now = useStableRelativeTimeNow(useCallback((currentNow: Date) => {
    if (!active) return null;
    let nextRefresh = nextRelativeTimeRefreshMs(active.peonLastInference, currentNow);
    nextRefresh = minDelay(nextRefresh, nextRelativeTimeRefreshMs(active.lastActivityAt, currentNow));
    nextRefresh = minDelay(nextRefresh, nextRelativeTimeRefreshMs(active.created_at, currentNow));
    return nextRefresh;
  }, [active?.id, active?.peonLastInference, active?.lastActivityAt, active?.created_at]));

  // Reset synchronously (during render, not in an effect) so a session
  // switch never paints the previous session's task history under the new
  // one's header while the fetch below is still in flight.
  if (active && active.id !== summaryLogSessionId) {
    setSummaryLogSessionId(active.id);
    setSummaryLog([]);
  }

  useEffect(() => {
    if (!active) return;
    let current = true;
    void window.orkworks.getBackendUrl()
      .then((baseUrl) => getSummaryLog(baseUrl, active.id))
      .then((entries) => { if (current) setSummaryLog(entries); })
      .catch(() => { if (current) setSummaryLog([]); });
    return () => { current = false; };
    // lastActivityAt advances for every summary-checkpoint source (Peon
    // inference and agent-hook attention reports alike), unlike
    // peonLastInference, which only advances for Peon's own inferences.
  }, [active?.id, active?.lastActivityAt]);

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
          <span className="detail-situation-time">{relativeTime(active.lastActivityAt, now) || relativeTime(active.created_at, now)}</span>
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
      {(active.recommendation || actionZone.kind !== "none" || active.hasOpenablePlan) && (
        <div className="detail-actions">
          {active.recommendation && <div className="recommendation-text">{active.recommendation}</div>}

          {active.hasOpenablePlan && (
            <div className="resume-chooser">
              <div className="resume-chooser-title">
                {tone === "needs-you" ? "Plan ready for review" : "Plan available"}
              </div>
              <button type="button" className="resume-option resume-option--recommended" onClick={onReviewPlan}>
                <span className="resume-option-icon"><FileText size={14} aria-hidden="true" /></span>
                <span className="resume-option-body">
                  <span className="resume-option-label">Review plan</span>
                </span>
              </button>
              {active.lifecycle === "alive" && (
                <button
                  type="button"
                  className={`resume-option${reviewingSessionId === active.id ? " resume-option--unavailable" : ""}`}
                  disabled={reviewingSessionId === active.id}
                  onClick={() => {
                    if (reviewingSessionId === active.id) return;
                    setReviewingSessionId(active.id);
                    void window.orkworks.requestPlanReview(active.id)
                      .catch((error: unknown) => pushToast("error", error instanceof Error ? error.message : "Couldn't request review."))
                      .finally(() => setReviewingSessionId((id) => id === active.id ? null : id));
                  }}
                >
                  <span className="resume-option-icon"><MessageCircle size={14} aria-hidden="true" /></span>
                  <span className="resume-option-body">
                    <span className="resume-option-label">
                      {reviewingSessionId === active.id ? "Requesting review…" : "Request independent review"}
                    </span>
                    <span className="resume-option-sub">Asks a separate subagent to check it, when the tooling supports one</span>
                  </span>
                </button>
              )}
            </div>
          )}

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

          {actionZone.kind === "plan" && !active.hasOpenablePlan && (
            <button
              className="detail-button detail-button--primary"
              type="button"
              onClick={onReviewPlan}
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
              <div className="peon-diagnostics">
                <div className="peon-diagnostics-title">Peon diagnostics</div>
                <dl className="peon-diagnostics-grid">
                  <div>
                    <dt>Scheduler state</dt>
                    <dd>{active.peonDiagnostics?.schedulerState ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Reason</dt>
                    <dd>{active.peonDiagnostics?.reason ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Last attempt</dt>
                    <dd>{active.peonDiagnostics?.lastAttemptAt ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Last successful inference</dt>
                    <dd>{active.peonDiagnostics?.lastSuccessfulInferenceAt ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Provider ID</dt>
                    <dd>{active.peonDiagnostics?.providerId ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Provider model</dt>
                    <dd>{active.peonDiagnostics?.providerModel ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Fallback step</dt>
                    <dd>{active.peonDiagnostics?.fallbackStep ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Attempt count</dt>
                    <dd>{active.peonDiagnostics?.attemptCount ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Error summary</dt>
                    <dd>{active.peonDiagnostics?.errorSummary ?? "—"}</dd>
                  </div>
                  <div>
                    <dt>Accepted observations</dt>
                    <dd>{active.peonDiagnostics?.observationCount ?? "Unavailable"}</dd>
                  </div>
                </dl>
              </div>
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

      {/* Surface 4 — task history: durable checkpoints of what the session has done, distinct from the live headline above. */}
      {summaryLog.length > 0 && (
        <div className="detail-task-history">
          <div className="detail-task-history-title">Task history</div>
          <ul className="detail-task-history-list">
            {summaryLog.map((entry, i) => (
              <li key={i} className="detail-task-history-item">
                <span className="detail-task-history-time">
                  {relativeTime(entry.timestamp, now) || entry.timestamp}
                </span>
                <span className="detail-task-history-summary">{entry.summary}</span>
                <SourceBadge source={entry.source}>
                  {sourceWithConfidence(entry.source, entry.confidence ?? undefined)}
                </SourceBadge>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Surface 5 — provenance footer. */}
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
