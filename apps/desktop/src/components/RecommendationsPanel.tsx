import { useCallback, useEffect, useState } from "react";
import {
  dismissTaskmasterRecommendation,
  getTaskmasterRecommendations,
  type ObservationDiagnostic,
  type WorkflowRecommendation,
} from "../api.ts";
import { formatImpact, formatRecurrence, formatTargetSurface, sortedEvidence } from "../taskmaster.ts";
import EmptyState from "./EmptyState";

interface RecommendationsPanelProps {
  hasWorkspace: boolean;
  onSelectSession?: (id: string) => void;
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
  dismissing,
  error,
}: {
  recommendation: WorkflowRecommendation;
  onDismiss: (id: string) => void;
  onSelectSession?: (id: string) => void;
  dismissing: boolean;
  error?: string;
}) {
  const improvement = recommendation.workflowImprovement;
  return (
    <article className="recommendation-card">
      <header className="recommendation-card-header">
        <div>
          <h3>{recommendation.title}</h3>
          <span className="recommendation-target">{formatTargetSurface(improvement.targetSurface)}</span>
        </div>
        <span className={`recommendation-impact recommendation-impact--${recommendation.priority}`}>
          {formatImpact(recommendation.priority)} impact
        </span>
      </header>
      <p className="recommendation-proposal">{improvement.proposedImprovement}</p>
      <p className="recommendation-reason">{recommendation.reason.join(" ")}</p>
      <dl className="recommendation-facts">
        <div><dt>Confidence</dt><dd>{formatImpact(recommendation.confidence)}</dd></div>
        <div><dt>Recurrence</dt><dd>{formatRecurrence(recommendation)}</dd></div>
        <div><dt>Expected benefit</dt><dd>{improvement.expectedBenefit}</dd></div>
      </dl>
      <div className="recommendation-sessions">
        {improvement.affectedSessionIds.map((sessionId) => (
          <button key={sessionId} type="button" onClick={() => onSelectSession?.(sessionId)}>
            Session {sessionId.slice(0, 8)}
          </button>
        ))}
      </div>
      <details className="recommendation-evidence">
        <summary>Evidence ({recommendation.evidence.length})</summary>
        {sortedEvidence(recommendation.evidence).map((item) => (
          <div className="recommendation-evidence-row" key={item.observationId}>
            <strong>{item.description}</strong>
            <span>{item.source} · {item.observedAt}</span>
            <p>{item.evidence}</p>
            <button type="button" onClick={() => onSelectSession?.(item.sessionId)}>
              Open session {item.sessionId.slice(0, 8)}
            </button>
          </div>
        ))}
      </details>
      {error && <p className="recommendation-error" role="alert">{error}</p>}
      <button className="recommendation-dismiss" type="button" disabled={dismissing} onClick={() => onDismiss(recommendation.id)}>
        {dismissing ? "Dismissing…" : "Dismiss"}
      </button>
    </article>
  );
}

function RecommendationsPanel({ hasWorkspace, onSelectSession }: RecommendationsPanelProps) {
  const [recommendations, setRecommendations] = useState<WorkflowRecommendation[]>([]);
  const [diagnostics, setDiagnostics] = useState<ObservationDiagnostic[]>([]);
  const [error, setError] = useState<string>();
  const [dismissing, setDismissing] = useState<string>();
  const [dismissErrors, setDismissErrors] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const response = await getTaskmasterRecommendations(baseUrl);
      setRecommendations(response.recommendations.filter((item) => item.status === "proposed"));
      setDiagnostics(response.diagnostics);
      setError(undefined);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Couldn't load recommendations.");
    }
  }, []);

  useEffect(() => {
    // The panel mounts as part of the default layout, before the sidecar's
    // async /workspace bootstrap has necessarily completed. Fetching before
    // that finishes races a 409 (no workspace set yet) into the console and
    // the error banner on every startup, so wait for the workspace instead.
    if (!hasWorkspace) return;
    let cancelled = false;
    // App-level workspace state is currently monotonic (null -> set, never
    // cleared back to null), so the initial refresh() below can't outlive a
    // workspace becoming unset again. If that ever changes, this refresh()
    // call needs the same `cancelled` guard the interval callback has.
    void refresh();
    const timer = window.setInterval(() => {
      if (!cancelled) void refresh();
    }, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [refresh, hasWorkspace]);

  async function dismiss(id: string) {
    setDismissing(id);
    setDismissErrors((current) => ({ ...current, [id]: "" }));
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      await dismissTaskmasterRecommendation(baseUrl, id);
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

  return (
    <section className="recommendations-panel">
      <div className="recommendations-panel-header">
        <div><h2>Recommendations</h2><p>Evidence-backed workflow improvements.</p></div>
        <button type="button" disabled={!hasWorkspace} onClick={() => void refresh()}>Reload</button>
      </div>
      {error && <p className="recommendation-error" role="alert">{error}</p>}
      <DiagnosticList diagnostics={diagnostics} />
      {recommendations.length === 0 && diagnostics.length === 0 && !error ? (
        <EmptyState message="No workflow recommendations yet." />
      ) : (
        recommendations.map((recommendation) => (
          <RecommendationCard
            key={recommendation.id}
            recommendation={recommendation}
            onDismiss={dismiss}
            onSelectSession={onSelectSession}
            dismissing={dismissing === recommendation.id}
            error={dismissErrors[recommendation.id] || undefined}
          />
        ))
      )}
    </section>
  );
}

export default RecommendationsPanel;
