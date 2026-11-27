// Direct execution intentionally avoids a test-runner child.
import assert from 'node:assert/strict';
const protocol = await import('./pair-reader-diagnostic-protocol.mjs').catch(e => {
  if (e.code === 'ERR_MODULE_NOT_FOUND') return {};
  throw e;
});
assert.equal(typeof protocol.Ledger, 'function', 'shared bounded ledger is missing');
const { Ledger, Decoder, settle, encodeResult } = protocol;
const nonce = '0123456789abcdef0123456789abcdef';
const ledger = new Ledger();
let vectors = 0;
function vector(name, fn) {
  ledger.beginVector();
  fn();
  vectors++;
}
const start = `[1,"${nonce}",1,"start","",0,0,0,0]\n`;
const ready = `[1,"${nonce}",2,"ready","",7,0,0,0]\n`;
const ok = `[1,"${nonce}",3,"ok","",7,0,0,0]\n`;
const parser = (mode = 'guards', grant = 1024) => new Decoder({nonce, mode, grant, ledger, source:'injected'});
vector('chunk boundaries and immutable prefix', () => {
  const p = parser();
  for (const b of Buffer.from(start + ready + ok)) p.feed(Buffer.of(b));
  assert.equal(p.trace.length, 3);
  assert.equal(p.trace[0][1][5], 0);
  assert.ok(Object.isFrozen(p.trace[0][1]));
  assert.equal(p.end(), true);
  assert.equal(settle(p, {code:0, drained:true, outputValid:true}).charged, 7);
});
const bad = [
 start.replace(nonce, 'f'.repeat(32)), start.replace(',1,"start"', ',2,"start"'),
 start.replace('"start"', '"ready"'), start.replace('"start"', '"foreign"'),
 start.replace('[1,', '[2,'), start.replace(',0,0,0,0]', ',0,0,0,0,0]'),
 start.replace(',0,0,0,0]', ',0,0,0]'), start.replace(',0,0,0,0]', ',"0",0,0,0]'),
 start.replace(',0,0,0,0]', ',-0,0,0,0]'), start.replace(',0,0,0,0]', ',1e0,0,0,0]'),
 start.replace(',0,0,0,0]', ',0.5,0,0,0]'), start.replace(',0,0,0,0]', ',9007199254740992,0,0,0]'),
 start.replace('start', 'sta\\u0072t'), ' ' + start, '\n' + start,
 '\ufeff' + start, start.replace('start', 'stárt'), start.replaceAll('\n','\r\n'),
 start.replace('"",0', '"msvc",0'), start.replace(',0,0,0,0]', ',1025,0,0,0]'),
 start.replace(',0,0,0,0]', ',0,2048,0,1]'), start.replace(',0,0,0,0]', ',0,0,255,1]'),
 start.replace(',0,0,0,0]', ',0,0,0,2]'), start.replace(',0,0,0,0]', ',0,1,0,0]'),
 start + start, start + ready + ok + ' ', start + ready + ok + start,
 start + ready.replace(',7,', ',6,') + ok.replace(',7,', ',5,'),
 start+`[1,"${nonce}",2,"error","",0,0,0,0,"foreign"]\n`,
 start+`[1,"${nonce}",2,"error","msvc",0,0,0,0,"hash-limit"]\n`
];
for (const [i, bytes] of bad.entries()) vector(`refuse malformed ${i}`, () => {
  const p = parser(); p.feed(Buffer.from(bytes));
  assert.equal(p.failure !== null, true, `accepted malformed ${i}`);
  const count = p.trace.length; p.feed(Buffer.from(ok)); assert.equal(p.trace.length, count);
  assert.equal(settle(p, {code:0,drained:true,outputValid:true}).charged, 1024);
});
vector('partial EOF keeps only complete prefix', () => {
 const p=parser(); p.feed(Buffer.from(start + ready.slice(0, 11)));
 assert.equal(p.end(),false); assert.equal(p.trace.length,1); assert.equal(p.tailBytes,11);
});
vector('191 unframed bytes and report cap refuse without retaining raw tail', () => {
 const p=parser(); p.feed(Buffer.alloc(192,65)); assert.equal(p.failure,'line-limit'); assert.equal(p.tail.length,0);
 const q=parser(); q.feed(Buffer.alloc(4097,65)); assert.equal(q.failure,'report-limit');
});
vector('all native phases and output admission', () => {
 const phases=[['start',''],['ready',''],['archives-begin',''],['archives-end',''],
 ...['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'].flatMap(r=>[['root-begin',r],['root-end',r]]),
 ['serialize-begin',''],['serialize-end',''],['ok','']];
 const p=parser('reference');
 p.feed(Buffer.from(phases.map(([phase,root],i)=>JSON.stringify([1,nonce,i+1,phase,root,i,0,0,0])+'\n').join('')));
 assert.equal(p.end(),true); assert.equal(p.trace.length,21);
 assert.equal(settle(p,{code:0,drained:true,outputValid:true}).reader_complete,true);
 assert.equal(settle(p,{code:0,drained:true,outputValid:false}).reader_complete,false);
 assert.equal(settle(p,{code:0,drained:true,outputValid:false}).charged,1024);
});
vector('wrong root fails at last valid boundary', () => {
 const p=parser('reuse');
 const rows=[['start',''],['ready',''],['archives-begin',''],['archives-end',''],['root-begin','registry']];
 p.feed(Buffer.from(rows.map(([x,r],i)=>JSON.stringify([1,nonce,i+1,x,r,0,0,0,0])+'\n').join('')));
 assert.equal(p.failure,'phase-order'); assert.equal(p.trace.length,4);
});
vector('cooperative error settles h only after correct close', () => {
 const p=parser(); p.feed(Buffer.from(start+`[1,"${nonce}",2,"error","",9,0,0,0,"hash-limit"]\n`));
 assert.equal(settle(p,{code:1,drained:true}).charged,9);
 assert.equal(settle(p,{code:0,drained:true}).charged,1024);
 assert.equal(settle(p,{code:1,drained:false}).charged,1024);
});
for (const kind of ['timeout-before-start','timeout-mid-record','timeout-after-checkpoint','timeout-after-ok','spawn-error','nonzero-close','undrained','stderr','stream-overflow']) vector(kind,()=>{
 const p=parser();
 if(kind!=='timeout-before-start' && kind!=='spawn-error') p.feed(Buffer.from(start));
 if(kind==='timeout-mid-record') p.feed(Buffer.from(ready.slice(0,8)));
 if(kind==='timeout-after-ok') p.feed(Buffer.from(ready+ok));
 p.fail(kind);
 const s=settle(p,{code:kind==='nonzero-close'?2:0,drained:kind!=='undrained',outputValid:true});
 assert.equal(s.charged,1024); assert.equal(s.reader_complete,false);
 assert.ok(p.trace.length <= (kind==='timeout-after-ok'?3:1));
});
vector('unsealed cooperative work never refunds the full grant',()=>{
 const p=parser();p.feed(Buffer.from(start+`[1,"${nonce}",2,"error","",9,0,0,0,"hash-limit"]\n`));
 assert.equal(settle(p,{code:1,drained:true,sourceValid:false}).charged,1024);
});
vector('zero grant transport accepts only its exact start',()=>{
 const p=new Decoder({nonce,mode:'transport',grant:0,ledger,source:'injected'});
 p.feed(Buffer.from(start)); assert.equal(p.trace.length,1);
 p.feed(Buffer.from(ready.replace(',7,', ',0,'))); assert.equal(p.failure,'phase-order');
});
vector('shared hash reservations cannot refund a checkpoint',()=>{
 const r=ledger.reserve(1024,true); const p=parser(); p.feed(Buffer.from(start)); p.fail('timeout');
 const s=settle(p,{code:null,drained:true}); ledger.settle(r,s.charged,true,true);
 assert.equal(s.charged,1024); assert.throws(()=>ledger.settle(r,0),/settlement/);
 assert.throws(()=>ledger.chargeHash(64*1024*1024),/hash-limit/);
});
vector('trace/result admission refuses oversize rather than truncating',()=>{
 assert.throws(()=>encodeResult({traces:['x'.repeat(18433)]}),/trace-limit/);
 assert.throws(()=>encodeResult({traces:[],extra:'x'.repeat(65537)}),/result-limit/);
});
assert.ok(vectors<=64); assert.ok(ledger.injectedBytes<=512*1024);

const controller = await import('./pair-reader-diagnostic.mjs').catch(e=>{
 if(e.code==='ERR_MODULE_NOT_FOUND')return {}; throw e;
});
assert.equal(typeof controller.runSuite,'function','owned controller is missing');
assert.equal(typeof controller.watchChild,'function','event-driven child observer is missing');
assert.equal(typeof controller.admitAndSettle,'function','output admission must own reservation settlement');
vector('invalid output keeps full reserved grant',()=>{
 const key=ledger.reserve(1024,true);const p=parser();p.feed(Buffer.from(start+ready+ok));
 const before=ledger.hashBytes;
 assert.throws(()=>controller.admitAndSettle(p,{code:0,drained:true},ledger,key,()=>{throw Error('output-invalid');}),/output-invalid/);
 assert.equal(ledger.hashBytes-before,1024);
 assert.equal(ledger.reservations.has(key),false);
});

assert.equal(typeof controller.identityMatches,'function','parent volume/index binding is missing');
vector('volume and file index both bind parent identity',()=>{
 assert.equal(controller.identityMatches('00000001:0000000000000002',{dev:1n,ino:2n}),true);
 assert.equal(controller.identityMatches('00000002:0000000000000002',{dev:1n,ino:2n}),false);
 assert.equal(controller.identityMatches('00000001:0000000000000003',{dev:1n,ino:2n}),false);
});
vector('modeled cap boundaries grant no new work or input credit',()=>{
 // Arithmetic-only transport models: no allocation, stream injection, hash or child.
 // Actual vectors, bytes and model debit stay in the one surrounding ledger.
 ledger.chargeHash(64,true);
 const model=()=>Object.assign(Object.create(ledger),{hashBytes:0,reserved:0,childBytes:0,injectedBytes:0,vectorBytes:0,vectors:0,scratchBytes:0});
 let m=model();m.hashBytes=67108863;m.chargeHash(1);assert.equal(m.hashBytes,67108864);assert.throws(()=>m.chargeHash(1),/hash-limit/);
 m=model();m.scratchBytes=8388607;m.scratch(1);assert.throws(()=>m.scratch(1),/scratch-limit/);
 m=model();m.vectors=63;m.beginVector();assert.throws(()=>m.beginVector(),/vector-limit/);
 m=model();m.vectorBytes=8191;m.stream(1,'injected');assert.throws(()=>m.stream(1,'injected'),/stream-limit/);
 m=model();m.injectedBytes=524287;m.stream(1,'injected');assert.throws(()=>m.stream(1,'injected'),/stream-limit/);
 m=model();m.childBytes=1048575;m.stream(1,'child');assert.throws(()=>m.stream(1,'child'),/stream-limit/);
 m=model();m.started-=120000;assert.throws(()=>m.active(),/active-timeout/);
});
vector('encoded trace and result exact byte boundaries',()=>{
 assert.equal(Buffer.byteLength(JSON.stringify(['x'.repeat(18428)])),18432);
 assert.doesNotThrow(()=>encodeResult({traces:['x'.repeat(18428)]}));
 assert.throws(()=>encodeResult({traces:['x'.repeat(18429)]}),/trace-limit/);
 const base=Buffer.byteLength(JSON.stringify({traces:[],extra:''})+'\n');
 assert.equal(encodeResult({traces:[],extra:'x'.repeat(65536-base)}).length,65536);
 assert.throws(()=>encodeResult({traces:[],extra:'x'.repeat(65537-base)}),/result-limit/);
});
assert.equal(typeof controller.finalizeOwned,'function','failure publication/cleanup disposition is missing');
for(const kind of ['cleanup','publication','undrained']) vector('finalization '+kind,()=>{
 let removed=false;const result={tests_passed:true,traces:[]};
 controller.finalizeOwned(result,{drained:kind!=='undrained',owned:'owned-fixture',remove:()=>{removed=true;if(kind==='cleanup')throw Error('not-published');},publish:()=>{if(kind==='publication')throw Error('not-published');}});
 assert.equal(result.tests_passed,false);
 assert.equal(removed,kind!=='undrained');
 assert.equal(result.cleanup,kind==='cleanup'?'failed':kind==='undrained'?'retained-undrained':'removed');
 assert.equal(result.reason,kind==='publication'?'publication-failed':undefined);
});
const { EventEmitter }=await import('node:events');
const { PassThrough }=await import('node:stream');
for (const kind of ['clean','nonzero','spawn','stderr','overflow','timeout','undrained']) {
 ledger.beginVector();vectors++;
 const child=new EventEmitter();child.stdout=new PassThrough();child.stderr=new PassThrough();
 const p=parser();let terminated=0;
 const waiting=controller.watchChild(child,{decoder:p,ledger,timeout:5,terminate:async()=>{terminated++;return kind!=='undrained';}});
 if(kind==='clean'||kind==='nonzero') {child.stdout.write(start+ready+ok);child.emit('close',kind==='clean'?0:2);}
 else if(kind==='spawn') child.emit('error',Error('not-published'));
 else if(kind==='stderr') child.stderr.write('not-published');
 else if(kind==='overflow') child.stdout.write(Buffer.alloc(4097,65));
 else child.stdout.write(start);
 const close=await waiting;const outcome=settle(p,{...close,outputValid:true});
 assert.equal(outcome.reader_complete,kind==='clean');
 assert.equal(outcome.charged,kind==='clean'?7:1024);
 assert.equal(close.drained,kind!=='undrained');
 assert.equal(terminated,['stderr','overflow','timeout','undrained'].includes(kind)?1:0);
 if(kind==='timeout'||kind==='undrained')assert.equal(p.trace.length,1);
}

ledger.beginVector();vectors++;
{
 // Deadline crossing during spawn must retain ownership and drain the child.
 const late=Object.create(ledger);late.elapsed=()=>120000;late.active=()=>{throw Error('active-timeout');};late.stream=ledger.stream.bind(ledger);
 const child=new EventEmitter();child.stdout=new PassThrough();child.stderr=new PassThrough();
 const decoder=new Decoder({nonce,mode:'guards',grant:1024,ledger:late,source:'injected'});
 let terminated=0;
 const close=await controller.watchChild(child,{decoder,ledger:late,timeout:1,terminate:async()=>{terminated++;return true;}});
 assert.equal(close.drained,true);assert.equal(terminated,1);assert.equal(decoder.failure,'active-timeout');
}
assert.ok(vectors<=64);console.log(`Pure diagnostic vectors passed: ${vectors}`);
const result=await controller.runSuite(ledger,{pureVectors:vectors});
assert.equal(result.tests_passed,true,JSON.stringify(result));
assert.equal(result.candidate_evidence_complete,false);
assert.equal(result.runtime_qualified,false);
assert.equal(result.production_pin_accepted,false);
assert.equal(result.target_executions,0);
assert.equal(result.transport.reader_complete,false);
assert.equal(result.transport.reason,'timeout');
assert.equal(result.transport.drained,true);
assert.equal(result.transport.trace.length,1);
assert.deepEqual(result.transport.trace[0][1].slice(2),[1,'start','',0,0,0,0]);
assert.equal(result.native.status,process.platform==='win32'?'passed':'skipped');
if(process.platform==='win32') {
 assert.equal(result.native.equality,true);
 assert.match(result.native.powershell_version,/^7\./);
 assert.equal(result.native.reuse_allocations,1);
 assert.equal(result.native.reference_allocations,4123); // 3 archive hashes + 1027 members + 1027 extracted + 2*1033 manifest reads.
 assert.equal(result.native.maximum_leases,1);
 assert.equal(result.native.children,3);
 assert.equal(result.native.guard_vectors,32);
 assert.deepEqual(result.native.guard_dispositions,[
  'traversal:format-denied','absolute:format-denied','backslash:format-denied','ads:format-denied','empty:format-denied','dot:format-denied','double-slash:format-denied','nul:format-denied','trailing-dot:format-denied','reparse:input-denied','path-limit:format-denied','foreign-root:input-denied','unc:input-denied','file-limit:input-limit','short-stream:identity-drift','hash-limit:hash-limit','growth:identity-drift','nested-lease:fixture-assertion','encoded-limit:output-limit','wrong-path-identity:identity-drift','duplicate:input-limit','case-fold:input-limit','type:format-denied','tar-traversal:format-denied','extra:format-denied','content:identity-drift','missing:format-denied','wrong-archive:identity-drift','member-limit:input-limit','expanded-limit:input-limit','extra-file:format-denied','helper:format-denied'
 ]);
}
assert.equal(result.cleanup,'removed');
assert.ok(result.streams.injected_bytes+result.streams.child_bytes<=1048576);
assert.ok(result.work.charged_bytes<=67108864);
console.log(JSON.stringify(result));
export { ledger, vectors };
