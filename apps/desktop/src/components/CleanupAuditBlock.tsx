import type { WorkflowRecommendation } from "../api.ts";
import { formatAuditCriterion, formatAuditSummary } from "../taskmaster.ts";

export default function CleanupAuditBlock({
  recommendation,
}: {
  recommendation: WorkflowRecommendation;
}) {
  const audit = recommendation.audit;
  if (!audit) return null;
  return (
    <section className="recommendation-audit" aria-label="Audit results">
      <p className="recommendation-audit-summary">{formatAuditSummary(recommendation)}</p>
      {audit.entries.length > 0 && (
        <details className="recommendation-audit-entries">
          <summary>
            {audit.entries.length} flagged recommendation{audit.entries.length === 1 ? "" : "s"}
          </summary>
          <ul>
            {audit.entries.map((entry) => (
              <li key={entry.id}>
                <span className="recommendation-audit-entry-title">{entry.title}</span>
                {entry.criteria.map((criterion) => (
                  <span
                    key={criterion}
                    className={`recommendation-audit-badge recommendation-audit-badge--${criterion}`}
                  >
                    {formatAuditCriterion(criterion)}
                  </span>
                ))}
              </li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}
