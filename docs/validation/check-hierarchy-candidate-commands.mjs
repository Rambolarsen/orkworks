// Symbolic design evidence only; no production parser, store or native probe.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const data=JSON.parse(readFileSync(new URL('./hierarchy-candidate-command-vectors.json',import.meta.url),'utf8'));
assert.equal(data.schemaVersion,1);
assert.equal(data.evidenceKind,'symbolic-effect-accounting-not-admitted-history');
assert.equal(data.slotBytes,16384);
assert.equal(data.commandByteCap,16*1024*1024);
assert.equal(data.commandRowCap,4096);
const names=new Set();
for(const c of data.cases){
 assert(!names.has(c.name)); names.add(c.name);
 for(const k of ['coreRows','quotaRows','summaryRows','payloadRows','controlRows','expectedRows'])assert(Number.isSafeInteger(c[k])&&c[k]>=0);
 assert.equal(c.coreRows+c.quotaRows+c.summaryRows+c.payloadRows+c.controlRows,c.expectedRows,c.name);
 assert.equal(c.controlRows,2);
 const delta=c.receiptDeltaSlots.used+c.receiptDeltaSlots.reserved;
 if(c.name.startsWith('admission'))assert.equal(delta,10);
 else if(c.name.startsWith('rejection')||c.name.startsWith('abandonment'))assert.equal(delta,-8);
 else assert.equal(delta,0);
}
assert.equal(data.cases.length,7);
assert.equal(data.cases[0].expectedRows,16+6+1);
assert.equal(data.cases[2].expectedRows,12+3+1);
assert.equal(data.cases[3].expectedRows,11+3+1);
// Keys are explicitly symbolic, not serialized source/store identities.
const row=(table,key)=>({table,key});
const encoded=r=>JSON.stringify([r.table,r.key]);
const union=(...sets)=>new Set(sets.flatMap(s=>[...s]));
const slots=(from,to)=>Array.from({length:to-from},(_,i)=>row('receipt_slots',`A:${from+i}`));
const scopes=['assignment:A','run:R','workspace:W'];
const q=(...resources)=>resources.flatMap(r=>scopes.map(owner=>row('quota_usage',`${owner}:${r}`)));
for(const c of data.cases){
 let core,quota,summary=[];
 if(c.name==='admission-detail-only'){
  core=[...slots(0,10),row('objects','candidate:C'),row('candidate_work','C'),row('claim_proofs','claim'),row('mutation_lookup','retry')];
  quota=q('reporting-receipt-bytes','assignment-record-bytes');
 }else if(c.name==='positive-work-reservation'){
  core=[row('candidate_work','C')];quota=[];
 }else if(c.name==='rejection-detail-only'){
  core=[...slots(1,10),row('candidate_work','C')];quota=q('reporting-receipt-bytes');
 }else if(c.name==='abandonment-detail-only'){
  core=[...slots(2,10),row('candidate_work','C')];quota=q('reporting-receipt-bytes');
 }else{
  assert(['accept-empty-current-summary','accept-two-current-summary','accept-two-pending-summary'].includes(c.name));
  core=[...slots(1,10),row('objects','result:R'),row('candidate_work','C'),row('heads','result:A')];
  quota=q('reporting-receipt-bytes','assignment-record-bytes',...(c.payloadRows?['result-payload-bytes']:[]));
  quota.push(row('quota_usage','workspace:W:summary-row-bytes'),row('quota_usage','workspace:W:summary-lineage-bytes'));
 }
 if(c.summaryRows)summary.push(row('summary_state','A'));
 if(c.name==='accept-two-pending-summary'){
  summary.push(row('pending_material','A:next-summary-revision'));
  quota.push(row('quota_usage','workspace:W:summary-pending-count'));
 }
 const expected={core,quota,summary,payload:Array.from({length:c.payloadRows},(_,i)=>row('payloads',`A:R:output-${i}`)),control:[row('store_header','1'),row('commit_proof','next')]};
 assert.deepEqual(Object.keys(c.effects).sort(),Object.keys(expected).sort());
 for(const [kind,rows] of Object.entries(expected)){
  assert.deepEqual(c.effects[kind].map(encoded).sort(),rows.map(encoded).sort(),`${c.name}:${kind}`);
  const countField={core:'coreRows',quota:'quotaRows',summary:'summaryRows',payload:'payloadRows',control:'controlRows'}[kind];
  assert.equal(rows.length,c[countField]);
 }
 const all=Object.values(c.effects).flat().map(encoded);
 const actual=new Set(all);
 assert.equal(actual.size,all.length,'duplicate changed key');
 assert.equal(actual.size,c.expectedRows,c.name);
 assert.equal(union(actual,new Set(c.effects.quota.map(encoded))).size,c.expectedRows);
 const validate=rows=>{
  assert.equal(new Set(rows).size,rows.length);
  assert.deepEqual([...rows].sort(),[...actual].sort());
 };
 validate(all);
 for(let i=0;i<all.length;i++)assert.throws(()=>validate(all.filter((_,j)=>j!==i)));
 assert.throws(()=>validate([...all,encoded(row('unregistered','extra'))]));
 assert.throws(()=>validate([...all,all[0]]));
}
const fits=(bytes,rows)=>bytes<=data.commandByteCap&&rows<=data.commandRowCap;
assert(fits(data.commandByteCap,4096));
assert(!fits(data.commandByteCap+1,1));
assert(!fits(1,4097));
assert(!fits(data.boundaries.two8MiBOutputs+1,1));
assert.equal(10*data.slotBytes,data.boundaries.admissionSlotAllowance);
assert.equal(9*data.slotBytes,data.boundaries.acceptanceSlotAllowance);
assert.equal(data.boundaries.sourceRecordCap,512*1024);
// Checked integer accounting never wraps or creates refunds from missing state.
const u64max=(1n<<64n)-1n;
const add=(before,delta)=>{
 assert(typeof before==='bigint'&&typeof delta==='bigint');
 const after=before+delta;assert(after>=0n&&after<=u64max);return after;
};
assert.equal(add(9n*16384n,-9n*16384n),0n);
assert.throws(()=>add(0n,-1n));assert.throws(()=>add(u64max,1n));
console.log('Verified all 7 symbolic table/key effect sets against command recipes, per-key missing/extra/duplicate denials, receipt conservation, coalescing, cap and u64 boundaries; no full-byte/native qualification.');
