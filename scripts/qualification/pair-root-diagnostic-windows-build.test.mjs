import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
const codecURL = new URL('./pair-root-diagnostic-windows-build-protocol.mjs', import.meta.url);
const nonce = 'a'.repeat(32);
let sharedPool;
const fixtureCounts={vectors:0,injected:0};
async function fixturePool(){if(!sharedPool){const {WorkBudget}=await import(codecURL);sharedPool=new WorkBudget({hashLimit:64*1024**2,streamLimit:1024**2,seconds:120});}return sharedPool;}
function injectedVector(bytes=Buffer.alloc(0)){assert.ok(++fixtureCounts.vectors<=64);assert.ok(bytes.length<=8192);fixtureCounts.injected+=bytes.length;assert.ok(fixtureCounts.injected<=512*1024);}


// Independent literal structural order, not the implementation's phase builder.
const structure = [[0,7],[1,7],[2,7],[3,7],[4,0],[5,0],[4,1],[5,1],[4,2],[5,2],[4,3],[5,3],[4,4],[5,4],[4,5],[5,5],[4,6],[5,6],[6,7],[7,7],[8,7]];
const samples = [[0,0],[0,0],[0,1],[0,1],[0,1],[0,1],[1,1],[16,1],[256,1],[4096,1],[65536,1]];
const literal = (seq,kind,root,h=100,f=0,d=0,a=0,detail=0) => [3,nonce,seq,kind,root,h,f,d,a,detail];
const wire = rows => Buffer.from(rows.map(x=>JSON.stringify(x)+'\n').join(''));
function complete(n=11) {
 const rows=[];let f=0,d=0;
 for(const [kind,root] of structure) {
  if(kind===5&&root===0) {
   for(let detail=1;detail<=n;detail++) { [f,d]=samples[detail-1];rows.push(literal(rows.length+1,10,0,100,f,d,0,detail)); }
   d=Math.max(d,1);
  }
  if(kind===5&&root>0)d++;
  rows.push(literal(rows.length+1,kind,root,100,f,d));
 }
 return rows;
}
function manifest(f=65536,d=1) {
 return {files:Array.from({length:f},(_,i)=>({root:'msvc',file:'f'+i})),directories:Array.from({length:d},(_,i)=>({root:'msvc',file:i?'d'+i:''}))};
}

test('v3 wire closes sparse samples without admitting missing work',async()=>{
 assert.ok(fs.existsSync(codecURL),'v3 codec behavior is not implemented');
 const {Decoder,WorkBudget,settle,encodeResult,parseStrict,boundedJSON}=await import(codecURL);
 let vectors=0,injected=0;const suite = await fixturePool();
 function decode(rows,{mode='before',bytes=wire(rows),now=0}={}) {
  assert.ok(++vectors<=64);injected+=bytes.length;injectedVector(bytes);
  const ledger={stream:n=>suite.stream(n),active:()=>{if(now>=2400000)throw Error('time-limit');},elapsed:()=>now};
  const d=new Decoder({nonce,mode,grant:1000,ledger});d.feed(bytes);return d;
 }
 const full=decode(complete());assert.equal(full.end(),true,full.failure);assert.equal(full.trace.length,32);
 assert.ok(Object.isFrozen(full.trace[5][1]));assert.equal(full.validateSamples(manifest()),true);
 assert.equal(settle(full,{code:0,drained:true,sourceValid:true,outputValid:true}).charged,100);
 for(const [f,d,n] of [[0,1,2],[0,2,3],[1,1,7],[15,1,7],[16,1,8],[255,1,8],[256,1,9],[4095,1,9],[4096,1,10],[65535,1,10]]) {
  const rows=complete(n); // Adjust the ending snapshots to the complete manifested count.
  const end=rows.findIndex(x=>x[3]===5&&x[4]===0);
  for(let i=end;i<rows.length;i++){rows[i][6]=f;rows[i][7]=d+structure.filter(([k,r])=>k===5&&r>0).slice(0,Math.max(0,rows[i][4])).length;}
  // Directory count monotonicity is all later roots need in this wire-only vector.
  for(let i=end;i<rows.length;i++)rows[i][7]=Math.max(d,rows[i-1]?.[7]??0);
  const x=decode(rows);assert.equal(x.end(),true,x.failure);assert.equal(x.validateSamples(manifest(f,d)),true);
 }
 const wrongManifest=decode(complete(7));assert.throws(()=>wrongManifest.validateSamples(manifest(16)),/sample/);
 for(const mutate of [
  r=>{r[0][0]=1;},r=>{r[0][0]=2;}, r=>{r[0].push(0);}, r=>{r[5][9]=2;},
  r=>{r[5][4]=1;},r=>{r[8][6]=1;},r=>{r[12][6]=17;},
  r=>{r[6][9]=1;},r=>{r[0][2]=2;},r=>{r[0][5]=1001;},
  r=>{r[0][6]=200001;},r=>{r[0][7]=50001;},r=>{r[0][8]=346;},
  r=>{r[0][3]=11;},r=>{r[16][9]=12;},r=>{r[17][3]=5;},
 ]) {const rows=complete();mutate(rows);const x=decode(rows);assert.equal(x.end(),false);}
 for(const bad of [Buffer.from(' '+wire(complete())),Buffer.from(wire(complete()).toString().replace('3,','3.0,')),Buffer.from('[3,0]\r\n'),Buffer.from([239,187,191]),Buffer.alloc(128,65),Buffer.alloc(4097,65),Buffer.from(wire(complete()).toString().slice(0,-1))]) {
  assert.equal(decode([],{bytes:bad}).end(),false);
 }
 const maxLine='[3,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",32,10,0,34359738368,65536,49992,345,11]\n';
 assert.equal(Buffer.byteLength(maxLine),78);
 const maxRows=complete();for(const row of maxRows){row[5]=34359738368;if(row[2]>=4)row[8]=345;}
 maxRows[15][7]=49992;let maximumDirs=49992;for(let i=16;i<maxRows.length;i++){if(maxRows[i][3]===5&&maxRows[i][4]>0)maximumDirs++;maxRows[i][7]=maximumDirs;}
 const maximum=new Decoder({nonce,mode:'before',grant:34359738368,ledger:suite});injectedVector(wire(maxRows));maximum.feed(wire(maxRows));assert.equal(maximum.end(),true,maximum.failure);assert.equal(maximum.trace[15][1][5],34359738368);
 const tail=new Decoder({nonce,mode:'before',grant:1000,ledger:suite});injectedVector(Buffer.alloc(127,65));tail.feed(Buffer.alloc(127,65));assert.equal(tail.failure,null);assert.equal(tail.tailLength,127);tail.feed(Buffer.from('A'));assert.equal(tail.failure,'line-limit');
 const after=decode(complete());after.feed(Buffer.from('x'));assert.ok(after.failure);assert.equal(after.trace.length,32);
 const partial=decode(complete().slice(0,10));assert.equal(partial.end(),false);assert.equal(settle(partial,{code:1,drained:true,sourceValid:true}).charged,1000);
 const err=decode([literal(1,0,7),literal(2,9,7,123,0,0,0,2)]);assert.equal(settle(err,{code:1,drained:true,sourceValid:true}).charged,123);
 assert.equal(settle(err,{code:1,drained:true,sourceValid:false}).charged,1000);
 assert.equal(settle(full,{code:0,drained:false,sourceValid:true,outputValid:true}).charged,1000);
 const impossibleEarly=decode([literal(1,0,7,100,1,1),literal(2,9,7,100,1,1,0,2)]);assert.equal(settle(impossibleEarly,{code:1,drained:true,sourceValid:true}).charged,1000);
 const missing=decode([literal(1,0,7),literal(2,1,7),literal(3,2,7),literal(4,3,7),literal(5,4,0),literal(6,9,7,100,16,1,0,2)]);
 assert.equal(settle(missing,{code:1,drained:true,sourceValid:true}).charged,1000);
 assert.equal(decode([literal(1,0,7)],{now:2400000}).end(),false);
 const pre=decode([[0,7],[1,7],[6,7],[7,7],[8,7]].map(([k,r],i)=>literal(i+1,k,r)),{mode:'preflight'});assert.equal(pre.end(),true);
 assert.equal(decode([literal(1,10,0,100,0,0,0,1)],{mode:'preflight'}).end(),false);
 injectedVector(wire(complete()));const split=new Decoder({nonce,mode:'before',grant:1000,ledger:suite});for(const b of wire(complete()))split.feed(Buffer.from([b]));assert.equal(split.end(),true,split.failure);
 const reserve=new WorkBudget();const grants=[];for(let i=0;i<3;i++){const r=reserve.readerGrant();grants.push(r.grant);reserve.hash(1024**2);r.finish(100);assert.throws(()=>r.finish(0));}assert.ok(grants[0]>grants[1]&&grants[1]>grants[2]);
 const max=new WorkBudget();const r=max.readerGrant();max.hash(129*1024**2);assert.throws(()=>max.hash(1));r.finish(r.grant);assert.equal(max.remaining(),3*1024**2);assert.throws(()=>max.readerGrant());
 const receipt=encodeResult({success:false,reader_traces:[{...settle(full,{code:0,drained:true,sourceValid:true,outputValid:true}),mode:'before',nonce,metrics_status:'admitted',metrics:{strategy:'reuse',scratch_allocations:1,scratch_capacity_bytes:1048576,maximum_leases:1,hash_bytes:0},output_sha256:'a'.repeat(64),output_bytes:64}]});assert.equal(JSON.parse(receipt).reader_traces[0].trace.length,32);
 assert.throws(()=>encodeResult({reader_traces:[{trace:full.trace,note:'x'.repeat(2049)}]}),/metadata/);
 const metadata=JSON.parse(receipt).reader_traces[0];assert.throws(()=>encodeResult({reader_traces:[{...metadata,raw:'untrusted'}]}),/metadata/);assert.throws(()=>encodeResult({reader_traces:[metadata,metadata]}),/binding/);
 assert.equal(typeof boundedJSON,'function','bounded encoding primitive required');
 assert.equal(boundedJSON({x:'1234'},12),'{"x":"1234"}');assert.throws(()=>boundedJSON({x:'1234'},11),/result/);
 const boundStatus=JSON.parse(receipt).reader_traces[0];
 const rebound=(trace,mode,id)=>({...boundStatus,mode,nonce:id,trace:trace.map(([time,row])=>[time,row.map((v,i)=>i===1?id:v)])});
 const maximumAggregate=[rebound(pre.trace,'preflight','b'.repeat(32)),rebound(full.trace,'before',nonce),rebound(full.trace,'after','c'.repeat(32))];
 const aggregateBytes=encodeResult({reader_traces:maximumAggregate});injectedVector(aggregateBytes);
 assert.equal(JSON.parse(aggregateBytes).reader_traces.reduce((n,t)=>n+t.trace.length,0),69);
 const tooMany=structuredClone(maximumAggregate);tooMany[0].trace.push(tooMany[0].trace[0]);assert.throws(()=>encodeResult({reader_traces:tooMany}),/trace/);
 assert.throws(()=>parseStrict('{"x":1,"x":1}'));assert.throws(()=>parseStrict('{"x":1} true'));
 console.log(JSON.stringify({evidence_kind:'root-diagnostic-pure-fixture',vectors,injected_bytes:injected,stream_bytes:suite.streamed,native_capacity:'unverified',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));
});


test('complete output and held source admission precede settlement; live entry has no effects',async()=>{
 const url=new URL('./pair-root-diagnostic-windows-build.mjs',import.meta.url);
 assert.ok(fs.existsSync(url),'v3 controller admission is not implemented');
 const {main,validateOutput,sealSources,admitReader,receipt,sourceFiles}=await import(url);
 const {Decoder,WorkBudget}=await import(codecURL);
 const path=await import('node:path'),os=await import('node:os');
 const pool=await fixturePool();pool.hash(12*1024**2); // Four source/output cases, each at most two 1MiB source passes plus 1MiB output work.
 const dir=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'root-diag-admission-')));
 try {
  assert.throws(()=>main(),/live-entry-disabled/);assert.deepEqual(fs.readdirSync(dir),[]);
  const decoder=()=>{injectedVector(wire(complete(2)));const d=new Decoder({nonce,mode:'before',grant:1000,ledger:new WorkBudget()});d.feed(wire(complete(2)));return d;};
  const data={success:true,files:[],directories:['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'].map(root=>({root,file:'',id:'00000001:0000000000000001'})),file_bytes:0,file_count:0,directory_count:7,hash_bytes:100,crates:[],archive_cache_id:'00000001:0000000000000001'};
  const envelope={schema:2,evidence_kind:'root-diagnostic-reader-output',mode:'before',nonce,policy_sha256:'1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6',data,metrics:{strategy:'reuse',scratch_allocations:1,scratch_capacity_bytes:1048576,maximum_leases:1,hash_bytes:0}};
  assert.equal(validateOutput(envelope,decoder()),data);
  assert.throws(()=>validateOutput({...envelope,schema:1,evidence_kind:'progress-reader-output'},decoder()),/binding/);
  assert.throws(()=>validateOutput({...envelope,extra:0},decoder()),/fields/);
  assert.throws(()=>validateOutput({...envelope,data:{...data,files:[{root:'msvc',file:'bad',id:'00000001:0000000000000001',bytes:1,sha256:'0'.repeat(64)}]}},decoder()),/manifest|sample|count/);
  let sourceBytes=0;const ownedFiles=new Set(),ownedDirs=new Set([dir]);
  function ownWrite(file,bytes){const n=Buffer.byteLength(bytes);assert.ok(sourceBytes+n<=1048576);sourceBytes+=n;ownedFiles.add(file);assert.ok(ownedFiles.size<=32);fs.writeFileSync(file,bytes);}
  for(const file of sourceFiles){sourceBytes+=fs.statSync(new URL('../../'+file,import.meta.url)).size;assert.ok(sourceBytes<=1048576);const dest=path.join(dir,file);ownedFiles.add(dest);assert.ok(ownedFiles.size<=32);ownedDirs.add(path.dirname(dest));ownedDirs.add(path.dirname(path.dirname(dest)));assert.ok(ownedDirs.size<=16);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(new URL('../../'+file,import.meta.url),dest);}
  const fixtureCopy=path.join(dir,'scripts/qualification/pair-root-diagnostic-windows-build-fixtures.ps1');
  if(!fs.existsSync(fixtureCopy)){const original=new URL('./pair-root-diagnostic-windows-build-fixtures.ps1',import.meta.url);sourceBytes+=fs.statSync(original).size;assert.ok(sourceBytes<=1048576);fs.copyFileSync(original,fixtureCopy);}
  const output=path.join(dir,'output.json');ownWrite(output,JSON.stringify(envelope));
  for(const scenario of ['ok','output-rejected','source-replaced','fixture-source-replaced']){
   const budget=new WorkBudget();const seal=sealSources(dir,budget),reservation=budget.readerGrant();
   injectedVector(wire(complete(2)));const d=new Decoder({nonce,mode:'before',grant:reservation.grant,ledger:budget});d.feed(wire(complete(2)));
   if(scenario==='output-rejected')ownWrite(output,JSON.stringify({...envelope,schema:1}));
   if(scenario==='source-replaced'){assert.ok(++sourceBytes<=1048576);fs.appendFileSync(path.join(dir,sourceFiles[0]),'\n');}
   if(scenario==='fixture-source-replaced'){assert.ok(++sourceBytes<=1048576);fs.appendFileSync(fixtureCopy,'\n');}
   const result=admitReader({decoder:d,close:{code:0,drained:true},reservation,budget,sourceRoot:dir,seal,output});
   assert.equal(result.disposition.reader_complete,scenario==='ok');
   assert.equal(result.disposition.charged,scenario==='ok'?100:d.grant);
   assert.equal(budget.readerOutstanding,false);assert.throws(()=>reservation.finish(0));
   const published=JSON.parse(receipt([result.disposition]));assert.equal(published.schema,4);assert.equal(published.runtime_qualified,false);assert.equal(published.candidate_evidence_complete,false);assert.equal(published.target_executions,0);assert.equal(published.reader_traces[0].trace.length,23);
   ownWrite(output,JSON.stringify(envelope));
  }
 }finally{fs.rmSync(dir,{recursive:true});}
});


test('runner latches deadline/framing/stderr failures while retaining and charging prefixes',async()=>{
 const {WorkBudget,Decoder}=await import(codecURL);const {CommandRunner}=await import('./pair-root-diagnostic-windows-build.mjs');
 const {EventEmitter}=await import('node:events'),{PassThrough}=await import('node:stream');const pool=await fixturePool();
 for(const scenario of ['spawn-deadline','close-deadline','late-after-refusal','stderr','unclosed-drain']){
  injectedVector();let now=0;const ledger=new WorkBudget({clock:()=>now,seconds:1});if(scenario==='unclosed-drain')ledger.finalDeadline=1;const stream=ledger.stream.bind(ledger);ledger.stream=n=>{pool.stream(n);stream(n);};
  const d=new Decoder({nonce,mode:'before',grant:1000,ledger});let child;
  const runner=new CommandRunner(ledger,{spawnProcess:()=>{
   child=new EventEmitter();Object.assign(child,{pid:123,exitCode:null,signalCode:null,stdout:new PassThrough(),stderr:new PassThrough(),unref:()=>{child.unreferenced=true;}});
   if(scenario==='spawn-deadline')now=1000;
   else queueMicrotask(()=>{
    if(scenario==='stderr')child.stderr.write('unexpected');
    else if(scenario==='unclosed-drain')child.stdout.write(Buffer.from('[2]\n'));
    else if(scenario==='late-after-refusal')child.stdout.write(Buffer.from('[2]\n'));
    else{child.stdout.write(wire([literal(1,0,7)]));now=1000;child.exitCode=0;child.emit('close',0);}
   });return child;
  },terminate:c=>{if(scenario==='unclosed-drain')return false;queueMicrotask(()=>{if(scenario==='late-after-refusal')c.stdout.write(wire([literal(1,0,7)]));c.exitCode=1;c.emit('close',1);});return true;}});
  await assert.rejects(()=>runner.run('literal-owned-double',[],{decoder:d,timeout:1000}),scenario.endsWith('deadline')?/time-limit/:scenario==='stderr'?/reader-stderr/:scenario==='unclosed-drain'?/process-drain-unavailable/:/framing/);
  assert.equal(runner.drained,scenario!=='unclosed-drain');assert.equal(runner.commands.length,1);
  if(scenario!=='close-deadline'){assert.equal(runner.failureDeadline,scenario==='unclosed-drain'?1:now+120000);assert.equal(ledger.finalDeadline,runner.failureDeadline);await assert.rejects(()=>runner.run('forbidden-later-child',[]),/runner-stopped/);assert.equal(runner.commands.length,1);}
  if(scenario==='unclosed-drain'){assert.equal(runner.terminationUnknown,true);assert.equal(child.unreferenced,true);}
  assert.equal(d.trace.length,scenario==='close-deadline'?1:0);
  if(scenario==='late-after-refusal')assert.equal(ledger.streamed,4+wire([literal(1,0,7)]).length);
 }
});

test('bounded owned Windows reader and final zero-hash timeout',{skip:process.platform!=='win32'},async()=>{
 const {WorkBudget,Decoder,settle}=await import(codecURL);
 const {CommandRunner,validateOutput,sealSources,admitReader,readOutput}=await import('./pair-root-diagnostic-windows-build.mjs');
 const {validateManifest}=await import('./pair-progress-windows-build.mjs');
 const path=await import('node:path'),os=await import('node:os');
 const budget=await fixturePool(),runner=new CommandRunner(budget);
 const cwd=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'root-diag-native-'))),repo=path.resolve('.');
 const env={SystemRoot:'C:\\Windows',WINDIR:'C:\\Windows',PATH:'C:\\Windows\\System32',TEMP:cwd,TMP:cwd};
 const args=kind=>['-NoProfile','-NonInteractive','-File',path.resolve('scripts/qualification/pair-root-diagnostic-windows-build-fixtures.ps1'),'-Case',kind];
 const traces=[];
 try {
  for(let i=0;i<4;i++)injectedVector(); // Four memory-only storage boundary models in the checks worker.
  budget.hash(16*1024**2); // Reserve the complete checks-worker ceiling before any child work.
  const checks=await runner.run('C:\\Program Files\\PowerShell\\7\\pwsh.exe',args('checks'),{env,cwd,timeout:30000,stdoutLimit:65536});
  assert.equal(checks.code,0,checks.stderr);assert.equal(checks.stderr,'');const checked=JSON.parse(checks.stdout);assert.equal(checked.tests_passed,true);assert.equal(checked.resource_vectors,4);assert.ok(checked.hash_bytes<=16*1024**2);
  for(const [key,cap] of [['files',512],['dirs',64],['inputs',2097152],['scratch',2097152]])assert.ok(checked.storage[key]<=cap);
  const seal=sealSources(repo,budget);budget.hash(1024**2); // Complete old-library comparison allowance, no refund.
  const finish=budget.reserve(1024**2),reservation={grant:1024**2,finish};
  const d=new Decoder({nonce,mode:'before',grant:reservation.grant,ledger:budget});
  const pipeline=await runner.run('C:\\Program Files\\PowerShell\\7\\pwsh.exe',args('pipeline'),{env,cwd,timeout:30000,stdoutLimit:4096,decoder:d});
  assert.equal(pipeline.code,0,pipeline.stderr);assert.equal(pipeline.stderr,'');assert.equal(d.trace.length,29);
  const admitted=admitReader({decoder:d,close:{code:pipeline.code,drained:runner.drained},reservation,budget,sourceRoot:repo,seal,output:path.join(cwd,'root-output.json')});
  assert.equal(admitted.disposition.reader_complete,true,JSON.stringify(admitted.disposition));traces.push(d.trace);
  const baseline=readOutput(path.join(cwd,'root-baseline.json'),budget).value;
  const labels=['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'];
  assert.deepEqual(validateManifest(admitted.data,labels),validateManifest(baseline,labels));
  assert.equal(admitted.data.files.length,17);assert.equal(admitted.data.files[0].sha256,'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  budget.hash(16*1024**2);
  const refused=await runner.run('C:\\Program Files\\PowerShell\\7\\pwsh.exe',args('refusals'),{env,cwd,timeout:30000,stdoutLimit:65536});
  assert.equal(refused.code,0,refused.stderr);assert.equal(refused.stderr,'');const summary=JSON.parse(refused.stdout);assert.equal(summary.tests_passed,true);assert.equal(summary.cases.length,7);assert.ok(summary.hash_bytes<=16*1024**2);
  for(const [key,cap] of [['files',512],['dirs',64],['inputs',524288],['scratch',2097152]])assert.ok(summary.storage[key]<=cap);
  // Preallocated maxima across parent plus three workers, without per-case pool resets.
  assert.ok(32+3*512<=2048);assert.ok(16+3*64<=256);assert.equal(1048576+2097152+2*524288,4194304);assert.equal(4*2097152,8388608);
  for(const item of summary.cases){
   const x=new Decoder({nonce:item.nonce,mode:'before',grant:16*1024**2,ledger:{stream:()=>{},active:()=>budget.check(),elapsed:()=>budget.elapsed()}});
   x.feed(Buffer.from(item.prefix));assert.equal(x.end(),true,x.failure);assert.equal(x.sample,item.stop);assert.equal(item.payload_bytes,item.stop>=5?3:0);
   assert.equal(settle(x,{code:1,drained:true,sourceValid:true}).reader_complete,false);traces.push(x.trace);
  }
  assert.ok(Buffer.byteLength(JSON.stringify(traces))<=18432);
  await assert.rejects(()=>runner.run(process.execPath,['-e','setInterval(()=>{},1000)'],{env,cwd,timeout:50,stdoutLimit:0}),/time-limit/);
  assert.equal(runner.kills,1);assert.equal(runner.drained,true);
  console.log(JSON.stringify({evidence_kind:'root-diagnostic-owned-fixture',tests_passed:true,node_version:process.version,ps_children:3,node_timeout_children:1,hash_bytes:budget.hashed,stream_bytes:budget.streamed,trace_records:d.trace.length,controlled_refusals:7,legacy_assertions:checked.assertions,forced_terminations:runner.kills,cleanup:'owned-scratch-removed',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));
 }finally{if(runner.drained)fs.rmSync(cwd,{recursive:true});}
});

test('fixture accounting stays in one cumulative pool',async()=>{
 const pool=await fixturePool();assert.ok(pool.hashed<=64*1024**2);assert.ok(pool.streamed<=1024**2);assert.ok(fixtureCounts.vectors<=64);assert.ok(fixtureCounts.injected<=512*1024);
 console.log(JSON.stringify({evidence_kind:'root-diagnostic-fixture-pool',vectors:fixtureCounts.vectors,injected_bytes:fixtureCounts.injected,hash_bytes_charged:pool.hashed,stream_bytes:pool.streamed,native_executed:process.platform==='win32',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));
});
