import { useEffect, useRef, useState, type ReactNode } from 'react';
import { defaultBaseline, parseReadinessStatus, readinessFailureMessage, readinessFailureRecovery, readinessSuggestionMessage, type ReadinessSnapshot, type ReadinessStatus } from '../readiness';
import ReadinessSummary from './ReadinessSummary';

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
  const refreshSequence = useRef(0);
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
  useEffect(() => {
    const generation = ++epoch.current;
    setStatus(null);
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
    const timer = window.setInterval(() => { void refresh(generation); }, 5000);
    return () => {
      epoch.current++;
      window.clearInterval(timer);
    };
  }, [workspacePath, enabled]);

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
    onBusyChange(Boolean(running || requesting));
    return () => onBusyChange(false);
  }, [running, requesting, onBusyChange]);

  const start = async (combined = false) => {
    if (!enabled || disabled || analysisActive || requesting || running) return;
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
  const actionsDisabled = !enabled || disabled || analysisActive || requesting || running;
  const actions = {
    findImprovement: <button className="recommendation-primary" type="button" disabled={actionsDisabled} title="Assess readiness and suggest one concrete improvement. Uses up to four model calls." onClick={() => void start(true)}>{running ? 'Assessing…' : requesting ? 'Requesting…' : 'Find next improvement'}</button>,
    checkReadiness: <button type="button" disabled={actionsDisabled} onClick={() => void start(false)}>Check agent readiness</button>,
  };
  const latest = snapshots[0];
  const latestData = latest && !latest.redacted ? latest.data : null;

  const failedRun = status?.run?.state === 'failed' || status?.run?.state === 'interrupted' ? status.run : null;
  const failedSuggestion = suggestion?.state === 'failed' || suggestion?.state === 'interrupted' ? suggestion : null;
  const separateSuggestionFailure = failedSuggestion && (!failedRun || failedSuggestion.readinessRunId !== failedRun.id);
  const freshnessLabel = latest?.redacted ? 'Redacted' : status?.freshness === 'changed' ? 'Needs reassessment' : status?.freshness === 'current' ? 'Inputs unchanged' : 'Currentness unknown';

  return <>
    {renderActions ? renderActions(actions) : <div className="readiness-actions">{actions.findImprovement}{actions.checkReadiness}</div>}
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
