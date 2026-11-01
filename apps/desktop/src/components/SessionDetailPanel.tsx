import { gitLocationLabel } from "../gitLocationLabel";
import { isSessionAlive } from "../sessionRuntimeState";
import { Fragment, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { ArrowLeft, ChevronRight, FileText, GitBranch, History, MessageCircle } from "lucide-react";
import { getSessionWorkflowObservations, getSummaryLog } from "../api";
import type { SessionAttention, SessionInfo, SummaryLogEntry, WorkflowObservationEntry } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import { nativeVoicePresentation } from "../nativeVoicePresentation";
import { sessionCodingTool, sessionProviderContext } from "../sessionProviderContext";
import { sessionAttentionStatus } from "../sessionSort";
import {
  attentionLabel,
  attentionTone,
  detailActionZone,
  lifecyclePhaseLabel,
  memoryStateLabel,
  minDelay,
  nextRelativeTimeRefreshMs,
  peonTimeoutNotice,
  relativeTime,
  sourceWithConfidence,
  workPhaseLabel,
} from "../labels";
import { pushToast } from "../feedback";
import { useStableRelativeTimeNow } from "../useStableRelativeTimeNow";
import DetailField from "./DetailField";
import EmptyState from "./EmptyState";
import SourceBadge from "./SourceBadge";
import StatusIndicator from "./StatusIndicator";
import ResumeChooser from "./ResumeChooser";
import SessionTitleEditor from "./SessionTitleEditor";
import BranchDocumentSteps from "./BranchDocumentSteps";
import type { PlanDocumentKind } from "../api";

const DEBUG_ATTENTION_OPTIONS: SessionAttention[] = ["working", "idle", "needs_you", "blocked", "failed", "capped"];

interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  visibleSessionId: string | null;
  harnesses: HarnessConfig[];
  resumingSessionIds?: ReadonlySet<string>;
  onResumeSession: (id: string, strategy?: import("../api").ResumeStrategy) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  onOpenSettings: () => void;
  onReviewPlan?: (kind?: PlanDocumentKind) => void;
  onReviewChanges?: () => void;
  onRequestPlanReview?: (sessionId: string) => Promise<void>;
  onOpenRecommendation?: (id: string) => void;
  showDebugMetadata: boolean;
  workspaceIdentity?: string;
  onTitleUpdated?: () => void;
}

function SessionDetailPanel({ sessions, visibleSessionId, harnesses, resumingSessionIds, onResumeSession, onApplyDebugAttention, onOpenSettings, showDebugMetadata, onReviewChanges, onReviewPlan, onRequestPlanReview, onOpenRecommendation, workspaceIdentity, onTitleUpdated }: SessionDetailPanelProps) {
  const [debugAttention, setDebugAttention] = useState<SessionAttention>("working");
  const [debugMessage, setDebugMessage] = useState("");
  const pendingReviews = useRef(new Set<string>());
  const [reviewingSessionId, setReviewingSessionId] = useState<string | null>(null);
  const [summaryLog, setSummaryLog] = useState<SummaryLogEntry[]>([]);
  const [summaryLogSessionId, setSummaryLogSessionId] = useState<string | null>(null);
  const [workflowObservations, setWorkflowObservations] = useState<WorkflowObservationEntry[]>([]);
  const [workflowObservationsSessionId, setWorkflowObservationsSessionId] = useState<string | null>(null);
  const [activitySessionId, setActivitySessionId] = useState<string | null>(null);
  const activityButton = useRef<HTMLButtonElement>(null);
  const backButton = useRef<HTMLButtonElement>(null);
  const menuFocus = useRef<"activity" | "back" | null>(null);
  const active = sessions.find((s) => s.id === visibleSessionId);
  const showingActivity = !!active && activitySessionId === active.id;
  // An inspected subject change must never restore the previous subject's subpage.
  if (activitySessionId !== null && activitySessionId !== active?.id) setActivitySessionId(null);
  useLayoutEffect(() => {
    if (menuFocus.current === "back" && showingActivity) backButton.current?.focus();
    else if (menuFocus.current === "activity") activityButton.current?.focus();
    menuFocus.current = null;
  }, [showingActivity]);
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
  if (active && active.id !== workflowObservationsSessionId) {
    setWorkflowObservationsSessionId(active.id);
    setWorkflowObservations([]);
  }

  useEffect(() => {
    if (!active) return;
    let current = true;
    const loadSummaryLog = () => {
      void window.orkworks.getBackendUrl()
        .then((baseUrl) => getSummaryLog(baseUrl, active.id))
        .then((entries) => { if (current) setSummaryLog(entries); })
        .catch(() => { if (current) setSummaryLog([]); });
    };
    loadSummaryLog();
    const summaryLogTimer = window.setInterval(loadSummaryLog, 5000);
    return () => {
      current = false;
      window.clearInterval(summaryLogTimer);
    };
    // lastActivityAt advances for every summary-checkpoint source (Peon
    // inference and agent-hook attention reports alike), unlike
    // peonLastInference, which only advances for Peon's own inferences.
  }, [active?.id, active?.lastActivityAt]);

  useEffect(() => {
    if (!active || !showDebugMetadata) return;
    let current = true;
    const loadWorkflowObservations = () => {
      void window.orkworks.getBackendUrl()
        .then((baseUrl) => getSessionWorkflowObservations(baseUrl, active.id))
        .then((observations) => { if (current) setWorkflowObservations(observations); })
        .catch(() => { if (current) setWorkflowObservations([]); });
    };
    loadWorkflowObservations();
    const workflowObservationsTimer = window.setInterval(loadWorkflowObservations, 5000);
    return () => {
      current = false;
      window.clearInterval(workflowObservationsTimer);
    };
  }, [active?.id, active?.lastActivityAt, showDebugMetadata]);

  if (!active) {
    return <EmptyState message="Select an agent session to see details." />;
  }

  const attn = sessionAttentionStatus(active);
  const tone = attentionTone(attn);
  const sourceTag = active.attentionSource ?? active.metadataSource;
  const sourceConfidence = active.attentionSource === undefined ? active.metadataConfidence : active.attentionConfidence;
  const providerContext = sessionProviderContext(active);
  const voice = nativeVoicePresentation(sessionCodingTool(active), harnesses);
  const folder = active.cwd.split("/").pop() || active.cwd;
  const timeoutNotice = peonTimeoutNotice(active);
  const actionZone = detailActionZone(active, tone);
  const badgeText =
    attn === "capped" && active.usageLimitResetHint
      ? `Capped · ${active.usageLimitResetHint}`
      : attentionLabel(attn);

  const provenanceItems: ReactNode[] = [];
  if (sourceTag) {
    provenanceItems.push(
      <SourceBadge key="source" source={sourceTag}>
        {sourceWithConfidence(sourceTag, sourceConfidence)}
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
      {showingActivity ? (
        <button type="button" className="session-menu-back" data-session-menu-back ref={backButton}
          onClick={() => { menuFocus.current = "activity"; setActivitySessionId(null); }}>
          <ArrowLeft size={14} aria-hidden="true" /> Session menu
        </button>
      ) : (
        <nav className="session-menu" aria-label="Session navigation">
          {(!active.planDocuments || (active.hasOpenablePlan && !active.planDocuments.steps.some(step => step.available))) && <button type="button" className="session-menu-item" data-session-plan
            disabled={!active.hasOpenablePlan || !onReviewPlan}
            title={active.hasOpenablePlan ? "Review this session's plan" : "No available plan for this session"}
            onClick={() => onReviewPlan?.()}>
            <span className="session-menu-icon"><FileText size={16} aria-hidden="true" /></span>
            <span className="session-menu-text"><span className="session-menu-label">Plan</span>
              <span className="session-menu-description">{active.hasOpenablePlan ? "Available for review" : "No plan available"}</span></span>
            <ChevronRight size={14} aria-hidden="true" />
          </button>}
          <button type="button" className="session-menu-item" data-review-changes
            disabled={!(active.repoRoot || active.branch) || !onReviewChanges}
            title={active.repoRoot || active.branch ? "Inspect this worktree and committed branch changes" : "No available Git context for this session"}
            onClick={onReviewChanges}>
            <span className="session-menu-icon"><GitBranch size={16} aria-hidden="true" /></span>
            <span className="session-menu-text"><span className="session-menu-label">Changes</span>
              <span className="session-menu-description">{active.repoRoot || active.branch ? "Worktree and branch diff" : "Git context unavailable"}</span></span>
            {active.changedFiles != null && <span className="session-menu-count">{active.changedFiles} {active.changedFiles === 1 ? "file" : "files"}</span>}
            <ChevronRight size={14} aria-hidden="true" />
          </button>
          <button type="button" className="session-menu-item" data-session-activity ref={activityButton}
            onClick={() => { menuFocus.current = "back"; setActivitySessionId(active.id); }}>
            <span className="session-menu-icon"><History size={16} aria-hidden="true" /></span>
            <span className="session-menu-text"><span className="session-menu-label">Activity</span>
              <span className="session-menu-description">Recent session checkpoints</span></span>
            <ChevronRight size={14} aria-hidden="true" />
          </button>
        </nav>
      )}
      {!showingActivity && active.planDocuments && onReviewPlan && <BranchDocumentSteps key={active.id}
        sessionId={active.id} documents={active.planDocuments} onOpen={kind => onReviewPlan(kind)}
        onUseLatest={async (kind, selectionId) => { await window.orkworks.useLatestPlanDocument(active.id, kind, selectionId); onTitleUpdated?.(); }} />}
      {!showingActivity && actionZone.kind === "resume" && (
        <div className="detail-actions">
          <ResumeChooser key={active.id} pending={resumingSessionIds?.has(active.id)} options={actionZone.options} onSelect={option => onResumeSession(active.id, option.strategy)} />
          {actionZone.note && <div className="detail-resume-note">{actionZone.note}</div>}
        </div>
      )}
      {!showingActivity && <>
        {active.conflictWarning && (
          <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
        )}
        {timeoutNotice && (
          <div className="peon-timeout-warning" role="alert">
            <div>{timeoutNotice.message}</div>
            <button type="button" onClick={onOpenSettings}>Open provider settings</button>
          </div>
        )}
      <details className="session-info" key={active.id}>
        <summary>Session info</summary>
      <div className="detail-facts">
        <div className="detail-situation-top">
          <span className="detail-badge" data-attention={tone}>
            <StatusIndicator tone={tone} label={attentionLabel(attn)} />{badgeText}
          </span>
          <span className="detail-situation-time">{relativeTime(active.lastActivityAt, now) || relativeTime(active.created_at, now)}</span>
        </div>
        {active.recommendation && <p className="detail-muted">{active.recommendation}</p>}
        {actionZone.kind === "cue" && <p className="detail-muted">{actionZone.text}</p>}
        {active.hasOpenablePlan && onReviewPlan && !active.planDocuments?.steps.some(step => step.available) && (
          <div className="detail-actions">
              {isSessionAlive(active) && (
                <button
                  type="button"
                  className={`resume-option${reviewingSessionId === active.id ? " resume-option--unavailable" : ""}`}
                  disabled={reviewingSessionId === active.id}
                  onClick={() => {
                    if (pendingReviews.current.has(active.id)) return;
                    pendingReviews.current.add(active.id);
                    setReviewingSessionId(active.id);
                    void (onRequestPlanReview ?? ((id: string) => window.orkworks.requestPlanReview(id)))(active.id)
                      .catch((error: unknown) => pushToast("error", error instanceof Error ? error.message : "Couldn't request review."))
                      .finally(() => {
                        pendingReviews.current.delete(active.id);
                        setReviewingSessionId((id) => id === active.id ? null : id);
                      });
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
        {workspaceIdentity && <SessionTitleEditor key={active.id} sessionId={active.id} title={active.label} workspaceIdentity={workspaceIdentity} onUpdated={onTitleUpdated} />}
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
          {voice && (
            <DetailField className="detail-fact" label="Voice">
              {voice.label}
              <span className="session-detail-value-sub">{voice.detail}</span>
            </DetailField>
          )}
          {active.sessionState !== undefined && <DetailField className="detail-fact" label="Session state">
            {active.sessionState === "archived" ? "Archived" : active.sessionState === "closed" ? "Closed" : active.sessionState === "active" ? "Active" : "Unavailable"}
            {active.sessionStateError && <span className="session-detail-value-sub">{active.sessionStateError}</span>}
          </DetailField>}
          {showDebugMetadata && (
            <>
              <DetailField className="detail-fact" label="Work phase">
                {workPhaseLabel(active.workPhase)}
              </DetailField>
              <DetailField className="detail-fact" label="Lifecycle">
                {lifecyclePhaseLabel(active.lifecyclePhase)}
              </DetailField>
              <DetailField className="detail-fact detail-fact--id" label="OrkWorks session ID">
                {active.id}
              </DetailField>
              <DetailField className="detail-fact detail-fact--id" label="Harness session ID">
                {active.resume?.harnessSessionId ?? "Not captured"}
              </DetailField>
              {isSessionAlive(active) && (
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
              {workflowObservations.length > 0 && (
                <div className="detail-workflow-observations">
                  <div className="detail-workflow-observations-title">Workflow observations</div>
                  <ul className="detail-workflow-observations-list">
                    {workflowObservations.map((entry) => (
                      <li key={entry.id} className="detail-workflow-observations-item">
                        <span className="detail-workflow-observations-time">
                          {relativeTime(entry.observedAt, now) || entry.observedAt}
                        </span>
                        <span className="detail-workflow-observations-kind">{entry.kind}</span>
                        <span className="detail-workflow-observations-description">{entry.description}</span>
                        <span className="detail-workflow-observations-impact">{entry.reportedImpact} impact</span>
                        <SourceBadge source={entry.source}>
                          {sourceWithConfidence(entry.source, entry.confidence)}
                        </SourceBadge>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </>
          )}
        </div>

        {(active.repoRoot || active.branch) && (
          <div className="detail-fact-git">
            <span className="git-branch-chip">
              <GitBranch size={11} />
              {active.branch || "Git repository"}
            </span>
            {active.isWorktree && <span className="git-worktree-tag">worktree</span>}
            <span className="git-state" data-state={active.dirty ? "dirty" : "clean"}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles != null && (
              <span className="git-changed" title="Live uncommitted changes in this worktree, including staged edits and new files. Shared by sessions using this worktree.">
                {`${active.changedFiles} ${active.changedFiles === 1 ? "file" : "files"}`}
                {active.lineChanges && ` · +${active.lineChanges.additions} −${active.lineChanges.deletions}`}
              </span>
            )}
          </div>
        )}
      </div>

      {(active.gitContextSource || active.worktreeBinding) && <p className="detail-muted">
        {gitLocationLabel(active.gitContextSource)}{active.worktreeBinding && !active.repoRoot && !active.branch ? " unavailable" : ""}
      </p>}
      {/* Keep observation authority alongside the inspected session facts. */}
      <div className="detail-provenance">
        {provenanceItems.map((item, i) => (
          <Fragment key={i}>
            {i > 0 && <span className="detail-provenance-sep">·</span>}
            {item}
          </Fragment>
        ))}
      </div>
      </details>
      </>}
      {/* Activity contains durable checkpoints for this subject only. */}
      {showingActivity && (
        <div className="detail-task-history">
          <h3 className="detail-task-history-title">Activity</h3>
          {summaryLog.length === 0 && <p className="detail-muted">No activity checkpoints available.</p>}
          <ul className="detail-task-history-list">
            {summaryLog.map((entry, i) => (
              <li key={i} className="detail-task-history-item">
                <span className="detail-task-history-time">
                  {relativeTime(entry.timestamp, now) || entry.timestamp}
                </span>
                <span className="detail-task-history-summary">{entry.summary}</span>
                {entry.recommendationId && (
                  <button
                    className="detail-task-history-recommendation"
                    type="button"
                    onClick={() => onOpenRecommendation?.(entry.recommendationId!)}
                  >
                    Recommendation {entry.recommendationId.replace(/^recommendation-/, "").slice(0, 8)}
                  </button>
                )}
                <SourceBadge source={entry.source}>
                  {sourceWithConfidence(entry.source, entry.confidence ?? undefined)}
                </SourceBadge>
              </li>
            ))}
          </ul>
        </div>
      )}


    </div>
  );
}

export default SessionDetailPanel;
