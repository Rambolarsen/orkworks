import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
const file = new URL('./pair-windows-metadata-evidence.mjs', import.meta.url);
const api = await import(file);
test('rerun or foreign workflow is rejected before acquisition', () => {
  const good={GITHUB_ACTIONS:'true', GITHUB_REPOSITORY:'Rambolarsen/orkworks', GITHUB_EVENT_NAME:'workflow_dispatch',GITHUB_RUN_ATTEMPT:'1',EXPECTED_SHA:'a'.repeat(40),GITHUB_SHA:'a'.repeat(40),GITHUB_WORKFLOW_REF:'Rambolarsen/orkworks/.github/workflows/pair-windows-metadata-evidence.yml@refs/heads/main'};
  api.assertDispatch(good,'win32');
  for (const bad of [{GITHUB_RUN_ATTEMPT:'2'},{GITHUB_EVENT_NAME:'pull_request'},{GITHUB_SHA:'b'.repeat(40)},{GITHUB_WORKFLOW_REF:'foreign'},{GITHUB_REPOSITORY:'other/repo'}]) assert.throws(()=>api.assertDispatch({...good,...bad},'win32'));
  assert.throws(()=>api.assertDispatch(good,'darwin'));
});
test('unknown Cargo configuration is refused at every checked ancestor',  t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'metadata-config-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  const root=path.join(dir,'repo');fs.mkdirSync(root);api.assertConfiguration(root);
  fs.mkdirSync(path.join(dir,'.cargo'));fs.writeFileSync(path.join(dir,'.cargo/config.toml'),'[source.crates-io]');
  assert.throws(()=>api.assertConfiguration(root));
});
test('Cargo children receive no inherited flags, credentials or proxy configuration', ()=>{
  const env=api.childEnvironment({SystemRoot:'C:/Windows',TEMP:'scratch',GH_TOKEN:'secret',RUSTFLAGS:'--cfg bad',HTTP_PROXY:'untrusted',CARGO_REGISTRIES_CRATES_IO_INDEX:'foreign'},'/tools/rustc','/owned/cache','/owned/target');
  assert.equal(env.RUSTC,'/tools/rustc');assert.equal(env.CARGO_HOME,'/owned/cache');
  for(const key of ['GH_TOKEN','RUSTFLAGS','HTTP_PROXY','CARGO_REGISTRIES_CRATES_IO_INDEX']) assert.equal(env[key],undefined);
});
test('only public locked registry dependencies are admitted', ()=>{
  api.assertLock('[[package]]\nname = "orkworksd"\nversion = "0.2.0"\n\n[[package]]\nname = "other"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "'+ 'a'.repeat(64)+'"\n');
  for(const source of ['git+https://github.com/a/b','registry+https://private.test/index']) assert.throws(()=>api.assertLock('[[package]]\nname = "other"\nsource = "'+source+'"\n'));
  assert.throws(()=>api.assertLock('[[package]]\nname = "foreign-path"\n'));
});
test('failed fetch is terminal, with no metadata fallback',  async()=>{
  const calls=[];const out=await api.acquire(async args=>{calls.push(args);return {code:101,stdout:'',stderr:'failed'};});
  assert.equal(out.success,false);assert.equal(out.stage,'fetch');assert.equal(calls.length,1);
  assert.deepEqual(calls[0],['fetch','--locked','--target','x86_64-pc-windows-msvc','--manifest-path','crates/orkworksd/Cargo.toml','--config','net.retry=0']);
});
test('metadata stays offline and incomplete JSON never counts as success', async()=>{
  const calls=[];const out=await api.acquire(async args=>{calls.push(args);return {code:0,stdout:args[0]==='fetch'?'':'{"version":1,"resolve":null}',stderr:''};});
  assert.equal(out.success,false);assert.equal(out.stage,'metadata');assert.equal(calls.length,2);
  assert.deepEqual(calls[1],['metadata','--locked','--offline','--format-version','1','--filter-platform','x86_64-pc-windows-msvc','--manifest-path','crates/orkworksd/Cargo.toml']);
});
test('complete metadata retains target edges and actual features without inventing build proof', async()=>{
  const graph={version:1,packages:[{id:'root',name:'orkworksd',version:'0.2.0',source:null},{id:'r',name:'rusqlite',version:'0.32.1',source:'registry+https://github.com/rust-lang/crates.io-index'},{id:'s',name:'libsqlite3-sys',version:'0.30.1',source:'registry+https://github.com/rust-lang/crates.io-index'}],resolve:{root:'root',nodes:[{id:'root',features:[],deps:[{pkg:'r',dep_kinds:[{kind:null,target:null}]}]},{id:'r',features:['bundled','hooks'],deps:[{pkg:'s',dep_kinds:[{kind:null,target:null}]}]},{id:'s',features:['bundled'],deps:[]}]}};
  const out=await api.acquire(async args=>({code:0,stdout:args[0]==='fetch'?'':JSON.stringify(graph),stderr:''}));
  assert.equal(out.success,true);assert.deepEqual(out.sqlite.map(x=>x.features),[['bundled','hooks'],['bundled']]);assert.equal(out.qualified,undefined);
});
test('evidence collection rejects oversize output and unallowlisted files', ()=>{
  assert.throws(()=>api.assertEvidence([{name:'metadata.json',bytes:32*1024*1024+1}]));
  assert.throws(()=>api.assertEvidence([{name:'credentials',bytes:10}]));
  api.assertEvidence([{name:'result.json',bytes:100},{name:'metadata.json',bytes:100}]);
});

test('oversize artifacts produce only a bounded failure receipt', t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'metadata-evidence-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  assert.equal(typeof api.collectEvidence,'function');
  const result={success:true,stage:'complete'};
  api.collectEvidence(dir,result,'x'.repeat(32*1024*1024+1),'');
  assert.equal(result.success,false);
  assert.deepEqual(fs.readdirSync(dir),['result.json']);
  assert.equal(JSON.parse(fs.readFileSync(path.join(dir,'result.json'),'utf8')).reason,'evidence-limit');
});

test('a distinct second dispatch cannot acquire under the same allowance',()=>{
  assert.equal(typeof api.assertFirstRun,'function');
  api.assertFirstRun({total_count:1,workflow_runs:[{id:123,run_attempt:1,head_sha:'a'.repeat(40)}]},'123','a'.repeat(40));
  assert.throws(()=>api.assertFirstRun({total_count:2,workflow_runs:[{id:124,run_attempt:1,head_sha:'a'.repeat(40)},{id:123,run_attempt:1,head_sha:'a'.repeat(40)}]},'124','a'.repeat(40)));
  assert.throws(()=>api.assertFirstRun({total_count:1,workflow_runs:[{id:123,run_attempt:1,head_sha:'a'.repeat(40)}]},'124','a'.repeat(40)));
});
test('a complete but drifting graph is retained as failed evidence',async()=>{
  const graph=JSON.stringify({version:1,packages:[{id:'root',name:'orkworksd'},{id:'r',name:'rusqlite',version:'unexpected'}],resolve:{root:'root',nodes:[{id:'root'}]}});
  const out=await api.acquire(async args=>({code:0,stdout:args[0]==='fetch'?'':graph,stderr:''}));
  assert.equal(out.success,false);assert.equal(out.metadata,graph);
});

test('Windows Git filtering preserves the frozen input bytes',()=>{
  for(const [name,digest] of Object.entries({
    'crates/orkworksd/Cargo.toml':'0134c524d6e92d0f01f3cf3953f687c1ee0b570e54da879659061238aa615cc8',
    'crates/orkworksd/Cargo.lock':'40845c5bdf0a4ed869e93af1de572220711f0d2fabf31f546488b1cdbab28ded',
    'crates/orkworksd/build.rs':'1111ea2be9dfa3ddeb9402608723b50b01bd304a81d3de63d7d1427fe7915edf',
    'rust-toolchain.toml':'d4d89c66d4e571b2f86ef1c5484191acca0465a101b6c46083eea9537e69fc25',
  })) {
    const filtered=spawnSync('git',['-c','core.autocrlf=true','cat-file','--filters','HEAD:'+name]);
    assert.equal(filtered.status,0);assert.equal(crypto.createHash('sha256').update(filtered.stdout).digest('hex'),digest,name);
  }
});
