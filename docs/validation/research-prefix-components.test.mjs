import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {validateResearchPrefix,validateResearchInput} from './research-prefix-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-prefix.py',import.meta.url))]);
test('independent packet binds a fresh input-2 source chain and separate input origins',()=>{
  const p=JSON.parse(generate());assert.equal(p.chain?.plan?.parentPlanBinding?.preparationRevision,2);
  assert.equal(p.inputs[0].value.producer.kind,'parent');assert.equal(p.inputs[1].value.producer.kind,'child');
  assert.equal(p.inputs[1].value.body.decision.inputRevision,2);
});
test('finite prefix checker measures input/control components',()=>{
  assert(validateResearchPrefix(JSON.parse(generate())).inputBytes>0);
});
const retained=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const domain=(name,x)=>sha(name+'\n'+canonical(x));
const fields=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
function sealInput(r){
  const b=r.value.body;delete b.inputDigest;b.inputDigest=domain('orkworks.preparation-input.v1',b);
  r.key=fields(b,'workspaceId runId preparationRevision inputRevision');
}
const runtimeRef=r=>({...r.key,attachmentDigest:r.value.digest});
const inputCheck=(row,p,selected=false)=>validateResearchInput(row,p,selected?p.reportAssumption.source:null);
const view=(p,id)=>p.views.find(x=>x.id===id);
const row=(p,id,table)=>view(p,id).rows.find(x=>x.table===table);
test('retained oracle independently regenerates without mutating prior packet inputs',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),before=canonical(p);validateResearchPrefix(p);assert.equal(canonical(p),before);
  const old=JSON.parse(readFileSync(new URL('research-chain-components.json',import.meta.url)));
  assert.notEqual(p.chain.plan.planDefinitionDigest,old.plan.planDefinitionDigest);
  assert.notEqual(p.chain.startup.scope.task.taskId,old.startup.scope.task.taskId);
});
test('Python helper/input paths containing spaces and percent signs preserve UTF-8 LF bytes',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research prefix % '));
  try{
    for(const name of ['generate-research-prefix.py','generate-research-chain.py','generate-research-startup.py','research-startup-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const output=join(dir,'out %.json');execFileSync('python3',[join(dir,'generate-research-prefix.py'),'--output',output]);assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});
for(const [name,mutate] of [
  ['direct-execution decision',r=>{r.value.body.decision.researchNeeded=false;}],
  ['material clarification',r=>{r.value.body.decision.clarificationNeeded=true;}],
  ['empty reasons',r=>{r.value.body.decision.reasons=[];}],
  ['reason above 2 KiB UTF-8',r=>{r.value.body.decision.reasons=['é'.repeat(1025)];}],
  ['research decision at root revision one',r=>{r.value.body.decision.inputRevision=1;}],
  ['future decision revision',r=>{r.value.body.decision.inputRevision=3;}],
  ['mismatched paired revisions',r=>{r.value.body.preparationRevision=3;}],
  ['revision 65',r=>{r.value.body.preparationRevision=r.value.body.inputRevision=r.value.body.decision.inputRevision=65;}],
  ['fractional revision',r=>{r.value.body.inputRevision=2.5;}],
  ['different goal',r=>{r.value.body.goalDigest='0'.repeat(64);}],
  ['different bootstrap',r=>{r.value.body.bootstrapConfigurationDigest='0'.repeat(64);}],
  ['scope alias',r=>{r.value.body.decision.scope.scopeDigest='0'.repeat(64);}],
  ['unknown decision field',r=>{r.value.body.decision.unknowns=[];}],
  ['missing capture consumption',r=>{r.value.body.records=r.value.body.records.filter(x=>x.kind!=='research-capture');}],
  ['duplicate input record',r=>{r.value.body.records.push(structuredClone(r.value.body.records[0]));}],
  ['unordered records',r=>{r.value.body.records.reverse();}],
  ['future reservation record',r=>{r.value.body.records.push({kind:'reservation',reference:{}});}],
  ['unknown input field',r=>{r.value.body.planDefinitionDigest='0'.repeat(64);}],
])test(`resealed input component denies ${name}`,()=>{const p=packet(),r=structuredClone(p.inputs[0]);mutate(r);sealInput(r);assert.throws(()=>inputCheck(r,p));});
test('a child cannot author a fresh parent assessment',()=>{
  const p=packet(),r=structuredClone(p.inputs[0]);r.value.producer=runtimeRef(p.runtimeHistory[1]);assert.throws(()=>inputCheck(r,p),/parent assessment producer/);
});
test('a reporting child cannot promote decision-2 into a fresh decision-3',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);r.value.body.decision.inputRevision=3;sealInput(r);assert.throws(()=>inputCheck(r,p,true),/parent assessment producer/);
});
test('stale decision provenance stays in the older parent row, not the new producer field',()=>{
  const p=packet();assert.doesNotThrow(()=>inputCheck(p.inputs[1],p,true));
  p.inputs[1].value.producer=runtimeRef(p.runtimeHistory[0]);assert.throws(()=>inputCheck(p.inputs[1],p,true),/child invalidation producer/);
});
test('missing original assessment is not repaired by equal copied decision bytes',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);p.inputs.shift();assert.throws(()=>inputCheck(r,p,true),/unique original assessment row/);
});
test('ambiguous original assessment key denies without electing a matching row',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);p.inputs.push(structuredClone(p.inputs[0]));assert.throws(()=>inputCheck(r,p,true),/unique original assessment row/);
});
test('a changed copied decision cannot borrow the old parent origin',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);r.value.body.decision.reasons=['Changed conclusion.'];sealInput(r);assert.throws(()=>inputCheck(r,p,true),/exact original assessment/);
});
test('a matching old body under a foreign key is not the original assessment',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);p.inputs[0].key.runId='foreign';assert.throws(()=>inputCheck(r,p,true),/unique original assessment row/);
});
test('stale origin must itself be a fresh parent-produced assessment',()=>{
  const p=packet();p.inputs[0].value.producer=runtimeRef(p.runtimeHistory[1]);assert.throws(()=>inputCheck(p.inputs[1],p,true),/parent assessment producer/);
});
test('missing original runtime source does not confer assessment authority',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);p.runtimeHistory.shift();assert.throws(()=>inputCheck(r,p,true));
});
test('selected input must consume the supplied report source exactly once',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);r.value.body.records=r.value.body.records.filter(x=>x.kind!=='research-report');sealInput(r);assert.throws(()=>inputCheck(r,p,true),/exact selected input records/);
});
test('component input checks do not recapture the historical parent from a current view',()=>{
  const p=packet();row(p,'selected-source','research_run_controls').value.body.parentRuntime.launchGeneration='e'.repeat(64);
  assert.doesNotThrow(()=>inputCheck(p.inputs[1],p,true)); // This does not validate the altered view.
});
test('research manifest revision 64 is a bounded source shape, while 65 denies',()=>{
  const p=packet(),r=structuredClone(p.inputs[0]);r.value.body.preparationRevision=r.value.body.inputRevision=r.value.body.decision.inputRevision=64;sealInput(r);assert.doesNotThrow(()=>inputCheck(r,p));
  r.value.body.preparationRevision=r.value.body.inputRevision=r.value.body.decision.inputRevision=65;sealInput(r);assert.throws(()=>inputCheck(r,p));
});
test('whole research input fits exactly 32 KiB and one byte over denies',()=>{
  const p=packet(),r=structuredClone(p.inputs[0]),b=r.value.body;b.decision.reasons=Array.from({length:16},()=> 'x');sealInput(r);
  for(let i=0;i<16;i++){const remaining=32768-Buffer.byteLength(canonical(b));if(remaining>0)b.decision.reasons[i]+='x'.repeat(Math.min(remaining,2047));}
  sealInput(r);assert.equal(Buffer.byteLength(canonical(b)),32768);assert.doesNotThrow(()=>inputCheck(r,p));
  const i=b.decision.reasons.findIndex(x=>x.length<2048);b.decision.reasons[i]+='x';sealInput(r);assert.throws(()=>inputCheck(r,p),/input body 32 KiB/);
});
for(const [name,mutate] of [
  ['mixed input key',p=>{p.inputs[1].key.inputRevision=2;}],
  ['stale current input pointer',p=>{row(p,'selected-source','research_run_controls').value.body.currentInput=row(p,'before-selection','research_run_controls').value.body.currentInput;}],
  ['report bumps task version',p=>{row(p,'selected-source','research_task_controls').value.body.taskVersion=8;}],
  ['frozen version replaces live version',p=>{row(p,'before-selection','research_task_controls').value.body.taskVersion=2;}],
  ['live version replaces frozen identity',p=>{row(p,'before-selection','research_session_states').value.body.assignmentIdentity.taskVersion=7;}],
  ['run report increment omitted',p=>{row(p,'selected-source','research_run_controls').value.body.runVersion=10;}],
  ['second mutable version field',p=>{row(p,'selected-source','research_run_controls').value.body.legacyRunVersion=11;}],
  ['direct-execution profile',p=>{row(p,'proposed','research_run_controls').value.body.profile='single-task-retained-data-execution-v1';}],
  ['fake missing current input default',p=>{row(p,'initial-control','research_run_controls').value.body.currentInput=null;}],
  ['prospective expectation omitted',p=>{view(p,'proposed').rows=view(p,'proposed').rows.filter(x=>x.table!=='research_expectations');}],
  ['collection disabled before a head exists',p=>{row(p,'proposed','research_collections').value.body.active=false;}],
  ['unbound expectation silently selected',p=>{row(p,'proposed','research_expectations').value.body.selectedReport=p.reportAssumption.source;}],
  ['expectation belongs to another run',p=>{row(p,'before-selection','research_expectations').value.body.collection.runId='foreign';}],
  ['selection counter not advanced',p=>{row(p,'selected-source','research_expectations').value.body.expectationVersion=2;}],
  ['caller completeness flag',p=>{p.consumersComplete=true;}],
  ['caller-selected report head',p=>{p.reportAssumption.head.contentDigest='0'.repeat(64);}],
  ['extra view',p=>{p.views.push(structuredClone(p.views[0]));}],
])test(`finite component view denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validateResearchPrefix(p));});
test('runtime attachment cannot use the direct-execution source domain',()=>{
  const p=packet(),r=p.runtimeHistory[0];r.value.digest=domain('orkworks.child-source-runtime-attachment.v1',r.value.body);assert.throws(()=>validateResearchPrefix(p),/research runtime source digest/);
});
test('a fake research input row cannot replace the unchanged root initial source',()=>{
  const p=packet();p.context.initialInput.decision={researchNeeded:true};assert.throws(()=>validateResearchPrefix(p));
});
test('Python-measured component sizes agree with the independent checker',()=>{
  assert.deepEqual(validateResearchPrefix(packet()),{inputBytes:5069,runtimeBytes:1207,viewWrapperBytes:[1407,6066,11828,12536],prospectiveExpectations:1});
});
test('stale child invalidation cannot omit the supplied selection',()=>{
  const p=packet(),r=structuredClone(p.inputs[1]);r.value.body.records=r.value.body.records.filter(x=>x.kind!=='research-report');sealInput(r);
  assert.throws(()=>inputCheck(r,p),/child report invalidation requires selected source/);
});
test('input domain excludes only its self digest',()=>{
  const p=packet(),r=structuredClone(p.inputs[0]);r.value.body.inputDigest='0'.repeat(64);assert.throws(()=>inputCheck(r,p),/research input digest/);
});
test('a matching runtime kind without its original generation cannot produce input',()=>{
  const p=packet(),r=structuredClone(p.inputs[0]);r.value.producer.launchGeneration='e'.repeat(64);assert.throws(()=>inputCheck(r,p),/parent assessment producer/);
});
test('symbolic commit refs retain positive uint64 counter syntax without proving commits',()=>{
  const p=packet();p.context.unqualifiedCommit.commitSequence='18446744073709551615';
  for(const r of [...p.inputs,...p.runtimeHistory,...p.views.flatMap(v=>v.rows)])r.value.lastCommit=structuredClone(p.context.unqualifiedCommit);
  assert.doesNotThrow(()=>validateResearchPrefix(p));p.context.unqualifiedCommit.commitSequence='18446744073709551616';assert.throws(()=>validateResearchPrefix(p),/positive uint64/);
});
