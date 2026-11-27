// Owned synthetic fixture controller. Never import a spent collector main.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { Decoder, settle, encodeResult, MiB } from './pair-reader-diagnostic-protocol.mjs';
const here=path.dirname(fileURLToPath(import.meta.url));
const labels=['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'];
const lexical=(a,b)=>a<b?-1:a>b?1:0;
function need(condition,why) {if(!condition)throw Error(why);}
function plain(p) {
 p=path.resolve(p);
 if(process.platform==='win32')need(/^[A-Za-z]:\\/.test(p)&&!p.slice(3).includes(':'),'input-denied');
 for(let q=p;;q=path.dirname(q)) {need(!fs.lstatSync(q).isSymbolicLink(),'input-denied');if(path.dirname(q)===q)break;}
 return p;
}
function stamp(s) {return `${s.dev}:${s.ino}:${s.size}:${s.mtimeNs}:${s.ctimeNs}:${s.mode}`;}
function heldRead(file,cap,ledger) {
 file=plain(file); const named=fs.lstatSync(file,{bigint:true});const fd=fs.openSync(file,fs.constants.O_RDONLY|(fs.constants.O_NOFOLLOW??0));
 try {
  const before=fs.fstatSync(fd,{bigint:true});need(stamp(before)===stamp(named),'identity-drift');need(before.isFile()&&before.size<=BigInt(cap),'input-limit');
  const n=Number(before.size);ledger.chargeHash(n); const b=Buffer.alloc(n);
  let at=0;while(at<n){const got=fs.readSync(fd,b,at,n-at,at);need(got>0,'identity-drift');at+=got;}
  need(stamp(before)===stamp(fs.fstatSync(fd,{bigint:true})),'identity-drift');
  need(stamp(before)===stamp(fs.lstatSync(plain(file),{bigint:true})),'identity-drift');
  return b;
 } finally {fs.closeSync(fd);}
}
function digest(bytes,ledger) {ledger.chargeHash(bytes.length);return crypto.createHash('sha256').update(bytes).digest('hex');}
function put(file,bytes,ledger) {
 const b=Buffer.isBuffer(bytes)?bytes:Buffer.from(bytes);
 ledger.scratch(b.length);plain(path.dirname(file));fs.writeFileSync(file,b,{flag:'wx'});
}
function tar(entries) {
 const parts=[];
 for(const [name,data,type='0'] of entries) {
  const h=Buffer.alloc(512);h.write(name,0,100,'ascii');h.write('0000644\0',100);h.write('0000000\0',108);h.write('0000000\0',116);
  h.write(data.length.toString(8).padStart(11,'0')+'\0',124);h.write('00000000000\0',136);h.fill(32,148,156);h.write(type,156);if(type==='2')h.write('target',157,100,'ascii');
  h.write('ustar\0',257);h.write('00',263);let sum=0;for(const b of h)sum+=b;h.write(sum.toString(8).padStart(6,'0')+'\0 ',148);
  parts.push(h,data,Buffer.alloc((512-data.length%512)%512));
 }
 parts.push(Buffer.alloc(1024));const all=Buffer.concat(parts);need(all.length<=4*MiB,'input-limit');return zlib.gzipSync(all);
}
function fixtures(owned,ledger) {
 const roots=Object.fromEntries(labels.map(l=>[l,path.join(owned,'fixtures',l==='registry'?'registry/src':l)]));
 for(const root of Object.values(roots))fs.mkdirSync(root,{recursive:true});
 const pkg=path.join(roots.registry,'synthetic-index','fixture-1.0.0');fs.mkdirSync(pkg,{recursive:true});
 const cache=path.join(owned,'fixtures/registry/cache/synthetic-index');fs.mkdirSync(cache,{recursive:true});
 const members=new Map();
 members.set('Cargo.toml',Buffer.from('[package]\nname = "fixture"\nversion = "1.0.0"\nbuild = false\n'));
 members.set('empty',Buffer.alloc(0));
 const large=Buffer.alloc(MiB);for(let i=0;i<large.length;i++)large[i]=i%251+1;members.set('large',large);
 for(let i=0;i<1024;i++)members.set('m'+String(i).padStart(4,'0'),Buffer.from([i%251+1]));
 const goldens=[];
 for(const [name,b] of [...members].sort(([a],[b])=>lexical(a,b))) {
  put(path.join(pkg,name),b,ledger);
  goldens.push(['registry','synthetic-index/fixture-1.0.0/'+name,b.length,digest(b,ledger)]);
 }
 for(const label of labels.filter(x=>x!=='registry')) {
  const b=Buffer.from('owned-'+label+'\n');put(path.join(roots[label],'tiny'),b,ledger);goldens.push([label,'tiny',b.length,digest(b,ledger)]);
 }
 const archive=tar([...members].map(([n,b])=>['fixture-1.0.0/'+n,b]));need(archive.length<=MiB,'input-limit');
 put(path.join(cache,'fixture-1.0.0.crate'),archive,ledger);
 const catalog=[1,'fixture@1.0.0',digest(archive,ledger),[...members].sort(([a],[b])=>lexical(a,b)).map(([n,b])=>[n,digest(b,ledger)]),null,null];
 const catBytes=Buffer.from(JSON.stringify(catalog));need(catBytes.length<=256*1024,'input-limit');put(path.join(owned,'catalog.json'),catBytes,ledger);
 const catalogHash=digest(catBytes,ledger);
 fs.mkdirSync(path.join(owned,'guards'));put(path.join(owned,'guards/tiny'),Buffer.from([42]),ledger);
 // All guard data are separate from immutable positive inputs.
 const guardTar=[
 ['duplicate',[['fixture-1.0.0/x',Buffer.from('a')],['fixture-1.0.0/x',Buffer.from('b')]]],
 ['case-fold',[['fixture-1.0.0/x',Buffer.from('a')],['fixture-1.0.0/X',Buffer.from('b')]]],
 ['type',[['fixture-1.0.0/x',Buffer.alloc(0),'2']]],
 ['traversal',[['fixture-1.0.0/../x',Buffer.from('a')]]],
 ['extra',[['fixture-1.0.0/extra',Buffer.from('a')]]],
 ['content',[['fixture-1.0.0/x',Buffer.from('b')]]],
 ['missing',[]],
 ['valid',[['fixture-1.0.0/x',Buffer.from('a')]]]
 ];
 for(const [n,entries] of guardTar)put(path.join(owned,'guards',n+'.crate'),tar(entries),ledger);
 return {roots,goldens:goldens.sort((a,b)=>lexical(a[0],b[0])||lexical(a[1],b[1])),catalog,catalogHash};
}
function witnessTree(owned) {
 const rows=[];let files=0,dirs=0,bytes=0;const todo=[path.join(owned,'fixtures'),path.join(owned,'guards')];
 while(todo.length) {
  const p=plain(todo.pop());const s=fs.lstatSync(p,{bigint:true});rows.push([p,p===path.join(owned,'guards')?`${s.dev}:${s.ino}:${s.mode}`:stamp(s)]);
  if(s.isDirectory()) {need(++dirs<=256,'input-limit');for(const n of fs.readdirSync(p))todo.push(path.join(p,n));}
  else {need(s.isFile()&&++files<=2048&&s.size<=BigInt(MiB),'input-limit');bytes+=Number(s.size);need(bytes<=4*MiB,'input-limit');}
 }
 return rows.sort(([a],[b])=>lexical(a,b));
}
function seals(owned,ledger) {
 const files=['pair-reader-diagnostic-protocol.mjs','pair-reader-diagnostic.mjs','pair-reader-diagnostic.test.mjs','pair-reader-diagnostic.ps1','pair-reader-diagnostic.test.ps1'].map(n=>path.join(here,n));
 files.push(path.join(owned,'catalog.json'));
 let total=0;
 return files.map(f=>{const b=heldRead(f,MiB-total,ledger);total+=b.length;need(total<=MiB,'source-limit');return digest(b,ledger);});
}
function childEnv(owned) {
 const env={TEMP:owned,TMP:owned,ORK_READER_SYNTHETIC:'synthetic-v1'};
 if(process.platform==='win32')Object.assign(env,{SystemRoot:'C:\\Windows',WINDIR:'C:\\Windows',PATH:'C:\\Windows\\System32'});
 else env.PATH='/usr/bin:/bin';
 return env;
}
export async function watchChild(child,{decoder,ledger,timeout,terminate}) {
 ledger.active();let closed=false,code=null,stopped=false,spawnFailed=false,timer;
 let resolve;const done=new Promise(r=>{resolve=r;});
 const fail=why=>{decoder.fail(why);if(stopped)return;stopped=true;child.stdout?.destroy();child.stderr?.destroy();resolve();};
 child.on('error',()=>{spawnFailed=!child.pid;fail('spawn-error');});
 child.on('close',c=>{closed=true;code=c;resolve();});
 child.stdout.on('data',b=>{decoder.feed(b);if(decoder.failure)fail(decoder.failure);});
 child.stderr.on('data',b=>{try{ledger.stream(b.length,decoder.source);}catch{}fail('stderr');});
 timer=setTimeout(()=>fail('timeout'),Math.max(1,Math.min(timeout,120000-ledger.elapsed())));
 await done;clearTimeout(timer);
 let drained=closed||spawnFailed;
 if(!drained) {try{drained=await terminate(child);}catch{drained=false;}}
 return {code,drained};
}
async function runChild({command,args,owned,decoder,ledger,timeout}) {
 ledger.active();
 const child=spawn(command,args,{cwd:owned,env:childEnv(owned),shell:false,windowsHide:true,stdio:['ignore','pipe','pipe']});
 return watchChild(child,{decoder,ledger,timeout,terminate:async current=>{
  need(current===child&&Number.isSafeInteger(child.pid)&&child.pid>0,'termination-owner');
  need(++ledger.terminations<=1,'termination-limit');
  const childClose=new Promise(r=>child.once('close',()=>r(true)));
  let killer,timer;
  const deadline=new Promise(r=>{timer=setTimeout(()=>r(false),Math.min(10000,Math.max(1,240000-ledger.elapsed())));});
  try {
   let complete;
   if(process.platform==='win32') {
    killer=spawn('C:/Windows/System32/taskkill.exe',['/PID',String(child.pid),'/T','/F'],{cwd:owned,env:childEnv(owned),shell:false,windowsHide:true,stdio:['ignore','pipe','pipe']});
    for(const stream of [killer.stdout,killer.stderr])stream.on('data',b=>{try{ledger.stream(b.length,'child');}catch{decoder.fail('stream-limit');}});
    const killed=new Promise(r=>{killer.once('error',()=>r(false));killer.once('close',c=>r(c===0));});
    complete=Promise.all([childClose,killed]).then(xs=>xs.every(Boolean));
   } else {child.kill('SIGKILL');complete=childClose;}
   const drained=await Promise.race([complete,deadline]);
   if(!drained){child.unref();killer?.unref();killer?.stdout?.destroy();killer?.stderr?.destroy();}
   return drained;
  } finally {clearTimeout(timer);}
 }});
}
export function admitAndSettle(decoder,close,ledger,reservation,verify,verifySources=()=>{}) {
 let outputValid=false,sourceValid=false;
 try {
  decoder.end();
  need(!decoder.failure&&close.drained&&((close.code===0&&decoder.terminal==='ok')||(close.code!==0&&Number.isInteger(close.code)&&decoder.terminal==='error')),'native-'+(decoder.failure||'bad-close'));
  verifySources();sourceValid=true;
  if(decoder.terminal==='error')throw Error('native-'+decoder.trace.at(-1)[1][9]);
  verify();outputValid=true;
 } finally {
  const outcome=settle(decoder,{...close,outputValid,sourceValid});
  ledger.settle(reservation,outcome.charged,outcome.unknown_work);
 }
}
const guardDispositions=[
 'traversal:format-denied','absolute:format-denied','backslash:format-denied',
 'ads:format-denied','empty:format-denied','dot:format-denied','double-slash:format-denied',
 'nul:format-denied','trailing-dot:format-denied','reparse:input-denied','path-limit:format-denied',
 'foreign-root:input-denied','unc:input-denied','file-limit:input-limit','short-stream:identity-drift',
 'hash-limit:hash-limit','growth:identity-drift','nested-lease:fixture-assertion',
 'encoded-limit:output-limit','wrong-path-identity:identity-drift','duplicate:input-limit',
 'case-fold:input-limit','type:format-denied','tar-traversal:format-denied','extra:format-denied',
 'content:identity-drift','missing:format-denied','wrong-archive:identity-drift',
 'member-limit:input-limit','expanded-limit:input-limit','extra-file:format-denied','helper:format-denied'
];
export function identityMatches(id,stat) {
 return typeof id==='string'&&/^[a-f0-9]{8}:[a-f0-9]{16}$/.test(id)&&BigInt('0x'+id.slice(0,8))===stat.dev&&BigInt('0x'+id.slice(9))===stat.ino;
}
export function finalizeOwned(result,{drained,owned,remove,publish}) {
 if(drained) {try{remove();result.cleanup='removed';}catch{result.cleanup='failed';result.tests_passed=false;result.owned_path=owned;}}
 else {result.cleanup='retained-undrained';result.tests_passed=false;result.owned_path=owned;}
 try{publish();}catch{result.reason='publication-failed';result.tests_passed=false;}
}
function admit(value,mode,fixture,owned,ledger,decoder) {
 need(value&&typeof value==='object'&&!Array.isArray(value),'output-invalid');
 if(mode==='guards') {
  need(Object.keys(value).sort().join(',')==='catalog_sha256,h,runtime_version,vectors','output-invalid');
  need(typeof value.runtime_version==='string'&&/^7\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(value.runtime_version)&&value.runtime_version.length<=64,'output-invalid');
  need(value.h===decoder.counters[0]&&value.catalog_sha256===fixture.catalogHash,'output-invalid');
  need(isDeepStrictEqual(value.vectors,guardDispositions),'output-invalid');return value;
 }
 need(Object.keys(value).sort().join(',')==='archives,directories,directory_count,file_bytes,file_count,files,hash_bytes,metrics','output-invalid');
 need(value.file_count===value.files.length&&value.directory_count===value.directories.length,'output-invalid');
 need(value.file_count===decoder.counters[1]&&value.directory_count===decoder.counters[2]&&value.archives.length===decoder.counters[3]&&value.hash_bytes===decoder.counters[0],'output-invalid');
 const actual=value.files.map(r=>{
  need(Object.keys(r).sort().join(',')==='bytes,file,id,root,sha256','output-invalid');
  need(labels.includes(r.root)&&typeof r.file==='string'&&!r.file.includes('..')&&!path.isAbsolute(r.file)&&!r.file.includes('\\'),'output-invalid');
  const p=path.join(fixture.roots[r.root],r.file);const stat=fs.lstatSync(plain(p),{bigint:true});
  need(identityMatches(r.id,stat),'identity-drift');
  const bytes=heldRead(p,MiB,ledger);need(bytes.length===r.bytes&&digest(bytes,ledger)===r.sha256,'identity-drift');
  return [r.root,r.file,r.bytes,r.sha256];
 });
 need(isDeepStrictEqual(actual,fixture.goldens),'golden-mismatch');
 need(value.file_bytes===actual.reduce((n,r)=>n+r[2],0),'output-invalid');
 const expectedDirs=[];
 for(const label of labels) {
  const todo=[fixture.roots[label]];
  while(todo.length) {
   const dir=plain(todo.pop());const st=fs.lstatSync(dir,{bigint:true});
   expectedDirs.push([label,path.relative(fixture.roots[label],dir).split(path.sep).join('/'),st]);
   for(const name of fs.readdirSync(dir)){const p=plain(path.join(dir,name));if(fs.lstatSync(p).isDirectory())todo.push(p);}
  }
 }
 expectedDirs.sort((a,b)=>lexical(a[0],b[0])||lexical(a[1],b[1]));
 need(value.directories.length===expectedDirs.length,'directory-invalid');
 for(const [i,r] of value.directories.entries()) {
  need(Object.keys(r).sort().join(',')==='file,id,root'&&r.root===expectedDirs[i][0]&&r.file===expectedDirs[i][1]&&identityMatches(r.id,expectedDirs[i][2]),'identity-drift');
 }
 const a=value.archives[0];
 const archive=path.join(owned,'fixtures/registry/cache/synthetic-index/fixture-1.0.0.crate');
 const archiveStat=fs.lstatSync(plain(archive),{bigint:true});
 need(value.archives.length===1&&Object.keys(a).sort().join(',')==='build_script,bytes,id,members,package,proc_macro,sha256'&&a.package==='fixture@1.0.0'&&a.bytes===Number(archiveStat.size)&&identityMatches(a.id,archiveStat)&&a.sha256===fixture.catalog[2]&&isDeepStrictEqual(a.members,fixture.catalog[3])&&a.build_script===null&&a.proc_macro===null,'archive-invalid');
 const m=value.metrics;need(Object.keys(m).sort().join(',')==='catalog_sha256,hash_bytes,maximum_leases,scratch_allocations,scratch_capacity_bytes,strategy','output-invalid');
 need(m.strategy===mode&&m.catalog_sha256===fixture.catalogHash&&m.maximum_leases===1&&Number.isSafeInteger(m.hash_bytes)&&m.hash_bytes>0&&m.hash_bytes<=value.hash_bytes,'metrics-invalid');
 need(Number.isSafeInteger(m.scratch_allocations)&&m.scratch_capacity_bytes===m.scratch_allocations*MiB&&(mode==='reuse'?m.scratch_allocations===1:m.scratch_allocations===4123),'metrics-invalid');
 return value;
}
export async function runSuite(ledger,{pureVectors}) {
 ledger.active();const owned=path.join(fs.realpathSync(os.tmpdir()),'ork-reader-diagnostic-'+crypto.randomBytes(16).toString('hex'));
 fs.mkdirSync(owned); // no recursive mkdir or reuse of an existing root.
 const ownerIdentity=fs.lstatSync(owned,{bigint:true});
 let marker,allDrained=true,initial;const used=new Set();const traces=[];
 const result={schema:1,evidence_kind:'synthetic-reader-diagnostic',tests_passed:false,
  candidate_evidence_complete:false,runtime_qualified:false,production_pin_accepted:false,target_executions:0,
  node_version:process.versions.node,pure_vectors:pureVectors,native:process.platform==='win32'?{status:'not-run'}:{status:'skipped',reason:'non-windows'},traces};
 try {
  for(const dir of ['requests','outputs'])fs.mkdirSync(path.join(owned,dir));
  put(path.join(owned,'owner'),Buffer.from('synthetic-v1'),ledger);marker=fs.openSync(path.join(owned,'owner'),'r');
  const markerStamp=stamp(fs.fstatSync(marker,{bigint:true}));
  const fixture=fixtures(owned,ledger);const witness=witnessTree(owned);initial=seals(owned,ledger);
  if(process.platform==='win32') {
   const outcomes=[];
   for(const mode of ['reference','reuse','guards']) {
    ledger.active();need(isDeepStrictEqual(witness,witnessTree(owned)),'identity-drift');
    const nonce=crypto.randomBytes(16).toString('hex');need(!used.has(nonce),'nonce-reuse');used.add(nonce);
    const grant=64*MiB-ledger.hashBytes-ledger.reserved-4*MiB;need(grant>0,'hash-limit');
    const reservation=ledger.reserve(grant);const req=JSON.stringify([1,'synthetic-v1',mode,nonce,grant]);
    need(Buffer.byteLength(req)<=4096,'request-limit');put(path.join(owned,'requests',nonce+'.json'),req,ledger);
    // Conservatively precharge the maximum output; no post-write capacity rescue.
    ledger.scratch(mode==='guards'?4096:MiB);
    const decoder=new Decoder({nonce,mode,grant,ledger});need(++ledger.nativeChildren<=3,'child-limit');
    const close=await runChild({command:'C:/Program Files/PowerShell/7/pwsh.exe',args:['-NoProfile','-NonInteractive','-File',path.join(here,'pair-reader-diagnostic.ps1'),'-Nonce',nonce],owned,decoder,ledger,timeout:30000});
    allDrained&&=close.drained;traces.push(decoder.trace);
    result.native={status:'failed',mode,exit_code:Number.isInteger(close.code)&&close.code>=0&&close.code<=255?close.code:null};
    let checked;
    admitAndSettle(decoder,close,ledger,reservation,()=>{
     const out=heldRead(path.join(owned,'outputs',nonce+'.json'),mode==='guards'?4096:MiB,ledger);
     let value;try{value=JSON.parse(out);}catch{throw Error('output-invalid');}
     checked=admit(value,mode,fixture,owned,ledger,decoder);
     need(isDeepStrictEqual(witness,witnessTree(owned)),'identity-drift');
    },()=>need(isDeepStrictEqual(initial,seals(owned,ledger)),'source-drift'));
    outcomes.push(checked);
   }
   const [reference,reuse,guards]=outcomes;
   const {metrics:rMetrics,...r}=reference,{metrics:uMetrics,...u}=reuse;
   need(isDeepStrictEqual(r,u)&&rMetrics.hash_bytes===uMetrics.hash_bytes,'pair-mismatch');
   result.native={status:'passed',powershell_version:guards.runtime_version,equality:true,reference_allocations:rMetrics.scratch_allocations,reuse_allocations:uMetrics.scratch_allocations,
    maximum_leases:uMetrics.maximum_leases,children:ledger.nativeChildren,guard_vectors:guards.vectors.length,guard_dispositions:guards.vectors,hash_bytes:reuse.hash_bytes};
  }
  // Only real failure child, always last. The script has no file or hash work.
  const nonce=crypto.randomBytes(16).toString('hex');need(!used.has(nonce),'nonce-reuse');used.add(nonce);
  const decoder=new Decoder({nonce,mode:'transport',grant:0,ledger});need(++ledger.nodeChildren<=1,'child-limit');
  const script='process.stdout.write(JSON.stringify([1,process.argv[1],1,"start","",0,0,0,0])+"\\n");setInterval(()=>{},1000)';
  const close=await runChild({command:process.execPath,args:['-e',script,nonce],owned,decoder,ledger,timeout:1000});
  allDrained&&=close.drained;const transport=settle(decoder,{...close});result.transport=transport;traces.push(decoder.trace);
  need(transport.reason==='timeout'&&transport.drained&&transport.trace.length===1&&isDeepStrictEqual(transport.trace[0][1].slice(2),[1,'start','',0,0,0,0]),'transport-fixture');
  need(isDeepStrictEqual(witness,witnessTree(owned)),'identity-drift');
  need(markerStamp===stamp(fs.fstatSync(marker,{bigint:true}))&&markerStamp===stamp(fs.lstatSync(plain(path.join(owned,'owner')),{bigint:true})),'identity-drift');
  result.tests_passed=true;
 } catch(e) {
  // No raw exception/path or partial manifest is published.
  const known=/^(active-timeout|input-denied|input-limit|identity-drift|hash-limit|scratch-limit|source-limit|output-invalid|golden-mismatch|archive-invalid|directory-invalid|metrics-invalid|pair-mismatch|transport-fixture|source-drift|request-limit|nonce-reuse|child-limit|native-[a-z-]+)$/;
  result.reason=known.test(e.message)?e.message:'fixture-failure';
 } finally {
  if(initial&&allDrained){try{need(isDeepStrictEqual(initial,seals(owned,ledger)),'source-drift');}catch{result.tests_passed=false;result.reason='source-drift';}}
  if(marker!==undefined){try{fs.closeSync(marker);}catch{allDrained=false;}}
  result.streams={child_bytes:ledger.childBytes,injected_bytes:ledger.injectedBytes};
  result.work={charged_bytes:ledger.hashBytes,unknown_work:ledger.unknownWork,fixture_accounting_bytes:ledger.modelBytes,fixture_unknown_grant_bytes:ledger.modelUnknownBytes,reserved_bytes:ledger.reserved};
  result.elapsed_ms=ledger.elapsed();result.terminations=ledger.terminations;
  finalizeOwned(result,{drained:allDrained,owned,remove:()=>{
   const current=fs.lstatSync(plain(owned),{bigint:true});
   need(current.isDirectory()&&current.dev===ownerIdentity.dev&&current.ino===ownerIdentity.ino,'identity-drift');
   fs.rmSync(owned,{recursive:true});
  },publish:()=>{
   ledger.chargeHash(65536); // Conservative final-result commitment precharge.
   result.work.charged_bytes=ledger.hashBytes;
   crypto.createHash('sha256').update(encodeResult(result)).digest();
  }});
 }
 return result;
}
