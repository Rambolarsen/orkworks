import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const registry = 'registry+https://github.com/rust-lang/crates.io-index';
const pins = {
  'crates/orkworksd/Cargo.toml':'0134c524d6e92d0f01f3cf3953f687c1ee0b570e54da879659061238aa615cc8',
  'crates/orkworksd/Cargo.lock':'40845c5bdf0a4ed869e93af1de572220711f0d2fabf31f546488b1cdbab28ded',
  'rust-toolchain.toml':'d4d89c66d4e571b2f86ef1c5484191acca0465a101b6c46083eea9537e69fc25',
  'crates/orkworksd/build.rs':'1111ea2be9dfa3ddeb9402608723b50b01bd304a81d3de63d7d1427fe7915edf',
};
const hash = data => crypto.createHash('sha256').update(data).digest('hex');
const fileHash = file => hash(fs.readFileSync(file));
const requireThat = (condition, message) => { if (!condition) throw new Error(message); };
export function assertDispatch(env, platform) {
  requireThat(platform === 'win32' && env.GITHUB_ACTIONS === 'true' &&
    env.GITHUB_REPOSITORY === 'Rambolarsen/orkworks' && env.GITHUB_EVENT_NAME === 'workflow_dispatch' &&
    env.GITHUB_RUN_ATTEMPT === '1' && /^[a-f0-9]{40}$/.test(env.EXPECTED_SHA ?? '') &&
    env.GITHUB_SHA === env.EXPECTED_SHA &&
    env.GITHUB_WORKFLOW_REF === 'Rambolarsen/orkworks/.github/workflows/pair-windows-metadata-evidence.yml@refs/heads/main', 'dispatch-denied');
}
export function assertFirstRun(history,id,sha) {
  requireThat(history.total_count===1 && history.workflow_runs?.length===1 && String(history.workflow_runs[0].id)===id && history.workflow_runs[0].run_attempt===1 && history.workflow_runs[0].head_sha===sha,'allowance-already-consumed-or-unknown');
}
export function assertConfiguration(root) {
  const dirs = [path.join(root,'crates'),path.join(root,'crates/orkworksd')];
  for(let dir=path.resolve(root);;dir=path.dirname(dir)) {
    dirs.push(dir); if(path.dirname(dir)===dir) break;
  }
  for(const dir of dirs) for(const name of ['config','config.toml'])
    requireThat(!fs.existsSync(path.join(dir,'.cargo',name)),'unknown-cargo-configuration');
}
export function assertLock(text) {
  const blocks=text.split('[[package]]').slice(1);
  requireThat(blocks.length>0,'empty-lock');
  for(const block of blocks) {
    const name=/^name = "([^"]+)"$/m.exec(block)?.[1];
    const source=/^source = "([^"]+)"$/m.exec(block)?.[1];
    requireThat(source ? source===registry && /^checksum = "[a-f0-9]{64}"$/m.test(block) : name==='orkworksd','unapproved-source');
  }
}
export function childEnvironment(inherited, rustc, cache, target) {
  const env={};
  for(const key of ['SystemRoot','WINDIR','TEMP','TMP']) if(inherited[key]) env[key]=inherited[key];
  env.PATH=[path.dirname(rustc),path.join(inherited.SystemRoot ?? '', 'System32')].join(path.delimiter);
  return {...env,RUSTC:rustc,CARGO_HOME:cache,CARGO_TARGET_DIR:target,CARGO_NET_RETRY:'0',CARGO_HTTP_TIMEOUT:'30',CARGO_REGISTRIES_CRATES_IO_PROTOCOL:'sparse',CARGO_TERM_COLOR:'never'};
}
function summarize(meta) {
  requireThat(meta.version===1 && meta.resolve?.root && Array.isArray(meta.resolve.nodes) && meta.resolve.nodes.length>0 && Array.isArray(meta.packages),'incomplete-graph');
  const root=meta.packages.find(p=>p.id===meta.resolve.root);
  requireThat(root?.name==='orkworksd','wrong-root');
  const families=[];
  for(const [name,version] of [['rusqlite','0.32.1'],['libsqlite3-sys','0.30.1']]) {
    const packages=meta.packages.filter(p=>p.name===name);
    requireThat(packages.length===1 && packages[0].version===version && packages[0].source===registry,'sqlite-version-drift');
    const node=meta.resolve.nodes.find(n=>n.id===packages[0].id);
    requireThat(node && Array.isArray(node.features) && node.features.every(x=>typeof x==='string') && Array.isArray(node.deps),'missing-sqlite-node');
    families.push({name,version,features:node.features,deps:node.deps});
  }
  return families;
}
export async function acquire(run) {
  const out={success:false,stage:'fetch',fetch_invocations:1,metadata_invocations:0};
  const fetch=await run(['fetch','--locked','--target','x86_64-pc-windows-msvc','--manifest-path','crates/orkworksd/Cargo.toml','--config','net.retry=0']);
  out.fetch_exit=fetch.code;out.diagnostics=fetch.stderr;
  if(fetch.code!==0) return out;
  out.stage='metadata';out.metadata_invocations=1;
  const metadata=await run(['metadata','--locked','--offline','--format-version','1','--filter-platform','x86_64-pc-windows-msvc','--manifest-path','crates/orkworksd/Cargo.toml']);
  out.metadata_exit=metadata.code;out.diagnostics+='\n'+metadata.stderr;
  out.metadata=metadata.stdout;
  if(metadata.code!==0) return out;
  try { out.sqlite=summarize(JSON.parse(metadata.stdout)); }
  catch { return out; }
  out.success=true;out.stage='source-check';return out;
}
export function assertEvidence(files) {
  requireThat(files.every(f=>['result.json','metadata.json','diagnostics.txt'].includes(f.name) && Number.isSafeInteger(f.bytes) && f.bytes>=0) && files.reduce((sum,f)=>sum+f.bytes,0)<=32*1024*1024,'evidence-limit');
}
export function collectEvidence(directory,result,metadata,diagnostics) {
  let files={'result.json':JSON.stringify(result,null,2),'diagnostics.txt':diagnostics};
  if(metadata!==undefined) files['metadata.json']=metadata;
  try {assertEvidence(Object.entries(files).map(([name,data])=>({name,bytes:Buffer.byteLength(data)})));}
  catch {
    result.success=false;result.reason='evidence-limit';
    files={'result.json':JSON.stringify({schema:1,success:false,stage:result.stage,reason:'evidence-limit',implementation_sha:result.implementation_sha,fetch_invocations:result.fetch_invocations,metadata_invocations:result.metadata_invocations})};
  }
  for(const [name,data] of Object.entries(files)) fs.writeFileSync(path.join(directory,name),data);
}
function assertPins(root) {
  for(const [name,digest] of Object.entries(pins)) requireThat(fileHash(path.join(root,name))===digest,'frozen-input-drift');
}
// Only the approved runner uses this process boundary. No shell or inherited Cargo environment.
function processRun(executable,args,options,deadline) {
  return new Promise(resolve=>{
    const child=spawn(executable,args,{...options,stdio:['ignore','pipe','pipe'],windowsHide:true});
    const stdout=[],stderr=[];let bytes=0,failed=false;
    const stop=()=>{
      failed=true;
      if(child.pid) spawnSync(path.join(options.env.SystemRoot,'System32/taskkill.exe'),['/PID',String(child.pid),'/T','/F'],{stdio:'ignore',timeout:10000,windowsHide:true});
      child.kill();
    };
    const timer=setTimeout(stop,Math.max(1,deadline-Date.now()));
    child.stdout.on('data',b=>{bytes+=b.length;if(bytes>16*1024*1024) stop();else stdout.push(b);});
    child.stderr.on('data',b=>{bytes+=b.length;if(bytes>16*1024*1024 || stderr.reduce((n,x)=>n+x.length,0)+b.length>65536) stop();else stderr.push(b);});
    child.on('error',()=>{failed=true;});
    child.on('close',code=>{clearTimeout(timer);resolve({code:failed?-1:code,stdout:Buffer.concat(stdout).toString('utf8'),stderr:Buffer.concat(stderr).toString('utf8')});});
  });
}
async function main() {
  assertDispatch(process.env,process.platform);
  const root=fs.realpathSync(process.cwd());
  requireThat(root===fs.realpathSync(process.env.GITHUB_WORKSPACE),'wrong-checkout');
  const scratch=path.join(process.env.RUNNER_TEMP,'ork-pair-windows-metadata');
  fs.mkdirSync(scratch); // Existing scratch is a consumed attempt, never reused.
  const evidence=path.join(scratch,'evidence');fs.mkdirSync(evidence);
  const result={schema:1,proposal_merge:'439767e1366b867634e5a38c7a929b5bac93bec6',implementation_sha:process.env.EXPECTED_SHA,run_id:process.env.GITHUB_RUN_ID,started_utc:new Date().toISOString(),stage:'preflight',success:false,fetch_invocations:0,metadata_invocations:0,compilations:0,target_executions:0};
  let diagnostics='';let metadataForEvidence;
  try {
    assertPins(root);assertConfiguration(root);assertLock(fs.readFileSync(path.join(root,'crates/orkworksd/Cargo.lock'),'utf8'));
    result.inputs=pins;
    result.stage='dispatch-history';
    requireThat(process.env.GITHUB_TOKEN,'run-history-unavailable');
    const response=await fetch('https://api.github.com/repos/Rambolarsen/orkworks/actions/workflows/pair-windows-metadata-evidence.yml/runs?event=workflow_dispatch&per_page=100', {headers:{Authorization:`Bearer ${process.env.GITHUB_TOKEN}`,Accept:'application/vnd.github+json'},signal:AbortSignal.timeout(30000)});
    requireThat(response.ok,'run-history-unavailable');
    const history=await response.json();
    assertFirstRun(history,process.env.GITHUB_RUN_ID,process.env.EXPECTED_SHA);
    result.dispatch_history_count=history.total_count;
    requireThat(/^[0-9]+(?:\.[0-9]+){1,3}$/.test(process.env.ImageVersion ?? '') && /^[a-zA-Z0-9_-]+$/.test(process.env.ImageOS ?? ''),'image-identity');
    result.image={version:process.env.ImageVersion,os:process.env.ImageOS};
    const bin=path.join(process.env.USERPROFILE,'.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin');
    const cargo=path.join(bin,'cargo.exe'),rustc=path.join(bin,'rustc.exe');
    for(const file of [cargo,rustc]) requireThat(fs.lstatSync(file).isFile() && !fs.lstatSync(file).isSymbolicLink(),'installed-tool-unavailable');
    result.tools={cargo_sha256:fileHash(cargo),rustc_sha256:fileHash(rustc)};
    const cache=path.join(scratch,'cargo-home');fs.mkdirSync(cache);
    const env=childEnvironment(process.env,rustc,cache,path.join(scratch,'target'));
    const deadline=Date.now()+12*60*1000;
    const run=(exe,args)=>processRun(exe,args,{cwd:root,env},deadline);
    result.stage='tool-identities';
    const cargoVersion=await run(cargo,['-Vv']);const rustVersion=await run(rustc,['-vV']);
    requireThat(cargoVersion.code===0 && rustVersion.code===0,'tool-identity-failed');
    result.tools.cargo=cargoVersion.stdout;result.tools.rustc=rustVersion.stdout;
    const release=s=>/^release: (\S+)$/m.exec(s)?.[1];
    requireThat(release(cargoVersion.stdout) && release(cargoVersion.stdout)===release(rustVersion.stdout) && /^host: x86_64-pc-windows-msvc$/m.test(cargoVersion.stdout) && /^host: x86_64-pc-windows-msvc$/m.test(rustVersion.stdout),'toolchain-mismatch');
    result.stage='acquisition';
    const out=await acquire(args=>{result.commands??=[];result.commands.push(args);if(args[0]==='fetch')result.fetch_invocations++;else result.metadata_invocations++;return run(cargo,args);});
    diagnostics=out.diagnostics;delete out.diagnostics;
    const metadata=out.metadata;delete out.metadata;Object.assign(result,out);
    if(metadata!==undefined) {metadataForEvidence=metadata;result.metadata_sha256=hash(metadata);result.metadata_bytes=Buffer.byteLength(metadata);}
    requireThat(out.success,'acquisition-failed');result.success=false;
    result.acquired_sources={};
    for(const [name,digest] of Object.entries({'sqlite3/sqlite3.c':'c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d','build.rs':'11bebc3787657749279375ab2e875e52259f995dce8916b4907103975e2a76ee'})) {
      const dirs=fs.readdirSync(path.join(cache,'registry/src'),{withFileTypes:true}).filter(d=>d.isDirectory());
      const candidates=dirs.map(d=>path.join(cache,'registry/src',d.name,'libsqlite3-sys-0.30.1',name)).filter(f=>fs.existsSync(f));
      result.acquired_sources[name]={expected:digest,observed:candidates.map(fileHash)};
      requireThat(candidates.length===1 && result.acquired_sources[name].observed[0]===digest,'acquired-source-drift');
    }
    metadataForEvidence=metadata;
    result.metadata_sha256=hash(metadata);result.metadata_bytes=Buffer.byteLength(metadata);result.stage='complete';result.success=true;
  } catch(error) {
    result.success=false;result.reason=['allowance-already-consumed-or-unknown','run-history-unavailable','frozen-input-drift','unknown-cargo-configuration','unapproved-source','image-identity','installed-tool-unavailable','tool-identity-failed','toolchain-mismatch','acquisition-failed','acquired-source-drift'].includes(error.message)?error.message:'unavailable';
  } finally {
    try {assertPins(root);} catch {result.success=false;result.reason='frozen-input-drift';}
    for(const value of [root,scratch,process.env.USERPROFILE]) if(value) diagnostics=diagnostics.split(value).join('[runner]');
    // Cleanup failures are terminal and must be reflected in the retained receipt.
    try {for(const name of ['cargo-home','target']) fs.rmSync(path.join(scratch,name),{recursive:true,force:true});result.cleanup='owned-scratch-removed';}
    catch {result.success=false;result.cleanup='unavailable';}
    result.ended_utc=new Date().toISOString();
    collectEvidence(evidence,result,metadataForEvidence,diagnostics);
  }
  if(!result.success) process.exitCode=1;
}
if(process.argv[1] && import.meta.url===pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch(()=>{console.error('Metadata evidence unavailable; no retry authorized');process.exitCode=1;});
}
