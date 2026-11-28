import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
const url = new URL('./pair-progress-windows-build-protocol.mjs', import.meta.url);
test('live progress domain, admission and decreasing grants', async () => {
 assert.ok(fs.existsSync(url), 'dedicated live codec is required');
 const {Decoder,WorkBudget,settle,phases,encodeResult,parseStrict}=await import(url);
 const nonce='a'.repeat(32), MiB=1024**2;
 const make=(mode='before', grant=1000)=>{const ledger=new WorkBudget(); return new Decoder({nonce,mode,grant,ledger});};
 const wire=(seq,phase,root='',h=0,f=0,d=0,a=0,version=2)=>Buffer.from(JSON.stringify([version,nonce,seq,phase,root,h,f,d,a])+'\n');
 const good=()=>{const d=make();phases.forEach(([p,r],i)=>d.feed(wire(i+1,p,r,12,3,7,1)));return d;};
 const d=good();assert.equal(d.trace.length,21);assert.equal(d.end(),true);
 assert.ok(Object.isFrozen(d.trace[0][1]));assert.equal(settle(d,{code:0,drained:true,sourceValid:true,outputValid:false}).charged,1000);
 assert.equal(settle(d,{code:0,drained:true,sourceValid:true,outputValid:true}).charged,12);
 for(const [p,r] of phases){assert.equal(typeof p,'string');assert.equal(typeof r,'string');}
 for(const version of [1,3]){const x=make();x.feed(wire(1,'start','',0,0,0,0,version));assert.ok(x.failure);}
 for(const mutate of [b=>Buffer.concat([b,b]),b=>Buffer.from(' '+b),b=>Buffer.from(b.toString().replace('2,','2.0,')),b=>Buffer.concat([b,Buffer.from('x')])]){
  const x=make();x.feed(mutate(wire(1,'start')));x.end();assert.ok(x.failure);
 }
 const split=make();for(const b of Buffer.concat(phases.map(([p,r],i)=>wire(i+1,p,r))))split.feed(Buffer.from([b]));assert.equal(split.end(),true);
 const pre=make('preflight');for(const [i,p] of ['start','ready','serialize-begin','serialize-end','ok'].entries())pre.feed(wire(i+1,p));assert.equal(pre.end(),true);
 const badpre=make('preflight');badpre.feed(wire(1,'start','',0,1));assert.ok(badpre.failure);
 const err=make();err.feed(wire(1,'start'));err.feed(Buffer.from(JSON.stringify([2,nonce,2,'error','',9,0,0,0,'input-denied'])+'\n'));
 assert.equal(settle(err,{code:1,drained:true,sourceValid:true}).charged,9);assert.equal(settle(err,{code:0,drained:true,sourceValid:true}).charged,1000);
 assert.equal(settle(good(),{code:0,drained:false,sourceValid:true,outputValid:true}).unknown_work,true);
 assert.equal(settle(good(),{code:0,drained:true,sourceValid:false,outputValid:true}).unknown_work,true);
 const extra=good();extra.feed(Buffer.from('x'));assert.ok(extra.failure);assert.equal(extra.trace.length,21);
 const overflow=make();overflow.feed(Buffer.alloc(5000,65));assert.equal(overflow.ledger.streamed,5000);assert.ok(overflow.failure);
 let now=0;const late=new Decoder({nonce,mode:'before',grant:1000,ledger:new WorkBudget({clock:()=>now,seconds:1})});now=1000;late.feed(wire(1,'start'));assert.ok(late.failure);
 const ledger=new WorkBudget();const grants=[];for(let i=0;i<3;i++){const r=ledger.readerGrant();grants.push(r.grant);ledger.hash(MiB);r.finish(100);}
 assert.ok(grants[0]>grants[1]&&grants[1]>grants[2]);assert.equal(ledger.hashed,3*MiB+300);
 const max=new WorkBudget();const r=max.readerGrant();assert.throws(()=>max.readerGrant());max.hash(129*MiB);assert.throws(()=>max.hash(1));r.finish(r.grant);assert.equal(max.remaining(),3*MiB);assert.throws(()=>max.readerGrant());max.emergency();max.hash(MiB+131072);assert.equal(max.remaining(),1920*1024);assert.throws(()=>r.finish(0));
 const receipt=encodeResult({success:false,traces:[d.trace]});assert.deepEqual(JSON.parse(receipt).traces,[d.trace]);assert.throws(()=>encodeResult({traces:[],x:'x'.repeat(65536)}));
 assert.throws(()=>parseStrict('{"x":1,"x":1}'));assert.throws(()=>parseStrict('{"x":1} true'));assert.deepEqual(parseStrict('{"x":1}'),{x:1});
});

test('live reader preserves the complete checks and adds isolated ownership', () => {
 const reader=new URL('./pair-progress-windows-build.ps1',import.meta.url);
 assert.ok(fs.existsSync(reader),'live reader required');
 const text=fs.readFileSync(reader,'utf8');
 for(const word of ['Assert-CrateArchives','archive-entry-type','unlisted-crate-file','unaudited-build-script','unaudited-proc-macro','Get-HandleInfo','file-drift','archive-drift','Get-SealedManifest','PE','serialize-end','OrkProgress.Payload','CreateNew','policy_sha256','Read-LiveRequest','Invoke-ManifestRead','Write-ReaderOutput','Charge-Encoding'])assert.ok(text.includes(word),word);
 assert.ok(!text.includes('ORK_REPLACEMENT'));
 assert.ok(!text.includes('[byte[]]::new(1048576)'),'no per-file payload allocation');
 assert.deepEqual(fs.readFileSync(new URL('./pair-progress-windows-build.inputs.json',import.meta.url)),fs.readFileSync(new URL('./pair-replacement-windows-build.inputs.json',import.meta.url)));
});

await import('./pair-progress-windows-build-controller.test.mjs');

test('owned Windows reader fixture and final zero-grant timeout', {skip:process.platform!=='win32'}, async()=>{
 const {WorkBudget,Decoder}=await import('./pair-progress-windows-build-protocol.mjs');const {CommandRunner,validateOutput}=await import('./pair-progress-windows-build.mjs');
 const path=await import('node:path'),os=await import('node:os');
 const b=new WorkBudget({hashLimit:64*1024**2,streamLimit:1024**2,seconds:120}),r=new CommandRunner(b);
 const cwd=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'progress-fixture-')));
 const runtime={SystemRoot:'C:\\Windows',WINDIR:'C:\\Windows',PATH:'C:\\Windows\\System32',TEMP:cwd,TMP:cwd};
 try{
  const ps=await r.run('C:\\Program Files\\PowerShell\\7\\pwsh.exe',['-NoProfile','-NonInteractive','-File',path.resolve('scripts/qualification/pair-progress-windows-build.test.ps1')],{env:runtime,cwd,timeout:30000,stdoutLimit:8192});
  assert.equal(ps.code,0,ps.stderr);assert.equal(ps.stderr,'');const lines=ps.stdout.trimEnd().split('\n');const summary=JSON.parse(lines.pop());
  const decoder=new Decoder({nonce:'a'.repeat(32),mode:'before',grant:4194304,ledger:{stream:()=>{},active:()=>b.check(),elapsed:()=>b.elapsed()}});
  // CommandRunner already charged these exact observed bytes to the shared fixture ledger.
  decoder.feed(Buffer.from(lines.join('\n')+'\n'));assert.equal(decoder.end(),true,JSON.stringify({failure:decoder.failure,trace:decoder.trace,bytes:Buffer.byteLength(ps.stdout)}));assert.equal(decoder.trace.length,21);
  assert.equal(validateOutput(summary.envelope,decoder).files[0].sha256,'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  assert.equal(summary.tests_passed,true);b.hash(summary.hash_bytes);assert.equal(summary.scratch_allocations,1);assert.equal(summary.maximum_leases,1);assert.equal(summary.target_executions,0);
  // The sole real Node child is final and performs no payload/hash work.
  await assert.rejects(()=>r.run(process.execPath,['-e','setInterval(()=>{},1000)'],{env:runtime,cwd,timeout:50,stdoutLimit:0}),/time-limit/);
  assert.ok(r.drained);assert.equal(r.kills,1);assert.ok(b.hashed<=64*1024**2);assert.ok(b.streamed<=1024**2);
 }finally{if(r.drained)fs.rmSync(cwd,{recursive:true});}
});
