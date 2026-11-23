// Offline byte/conditional-join components; no actual collector, native eligibility or I/O.
import {createHash} from 'node:crypto';
import {posix,win32} from 'node:path';
import {encodePrepared,validateJoin as validatePreparedJoin} from './ordinary-review-launch-input-components.mjs';
const MAX=18446744073709551615n;
const CATALOG_DOMAIN='orkworks.ordinary-review.catalog-input.v1\n';
const INTEGRATION_DOMAIN='orkworks.ordinary-review.integration-input.v1\n';
const hash=(domain,bytes)=>createHash('sha256').update(domain).update(bytes).digest('hex');
const CATALOG_CAP=131072,INTEGRATION_CAP=524288,COMBINED_CAP=655360;
const PLATFORMS=['macos_arm64','linux_x64','windows_x64'];
const ROLES=['workspace_preference','native_owned_fragment','source_asset','installed_asset'];
const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const opaque=/^[A-Za-z0-9_-]{1,128}$/;
const hex=/^[0-9a-f]{64}$/;
const requireValue=(ok,why)=>{if(!ok)throw new Error(why);};
const object=x=>x!==null && typeof x==='object' && !Array.isArray(x);
function shape(x,fields){const keys=fields===''?[]:fields.split(' ');requireValue(object(x) && Object.keys(x).length===keys.length && keys.every(k=>Object.hasOwn(x,k)),'closed keys');return Object.fromEntries(keys.map(k=>[k,x[k]]));}
function match(x,re){requireValue(typeof x==='string' && re.test(x),'identifier');}
function text(x,max,{empty=false,controls=false}={}){
 requireValue(typeof x==='string' && x.isWellFormed() && (empty || x.length>0) && Buffer.byteLength(x)<=max,'scalar UTF-8');
 requireValue(!(controls?/[\x00-\x1f\x7f]/:/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/).test(x),'controls');
}
function counter(x,{zero=false,owner=false}={}){match(x,/^(0|[1-9][0-9]{0,19})$/);requireValue((zero || x!=='0') && BigInt(x)<=MAX && (!owner || BigInt(x)<MAX),'counter domain');}
function stockId(x){match(x,opaque);requireValue(!['generic-shell','gh-copilot','gemini'].includes(x),'unsupported first-profile ID');}
function absolute(x,platform){
 text(x,4096,{controls:true});
 requireValue(platform==='windows_x64'?win32.isAbsolute(x) && (/^[A-Za-z]:[\\/]/.test(x) || /^\\\\[^\\/]+[\\/][^\\/]+/.test(x)):posix.isAbsolute(x),'lexical absolute path');
}
function list(x,max,validate){requireValue(Array.isArray(x) && x.length<=max,'array bound');return Array.from(x,validate);}
function sortedUnique(xs,key=x=>x){const ids=xs.map(key);for(let n=1;n<ids.length;n++)requireValue(Buffer.compare(Buffer.from(ids[n-1]),Buffer.from(ids[n]))<0,'sorted unique IDs');}
function base64(x,cap){
 requireValue(typeof x==='string' && x.length<=4*Math.ceil(cap/3),'base64 bound');
 const bytes=Buffer.from(x,'base64');requireValue(bytes.length<=cap && bytes.toString('base64')===x,'canonical base64');return x;
}
function command(x,platform){
 const r=shape(x,'program argv cwd');absolute(r.program,platform);absolute(r.cwd,platform);
 r.argv=list(r.argv,64,a=>{text(a,1024,{empty:true});return a;});return r;
}
function bounded(value,cap){const bytes=Buffer.from(JSON.stringify(value));requireValue(bytes.length<=cap,'complete encoded bytes');return bytes;}
function catalog(x){
 const r=shape(x,'schemaVersion kind ownerInstanceId ownerRevision sidecarBuildId platform requestedToolId modelId builtinDocumentBase64 userDocumentState resolverPolicy resolvedCommand');
 requireValue(r.schemaVersion===1 && r.kind==='ordinary_review_catalog','catalog schema');
 match(r.ownerInstanceId,uuid);counter(r.ownerRevision,{owner:true});match(r.sidecarBuildId,hex);requireValue(PLATFORMS.includes(r.platform),'platform');
 stockId(r.requestedToolId);text(r.modelId,256,{controls:true});r.builtinDocumentBase64=base64(r.builtinDocumentBase64,65536);
 requireValue(r.userDocumentState==='absent' && r.resolverPolicy==='stock-registry-v2-default-user-v3-v1','stock absence policy');
 r.resolvedCommand=command(r.resolvedCommand,r.platform);return r;
}
function inventory(x){
 const result=list(x,64,item=>{const r=shape(item,'id role presence');match(r.id,opaque);requireValue(ROLES.includes(r.role) && ['required','observed'].includes(r.presence),'inventory domain');return r;});
 sortedUnique(result,r=>r.id);return result;
}
function integration(x,platform){
 requireValue(PLATFORMS.includes(platform),'explicit catalog platform');
 const r=shape(x,'schemaVersion kind ownerInstanceId ownerRevision workspaceId workspaceGeneration catalogDigest selectedToolId adapter activeSelection inventory resources runtimePolicy finalCommand');
 requireValue(r.schemaVersion===1 && r.kind==='ordinary_review_integration','integration schema');
 match(r.ownerInstanceId,uuid);counter(r.ownerRevision,{owner:true});match(r.workspaceId,hex);counter(r.workspaceGeneration);match(r.catalogDigest,hex);stockId(r.selectedToolId);
 r.adapter=shape(r.adapter,'binding representativeToolId policyVersion');
 requireValue(['none','claude','codex','open-code','gemini','copilot','aider'].includes(r.adapter.binding) && r.adapter.policyVersion==='owned-integration-inputs-v1','adapter policy');
 if(r.adapter.binding==='none')requireValue(r.adapter.representativeToolId===null,'none representative');else stockId(r.adapter.representativeToolId);
 r.activeSelection=shape(r.activeSelection,'revision toolIds');counter(r.activeSelection.revision,{zero:true});r.activeSelection.toolIds=list(r.activeSelection.toolIds,64,id=>{stockId(id);return id;});sortedUnique(r.activeSelection.toolIds);
 requireValue(r.activeSelection.toolIds.includes(r.selectedToolId),'selected tool active');
 r.inventory=inventory(r.inventory);
 r.resources=list(r.resources,64,item=>{
  const z=shape(item,'id role path state bytesBase64');match(z.id,opaque);requireValue(ROLES.includes(z.role),'resource role');absolute(z.path,platform);
  requireValue(['absent','regular'].includes(z.state),'resource state');
  if(z.state==='absent')requireValue(z.bytesBase64===null,'absence bytes');else z.bytesBase64=base64(z.bytesBase64,262144);
  return z;
 });
 sortedUnique(r.resources,z=>z.id);requireValue(r.resources.length===r.inventory.length,'coverage count');
 for(let n=0;n<r.inventory.length;n++){
  const expected=r.inventory[n],observed=r.resources[n];
  requireValue(expected.id===observed.id && expected.role===observed.role,'coverage membership');
  requireValue(expected.presence!=='required' || observed.state==='regular','required presence');
 }
 r.runtimePolicy=shape(r.runtimePolicy,'terminalEnvironment sessionReporting promptHook worktreeInstructions initialInstruction');
 requireValue(r.runtimePolicy.terminalEnvironment==='terminal-v1' && r.runtimePolicy.sessionReporting==='fresh-session-report-v1' && r.runtimePolicy.promptHook==='disabled' && r.runtimePolicy.worktreeInstructions==='none' && r.runtimePolicy.initialInstruction==='exact-one-turn-v1','fixed runtime policy');
 r.finalCommand=command(r.finalCommand,platform);return r;
}
export const encodeCatalog=x=>bounded(catalog(x),CATALOG_CAP);
export const encodeIntegration=(x,platform)=>bounded(integration(x,platform),INTEGRATION_CAP);
function decode(bytes,cap,encode){
 requireValue(bytes instanceof Uint8Array && bytes.byteLength<=cap,'raw bytes');
 const value=JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes));
 requireValue(encode(value).equals(Buffer.from(bytes)),'canonical byte equality');return value;
}
export const decodeCatalog=bytes=>decode(bytes,CATALOG_CAP,encodeCatalog);
export const decodeIntegration=(bytes,platform)=>decode(bytes,INTEGRATION_CAP,x=>encodeIntegration(x,platform));
export const catalogDigest=x=>hash(CATALOG_DOMAIN,encodeCatalog(x));
export const integrationDigest=(x,platform)=>hash(INTEGRATION_DOMAIN,encodeIntegration(x,platform));
export function measureCapture(c,i){const catalogBytes=encodeCatalog(c).length,integrationBytes=encodeIntegration(i,c.platform).length,totalBytes=catalogBytes+integrationBytes;requireValue(totalBytes<=COMBINED_CAP,'combined encoded bytes');return {catalogBytes,integrationBytes,totalBytes};}
export function validateLeafJoin(c,i,p,facts){
 try{
  const cb=encodeCatalog(c),ib=encodeIntegration(i,c.platform),pb=encodePrepared(p);
  const cd=hash(CATALOG_DOMAIN,cb),id=hash(INTEGRATION_DOMAIN,ib);
  const f=shape(facts,'expectedInventory captured current');
  const captured=shape(f.captured,'catalog integration prepared configurationOwner resourceOwners preparedEntry');
  const current=shape(f.current,'catalog integration configurationOwner resourceOwners preparedCurrent sourceFacts');
  const s=shape(current.sourceFacts,'stockToolIds stockResolverQualified inventoryQualified fragmentsQualified literalCommandQualified userDocumentState requiresLateArgv requiresPromptAuthority');
  for(const flag of ['stockResolverQualified','inventoryQualified','fragmentsQualified','literalCommandQualified'])if(s[flag]!==true)return false;
  if(s.userDocumentState!=='absent' || s.requiresLateArgv!==false || s.requiresPromptAuthority!==false)return false;
  const stockTools=list(s.stockToolIds,64,id=>{stockId(id);return id;});sortedUnique(stockTools);
  if(![c.requestedToolId,...i.activeSelection.toolIds,...(i.adapter.representativeToolId===null?[]:[i.adapter.representativeToolId])].every(id=>stockTools.includes(id)))return false;
  if(!object(captured.configurationOwner) || captured.configurationOwner!==current.configurationOwner || captured.configurationOwner!==captured.preparedEntry.owners.configuration || current.configurationOwner!==current.preparedCurrent.owners.configuration)return false;
  const ids=i.resources.map(r=>r.id);shape(captured.resourceOwners,ids.join(' '));shape(current.resourceOwners,ids.join(' '));
  for(const id of ids)if(!object(captured.resourceOwners[id]) || captured.resourceOwners[id]!==current.resourceOwners[id])return false;
  if(!cb.equals(encodeCatalog(captured.catalog)) || !cb.equals(encodeCatalog(current.catalog)))return false;
  if(!ib.equals(encodeIntegration(captured.integration,c.platform)) || !ib.equals(encodeIntegration(current.integration,c.platform)))return false;
  if(!pb.equals(encodePrepared(captured.prepared)))return false;
  if(JSON.stringify(inventory(f.expectedInventory))!==JSON.stringify(inventory(i.inventory)))return false;
  const config=p.configuration;
  if(c.ownerInstanceId!==i.ownerInstanceId || c.ownerInstanceId!==config.ownerInstanceId || c.ownerRevision!==i.ownerRevision || c.ownerRevision!==config.ownerRevision)return false;
  if(c.platform!==config.platform || c.requestedToolId!==i.selectedToolId || c.requestedToolId!==config.codingToolId || c.modelId!==config.modelId)return false;
  if(i.workspaceId!==p.workspace.workspaceId || i.workspaceGeneration!==p.workspace.workspaceGeneration)return false;
  if(i.catalogDigest!==cd || config.catalogDigest!==cd || config.integrationDigest!==id)return false;
  if(c.resolvedCommand.cwd!==p.workspace.canonicalRoot || i.finalCommand.cwd!==p.workspace.canonicalRoot || i.finalCommand.program!==config.executable || JSON.stringify(i.finalCommand.argv)!==JSON.stringify(config.argv))return false;
  return validatePreparedJoin(p,captured.preparedEntry,current.preparedCurrent);
 }catch{return false;}
}
