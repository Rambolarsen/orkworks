import {useEffect,useRef,useState} from 'react';
import {defaultBaseline,parseReadinessStatus,readinessFailureMessage,readinessSuggestionMessage,type ReadinessStatus} from '../readiness';
import ReadinessSummary from './ReadinessSummary';
interface Props{workspacePath:string;disabled:boolean;onBusyChange:(busy:boolean)=>void;onSuggestion?:(recommendationId:string|null)=>void}
const phaseLabels={collecting_evidence:'Collecting evidence',reading_evidence:'Reading evidence',scoring_readiness:'Scoring readiness'};
export default function ReadinessPanel({workspacePath,disabled,onBusyChange,onSuggestion}:Props){
 const [status,setStatus]=useState<ReadinessStatus|null>(null);
 const [error,setError]=useState<string>();const [message,setMessage]=useState<string>();const [requesting,setRequesting]=useState(false);
 const [selectedId,setSelectedId]=useState('');const [baselineId,setBaselineId]=useState<string|null>(null);
 const refreshSequence=useRef(0);
 const epoch=useRef(0);const workspace=useRef(workspacePath);workspace.current=workspacePath;
 const refresh=async(generation:number)=>{
  const sequence=++refreshSequence.current;
  try{const result=parseReadinessStatus(await window.orkworks.getReadinessStatus());if(generation!==epoch.current||sequence!==refreshSequence.current||workspace.current!==workspacePath)return;if(result.workspacePath!==workspacePath){setStatus(null);return;}setStatus(result);setError(undefined);}
  catch(cause){if(generation===epoch.current&&sequence===refreshSequence.current){setStatus(null);setError(cause instanceof Error?cause.message:'Readiness history is unavailable');}}
 };
 useEffect(()=>{const generation=++epoch.current;setStatus(null);setSelectedId('');setBaselineId(null);setError(undefined);setMessage(undefined);void refresh(generation);const timer=window.setInterval(()=>{void refresh(generation)},5000);return()=>{epoch.current++;window.clearInterval(timer);};},[workspacePath]);
 const suggestion=status?.suggestionRun;
 const suggestionActive=suggestion?.state==='queued'||suggestion?.state==='running';
 const running=suggestionActive||status?.run?.state==='queued'||status?.run?.state==='running';
 const surfaced=useRef(new Set<string>());
 useEffect(()=>{if(suggestion?.state==='succeeded'&&!surfaced.current.has(suggestion.id)){surfaced.current.add(suggestion.id);onSuggestion?.(suggestion.recommendationId);}},[suggestion,onSuggestion]);
 useEffect(()=>{onBusyChange(Boolean(running||requesting));return()=>onBusyChange(false);},[running,requesting,onBusyChange]);
 const start=async(combined=false)=>{const generation=epoch.current;setRequesting(true);setError(undefined);setMessage(undefined);try{const result=await (combined?window.orkworks.requestReadinessSuggestion():window.orkworks.requestReadinessAssessment());if(generation!==epoch.current)return;setMessage(result.outcome==='scheduled'?combined?'Assessment and suggestion requested.':'Readiness assessment requested.':result.outcome==='already_running'?'Another Taskmaster analysis is active.':'Readiness requires complete verified Brain guidance, repository access and an available Taskmaster model.');await refresh(generation);}catch(cause){if(generation===epoch.current)setError(cause instanceof Error?cause.message:'Readiness assessment could not start');}finally{if(generation===epoch.current)setRequesting(false);}};
 const snapshots=status?.snapshots??[];const selected=snapshots.find(s=>s.id===selectedId)??snapshots[0];
 const earlier=selected?snapshots.slice(snapshots.findIndex(s=>s.id===selected.id)+1):[];
 const baseline=baselineId===null?defaultBaseline(selected?[selected,...earlier]:[]):earlier.find(s=>s.id===baselineId);
 return <section className="readiness-panel" aria-label="Repository agent readiness">
  <div className="readiness-heading"><div><h3>Agent readiness</h3><p>Assess, improve, and see your repository evolve.</p></div><button type="button" disabled={disabled||requesting||running} onClick={()=>void start(true)}>{running?'Assessing…':requesting?'Requesting…':'Assess and suggest'}</button><button type="button" disabled={disabled||requesting||running} onClick={()=>void start(false)}>{running?'Assessing readiness…':requesting?'Requesting…':'Assess readiness'}</button></div>
  <p>Assess and suggest uses up to four model calls with your Taskmaster model to score readiness and seek one grounded improvement. Assess readiness scores only.</p>
  {suggestion&&<p role="status">{readinessSuggestionMessage(suggestion)}</p>}
  {running&&!suggestionActive&&<p role="status">{status?.run?.phase?phaseLabels[status.run.phase]:'Collecting evidence'}…</p>}
  {message&&<p role="status">{message}</p>}{error&&<p className="recommendation-error" role="alert">{error}</p>}
  {(status?.run?.state==='failed'||status?.run?.state==='interrupted')&&<p>The latest attempt {status.run.state==='failed'?'failed':'was interrupted'}. Previous assessments are retained. {readinessFailureMessage(status.run.errorCode)}</p>}
  {status?.reasonCode&&<p>Readiness history is unavailable ({status.reasonCode.replaceAll('_',' ')}).</p>}
  {!selected&&!running&&!status?.reasonCode&&<p>No readiness assessment yet. Assess this workspace to establish a baseline.</p>}
  {selected&&<>
   <div className="readiness-history-controls"><label>Assessment <select value={selected.id} onChange={e=>{setSelectedId(e.target.value);setBaselineId(null);}}>{snapshots.map(s=><option key={s.id} value={s.id}>{new Date(s.completedAt).toLocaleString()} · {s.data?.overallScore!=null?`${s.data.overallScore}/100`:s.redacted?'Redacted':'Partial'}</option>)}</select></label>
   {earlier.length>0&&<label>Compare with <select value={baseline?.id??''} onChange={e=>setBaselineId(e.target.value)}><option value="">No baseline</option>{earlier.map(s=><option key={s.id} value={s.id}>{new Date(s.completedAt).toLocaleString()} · {s.data?.overallScore!=null?`${s.data.overallScore}/100`:s.redacted?'Redacted':'Partial'}</option>)}</select></label>}</div>
   <ReadinessSummary snapshot={selected} baseline={baseline} freshness={selected.id===snapshots[0]?.id?(status?.freshness??'unknown'):'unknown'}/>
   <p className="readiness-retention">Up to {status?.retentionLimit??100} assessments are retained locally for this workspace.</p>
  </>}
 </section>;
}
