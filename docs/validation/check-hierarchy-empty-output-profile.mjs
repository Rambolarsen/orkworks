// Reproducible synthetic logical-store witness. Never opens a database or model.
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const dir=new URL('./',import.meta.url);
const seedBytes=readFileSync(new URL('hierarchy-claim-proof-vectors.json',dir));
const compactBytes=readFileSync(new URL('hierarchy-compact-claim-vectors.json',dir));
const seed=JSON.parse(seedBytes).vectors.find(v=>v.id==='root-candidate-initial');
const compact=JSON.parse(compactBytes).vectors.find(v=>v.id===seed.id);
const clone=x=>structuredClone(x);
const canon=x=>Array.isArray(x)?'['+x.map(canon).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canon(x[k])).join(',')+'}':JSON.stringify(x);
const bytes=x=>Buffer.byteLength(canon(x));
const sha=x=>createHash('sha256').update(x).digest('hex');
const hash=(domain,x)=>sha(domain+'\n'+canon(x));
const A=seed.source.assignmentIdentity,epoch=compact.compactProof.decision.storageEpoch;
const candidate=clone(seed.source),cd=hash('orkworks.assignment-result-candidate.v1',candidate);
assert.equal(cd,seed.expected.sourceDigest);
const result={...clone(candidate),resultRevision:1,candidateDigest:cd,timeCostEvidence:[]};
const rd=hash('orkworks.assignment-result.v1',result);
const C={key:{kind:'assignment-result-candidate',identity:{assignmentIdentity:A,candidateDigest:cd}},digest:cd};
const R={assignmentIdentity:A,resultRevision:1,resultDigest:rd};
const resultRef={key:{kind:'assignment-result',identity:R},digest:rd};
const criteria=[{id:'c',requirement:'required',description:'The assignment outcome is supported by evidence.'}];
const rubric={id:'r',version:1,dimensions:[{id:'d',description:'The outcome is correct.'}],evaluatorRole:'review'};
const material=accepted=>({schemaVersion:1,assignmentIdentity:A,accepted,criteriaSnapshotDigest:hash('orkworks.criteria-snapshot.v1',criteria),rubricSnapshotDigest:hash('orkworks.rubric-snapshot.v1',rubric),overall:'unassessed',requiredOutcomes:[{target:{kind:'criterion',id:'c'},disposition:'unassessed'},{target:{kind:'quality-dimension',id:'d'},disposition:'unassessed'},{target:{kind:'rating'},disposition:'unassessed'}].sort((a,b)=>Buffer.compare(Buffer.from(canon(a.target)),Buffer.from(canon(b.target)))),requiredFindings:[]});
const M0=material({kind:'none'}),M1=material({kind:'result',reference:R});
const owners=[{kind:'assignment',assignmentIdentity:A},{kind:'run',workspaceId:A.workspaceId,runId:A.runId},{kind:'workspace',workspaceId:A.workspaceId}];
const W=owners[2],scopeResources=['reporting-receipt-bytes','assignment-record-bytes','result-payload-bytes'];
const summaryResources=['summary-row-bytes','summary-row-count','summary-lineage-bytes','summary-pending-count'];
const capFor=(owner,r)=>r==='reporting-receipt-bytes'?({assignment:17,run:64,workspace:256}[owner.kind]*1048576):r==='assignment-record-bytes'?1048576:r==='result-payload-bytes'?0:r==='summary-row-bytes'?256*1048576:r==='summary-row-count'?4096:r==='summary-lineage-bytes'?64*1048576:4096;
const capWitness=[...owners.flatMap(owner=>scopeResources.map(resource=>({key:{owner,resource},maximumTotal:String(capFor(owner,resource))}))),...summaryResources.map(resource=>({key:{owner:W,resource},maximumTotal:String(capFor(W,resource))}))];
const rowId=(table,key)=>table+'\n'+canon(key);
const row=(table,key,value)=>({table,key:clone(key),value:clone(value)});
const project=r=>{
 const v=clone(r.value);
 if(r.table==='claim_proofs')delete v.commit;
 else if(!['store_header','commit_proof','mutation_lookup'].includes(r.table))delete v.lastCommit;
 if(r.table==='summary_state'&&v.checkpoint)delete v.checkpoint.applied;
 if(r.table==='receipt_slots'&&v.receipt)delete v.receipt.commit;
 return v;
};
const digest=r=>hash('orkworks.hierarchy-storage-row.v3',{table:r.table,key:r.key,body:project(r)});
let store=new Map();const commands=[],states=[{name:'absent',rows:[]}];
const sortedRows=s=>[...s.values()].sort((a,b)=>Buffer.compare(Buffer.from(a.table),Buffer.from(b.table))||Buffer.compare(Buffer.from(canon(a.key)),Buffer.from(canon(b.key))));
const encodeState=(name,s)=>({name,rows:sortedRows(s).map(r=>({table:r.table,keyCanonical:canon(r.key),valueCanonical:canon(r.value),rowDigest:digest(r)}))});
const keyBytes=r=>r.table==='mutation_lookup'?64:bytes(r.key);
const charge=r=>keyBytes(r)+bytes(r.value);
function commit(name,kind,intents){
 const before=store,newStore=new Map([...before].map(([k,v])=>[k,clone(v)]));
 const oldHeader=before.get(rowId('store_header',1));
 const sequence=oldHeader?String(BigInt(oldHeader.value.latest.commitSequence)+1n):'1';
 const changes=[];const ids=new Set();
 for(const intent of intents){
  const id=rowId(intent.table,intent.key);assert(!ids.has(id));ids.add(id);
  const old=before.get(id);assert(!old||canon(project(old))!==canon(project(intent)),'no link-only changes');
  changes.push({table:intent.table,key:intent.key,beforeDigest:old?digest(old):null,afterDigest:digest(intent)});
 }
 changes.sort((a,b)=>Buffer.compare(Buffer.from(a.table),Buffer.from(b.table))||Buffer.compare(Buffer.from(canon(a.key)),Buffer.from(canon(b.key))));
 const manifest={schemaVersion:3,workspaceId:A.workspaceId,storageEpoch:epoch,commitSequence:sequence,predecessor:oldHeader?oldHeader.value.latest:null,commandKind:kind,changes};
 const changeDigest=hash('orkworks.hierarchy-storage-change.v3',manifest);
 const link={storageEpoch:epoch,commitSequence:sequence,changeDigest};
 for(const intent of intents){
  const r=clone(intent);
  if(r.table==='claim_proofs')r.value.commit=link;
  else if(r.table!=='mutation_lookup')r.value.lastCommit=link;
  if(r.table==='summary_state'&&r.value.checkpoint&&r.value.checkpoint.applied==='THIS_COMMIT')r.value.checkpoint.applied=link;
  if(r.table==='receipt_slots'&&r.value.receipt)r.value.receipt.commit=link;
  assert.equal(digest(r),changes.find(c=>c.table===r.table&&canon(c.key)===canon(r.key)).afterDigest);
  newStore.set(rowId(r.table,r.key),r);
 }
 newStore.set(rowId('store_header',1),row('store_header',1,{schemaVersion:3,workspaceId:A.workspaceId,storageEpoch:epoch,latest:link}));
 newStore.set(rowId('commit_proof',sequence),row('commit_proof',sequence,{body:manifest,changeDigest}));
 const changed=sortedRows(newStore).filter(r=>!before.has(rowId(r.table,r.key))||canon(before.get(rowId(r.table,r.key)).value)!==canon(r.value));
 assert.equal(changed.length,intents.length+2);
 const commandBytes=changed.reduce((n,r)=>n+charge(r),0);
 assert(commandBytes<=16777216&&changed.length<=4096);
 const after=encodeState(name,newStore);
 commands.push({name,commandKind:kind,manifestCanonical:canon(manifest),changeDigest,changedRows:changed.length,commandBytes,beforeState:states.at(-1).name,afterState:name});
 store=newStore;states.push(after);return link;
}
const initialize=commit('initialized','initialize-store',[]);
const summaryValue=(m,sr,subject,applied)=>({subjectRevision:subject,row:{schemaVersion:1,assignmentIdentity:A,summaryRevision:sr,state:{kind:'current',material:m}},checkpoint:{material:m,summaryRevision:sr,applied},admission:'initialized',lastCommit:null});
const preparedSummary=summaryValue(M0,'1','1','THIS_COMMIT');
const usedSummary={ 'summary-row-bytes':bytes(preparedSummary.row),'summary-row-count':1,'summary-lineage-bytes':bytes(M0),'summary-pending-count':0};
const prep=[row('summary_state',A,preparedSummary),...capWitness.map(c=>row('quota_usage',c.key,{used:String(c.key.owner.kind==='workspace'&&c.key.resource in usedSummary?usedSummary[c.key.resource]:0),reserved:'0',lastCommit:null}))];
const prepared=commit('prepared','subject-preparation',prep);
const proof=clone(compact.compactProof),pd=hash('orkworks.hierarchy-claim-proof.v2',proof);
assert.equal(pd,compact.expected.proofDigest);
const proofRef={claimIdentity:proof.decision.claimIdentity,proofDigest:pd};
const keyDigest=hash('orkworks.assignment-report-key.v1',compact.retryKey);
assert.equal(keyDigest,compact.expected.retryKeyDigest);
const work={reservedRehashBytes:'0',consumedPasses:'0',state:'open',outcomeReceipt:null,lastCommit:null};
const candidateSlots=Array.from({length:10},(_,slotId)=>row('receipt_slots',{assignmentIdentity:A,slotId},slotId===0?{schemaVersion:3,state:'consumed',purpose:'candidate-admission',receipt:{kind:'mutation',protocol:'assignment-report-v1',claim:proofRef,responseBase64:Buffer.from(canon(seed.receipt)).toString('base64'),commit:null},lastCommit:null}:{schemaVersion:3,state:'reserved',purpose:slotId===1?'candidate-outcome':slotId%2===0?'user-evaluation':'user-invalidation',reservation:{kind:'candidate',candidateDigest:cd},lastCommit:null}));
const quota=(owner,resource,used,reserved=0)=>row('quota_usage',{owner,resource},{used:String(used),reserved:String(reserved),lastCommit:null});
const admission=commit('admitted','candidate-admission',[
 row('objects',C.key,{body:candidate,digest:cd,lastCommit:null}),row('candidate_work',C,work),...candidateSlots,
 row('claim_proofs',proof.decision.claimIdentity,{body:proof,proofDigest:pd,commit:null}),row('mutation_lookup',keyDigest,{schemaVersion:3,claim:proofRef,slotId:0}),
 ...owners.flatMap(o=>[quota(o,'reporting-receipt-bytes',16384,9*16384),quota(o,'assignment-record-bytes',bytes(candidate))]),
 row('summary_state',A,summaryValue(M0,'1','2',prepared))
]);
const finalResponse={schemaVersion:1,candidateDigest:cd,outcome:'accepted',result:R,reasonCode:null};
const finalSlots=Array.from({length:9},(_,j)=>{
 const slotId=j+1;return row('receipt_slots',{assignmentIdentity:A,slotId},slotId===1?{schemaVersion:3,state:'consumed',purpose:'candidate-outcome',receipt:{kind:'candidate-finalization',candidate:C,responseBase64:Buffer.from(canon(finalResponse)).toString('base64'),commit:null},lastCommit:null}:{schemaVersion:3,state:'reserved',purpose:slotId%2===0?'user-evaluation':'user-invalidation',reservation:{kind:'result',resultRevision:1,resultDigest:rd,stream:null},lastCommit:null});
});
const finalSummary=summaryValue(M1,'2','3','THIS_COMMIT');
const finalization=commit('accepted','result-finalization',[
 row('objects',resultRef.key,{body:result,digest:rd,lastCommit:null}),row('heads',{kind:'assignment-result',identity:A},{target:resultRef,lastCommit:null}),
 row('candidate_work',C,{...work,state:'accepted',outcomeReceipt:1}),...finalSlots,
 ...owners.flatMap(o=>[quota(o,'reporting-receipt-bytes',2*16384,8*16384),quota(o,'assignment-record-bytes',bytes(candidate)+bytes(result))]),
 row('summary_state',A,finalSummary),quota(W,'summary-row-bytes',bytes(finalSummary.row)),quota(W,'summary-lineage-bytes',bytes(M1))
]);
const exact=(value,fields)=>assert.deepEqual(Object.keys(value).sort(),[...fields].sort());
function validateSlot(r,map,stateName){
 exact(r.key,['assignmentIdentity','slotId']);assert.equal(canon(r.key.assignmentIdentity),canon(A));
 const i=r.key.slotId;assert(Number.isInteger(i)&&i>=0&&i<=1087&&i<10);
 const v=r.value;assert.equal(v.schemaVersion,3);
 assert.equal(v.purpose,i===0?'candidate-admission':i===1?'candidate-outcome':i%2===0?'user-evaluation':'user-invalidation');
 const consumed=i===0||(i===1&&stateName==='accepted');
 assert.equal(v.state,consumed?'consumed':'reserved');
 exact(v,consumed?['schemaVersion','state','purpose','receipt','lastCommit']:['schemaVersion','state','purpose','reservation','lastCommit']);
 if(!consumed){
  const expected=i===1||stateName!=='accepted'?{kind:'candidate',candidateDigest:cd}:{kind:'result',resultRevision:1,resultDigest:rd,stream:null};
  assert.equal(canon(v.reservation),canon(expected));
 }else{
  const receipt=v.receipt;
  exact(receipt,i===0?['kind','protocol','claim','responseBase64','commit']:['kind','candidate','responseBase64','commit']);
  const decoded=Buffer.from(receipt.responseBase64,'base64');
  assert.equal(decoded.toString('base64'),receipt.responseBase64);
  assert.equal(decoded.toString('utf8'),canon(i===0?seed.receipt:finalResponse));
  assert.equal(canon(receipt.commit),canon(v.lastCommit));
  if(i===0){
   assert.equal(receipt.kind,'mutation');assert.equal(receipt.protocol,'assignment-report-v1');
   assert.equal(canon(receipt.claim),canon(proofRef));
   const claim=map.get(rowId('claim_proofs',receipt.claim.claimIdentity));assert(claim);
   const lookup=map.get(rowId('mutation_lookup',keyDigest));assert(lookup);
   assert.equal(canon(claim.value.commit),canon(receipt.commit));
   assert.equal(claim.value.body.record.candidateDigest,cd);
   assert.equal(claim.value.body.decision.requestDigest,hash('orkworks.assignment-report-request.v1',seed.request));
   assert.equal(charge(r)+charge(claim)+charge(lookup)<=16384,true);
  }else{
   assert.equal(receipt.kind,'candidate-finalization');assert.equal(canon(receipt.candidate),canon(C));
   const workRow=map.get(rowId('candidate_work',C));assert(workRow);
   assert.equal(workRow.value.state,'accepted');assert.equal(workRow.value.outcomeReceipt,i);
   assert.equal(canon(workRow.value.lastCommit),canon(receipt.commit));
   assert.equal(map.get(rowId('objects',resultRef.key))?.value.body.candidateDigest,cd);
  }
 }
 assert(charge(r)<=16384);
}
function validateTransition(before,after,command){
 const decode=s=>new Map(s.rows.map(x=>{
  const r=row(x.table,JSON.parse(x.keyCanonical),JSON.parse(x.valueCanonical));return [rowId(r.table,r.key),r];
 }));
 const old=decode(before),next=decode(after),manifest=JSON.parse(command.manifestCanonical);
 assert.equal(command.manifestCanonical,canon(manifest));
 const changed=[...new Set([...old.keys(),...next.keys()])].filter(k=>!old.has(k)||!next.has(k)||canon(old.get(k).value)!==canon(next.get(k).value));
 const newProofId=rowId('commit_proof',manifest.commitSequence),headerId=rowId('store_header',1);
 assert(changed.includes(headerId)&&changed.includes(newProofId));
 const expected=changed.filter(k=>k!==headerId&&k!==newProofId).map(k=>{
  const a=old.get(k),b=next.get(k),r=b??a;return {table:r.table,key:r.key,beforeDigest:a?digest(a):null,afterDigest:b?digest(b):null};
 }).sort((a,b)=>Buffer.compare(Buffer.from(a.table),Buffer.from(b.table))||Buffer.compare(Buffer.from(canon(a.key)),Buffer.from(canon(b.key))));
 assert.equal(canon(manifest.changes),canon(expected),'manifest must equal complete before/after body diff');
 for(const c of expected)assert(c.beforeDigest!==c.afterDigest,'no link-only churn');
 assert.equal(command.changedRows,changed.length);
 const measured=changed.reduce((n,k)=>n+(next.has(k)?charge(next.get(k)):keyBytes(old.get(k))),0);
 assert.equal(command.commandBytes,measured);assert(measured<=16777216&&changed.length<=4096);
 const oldHeader=old.get(headerId),newHeader=next.get(headerId);
 assert.equal(manifest.commitSequence,oldHeader?String(BigInt(oldHeader.value.latest.commitSequence)+1n):'1');
 assert.equal(canon(manifest.predecessor),canon(oldHeader?.value.latest??null));
 assert.equal(manifest.schemaVersion,3);assert.equal(manifest.workspaceId,A.workspaceId);assert.equal(manifest.storageEpoch,epoch);
 assert.equal(command.changeDigest,hash('orkworks.hierarchy-storage-change.v3',manifest));
 assert.equal(canon(next.get(newProofId).value),canon({body:manifest,changeDigest:command.changeDigest}));
 assert.equal(canon(newHeader.value.latest),canon({storageEpoch:epoch,commitSequence:manifest.commitSequence,changeDigest:command.changeDigest}));
}
function validateState(s){
 const rows=s.rows.map(r=>row(r.table,JSON.parse(r.keyCanonical),JSON.parse(r.valueCanonical)));
 const map=new Map(rows.map(r=>[rowId(r.table,r.key),r]));assert.equal(map.size,rows.length);
 if(!rows.length)return;
 const header=map.get(rowId('store_header',1)).value;
 const proofFor=l=>{
  assert(l&&l.storageEpoch===epoch);
  const p=map.get(rowId('commit_proof',l.commitSequence));assert(p);
  assert.equal(p.value.changeDigest,l.changeDigest);
  assert.equal(hash('orkworks.hierarchy-storage-change.v3',p.value.body),l.changeDigest);
  assert(BigInt(l.commitSequence)<=BigInt(header.latest.commitSequence));return p.value.body;
 };
 proofFor(header.latest);
 for(const r of rows){
  const encoded=s.rows.find(x=>x.table===r.table&&x.keyCanonical===canon(r.key));
  assert.equal(encoded.keyCanonical,canon(r.key));assert.equal(encoded.valueCanonical,canon(r.value));assert.equal(encoded.rowDigest,digest(r));
  const link=r.value.lastCommit??r.value.commit;
  if(r.table==='commit_proof'){assert.equal(r.key,r.value.body.commitSequence);assert.equal(hash('orkworks.hierarchy-storage-change.v3',r.value.body),r.value.changeDigest);}
  if(r.table==='objects'){
   const expected=r.key.kind==='assignment-result-candidate'?candidate:result;
   assert.equal(canon(r.value.body),canon(expected));
   assert.equal(r.value.digest,r.key.kind==='assignment-result-candidate'?cd:rd);
  }
  if(link){
   const m=proofFor(link);const change=m.changes.find(c=>c.table===r.table&&canon(c.key)===canon(r.key));assert(change);assert.equal(change.afterDigest,digest(r));
  }
 }
 if(s.name!=='prepared'){
  assert.equal(map.get(rowId('objects',C.key))?.value.digest,cd);
  const workRow=map.get(rowId('candidate_work',C));assert(workRow);
  assert.equal(workRow.value.state,s.name==='accepted'?'accepted':'open');
  assert.equal(workRow.value.reservedRehashBytes,'0');assert.equal(workRow.value.consumedPasses,'0');
  for(let slotId=0;slotId<10;slotId++){
   const slot=map.get(rowId('receipt_slots',{assignmentIdentity:A,slotId}));assert(slot);validateSlot(slot,map,s.name);
   const expectedConsumed=slotId===0||(slotId===1&&s.name==='accepted');
   assert.equal(slot.value.state,expectedConsumed?'consumed':'reserved');
   if(slotId===0)assert.equal(Buffer.from(slot.value.receipt.responseBase64,'base64').toString(),canon(seed.receipt));
   if(slotId===1&&s.name==='accepted')assert.equal(Buffer.from(slot.value.receipt.responseBase64,'base64').toString(),canon(finalResponse));
  }
  const proofRow=map.get(rowId('claim_proofs',proof.decision.claimIdentity));assert(proofRow);
  assert.equal(canon(proofRow.value.body),canon(proof));assert.equal(proofRow.value.proofDigest,pd);
  assert.equal(hash('orkworks.hierarchy-claim-proof.v2',proofRow.value.body),pd);
  const lookup=map.get(rowId('mutation_lookup',keyDigest));assert(lookup);
  assert.equal(canon(lookup.value),canon({schemaVersion:3,claim:proofRef,slotId:0}));
 }
 if(s.name==='accepted'){
  assert.equal(map.get(rowId('objects',resultRef.key))?.value.digest,rd);
  assert.equal(canon(map.get(rowId('heads',{kind:'assignment-result',identity:A}))?.value.target),canon(resultRef));
 }else assert(!map.has(rowId('heads',{kind:'assignment-result',identity:A})));
 const qget=(owner,resource)=>{const r=map.get(rowId('quota_usage',{owner,resource}));assert(r);return r.value;};
 const slots=rows.filter(r=>r.table==='receipt_slots');
 const consumed=slots.filter(r=>r.value.state==='consumed').length;
 const recordBytes=rows.filter(r=>r.table==='objects').reduce((n,r)=>n+bytes(r.value.body),0);
 for(const o of owners){
  assert.equal(qget(o,'reporting-receipt-bytes').used,String(consumed*16384));
  assert.equal(qget(o,'reporting-receipt-bytes').reserved,String((slots.length-consumed)*16384));
  assert.equal(qget(o,'assignment-record-bytes').used,String(recordBytes));
  assert.equal(qget(o,'result-payload-bytes').used,'0');
 }
 const ss=map.get(rowId('summary_state',A));
 if(ss){
  proofFor(ss.value.checkpoint.applied);
  assert.equal(qget(W,'summary-row-bytes').used,String(bytes(ss.value.row)));
  assert.equal(qget(W,'summary-lineage-bytes').used,String(bytes(ss.value.checkpoint.material)));
  assert.equal(qget(W,'summary-row-count').used,'1');assert.equal(qget(W,'summary-pending-count').used,'0');
  assert.equal(canon(ss.value.row.state.material),canon(ss.value.checkpoint.material));
 }
 for(const c of capWitness){
  const q=map.get(rowId('quota_usage',c.key));if(!q)continue;
  assert(BigInt(q.value.used)>=0n&&BigInt(q.value.reserved)>=0n);
  assert(BigInt(q.value.used)+BigInt(q.value.reserved)<=BigInt(c.maximumTotal));
 }
 assert(!rows.some(r=>['payloads','dependencies','pending_material'].includes(r.table)));
}
for(let i=0;i<commands.length;i++)validateTransition(states[i],states[i+1],commands[i]);
// Initialization legitimately predates explicit subject/ledger preparation.
for(const s of states.filter(s=>['prepared','admitted','accepted'].includes(s.name)))validateState(s);
const accepted=states.at(-1);
const mutate=(state,table,fn)=>{const s=clone(state),r=s.rows.find(r=>r.table===table);fn(r,s);return s;};
const pruned=mutate(accepted,'commit_proof',(_,s)=>{const i=s.rows.findIndex(r=>r.table==='commit_proof'&&r.keyCanonical===canon(initialize.commitSequence));assert(i>=0);s.rows.splice(i,1);});
validateState(pruned); // No retained row/latest link requires the old initialization proof.
const negativeChecks=[];
for(const [name,changed] of [
 ['wrong-outer-link',mutate(accepted,'candidate_work',r=>{const v=JSON.parse(r.valueCanonical);v.lastCommit.changeDigest='0'.repeat(64);r.valueCanonical=canon(v);})],
 ['wrong-source-byte',mutate(accepted,'objects',r=>{const v=JSON.parse(r.valueCanonical);v.body.outputs=[{}];r.valueCanonical=canon(v);})],
 ['missing-ledger',mutate(accepted,'quota_usage',(r,s)=>s.rows.splice(s.rows.indexOf(r),1))],
 ['wrong-ledger',mutate(accepted,'quota_usage',r=>{const v=JSON.parse(r.valueCanonical);v.used='1';r.valueCanonical=canon(v);})],
 ['missing-required-commit-proof',mutate(accepted,'commit_proof',(_,s)=>{const i=s.rows.findIndex(r=>r.table==='commit_proof'&&r.keyCanonical===canon(finalization.commitSequence));assert(i>=0);s.rows.splice(i,1);})],
 ['duplicate-row',mutate(accepted,'summary_state',(r,s)=>s.rows.push(clone(r)))],
 ['missing-head',mutate(accepted,'heads',(r,s)=>s.rows.splice(s.rows.indexOf(r),1))],
 ['missing-slot',mutate(accepted,'receipt_slots',(r,s)=>s.rows.splice(s.rows.indexOf(r),1))]
]){assert.throws(()=>validateState(changed),name);negativeChecks.push(name);}
for(const mode of ['missing-change','extra-change','wrong-before']){
 const c=clone(commands[2]),m=JSON.parse(c.manifestCanonical);
 if(mode==='missing-change')m.changes.pop();
 if(mode==='extra-change')m.changes.push({table:'unregistered',key:'extra',beforeDigest:null,afterDigest:'0'.repeat(64)});
 if(mode==='wrong-before')m.changes[0].beforeDigest='0'.repeat(64);
 c.manifestCanonical=canon(m);c.changeDigest=hash('orkworks.hierarchy-storage-change.v3',m);
 assert.throws(()=>validateTransition(states[2],states[3],c));negativeChecks.push(mode);
}
const acceptedMap=new Map(accepted.rows.map(x=>{const r=row(x.table,JSON.parse(x.keyCanonical),JSON.parse(x.valueCanonical));return [rowId(r.table,r.key),r];}));
for(const mode of ['wrong-purpose','wrong-reservation','wrong-claim','wrong-candidate','wrong-slot-bound']){
 const id=mode==='wrong-claim'?0:mode==='wrong-candidate'?1:2;
 const r=clone(acceptedMap.get(rowId('receipt_slots',{assignmentIdentity:A,slotId:id})));
 if(mode==='wrong-purpose')r.value.purpose='child-evaluation';
 if(mode==='wrong-reservation')r.value.reservation.stream={kind:'user',userReviewId:'unissued'};
 if(mode==='wrong-claim')r.value.receipt.claim.proofDigest='0'.repeat(64);
 if(mode==='wrong-candidate')r.value.receipt.candidate.digest='0'.repeat(64);
 if(mode==='wrong-slot-bound')r.key.slotId=1088;
 assert.throws(()=>validateSlot(r,acceptedMap,'accepted'));negativeChecks.push(mode);
}
const catalog={fixtureSchemaVersion:1,evidenceKind:'synthetic-logical-store-bytes-not-runtime-eligibility',inspectionBase:'a10f11f1',seedCatalog:'hierarchy-claim-proof-vectors.json',seedSha256:sha(seedBytes),compactCatalog:'hierarchy-compact-claim-vectors.json',compactSha256:sha(compactBytes),profile:{assignmentIdentity:A,outputs:[],criteria,rubric,criteriaSnapshotDigest:M0.criteriaSnapshotDigest,rubricSnapshotDigest:M0.rubricSnapshotDigest,ownerCapStatus:'fixture-only witness ceilings; no production aggregate defaults',ledgerCapWitness:capWitness},sources:{candidateCanonical:canon(candidate),candidateDigest:cd,resultCanonical:canon(result),resultDigest:rd,requestCanonical:canon(seed.request),admissionReceiptCanonical:canon(seed.receipt),finalizationReceiptCanonical:canon(finalResponse)},states,commands,negativeChecks,measurements:{candidateBytes:bytes(candidate),resultBytes:bytes(result),initialMaterialBytes:bytes(M0),acceptedMaterialBytes:bytes(M1),admissionRows:commands[2].changedRows,admissionBytes:commands[2].commandBytes,finalizationRows:commands[3].changedRows,finalizationBytes:commands[3].commandBytes,receiptPoolBytes:10*16384}};
const output=JSON.stringify(catalog,null,2)+'\n';assert(Buffer.byteLength(output)<1048576);
const target=new URL('hierarchy-empty-output-profile.json',dir);
if(process.argv.includes('--write'))writeFileSync(target,output);
else assert.equal(readFileSync(target,'utf8'),output,'golden logical bytes differ');
console.log(JSON.stringify({status:'verified',...catalog.measurements,negativeChecks:negativeChecks.length,catalogBytes:Buffer.byteLength(output),scope:catalog.evidenceKind}));
