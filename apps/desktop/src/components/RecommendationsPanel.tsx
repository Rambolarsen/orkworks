import { useCallback, useEffect, useRef, useState } from "react";
import {
  dismissTaskmasterRecommendation,
  getTaskmasterRecommendation,
  getTaskmasterRecommendations,
  type ObservationDiagnostic,
  type WorkflowRecommendation,
} from "../api.ts";
import { formatImpact, formatRecurrence, formatTargetSurface } from "../taskmaster.ts";
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
  const refreshGeneration = useRef(0);

  const refresh = useCallback(async () => {
    if (!hasWorkspace || !taskmasterReady) return;
    const generation = ++refreshGeneration.current;
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      if (!hasWorkspace || !taskmasterReady || generation !== refreshGeneration.current) return;
      const response = await getTaskmasterRecommendations(baseUrl);
      let nextRecommendations = response.recommendations;
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
  }, [focusedRecommendationId, hasWorkspace, taskmasterReady]);

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
      setRecommendations([]);
      setDiagnostics([]);
      setError(undefined);
      return;
    }
    if (!taskmasterReady) {
      ++refreshGeneration.current;
      setRecommendations([]);
      setDiagnostics([]);
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
        <button type="button" disabled={!hasWorkspace || !taskmasterReady} onClick={() => void refresh()}>Reload</button>
      </div>
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

export default RecommendationsPanel;
