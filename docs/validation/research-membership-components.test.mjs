import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {validateResearchMembership} from './research-membership-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-membership.py',import.meta.url))]);
const prefix=()=>JSON.parse(readFileSync(new URL('research-prefix-components.json',import.meta.url)));
test('independent generator derives separate forward and reverse membership groups',()=>{
  const p=JSON.parse(generate());assert.equal(p.groups?.length,7);assert(p.groups[0].entries.length>0);
});
test('checker measures the bounded membership and local effect comparisons',()=>{
  const p=JSON.parse(generate());assert(validateResearchMembership(p,prefix()).entries>0);
});
const retained=readFileSync(new URL('research-membership-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const dom=(name,x)=>sha(name+'\n'+canonical(x));
const group=(p,id)=>p.groups.find(g=>g.id===id);
const entry=(p,g,id)=>group(p,g).entries.find(e=>e.id===id);
function reverse(g){
  const out=new Map();for(const e of g.entries)for(const target of e.targets){const k=canonical(target);if(!out.has(k))out.set(k,{target,consumers:new Map()});out.get(k).consumers.set(canonical(e.consumer),e.consumer);}
  const order=([a],[b])=>Buffer.compare(Buffer.from(a),Buffer.from(b));
  g.reverse=[...out].sort(order).map(([,x])=>({target:x.target,consumers:[...x.consumers].sort(order).map(([,c])=>c)}));
}
test('oracle regenerates independently and checker leaves both inputs unchanged',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),s=prefix(),old=[canonical(p),canonical(s)];validateResearchMembership(p,s);assert.deepEqual([canonical(p),canonical(s)],old);
});
test('generator and input copied under spaces and percent signs preserve exact output',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research membership % '));try{
    for(const name of ['generate-research-membership.py','research-prefix-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const output=join(dir,'output %.json');execFileSync('python3',[join(dir,'generate-research-membership.py'),'--output',output]);assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});
test('independent reverse reconstruction preserves the finite packet',()=>{
  const p=packet();p.groups.forEach(reverse);assert.doesNotThrow(()=>validateResearchMembership(p,prefix()));
  assert.deepEqual(p.groups.map(g=>g.entries.length),[11,8,2,1,5,7,7]);
});
test('independent Python counts and conditional trace measurements agree',()=>{
  assert.deepEqual(validateResearchMembership(packet(),prefix()),{
    classification:'conditional-research-membership-components-only',entries:41,occurrences:176,localChanges:4,forwardBytes:72528,comparisonBytes:14706,
    discovery:{classification:'conditional-discovery-trace-only',observations:3,canonicalBytes:2362},
  });
});
for(const table of ['research_task_controls','research_session_states','research_launch_controls'])test(`report comparison cannot mutate ${table}`,()=>{
  const p=packet(),s=prefix(),old=s.views.find(v=>v.id==='before-selection').rows.find(r=>r.table===table);
  const after=structuredClone(old.value);
  if(table==='research_task_controls')after.body.taskVersion=8;
  else if(table==='research_session_states')after.body.assignmentIdentity.taskVersion=7;
  else after.body.admission.assignmentTaskVersion=7;
  const location={kind:'metadata',table,key:old.key};p.effectComparison.unchanged=p.effectComparison.unchanged.filter(x=>x.table!==table);
  p.effectComparison.changes.push({location,before:old.value,after});assert.throws(()=>validateResearchMembership(p,s),/local value comparison/);
});
for(const [name,mutate] of [
  ['missing research owner',p=>{group(p,'sources').entries.pop();}],
  ['unknown research owner',p=>{const e=structuredClone(group(p,'sources').entries[0]);e.id='unknown';e.consumer.reference.key.kind='other-source';group(p,'sources').entries.push(e);}],
  ['missing scope capture',p=>{entry(p,'sources','scope').targets.pop();}],
  ['missing original plan input',p=>{const e=entry(p,'sources','plan');e.targets=e.targets.filter(t=>t.table!=='research_control_inputs');}],
  ['stale decision origin omitted',p=>{const e=entry(p,'history','input-3');e.targets=e.targets.filter(t=>t.table!=='research_control_inputs');}],
  ['historical parent origin omitted',p=>{const e=entry(p,'history','input-3');e.targets=e.targets.filter(t=>t.table!=='research_runtime_attachments'||t.key.kind!=='parent');}],
  ['foreign task source',p=>{entry(p,'sources','assignment').consumer.reference.key.identity.taskId='foreign';}],
  ['unknown membership target',p=>{entry(p,'sources','plan').targets.push({kind:'caller-target'});}],
  ['fake approval on a pending plan',p=>{entry(p,'proposed','research_plan_controls').targets.push(entry(p,'history','research_plan_approvals').consumer);}],
  ['fake reservation on a pending task',p=>{entry(p,'proposed','research_task_controls').targets.push(entry(p,'history','research_reservations').consumer);}],
  ['pre-report expectation consumes a report',p=>{entry(p,'proposed','research_expectations').targets.push(entry(p,'sources','report').consumer);}],
  ['inline input invents standalone evidence source',p=>{entry(p,'history','input-3').targets.push(p.standaloneSourceExample.consumer);}],
  ['selected expectation loses report retention',p=>{const e=entry(p,'selected-source','research_expectations');e.targets=e.targets.filter(t=>t.reference?.key.kind!=='research-report');}],
  ['separate standalone source loses report',p=>{p.standaloneSourceExample.targets=[];}],
  ['reverse-only missing consumer',p=>{group(p,'history').reverse.pop();}],
  ['alternative views flattened together',p=>{group(p,'before-selection').entries.push(...group(p,'selected-source').entries);}],
  ['foreign prospective consumer',p=>{p.prospectiveOwnerAssumption.push({collection:{workspaceId:'foreign',runId:'foreign'},task:{}});}],
  ['missing positive collection',p=>{p.prospectiveOwnerAssumption=[];}],
  ['head reference drift',p=>{p.registryComponents.headRow.value.head.contentDigest='0'.repeat(64);}],
  ['receipt request domain substitution',p=>{p.registryComponents.receiptRow.value.requestDigest=dom('orkworks.other-request.v1',p.registryComponents.request);}],
  ['receipt copies descriptor',p=>{p.registryComponents.receiptRow.value.report=p.registryComponents.request.report;}],
  ['frozen version substitutes for live report CAS',p=>{const c=p.registryComponents;c.request.expectedTaskVersion=c.receiptRow.value.expectedTaskVersion=2;c.receiptRow.value.requestDigest=dom('orkworks.research-report-request.v1',c.request);}],
  ['report event producer lost',p=>{p.effectComparison.changes.find(x=>x.location.table==='research_control_inputs').after.producer.kind='parent';}],
  ['selected input pointer not updated',p=>{const x=p.effectComparison.changes.find(x=>x.location.table==='research_run_controls');x.after.body.currentInput=x.before.body.currentInput;}],
  ['collection version unchanged',p=>{const x=p.effectComparison.changes.find(x=>x.location.table==='research_collections');x.after.body.collectionVersion=x.before.body.collectionVersion;}],
  ['task comparison erased',p=>{p.effectComparison.unchanged=p.effectComparison.unchanged.filter(x=>x.table!=='research_task_controls');}],
  ['unsupported completeness claim',p=>{p.consumersComplete=true;}],
])test(`component packet denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validateResearchMembership(p,prefix()));});
for(const id of ['plan','input-3'])test(`forward ${id} loss denies even with matching rebuilt reverse`,()=>{
  const p=packet(),g=group(p,id==='plan'?'sources':'history'),e=g.entries.find(x=>x.id===id);e.targets=e.targets.filter(t=>t.table!=='research_control_inputs');reverse(g);
  assert.throws(()=>validateResearchMembership(p,prefix()),/forward membership/);
});
test('conditional discovery admits 4096 observations and denies the next miss',()=>{
  const p=packet();p.discoveryTrace=Array.from({length:4096},()=>({kind:'miss',key:null}));assert.equal(validateResearchMembership(p,prefix()).discovery.observations,4096);
  p.discoveryTrace.push({kind:'miss',key:null});assert.throws(()=>validateResearchMembership(p,prefix()),/observation ceiling/);
});
test('repeated owned copies consume cumulative bytes independently of observation count',()=>{
  const p=packet();p.discoveryTrace=Array.from({length:8192},()=>({kind:'copy',key:'',value:'x'.repeat(2044)}));assert.equal(validateResearchMembership(p,prefix()).discovery.canonicalBytes,16777216);
  p.discoveryTrace.push({kind:'copy',key:'',value:''});assert.throws(()=>validateResearchMembership(p,prefix()),/byte ceiling/);
});
for(const [name,mutate] of [
  ['unproven completion',p=>{p.discoveryAssumption.complete=false;}],
  ['changed captured header',p=>{p.discoveryAssumption.finalHeader.commitSequence='2';}],
  ['unknown query event',p=>{p.discoveryTrace=[{kind:'free-query'}];}],
])test(`conditional discovery denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validateResearchMembership(p,prefix()));});
