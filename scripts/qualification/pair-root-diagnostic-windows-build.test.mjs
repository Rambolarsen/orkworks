import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
const codecURL = new URL('./pair-root-diagnostic-windows-build-protocol.mjs', import.meta.url);
const nonce = 'a'.repeat(32);

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
 const {Decoder,WorkBudget,settle,encodeResult,parseStrict}=await import(codecURL);
 let vectors=0,injected=0;const suite = new WorkBudget({hashLimit:64*1024**2,streamLimit:1024**2,seconds:120});
 function decode(rows,{mode='before',bytes=wire(rows),now=0}={}) {
  assert.ok(++vectors<=64);assert.ok(bytes.length<=8192);injected+=bytes.length;assert.ok(injected<=512*1024);
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
  r=>{r[0][0]=2;}, r=>{r[0].push(0);}, r=>{r[5][9]=2;},
  r=>{r[5][4]=1;},r=>{r[8][6]=1;},r=>{r[12][6]=17;},
  r=>{r[6][9]=1;},r=>{r[0][2]=2;},r=>{r[0][5]=1001;},
  r=>{r[0][6]=200001;},r=>{r[0][7]=50001;},r=>{r[0][8]=346;},
  r=>{r[0][3]=11;},r=>{r[16][9]=12;},r=>{r[17][3]=5;},
 ]) {const rows=complete();mutate(rows);const x=decode(rows);assert.equal(x.end(),false);}
 for(const bad of [Buffer.from(' '+wire(complete())),Buffer.from(wire(complete()).toString().replace('3,','3.0,')),Buffer.from('[3,0]\r\n'),Buffer.from([239,187,191]),Buffer.alloc(128,65),Buffer.alloc(4097,65),Buffer.from(wire(complete()).toString().slice(0,-1))]) {
  assert.equal(decode([],{bytes:bad}).end(),false);
 }
 const after=decode(complete());after.feed(Buffer.from('x'));assert.ok(after.failure);assert.equal(after.trace.length,32);
 const partial=decode(complete().slice(0,10));assert.equal(partial.end(),false);assert.equal(settle(partial,{code:1,drained:true,sourceValid:true}).charged,1000);
 const err=decode([literal(1,0,7),literal(2,9,7,123,0,0,0,2)]);assert.equal(settle(err,{code:1,drained:true,sourceValid:true}).charged,123);
 assert.equal(settle(err,{code:1,drained:true,sourceValid:false}).charged,1000);
 assert.equal(settle(full,{code:0,drained:false,sourceValid:true,outputValid:true}).charged,1000);
 const missing=decode([literal(1,0,7),literal(2,1,7),literal(3,2,7),literal(4,3,7),literal(5,4,0),literal(6,9,7,100,16,1,0,2)]);
 assert.equal(settle(missing,{code:1,drained:true,sourceValid:true}).charged,1000);
 assert.equal(decode([literal(1,0,7)],{now:2400000}).end(),false);
 const pre=decode([[0,7],[1,7],[6,7],[7,7],[8,7]].map(([k,r],i)=>literal(i+1,k,r)),{mode:'preflight'});assert.equal(pre.end(),true);
 assert.equal(decode([literal(1,10,0,100,0,0,0,1)],{mode:'preflight'}).end(),false);
 const split=new Decoder({nonce,mode:'before',grant:1000,ledger:suite});for(const b of wire(complete()))split.feed(Buffer.from([b]));assert.equal(split.end(),true,split.failure);
 const reserve=new WorkBudget();const grants=[];for(let i=0;i<3;i++){const r=reserve.readerGrant();grants.push(r.grant);reserve.hash(1024**2);r.finish(100);assert.throws(()=>r.finish(0));}assert.ok(grants[0]>grants[1]&&grants[1]>grants[2]);
 const max=new WorkBudget();const r=max.readerGrant();max.hash(129*1024**2);assert.throws(()=>max.hash(1));r.finish(r.grant);assert.equal(max.remaining(),3*1024**2);assert.throws(()=>max.readerGrant());
 const receipt=encodeResult({success:false,reader_traces:[{...settle(full,{code:0,drained:true,sourceValid:true,outputValid:true}),mode:'before',nonce}]});assert.equal(JSON.parse(receipt).reader_traces[0].trace.length,32);
 assert.throws(()=>encodeResult({reader_traces:[{trace:full.trace,note:'x'.repeat(2049)}]}),/metadata/);
 assert.throws(()=>encodeResult({reader_traces:[],note:'x'.repeat(65536)}),/result/);
 assert.throws(()=>parseStrict('{"x":1,"x":1}'));assert.throws(()=>parseStrict('{"x":1} true'));
 console.log(JSON.stringify({evidence_kind:'root-diagnostic-pure-fixture',vectors,injected_bytes:injected,stream_bytes:suite.streamed,native_capacity:'unverified',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));
});


test('complete output and held source admission precede settlement; live entry has no effects',async()=>{
 const url=new URL('./pair-root-diagnostic-windows-build.mjs',import.meta.url);
 assert.ok(fs.existsSync(url),'v3 controller admission is not implemented');
 const {main,validateOutput,sealSources,admitReader,receipt,sourceFiles}=await import(url);
 const {Decoder,WorkBudget}=await import(codecURL);
 const path=await import('node:path'),os=await import('node:os');
 const dir=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'root-diag-admission-')));
 try {
  assert.throws(()=>main(),/live-entry-disabled/);assert.deepEqual(fs.readdirSync(dir),[]);
  const decoder=()=>{const d=new Decoder({nonce,mode:'before',grant:1000,ledger:new WorkBudget()});d.feed(wire(complete(2)));return d;};
  const data={success:true,files:[],directories:['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'].map(root=>({root,file:'',id:'00000001:0000000000000001'})),file_bytes:0,file_count:0,directory_count:7,hash_bytes:100,crates:[],archive_cache_id:'00000001:0000000000000001'};
  const envelope={schema:2,evidence_kind:'root-diagnostic-reader-output',mode:'before',nonce,policy_sha256:'1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6',data,metrics:{strategy:'reuse',scratch_allocations:1,scratch_capacity_bytes:1048576,maximum_leases:1,hash_bytes:0}};
  assert.equal(validateOutput(envelope,decoder()),data);
  assert.throws(()=>validateOutput({...envelope,schema:1,evidence_kind:'progress-reader-output'},decoder()),/binding/);
  assert.throws(()=>validateOutput({...envelope,extra:0},decoder()),/fields/);
  assert.throws(()=>validateOutput({...envelope,data:{...data,files:[{root:'msvc',file:'bad',id:'00000001:0000000000000001',bytes:1,sha256:'0'.repeat(64)}]}},decoder()),/manifest|sample|count/);
  for(const file of sourceFiles){const dest=path.join(dir,file);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(new URL('../../'+file,import.meta.url),dest);}
  const output=path.join(dir,'output.json');fs.writeFileSync(output,JSON.stringify(envelope));
  for(const scenario of ['ok','output-rejected','source-replaced']){
   const budget=new WorkBudget();const seal=sealSources(dir,budget),reservation=budget.readerGrant();
   const d=new Decoder({nonce,mode:'before',grant:reservation.grant,ledger:budget});d.feed(wire(complete(2)));
   if(scenario==='output-rejected')fs.writeFileSync(output,JSON.stringify({...envelope,schema:1}));
   if(scenario==='source-replaced')fs.appendFileSync(path.join(dir,sourceFiles[0]),'\n');
   const result=admitReader({decoder:d,close:{code:0,drained:true},reservation,budget,sourceRoot:dir,seal,output});
   assert.equal(result.disposition.reader_complete,scenario==='ok');
   assert.equal(result.disposition.charged,scenario==='ok'?100:d.grant);
   assert.equal(budget.readerOutstanding,false);assert.throws(()=>reservation.finish(0));
   const published=JSON.parse(receipt([result.disposition]));assert.equal(published.schema,4);assert.equal(published.runtime_qualified,false);assert.equal(published.candidate_evidence_complete,false);assert.equal(published.target_executions,0);assert.equal(published.reader_traces[0].trace.length,23);
   fs.writeFileSync(output,JSON.stringify(envelope));
  }
 }finally{fs.rmSync(dir,{recursive:true});}
});


test('bounded owned Windows reader and final zero-hash timeout',{skip:process.platform!=='win32'},async()=>{
 const {WorkBudget,Decoder,settle}=await import(codecURL);
 const {CommandRunner,validateOutput,sealSources,admitReader,readOutput}=await import('./pair-root-diagnostic-windows-build.mjs');
 const {validateManifest}=await import('./pair-progress-windows-build.mjs');
 const path=await import('node:path'),os=await import('node:os');
 const budget=new WorkBudget({hashLimit:64*1024**2,streamLimit:1024**2,seconds:120}),runner=new CommandRunner(budget);
 const cwd=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'root-diag-native-'))),repo=path.resolve('.');
 const env={SystemRoot:'C:\\Windows',WINDIR:'C:\\Windows',PATH:'C:\\Windows\\System32',TEMP:cwd,TMP:cwd};
 const args=kind=>['-NoProfile','-NonInteractive','-File',path.resolve('scripts/qualification/pair-root-diagnostic-windows-build-fixtures.ps1'),'-Case',kind];
 const traces=[];
 try {
  budget.hash(16*1024**2); // Reserve the complete checks-worker ceiling before any child work.
  const checks=await runner.run('C:\\Program Files\\PowerShell\\7\\pwsh.exe',args('checks'),{env,cwd,timeout:30000,stdoutLimit:65536});
  assert.equal(checks.code,0,checks.stderr);assert.equal(checks.stderr,'');const checked=JSON.parse(checks.stdout);assert.equal(checked.tests_passed,true);assert.ok(checked.hash_bytes<=16*1024**2);
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
  for(const item of summary.cases){
   const x=new Decoder({nonce:item.nonce,mode:'before',grant:16*1024**2,ledger:{stream:()=>{},active:()=>budget.check(),elapsed:()=>budget.elapsed()}});
   x.feed(Buffer.from(item.prefix));assert.equal(x.end(),true,x.failure);assert.equal(x.sample,item.stop);assert.equal(item.payload_bytes,item.stop>=5?3:0);
   assert.equal(settle(x,{code:1,drained:true,sourceValid:true}).reader_complete,false);traces.push(x.trace);
  }
  assert.ok(Buffer.byteLength(JSON.stringify(traces))<=18432);
  await assert.rejects(()=>runner.run(process.execPath,['-e','setInterval(()=>{},1000)'],{env,cwd,timeout:50,stdoutLimit:0}),/time-limit/);
  assert.equal(runner.kills,1);assert.equal(runner.drained,true);
  console.log(JSON.stringify({evidence_kind:'root-diagnostic-owned-fixture',tests_passed:true,ps_children:3,node_timeout_children:1,hash_bytes:budget.hashed,stream_bytes:budget.streamed,trace_records:d.trace.length,controlled_refusals:7,legacy_assertions:checked.assertions,forced_terminations:runner.kills,cleanup:'owned-scratch-removed',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));
 }finally{if(runner.drained)fs.rmSync(cwd,{recursive:true});}
});
