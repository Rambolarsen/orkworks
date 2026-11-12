import { conceptIds, type ReadinessRow } from './readiness.ts';
// Renderer-owned copy of the public improvement contract. Keep independent of Electron/Rust.
export type ImprovementOpportunity = {
  id: string; targetSurface: string; title: string; summary: string; mechanism: string; benefit: string;
  evidenceStrength: 'observed_problem' | 'supported_hypothesis' | 'investigation_needed';
  effort: 'low' | 'medium' | 'high'; risk: 'low' | 'medium' | 'high'; reversibility: 'low' | 'medium' | 'high';
  repositoryFactHashes: string[]; knowledgePageIds: string[]; validation: string; rollback: string;
};
export type ImprovementReport = {
  summary: string; concepts: ReadinessRow[]; opportunities: ImprovementOpportunity[]; preferredOpportunityId: string | null;
  noProposalReason: string | null; limitations: string[]; coverage: number; overallScore: number | null;
  recommendationId: string | null; historySnapshotId: string | null; historyError: string | null;
  repositoryEvidence: { path: string; offset: number; fileSize: number; sha256: string; text: string; complete: boolean }[];
  knowledgeEvidence: { pageId: string; title: string; status: string; bundleVersion: string; sha256: string; excerpt: string }[];
};
export type ImprovementRun = { id: string; state: 'queued'|'running'|'pending_recommendation'|'succeeded'|'failed'|'interrupted'|'access_redacted'; phase: 'collecting_evidence'|'reading_evidence'|'assessing'|null; startedAt: string; completedAt: string|null; errorCode: string|null; report: ImprovementReport|null };
export type ImprovementStatus = { workspacePath: string|null; available: boolean; reasonCode: string|null; run: ImprovementRun|null };
export type ImprovementAdmission = { outcome: 'scheduled'|'already_running'|'unavailable'; runId: string|null; reasonCode: string|null };
export type ImprovementCancellation = { outcome: 'cancel_requested'|'not_running'|'unavailable'; runId: string|null; reasonCode: string|null };

const obj = (v: unknown): Record<string, unknown> => {
  if (!v || typeof v !== 'object' || Array.isArray(v)) throw Error('Invalid improvement response');
  return v as Record<string, unknown>;
};
const str = (v: unknown, n: number) => typeof v === 'string' && v.length <= n;
const nullable = (v: unknown, n: number) => v === null || str(v,n);
const hash = (v: unknown) => typeof v === 'string' && /^[a-f0-9]{64}$/i.test(v);
function parseOpportunity(value: unknown): ImprovementOpportunity {
  const o = obj(value);
  if (!str(o.id,128) || !str(o.targetSurface,64) || !str(o.title,240) || !str(o.summary,1000) || !str(o.mechanism,1000) || !str(o.benefit,1000)
    || !['observed_problem','supported_hypothesis','investigation_needed'].includes(o.evidenceStrength as string)
    || !['low','medium','high'].includes(o.effort as string) || !['low','medium','high'].includes(o.risk as string) || !['low','medium','high'].includes(o.reversibility as string)
    || !Array.isArray(o.repositoryFactHashes) || o.repositoryFactHashes.length>32 || o.repositoryFactHashes.some(x=>!hash(x))
    || !Array.isArray(o.knowledgePageIds) || o.knowledgePageIds.length>16 || o.knowledgePageIds.some(x=>!str(x,256))
    || !str(o.validation,1000) || !str(o.rollback,1000)) throw Error('Invalid improvement opportunity');
  return o as unknown as ImprovementOpportunity;
}
function parseReport(value: unknown): ImprovementReport {
  const r=obj(value);
  if (!str(r.summary,1000) || !Array.isArray(r.concepts) || r.concepts.length!==14 || !Array.isArray(r.opportunities) || r.opportunities.length>5
    || !nullable(r.preferredOpportunityId,128) || !nullable(r.noProposalReason,1000) || !Array.isArray(r.limitations) || r.limitations.length>32 || r.limitations.some(x=>!str(x,500))
    || !Number.isInteger(r.coverage) || (r.coverage as number)<0 || (r.coverage as number)>14
    || !(r.overallScore===null || Number.isInteger(r.overallScore) && (r.overallScore as number)>=0 && (r.overallScore as number)<=100)
    || !nullable(r.recommendationId,128) || !nullable(r.historySnapshotId,128) || !nullable(r.historyError,128)
    || !Array.isArray(r.repositoryEvidence) || r.repositoryEvidence.length>32 || !Array.isArray(r.knowledgeEvidence) || r.knowledgeEvidence.length>16) throw Error('Invalid improvement report');
  const opportunities=r.opportunities.map(parseOpportunity);
  const ids=opportunities.map(x=>x.id);
  if(new Set(ids).size!==ids.length || (r.preferredOpportunityId!==null && !ids.includes(r.preferredOpportunityId as string)) || (r.preferredOpportunityId===null ? !(typeof r.noProposalReason==='string' && r.noProposalReason.length>0) : r.noProposalReason!==null)) throw Error('Invalid improvement preference');
  const concepts=r.concepts.map((value,index)=>{
    const c=obj(value);
    const unassessed=c.maturity===null;
    if(c.conceptId!==conceptIds[index]||!(unassessed||Number.isInteger(c.maturity)&&(c.maturity as number)>=0&&(c.maturity as number)<=4)||!(unassessed?c.confidence===null:['low','medium','high'].includes(c.confidence as string))
      ||!Array.isArray(c.repositoryFactHashes)||c.repositoryFactHashes.length>32||c.repositoryFactHashes.some(x=>!hash(x))
      ||!str(c.rubricPageId,256)||!str(c.rationale,500)||!Array.isArray(c.limits)||c.limits.length>4||c.limits.some(x=>!str(x,200))
      ||!str(c.nextTransition,500)
      ||(unassessed?!(str(c.unassessedReason,500)&&Boolean(c.unassessedReason)):!(c.unassessedReason===null&&c.repositoryFactHashes.length>0))) throw Error('Invalid improvement concept');
    return c;
  });
  const rated=concepts.filter(c=>c.maturity!==null);
  const expectedScore=rated.length===14?Math.round(100*rated.reduce((sum,c)=>sum+(c.maturity as number),0)/56):null;
  if(r.coverage!==rated.length||r.overallScore!==expectedScore)throw Error('Invalid improvement score or coverage');
  for(const item of r.repositoryEvidence){ const f=obj(item); const bytes=typeof f.text==='string'?new TextEncoder().encode(f.text).byteLength:-1; if(!str(f.path,512)||!Number.isSafeInteger(f.offset)||!Number.isSafeInteger(f.fileSize)||!hash(f.sha256)||!str(f.text,8192)||bytes>8192||typeof f.complete!=='boolean'||(f.offset as number)<0||(f.fileSize as number)<(f.offset as number)+bytes||f.complete!==((f.offset===0&&bytes===f.fileSize))) throw Error('Invalid improvement evidence'); }
  const knowledgeEvidence=r.knowledgeEvidence.map(value=>{const p=obj(value);if(!str(p.pageId,256)||!str(p.title,512)||!str(p.status,128)||!str(p.bundleVersion,128)||!hash(p.sha256)||!str(p.excerpt,4000))throw Error('Invalid improvement knowledge');return p;});
  return {...r, concepts, opportunities, repositoryEvidence:r.repositoryEvidence, knowledgeEvidence} as unknown as ImprovementReport;
}
export function parseImprovementStatus(value: unknown): ImprovementStatus {
  const s=obj(value);
  if (!(s.workspacePath===null||str(s.workspacePath,4096)) || typeof s.available!=='boolean' || !nullable(s.reasonCode,128)) throw Error('Invalid improvement status');
  let run: ImprovementRun|null=null;
  if(s.run!==null){const x=obj(s.run); if(!str(x.id,128)||!['queued','running','pending_recommendation','succeeded','failed','interrupted','access_redacted'].includes(x.state as string)||!(x.phase===null||['collecting_evidence','reading_evidence','assessing'].includes(x.phase as string))||!str(x.startedAt,40)||!nullable(x.completedAt,40)||!nullable(x.errorCode,128))throw Error('Invalid improvement run'); run={...x,report:x.report===null?null:parseReport(x.report)} as unknown as ImprovementRun;}
  return {...s,run} as ImprovementStatus;
}
export function improvementStatusForWorkspace(value: ImprovementStatus, workspacePath: string): ImprovementStatus | null {
  return value.workspacePath === workspacePath ? value : null;
}
export function canUseImprovementPrimary(value: ImprovementStatus | null): boolean {
  return value?.available === true;
}
export function improvementHandoffLabel(intent: 'implementation'|'investigation'|undefined): string {
  return intent === 'investigation' ? 'Investigate with AI' : 'Fix with AI';
}
export function readImprovementIntent(assessment: unknown): 'implementation'|'investigation'|undefined|null {
  if(assessment===undefined||assessment===null)return undefined;
  if(typeof assessment!=='object'||Array.isArray(assessment))return null;
  const provenance=(assessment as Record<string,unknown>).improvementReference;
  if(provenance===undefined)return undefined;
  if(!provenance||typeof provenance!=='object'||Array.isArray(provenance))return null;
  const value=provenance as Record<string,unknown>;
  return Object.keys(value).sort().join(',')==='intent,version'&&value.version===1&&(value.intent==='implementation'||value.intent==='investigation')?value.intent:null;
}
