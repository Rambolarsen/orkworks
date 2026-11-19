// Offline logical oracle only. No application owner, I/O, process or provider binding.
const MAX_U64 = 18446744073709551615n;
const MAX_ATTEMPTS = 256;
const ATTEMPT_BYTES = 4096;
const REGISTRY_BYTES = 2097152;
const envelopeKeys = ['schemaVersion','workspaceId','revision','attempts'];
const attemptKeys = ['attemptId','previewId','allocatedSessionId','recommendationId','sourceSessionId','proposalRevision','preparedInputDigest','state','recordRevision','reason'];
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const hex = /^[0-9a-f]{64}$/;
const opaque = /^[A-Za-z0-9_-]{1,128}$/;
const reasons = {
 reserved:[null], dispatchCommitted:[null], linked:[null], settled:[null],
 rejectedBeforeCreate:['stale','expired','source_unavailable','configuration_unavailable','creation_failed'],
 unknown:['storage_uncertain','creation_uncertain','source_owner_lost'],
};
const occupied = new Set(['reserved','dispatchCommitted','linked','unknown']);
const copy = value => structuredClone(value);
const requireValue = (condition, reason) => { if (!condition) throw new Error(reason); };
function counter(value, zero=false) {
 return typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/.test(value) && BigInt(value)<=MAX_U64 && (zero || value!=='0');
}
function shape(value, keys) {
 requireValue(value!==null && typeof value==='object' && !Array.isArray(value), 'object required');
 requireValue(Object.keys(value).length===keys.length && keys.every(key=>Object.hasOwn(value,key)), 'closed keys');
}
const ordered = (value, keys) => Object.fromEntries(keys.map(key=>[key,value[key]]));
function checkedAttempt(value) {
 shape(value,attemptKeys);
 for(const key of ['attemptId','previewId','allocatedSessionId']) requireValue(typeof value[key]==='string' && uuid.test(value[key]), 'UUIDv4 required');
 for(const key of ['recommendationId','sourceSessionId']) requireValue(typeof value[key]==='string' && opaque.test(value[key]), 'opaque identifier required');
 for(const key of ['proposalRevision','recordRevision']) requireValue(counter(value[key]), 'positive u64 required');
 requireValue(typeof value.preparedInputDigest==='string' && hex.test(value.preparedInputDigest), 'digest required');
 requireValue(typeof value.state==='string' && Object.hasOwn(reasons,value.state) && reasons[value.state].includes(value.reason), 'state/reason');
 const result=ordered(value,attemptKeys);
 requireValue(Buffer.byteLength(JSON.stringify(result))<=ATTEMPT_BYTES,'attempt bytes');
 return result;
}
export function encodeRegistry(value, expectedWorkspace) {
 shape(value,envelopeKeys);
 requireValue(typeof expectedWorkspace==='string' && hex.test(expectedWorkspace) && value.workspaceId===expectedWorkspace,'workspace owner mismatch');
 requireValue(value.schemaVersion===1 && counter(value.revision),'registry version/revision');
 requireValue(Array.isArray(value.attempts) && value.attempts.length<=MAX_ATTEMPTS,'attempt count');
 const attempts=Array.from(value.attempts,checkedAttempt);
 for(const key of ['attemptId','allocatedSessionId']) requireValue(new Set(attempts.map(a=>a[key])).size===attempts.length,'duplicate identity');
 const active=attempts.filter(a=>occupied.has(a.state));
 requireValue(new Set(active.map(a=>a.sourceSessionId)).size===active.length,'duplicate occupied source');
 const result=Buffer.from(JSON.stringify({schemaVersion:1,workspaceId:value.workspaceId,revision:value.revision,attempts}));
 requireValue(result.length<=REGISTRY_BYTES,'registry bytes');
 return result;
}
export function decodeRegistry(bytes, expectedWorkspace) {
 requireValue(bytes instanceof Uint8Array && bytes.byteLength<=REGISTRY_BYTES,'raw registry bytes');
 // ignoreBOM preserves a BOM as a character, causing JSON.parse to reject it.
 const parsed=JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes));
 const canonical=encodeRegistry(parsed,expectedWorkspace);
 requireValue(canonical.equals(Buffer.from(bytes)),'noncanonical registry');
 return parsed;
}
const bindingKeys=['owner','generation','branch','kind','anchor','pinned','selectionId'];
function validBinding(value) {
 if (!value || Object.keys(value).length!==bindingKeys.length || !bindingKeys.every(k=>Object.hasOwn(value,k))) return false;
 return counter(value.generation,true) && BigInt(value.generation)<MAX_U64 &&
  typeof value.pinned==='boolean' && ['owner','branch','kind','anchor','selectionId'].every(k=>typeof value[k]==='string' && value[k].length>0);
}
export function bindingMatches(original,current) {
 return validBinding(original) && validBinding(current) && bindingKeys.every(key=>original[key]===current[key]);
}
export function newModel(workspaceId) {
 const registry={schemaVersion:1,workspaceId,revision:'1',attempts:[]};
 encodeRegistry(registry,workspaceId);
 return {registry, fenced:false, live:[]};
}
const output=(model,result)=>({model,result});
function fence(model) {
 const next=copy(model);next.fenced=true;next.live=[];
 return output(next,'unavailable');
}
const get=(model,id)=>model.registry.attempts.find(a=>a.attemptId===id);
const owner=(model,id)=>model.live.find(a=>a.id===id);
function publish(model,next,fault) {
 requireValue(fault===undefined || fault==='before' || fault==='after','fault checkpoint');
 encodeRegistry(next.registry,next.registry.workspaceId);
 if (fault) {
  const uncertain=copy(fault==='before'?model:next);
  uncertain.fenced=true;
  uncertain.live=[];
  return output(uncertain,'unavailable');
 }
 return output(next,'published');
}
function change(model,id,state,reason=null,fault) {
 const next=copy(model),record=get(next,id);
 requireValue(record,'missing attempt');
 if(BigInt(record.recordRevision)===MAX_U64 || BigInt(next.registry.revision)===MAX_U64) return fence(model);
 record.state=state;record.reason=reason;
 record.recordRevision=String(BigInt(record.recordRevision)+1n);
 next.registry.revision=String(BigInt(next.registry.revision)+1n);
 return publish(model,next,fault);
}
export function reserve(model,record,{previewValid=false,fault}={}) {
 checkedAttempt(record);
 const retained=get(model,record.attemptId);
 if(retained) return output(model,retained.recommendationId===record.recommendationId && retained.previewId===record.previewId?(model.fenced?'unavailable':'replay'):'conflict');
 if(model.fenced || previewValid!==true || model.registry.attempts.length===MAX_ATTEMPTS || BigInt(model.registry.revision)===MAX_U64) return output(model,'unavailable');
 if(model.registry.attempts.some(a=>a.allocatedSessionId===record.allocatedSessionId || (a.sourceSessionId===record.sourceSessionId && occupied.has(a.state)))) return output(model,'conflict');
 requireValue(record.state==='reserved' && record.recordRevision==='1' && record.reason===null,'fresh reservation');
 const next=copy(model);
 next.registry.attempts.push(copy(record));
 next.registry.revision=String(BigInt(next.registry.revision)+1n);
 next.live.push({id:record.attemptId,entered:false});
 return publish(model,next,fault);
}
export function dispatch(model,id,{fault}={}) {
 if(model.fenced || get(model,id)?.state!=='reserved' || !owner(model,id)) return output(model,'unavailable');
 const out=change(model,id,'dispatchCommitted',null,fault);
 return out;
}
export function enterCreator(model,id,{validation,fault}={},simulatedCreate) {
 const live=owner(model,id);
 if(model.fenced || get(model,id)?.state!=='dispatchCommitted' || !live || live.entered) return output(model,'unavailable');
 if(BigInt(model.registry.revision)===MAX_U64 || BigInt(get(model,id).recordRevision)===MAX_U64) return fence(model);
 if(validation!=='valid' && !reasons.rejectedBeforeCreate.includes(validation)) return output(model,'unavailable');
 if(validation!=='valid') {
  requireValue(reasons.rejectedBeforeCreate.includes(validation),'known pre-entry rejection');
  return change(model,id,'rejectedBeforeCreate',validation,fault);
 }
 if(fault==='before') return change(model,id,'unknown','creation_uncertain');
 requireValue(fault===undefined || fault==='after','creator checkpoint');
 const next=copy(model);
 owner(next,id).entered=true;
 try { simulatedCreate(get(next,id).allocatedSessionId); }
 catch { return change(next,id,'unknown','creation_uncertain'); }
 if(fault==='after') return change(next,id,'unknown','creation_uncertain');
 return output(next,'creatorEntered');
}
export function link(model,id,{fault}={}) {
 if(model.fenced || get(model,id)?.state!=='dispatchCommitted' || !owner(model,id)?.entered) return output(model,'unavailable');
 return change(model,id,'linked',null,fault);
}
function exactEvidence(model,id,evidence,state) {
 return evidence?.qualified===true && evidence.sessionId===get(model,id)?.allocatedSessionId && evidence.disposition===state;
}
export function settle(model,id,evidence,{fault}={}) {
 if(model.fenced || get(model,id)?.state!=='linked' || !exactEvidence(model,id,evidence,'settled')) return output(model,'unavailable');
 return change(model,id,'settled',null,fault);
}
export function resolveEvidence(model,id,evidence,{fault}={}) {
 if(model.fenced || get(model,id)?.state!=='unknown' || !['linked','settled'].includes(evidence?.disposition) || !exactEvidence(model,id,evidence,evidence.disposition)) return output(model,'unavailable');
 return change(model,id,evidence.disposition,null,fault);
}
export function restart(model,{fault}={}) {
 // A failed publication is not reconciled by successful readback or this logical restart.
 if(model.fenced) return output(model,'unavailable');
 let next=copy(model); next.live=[];
 for(const record of model.registry.attempts) if(['reserved','dispatchCommitted'].includes(record.state)) {
  const out=change(next,record.attemptId,'unknown','source_owner_lost',fault);
  if(out.result!=='published') return out;
  next=out.model;
 }
 return output(next,'reopened');
}
