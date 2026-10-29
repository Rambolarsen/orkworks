// Offline component accounting; no runtime admission or native qualification.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {validateArtifactOccurrences} from './root-preparation-effects.mjs';
import {compareBudgetTotals,measureBudgetState} from './root-preparation-budgets.mjs';
const read=n=>readFileSync(new URL(n,import.meta.url));
const effects=JSON.parse(read('root-preparation-effects.json'));
const zero={runPreparationBytes:'0',workspacePreparationBytes:'0',workspaceRunCount:'0',proofBytes:'0',proofCount:'0',logicalBytes:'0',logicalRows:'0'};
for(const [name,cap] of Object.entries({runPreparationBytes:8388608,workspacePreparationBytes:134217728,workspaceRunCount:32,proofBytes:67108864,proofCount:4096,logicalBytes:536870912,logicalRows:8192})){
  test(`${name}: inclusive fit and one-unit overflow`,()=>{
    assert.equal(compareBudgetTotals({...zero,[name]:String(cap)}).fits,true);
    assert.deepEqual(compareBudgetTotals({...zero,[name]:String(cap+1)}),{classification:'budget-arithmetic-only',fits:false,exceeded:[name]});
  });
}
for(const value of ['01','-1','1.0','9223372036854775808',1,null])test(`reject noncanonical or out-of-range usage ${JSON.stringify(value)}`,()=>{
  assert.throws(()=>compareBudgetTotals({...zero,proofCount:value}));
});
test('unknown or missing usage fields cannot hide a budget',()=>{
  assert.throws(()=>compareBudgetTotals({...zero,extra:'0'}));
  const bad={...zero};delete bad.proofBytes;assert.throws(()=>compareBudgetTotals(bad));
});
test('full inventory counts initialized control rows and all nine zero ledgers',()=>{
  const s=effects.states.find(s=>s.name==='initialized');
  const result=measureBudgetState(s,effects.contexts,effects.caps.fitting);
  assert.equal(result.usage.logicalRows,'11');assert.equal(result.usage.proofCount,'1');
  assert.equal(result.usage.workspacePreparationBytes,'0');assert.equal(result.usage.workspaceRunCount,'0');
});
test('measured inventory cannot accept omitted required proof',()=>{
  const s=structuredClone(effects.states.find(s=>s.name==='target-admitted'));
  s.rows.splice(s.rows.findIndex(r=>r.table==='commit_proof'),1);
  assert.throws(()=>measureBudgetState(s,effects.contexts,effects.caps.fitting));
});

const catalog=JSON.parse(read('root-preparation-budgets.json'));
const select=name=>effects.states.find(s=>s.name===name);
const measurement=name=>catalog.states.find(s=>s.name===name);
for(const expected of catalog.states)test(`${expected.name}: independent Python full inventory totals`,()=>{
  const caps=expected.name==='target-never-initialized'?effects.caps['summary-full']:effects.caps.fitting;
  const actual=measureBudgetState(select(expected.name),effects.contexts,caps);
  assert.deepEqual(actual.usage,expected.usage);assert.deepEqual(actual.runs,expected.runs);
  assert.equal(actual.comparison.fits,true);
});
for(const vector of catalog.arithmetic)test(`isolated arithmetic ${vector.name}`,()=>{
  assert.deepEqual(compareBudgetTotals(vector.usage),{classification:'budget-arithmetic-only',fits:vector.exceeded.length===0,exceeded:vector.exceeded});
});
test('cancel releases target occurrences while retained proof history grows',()=>{
  const before=measurement('target-draft').usage,after=measurement('target-cancelled').usage;
  assert.equal(BigInt(before.workspaceRunCount)-BigInt(after.workspaceRunCount),1n);
  assert(BigInt(after.workspacePreparationBytes)<BigInt(before.workspacePreparationBytes));
  assert.equal(BigInt(after.proofCount)-BigInt(before.proofCount),1n);
  assert(BigInt(after.proofBytes)>BigInt(before.proofBytes));
});
test('admission keeps count while source and membership add retained charge',()=>{
  const a=measurement('target-draft').usage,b=measurement('target-admitted').usage;
  assert.equal(a.workspaceRunCount,b.workspaceRunCount);
  assert(BigInt(b.workspacePreparationBytes)>BigInt(a.workspacePreparationBytes));
});
test('both summary branches count full rows and checkpoint/link bytes',()=>{
  const a=measurement('target-admitted').usage,b=measurement('target-never-initialized').usage;
  assert.equal(a.workspacePreparationBytes,b.workspacePreparationBytes);
  assert(BigInt(a.logicalBytes)>BigInt(b.logicalBytes));assert.equal(a.logicalRows,b.logicalRows); // same marker/ledger keys; three changed ledgers are replacements
  assert(BigInt(b.logicalBytes)>BigInt(b.workspacePreparationBytes)+BigInt(b.proofBytes));
});
for(const [name,mutate] of [
  ['missing ledger',s=>s.rows.splice(s.rows.findIndex(r=>r.table==='quota_usage'),1)],
  ['duplicate key',s=>s.rows.push(structuredClone(s.rows[0]))],
  ['unknown table',s=>s.rows.push({table:'unknown',keyCanonical:'1',valueCanonical:'{}',rowDigest:'0'.repeat(64)})],
  ['unproven byte drift',s=>s.rows[0].valueCanonical+=' '],
])test(`inventory inspection rejects ${name} before summing`,()=>{
  const s=structuredClone(select('target-admitted'));mutate(s);
  assert.throws(()=>measureBudgetState(s,effects.contexts,effects.caps.fitting));
});

const sha=x=>createHash('sha256').update(x).digest('hex');
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const size=x=>Buffer.byteLength(canonical(x));
test('budget measurements are tied to exact earlier catalog bytes',()=>{
  assert.equal(sha(read('root-preparation-effects.json')),catalog.effectsCatalogSha256);
});
test('sixteen separately encoded occurrences reach the raw aggregate without a sharing discount',()=>{
  const refs=[],rows=[],owner={workspaceId:'w'.repeat(128),runId:'e'.repeat(64)+'-'+'9'.repeat(19)};
  const link={storageEpoch:'e'.repeat(64),commitSequence:'9223372036854775807',changeDigest:'d'.repeat(64)};
  let encoded=0,full=0,projected=0;
  for(const [i,n] of catalog.envelopes.rawLengths.entries()){
    const raw=Buffer.alloc(n),id='r'.repeat(125)+String(i).padStart(3,'0');
    const contentDigest=sha(raw),key={...owner,referenceId:id};
    refs.push({id,kind:'fixture',sourceReference:'fixture-only',contentDigest});
    const value={schemaVersion:1,contentBase64:raw.toString('base64'),sizeBytes:String(n),contentDigest,lastCommit:link};
    rows.push({table:'preparation_artifacts',key,value});encoded+=value.contentBase64.length;full+=size(key)+size(value);
    const body=structuredClone(value);delete body.lastCommit;projected+=size(key)+size(body);
  }
  assert.equal(validateArtifactOccurrences(rows,refs,owner),1048576);
  assert.equal(encoded,1398144);assert.equal(encoded,catalog.envelopes.encodedArtifactBytes);
  assert(full>projected);assert.equal(refs[0].contentDigest,refs[1].contentDigest);
  // Upper bound deliberately uses the seven-digit size field for every wrapper.
  assert(projected<=encoded+16*(catalog.envelopes.artifactKeyBytes+catalog.envelopes.artifactProjectedOverheadBytes));
  const bad=structuredClone(rows);bad[0].value.contentBase64=Buffer.alloc(2).toString('base64');
  bad[0].value.sizeBytes='2';bad[0].value.contentDigest=sha(Buffer.alloc(2));const badRefs=structuredClone(refs);badRefs[0].contentDigest=bad[0].value.contentDigest;
  assert.throws(()=>validateArtifactOccurrences(bad,badRefs,owner),/raw artifact aggregate ceiling/);
});
test('conservative preparation envelope bounds fit the per-run cap without claiming source validity',()=>{
  const e=catalog.envelopes;
  assert.equal(e.classification,'conservative-envelope-bounds-not-source-valid');
  assert.equal(e.pendingPreparationUpperBytes,e.workspaceKeyBytes+e.draftWrapperBytes+1081344+e.encodedArtifactBytes+16*(e.artifactKeyBytes+e.artifactProjectedOverheadBytes));
  assert.equal(e.admittedPreparationUpperBytes,2*e.sourceKeyBytes+1081344+8192+e.parentKeyBytes+e.encodedArtifactBytes+16*(e.artifactKeyBytes+e.artifactProjectedOverheadBytes));
  assert(e.pendingPreparationUpperBytes<=8388608);assert(e.admittedPreparationUpperBytes<=8388608);
  assert(1398104+e.artifactFullOverheadBytes<=2097152);
});
test('maximum structural manifest bytes do not override the complete changed-row limit',()=>{
  const identity={kind:'root',workspaceId:'w'.repeat(128),runId:'e'.repeat(64)+'-'+'9'.repeat(19),rootAssignmentId:'a'.repeat(128),parentSessionId:'p'.repeat(128),bootstrapId:'b'.repeat(128),bootstrapConfigurationDigest:'d'.repeat(64)};
  const link={storageEpoch:'e'.repeat(64),commitSequence:'9'.repeat(19),changeDigest:'d'.repeat(64)};
  const change={table:'quota_usage',key:{owner:{kind:'assignment',assignmentIdentity:identity},resource:'reporting-receipt-bytes'},beforeDigest:'a'.repeat(64),afterDigest:'b'.repeat(64)};
  const value={body:{schemaVersion:3,workspaceId:identity.workspaceId,storageEpoch:'e'.repeat(64),commitSequence:'9'.repeat(19),predecessor:link,commandKind:'subject-preparation',changes:Array(4096).fill(change)},changeDigest:'d'.repeat(64)};
  assert.equal(size(value),catalog.envelopes.proofValueBytes);assert.equal(sha(canonical(value)),catalog.envelopes.proofValueSha256);
  assert(size(value)+catalog.envelopes.proofKeyBytes<=67108864);
  assert.equal(catalog.envelopes.minimumChangedRowsWithControls,4098);
  assert(catalog.envelopes.minimumChangedRowsWithControls>4096);
  // Duplicate maximal entries are a byte upper bound, never a valid manifest.
});

test('conservative proof bound covers every registered initial-profile key family',()=>{
  const identity={kind:'root',workspaceId:'w'.repeat(128),runId:'e'.repeat(64)+'-'+'9'.repeat(19),rootAssignmentId:'a'.repeat(128),parentSessionId:'p'.repeat(128),bootstrapId:'b'.repeat(128),bootstrapConfigurationDigest:'d'.repeat(64)};
  const sourceKey={workspaceId:identity.workspaceId,runId:identity.runId};
  const rows=select('target-admitted').rows.filter(r=>!['commit_proof','store_header'].includes(r.table));
  const candidates=rows.map(r=>{
    let key;
    if(r.table==='quota_usage'){
      const original=JSON.parse(r.keyCanonical),kind=original.owner.kind;
      const owner=kind==='assignment'?{kind,assignmentIdentity:identity}:kind==='run'?{kind,...sourceKey}:{kind,workspaceId:identity.workspaceId};
      key={owner,resource:original.resource};
    }else if(r.table==='summary_state')key=identity;
    else if(r.table==='root_drafts')key=identity.workspaceId;
    else if(r.table==='parent_attachments')key=identity.parentSessionId;
    else if(r.table==='preparation_artifacts')key={...sourceKey,referenceId:'r'.repeat(128)};
    else key=sourceKey;
    return {table:r.table,key,beforeDigest:'a'.repeat(64),afterDigest:'b'.repeat(64)};
  });
  // Include the pending-only draft key as well as admitted-state keys.
  candidates.push({table:'root_drafts',key:identity.workspaceId,beforeDigest:'a'.repeat(64),afterDigest:'b'.repeat(64)});
  const maximum=Math.max(...candidates.map(size));
  assert.equal(catalog.envelopes.largestChangeBytes,maximum);
  assert(candidates.some(c=>c.table===catalog.envelopes.largestChangeTable&&canonical(c.key)===catalog.envelopes.largestChangeKeyCanonical&&size(c)===maximum));
  const link={storageEpoch:'e'.repeat(64),commitSequence:'9'.repeat(19),changeDigest:'d'.repeat(64)};
  const fixed={body:{schemaVersion:3,workspaceId:identity.workspaceId,storageEpoch:'e'.repeat(64),commitSequence:'9'.repeat(19),predecessor:link,commandKind:'subject-preparation',changes:[]},changeDigest:'d'.repeat(64)};
  assert.equal(catalog.envelopes.proofValueBytes,size(fixed)+4096*maximum+4095);
});
