// Offline documentation components only; never import into runtime.
// Supplied context is a synthetic assumption, not retained producer authority.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

const sha = x => createHash('sha256').update(x).digest('hex');
const order = (a,b) => Buffer.compare(Buffer.from(a),Buffer.from(b));
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
function exact(x, fields) {
  assert(object(x), 'record object');
  assert.deepEqual(Object.keys(x).sort(),fields.split(' ').sort(),'closed fields');
}
function text(x, max=2048) {
  assert(typeof x === 'string' && x.isWellFormed(),'well-formed UTF-8');
  assert(Buffer.byteLength(x)>0 && Buffer.byteLength(x)<=max,'bounded text');
}
const id = x => assert(typeof x==='string' && /^[A-Za-z0-9._-]{1,128}$/.test(x),'ASCII ID');
const digest = x => assert(typeof x==='string' && /^[0-9a-f]{64}$/.test(x),'lowercase SHA-256');
const integer = (x,min,max,label='integer') => assert(Number.isSafeInteger(x) && !Object.is(x,-0) && x>=min && x<=max,label);
function canonical(x) {
  if(x===null || typeof x==='boolean')return JSON.stringify(x);
  if(typeof x==='string') { assert(x.isWellFormed(),'well-formed UTF-8');return JSON.stringify(x); }
  if(typeof x==='number') { integer(x,-Number.MAX_SAFE_INTEGER,Number.MAX_SAFE_INTEGER);return JSON.stringify(x); }
  if(Array.isArray(x))return '['+x.map(canonical).join(',')+']';
  assert(object(x),'canonical object');
  return '{'+Object.keys(x).sort(order).map(k=>canonical(k)+':'+canonical(x[k])).join(',')+'}';
}
const bytes = x => Buffer.byteLength(canonical(x));
function sealed(x,field,domain,label) {
  digest(x[field]);const body={...x};delete body[field];
  assert.equal(x[field],sha(domain+'\n'+canonical(body)),label);
}
function count(xs,min,max,label) { assert(Array.isArray(xs) && xs.length>=min && xs.length<=max,label); }
function sorted(xs) {
  assert.equal(new Set(xs).size,xs.length,'sorted unique');
  assert.deepEqual(xs,[...xs].sort(order),'sorted unique');
}
function task(x) {
  exact(x,'workspaceId runId planId planRevision taskId');
  for(const k of ['workspaceId','runId','planId','taskId'])id(x[k]);
  const run=/^[0-9a-f]{64}-([1-9][0-9]{0,18})$/.exec(x.runId);
  assert(run && BigInt(run[1])<=9223372036854775807n,'run identity');
  integer(x.planRevision,1,64);
}
function bindings(x,context) {
  task(x.task);assert.deepEqual(x.task,context.task,'task binding');
  digest(x.repositoryBindingDigest);digest(x.sourceWorktreeBindingDigest);
  assert.equal(x.repositoryBindingDigest,context.repositoryBindingDigest,'repository binding');
  assert.equal(x.sourceWorktreeBindingDigest,context.sourceWorktreeBindingDigest,'worktree binding');
}
function path(x) {
  text(x);
  assert(!x.includes('\\') && !x.includes('\0') && !/^[A-Za-z]:/.test(x)
    && x.split('/').every(c=>c!=='' && c!=='.' && c!=='..'),'relative path');
}
function capture(x,context) {
  exact(x,'schemaVersion task sourceId repositoryBindingDigest sourceWorktreeBindingDigest path revision content sizeBytes contentDigest captureDigest');
  assert.equal(x.schemaVersion,1);bindings(x,context);id(x.sourceId);path(x.path);
  assert.equal(x.revision,context.baseCommit,'base commit');
  assert(typeof x.content==='string' && x.content.isWellFormed(),'well-formed UTF-8');
  const raw=Buffer.from(x.content);
  assert(raw.length>=1 && raw.length<=65536 && !raw.includes(0),'capture content');
  integer(x.sizeBytes,1,65536,'raw size');assert.equal(x.sizeBytes,raw.length,'raw size');
  digest(x.contentDigest);assert.equal(x.contentDigest,sha(raw),'content digest');
  sealed(x,'captureDigest','orkworks.research-source-capture.v1','capture digest');
  assert(bytes(x)<=512*1024,'capture body bytes');
  return raw;
}
function question(x,map) {
  exact(x,'questionId questionText sourceIds knownInputs expectedEvidence stoppingConditions uncertainty');
  id(x.questionId);
  for(const k of ['questionText','knownInputs','expectedEvidence','stoppingConditions','uncertainty'])text(x[k]);
  count(x.sourceIds,1,16,'question source count');
  for(const source of x.sourceIds){id(source);assert(map.has(source),'capture reference');}
  sorted(x.sourceIds);assert(bytes(x)<=8192,'question bytes');
}
function excerpt(x,map,raws,used) {
  exact(x,'reference sourceId startByte lengthBytes');id(x.sourceId);
  assert(map.has(x.sourceId) && used.has(x.sourceId),'excerpt source');
  const c=map.get(x.sourceId),raw=raws.get(x.sourceId),r=x.reference;
  exact(r,'kind id path revision location excerptDigest');assert.equal(r.kind,'file','file reference');id(r.id);
  path(r.path);text(r.revision);text(r.location);digest(r.excerptDigest);
  assert.equal(r.path,c.path,'excerpt path');assert.equal(r.revision,c.revision,'excerpt revision');
  integer(x.startByte,0,Number.MAX_SAFE_INTEGER,'excerpt start');
  integer(x.lengthBytes,1,2048,'excerpt length');
  const end=x.startByte+x.lengthBytes;
  assert(Number.isSafeInteger(end) && end<=raw.length,'slice bounds');
  const selected=raw.subarray(x.startByte,end);
  assert.doesNotThrow(()=>new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(selected),'UTF-8 slice');
  assert.equal(r.location,`bytes:${x.startByte}-${end}`,'exact byte locator');
  assert.equal(r.excerptDigest,sha(selected),'excerpt digest');
  assert(bytes(r)<=1024,'reference bytes');
}

// This byte boundary rejects duplicate members, alternative JSON spellings,
// unsafe numbers, malformed UTF-8 and BOMs by canonical round trip.
export function decodeResearchComponent(input) {
  const wire=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(input);
  const value=JSON.parse(wire);
  assert.equal(canonical(value),wire,'canonical research component bytes');
  return value;
}

export function validateResearchScope(captures,scope,context) {
  exact(context,'task repositoryBindingDigest sourceWorktreeBindingDigest baseCommit objectFormat');
  task(context.task);digest(context.repositoryBindingDigest);digest(context.sourceWorktreeBindingDigest);
  assert(['sha1','sha256'].includes(context.objectFormat),'object format');
  assert(typeof context.baseCommit==='string' && (context.objectFormat==='sha1'?/^[0-9a-f]{40}$/:/^[0-9a-f]{64}$/).test(context.baseCommit),'object format');
  count(captures,1,16,'capture count');
  const map=new Map(),raws=new Map();let rawCaptureBytes=0;
  for(const c of captures){
    const raw=capture(c,context);assert(!map.has(c.sourceId),'unique capture ID');
    map.set(c.sourceId,c);raws.set(c.sourceId,raw);rawCaptureBytes+=raw.length;
  }
  sorted(captures.map(c=>c.sourceId));assert(rawCaptureBytes<=1024*1024,'aggregate raw bytes');
  exact(scope,'schemaVersion task repositoryBindingDigest sourceWorktreeBindingDigest captures questions excerpts scopeDigest');
  assert.equal(scope.schemaVersion,1);bindings(scope,context);
  assert.deepEqual(scope.captures,captures.map(c=>({task:c.task,sourceId:c.sourceId,captureDigest:c.captureDigest})),'exact capture references');
  count(scope.questions,1,32,'question count');
  for(const q of scope.questions)question(q,map);
  sorted(scope.questions.map(q=>q.questionId));
  const used=new Set(scope.questions.flatMap(q=>q.sourceIds));
  assert.equal(used.size,map.size,'every capture used');
  count(scope.excerpts,0,64,'excerpt count');
  for(const e of scope.excerpts)excerpt(e,map,raws,used);
  sorted(scope.excerpts.map(e=>e.reference.id));
  sealed(scope,'scopeDigest','orkworks.research-task-scope.v1','scope digest');
  assert(bytes(scope)<=32768,'scope bytes');
  return {rawCaptureBytes,captureBodyBytes:captures.reduce((n,c)=>n+bytes(c),0),scopeBodyBytes:bytes(scope)};
}
