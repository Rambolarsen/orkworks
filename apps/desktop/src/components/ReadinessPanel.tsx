import { useEffect, useRef, useState, type ReactNode } from 'react';
import { conceptLabel, defaultBaseline, parseReadinessStatus, readinessFailureMessage, readinessFailureRecovery, readinessSuggestionMessage, type ReadinessSnapshot, type ReadinessStatus } from '../readiness';
import ReadinessSummary from './ReadinessSummary';
import { canUseImprovementPrimary, improvementStatusForWorkspace, parseImprovementStatus, type ImprovementReport, type ImprovementStatus } from '../improvement';

interface Props {
  workspacePath: string;
  enabled?: boolean;
  disabled: boolean;
  analysisActive?: boolean;
  onBusyChange: (busy: boolean) => void;
  requestedSnapshot?: { snapshotId: string };
  onHistory?: (snapshots: ReadinessSnapshot[]) => void;
  onSuggestion?: (recommendationId: string | null) => void;
  renderActions?: (actions: { findImprovement: ReactNode; checkReadiness: ReactNode }) => ReactNode;
}
const phaseLabels = {
  collecting_evidence: 'Collecting evidence',
  reading_evidence: 'Reading evidence',
  scoring_readiness: 'Scoring readiness',
};

function snapshotOptions(snapshots: ReadinessSnapshot[]) {
  return snapshots.map(snapshot => <option key={snapshot.id} value={snapshot.id}>
    {new Date(snapshot.completedAt).toLocaleString()} · {snapshot.data?.overallScore != null ? `${snapshot.data.overallScore}/100` : snapshot.redacted ? 'Redacted' : 'Partial'}
  </option>);
}

function ImprovementAssessment({ report }: { report: ImprovementReport }) {
  return <details><summary>Assessment, coverage and limits</summary>
    <p>{report.summary}</p>
    <p>{report.coverage}/14 areas assessed · {report.overallScore === null ? 'No complete score' : `${report.overallScore}/100`}</p>
    {report.concepts.map(row => <details className="readiness-area" key={row.conceptId}>
      <summary>{conceptLabel(row.conceptId)} · {row.maturity === null ? 'Unassessed' : `${row.maturity} / 4`}</summary>
      <p>{row.unassessedReason ?? row.rationale}</p>
      {row.confidence && <p>{row.confidence} confidence</p>}
      {row.limits.map((limit, index) => <p key={index}>{limit}</p>)}
      <p>{row.nextTransition}</p><p>Rubric: {row.rubricPageId}</p>
      {report.repositoryEvidence.filter(fact => row.repositoryFactHashes.includes(fact.sha256)).map(fact => <div key={`${fact.path}:${fact.offset}`}><p><code>{fact.path}</code></p><pre className="recommendation-evidence-excerpt">{fact.text}</pre></div>)}
    </details>)}
    {report.limitations.map((limit, index) => <p key={index}>{limit}</p>)}
  </details>;
}

export default function ReadinessPanel({ workspacePath, enabled = true, disabled, analysisActive = false, onBusyChange, onSuggestion, onHistory, requestedSnapshot, renderActions }: Props) {
  const [status, setStatus] = useState<ReadinessStatus | null>(null);
  const [error, setError] = useState<string>();
  const [message, setMessage] = useState<string>();
  const [blocked, setBlocked] = useState(false);
  const [requesting, setRequesting] = useState(false);
  const [selectedId, setSelectedId] = useState('');
  const [baselineId, setBaselineId] = useState<string | null>(null);
  const [reportOpen, setReportOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [improvement, setImprovement] = useState<ImprovementStatus | null>(null);
  const [improvementError, setImprovementError] = useState<string>();
  const refreshSequence = useRef(0);
  const improvementRefreshSequence = useRef(0);
  const epoch = useRef(0);
  const workspace = useRef(workspacePath);
  workspace.current = workspacePath;

  const refresh = async (generation: number) => {
    const sequence = ++refreshSequence.current;
    try {
      const result = parseReadinessStatus(await window.orkworks.getReadinessStatus());
      if (generation !== epoch.current || sequence !== refreshSequence.current || workspace.current !== workspacePath) return;
      if (result.workspacePath !== workspacePath) {
        setStatus(null);
        return;
      }
      setStatus(result);
      if (!analysisActive && ![result.run, result.suggestionRun].some(run => run?.state === 'queued' || run?.state === 'running')) setBlocked(false);
      onHistory?.(result.snapshots);
      setError(undefined);
    } catch (cause) {
      if (generation === epoch.current && sequence === refreshSequence.current) {
        setStatus(null);
        setError(cause instanceof Error ? cause.message : 'Readiness history is unavailable');
      }
    }
  };
  const refreshImprovement = async (generation: number) => {
    const sequence = ++improvementRefreshSequence.current;
    try {
      const result = parseImprovementStatus(await window.orkworks.getImprovementStatus());
      if (generation !== epoch.current || sequence !== improvementRefreshSequence.current || workspace.current !== workspacePath) return;
      const current=improvementStatusForWorkspace(result,workspacePath);
      if(!current) { setImprovement(null); return; }
      setImprovement(current);
      setImprovementError(undefined);
    } catch (cause) {
      if (generation === epoch.current && sequence === improvementRefreshSequence.current && workspace.current === workspacePath) {
        setImprovement(null);
        setImprovementError(cause instanceof Error ? cause.message : 'Improvement status is unavailable');
      }
    }
  };
  useEffect(() => {
    const generation = ++epoch.current;
    setStatus(null);
    setImprovement(null);
    setImprovementError(undefined);
    setSelectedId('');
    setBaselineId(null);
    setError(undefined);
    setMessage(undefined);
    setBlocked(false);
    setRequesting(false);
    setReportOpen(false);
    setHistoryOpen(false);
    if (!enabled) return;
    void refresh(generation);
    void refreshImprovement(generation);
    const timer = window.setInterval(() => { void refresh(generation); void refreshImprovement(generation); }, 5000);
    return () => {
      epoch.current++;
      window.clearInterval(timer);
    };
  }, [workspacePath, enabled]);

  const startImprovement = async () => {
    if (!enabled || disabled || analysisActive || requesting || running || improvementActive) return;
    const generation=epoch.current; setRequesting(true); setError(undefined); setMessage(undefined);
    try {
      const result=await window.orkworks.requestImprovement();
      if(generation!==epoch.current)return;
      setMessage(result.outcome==='scheduled'?'Improvement investigation requested. It uses up to six model calls.':result.outcome==='already_running'?'An improvement investigation is already running.':`Improvement investigation is unavailable${result.reasonCode?` (${result.reasonCode.replaceAll('_',' ')})`:''}.`);
      await refreshImprovement(generation);
    } catch(cause) { if(generation===epoch.current)setError(cause instanceof Error?cause.message:'Improvement investigation could not start'); }
    finally { if(generation===epoch.current)setRequesting(false); }
  };
  const cancelImprovement = async () => {
    const run=improvement && improvementStatusForWorkspace(improvement, workspacePath)?.run; if(!run || (run.state!=='queued'&&run.state!=='running'))return;
    const generation=epoch.current;
    try { await window.orkworks.cancelImprovement(run.id); if(generation===epoch.current) await refreshImprovement(generation); }
    catch(cause) { if(generation===epoch.current)setError(cause instanceof Error?cause.message:'Improvement cancellation failed'); }
  };

  const openedSource = useRef<Props['requestedSnapshot']>(undefined);
  useEffect(() => {
    if (requestedSnapshot && openedSource.current !== requestedSnapshot && status?.snapshots.some(snapshot => snapshot.id === requestedSnapshot.snapshotId && !snapshot.redacted)) {
      openedSource.current = requestedSnapshot;
      setSelectedId(requestedSnapshot.snapshotId);
      setBaselineId(null);
      setReportOpen(true);
      setHistoryOpen(true);
      document.getElementById('readiness-history')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, [requestedSnapshot, status?.snapshots]);

  const suggestion = status?.suggestionRun;
  const suggestionActive = suggestion?.state === 'queued' || suggestion?.state === 'running';
  const running = suggestionActive || status?.run?.state === 'queued' || status?.run?.state === 'running';
  const currentImprovement = improvement ? improvementStatusForWorkspace(improvement, workspacePath) : null;
  const improvementRun=currentImprovement?.run;
  const improvementCancellable=improvementRun?.state==='queued'||improvementRun?.state==='running';
  const improvementActive=improvementCancellable||improvementRun?.state==='pending_recommendation';
  // Request acknowledgements yield to authoritative live status as it advances.
  useEffect(() => {
    setMessage(undefined);
    setBlocked(false);
  }, [status?.run?.id, status?.run?.state, status?.run?.phase, suggestion?.id, suggestion?.state, suggestion?.phase, analysisActive]);
  const surfaced = useRef(new Set<string>());
  useEffect(() => {
    if (suggestion?.state === 'succeeded' && !surfaced.current.has(suggestion.id)) {
      surfaced.current.add(suggestion.id);
      onSuggestion?.(suggestion.recommendationId);
    }
  }, [suggestion, onSuggestion]);
  useEffect(() => {
    if (improvementRun?.state === 'succeeded' && !surfaced.current.has(`improvement:${improvementRun.id}`)) {
      surfaced.current.add(`improvement:${improvementRun.id}`);
      onSuggestion?.(improvementRun.report?.recommendationId ?? null);
    }
  }, [improvementRun, onSuggestion]);
  useEffect(() => {
    onBusyChange(Boolean(running || requesting || improvementActive));
    return () => onBusyChange(false);
  }, [running, requesting, improvementActive, onBusyChange]);

  const start = async (combined = false) => {
    if (!enabled || disabled || analysisActive || requesting || running || improvementActive) return;
    const generation = epoch.current;
    setRequesting(true);
    setBlocked(false);
    setError(undefined);
    setMessage(undefined);
    try {
      const result = await (combined ? window.orkworks.requestReadinessSuggestion() : window.orkworks.requestReadinessAssessment());
      if (generation !== epoch.current) return;
      if (result.outcome === 'scheduled') {
        setMessage(combined ? 'Assessment and suggestion requested.' : 'Readiness assessment requested.');
      } else if (result.outcome === 'already_running') {
        setBlocked(true);
      } else {
        setMessage('Readiness requires complete verified Brain guidance, repository access and an available Taskmaster model.');
      }
      await refresh(generation);
    } catch (cause) {
      if (generation === epoch.current) setError(cause instanceof Error ? cause.message : 'Readiness assessment could not start');
    } finally {
      if (generation === epoch.current) setRequesting(false);
    }
  };

  const snapshots = status?.snapshots ?? [];
  const selected = snapshots.find(snapshot => snapshot.id === selectedId) ?? snapshots[0];
  const earlier = selected ? snapshots.slice(snapshots.findIndex(snapshot => snapshot.id === selected.id) + 1) : [];
  const baseline = baselineId === null
    ? defaultBaseline(selected ? [selected, ...earlier] : [])
    : earlier.find(snapshot => snapshot.id === baselineId);
  const actionsDisabled = !enabled || disabled || analysisActive || requesting || running || improvementActive;
  const actions = {
    findImprovement: canUseImprovementPrimary(currentImprovement)
      ? <button className="recommendation-primary" type="button" disabled={actionsDisabled || improvementActive} title="Investigate a preferred next improvement using up to six model calls." onClick={() => void startImprovement()}>{improvementActive ? 'Investigating…' : requesting ? 'Requesting…' : 'Find next improvement'}</button>
      : <button className="recommendation-primary" type="button" disabled={actionsDisabled} title="Assess readiness and suggest one concrete improvement. Uses up to four model calls." onClick={() => void start(true)}>{running ? 'Assessing…' : requesting ? 'Requesting…' : 'Find next improvement'}</button>,
    checkReadiness: <button type="button" disabled={actionsDisabled} onClick={() => void start(false)}>Check agent readiness</button>,
  };
  const latest = snapshots[0];
  const latestData = latest && !latest.redacted ? latest.data : null;
  const preferred=improvementRun?.report?.opportunities.find(item=>item.id===improvementRun.report?.preferredOpportunityId);

  const failedRun = status?.run?.state === 'failed' || status?.run?.state === 'interrupted' ? status.run : null;
  const failedSuggestion = suggestion?.state === 'failed' || suggestion?.state === 'interrupted' ? suggestion : null;
  const separateSuggestionFailure = failedSuggestion && (!failedRun || failedSuggestion.readinessRunId !== failedRun.id);
  const freshnessLabel = latest?.redacted ? 'Redacted' : status?.freshness === 'changed' ? 'Needs reassessment' : status?.freshness === 'current' ? 'Inputs unchanged' : 'Currentness unknown';

  return <>
    {renderActions ? renderActions(actions) : <div className="readiness-actions">{actions.findImprovement}{actions.checkReadiness}</div>}
    {currentImprovement && (currentImprovement.available || Boolean(currentImprovement.run?.report)) && <section className="readiness-panel improvement-panel" aria-label="Preferred next improvement">
      <div className="readiness-heading"><div className="readiness-overview-copy"><h3>Preferred next improvement</h3><p>One evidence based investigation, up to six model calls.</p></div>
        {improvementCancellable && <button type="button" onClick={() => void cancelImprovement()}>Cancel investigation</button>}
      </div>
      {improvementActive && <p role="status">{improvementRun?.state==='pending_recommendation'?'Saving the recommendation…':improvementRun?.phase==='collecting_evidence'?'Collecting evidence…':improvementRun?.phase==='reading_evidence'?'Reading evidence…':'Assessing opportunities…'}</p>}
      {improvementError && <p className="recommendation-error" role="alert">{improvementError}</p>}
      {improvementRun?.state==='failed' || improvementRun?.state==='interrupted' ? <p role="status">Investigation {improvementRun.state==='failed'?'failed':'was interrupted'}. {improvementRun.errorCode?.replaceAll('_',' ') ?? 'No new result was saved.'}</p> : null}
      {preferred ? <article><h4>{preferred.title}</h4><p>{preferred.summary}</p><p>{preferred.mechanism} {preferred.benefit}</p><p>Evidence: {preferred.evidenceStrength.replaceAll('_',' ')} · Effort: {preferred.effort} · Risk: {preferred.risk} · Reversibility: {preferred.reversibility}</p>
        <p>Validate: {preferred.validation}</p><p>Rollback: {preferred.rollback}</p>
        <details><summary>Evidence for this improvement</summary>
          {improvementRun!.report!.repositoryEvidence.filter(fact=>preferred.repositoryFactHashes.includes(fact.sha256)).map(fact=><div key={`${fact.path}:${fact.offset}:${fact.sha256}`}>
            <p><code>{fact.path}</code> · {fact.complete?'Complete file':'Partial file range'} · bytes {fact.offset}–{fact.offset+new TextEncoder().encode(fact.text).byteLength}</p>
            <pre className="recommendation-evidence-excerpt">{fact.text}</pre><small>{fact.sha256}</small>
          </div>)}
          {improvementRun!.report!.knowledgeEvidence.filter(page=>preferred.knowledgePageIds.includes(page.pageId)).map(page=><div key={page.pageId}>
            <p>{page.title} · <code>{page.pageId}</code> · {page.status} · {page.bundleVersion}</p>
            <blockquote>{page.excerpt}</blockquote><small>{page.sha256}</small>
          </div>)}
        </details>
        {improvementRun!.report!.opportunities.filter(item=>item.id!==preferred.id).slice(0,4).map(item=><details key={item.id}><summary>Alternative: {item.title}</summary><p>{item.summary}</p><p>{item.mechanism} {item.benefit}</p></details>)}
        <ImprovementAssessment report={improvementRun!.report!} />
      </article> : improvementRun?.report ? <><p>{improvementRun.report.noProposalReason ?? 'No supported next improvement was identified.'}</p><ImprovementAssessment report={improvementRun.report} /></> : <p>{currentImprovement.reasonCode?.replaceAll('_',' ') ?? 'No current improvement investigation is available.'}</p>}
    </section>}
    {enabled && <section id="readiness-history" className="readiness-panel" aria-label="Repository agent readiness">
    <div className="readiness-heading">
      <div className={`readiness-overview-score${latestData?.overallScore == null || status?.freshness !== 'current' ? ' readiness-overview-score--empty' : ''}`} aria-label="Saved overall agent readiness">
        {latestData?.overallScore != null ? <><strong>{latestData.overallScore}</strong><span>/ 100</span></> : <span>{latest?.redacted ? 'Redacted' : latestData ? 'Partial' : '—'}</span>}
      </div>
      <div className="readiness-overview-copy"><h3>Agent readiness</h3>
        {latest ? <>
          <p className="readiness-saved">Saved <time dateTime={latest.completedAt}>{new Date(latest.completedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}</time> <span className="readiness-freshness" title={latest.redacted ? 'Assessment evidence and scores were redacted after access changed.' : status?.freshness === 'changed' ? 'Repository changed since assessment.' : status?.freshness === 'current' ? 'Assessment inputs are unchanged; this is still a saved estimate.' : 'Currentness could not be verified.'}>{freshnessLabel}</span></p>
          {latestData && <p className="readiness-estimate">{latestData.coverage}/14 areas assessed · Evidence-limited estimate</p>}
        </> : <p>Check how well this repository supports coding agents.</p>}
      </div>
      {selected && <div className="readiness-overview-actions">
        <button type="button" aria-expanded={reportOpen} aria-controls="readiness-report" onClick={() => setReportOpen(!reportOpen)}>View report</button>
        <button type="button" aria-expanded={historyOpen && reportOpen} aria-controls="readiness-report" onClick={() => { setReportOpen(true); setHistoryOpen(!(historyOpen && reportOpen)); }}>History</button>
      </div>}
    </div>
    {running ? <p className="readiness-activity" role="status">{suggestionActive ? readinessSuggestionMessage(suggestion!) : `${status?.run?.phase ? phaseLabels[status.run.phase] : 'Collecting evidence'}…`}</p>
      : (analysisActive || blocked) && <p className="readiness-activity" role="status">Another Taskmaster analysis is active. Assessment actions will be available when it finishes.</p>}
    {suggestion && !suggestionActive && !failedSuggestion && <p role="status">{readinessSuggestionMessage(suggestion)}</p>}
    {message && !running && !analysisActive && <p role="status">{message}</p>}
    {error && <p className="recommendation-error" role="alert">{error}</p>}
    {failedRun && <div className="readiness-failure">
      <p className="readiness-failure-heading">Previous assessment {failedRun.state === 'failed' ? 'failed' : 'was interrupted'} · <time dateTime={failedRun.completedAt ?? failedRun.startedAt}>{new Date(failedRun.completedAt ?? failedRun.startedAt).toLocaleString()}</time></p>
      <p>{latestData ? 'Your saved assessment is retained.' : 'No new assessment was saved.'} {readinessFailureRecovery(failedRun.errorCode)}</p>
      <details><summary>Failure details</summary><p>{readinessFailureMessage(failedRun.errorCode)}</p></details>
    </div>}
    {separateSuggestionFailure && <div className="readiness-failure">
      <p className="readiness-failure-heading">Previous improvement attempt {failedSuggestion.state === 'failed' ? 'failed' : 'was interrupted'}</p>
      <p>{latestData ? 'Your saved score is retained.' : 'No new improvement is available.'} {failedSuggestion.reasonCode === 'suggestion_failed' ? 'The suggestion stage did not finish. Try Find next improvement again when analysis is idle.' : 'Try again when the workspace and settings are stable and analysis is idle.'}</p>
      <details><summary>Failure details</summary><p>{readinessSuggestionMessage(failedSuggestion)}</p></details>
    </div>}
    {status?.reasonCode && <p>Readiness history is unavailable ({status.reasonCode.replaceAll('_', ' ')}).</p>}
    {!selected && !running && !status?.reasonCode && <p className="readiness-empty">No assessment yet. Find next improvement assesses readiness and seeks one grounded suggestion; Check agent readiness scores only.</p>}
    {selected && <div id="readiness-report" hidden={!reportOpen}>
      <div className="readiness-history-controls" hidden={!historyOpen}>
        <label>Assessment <select value={selected.id} onChange={event => {
          setSelectedId(event.target.value);
          setBaselineId(null);
        }}>{snapshotOptions(snapshots)}</select></label>
        {earlier.length > 0 && <label>Compare with <select value={baseline?.id ?? ''} onChange={event => setBaselineId(event.target.value)}>
          <option value="">No baseline</option>{snapshotOptions(earlier)}
        </select></label>}
      </div>
      <ReadinessSummary snapshot={selected} baseline={baseline} freshness={selected.id === snapshots[0]?.id ? (status?.freshness ?? 'unknown') : 'unknown'} />
      <p className="readiness-retention">Up to {status?.retentionLimit ?? 100} assessments are retained locally for this workspace.</p>
    </div>}
  </section>}
  </>;
}
