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
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw Error('Invalid readiness response');
  }
  return value as Record<string, unknown>;
}

const text = (value: unknown, max: number) => typeof value === 'string' && value.length <= max;
const digest = (value: unknown) => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);

function validateRow(value: unknown, index: number): number | null {
  const row = object(value);
  if (
    row.conceptId !== conceptIds[index] ||
    !text(row.rationale, 500) || !text(row.nextTransition, 500) ||
    !Array.isArray(row.limits) || row.limits.length > 4 || row.limits.some(x => !text(x, 200)) ||
    !Array.isArray(row.repositoryFactHashes) || row.repositoryFactHashes.length > 8 ||
    row.repositoryFactHashes.some(x => !digest(x)) || !text(row.rubricPageId, 256)
  ) {
    throw Error('Invalid readiness row');
  }

  if (row.maturity === null) {
    if (row.confidence !== null || !text(row.unassessedReason, 500) || !row.unassessedReason) {
      throw Error('Invalid unassessed row');
    }
    return null;
  }
  if (
    !Number.isInteger(row.maturity) || (row.maturity as number) < 0 || (row.maturity as number) > 4 ||
    !['low', 'medium', 'high'].includes(row.confidence as string) ||
    row.unassessedReason !== null || row.repositoryFactHashes.length === 0
  ) {
    throw Error('Invalid readiness rating');
  }
  return row.maturity as number;
}

function validateData(value: unknown): void {
  const data = object(value);
  if (
    !digest(data.comparisonFingerprint) || !digest(data.inputFingerprint) || !text(data.summary, 1000) ||
    !Array.isArray(data.concepts) || data.concepts.length !== 14 ||
    !Array.isArray(data.repositoryEvidence) || data.repositoryEvidence.length > 32 ||
    !Array.isArray(data.knowledgeEvidence) || data.knowledgeEvidence.length > 14
  ) {
    throw Error('Invalid readiness details');
  }

  let coverage = 0;
  let total = 0;
  data.concepts.forEach((item, index) => {
    const maturity = validateRow(item, index);
    if (maturity !== null) {
      coverage++;
      total += maturity;
    }
  });
  if (
    data.coverage !== coverage || data.rawTotal !== (coverage === 14 ? total : null) ||
    data.overallScore !== (coverage === 14 ? Math.floor((100 * total + 28) / 56) : null)
  ) {
    throw Error('Invalid readiness totals');
  }

  for (const item of data.repositoryEvidence) {
    const fact = object(item);
    if (
      !digest(fact.factId) || !text(fact.path, 512) || !digest(fact.sha256) ||
      !text(fact.excerpt, 4000) || !text(fact.observedAt, 40)
    ) {
      throw Error('Invalid readiness evidence');
    }
  }
  for (const item of data.knowledgeEvidence) {
    const page = object(item);
    if (!text(page.pageId, 256) || !digest(page.sha256) || !text(page.title, 512) || !text(page.excerpt, 4000)) {
      throw Error('Invalid readiness knowledge');
    }
  }
  object(data.inferenceIdentity);
  object(data.knowledgeIdentity);
  const git = object(data.git);
  if (
    !(git.headOid === null || text(git.headOid, 64)) ||
    !(git.dirty === null || typeof git.dirty === 'boolean') || !digest(data.rubricDigest)
  ) {
    throw Error('Invalid readiness provenance');
  }
}

function validateSnapshot(value: unknown, ids: Set<string>): void {
  const snapshot = object(value);
  if (
    !text(snapshot.id, 128) || ids.has(snapshot.id as string) ||
    !text(snapshot.startedAt, 40) || !text(snapshot.completedAt, 40) || typeof snapshot.redacted !== 'boolean'
  ) {
    throw Error('Invalid readiness snapshot');
  }
  ids.add(snapshot.id as string);
  if (snapshot.redacted) {
    if (snapshot.data !== null) throw Error('Redacted readiness contains source data');
    return;
  }
  validateData(snapshot.data);
}

export function parseReadinessStatus(value: unknown): ReadinessStatus {
  const status = object(value);
  if (
    !(status.workspacePath === null || text(status.workspacePath, 4096)) ||
    !Array.isArray(status.snapshots) || status.snapshots.length > 100 || status.retentionLimit !== 100 ||
    !['none', 'current', 'changed', 'unknown', 'redacted'].includes(status.freshness as string) ||
    !(status.reasonCode === null || text(status.reasonCode, 128))
  ) {
    throw Error('Invalid readiness status');
  }
  const ids = new Set<string>();
  for (const snapshot of status.snapshots) validateSnapshot(snapshot, ids);

  if (status.run !== null) {
    const run = object(status.run);
    if (
      !text(run.id, 128) || !['queued', 'running', 'succeeded', 'failed', 'interrupted'].includes(run.state as string) ||
      !(run.phase === null || ['collecting_evidence', 'reading_evidence', 'scoring_readiness'].includes(run.phase as string))
    ) {
      throw Error('Invalid readiness progress');
    }
  }
  return status as unknown as ReadinessStatus;
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
