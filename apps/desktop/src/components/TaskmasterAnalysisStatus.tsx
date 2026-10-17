import { useEffect, useState } from "react";
import { formatAnalysisDuration } from "../taskmasterAnalysisFeedback.ts";
import { formatTaskmasterManualRequest, formatTaskmasterProposalCounts, type TaskmasterRunStatus } from "../taskmasterSettings.ts";

export default function TaskmasterAnalysisStatus({ status, requesting, pending, error }: {
  status: TaskmasterRunStatus | null;
  requesting: boolean;
  pending: boolean;
  error?: string;
}) {
  const [now, setNow] = useState(Date.now);
  const active = status?.activeAttempt;
  const busy = requesting || (!error && (pending || Boolean(active)));
  useEffect(() => {
    if (!busy) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [busy, active?.id]);

  const outcome = status?.latestOutcome;
  const manual = status?.latestManualRequest;
  const showManual = Boolean(manual && (!outcome
    || Date.parse(manual.completedAt) >= Date.parse(outcome.completedAt)));
  let title = "Brain analysis";
  let detail = status ? "No analysis has run for this workspace." : "Loading analysis status…";
  let timing: string | null = null;
  let tone = "idle";
  if (error && !requesting) {
    title = "Analysis status unavailable";
    detail = error;
    tone = "failed";
  } else if (requesting || pending) {
    title = requesting ? "Requesting Brain analysis…" : "Brain analysis requested";
    detail = "Waiting for analysis status. Results refresh automatically.";
    tone = "running";

  } else if (active) {
    title = active.state === "queued" ? "Brain analysis queued" : "Analyzing workspace…";
    detail = active.state === "queued" ? "Waiting to start. Results refresh automatically." : "Analysis is running. Recommendations will appear here when it finishes.";
    const elapsed = formatAnalysisDuration(active.startedAt ?? active.queuedAt, now);
    timing = elapsed ? `${active.state === "queued" ? "Queued for" : "Elapsed"} ${elapsed}` : null;
    tone = "running";
  } else if (manual && showManual) {
    title = manual.disposition === "cached" ? "Previous analysis reused" : "Analysis did not start";
    detail = formatTaskmasterManualRequest(manual);
    tone = manual.disposition === "cached" ? "succeeded" : "idle";
  } else if (outcome) {
    title = outcome.state === "succeeded" ? "Brain analysis complete" : outcome.state === "failed" ? "Brain analysis failed" : "Brain analysis interrupted";
    detail = outcome.state === "succeeded"
      ? formatTaskmasterProposalCounts(outcome) || "Analysis finished successfully."
      : outcome.errorSummary || (outcome.state === "failed" ? "Analysis could not finish. Try Analyze now again." : "Analysis stopped before finishing. You can run it again.");
    const elapsed = formatAnalysisDuration(outcome.startedAt, Date.parse(outcome.completedAt));
    const completed = new Date(outcome.completedAt);
    timing = Number.isFinite(completed.getTime()) ? `Finished ${completed.toLocaleString()}${elapsed ? ` · Took ${elapsed}` : ""}` : null;
    tone = outcome.state;
  }
  const source = requesting || pending || error || (!active && showManual) ? null : active ?? outcome;
  return <section className={`taskmaster-analysis-feedback taskmaster-analysis-feedback--${tone}`} aria-label="Brain analysis status" role="status" aria-live="polite">
    <div className="taskmaster-analysis-feedback-heading">
      <span className={`taskmaster-analysis-indicator${busy ? " taskmaster-analysis-indicator--busy" : ""}`} aria-hidden="true" />
      <strong>{title}</strong>
      {timing && <span className="taskmaster-analysis-timing">{timing}</span>}
    </div>
    <p>{detail}</p>
    {source && <p className="taskmaster-analysis-meta">{source.provider} / {source.model} · {source.trigger === "manual" ? "Manual analysis" : "Background analysis"}</p>}
  </section>;
}
