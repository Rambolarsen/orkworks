import { useCallback, useEffect, useRef, useState } from "react";
import {
  ApiError,
  dismissTaskmasterRecommendation,
  getTaskmasterRecommendation,
  getTaskmasterRecommendations,
  type ObservationDiagnostic,
  type WorkflowRecommendation,
} from "../api.ts";
import {
  formatImpact,
  formatPacketEvidence,
  formatPacketReadiness,
  formatRecurrence,
  formatTargetSurface,
} from "../taskmaster.ts";
import EmptyState from "./EmptyState";
import RecommendationEvidence from "./RecommendationEvidence";

interface RecommendationsPanelProps {
  hasWorkspace: boolean;
  taskmasterReady: boolean;
  canFixWithAi: boolean;
  onSelectSession?: (id: string) => void;
  onFixWithAi?: (recommendation: WorkflowRecommendation) => void;
  focusedRecommendationId?: string | null;
}

function isActiveBrainRecommendation(recommendation: WorkflowRecommendation): boolean {
  const brainDerived = recommendation.dedupeKey.startsWith("proactive:v1:")
    || recommendation.dedupeKey.startsWith("rollup:v1:");
  return brainDerived
    && recommendation.type === "improve_workflow"
    && (recommendation.status === "proposed"
      || recommendation.status === "accepted"
      || recommendation.status === "executing");
}

function activeRecommendationMessage(recommendation: WorkflowRecommendation): string {
  return recommendation.status === "proposed"
    ? "A Brain recommendation is already waiting. Implement it with Fix with AI before requesting another analysis."
    : "A Brain recommendation is already being handled. Complete it before requesting another analysis.";
}

function DiagnosticList({ diagnostics }: { diagnostics: ObservationDiagnostic[] }) {
  if (diagnostics.length === 0) return null;
  return (
    <div className="recommendations-diagnostics" role="status">
      <strong>Evidence diagnostics</strong>
      {diagnostics.map((diagnostic) => (
        <p key={`${diagnostic.code}-${diagnostic.sessionId ?? "workspace"}`}>{diagnostic.message}</p>
      ))}
    </div>
  );
}

function RecommendationCard({
  recommendation,
  onDismiss,
  onSelectSession,
  onFixWithAi,
  canFixWithAi,
  dismissing,
  error,
  focused,
}: {
  recommendation: WorkflowRecommendation;
  onDismiss: (id: string) => void;
  onSelectSession?: (id: string) => void;
  onFixWithAi?: (recommendation: WorkflowRecommendation) => void;
  canFixWithAi: boolean;
  dismissing: boolean;
  error?: string;
  focused?: boolean;
}) {
  const improvement = recommendation.workflowImprovement;
  return (
    <article className={`recommendation-card${focused ? " recommendation-card--focused" : ""}`}>
      <header className="recommendation-card-header">
        <div>
          <h3>{recommendation.title}</h3>
          <span className="recommendation-target">{formatTargetSurface(improvement.targetSurface)}</span>
        </div>
        <span className={`recommendation-impact recommendation-impact--${recommendation.priority}`}>
          {formatImpact(recommendation.priority)} impact
        </span>
      </header>
      <div className="recommendation-status-row">
        <span className={`recommendation-status recommendation-status--${recommendation.status}`}>
          {recommendation.status}
        </span>
        <span className="recommendation-updated">Updated {recommendation.updatedAt}</span>
      </div>
      <p className="recommendation-proposal">{improvement.proposedImprovement}</p>
      <p className="recommendation-reason">{recommendation.reason.join(" ")}</p>
      <ResurfacedLineage recommendation={recommendation} />
      {recommendation.rollupMemberIds.length > 0 && (
        <p className="recommendation-rollup-meta">
          Rollup of {recommendation.rollupMemberIds.length} exact families · {formatRecurrence(recommendation)}
          {recommendation.rollupGeneration === null ? "" : ` · Generation ${recommendation.rollupGeneration}`}
        </p>
      )}
      <dl className="recommendation-facts">
        <div><dt>Confidence</dt><dd>{formatImpact(recommendation.confidence)}</dd></div>
        <div><dt>{recommendation.rollupMemberIds.length > 0 ? "Combined evidence" : "Evidence origin"}</dt><dd>{recommendation.evidence.length ? formatRecurrence(recommendation) : "Repository discovery"}</dd></div>
        <div><dt>Expected benefit</dt><dd>{improvement.expectedBenefit}</dd></div>
      </dl>
      {recommendation.completionPacket && (
        <section className="completion-packet" aria-label="Completion packet">
          <div className="completion-packet-header">
            <strong>Completion packet</strong>
            <span className="recommendation-status">{formatPacketReadiness(recommendation.completionPacket.readiness)}</span>
          </div>
          <p>{formatPacketEvidence(recommendation.completionPacket)}</p>
          <p>Observed {recommendation.completionPacket.observedAt} · revision {recommendation.completionPacket.revision}</p>
          {(recommendation.completionPacket.missingEvidence.length > 0 || recommendation.completionPacket.conflictingEvidence.length > 0) && (
            <ul>
              {[...recommendation.completionPacket.missingEvidence, ...recommendation.completionPacket.conflictingEvidence].map((issue, index) => (
                <li key={`${issue.kind}-${index}`}>{issue.kind}: {issue.detail}</li>
              ))}
            </ul>
          )}
        </section>
      )}
      <div className="recommendation-sessions">
        {improvement.affectedSessionIds.map((sessionId) => (
          <button key={sessionId} type="button" onClick={() => onSelectSession?.(sessionId)}>
            Session {sessionId.slice(0, 8)}
          </button>
        ))}
      </div>
      <RecommendationEvidence recommendation={recommendation} onSelectSession={onSelectSession} />
      {error && <p className="recommendation-error" role="alert">{error}</p>}
      {recommendation.status === "proposed" && (
        <div className="recommendation-actions">
          <button
            className="recommendation-fix"
            type="button"
            disabled={dismissing || !canFixWithAi}
            title={canFixWithAi ? undefined : "Open a session to send this fix to"}
            onClick={() => onFixWithAi?.(recommendation)}
          >
            Fix with AI
          </button>
          <button className="recommendation-dismiss" type="button" disabled={dismissing} onClick={() => onDismiss(recommendation.id)}>
            {dismissing ? "Dismissing…" : "Dismiss"}
          </button>
        </div>
      )}
    </article>
  );
}

function RecommendationsPanel({ hasWorkspace, taskmasterReady, canFixWithAi, onSelectSession, onFixWithAi, focusedRecommendationId }: RecommendationsPanelProps) {
  const [recommendations, setRecommendations] = useState<WorkflowRecommendation[]>([]);
  const [diagnostics, setDiagnostics] = useState<ObservationDiagnostic[]>([]);
  const [error, setError] = useState<string>();
  const [dismissing, setDismissing] = useState<string>();
  const [dismissErrors, setDismissErrors] = useState<Record<string, string>>({});
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [analysisMessage, setAnalysisMessage] = useState<string>();
  const [analysisError, setAnalysisError] = useState<string>();
  const [blockedRecommendation, setBlockedRecommendation] = useState<WorkflowRecommendation>();
  const [blockedRecommendationId, setBlockedRecommendationId] = useState<string>();
  const [blockedRecommendationRecoveryAllowed, setBlockedRecommendationRecoveryAllowed] = useState(false);
  const refreshGeneration = useRef(0);
  const workspaceGeneration = useRef(0);

  const refresh = useCallback(async () => {
    if (!hasWorkspace || !taskmasterReady) return;
    const generation = ++refreshGeneration.current;
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      if (!hasWorkspace || !taskmasterReady || generation !== refreshGeneration.current) return;
      const response = await getTaskmasterRecommendations(baseUrl);
      let nextRecommendations = response.recommendations;
      if (blockedRecommendationId) {
        try {
          const blocked = await getTaskmasterRecommendation(baseUrl, blockedRecommendationId);
          if (generation !== refreshGeneration.current) return;
          if (isActiveBrainRecommendation(blocked)) {
            setBlockedRecommendation(blocked);
          } else {
            setBlockedRecommendation(undefined);
            setBlockedRecommendationId(undefined);
            setBlockedRecommendationRecoveryAllowed(false);
            setAnalysisMessage(undefined);
          }
        } catch (cause) {
          if (cause instanceof ApiError && cause.status === 404) {
            if (generation !== refreshGeneration.current) return;
            setBlockedRecommendation(undefined);
            setBlockedRecommendationId(undefined);
            setBlockedRecommendationRecoveryAllowed(false);
            setAnalysisMessage(undefined);
          }
        }
      }
      if (
        focusedRecommendationId
        && !nextRecommendations.some((item) => item.id === focusedRecommendationId)
      ) {
        try {
          const detail = await getTaskmasterRecommendation(baseUrl, focusedRecommendationId);
          if (generation !== refreshGeneration.current) return;
          nextRecommendations = [...nextRecommendations, detail];
        } catch {
          // A stale history link should not make the actionable list fail.
        }
      }
      if (generation !== refreshGeneration.current) return;
      setRecommendations(nextRecommendations);
      setDiagnostics(response.diagnostics);
      setError(undefined);
    } catch (cause) {
      if (generation !== refreshGeneration.current) return;
      setError(cause instanceof Error ? cause.message : "Couldn't load recommendations.");
    }
  }, [blockedRecommendationId, focusedRecommendationId, hasWorkspace, taskmasterReady]);

  useEffect(() => {
    // The panel mounts as part of the default layout, before the sidecar's
    // async /workspace bootstrap has necessarily completed, and hasWorkspace
    // also drops while App.tsx is mid-switch to a different workspace (see
    // isSwitchingWorkspace there). Fetching in either window races a 409
    // (no workspace set yet on the current/new sidecar) into the console
    // and the error banner, so wait for a ready workspace instead — and
    // drop whatever was on screen for the previous one rather than leaving
    // it visible until the next successful poll.
    if (!hasWorkspace) {
      ++refreshGeneration.current;
      ++workspaceGeneration.current;
      setRecommendations([]);
      setDiagnostics([]);
      setBlockedRecommendation(undefined);
      setBlockedRecommendationId(undefined);
      setBlockedRecommendationRecoveryAllowed(false);
      setAnalysisBusy(false);
      setAnalysisMessage(undefined);
      setAnalysisError(undefined);
      setError(undefined);
      return;
    }
    if (!taskmasterReady) {
      ++refreshGeneration.current;
      ++workspaceGeneration.current;
      setRecommendations([]);
      setDiagnostics([]);
      setBlockedRecommendation(undefined);
      setBlockedRecommendationId(undefined);
      setBlockedRecommendationRecoveryAllowed(false);
      setAnalysisBusy(false);
      setAnalysisMessage(undefined);
      setAnalysisError(undefined);
      setError(undefined);
      return;
    }
    let cancelled = false;
    void refresh();
    const timer = window.setInterval(() => {
      if (!cancelled) void refresh();
    }, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [refresh, hasWorkspace, taskmasterReady]);

  async function analyzeNow() {
    if (!hasWorkspace || !taskmasterReady || analysisBusy) return;

    setAnalysisBusy(true);
    setAnalysisError(undefined);
    setAnalysisMessage(undefined);
    const generation = workspaceGeneration.current;
    try {
      const result = await window.orkworks.requestTaskmasterAnalysis();
      if (!hasWorkspace || !taskmasterReady || generation !== workspaceGeneration.current) return;
      if (result.status === "active_recommendation" && result.recommendation) {
        setBlockedRecommendation(result.recommendation);
        setBlockedRecommendationId(result.recommendation.id);
        setBlockedRecommendationRecoveryAllowed(result.recoveryAllowed);
        setAnalysisMessage(activeRecommendationMessage(result.recommendation));
      } else {
        setBlockedRecommendation(undefined);
        setBlockedRecommendationId(undefined);
        setBlockedRecommendationRecoveryAllowed(false);
        setAnalysisMessage(result.message);
      }
      if (result.status === "scheduled" || result.status === "active_recommendation") void refresh();
    } catch (cause) {
      if (generation !== workspaceGeneration.current) return;
      setAnalysisError(cause instanceof Error ? cause.message : "Couldn't start Brain analysis.");
    } finally {
      if (generation === workspaceGeneration.current) setAnalysisBusy(false);
    }
  }

  async function dismiss(id: string) {
    if (!hasWorkspace || !taskmasterReady) return;
    const generation = refreshGeneration.current;
    setDismissing(id);
    setDismissErrors((current) => ({ ...current, [id]: "" }));
    try {
      if (!hasWorkspace || !taskmasterReady || generation !== refreshGeneration.current) return;
      await dismissTaskmasterRecommendation(id);
      if (generation !== refreshGeneration.current) return;
      await refresh();
    } catch (cause) {
      setDismissErrors((current) => ({
        ...current,
        [id]: cause instanceof Error ? cause.message : "Couldn't dismiss recommendation.",
      }));
    } finally {
      setDismissing(undefined);
    }
  }

  const visibleRecommendations = hasWorkspace && taskmasterReady ? recommendations.filter(
    (item) => item.status === "proposed"
      || (item.status === "executing" && item.rollupMemberIds.length > 0)
      || item.id === focusedRecommendationId,
  ) : [];

  return (
    <section className="recommendations-panel">
      <div className="recommendations-panel-header">
        <div><h2>Recommendations</h2><p>Evidence-backed workflow improvements.</p></div>
        <div className="recommendations-panel-actions">
          <button type="button" disabled={!hasWorkspace || !taskmasterReady || analysisBusy} onClick={() => void analyzeNow()}>
            {analysisBusy ? "Requesting…" : "Analyze now"}
          </button>
          <button type="button" disabled={!hasWorkspace || !taskmasterReady} onClick={() => void refresh()}>Reload</button>
        </div>
      </div>
      {analysisMessage && <div className="recommendation-analysis-status" role="status">
        {blockedRecommendation && <strong>{blockedRecommendation.title}</strong>}
        <p>{analysisMessage}</p>
        {blockedRecommendation?.status === "executing" && blockedRecommendationRecoveryAllowed && <button
          type="button"
          disabled={dismissing === blockedRecommendation.id}
          onClick={() => void dismiss(blockedRecommendation.id)}
        >{dismissing === blockedRecommendation.id ? "Recovering…" : "Recover stuck recommendation"}</button>}
      </div>}
      {analysisError && <p className="recommendation-error" role="alert">{analysisError}</p>}
      {error && <p className="recommendation-error" role="alert">{error}</p>}
      <DiagnosticList diagnostics={diagnostics} />
      {visibleRecommendations.length === 0 && diagnostics.length === 0 && !error ? (
        <EmptyState message="No workflow recommendations yet." />
      ) : (
        visibleRecommendations.map((recommendation) => (
          <RecommendationCard
            key={recommendation.id}
            recommendation={recommendation}
            onDismiss={dismiss}
            onSelectSession={onSelectSession}
            onFixWithAi={onFixWithAi}
            canFixWithAi={canFixWithAi}
            dismissing={dismissing === recommendation.id}
            error={dismissErrors[recommendation.id] || undefined}
            focused={recommendation.id === focusedRecommendationId}
          />
        ))
      )}
    </section>
  );
}

function ResurfacedLineage({ recommendation }: { recommendation: WorkflowRecommendation }) {
  const supersedesRecommendationId = recommendation.workflowImprovement.supersedesRecommendationId;
  const [predecessor, setPredecessor] = useState<WorkflowRecommendation | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setPredecessor(null);
    setFailed(false);
    if (!supersedesRecommendationId) return;
    let cancelled = false;
    void (async () => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        if (cancelled) return;
        const detail = await getTaskmasterRecommendation(baseUrl, supersedesRecommendationId);
        if (cancelled) return;
        if (detail.id !== supersedesRecommendationId) throw new Error("Invalid lineage detail");
        setPredecessor(detail);
      } catch {
        if (!cancelled) setFailed(true);
      }
    })();
    return () => { cancelled = true; };
  }, [supersedesRecommendationId]);

  if (!supersedesRecommendationId) return null;
  if (!predecessor && !failed) return null;
  if (predecessor?.status === "dismissed") {
    return (
      <p className="recommendation-lineage" role="status">
        Resurfaced after you dismissed “{predecessor.title}” on {predecessor.updatedAt} — newer
        evidence qualified again. Replaces {supersedesRecommendationId.slice(0, 8)}.
      </p>
    );
  }
  if (predecessor) {
    // The predecessor can be terminal for reasons other than supersession
    // (accepted, completed, expired, failed), so the status is shown as-is
    // rather than hardcoded to "superseded".
    return (
      <p className="recommendation-lineage" role="status">
        Replaces “{predecessor.title}” ({predecessor.status}, {supersedesRecommendationId.slice(0, 8)}).
      </p>
    );
  }
  return (
    <p className="recommendation-lineage" role="status">
      Replaces an earlier recommendation ({supersedesRecommendationId.slice(0, 8)}).
    </p>
  );
}

export default RecommendationsPanel;
