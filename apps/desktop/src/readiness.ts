// Renderer contract, independent of Electron and Rust types.
export const conceptIds = ['context-discovery','progressive-disclosure','instruction-scoping','architecture-awareness','domain-knowledge','environment-reproducibility','tool-accessibility','task-decomposition','verification','observability','change-safety','feedback-loops','durable-memory','human-oversight'] as const;
export interface ReadinessRow { conceptId:string; maturity:number|null; confidence:'low'|'medium'|'high'|null; repositoryFactHashes:string[]; rubricPageId:string; rationale:string; limits:string[]; nextTransition:string; unassessedReason:string|null }
export interface ReadinessData {
 workspacePath:string; inputFingerprint:string; comparisonFingerprint:string; git:{headOid:string|null;dirty:boolean|null};
 inferenceIdentity:{selection?:{provider:string;model:string;reasoningEffort?:string}}; knowledgeIdentity:{version?:string}; rubricDigest:string;
 summary:string; concepts:ReadinessRow[]; coverage:number; rawTotal:number|null; overallScore:number|null;
 repositoryEvidence:{factId:string;path:string;sha256:string;excerpt:string;observedAt:string}[];
 knowledgeEvidence:{pageId:string;title:string;status:string;bundleVersion:string;sha256:string;excerpt:string}[];
}
export interface ReadinessSnapshot {id:string;startedAt:string;completedAt:string;redacted:boolean;data:ReadinessData|null}
export interface ReadinessRun {id:string;state:'queued'|'running'|'succeeded'|'failed'|'interrupted';phase:'collecting_evidence'|'reading_evidence'|'scoring_readiness'|null;startedAt:string;completedAt:string|null;errorCode:string|null}
export interface ReadinessStatus {workspacePath:string|null;run:ReadinessRun|null;snapshots:ReadinessSnapshot[];freshness:'none'|'current'|'changed'|'unknown'|'redacted';retentionLimit:number;reasonCode:string|null}
export interface ReadinessAdmission {outcome:'scheduled'|'already_running'|'unavailable';runId:string|null;reasonCode:string|null}
const object=(x:unknown):Record<string,unknown>=>{if(!x||typeof x!=='object'||Array.isArray(x))throw Error('Invalid readiness response');return x as Record<string,unknown>};
const text=(x:unknown,max:number)=>typeof x==='string'&&x.length<=max;
const digest=(x:unknown)=>typeof x==='string'&&/^[0-9a-f]{64}$/.test(x);
export function parseReadinessStatus(value:unknown):ReadinessStatus {
 const v=object(value);
 if(!(v.workspacePath===null||text(v.workspacePath,4096))||!Array.isArray(v.snapshots)||v.snapshots.length>100||v.retentionLimit!==100||!['none','current','changed','unknown','redacted'].includes(v.freshness as string)||!(v.reasonCode===null||text(v.reasonCode,128)))throw Error('Invalid readiness status');
 const ids=new Set<string>();
 for(const item of v.snapshots){const s=object(item);if(!text(s.id,128)||ids.has(s.id as string)||!text(s.startedAt,40)||!text(s.completedAt,40)||typeof s.redacted!=='boolean')throw Error('Invalid readiness snapshot');ids.add(s.id as string);
  if(s.redacted){if(s.data!==null)throw Error('Redacted readiness contains source data');continue;}
  const d=object(s.data);if(!digest(d.comparisonFingerprint)||!digest(d.inputFingerprint)||!text(d.summary,1000)||!Array.isArray(d.concepts)||d.concepts.length!==14||!Array.isArray(d.repositoryEvidence)||d.repositoryEvidence.length>32||!Array.isArray(d.knowledgeEvidence)||d.knowledgeEvidence.length>14)throw Error('Invalid readiness details');
  let coverage=0,total=0;
  d.concepts.forEach((item,index)=>{const row=object(item);if(row.conceptId!==conceptIds[index]||!text(row.rationale,500)||!text(row.nextTransition,500)||!Array.isArray(row.limits)||row.limits.length>4||row.limits.some(x=>!text(x,200))||!Array.isArray(row.repositoryFactHashes)||row.repositoryFactHashes.length>8||row.repositoryFactHashes.some(x=>!digest(x))||!text(row.rubricPageId,256))throw Error('Invalid readiness row');
   if(row.maturity===null){if(row.confidence!==null||!text(row.unassessedReason,500)||!row.unassessedReason)throw Error('Invalid unassessed row');}
   else{if(!Number.isInteger(row.maturity)||(row.maturity as number)<0||(row.maturity as number)>4||!['low','medium','high'].includes(row.confidence as string)||row.unassessedReason!==null||row.repositoryFactHashes.length===0)throw Error('Invalid readiness rating');coverage++;total+=row.maturity as number;}
  });
  if(d.coverage!==coverage||d.rawTotal!==(coverage===14?total:null)||d.overallScore!==(coverage===14?Math.floor((100*total+28)/56):null))throw Error('Invalid readiness totals');
  for(const item of d.repositoryEvidence){const fact=object(item);if(!digest(fact.factId)||!text(fact.path,512)||!digest(fact.sha256)||!text(fact.excerpt,4000)||!text(fact.observedAt,40))throw Error('Invalid readiness evidence');}
  for(const item of d.knowledgeEvidence){const page=object(item);if(!text(page.pageId,256)||!digest(page.sha256)||!text(page.title,512)||!text(page.excerpt,4000))throw Error('Invalid readiness knowledge');}
  object(d.inferenceIdentity);object(d.knowledgeIdentity);
  const git=object(d.git);if(!(git.headOid===null||text(git.headOid,64))||!(git.dirty===null||typeof git.dirty==='boolean')||!digest(d.rubricDigest))throw Error('Invalid readiness provenance');
 }
 if(v.run!==null){const run=object(v.run);if(!text(run.id,128)||!['queued','running','succeeded','failed','interrupted'].includes(run.state as string)||!(run.phase===null||['collecting_evidence','reading_evidence','scoring_readiness'].includes(run.phase as string)))throw Error('Invalid readiness progress');}
 return v as unknown as ReadinessStatus;
}
export function compareReadiness(current:ReadinessSnapshot,baseline:ReadinessSnapshot|null|undefined):{delta:number|null;reason:string|null}{
 if(!baseline)return{delta:null,reason:'No earlier comparable assessment'};
 if(!current.data||!baseline.data)return{delta:null,reason:'Assessment evidence was redacted'};
 if(current.data.coverage!==14||baseline.data.coverage!==14)return{delta:null,reason:'Partial assessment — no overall comparison'};
 if(current.data.comparisonFingerprint!==baseline.data.comparisonFingerprint)return{delta:null,reason:'Assessment conditions changed'};
 return{delta:current.data.overallScore!-baseline.data.overallScore!,reason:null};
}
export function defaultBaseline(snapshots:ReadinessSnapshot[]):ReadinessSnapshot|undefined{
 const current=snapshots[0];if(!current?.data||current.data.coverage!==14)return undefined;
 return snapshots.slice(1).find(snapshot=>snapshot.data?.coverage===14&&snapshot.data.comparisonFingerprint===current.data!.comparisonFingerprint);
}
export const conceptLabel=(id:string)=>id.split('-').map(word=>word[0].toUpperCase()+word.slice(1)).join(' ');

// Only render fixed local descriptions: persisted codes are not trusted prose.
export function readinessFailureMessage(code:string|null|undefined):string {
 const stages:Record<string,string>={discovery:'Selecting evidence',evidence:'Reading evidence',scoring:'Scoring readiness',publication:'Saving the assessment'};
 const causes:Record<string,string>={
  provider_timeout:'the model request timed out. Retry or check Taskmaster model settings.',
  provider_authentication:'the model provider requires authentication. Check its login.',
  provider_model:'the selected model was unavailable. Check the model ID in Taskmaster settings.',
  provider_unsupported:'the provider does not support the assessment request. Check the coding tool version and Taskmaster settings.',
  provider_request:'the provider rejected the request. Check the model ID and reasoning effort in Taskmaster settings.',
  provider_unavailable:'the model provider could not complete the request. Check its availability.',
  invalid_response:'the model returned an invalid assessment response. No score was saved.',
  invalid_citations:'the model cited evidence outside the supplied assessment context. No score was saved.',
  invalid_prose:'the model response failed assessment text validation. No score was saved.',
  invalid_rating:'the model returned inconsistent ratings or coverage. No score was saved.',
  inputs_changed:'the workspace or assessment inputs changed. Assess again once they are stable.',
  evidence_unavailable:'the permitted repository evidence could not be read. Check access and exclusions.',
  evidence_limit:'the selected evidence exceeded the assessment allowance. No score was saved.',
  prompt_limit:'the assessment context exceeded its size limit. No score was saved.',
  metadata_unavailable:'assessment history could not be read or saved. Check local storage access.',
 };
 const match=/^(discovery|evidence|scoring|publication)_(.+)$/.exec(code??'');
 if(!match||!Object.hasOwn(causes,match[2]))return 'The assessment could not finish. Try Assess readiness again.';
 return `${stages[match[1]]}: ${causes[match[2]]}`;
}
