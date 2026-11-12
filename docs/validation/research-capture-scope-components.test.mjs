import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import test from 'node:test';
import {decodeResearchComponent,validateResearchScope} from './research-capture-scope-components.mjs';
import {execFileSync} from 'node:child_process';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

// Test-side construction; no digest helper from the validator.
const canonical = x => {
  if(Array.isArray(x))return '['+x.map(canonical).join(',')+']';
  if(x&&typeof x==='object')return '{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}';
  return JSON.stringify(x);
};
const sha=x=>createHash('sha256').update(x).digest('hex');
function seal(x,field,domain){delete x[field];x[field]=sha(domain+'\n'+canonical(x));}
const task={workspaceId:'w',runId:'r',planId:'p',planRevision:1,taskId:'t'};
function example(content='Aé🙂Z\n'){
  const context={task:structuredClone(task),repositoryBindingDigest:'a'.repeat(64),sourceWorktreeBindingDigest:'b'.repeat(64),baseCommit:'c'.repeat(40),objectFormat:'sha1'};
  const c={schemaVersion:1,task:structuredClone(task),sourceId:'source-a',repositoryBindingDigest:context.repositoryBindingDigest,sourceWorktreeBindingDigest:context.sourceWorktreeBindingDigest,path:'src/note.txt',revision:context.baseCommit,content,sizeBytes:Buffer.byteLength(content),contentDigest:sha(content)};
  const scope={schemaVersion:1,task:structuredClone(task),repositoryBindingDigest:context.repositoryBindingDigest,sourceWorktreeBindingDigest:context.sourceWorktreeBindingDigest,captures:[],questions:[{questionId:'q-a',questionText:'What is retained?',sourceIds:['source-a'],knownInputs:'Synthetic text only.',expectedEvidence:'A byte-exact excerpt.',stoppingConditions:'Stop after the retained input.',uncertainty:'Native origin is unverified.'}],excerpts:[]};
  const p={context,captures:[c],scope};reseal(p);return p;
}
function reseal(p){
  for(const c of p.captures)seal(c,'captureDigest','orkworks.research-source-capture.v1');
  p.scope.captures=p.captures.map(c=>({task:structuredClone(c.task),sourceId:c.sourceId,captureDigest:c.captureDigest}));
  seal(p.scope,'scopeDigest','orkworks.research-task-scope.v1');
}
const check=p=>validateResearchScope(p.captures,p.scope,p.context);
function excerpt(p,startByte=1,lengthBytes=6){
  const c=p.captures[0];return {sourceId:c.sourceId,startByte,lengthBytes,reference:{kind:'file',id:'e-a',path:c.path,revision:c.revision,location:`bytes:${startByte}-${startByte+lengthBytes}`,excerptDigest:sha(Buffer.from(c.content).subarray(startByte,startByte+lengthBytes))}};
}
test('exact multibyte excerpt fits without source mutation',()=>{
  const p=example();p.scope.excerpts=[excerpt(p)];reseal(p);const before=canonical(p);
  assert.equal(check(p).rawCaptureBytes,9);assert.equal(canonical(p),before);
});
// Reseal semantic mutations so stale hashes cannot mask the intended denial.
for(const [name,mutate,reason] of [
  ['raw digest mismatch',p=>{p.captures[0].contentDigest='0'.repeat(64);},/content digest/],
  ['raw size mismatch',p=>{p.captures[0].sizeBytes++;},/raw size/],
  ['empty content',p=>{p.captures[0].content='';},/capture content/],
  ['NUL content',p=>{p.captures[0].content='\0';},/capture content/],
  ['unpaired surrogate',p=>{p.captures[0].content='\ud800';},/UTF-8/],
  ['cross-task capture',p=>{p.captures[0].task.taskId='foreign';},/task binding/],
  ['foreign repository',p=>{p.captures[0].repositoryBindingDigest='d'.repeat(64);},/repository binding/],
  ['foreign worktree',p=>{p.captures[0].sourceWorktreeBindingDigest='d'.repeat(64);},/worktree binding/],
  ['foreign revision',p=>{p.captures[0].revision='d'.repeat(40);},/base commit/],
  ['traversal path',p=>{p.captures[0].path='src/../a';},/relative path/],
  ['absolute path',p=>{p.captures[0].path='/a';},/relative path/],
  ['backslash path',p=>{p.captures[0].path='src\\a';},/relative path/],
  ['extra capture field',p=>{p.captures[0].nativeVerified=true;},/closed fields/],
  ['unresolved source',p=>{p.scope.questions[0].sourceIds=['missing'];},/capture reference/],
  ['empty question set',p=>{p.scope.questions=[];},/question count/],
  ['empty source set',p=>{p.scope.questions[0].sourceIds=[];},/question source count/],
  ['duplicate source ID',p=>{p.scope.questions[0].sourceIds=['source-a','source-a'];},/sorted unique/],
  ['empty known inputs',p=>{p.scope.questions[0].knownInputs='';},/bounded text/],
  ['field above 2 KiB',p=>{p.scope.questions[0].uncertainty='é'.repeat(1025);},/bounded text/],
  ['whole question above 8 KiB',p=>{for(const k of ['questionText','knownInputs','expectedEvidence','stoppingConditions','uncertainty'])p.scope.questions[0][k]='x'.repeat(1700);},/question bytes/],
  ['extra scope field',p=>{p.scope.configurationDigest='d'.repeat(64);},/closed fields/],
  ['cross-task scope',p=>{p.scope.task.taskId='foreign';},/task binding/],
  ['fractional plan revision',p=>{p.context.task.planRevision=1.5;},/integer/],
  ['plan revision above 64',p=>{p.context.task.planRevision=65;},/integer/],
  ['wrong object format',p=>{p.context.objectFormat='sha256';},/object format/],
  ['extra context field',p=>{p.context.approved=true;},/closed fields/],
])test(`resealed source rejects ${name}`,()=>{const p=example();mutate(p);reseal(p);assert.throws(()=>check(p),reason);});
for(const [name,mutate,reason] of [
  ['split start codepoint',e=>{e.startByte=2;e.lengthBytes=5;e.reference.location='bytes:2-7';},/UTF-8 slice/],
  ['split end codepoint',e=>{e.lengthBytes=3;e.reference.location='bytes:1-4';},/UTF-8 slice/],
  ['zero slice',e=>{e.lengthBytes=0;},/excerpt length/],
  ['negative start',e=>{e.startByte=-1;},/excerpt start/],
  ['fractional start',e=>{e.startByte=0.5;},/excerpt start/],
  ['overrun',e=>{e.lengthBytes=20;},/slice bounds/],
  ['unsafe start',e=>{e.startByte=Number.MAX_SAFE_INTEGER+1;},/excerpt start/],
  ['wrong path',e=>{e.reference.path='elsewhere';},/excerpt path/],
  ['wrong revision',e=>{e.reference.revision='f'.repeat(40);},/excerpt revision/],
  ['wrong digest',e=>{e.reference.excerptDigest='0'.repeat(64);},/excerpt digest/],
  ['padded decimal',e=>{e.reference.location='bytes:01-7';},/exact byte locator/],
  ['wrong exclusive end',e=>{e.reference.location='bytes:1-6';},/exact byte locator/],
  ['missing source',e=>{e.sourceId='unknown';},/excerpt source/],
  ['URL variant',e=>{e.reference.kind='url';},/file reference/],
  ['extra reference field',e=>{e.reference.url='https://example.test';},/closed fields/],
])test(`resealed excerpt rejects ${name}`,()=>{const p=example(),e=excerpt(p);mutate(e);p.scope.excerpts=[e];reseal(p);assert.throws(()=>check(p),reason);});
test('stale capture digest cannot be repaired by a scope digest',()=>{
  const p=example();p.captures[0].captureDigest='0'.repeat(64);p.scope.captures[0].captureDigest='0'.repeat(64);seal(p.scope,'scopeDigest','orkworks.research-task-scope.v1');assert.throws(()=>check(p),/capture digest/);
});
test('scope rejects the direct-execution source domain',()=>{
  const p=example();seal(p.scope,'scopeDigest','orkworks.child-source-scope.v1');assert.throws(()=>check(p),/scope digest/);
});
test('valid capture does not repair a stale scope reference',()=>{
  const p=example();p.scope.captures[0].captureDigest='0'.repeat(64);seal(p.scope,'scopeDigest','orkworks.research-task-scope.v1');assert.throws(()=>check(p),/exact capture references/);
});
test('64 KiB raw capture fits and one byte more denies',()=>{
  assert.doesNotThrow(()=>check(example('x'.repeat(65536))));assert.throws(()=>check(example('x'.repeat(65537))),/capture content/);
});
test('2 KiB excerpt fits and one byte more denies',()=>{
  const p=example('x'.repeat(4096));p.scope.excerpts=[excerpt(p,0,2048)];reseal(p);assert.doesNotThrow(()=>check(p));p.scope.excerpts=[excerpt(p,0,2049)];reseal(p);assert.throws(()=>check(p),/excerpt length/);
});

test('canonical byte decoder preserves exact Unicode without normalization',()=>{
  const p=example('e\u0301 ≠ é\n');const wire=canonical(p.scope);
  assert.deepEqual(decodeResearchComponent(Buffer.from(wire)),p.scope);
});
for(const [name,wire] of [
  ['duplicate members','{"a":1,"a":1}'],['whitespace',' {"a":1}'],
  ['fractional number','{"a":1.5}'],['negative zero','{"a":-0}'],
  ['unsafe integer','{"a":9007199254740993}'],['unpaired surrogate','{"a":"\\ud800"}'],
  ['BOM','\ufeff{"a":1}'],['noncanonical escape','{"a":"\\u0061"}'],
  ['out of order','{"b":1,"a":2}'],
])test(`byte decoder rejects ${name}`,()=>assert.throws(()=>decodeResearchComponent(Buffer.from(wire))));
test('byte decoder rejects malformed UTF-8 without replacement',()=>{
  assert.throws(()=>decodeResearchComponent(Buffer.from([0x22,0xc3,0x28,0x22])));
});
test('independent Python packet contains two complete joined source occurrences',()=>{
  const wire=execFileSync('python3',[fileURLToPath(new URL('generate-research-capture-scope.py',import.meta.url))]);
  const p=JSON.parse(wire);assert.equal(p.captures?.length,2);
  assert.equal(check(p).rawCaptureBytes,29);
});
test('retained oracle regenerates byte-for-byte without validator code',()=>{
  const wire=execFileSync('python3',[fileURLToPath(new URL('generate-research-capture-scope.py',import.meta.url))]);
  assert.deepEqual(wire,readFileSync(new URL('research-capture-scope-components.json',import.meta.url)));
  // Independently measured from the Python packet, including JSON framing.
  const p=JSON.parse(wire);assert.deepEqual(check(p),{rawCaptureBytes:29,captureBodyBytes:1200,scopeBodyBytes:1864});
});
test('generator paths with spaces and percent signs produce identical UTF-8 LF bytes',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research source % '));
  try{
    const script=join(dir,'generate source.py'),output=join(dir,'result %.json');
    copyFileSync(fileURLToPath(new URL('generate-research-capture-scope.py',import.meta.url)),script);
    execFileSync('python3',[script,'--output',output]);
    assert.deepEqual(readFileSync(output),readFileSync(new URL('research-capture-scope-components.json',import.meta.url)));
  }finally{rmSync(dir,{recursive:true,force:true});}
});
function sources(n,size=1){
  const p=example('x'.repeat(size));
  p.captures=Array.from({length:n},(_,i)=>({...structuredClone(p.captures[0]),sourceId:`s-${String(i).padStart(2,'0')}`}));
  p.scope.questions[0].sourceIds=p.captures.map(c=>c.sourceId);reseal(p);return p;
}
test('16 raw 64 KiB occurrences fit the 1 MiB aggregate and 17 captures deny',()=>{
  assert.equal(check(sources(16,65536)).rawCaptureBytes,1048576);
  assert.throws(()=>check(sources(17)),/capture count/);
  assert.throws(()=>check(sources(0)),/capture count/);
});
test('equal contents under distinct IDs remain separate measured occurrences',()=>{
  const p=sources(2,10);assert.equal(check(p).rawCaptureBytes,20);
  p.scope.questions[0].sourceIds=['s-00'];reseal(p);assert.throws(()=>check(p),/every capture used/);
});
for(const [name,mutate,reason] of [
  ['duplicate capture',p=>{p.captures.push(structuredClone(p.captures[0]));},/unique capture ID/],
  ['capture order',p=>{p.captures.reverse();},/sorted unique/],
  ['duplicate question',p=>{p.scope.questions.push(structuredClone(p.scope.questions[0]));},/sorted unique/],
  ['question order',p=>{const q=structuredClone(p.scope.questions[0]);q.questionId='a';p.scope.questions.push(q);},/sorted unique/],
  ['source order',p=>{p.scope.questions[0].sourceIds.reverse();},/sorted unique/],
  ['invalid source ID',p=>{p.captures[0].sourceId='has space';},/ASCII ID/],
  ['extra question field',p=>{p.scope.questions[0].answer='invented';},/closed fields/],
  ['extra task field',p=>{p.captures[0].task.epoch=1;},/closed fields/],
  ['uppercase digest',p=>{p.captures[0].contentDigest='A'.repeat(64);},/lowercase SHA-256/],
])test(`source collection rejects ${name}`,()=>{const p=sources(2);mutate(p);reseal(p);assert.throws(()=>check(p),reason);});
test('32 questions fit and 33 deny',()=>{
  const p=example();p.scope.questions=Array.from({length:32},(_,i)=>({...structuredClone(p.scope.questions[0]),questionId:`q-${String(i).padStart(2,'0')}`}));
  reseal(p);assert.doesNotThrow(()=>check(p));p.scope.questions.push({...p.scope.questions[0],questionId:'q-32'});reseal(p);assert.throws(()=>check(p),/question count/);
});
test('64 ordered excerpts fit and 65 deny',()=>{
  const p=example();p.scope.excerpts=Array.from({length:64},(_,i)=>{const e=excerpt(p);e.reference.id=`e-${String(i).padStart(2,'0')}`;return e;});
  reseal(p);assert.doesNotThrow(()=>check(p));p.scope.excerpts.push({...excerpt(p),reference:{...excerpt(p).reference,id:'e-64'}});reseal(p);assert.throws(()=>check(p),/excerpt count/);
});
for(const name of ['duplicate','unsorted'])test(`${name} excerpt reference IDs deny`,()=>{
  const p=example(),e=excerpt(p),other=structuredClone(e);other.reference.id=name==='duplicate'?'e-a':'a';p.scope.excerpts=[e,other];reseal(p);assert.throws(()=>check(p),/sorted unique/);
});
test('1 KiB whole reference fits and one additional path byte denies',()=>{
  const p=example();p.captures[0].path='a';p.scope.excerpts=[excerpt(p)];
  const padding=1024-Buffer.byteLength(canonical(p.scope.excerpts[0].reference));
  p.captures[0].path+='a'.repeat(padding);p.scope.excerpts=[excerpt(p)];reseal(p);assert.doesNotThrow(()=>check(p));
  p.captures[0].path+='a';p.scope.excerpts=[excerpt(p)];reseal(p);assert.throws(()=>check(p),/reference bytes/);
});
test('32 KiB whole scope fits and one more byte denies before any truncation',()=>{
  const p=example();p.scope.questions=Array.from({length:8},(_,i)=>({...structuredClone(p.scope.questions[0]),questionId:`q-${i}`}));reseal(p);
  for(const q of p.scope.questions)for(const k of ['questionText','knownInputs','expectedEvidence']){
    const remaining=32768-Buffer.byteLength(canonical(p.scope));
    if(remaining>0)q[k]+='x'.repeat(Math.min(remaining,2048-Buffer.byteLength(q[k])));
  }
  reseal(p);assert.equal(Buffer.byteLength(canonical(p.scope)),32768);assert.doesNotThrow(()=>check(p));
  p.scope.questions[7].uncertainty+='x';reseal(p);assert.throws(()=>check(p),/scope bytes/);
});
test('registered SHA-256 object format binds a full 64-digit base revision',()=>{
  const p=example();p.context.objectFormat='sha256';p.context.baseCommit='c'.repeat(64);p.captures[0].revision=p.context.baseCommit;reseal(p);assert.doesNotThrow(()=>check(p));
});
