// Offline accounting of archived components and one fixed empty-output profile.
// Not a general source codec, store validator, writer or authority implementation.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const order=(a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b));
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort(order).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const bytes=x=>Buffer.byteLength(canonical(x));
const hash=(domain,x)=>createHash('sha256').update(domain+'\n'+canonical(x)).digest('hex');
const same=(a,b)=>assert.deepEqual(a,b);
const domains={candidate:'orkworks.assignment-result-candidate.v1',result:'orkworks.assignment-result.v1',evaluation:'orkworks.assignment-evaluation.v1',disposition:'orkworks.assignment-disposition.v1'};
const resources=['assignment-record-bytes','result-payload-bytes'];
function owners(A){
  assert(['root','child'].includes(A.kind),'assignment variant');
  const fields=A.kind==='root'?'kind workspaceId runId rootAssignmentId parentSessionId bootstrapId bootstrapConfigurationDigest':'kind workspaceId runId planId planRevision taskId taskVersion reservationId parentSessionId childSessionId configurationId configurationDigest';
  same(Object.keys(A).sort(),fields.split(' ').sort());
  return [{kind:'assignment',assignmentIdentity:A},{kind:'run',workspaceId:A.workspaceId,runId:A.runId},{kind:'workspace',workspaceId:A.workspaceId}];
}
export function measureArchivedRecord(vector){
  const {sourceKind,source,expected}=vector;
  assert(Object.hasOwn(domains,sourceKind),'supported source kind');
  const sourceDigest=hash(domains[sourceKind],source);same(sourceDigest,expected.sourceDigest);
  assert(bytes(source)<=524288,'record bound');
  return {id:vector.id,sourceKind,bodyBytes:String(bytes(source)),sourceDigest,owners:owners(source.assignmentIdentity)};
}
function projection(row){
  const value=structuredClone(row.value);
  if(row.table==='claim_proofs')delete value.commit;
  else if(!['store_header','commit_proof','mutation_lookup'].includes(row.table))delete value.lastCommit;
  if(row.table==='summary_state'&&value.checkpoint)delete value.checkpoint.applied;
  if(row.table==='receipt_slots'&&value.receipt)delete value.receipt.commit;
  return value;
}
const rowId=row=>row.table+'\n'+row.keyCanonical;
function decode(state){
  const allowed=['store_header','commit_proof','objects','heads','candidate_work','receipt_slots','claim_proofs','mutation_lookup','quota_usage','summary_state'];
  const rows=state.rows.map(encoded=>{
    assert(allowed.includes(encoded.table),'unsupported table in empty-output profile');
    const key=JSON.parse(encoded.keyCanonical),value=JSON.parse(encoded.valueCanonical);
    same(canonical(key),encoded.keyCanonical);same(canonical(value),encoded.valueCanonical);
    const row={...encoded,key,value};
    same(hash('orkworks.hierarchy-storage-row.v3',{table:row.table,key,body:projection(row)}),row.rowDigest);
    return row;
  });
  const ids=rows.map(rowId);same(new Set(ids).size,ids.length);same(ids,[...ids].sort(order));
  return rows;
}
export function measureEmptyOutputState(state,packet){
  const rows=decode(state),A=packet.profile.assignmentIdentity,scopeOwners=owners(A);
  same(packet.profile.outputs,[]);
  let total=0n;const objects=rows.filter(r=>r.table==='objects');
  for(const r of objects){
    const kind=r.key.kind==='assignment-result-candidate'?'candidate':r.key.kind==='assignment-result'?'result':null;
    assert(kind,'unsupported retained source kind');
    const source=JSON.parse(packet.sources[kind+'Canonical']);same(canonical(source),packet.sources[kind+'Canonical']);
    same(r.value.body,source);same(source.assignmentIdentity,A);
    const digest=hash(domains[kind],source);same(digest,packet.sources[kind+'Digest']);same(r.value.digest,digest);
    const identity=kind==='candidate'?{assignmentIdentity:A,candidateDigest:digest}:{assignmentIdentity:A,resultRevision:source.resultRevision,resultDigest:digest};
    same(r.key,{kind:r.key.kind,identity});same(source.outputs,[]);
    total+=BigInt(bytes(source));
  }
  assert(objects.length<=2);assert(!objects.some(r=>r.key.kind==='assignment-result')||objects.some(r=>r.key.kind==='assignment-result-candidate'),'result retains its candidate');
  const ledgers=rows.filter(r=>r.table==='quota_usage'&&resources.includes(r.key.resource));
  if(ledgers.length){
    const expectedKeys=scopeOwners.flatMap(owner=>resources.map(resource=>({owner,resource})));
    same(ledgers.map(r=>canonical(r.key)).sort(order),expectedKeys.map(canonical).sort(order));
    for(const r of ledgers){same(r.value.used,r.key.resource==='assignment-record-bytes'?String(total):'0');same(r.value.reserved,'0');}
  }else{same(objects.length,0);assert(!rows.some(r=>r.table==='summary_state'),'prepared subject needs all source ledgers');}
  return {name:state.name,recordBytes:String(total),payloadBytes:'0',objectCount:objects.length,ledgerCount:ledgers.length};
}
export function measureEmptyOutputTransition(before,after,command,packet){
  const oldMeasure=measureEmptyOutputState(before,packet),newMeasure=measureEmptyOutputState(after,packet);
  same(command.beforeState,before.name);same(command.afterState,after.name);
  const oldRows=decode(before),newRows=decode(after),old=new Map(oldRows.map(r=>[rowId(r),r])),next=new Map(newRows.map(r=>[rowId(r),r]));
  assert([...old.keys()].every(k=>next.has(k)),'this archived profile has no deletions');
  const changed=newRows.filter(r=>!old.has(rowId(r))||r.valueCanonical!==old.get(rowId(r)).valueCanonical);
  const manifest=JSON.parse(command.manifestCanonical);same(canonical(manifest),command.manifestCanonical);
  same(hash('orkworks.hierarchy-storage-change.v3',manifest),command.changeDigest);
  const declared=changed.filter(r=>!['store_header','commit_proof'].includes(r.table)).map(r=>({table:r.table,key:r.key,beforeDigest:old.get(rowId(r))?.rowDigest??null,afterDigest:r.rowDigest}));
  same(manifest.changes,declared);
  const charges=changed.map(r=>({table:r.table,keyCanonical:r.keyCanonical,keyBytes:r.table==='mutation_lookup'?64:Buffer.byteLength(r.keyCanonical),valueBytes:Buffer.byteLength(r.valueCanonical)}));
  const commandBytes=charges.reduce((sum,r)=>sum+r.keyBytes+r.valueBytes,0);
  same(changed.length,command.changedRows);same(commandBytes,command.commandBytes);
  return {name:command.name,changedRows:changed.length,commandBytes,recordDelta:String(BigInt(newMeasure.recordBytes)-BigInt(oldMeasure.recordBytes)),payloadDelta:'0',charges};
}
