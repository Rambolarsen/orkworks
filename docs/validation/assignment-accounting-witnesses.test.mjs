// Regressions catch head-only recounts, wrong target owners and metadata exemptions.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {measureArchivedRecord,measureEmptyOutputState,measureEmptyOutputTransition} from './assignment-accounting-witnesses.mjs';
const read=name=>readFileSync(new URL(name,import.meta.url));
const store=JSON.parse(read('hierarchy-empty-output-profile.json'));
const claims=JSON.parse(read('hierarchy-claim-proof-vectors.json'));
const packet=JSON.parse(read('assignment-accounting-witnesses.json'));
const state=name=>store.states.find(s=>s.name===name);
const source=id=>claims.vectors.find(v=>v.id===id);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');

test('original full witness checker still validates exact golden states and complete transitions',()=>{
  const output=execFileSync(process.execPath,[new URL('./check-hierarchy-empty-output-profile.mjs',import.meta.url).pathname],{encoding:'utf8'});
  assert.equal(JSON.parse(output).status,'verified');
});
for(const [name,digest] of Object.entries(packet.inputs))test(`${name}: exact source catalog fingerprint`,()=>assert.equal(sha(read(name)),digest));
for(const expected of packet.states)test(`${expected.name}: independent retained-body and ledger recount`,()=>{
  assert.deepEqual(measureEmptyOutputState(state(expected.name),store),expected);
});
for(const expected of packet.commands)test(`${expected.name}: independently counted complete changed rows and bytes`,()=>{
  const command=store.commands.find(c=>c.name===expected.name);
  assert.deepEqual(measureEmptyOutputTransition(state(command.beforeState),state(command.afterState),command,store),expected);
});
for(const expected of packet.records)test(`${expected.id}: source bytes and evaluated/producing target owners`,()=>{
  assert.deepEqual(measureArchivedRecord(source(expected.id)),expected);
});
test('accepted result retains candidate charge; headers and receipt metadata are additional command bytes',()=>{
  const admitted=measureEmptyOutputState(state('admitted'),store),accepted=measureEmptyOutputState(state('accepted'),store);
  assert.equal(admitted.recordBytes,'618');assert.equal(accepted.recordBytes,'1362');assert.equal(accepted.objectCount,2);
  const command=store.commands.find(c=>c.name==='accepted');
  const change=measureEmptyOutputTransition(state('admitted'),state('accepted'),command,store);
  assert.equal(change.recordDelta,'744');assert.equal(change.commandBytes,30151);assert.equal(change.changedRows,23);
  assert(change.charges.some(r=>r.table==='store_header'));assert(change.charges.some(r=>r.table==='commit_proof'));
});
test('cross-run reviewer source bytes charge target run and never reviewer run',()=>{
  const vector=source('child-evaluation-correction'),charge=measureArchivedRecord(vector);
  assert.notEqual(vector.source.assignmentIdentity.runId,vector.source.reviewerIdentity.assignmentIdentity.runId);
  assert.equal(charge.owners[1].runId,vector.source.assignmentIdentity.runId);
  assert.notEqual(charge.owners[1].runId,vector.source.reviewerIdentity.assignmentIdentity.runId);
});
test('archive counterexample: standalone correction cannot certify a valid initial-to-correction history',()=>{
  assert.equal(source('child-evaluation-correction').source.predecessor.evaluationDigest,'c'.repeat(64));
  assert.notEqual(source('child-evaluation-correction').source.predecessor.evaluationDigest,source('child-evaluation-initial').expected.sourceDigest);
});
for(const [name,mutate] of [
 ['missing source ledger',s=>s.rows.splice(s.rows.findIndex(r=>r.table==='quota_usage'&&JSON.parse(r.keyCanonical).resource==='assignment-record-bytes'),1)],
 ['duplicate source row',s=>s.rows.push(structuredClone(s.rows.find(r=>r.table==='objects')))],
 ['noncanonical row key',s=>s.rows.find(r=>r.table==='objects').keyCanonical+=' '],
 ['noncanonical row value',s=>s.rows.find(r=>r.table==='objects').valueCanonical+=' '],
 ['unknown payload row',s=>s.rows.push({table:'payloads',keyCanonical:'{}',valueCanonical:'{}',rowDigest:'0'.repeat(64)})],
 ['unsupported object kind',s=>{const r=s.rows.find(r=>r.table==='objects');const k=JSON.parse(r.keyCanonical);k.kind='research-report';r.keyCanonical=JSON.stringify(k);}],
 ['changed source body',s=>{const r=s.rows.find(r=>r.table==='objects');const v=JSON.parse(r.valueCanonical);v.body.outputs=[{}];r.valueCanonical=JSON.stringify(v);}],
 ['wrong retained usage',s=>{const r=s.rows.find(r=>r.table==='quota_usage'&&JSON.parse(r.keyCanonical).resource==='assignment-record-bytes');const v=JSON.parse(r.valueCanonical);v.used='744';r.valueCanonical=JSON.stringify(v);}],
])test(`accounting inspection rejects ${name}`,()=>{
  const s=structuredClone(state('accepted'));mutate(s);assert.throws(()=>measureEmptyOutputState(s,store));
});
test('archived source hash corruption cannot redirect ownership',()=>{
  const v=structuredClone(source('child-evaluation-correction'));v.source.assignmentIdentity.runId='other-run';assert.throws(()=>measureArchivedRecord(v));
});
test('command measurement rejects a missing mandatory changed row',()=>{
  const s=structuredClone(state('accepted'));s.rows=s.rows.filter(r=>r.table!=='heads');
  assert.throws(()=>measureEmptyOutputTransition(state('admitted'),s,store.commands.at(-1),store));
});

// Repair local row commitments so these probe ledger semantics, not stale hashes.
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
function repairLedger(row,key,value){
  row.keyCanonical=canonical(key);row.valueCanonical=canonical(value);
  const body=structuredClone(value);delete body.lastCommit;
  row.rowDigest=sha('orkworks.hierarchy-storage-row.v3\n'+canonical({table:row.table,key,body}));
}
for(const mode of ['head-only-used','wrong-target-owner','reserved-source-credit'])test(`repaired row commitment still rejects ${mode}`,()=>{
  const s=structuredClone(state('accepted'));
  const row=s.rows.find(r=>r.table==='quota_usage'&&JSON.parse(r.keyCanonical).resource==='assignment-record-bytes');
  const key=JSON.parse(row.keyCanonical),value=JSON.parse(row.valueCanonical);
  if(mode==='head-only-used')value.used='744';
  if(mode==='wrong-target-owner')key.owner.assignmentIdentity.runId='different-run';
  if(mode==='reserved-source-credit')value.reserved='1';
  repairLedger(row,key,value);
  s.rows.sort((a,b)=>Buffer.compare(Buffer.from(a.table+'\n'+a.keyCanonical),Buffer.from(b.table+'\n'+b.keyCanonical)));
  assert.throws(()=>measureEmptyOutputState(s,store));
});
