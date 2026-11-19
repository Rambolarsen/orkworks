// Offline logical components only. No actual source custody, native eligibility or I/O.
import {createHash} from 'node:crypto';
import {posix,win32} from 'node:path';
const MAX=18446744073709551615n;
const DOMAIN='orkworks.ordinary-plan-review.prepared-input.v1\n';
const LIMIT={proposal:24576,configuration:12288,prepared:49152,request:24576};
const OWNER_KEYS=['workspace','proposal','source','artifact','configuration'];
const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const opaque=/^[A-Za-z0-9_-]{1,128}$/;
const hex=/^[0-9a-f]{64}$/;
const requireValue=(condition,reason)=>{if(!condition)throw new Error(reason);};
const object=x=>x!==null && typeof x==='object' && !Array.isArray(x);
function shape(x,keys){requireValue(object(x) && Object.keys(x).length===keys.length && keys.every(k=>Object.hasOwn(x,k)),'closed keys');}
function record(x,fields){const keys=fields.split(' ');shape(x,keys);return Object.fromEntries(keys.map(k=>[k,x[k]]));}
function text(x,max,{empty=false,controls=false}={}){
 requireValue(typeof x==='string' && x.isWellFormed() && (empty || x.length>0) && Buffer.byteLength(x)<=max,'bounded scalar UTF-8');
 requireValue(!(controls?/[\x00-\x1f\x7f]/:/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/).test(x),'text controls');
}
function match(x,pattern){requireValue(typeof x==='string' && pattern.test(x),'identifier');}
function counter(x,{zero=false,exhausted=true}={}){
 match(x,/^(0|[1-9][0-9]{0,19})$/);
 requireValue((zero || x!=='0') && BigInt(x)<=MAX && (exhausted || BigInt(x)<MAX),'u64 domain');
}
function absolute(x,platform){
 text(x,4096,{controls:true});
 // Lexical absolute path shape only; native canonicality/custody is unqualified.
 requireValue(platform==='windows_x64'?win32.isAbsolute(x) && (/^[A-Za-z]:[\\/]/.test(x) || /^\\\\[^\\/]+[\\/][^\\/]+/.test(x)):posix.isAbsolute(x),'absolute path');
}
function bounded(value,limit){const bytes=Buffer.from(JSON.stringify(value));requireValue(bytes.length<=limit,'complete encoded bytes');return bytes;}
function draft(x){
 const r=record(x,'codingToolId modelId userInstruction');
 match(r.codingToolId,opaque);text(r.modelId,256,{controls:true});text(r.userInstruction,16384);
 return r;
}
function proposal(x){
 const r=record(x,'schemaVersion intent recommendationId proposalRevision sourceSessionId sourceRuntime request chain proposedDraft');
 requireValue(r.schemaVersion===1 && r.intent==='ordinary_plan_review','proposal schema/intent');
 match(r.recommendationId,opaque);counter(r.proposalRevision);match(r.sourceSessionId,opaque);
 r.sourceRuntime=record(r.sourceRuntime,'runtimeInstanceId runGeneration');match(r.sourceRuntime.runtimeInstanceId,uuid);counter(r.sourceRuntime.runGeneration);
 r.request=record(r.request,'requestId requestRevision text');match(r.request.requestId,uuid);counter(r.request.requestRevision);text(r.request.text,2048);
 r.chain=record(r.chain,'chainId depth');match(r.chain.chainId,opaque);requireValue(Number.isInteger(r.chain.depth) && !Object.is(r.chain.depth,-0) && r.chain.depth>=0 && r.chain.depth<=2,'chain depth');
 r.proposedDraft=draft(r.proposedDraft);bounded(r,LIMIT.proposal);return r;
}
function configuration(x){
 const r=record(x,'profile ownerInstanceId ownerRevision codingToolId modelId platform executable argv catalogDigest integrationDigest capabilityEvidenceId capabilityEvidenceRevision role assurance skills reasoningEffort images');
 requireValue(r.profile==='ordinary-plan-review-instruction-v1' && r.role==='reviewer' && r.assurance==='instruction_only','narrow profile');
 match(r.ownerInstanceId,uuid);counter(r.ownerRevision);match(r.codingToolId,opaque);requireValue(r.codingToolId!=='generic-shell','no generic fallback');text(r.modelId,256,{controls:true});
 requireValue(['macos_arm64','linux_x64','windows_x64'].includes(r.platform),'platform tag');absolute(r.executable,r.platform);
 requireValue(Array.isArray(r.argv) && r.argv.length<=64,'argv count');r.argv=Array.from(r.argv,a=>{text(a,1024,{empty:true});return a;});
 match(r.catalogDigest,hex);match(r.integrationDigest,hex);match(r.capabilityEvidenceId,opaque);counter(r.capabilityEvidenceRevision);
 for(const k of ['skills','images']){requireValue(Array.isArray(r[k]) && r[k].length===0,'unsupported additions');r[k]=[];}
 requireValue(r.reasoningEffort===null,'no custom effort');bounded(r,LIMIT.configuration);return r;
}
function prepared(x){
 const r=record(x,'schemaVersion previewId workspace proposal source artifact configuration userInstruction');

 requireValue(r.schemaVersion===1,'prepared schema');
 match(r.previewId,uuid);

 r.proposal=proposal(r.proposal);
 r.configuration=configuration(r.configuration);

 const platform=r.configuration.platform;

 r.workspace=record(r.workspace,'workspaceId sidecarInstanceId workspaceGeneration canonicalRoot');
 match(r.workspace.workspaceId,hex);
 match(r.workspace.sidecarInstanceId,uuid);
 counter(r.workspace.workspaceGeneration);
 absolute(r.workspace.canonicalRoot,platform);

 r.source=record(r.source,'sessionId runtimeInstanceId runGeneration eligibilityRevision lifecycle organization effectiveWorktree git');

 match(r.source.sessionId,opaque);
 match(r.source.runtimeInstanceId,uuid);
 counter(r.source.runGeneration);
 counter(r.source.eligibilityRevision);
 requireValue(r.source.lifecycle==='live' && r.source.organization==='active','source eligibility shape');
 absolute(r.source.effectiveWorktree,platform);

 r.source.git=record(r.source.git,'commonDirectory head');
 absolute(r.source.git.commonDirectory,platform);

 const head=r.source.git.head;

 if(head?.kind==='branch'){r.source.git.head=record(head,'kind reference');
 text(head.reference,1024,{controls:true});
 }
 else {r.source.git.head=record(head,'kind commit');
 requireValue(head.kind==='detached','head kind');
 match(head.commit,/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/);
 }
 r.artifact=record(r.artifact,'ownerCaptureId generation branchIdentity kind anchor reference pinned selectionId');
 match(r.artifact.ownerCaptureId,uuid);
 counter(r.artifact.generation,{zero:true,exhausted:false});
 text(r.artifact.branchIdentity,1024,{controls:true});
 requireValue(['design','implementation'].includes(r.artifact.kind),'selected document kind');

 text(r.artifact.anchor,2048,{controls:true});
 text(r.artifact.reference,2048,{controls:true});
 requireValue(typeof r.artifact.pinned==='boolean','pin');
 match(r.artifact.selectionId,/^[\x20-\x7e]{1,128}$/);

 text(r.userInstruction,16384);

 requireValue(r.source.sessionId===r.proposal.sourceSessionId && r.source.runtimeInstanceId===r.proposal.sourceRuntime.runtimeInstanceId && r.source.runGeneration===r.proposal.sourceRuntime.runGeneration,'proposal/source runtime join');

 requireValue(r.source.effectiveWorktree===r.workspace.canonicalRoot,'current-root join');

 if(head.kind==='branch')requireValue(r.artifact.branchIdentity===head.reference,'artifact/branch join');

 // Cwd is workspace.canonicalRoot. The selected draft is independently joined below.
 bounded(r,LIMIT.prepared);
 return r;

}
function previewRequest(x){
 const r=record(x,'schemaVersion expectedProposalRevision draft');requireValue(r.schemaVersion===1,'request schema');counter(r.expectedProposalRevision);r.draft=r.draft===null?null:draft(r.draft);return r;
}
function decode(bytes,limit,encoder){
 requireValue(bytes instanceof Uint8Array && bytes.byteLength<=limit,'raw bytes');
 const value=JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes));
 requireValue(encoder(value).equals(Buffer.from(bytes)),'canonical byte equality');return value;
}
export const encodeProposal=x=>bounded(proposal(x),LIMIT.proposal);
export const decodeProposal=bytes=>decode(bytes,LIMIT.proposal,encodeProposal);
export const encodeConfiguration=x=>bounded(configuration(x),LIMIT.configuration);
export const decodeConfiguration=bytes=>decode(bytes,LIMIT.configuration,encodeConfiguration);
export const encodePrepared=x=>bounded(prepared(x),LIMIT.prepared);
export const decodePrepared=bytes=>decode(bytes,LIMIT.prepared,encodePrepared);
export const encodePreviewRequest=x=>bounded(previewRequest(x),LIMIT.request);
export const decodePreviewRequest=bytes=>decode(bytes,LIMIT.request,encodePreviewRequest);
export function preparedDigest(value){return createHash('sha256').update(DOMAIN).update(encodePrepared(value)).digest('hex');}
export function validateJoin(value,entry,current){
 try{
  const captured=encodePrepared(entry.prepared);
  if(!captured.equals(encodePrepared(value)))return false;
  shape(entry,['prepared','draft','preparedAtMs','expiresAtMs','owners']);
  shape(current,[...OWNER_KEYS,'owners','status','targetSessionId','sourceEligible','configurationQualified','nowMs']);
  shape(entry.owners,OWNER_KEYS);shape(current.owners,OWNER_KEYS);
  if(value.proposal.proposalRevision===String(MAX))return false;
  if(current.status!=='proposed' || current.targetSessionId!==null || current.sourceEligible!==true || current.configurationQualified!==true)return false;
  if(![entry.preparedAtMs,entry.expiresAtMs,current.nowMs].every(n=>Number.isSafeInteger(n) && n>=0) || entry.expiresAtMs-entry.preparedAtMs!==30000 || current.nowMs<entry.preparedAtMs || current.nowMs>=entry.expiresAtMs)return false;
  const live={...value};
  for(const key of OWNER_KEYS){
   if(!object(entry.owners[key]) || entry.owners[key]!==current.owners[key])return false;
   live[key]=current[key];
  }
  if(!captured.equals(encodePrepared(live)))return false;
  const selected=draft(entry.draft===null?value.proposal.proposedDraft:entry.draft);
  if(selected.codingToolId!==value.configuration.codingToolId || selected.modelId!==value.configuration.modelId || selected.userInstruction!==value.userInstruction)return false;
  return true;
 }catch{return false;}
}
