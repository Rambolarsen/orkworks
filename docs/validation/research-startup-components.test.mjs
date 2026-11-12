import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {renderChildInstructions} from './child-source-components.mjs';
import {validateResearchStartup} from './research-startup-components.mjs';

const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-startup.py',import.meta.url))]);
test('independent packet builds a research configuration with retained startup data',()=>{
  const p=JSON.parse(generate());assert.equal(p.configuration?.roleTemplate?.role,'research');
  assert.equal(p.configuration.schemaVersion,3);
  assert.match(p.declaredStartup,/ORKWORKS RETAINED RESEARCH INPUT v1\n/);
});
test('offline configuration/startup validator returns measured bytes',()=>{
  // Literal measurements independently computed with Python canonical UTF-8.
  const p=JSON.parse(generate());assert.deepEqual(validateResearchStartup(p),{configurationBytes:11695,startupBytes:5110,rawEvidenceBytes:1288,rawCaptureBytes:29});
});
const retained=readFileSync(new URL('research-startup-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const sha=x=>createHash('sha256').update(x).digest('hex');
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
function seal(x,field,domain){delete x[field];x[field]=sha(domain+'\n'+canonical(x));}
const artifact=(p,i)=>JSON.parse(Buffer.from(p.evidenceObjects[i].contentBase64,'base64'));
function writeArtifact(p,i,value){
  const wire=canonical(value),v=p.evidenceObjects[i];v.contentBase64=Buffer.from(wire).toString('base64');v.sizeBytes=Buffer.byteLength(wire);v.contentDigest=sha(wire);
  p.configuration.capabilityEvidence.references[i].contentDigest=v.contentDigest;
  if(i===0)p.configuration.harness.effectiveSettingsDigest=p.configuration.capabilityEvidence.effectiveSettingsDigest=v.contentDigest;
  if(i===2)p.configuration.model.policyDigest=v.contentDigest;
}
function compose(p){
  const c=p.configuration,s=artifact(p,0),lookup={'rule':c.rules,'role-template':[c.roleTemplate],'skill':c.skills,'skill-resource':c.skillResources};
  const payload=canonical({scope:p.scope,captures:p.captures});
  return s.deliveryOrder.map(d=>{
    if(d.kind==='supplied')return c.renderedInstructions;
    if(d.kind==='research-input')return '\nORKWORKS RETAINED RESEARCH INPUT v1\n'+Buffer.byteLength(payload)+'\n'+payload+'\n';
    const a=s.discoveredInstructions[d.precedence],v=lookup[a?.snapshotKind]?.find(x=>x.id===a.snapshotId);return v?.content??v?.instructions??'';
  }).join('');
}
function refresh(p,render=true){
  const c=p.configuration,e=c.capabilityEvidence;
  e.requestedProfileDigest=sha(canonical(c.permissions.requested));e.effectiveProfileDigest=sha(canonical(c.permissions.effective));
  seal(e,'evidenceDigest','orkworks.capability-evidence.v1');c.harness.evidenceDigest=e.evidenceDigest;
  if(render)c.renderedInstructions=renderChildInstructions(c);
  c.renderedInstructionsDigest=sha(c.renderedInstructions);seal(c,'configurationDigest','orkworks.assignment-configuration.v3');
  p.declaredStartup=compose(p);
}
test('retained oracle independently regenerates and does not mutate source inputs',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),before=canonical(p);
  validateResearchStartup(p);assert.equal(canonical(p),before);
});
test('generator supports UTF-8 LF output under paths with spaces and percent signs',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research startup % '));
  try{
    for(const name of ['generate-research-startup.py','child-source-vectors.json','research-capture-scope-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const out=join(dir,'out %.json');execFileSync('python3',[join(dir,'generate-research-startup.py'),'--output',out]);assert.deepEqual(readFileSync(out),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});
for(const [name,mutate] of [
  ['direct-execution role',p=>{p.configuration.roleTemplate.role='verification';}],
  ['inline research input',p=>{p.configuration.assignment.inputs=[{kind:'inline',id:'bad',content:'x',contentDigest:sha('x')}];}],
  ['dependency',p=>{p.configuration.assignment.dependencies=['foreign'];}],
  ['live read path',p=>{p.configuration.permissions.requested.readPaths=['src'];}],
  ['live write path',p=>{p.configuration.permissions.effective.writePaths=['src'];}],
  ['command',p=>{p.configuration.permissions.effective.commands=['git'];}],
  ['external source',p=>{p.configuration.permissions.effective.externalSources=['https://example.test'];}],
  ['connector',p=>{p.configuration.permissions.effective.connectors=['search'];}],
  ['coordination authority',p=>{p.configuration.permissions.effective.coordinationActions=['approve'];}],
  ['result tool',p=>{p.configuration.permissions.requested.tools.push('orkworks.result.submit.v1');}],
  ['missing report tool',p=>{p.configuration.permissions.effective.tools.pop();}],
  ['native delegation',p=>{p.configuration.permissions.effective.nativeDelegation='allowed';}],
  ['access change',p=>{p.configuration.permissions.effective.accessChanges='automatic';}],
  ['output allowance',p=>{p.configuration.outputContract.maxSnapshotBytes=1;}],
  ['review snapshot',p=>{p.configuration.reviewInput={};}],
  ['cross-workspace configuration',p=>{p.configuration.workspaceId='foreign';}],
  ['wrong group',p=>{p.configuration.assignment.worktreeGroupId='foreign';}],
  ['wrong base',p=>{p.configuration.assignment.baseCommit='e'.repeat(40);}],
  ['wrong repository ID',p=>{p.configuration.repositoryId='foreign';}],
  ['manifest task alias',p=>{p.configuration.requirementManifests[0].taskId='foreign';}],
  ['missing mandatory skill',p=>{p.configuration.skills=[];p.configuration.skillResources=[];}],
  ['root evidence relabel',p=>{p.configuration.capabilityEvidence.role='orchestrator';}],
  ['verified assertion',p=>{p.configuration.capabilityEvidence.decision='verified';}],
  ['new protocol field',p=>{p.configuration.scopeDigest=p.scope.scopeDigest;}],
  ['unknown packet field',p=>{p.admitted=true;}],
])test(`resealed configuration denies ${name}`,()=>{const p=packet();mutate(p);refresh(p);assert.throws(()=>validateResearchStartup(p));});
for(const [name,mutate] of [
  ['wrong scope digest',s=>{s.deliveryOrder.at(-1).scopeDigest='0'.repeat(64);}],
  ['missing research input',s=>{s.deliveryOrder.pop();}],
  ['duplicate research input',s=>{s.deliveryOrder.push({...s.deliveryOrder.at(-1)});}],
  ['research data before supplied',s=>{[s.deliveryOrder[1],s.deliveryOrder[2]]=[s.deliveryOrder[2],s.deliveryOrder[1]];}],
  ['duplicate supplied',s=>{s.deliveryOrder.unshift({kind:'supplied'});}],
  ['automatic instruction omitted',s=>{s.deliveryOrder.shift();}],
  ['unlisted automatic instruction',s=>{s.deliveryOrder.unshift({kind:'automatic',precedence:99});}],
  ['review-input variant',s=>{s.deliveryOrder.at(-1).kind='review-input';}],
  ['unknown settings field',s=>{s.nativeEligible=true;}],
  ['unmatched discovery digest',s=>{s.discoveredInstructions[0].contentDigest='0'.repeat(64);}],
])test(`resealed startup settings deny ${name}`,()=>{const p=packet(),s=artifact(p,0);mutate(s);writeArtifact(p,0,s);refresh(p);assert.throws(()=>validateResearchStartup(p));});
for(const [name,mutate] of [
  ['missing evidence occurrence',p=>{p.evidenceObjects.pop();}],
  ['extra evidence occurrence',p=>{p.evidenceObjects.push(structuredClone(p.evidenceObjects[0]));}],
  ['wrong evidence task',p=>{p.evidenceObjects[0].task.taskId='foreign';}],
  ['wrong raw evidence size',p=>{p.evidenceObjects[0].sizeBytes++;}],
  ['unpadded Base64',p=>{p.evidenceObjects[0].contentBase64+='=';}],
])test(`retained evidence denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validateResearchStartup(p));});
test('model policy bytes cannot disagree with the selected model',()=>{
  const p=packet(),policy=artifact(p,2);policy.modelId='foreign';writeArtifact(p,2,policy);refresh(p);assert.throws(()=>validateResearchStartup(p));
});
test('rendered child framing must match the complete assignment/rules/skills',()=>{
  const p=packet();p.configuration.renderedInstructions+='\n';refresh(p,false);assert.throws(()=>validateResearchStartup(p));
});
test('declared startup cannot omit data or change framed byte length',()=>{
  const p=packet();p.declaredStartup=p.configuration.renderedInstructions;assert.throws(()=>validateResearchStartup(p));
  const q=packet();q.declaredStartup=q.declaredStartup.replace('ORKWORKS RETAINED RESEARCH INPUT v1\n','ORKWORKS RETAINED RESEARCH INPUT v1\r\n');assert.throws(()=>validateResearchStartup(q));
});
function growRole(p,n){
  const c=p.configuration,r=c.roleTemplate;r.instructions=n>=0?r.instructions+'x'.repeat(n):r.instructions.slice(0,n);r.instructionsDigest=sha(r.instructions);
  delete r.contentDigest;r.contentDigest=sha(canonical(r));
  for(const m of c.requirementManifests)if(m.sourceId===r.id)m.sourceDigest=r.contentDigest;
  for(const s of c.skills)for(const source of s.requirementSources)if(source.sourceId===r.id)source.sourceDigest=r.contentDigest;
  refresh(p);
}
test('complete composed startup fits at 256 KiB and one extra byte denies',()=>{
  const p=packet();growRole(p,262144-Buffer.byteLength(p.declaredStartup));
  // Account for the decimal framing length growing from two to six digits.
  growRole(p,262144-Buffer.byteLength(p.declaredStartup));
  assert.equal(Buffer.byteLength(p.declaredStartup),262144);assert.equal(validateResearchStartup(p).startupBytes,262144);
  growRole(p,1);assert.throws(()=>validateResearchStartup(p),/composed startup/);
});
test('an automatic instruction may follow the complete supplied/data pair',()=>{
  const p=packet(),s=artifact(p,0);s.deliveryOrder.push(s.deliveryOrder.shift());writeArtifact(p,0,s);refresh(p);assert.doesNotThrow(()=>validateResearchStartup(p));
});
test('tool-managed model has a null explicit model and matching retained policy',()=>{
  const p=packet(),s=artifact(p,2);s.mode='tool-managed';s.modelId=null;s.unknownModelAllowed=true;writeArtifact(p,2,s);
  p.configuration.model.mode='tool-managed';p.configuration.model.modelId=null;refresh(p);assert.doesNotThrow(()=>validateResearchStartup(p));
});
test('zero-byte non-settings evidence is retained as a real occurrence',()=>{
  const p=packet(),v=p.evidenceObjects[3];v.contentBase64='';v.sizeBytes=0;v.contentDigest=sha('');p.configuration.capabilityEvidence.references[3].contentDigest=v.contentDigest;refresh(p);assert.doesNotThrow(()=>validateResearchStartup(p));
});
test('equal evidence under a new reference ID contributes its full raw occurrence size',()=>{
  const p=packet(),before=validateResearchStartup(p).rawEvidenceBytes,v=structuredClone(p.evidenceObjects[3]),r=structuredClone(p.configuration.capabilityEvidence.references[3]);
  v.referenceId=r.id='evidence-4';p.evidenceObjects.push(v);p.configuration.capabilityEvidence.references.push(r);refresh(p);
  assert.equal(validateResearchStartup(p).rawEvidenceBytes,before+v.sizeBytes);
});
test('noncanonical retained settings JSON cannot qualify through a matching raw hash',()=>{
  const p=packet(),v=p.evidenceObjects[0],wire=Buffer.from(v.contentBase64,'base64').toString();
  const duplicate='{"schemaVersion":1,'+wire.slice(1);v.contentBase64=Buffer.from(duplicate).toString('base64');v.sizeBytes=Buffer.byteLength(duplicate);v.contentDigest=sha(duplicate);
  const c=p.configuration;c.capabilityEvidence.references[0].contentDigest=c.capabilityEvidence.effectiveSettingsDigest=c.harness.effectiveSettingsDigest=v.contentDigest;refresh(p);
  assert.throws(()=>validateResearchStartup(p),/canonical stored bytes/);
});
test('configuration cannot substitute a new research-specific digest domain',()=>{
  const p=packet();seal(p.configuration,'configurationDigest','orkworks.research-configuration.v1');assert.throws(()=>validateResearchStartup(p),/unchanged v3 configuration digest/);
});
test('current settings cannot be selected by an unrelated harness digest',()=>{
  const p=packet();p.configuration.harness.effectiveSettingsDigest='0'.repeat(64);refresh(p);assert.throws(()=>validateResearchStartup(p),/harness\/evidence binding/);
});
