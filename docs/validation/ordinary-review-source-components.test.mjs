import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { encodeRegistry, decodeRegistry, bindingMatches, newModel, reserve, dispatch, enterCreator, link, settle, restart, resolveEvidence } from './ordinary-review-source-components.mjs';
const vectors = JSON.parse(readFileSync(new URL('./ordinary-review-source-vectors.json', import.meta.url)));
const clone = value => structuredClone(value);
const workspace = vectors.registry.workspaceId;
const attempt = () => clone(vectors.registry.attempts[0]);
const registry = () => clone(vectors.registry);
const bytes = value => Buffer.from(JSON.stringify(value));
const decode = value => decodeRegistry(bytes(value), workspace);
const nextAttempt = (n=2, source='source_2') => ({...attempt(), attemptId:`${String(n).padStart(8,'0')}-1111-4111-8111-111111111111`, allocatedSessionId:`${String(n).padStart(8,'0')}-3333-4333-8333-333333333333`, sourceSessionId:source});

test('independent compact golden bytes and strict round trip', () => {
  assert.equal(encodeRegistry(registry(), workspace).toString(), vectors.canonical);
  assert.deepEqual(decodeRegistry(Buffer.from(vectors.canonical), workspace), registry());
});
for (const [name, change] of Object.entries({
  owner: b => b.owner='other-owner', generation: b => b.generation='1',
  pin: b => b.pinned=true, selection: b => b.selectionId='selection-B',
  branch: b => b.branch='other', kind: b => b.kind='spec', anchor: b => b.anchor='other',
  exhaustion: b => b.generation='18446744073709551615', malformed: b => b.generation='00',
})) test(`artifact binding denies ${name}`, () => {
  const current=clone(vectors.binding); change(current);
  assert.equal(bindingMatches(vectors.binding,current),false);
});
test('zero epoch is valid, unrelated publication invalidates, unobserved history is not invented', () => {
  assert.equal(bindingMatches(vectors.binding,clone(vectors.binding)),true);
  assert.equal(bindingMatches(vectors.binding,{...vectors.binding,generation:'1'}),false);
  assert.equal(bindingMatches({...vectors.binding,generation:'18446744073709551614'}, {...vectors.binding,generation:'18446744073709551614'}),true);
  assert.equal(bindingMatches({...vectors.binding,generation:'18446744073709551615'}, {...vectors.binding,generation:'18446744073709551615'}),false);
});
for (const [name, change] of Object.entries({
  unknown: r => r.extra=1, schema: r => r.schemaVersion=2, workspace: r => r.workspaceId='c'.repeat(64),
  zero: r => r.revision='0', overflow: r => r.revision='18446744073709551616', numeric: r => r.revision=1,
  leading: r => r.attempts[0].proposalRevision='01', uuid: r => r.attempts[0].attemptId='not-uuid',
  uuidVersion: r => r.attempts[0].previewId='22222222-2222-1222-8222-222222222222',
  uppercase: r => r.attempts[0].allocatedSessionId=r.attempts[0].allocatedSessionId.toUpperCase().replace('3333','ABCD'),
  opaque: r => r.attempts[0].sourceSessionId='a/b', idTooLong: r => r.attempts[0].recommendationId='x'.repeat(129),
  digest: r => r.attempts[0].preparedInputDigest='B'.repeat(64), state: r => r.attempts[0].state='running', stateArray: r => r.attempts[0].state=['reserved'],
  nullReason: r => r.attempts[0].state='unknown', badReason: r => r.attempts[0].reason='creation_failed',
  missing: r => delete r.attempts[0].recordRevision, duplicate: r => r.attempts.push(attempt()),
  sessionCollision: r => r.attempts.push({...nextAttempt(),allocatedSessionId:attempt().allocatedSessionId}),
})) test(`closed codec rejects ${name}`, () => { const r=registry(); change(r); assert.throws(()=>decode(r)); assert.throws(()=>encodeRegistry(r,workspace)); });
for (const [name, raw] of Object.entries({
  duplicate: vectors.canonical.replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'),
  whitespace: vectors.canonical+'\n', reordered: JSON.stringify({workspaceId:workspace,schemaVersion:1,revision:'1',attempts:[attempt()]}),
  escaped: vectors.canonical.replace('schemaVersion','schema\\u0056ersion'),
  bom: '\ufeff'+vectors.canonical, malformedUTF8: Buffer.from([0xff]), rawCap: Buffer.alloc(2097153,32),
})) test(`canonical decoder rejects ${name}`,()=>assert.throws(()=>decodeRegistry(Buffer.from(raw),workspace)));
test('maximum valid fields/counters and 256 records; 257 and overflow denied',()=>{
 const r=registry(); r.revision='18446744073709551615';
 r.attempts=Array.from({length:256},(_,i)=>({...nextAttempt(i+1),recommendationId:'x'.repeat(128),sourceSessionId:'s'.repeat(125)+String(i).padStart(3,'0'),proposalRevision:r.revision,recordRevision:r.revision}));
 assert.deepEqual(decodeRegistry(encodeRegistry(r,workspace),workspace),r);
 r.attempts.push(nextAttempt(257)); assert.throws(()=>encodeRegistry(r,workspace));
});
for(const [state,reasons] of Object.entries({reserved:[null],dispatchCommitted:[null],linked:[null],settled:[null],rejectedBeforeCreate:['stale','expired','source_unavailable','configuration_unavailable','creation_failed'],unknown:['storage_uncertain','creation_uncertain','source_owner_lost']}))
 test(`state/reason whitelist ${state}`,()=>{ for(const reason of [null,'stale','expired','source_unavailable','configuration_unavailable','creation_failed','storage_uncertain','creation_uncertain','source_owner_lost','arbitrary']) {const r=registry(); Object.assign(r.attempts[0],{state,reason}); if(reasons.includes(reason)) assert.doesNotThrow(()=>decode(r)); else assert.throws(()=>decode(r));} });

for(const trace of vectors.traces) test(`literal trace: ${trace.name}`,()=>{
 let model=newModel(workspace),calls=0;
 for(let i=0;i<trace.steps.length;i++) {
  let out;
  switch(trace.steps[i]) {
   case 'reserve':out=reserve(model,attempt(),{previewValid:true});break;
   case 'dispatch':out=dispatch(model,attempt().attemptId);break;
   case 'enter':out=enterCreator(model,attempt().attemptId,{validation:'valid'},()=>calls++);break;
   case 'stale':out=enterCreator(model,attempt().attemptId,{validation:'stale'},()=>calls++);break;
   case 'link':out=link(model,attempt().attemptId);break;
   case 'settle':out=settle(model,attempt().attemptId,{qualified:true,sessionId:attempt().allocatedSessionId,disposition:'settled'});break;
   case 'restart':out=restart(model);break;
   case 'retry':out=reserve(model,attempt(),{previewValid:false});break;
  }
  model=out.model;
  assert.equal(model.registry.attempts[0].state,trace.states[i]);
  assert.equal(model.registry.attempts[0].recordRevision,trace.revisions[i]);
  assert.equal(model.registry.revision,trace.registryRevisions[i]);
  assert.equal(calls,trace.calls[i]);
 }
});

const start = () => reserve(newModel(workspace),attempt(),{previewValid:true}).model;
const committed = () => dispatch(start(),attempt().attemptId).model;
const entered = (callback=()=>{}) => enterCreator(committed(),attempt().attemptId,{validation:'valid'},callback).model;
const linked = () => link(entered(),attempt().attemptId).model;
const evidence = disposition => ({qualified:true,sessionId:attempt().allocatedSessionId,disposition});
test('serialized repeated Starts preserve original tuple/id and invoke once',()=>{
 let m=start(),calls=0;
 const before=clone(m);
 assert.equal(reserve(m,attempt(),{previewValid:false}).result,'replay');
 assert.deepEqual(m,before);
 m=dispatch(m,attempt().attemptId).model;
 assert.equal(dispatch(m,attempt().attemptId).result,'unavailable');
 m=enterCreator(m,attempt().attemptId,{validation:'valid'},id=>{assert.equal(id,attempt().allocatedSessionId);calls++;}).model;
 m=enterCreator(m,attempt().attemptId,{validation:'valid'},()=>calls++).model;
 m=link(m,attempt().attemptId).model;
 assert.equal(reserve(m,attempt(),{previewValid:false}).result,'replay');
 assert.equal(calls,1);
});
test('replay tuple checked before preview; draft/source replacements cannot replace retained record',()=>{
 const m=linked(), original=clone(m);
 for(const key of ['recommendationId','previewId']) {const a=attempt(); a[key]=key==='previewId'?nextAttempt().attemptId:'different';assert.equal(reserve(m,a,{previewValid:true}).result,'conflict');}
 const a={...attempt(),preparedInputDigest:'c'.repeat(64),sourceSessionId:'other',proposalRevision:'2'};
 assert.equal(reserve(m,a,{previewValid:false}).result,'replay');
 assert.deepEqual(m,original);
});
for(const stage of ['reserved','dispatchCommitted','linked','unknown']) test(`occupied source survives new recommendation at ${stage}`,()=>{
 let m=stage==='reserved'?start():stage==='dispatchCommitted'?committed():stage==='linked'?linked():restart(committed()).model;
 const a={...nextAttempt(2,'source_1'),recommendationId:'new_rec',proposalRevision:'2'};
 assert.equal(reserve(m,a,{previewValid:true}).result,'conflict');
});
test('known rejection and qualified settlement alone release source slot',()=>{
 let m=enterCreator(committed(),attempt().attemptId,{validation:'stale'},()=>assert.fail('must not enter')).model;
 assert.equal(m.registry.attempts[0].state,'rejectedBeforeCreate');
 assert.equal(reserve(m,nextAttempt(2,'source_1'),{previewValid:true}).result,'published');
 m=settle(linked(),attempt().attemptId,evidence('settled')).model;
 assert.equal(reserve(m,nextAttempt(2,'source_1'),{previewValid:true}).result,'published');
 assert.equal(dispatch(m,attempt().attemptId).result,'unavailable');
 assert.equal(link(m,attempt().attemptId).result,'unavailable');
});
for(const reason of ['stale','expired','source_unavailable','configuration_unavailable','creation_failed']) test(`late final validation denies ${reason} after durable dispatch`,()=>{
 let calls=0;
 const out=enterCreator(committed(),attempt().attemptId,{validation:reason},()=>calls++);
 assert.equal(out.model.registry.attempts[0].state,'rejectedBeforeCreate');
 assert.equal(out.model.registry.attempts[0].reason,reason);
 assert.equal(calls,0);
});
test('both source-change orders: before final check denies; after entry cannot undo admission',()=>{
 let calls=0;
 const denied=enterCreator(committed(),attempt().attemptId,{validation:'stale'},()=>calls++);
 assert.equal(denied.model.registry.attempts[0].state,'rejectedBeforeCreate');
 let m=enterCreator(committed(),attempt().attemptId,{validation:'valid'},()=>calls++).model;
 m=enterCreator(m,attempt().attemptId,{validation:'stale'},()=>calls++).model;
 assert.equal(link(m,attempt().attemptId).model.registry.attempts[0].state,'linked');
 assert.equal(calls,1);
});
for(const fault of ['before','after']) {
 for(const checkpoint of ['reserve','dispatch','link','reject','settle','recovery','evidence']) test(`uncertain ${checkpoint} publication ${fault} fences all fresh admission`,()=>{
  let m,out;
  switch(checkpoint) {
   case 'reserve':m=newModel(workspace);out=reserve(m,attempt(),{previewValid:true,fault});break;
   case 'dispatch':m=start();out=dispatch(m,attempt().attemptId,{fault});break;
   case 'link':m=entered();out=link(m,attempt().attemptId,{fault});break;
   case 'reject':m=committed();out=enterCreator(m,attempt().attemptId,{validation:'stale',fault},()=>assert.fail());break;
   case 'settle':m=linked();out=settle(m,attempt().attemptId,evidence('settled'),{fault});break;
   case 'recovery':m=committed();out=restart(m,{fault});break;
   case 'evidence':m=restart(committed()).model;out=resolveEvidence(m,attempt().attemptId,evidence('linked'),{fault});break;
  }
  assert.equal(out.result,'unavailable'); assert.equal(out.model.fenced,true);
  assert.equal(reserve(out.model,nextAttempt(),{previewValid:true}).result,'unavailable');
  assert.equal(reserve(out.model,attempt(),{previewValid:true}).result,'unavailable');
  assert.equal(restart(out.model).result,'unavailable');
  let calls=0; enterCreator(out.model,attempt().attemptId,{validation:'valid'},()=>calls++);assert.equal(calls,0);
 });
 test(`creator fault ${fault} never creates on replay`,()=>{
  let calls=0,m=enterCreator(committed(),attempt().attemptId,{fault,validation:'valid'},()=>calls++).model;
  assert.equal(calls,fault==='before'?0:1);
  assert.equal(m.registry.attempts[0].state,'unknown');
  assert.equal(m.registry.attempts[0].reason,'creation_uncertain');
  m=reserve(m,attempt(),{previewValid:false}).model;
  enterCreator(m,attempt().attemptId,{validation:'valid'},()=>calls++);
  assert.equal(calls,fault==='before'?0:1);
 });
}
test('throwing creator is uncertain, never pre-entry rejection',()=>{
 const m=enterCreator(committed(),attempt().attemptId,{validation:'valid'},()=>{throw Error('synthetic');}).model;
 assert.equal(m.registry.attempts[0].state,'unknown');
 assert.equal(m.registry.attempts[0].reason,'creation_uncertain');
});
test('unqualified/wrong identity evidence cannot resolve unknown; qualified evidence is forward only',()=>{
 const m=restart(committed()).model;
 for(const bad of [{...evidence('linked'),qualified:false},{...evidence('linked'),sessionId:nextAttempt().allocatedSessionId},{...evidence('linked'),disposition:'reserved'},{title:'same',path:'same'},true])
  assert.equal(resolveEvidence(m,attempt().attemptId,bad).result,'unavailable');
 for(const disposition of ['linked','settled']) {
  const out=resolveEvidence(m,attempt().attemptId,evidence(disposition));
  assert.equal(out.model.registry.attempts[0].state,disposition);
  assert.equal(out.model.registry.attempts[0].recordRevision,'4');
 }
 assert.equal(settle(linked(),attempt().attemptId,{...evidence('settled'),qualified:false}).result,'unavailable');
});
test('restart loses invocation owner, retains linked identity, and never creates',()=>{
 for(const initial of [start(),committed(),entered()]) {
  const m=restart(initial).model;assert.equal(m.registry.attempts[0].state,'unknown');
  assert.equal(dispatch(m,attempt().attemptId).result,'unavailable');
  assert.equal(reserve(m,attempt(),{previewValid:false}).result,'replay');
 }
 const m=restart(linked()).model;assert.equal(m.registry.attempts[0].state,'linked');assert.deepEqual(m.live,[]);
});
test('capacity and exhausted counters deny new writes while exact retry remains readable',()=>{
 let m=newModel(workspace);m.registry=registry();
 m.registry.attempts=Array.from({length:256},(_,i)=>({...nextAttempt(i+1),state:'settled'}));
 assert.equal(reserve(m,nextAttempt(257),{previewValid:true}).result,'unavailable');
 assert.equal(reserve(m,m.registry.attempts[0],{previewValid:false}).result,'replay');
 m=start();m.registry.revision='18446744073709551615';assert.equal(dispatch(m,attempt().attemptId).result,'unavailable');
 assert.equal(reserve(m,nextAttempt(),{previewValid:true}).result,'unavailable');
 m=start();m.registry.attempts[0].recordRevision='18446744073709551615';assert.equal(dispatch(m,attempt().attemptId).result,'unavailable');
});
test('operations never mutate their input registry or owner flags',()=>{
 for(const [m,operation] of [[start(),m=>dispatch(m,attempt().attemptId)],[committed(),m=>enterCreator(m,attempt().attemptId,{validation:'valid'},()=>{})],[entered(),m=>link(m,attempt().attemptId)],[linked(),m=>settle(m,attempt().attemptId,evidence('settled'))]]) {
  const before=clone(m);operation(m);assert.deepEqual(m,before);
 }
});

test('no creator before dispatch and no link before creator entry',()=>{
 let calls=0;
 assert.equal(enterCreator(start(),attempt().attemptId,{validation:'valid'},()=>calls++).result,'unavailable');
 assert.equal(link(committed(),attempt().attemptId).result,'unavailable');
 assert.equal(calls,0);
});
test('uncertain publication preserves only the selected before/after snapshot',()=>{
 for(const fault of ['before','after']) {
  const out=dispatch(start(),attempt().attemptId,{fault});
  assert.equal(out.model.registry.attempts[0].state,fault==='before'?'reserved':'dispatchCommitted');
  assert.equal(out.model.registry.attempts[0].recordRevision,fault==='before'?'1':'2');
 }
});

test('closed maximum record is 701 bytes and 256-record envelope is 179859 bytes',()=>{
 const r=registry();r.revision='18446744073709551615';
 const a={...attempt(),recommendationId:'r'.repeat(128),sourceSessionId:'s'.repeat(128),proposalRevision:r.revision,recordRevision:r.revision,state:'rejectedBeforeCreate',reason:'configuration_unavailable'};
 assert.equal(Buffer.byteLength(JSON.stringify(a)),701);
 r.attempts=Array.from({length:256},(_,i)=>({...a,attemptId:`${String(i).padStart(8,'0')}-1111-4111-8111-111111111111`,allocatedSessionId:`${String(i).padStart(8,'0')}-3333-4333-8333-333333333333`}));
 assert.equal(encodeRegistry(r,workspace).length,179859);
});
test('observed pin/recovery A-B-A invalidates the original epoch despite equal final selection',()=>{
 const original=clone(vectors.binding),after={...original,generation:'2'};
 assert.equal(after.selectionId,original.selectionId);
 assert.equal(bindingMatches(original,after),false);
});

test('encoder rejects sparse attempts rather than emitting null records',()=>{
 const r=registry();r.attempts=Array(1);assert.throws(()=>encodeRegistry(r,workspace));
});
test('only literal true is a valid simulated inspected preview',()=>{
 for(const previewValid of ['false','true',[],{validation:'valid'},1,null]) assert.equal(reserve(newModel(workspace),attempt(),{previewValid}).result,'unavailable');
});
test('exhausted final disposition consumes live permission, never permits a later creator',()=>{
 for(const validation of ['stale','valid']) for(const fault of [undefined,'before']) {
  let m=start(),calls=0;m.registry.revision='18446744073709551614';
  m=dispatch(m,attempt().attemptId).model;
  m=enterCreator(m,attempt().attemptId,{validation,fault},()=>calls++).model;
  m=enterCreator(m,attempt().attemptId,{validation:'valid'},()=>calls++).model;
  assert.equal(m.fenced,true);assert.equal(calls,0);
 }
});

test('missing or malformed final validation never enters the creator',()=>{
 for(const validation of [undefined,null,true,[],{},'unknown']) {
  let calls=0;
  assert.equal(enterCreator(committed(),attempt().attemptId,{validation},()=>calls++).result,'unavailable');
  assert.equal(calls,0);
 }
});

test('closed identifiers, workspace and counters reject trailing line terminators',()=>{
 for(const suffix of ['\n','\r','\r\n']) {
  for(const key of ['attemptId','previewId','allocatedSessionId','recommendationId','sourceSessionId','proposalRevision','recordRevision','preparedInputDigest']) {
   const r=registry();r.attempts[0][key]+=suffix;assert.throws(()=>encodeRegistry(r,workspace),key);
  }
  const r=registry();r.revision+=suffix;assert.throws(()=>encodeRegistry(r,workspace));
  r.revision='1';r.workspaceId+=suffix;assert.throws(()=>encodeRegistry(r,r.workspaceId));
  assert.equal(bindingMatches({...vectors.binding,generation:'1'+suffix},{...vectors.binding,generation:'1'+suffix}),false);
 }
});
