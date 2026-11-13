import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {validateResearchInitialization} from './research-initialization-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-initialization.py',import.meta.url))]);
const baselineBytes=readFileSync(new URL('root-preparation-effects.json',import.meta.url));
const baseline=()=>JSON.parse(baselineBytes);
test('independent generator retains fourteen workspace resource values and two branches',()=>{
  const p=JSON.parse(generate());assert.equal(p.workspaceInitialization?.length,14);assert.equal(p.admissions?.length,2);
});
test('checker measures the accepted finite initializer components',()=>{
  assert.equal(validateResearchInitialization(JSON.parse(generate()),baseline(),baselineBytes).classification,'offline-research-initialization-components-only');
});
const retained=readFileSync(new URL('research-initialization-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const validate=p=>validateResearchInitialization(p,baseline(),baselineBytes);
const branch=(p,id='initialized')=>p.admissions.find(x=>x.id===id);
const peer=(xs,name)=>xs.find(r=>r.key.resource===name);
test('independent regeneration and validation preserve both retained inputs',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),b=baseline(),before=[JSON.stringify(p),JSON.stringify(b)];
  validateResearchInitialization(p,b,baselineBytes);assert.deepEqual([JSON.stringify(p),JSON.stringify(b)],before);
});
test('generator/input/output paths with spaces and percent signs preserve exact UTF-8 LF bytes',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research init % '));try{
    for(const n of ['generate-research-initialization.py','root-preparation-effects.json'])copyFileSync(fileURLToPath(new URL(n,import.meta.url)),join(dir,n));
    const output=join(dir,'oracle %.json');execFileSync('python3',[join(dir,'generate-research-initialization.py'),'--output',output]);assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});
for(const resource of ['evaluation-artifact-bytes','evaluation-artifact-read-bytes','child-source-retained-bytes','child-control-retained-bytes','report-retained-bytes'])test(`workspace creation cannot omit ${resource}`,()=>{
  const p=packet();p.workspaceInitialization=p.workspaceInitialization.filter(r=>r.key.resource!==resource);assert.throws(()=>validate(p));
});
for(const [name,mutate] of [
  ['workspace reset at admission',p=>{peer(branch(p).workspaceAfter,'child-source-retained-bytes').value.used='0';}],
  ['lifetime read-work reset',p=>{peer(branch(p).workspaceAfter,'evaluation-artifact-read-bytes').value.used='0';}],
  ['report balance reset',p=>{peer(branch(p).workspaceAfter,'report-retained-bytes').value.used='0';}],
  ['unchanged peer stamped',p=>{peer(branch(p).workspaceAfter,'child-source-retained-bytes').value.lastCommit=p.unqualifiedOrigins.admission;}],
  ['run-count incremented twice',p=>{const r=peer(branch(p).workspaceAfter,'preparation-run-count');r.value.used=String(BigInt(r.value.used)+1n);}],
  ['summary pending fabricated',p=>{peer(branch(p).workspaceAfter,'summary-pending-count').value.used='1';}],
  ['control workspace delta doubled',p=>{const b=branch(p),r=peer(b.workspaceAfter,'child-control-retained-bytes'),old=peer(p.capturedWorkspacePeers,'child-control-retained-bytes');r.value.used=String(2n*BigInt(r.value.used)-BigInt(old.value.used));}],
  ['initial run control uncharged',p=>{peer(branch(p).newResourceRows,'child-control-retained-bytes').value.used='0';}],
  ['new artifact pool used nonzero',p=>{peer(branch(p).newResourceRows,'evaluation-artifact-bytes').value.used='1';}],
  ['new artifact resource missing',p=>{const b=branch(p);b.newResourceRows=b.newResourceRows.filter(r=>r.key.resource!=='evaluation-artifact-read-bytes');}],
  ['new report assignment resource',p=>{const b=branch(p),r=structuredClone(peer(b.newResourceRows,'report-retained-bytes'));r.key.owner=b.newResourceRows.find(r=>r.key.owner.kind==='assignment').key.owner;b.newResourceRows.push(r);}],
  ['new row reserved credit',p=>{branch(p).newResourceRows[0].value.reserved='1';}],
  ['foreign run resource',p=>{branch(p).newResourceRows.find(r=>r.key.owner.kind==='run').key.owner.runId='foreign';}],
  ['unknown resource',p=>{p.workspaceInitialization[0].key.resource='another-pool';}],
  ['numeric ledger counter',p=>{p.workspaceInitialization[0].value.used=0;}],
  ['noncanonical counter',p=>{p.workspaceInitialization[0].value.used='00';}],
  ['uint64 overflow',p=>{p.workspaceInitialization[0].value.used='18446744073709551616';}],
  ['control starts at version two',p=>{branch(p).initialControl.value.body.runVersion=2;}],
  ['wrong current input domain',p=>{branch(p).initialControl.value.body.currentInput.inputDigest='0'.repeat(64);}],
  ['fake research input-1 producer',p=>{branch(p).initialControl.value.body.currentInput.producer={kind:'parent'};}],
  ['initial parent runtime invented',p=>{branch(p).initialControl.value.body.parentRuntime={kind:'parent'};}],
  ['second current-owner family',p=>{branch(p).initialControl.table='child_run_controls';}],
  ['run namespace changed',p=>{branch(p).initialControl.key.runId='foreign';}],
  ['control origin not admission',p=>{branch(p).initialControl.value.lastCommit=p.unqualifiedOrigins.draft;}],
  ['artifact projection occurrence omitted',p=>{p.preparationChargeFacts=p.preparationChargeFacts.filter(f=>f.table!=='preparation_artifacts');}],
  ['preparation bytes discounted',p=>{p.preparationChargeFacts[0].afterBytes=0;}],
  ['run preparation reset at admission',p=>{branch(p).runPreparationAfter.value.used='0';}],
  ['never-initialized summary charged',p=>{branch(p,'never-initialized').summaryChargeDeltas['summary-row-count']=1;}],
  ['initialized summary charge omitted',p=>{branch(p).summaryChargeDeltas['summary-row-bytes']=0;}],
  ['logical zero-row member missing',p=>{branch(p).logicalLocations.pop();}],
  ['extra logical member',p=>{branch(p).logicalLocations.push({table:'report_heads',key:{}});}],
  ['actual header shape supplied',p=>{p.unqualifiedControls[0]={schemaVersion:7};}],
  ['format field fabricated',p=>{p.format=7;}],
  ['row commitment fabricated',p=>{branch(p).initialControl.rowDigest='0'.repeat(64);}],
  ['original proof qualification flag',p=>{p.originalProducerQualified=true;}],
  ['assumed complete inventory substituted',p=>{p.assumptions=['complete'];}],
  ['origin counter zero',p=>{p.unqualifiedOrigins.admission.commitSequence='0';}],
  ['origin counter numeric',p=>{p.unqualifiedOrigins.admission.commitSequence=4;}],
])test(`initializer component denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validate(p));});
for(const resource of ['report-retained-bytes','child-source-retained-bytes','child-control-retained-bytes','reporting-receipt-bytes'])test(`draft lifetime cannot initialize ${resource}`,()=>{
  const p=packet(),r=structuredClone(p.draftRunResources[0]);r.key.resource=resource;p.draftRunResources.push(r);assert.throws(()=>validate(p));
});
test('cancel locations cannot delete admitted source history',()=>{const p=packet();p.cancelLocations.push({table:'root_preparations',key:branch(p).initialControl.key});assert.throws(()=>validate(p));});
test('exact retry comparison has no writes, controls or initialization',()=>{const p=packet();p.retryLocations.push(branch(p).logicalLocations[0]);assert.throws(()=>validate(p));});
test('fixed logical location counts do not count header/proof labels as real rows',()=>{
  const p=packet();assert.deepEqual(p.admissions.map(b=>b.logicalLocations.length),[25,22]);assert.equal(p.unqualifiedControls.length,2);
  assert.deepEqual(p.admissions.map(b=>b.newResourceRows.length),[13,13]);
});
test('independent Python measurements agree with the finite checker',()=>{
  const result=validate(packet());assert.deepEqual(result.admissions,[{id:'initialized',newResourceRows:13,logicalRows:27},{id:'never-initialized',newResourceRows:13,logicalRows:24}]);
  assert.equal(result.workspaceResources,14);assert.equal(result.initializeLogicalRows,16);assert.equal(result.draftLogicalRows,11);assert.equal(result.cancelLogicalRows,11);assert.equal(result.retryLogicalRows,0);
  assert.equal(result.controlChargeBytes,1202);assert.equal(result.preparationBeforeBytes,19259);assert.equal(result.preparationAfterBytes,21741);
  assert.equal(result.controlWrapperBytes,1419);assert.equal(result.workspaceWrapperBytes,4744);
});
