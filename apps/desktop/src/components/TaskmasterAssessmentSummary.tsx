import { formatTaskmasterAssessment, type TaskmasterAssessmentStatus } from "../taskmasterSettings";

interface Props {
  assessment: TaskmasterAssessmentStatus;
}

export default function TaskmasterAssessmentSummary({ assessment }: Props) {
  const report = assessment.report;
  return <section className="recommendation-analysis-status" aria-label="Workflow assessment" role="status">
    <p>{formatTaskmasterAssessment(assessment)}</p>
    {report && <>
      {report.outcome === "proposed" && report.recommendationId && <p>Assessment recommendation ID: {report.recommendationId}.</p>}
      <p>Outcome: {report.outcome === "proposed" ? "Proposal" : "No proposal"}</p>
      <p>Assessment {report.id} · Workspace {report.workspacePath} · Observed {report.observedAt} · {report.provider} / {report.model} · Brain bundle {report.bundleVersion}</p>
      {report.unverifiedChecks.length > 0 && <div>
        <strong>Not verified</strong>
        <ul>{report.unverifiedChecks.map((check, index) => <li key={`${index}:${check}`}>{check}</li>)}</ul>
      </div>}
      {(report.repositoryEvidence.length > 0 || report.knowledgeEvidence.length > 0) && <details>
        <summary>Assessment evidence ({report.repositoryEvidence.length + report.knowledgeEvidence.length})</summary>
        {report.repositoryEvidence.map((item) => <div className="recommendation-evidence-row" key={`${item.path}:${item.sha256}`}>
          <strong>{item.path}</strong><span>Repository snapshot · {item.observedAt}</span><p>{item.excerpt}</p>
        </div>)}
        {report.knowledgeEvidence.map((item) => <div className="recommendation-evidence-row" key={`${item.pageId}:${item.sha256}`}>
          <strong>{item.title}</strong><span>Knowledge {item.bundleVersion} · {item.status}</span><p>{item.excerpt}</p>
          <span>{item.pageId}</span>
        </div>)}
      </details>}
    </>}
  </section>;
}
