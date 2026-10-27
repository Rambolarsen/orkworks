import { compareReadiness, conceptLabel, readinessQualification, type ReadinessSnapshot, type ReadinessStatus } from '../readiness';
interface Props {snapshot:ReadinessSnapshot;baseline?:ReadinessSnapshot|null;freshness:ReadinessStatus['freshness']}
export default function ReadinessSummary({snapshot,baseline,freshness}:Props){
 const data=snapshot.data;
 if(snapshot.redacted||!data)return <p>Assessment evidence and scores were redacted after access changed.</p>;
 const comparison=compareReadiness(snapshot,baseline);
 const selection=data.inferenceIdentity.selection;
 return <div className="readiness-summary">
  <div className="readiness-score" aria-label="Overall agent readiness">
   {data.overallScore===null?<strong>Partial assessment</strong>:<><strong>{data.overallScore}</strong><span> / 100</span></>}
   {comparison.delta!==null&&<span className="readiness-delta">{comparison.delta>0?'+':''}{comparison.delta} since baseline</span>}
  </div>
  <p>{data.coverage}/14 areas assessed{data.rawTotal!==null?` · ${data.rawTotal}/56 maturity points`:''}</p>
  {freshness==='changed'&&<p className="readiness-notice">Changes since assessment — rerun to update.</p>}
  {freshness==='unknown'&&<p className="readiness-notice">Currentness could not be verified. This is a saved assessment.</p>}
  <p>Assessed at <time dateTime={snapshot.completedAt}>{new Date(snapshot.completedAt).toLocaleString()}</time> · {data.git.headOid?`Commit ${data.git.headOid.slice(0,10)}`:'No commit available'} · {data.git.dirty===true?'uncommitted changes':data.git.dirty===false?'clean worktree':'worktree state unknown'}</p>
  <p>{data.summary}</p>
  <p className="readiness-notice">{readinessQualification(data)}</p>
  <section aria-label="What to improve next">
   <h4>What to improve next</h4>
   <p>These are evidence and improvement needs from the saved assessment, not confirmed defects. Assess and suggest checks current evidence for one actionable fix.</p>
   <ol>{[...data.concepts].filter(row=>row.nextTransition||row.unassessedReason).sort((a,b)=>(a.maturity??-1)-(b.maturity??-1)).slice(0,3).map(row=><li key={row.conceptId}>
    <strong>{conceptLabel(row.conceptId)}</strong>{row.confidence&&<span> · {row.confidence} confidence</span>}
    {row.unassessedReason&&<p><strong>Not assessed:</strong> {row.unassessedReason}</p>}
    {row.nextTransition&&<p>{row.nextTransition}</p>}
    {row.limits.length>0&&<ul>{row.limits.map((limit,i)=><li key={i}>{limit}</li>)}</ul>}
   </li>)}</ol>
  </section>
  {baseline&&<p>Baseline: {baseline.data?.overallScore??'Partial/redacted'}{baseline.data?.overallScore!==null&&baseline.data?' / 100':''} · {new Date(baseline.completedAt).toLocaleString()}{comparison.reason?` · ${comparison.reason}`:''}</p>}
  {data.inputFingerprint && baseline?.data?.inputFingerprint === data.inputFingerprint && <p>Same assessed inputs — score differences may reflect model variability.</p>}
  <details><summary>Individual scores ({data.concepts.length})</summary>
   <div className="readiness-areas">{data.concepts.map(row=>{
    const prior=baseline?.data?.concepts.find(p=>p.conceptId===row.conceptId);
    const delta=comparison.delta!==null&&row.maturity!==null&&prior?.maturity!=null?row.maturity-prior.maturity:null;
    return <details key={row.conceptId} className="readiness-area"><summary><span>{conceptLabel(row.conceptId)}</span><strong>{row.maturity===null?'Unassessed':`${row.maturity} / 4`}</strong>{delta!==null&&delta!==0&&<span>{delta>0?'+':''}{delta}</span>}</summary>
     {row.confidence&&<p>{row.confidence} confidence{prior?.confidence&&prior.confidence!==row.confidence?` (previously ${prior.confidence})`:''}</p>}
     <p>{row.unassessedReason??row.rationale}</p>
     {row.limits.length>0&&<ul>{row.limits.map((limit,i)=><li key={i}>{limit}</li>)}</ul>}
     {row.nextTransition&&<p><strong>Next improvement:</strong> {row.nextTransition}</p>}
     <p>Rubric: {row.rubricPageId}</p>
     {data.repositoryEvidence.filter(fact=>row.repositoryFactHashes.includes(fact.factId)).map(fact=><div className="recommendation-evidence-row" key={fact.factId}><strong>{fact.path}</strong><p>{fact.excerpt}</p></div>)}
    </details>;
   })}</div>
  </details>
  <details><summary>Saved assessment evidence ({data.repositoryEvidence.length})</summary>{data.repositoryEvidence.map((fact,i)=><div key={`${fact.path}:${i}`} className="recommendation-evidence-row"><strong>{fact.path}</strong><span>Saved excerpt · {fact.observedAt}</span><p>{fact.excerpt}</p></div>)}</details>
  <details><summary>Assessment conditions</summary><p>{selection?.provider} / {selection?.model}{selection?.reasoningEffort?` · ${selection.reasoningEffort}`:''} · Brain {data.knowledgeIdentity.version??'saved guidance'}</p><p>Ratings describe available evidence; this assessment did not execute repository commands.</p></details>
 </div>;
}
