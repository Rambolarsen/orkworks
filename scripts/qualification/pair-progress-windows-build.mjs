import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import {
  assertFirstRun,
  assertConfiguration,
  assertLock,
} from "./pair-windows-metadata-evidence.mjs";
import {
  buildEnvironment,
  sqliteInvocation,
  assertSqliteSource,
  preprocessArguments,
  sqliteRlib,
  archiveMember,
  assertSqliteMap,
  oracleSource,
  packageBuildScript,
} from "./pair-fixed-windows-build.mjs";
const need = (v, r) => {
  if (!v) throw Error(r);
};
const sha = (b) => crypto.createHash("sha256").update(b).digest("hex");
const MiB = 1024 ** 2,
  GiB = 1024 ** 3;
const policy = JSON.parse(
  fs.readFileSync(
    new URL("./pair-progress-windows-build.inputs.json", import.meta.url),
  ),
);
const workflow = "pair-progress-windows-build.yml";
const python = "C:\\hostedtoolcache\\windows\\Python\\3.12.10\\x64\\python.exe";
const powershell = "C:\\Program Files\\PowerShell\\7\\pwsh.exe";
const sdkKey =
  "SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall/{45e335f3-c807-43b8-9ed1-43666cfcf939}";
const sdkName = "Windows Software Development Kit - Windows 10.0.26100.9457";
const kit = "C:/Program Files (x86)/Windows Kits/10/";
const authority = (complete) => ({
  candidate_evidence_complete: complete === true,
  runtime_qualified: false,
  production_pin_accepted: false,
  target_executions: 0,
});
const allowed = [
  "inputs.json",
  "cargo.jsonl",
  "diagnostics.txt",
  "compiler.json",
  "oracle.c",
  "oracle.i",
  "sqlite3.lib",
  "libsqlite3_sys.rlib",
  "orkworksd.exe",
  "candidate.map",
  "dependencies.txt",
  "archive-members.txt",
  "link-custody.json",
  "sdk-record.json",
];
import {WorkBudget,Decoder,settle,parseStrict,encodeResult} from './pair-progress-windows-build-protocol.mjs';
export {WorkBudget};
const policySha='1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6';
function safeName(file) {
  const full = path.resolve(file);
  for (let p = full; ; p = path.dirname(p)) {
    const s = fs.lstatSync(p);
    need(!s.isSymbolicLink(), "linked-input");
    if (path.dirname(p) === p) break;
  }
  need(
    path.resolve(fs.realpathSync(full)).toLowerCase() === full.toLowerCase(),
    "foreign-input",
  );
  return full;
}
function withSealed(file, budget, maximum, reader) {
  if(budget.failurePhase)budget.emergency();else budget.check();
  const full = safeName(file),
    named = fs.lstatSync(full, { bigint: true });
  need(named.isFile() && named.size <= BigInt(maximum), "file-limit");
  const expected = Number(named.size);
  budget.hash(expected);
  const fd = fs.openSync(
    full,
    fs.constants.O_RDONLY | (fs.constants.O_NOFOLLOW ?? 0),
  );
  try {
    const before = fs.fstatSync(fd, { bigint: true });
    need(
      before.isFile() &&
        before.size === named.size &&
        before.ino === named.ino &&
        before.dev === named.dev,
      "file-drift",
    );
    const result = reader(fd, expected);
    const after = fs.fstatSync(fd, { bigint: true }),
      current = fs.lstatSync(full, { bigint: true });
    need(
      !current.isSymbolicLink() &&
        current.isFile() &&
        after.size === before.size &&
        current.size === before.size &&
        current.ino === before.ino &&
        current.dev === before.dev &&
        after.mtimeNs === before.mtimeNs &&
        after.ctimeNs === before.ctimeNs,
      "file-drift",
    );
    safeName(full);
    return result;
  } finally {
    fs.closeSync(fd);
  }
}
function chunks(fd, size, budget, consume) {
  const buffer = Buffer.alloc(Math.min(MiB, size));
  let offset = 0;
  while (offset < size) {
    if(budget.failurePhase)budget.emergency();else budget.check();
    const n = fs.readSync(
      fd,
      buffer,
      0,
      Math.min(buffer.length, size - offset),
      offset,
    );
    need(n > 0, "file-short-read");
    consume(buffer.subarray(0, n), offset);
    offset += n;
  }
}
export function boundedRead(file, budget, maximum = 512 * MiB) {
  return withSealed(file, budget, maximum, (fd, size) => {
    const data = Buffer.alloc(size);
    chunks(fd, size, budget, (bytes, offset) => bytes.copy(data, offset));
    return data;
  });
}
export function boundedHash(file, budget, maximum = 512 * MiB) {
  return withSealed(file, budget, maximum, (fd, size) => {
    const h = crypto.createHash("sha256");
    chunks(fd, size, budget, (bytes) => h.update(bytes));
    return h.digest("hex");
  });
}
export class SdkLedger {
  constructor(budget) {
    this.budget = budget;
    this.passes = [];
    this.api = 0;
    this.read = 0;
    this.hash = 0;
  }
  remaining() {
    return {
      api: 32 - this.api,
      read: 4096 - this.read,
      hash: Math.min(32768, 65536 - 32768 - this.hash),
    };
  }
  add(p) {
    need(this.passes.length < 2, "sdk-pass-count");
    need(
      p?.schema === 3 &&
        p.evidence_kind === "sdk-positive-build-bracket" &&
        p.status === "complete" &&
        p.inventory_complete === false &&
        p.enumerated_entries === 0 &&
        p.open_handles === 0,
      "sdk-unavailable",
    );
    need(
      p.api_calls === 13 &&
        p.read_bytes_charged === 1358 &&
        Number.isSafeInteger(p.hashed_bytes) &&
        p.hashed_bytes > 0 &&
        p.hashed_bytes <= 9478,
      "sdk-counters",
    );
    need(
      p.kits_root === kit &&
        isDeepStrictEqual(p.record, {
          key: sdkKey,
          name: sdkName,
          version: "10.1.26100.9457",
        }),
      "sdk-record",
    );
    need(
      Array.isArray(p.before_write) &&
        p.before_write.length === 2 &&
        p.before_write.every(
          (s) => typeof s === "string" && /^[0-9]{1,20}$/.test(s),
        ) &&
        isDeepStrictEqual(p.before_write, p.after_write) &&
        /^[a-f0-9]{64}$/.test(p.digest ?? ""),
      "sdk-marker",
    );
    need(
      this.api + p.api_calls <= 32 &&
        this.read + p.read_bytes_charged <= 4096 &&
        this.hash + p.hashed_bytes + 32768 <= 65536,
      "sdk-budget",
    );
    if (this.passes.length)
      need(
        p.digest === this.passes[0].digest &&
          isDeepStrictEqual(p.before_write, this.passes[0].before_write),
        "sdk-drift",
      );
    const pass = {
      status: "complete",
      before_write: p.before_write,
      after_write: p.after_write,
      kits_root: kit,
      record: p.record,
      digest: p.digest,
    };
    this.api += p.api_calls;
    this.read += p.read_bytes_charged;
    this.hash += p.hashed_bytes;
    this.passes.push(pass);
  }
  finish(complete = true) {
    need(this.passes.length === 2, "sdk-incomplete");
    return {
      schema: 3,
      evidence_kind: "sdk-positive-build-bracket",
      success: complete,
      positive_record_observed: complete,
      inventory_complete: false,
      passes: this.passes,
      api_calls: this.api,
      read_bytes_charged: this.read,
      hashed_bytes: this.hash,
      enumerated_entries: 0,
      open_handles: 0,
    };
  }
  receipt(n) {
    need(n <= 32768 && this.hash + n <= 65536, "sdk-receipt-limit");
    this.hash += n;
  }
}
const stages = [
  "preflight",
  "sdkBefore",
  "versions",
  "fetch",
  "sources",
  "sealBefore",
  "build",
  "prepareOracle",
  "oracle",
  "inspect",
  "sdkAfter",
  "sealAfter",
];
export async function runStages(io) {
  const r = {
    schema: 3,
    evidence_kind: "progress-sidecar-candidate",
    success: false,
    ...authority(false),
    fetch_invocations: 0,
    build_invocations: 0,
    oracle_invocations: 0,
  };
  try {
    for (const stage of stages) {
      r.stage = stage;
      const counter = {
        fetch: "fetch_invocations",
        build: "build_invocations",
        oracle: "oracle_invocations",
      }[stage];
      if (counter) r[counter]++;
      await io[stage]();
    }
    r.stage = "complete";
    r.success = true;
    Object.assign(r, authority(true));
  } catch (e) {
    r.reason = /^[a-z0-9-]{1,80}$/.test(e.message ?? "")
      ? e.message
      : "stage-unavailable";
  }
  return r;
}
export class CommandRunner {
 constructor(budget,{spawnProcess=spawn,terminate}={}){
  Object.assign(this,{budget,spawnProcess,terminate});this.stopped=false;this.drained=true;this.kills=0;this.commands=[];
 }
 async run(exe,args,{env,cwd,timeout=30000,stdoutLimit=64*MiB,decoder}={}){
  need(!this.stopped,'runner-stopped');this.budget.check();
  const end=Math.min(this.budget.deadline,this.budget.clock()+timeout);
  need(end>this.budget.clock(),'time-limit');this.commands.push({exe,args,cwd});this.drained=false;
  return await new Promise((resolve,reject)=>{
   let c,failed,closed=false,timer,drainTimer,stdoutBytes=0;const out=[],err=[];
   const stop=reason=>{
    if(failed)return;failed=reason;this.stopped=true;decoder?.fail(reason);
    if(this.failureDeadline===undefined)this.failureDeadline=Math.min(this.budget.finalDeadline,this.budget.clock()+120000);
    this.budget.finalDeadline=this.failureDeadline;
    drainTimer=setTimeout(()=>{clearTimeout(timer);reject(Error('process-drain-unavailable'));},Math.max(1,this.failureDeadline-this.budget.clock()));
    if(c&&!closed&&c.exitCode===null&&c.signalCode===null&&c.pid){
     if(this.kills===0){this.kills++;
      if(this.terminate){if(this.terminate(c)===false)this.terminationUnknown=true;}
      else if(process.platform==='win32'){const killed=spawnSync('C:\\Windows\\System32\\taskkill.exe',['/PID',String(c.pid),'/T','/F'],{env:{SystemRoot:'C:\\Windows',PATH:'C:\\Windows\\System32'},stdio:'ignore',timeout:Math.min(10000,Math.max(1,this.failureDeadline-this.budget.clock())),windowsHide:true});if(killed.status!==0||killed.error)this.terminationUnknown=true;}
      else c.kill('SIGKILL');
     }
    }
   };
   try{c=this.spawnProcess(exe,args,{env,cwd,stdio:['ignore','pipe','pipe'],windowsHide:true});}catch{this.stopped=true;this.drained=true;reject(Error('process-unavailable'));return;}
   for(const [stream,parts] of [[c.stdout,out],[c.stderr,err]])stream.on('data',b=>{
    // Every observed byte is charged, including late chunks after a previous failure.
    try{
     if(decoder&&stream===c.stdout)decoder.feed(b);else this.budget.stream(b.length);
     if(this.budget.clock()>=end)throw Error('time-limit');
     if(stream===c.stdout){stdoutBytes+=b.length;need(stdoutBytes<=stdoutLimit,'stdout-limit');}
     if(decoder){need(stream!==c.stderr||b.length===0,'reader-stderr');need(!decoder.failure,decoder.failure||'reader-framing');}
     if(!failed&&!decoder)parts.push(b);
    }catch(e){stop(e.message);}
   });
   c.on('error',()=>stop('process-unavailable'));
   c.on('close',code=>{closed=true;this.drained=!this.terminationUnknown;clearTimeout(timer);clearTimeout(drainTimer);
    if(!failed&&this.budget.clock()>=end){failed='time-limit';this.stopped=true;}
    if(failed)reject(Error(failed));else resolve({code,stdout:Buffer.concat(out).toString('utf8'),stderr:Buffer.concat(err).toString('utf8')});
   });
   timer=setTimeout(()=>stop('time-limit'),Math.max(1,end-this.budget.clock()));
   if(this.budget.clock()>=end)stop('time-limit');
  });
 }
}
export function validateOutput(v,decoder){
 const keys=(x,list)=>need(x&&typeof x==='object'&&!Array.isArray(x)&&isDeepStrictEqual(Object.keys(x).sort(),list.sort()),'output-fields');
 keys(v,['schema','evidence_kind','mode','nonce','policy_sha256','data','metrics']);
 need(v.schema===1&&v.evidence_kind==='progress-reader-output'&&v.mode===decoder.mode&&v.nonce===decoder.nonce&&v.policy_sha256===policySha,'output-binding');
 keys(v.metrics,['strategy','scratch_allocations','scratch_capacity_bytes','maximum_leases','hash_bytes']);
 const m=v.metrics;need(m.strategy==='reuse'&&m.scratch_allocations===1&&m.scratch_capacity_bytes===MiB&&m.maximum_leases===1&&Number.isSafeInteger(m.hash_bytes)&&m.hash_bytes>=0&&m.hash_bytes<=decoder.counters[0],'output-metrics');
 need(v.data?.hash_bytes===decoder.counters[0],'reader-report-drift');
 if(decoder.mode==='preflight'){
  keys(v.data,['success','tools','powershell_version','hash_bytes']);validateTools(v.data);
  need(/^7\./.test(v.data.powershell_version??''),'powershell-version');
 }else{
  keys(v.data,['success','files','directories','file_bytes','file_count','directory_count','hash_bytes','crates','archive_cache_id']);
  validateManifest(v.data,['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source']);
  need(v.data.files.length===decoder.counters[1]&&v.data.directories.length===decoder.counters[2]&&v.data.crates.length===decoder.counters[3],'reader-count-drift');
 }
 return v.data;
}
export function readOutput(file,budget){
 return withSealed(file,budget,64*MiB,(fd,size)=>{
  budget.hash(size);const bytes=Buffer.alloc(size),hash=crypto.createHash('sha256');
  chunks(fd,size,budget,(b,o)=>{b.copy(bytes,o);hash.update(b);});
  return {value:parseStrict(new TextDecoder('utf-8',{fatal:true}).decode(bytes)),bytes:size,sha256:hash.digest('hex')};
 });
}
export function publicationAvailable(budget,result){
 try{budget.emergency();return true;}
 catch{result.success=false;Object.assign(result,authority(false));result.publication='unavailable-reserve-expired';result.cleanup||='unavailable-reserve-expired';return false;}
}
function unavailablePublication(result){
 console.error(JSON.stringify({schema:3,evidence_kind:'progress-sidecar-candidate',success:false,...authority(false),publication:result.publication??'unavailable',cleanup:result.cleanup??'unavailable'}));
 process.exitCode=1;
}
export function publishEvidence(directory,result,files,budget,limit=256*MiB,sdk){
 const data={};try{for(const [name,file]of Object.entries(files)){need(allowed.includes(name),'foreign-evidence');data[name]=boundedRead(file,budget,limit);}}
 catch{result.success=false;result.reason='evidence-limit';for(const n of Object.keys(data))delete data[n];}
 return publishBuffers(directory,result,data,budget,limit,sdk);
}
function publishBuffers(directory,result,data,budget,limit=256*MiB,sdk){
 let resultCharged=false;
 const clock=()=>result.success?budget.check():budget.emergency();
 const publish=buffers=>{
  clock();let total=0;const commitments={};
  for(const [name,bytes]of Object.entries(buffers)){
   need(allowed.includes(name)&&Buffer.isBuffer(bytes),'foreign-evidence');total+=bytes.length;need(total+65536<=limit,'evidence-limit');
  }
  // One atomic precharge precedes every commitment hash and final-result encoding.
  budget.hash(total+65536);resultCharged=true;
  for(const [name,bytes]of Object.entries(buffers)){
   clock();if(name==='sdk-record.json')sdk?.receipt(bytes.length);
   commitments[name]={bytes:bytes.length,sha256:sha(bytes)};
  }
  Object.assign(result,authority(result.success));result.files=commitments;result.stream_bytes=budget.streamed;
  if(sdk)result.sdk_hash_bytes_charged=sdk.hash;
  result.hash_bytes_charged=budget.hashed;const bytes=encodeResult(result);
  need(total+bytes.length<=limit,'evidence-limit');clock();
  for(const [name,value]of Object.entries(buffers)){clock();fs.writeFileSync(path.join(directory,name),value,{flag:'wx'});clock();}
  clock();fs.writeFileSync(path.join(directory,'result.json'),bytes,{flag:'wx'});clock();
 };
 try{
  if(!result.success){for(const n of Object.keys(data))if(n!=='sdk-record.json')delete data[n];budget.emergency();}
  publish(data);
 }catch(error){
  result.success=false;Object.assign(result,authority(false));result.reason=error.message==='time-limit'?'active-publication-expired':'evidence-limit';
  if(!publicationAvailable(budget,result)){unavailablePublication(result);return;}
  for(const n of [...allowed,'result.json']){try{fs.rmSync(path.join(directory,n),{force:true});}catch{}}
  // Full trace and bounded SDK observations survive receipt-only publication.
  const receipt={schema:3,evidence_kind:'progress-sidecar-candidate',success:false,...authority(false),reason:result.reason,run_id:result.run_id,implementation_sha:result.implementation_sha,cleanup:result.cleanup,sdk_observed:result.sdk_observed,sdk_last_report:result.sdk_last_report,sdk_invocations:result.sdk_invocations,sdk_work_unknown:result.sdk_work_unknown===true,reader_traces:result.reader_traces??[],hash_work_unknown:result.hash_work_unknown===true,hash_bytes_charged:budget.hashed,stream_bytes:budget.streamed};
  if(!resultCharged)budget.hash(65536);receipt.hash_bytes_charged=budget.hashed;
  budget.emergency();fs.writeFileSync(path.join(directory,'result.json'),encodeResult(receipt),{flag:'wx'});
  if(!publicationAvailable(budget,result))unavailablePublication(result);
 }
}
export function assertDispatch(e, platform) {
  need(
    platform === "win32" &&
      e.GITHUB_ACTIONS === "true" &&
      e.GITHUB_REPOSITORY === "Rambolarsen/orkworks" &&
      e.GITHUB_EVENT_NAME === "workflow_dispatch" &&
      e.GITHUB_REF === "refs/heads/main" &&
      e.GITHUB_RUN_ATTEMPT === "1" &&
      /^[a-f0-9]{40}$/.test(e.EXPECTED_SHA ?? "") &&
      e.GITHUB_SHA === e.EXPECTED_SHA &&
      e.GITHUB_WORKFLOW_REF ===
        "Rambolarsen/orkworks/.github/workflows/" +
          workflow +
          "@refs/heads/main",
    "dispatch-denied",
  );
}
const relative = (s, empty = false) =>
  typeof s === "string" &&
  s.length <= 1024 &&
  ((empty && s === "") ||
    (s !== "" &&
      !/[\\:\x00-\x1f]/.test(s) &&
      !s.split("/").some((p) => p === "" || p === "." || p === "..")));
const identity = (s) =>
  typeof s === "string" && /^[a-f0-9]{8}:[a-f0-9]{16}$/.test(s);
const checksum = (s) => typeof s === "string" && /^[a-f0-9]{64}$/.test(s);
const exactKeys=(value,names)=>need(value&&typeof value==='object'&&!Array.isArray(value)&&isDeepStrictEqual(Object.keys(value).sort(),names.sort()),'manifest-fields');
export function validateManifest(m, labels) {
  need(
    m?.success === true &&
      Array.isArray(m.files) &&
      Array.isArray(m.directories) &&
      Array.isArray(m.crates),
    "manifest-unavailable",
  );
  for (const [k, cap] of [
    ["file_count", 200000],
    ["directory_count", 50000],
    ["file_bytes", 12 * GiB],
    ["hash_bytes", 32 * GiB],
  ])
    need(
      Number.isSafeInteger(m[k]) && m[k] >= 0 && m[k] <= cap,
      "manifest-limit",
    );
  const seen = new Set(),
    files = [],
    directories = [];
  for (const [rows, out, dir] of [
    [m.files, files, false],
    [m.directories, directories, true],
  ])
    for (const row of rows) {
      exactKeys(row,dir?['root','file','id']:['root','file','id','bytes','sha256']);
      need(
        labels.includes(row.root) &&
          relative(row.file, dir) &&
          identity(row.id),
        "manifest-member",
      );
      const key = (row.root + "/" + row.file).toLowerCase();
      need(!seen.has(key), "manifest-duplicate");
      seen.add(key);
      const r = { root: row.root, file: row.file, id: row.id };
      if (!dir) {
        need(
          Number.isSafeInteger(row.bytes) &&
            row.bytes >= 0 &&
            row.bytes <= 512 * MiB &&
            checksum(row.sha256),
          "manifest-file",
        );
        Object.assign(r, { bytes: row.bytes, sha256: row.sha256 });
      }
      out.push(r);
    }
  const members = new Map(files.map((r) => [r.root + "/" + r.file, r]));
  const packages = new Set(),
    crates = [];
  let archiveBytes = 0;
  for (const c of m.crates) {
    exactKeys(c,['package','directory','archive','files','build_script','proc_macro']);
    exactKeys(c.archive,['file','bytes','sha256','id']);
    need(
      typeof c.package === "string" &&
        !packages.has(c.package) &&
        policy.packages[c.package] &&
        relative(c.directory) &&
        relative(c.archive?.file) &&
        identity(c.archive.id) &&
        c.archive.sha256 === policy.packages[c.package] &&
        Number.isSafeInteger(c.archive.bytes) &&
        c.archive.bytes >= 0 &&
        c.archive.bytes <= 512 * MiB,
      "manifest-crate",
    );
    packages.add(c.package);
    archiveBytes += c.archive.bytes;
    need(
      c.files && typeof c.files === "object" && !Array.isArray(c.files),
      "manifest-crate",
    );
    for (const [file, hash] of Object.entries(c.files)) {
      need(relative(file) && checksum(hash), "manifest-crate-member");
      const actual = members.get("registry/" + c.directory + "/" + file);
      need(actual?.sha256 === hash, "crate-file-drift");
    }
    for (const [field, list] of [
      ["build_script", policy.scripts],
      ["proc_macro", policy.proc_macros],
    ])
      if (c[field] !== null) {
        const pin = list[c.package];
        need(
          pin && pin.file === c[field] && c.files[c[field]] === pin.sha256,
          "unaudited-crate-entrypoint",
        );
      }
    crates.push({
      package: c.package,
      directory: c.directory,
      archive: c.archive,
      files: c.files,
      build_script: c.build_script,
      proc_macro: c.proc_macro,
    });
  }
  need(
    m.file_count === files.length + crates.length &&
      m.directory_count === directories.length + (crates.length ? 2 : 0) &&
      m.file_bytes === files.reduce((n, r) => n + r.bytes, archiveBytes),
    "manifest-counts",
  );
  if (crates.length) need(identity(m.archive_cache_id), "manifest-cache");
  const sort = (a, b) => {
    const x = a.root + "/" + a.file,
      y = b.root + "/" + b.file;
    return x < y ? -1 : x > y ? 1 : 0;
  };
  return {
    files: files.sort(sort),
    directories: directories.sort(sort),
    crates: crates.sort((a, b) => a.package.localeCompare(b.package)),
    archive_cache_id: m.archive_cache_id,
  };
}
export function sameManifest(a, b) {
  need(isDeepStrictEqual(a, b), "input-drift");
}

export function validateTools(p) {
  need(
    p?.success === true &&
      p.tools &&
      isDeepStrictEqual(
        Object.keys(p.tools).sort(),
        Object.keys(policy.tools).sort(),
      ),
    "tool-set",
  );
  for (const [n, h] of Object.entries(policy.tools)) {
    const t = p.tools[n];
    need(
      t.sha256 === h && t.machine === 34404 && identity(t.id),
      "tool-byte-mismatch",
    );
    if (policy.versions[n])
      need(t.version === policy.versions[n], "tool-version-mismatch");
  }
  return p.tools;
}
const ownSources = [
  "scripts/qualification/pair-progress-windows-build.mjs",
  "scripts/qualification/pair-progress-windows-build-protocol.mjs",
  "scripts/qualification/pair-progress-windows-build.ps1",
  "scripts/qualification/pair-progress-windows-build-sdk.py",
  "scripts/qualification/pair-progress-windows-build.inputs.json",
  "scripts/qualification/pair-fixed-windows-build.mjs",
  "scripts/qualification/pair-fixed-windows-build.inputs.json",
  "scripts/qualification/pair-windows-metadata-evidence.mjs",
  "scripts/qualification/pair-windows-sdk-positive-record.py",
  ".github/workflows/" + workflow,
];
function sealControllers(root, budget) {
  let bytes = 0;
  const hashes = {};
  for (const f of ownSources) {
    const n = fs.lstatSync(path.join(root, f)).size;
    bytes += n;
    need(bytes <= MiB, "controller-source-limit");
    hashes[f] = withSealed(path.join(root,f),budget,MiB,(fd,size)=>{
      const stat=fs.fstatSync(fd,{bigint:true}),h=crypto.createHash('sha256');
      chunks(fd,size,budget,b=>h.update(b));
      return {sha256:h.digest('hex'),id:String(stat.dev)+':'+String(stat.ino),bytes:size};
    });
  }
  return hashes;
}
const win = (s) => path.win32.normalize(s).toLowerCase();
function owned(file, root) {
  need(win(file).startsWith(win(root) + path.win32.sep), "foreign-output");
  return file;
}
async function dispatchHistory(e) {
  const response = await fetch(
    "https://api.github.com/repos/Rambolarsen/orkworks/actions/workflows/" +
      workflow +
      "/runs?event=workflow_dispatch&per_page=100",
    {
      headers: {
        Authorization: `Bearer ${e.GITHUB_TOKEN}`,
        Accept: "application/vnd.github+json",
      },
      signal: AbortSignal.timeout(30000),
    },
  );
  need(response.ok, "run-history-unavailable");
  const parts = [];
  let length = 0;
  for await (const b of response.body) {
    length += b.length;
    need(length <= MiB, "run-history-limit");
    parts.push(b);
  }
  assertFirstRun(
    JSON.parse(Buffer.concat(parts).toString("utf8")),
    e.GITHUB_RUN_ID,
    e.EXPECTED_SHA,
  );
}
export function redactDiagnostics(value, roots) {
  const replacements = [];
  for (const [label, root] of Object.entries(roots))
    if (root) {
      for (const variant of new Set([
        root,
        root.replaceAll("\\", "/"),
        root.replaceAll("/", "\\"),
      ])) {
        replacements.push(
          [variant, "[" + label + "]"],
          [JSON.stringify(variant).slice(1, -1), "[" + label + "]"],
        );
      }
    }
  replacements.sort((a, b) => b[0].length - a[0].length);
  for (const [from, to] of replacements) value = value.replaceAll(from, to);
  return value;
}
async function main() {
  const e = process.env;
  assertDispatch(e, process.platform);
  const root = safeName(process.cwd());
  need(
    win(root) === win(fs.realpathSync(e.GITHUB_WORKSPACE)),
    "wrong-checkout",
  );
  need(
    e.SystemRoot?.toLowerCase() === "c:\\windows" &&
      e.ImageOS === "win22" &&
      e.RUNNER_ARCH === "X64" &&
      /^[0-9]+(?:\.[0-9]+){1,3}$/.test(e.ImageVersion ?? ""),
    "image-identity",
  );
  const scratch = path.join(
    safeName(e.RUNNER_TEMP),
    "ork-pair-progress-windows-build-2026-11-27",
  );
  fs.mkdirSync(scratch);
  const working = path.join(scratch, "working"),
    evidence = path.join(scratch, "evidence");
  fs.mkdirSync(working);
  fs.mkdirSync(evidence);
  fs.mkdirSync(path.join(scratch,"requests"));
  fs.mkdirSync(path.join(scratch,"outputs"));
  for (const n of ["cargo-home", "target", "temp"])
    fs.mkdirSync(path.join(working, n));
  const budget = new WorkBudget(),
    runner = new CommandRunner(budget),
    sdk = new SdkLedger(budget);
  const result = {
    schema: 3,
    evidence_kind: "progress-sidecar-candidate",
    success: false,
    ...authority(false),
    proposal_merge: "164309849f9c606e3868c7c31d52fbeb8818bbef",
    implementation_sha: e.EXPECTED_SHA,
    run_id: e.GITHUB_RUN_ID,
    started_utc: new Date().toISOString(),
    image: { os: e.ImageOS, version: e.ImageVersion },
  };
  const files = {},
    save = (name, value) => {
      need(allowed.includes(name), "foreign-evidence");
      const f = path.join(working, name);
      const data = Buffer.isBuffer(value) ? value : Buffer.from(value);
      need(data.length <= 256 * MiB, "evidence-limit");
      fs.writeFileSync(f, data);
      files[name] = f;
      return f;
    };
  const text = (name, value) => save(name, JSON.stringify(value));
  const p = {
    cargo: policy.paths["cargo.exe"],
    rustc: policy.paths["rustc.exe"],
    vc: policy.roots.msvc,
    kit: kit.slice(0, -1),
    sdk: "10.0.26100.0",
  };
  const env = buildEnvironment(
    {
      SystemRoot: "C:\\Windows",
      WINDIR: "C:\\Windows",
      TEMP: path.join(working, "temp"),
      TMP: path.join(working, "temp"),
    },
    p,
    working,
  );
  const runtime = {
    SystemRoot: "C:\\Windows",
    WINDIR: "C:\\Windows",
    TEMP: env.TEMP,
    TMP: env.TMP,
    PATH: "C:\\Windows\\System32",
  };
  const roots = {
    source: root,
    ...policy.roots,
    registry: path.join(env.CARGO_HOME, "registry/src"),
  };
  let manifest,
    controllers,
    sqlite,
    invocation,
    object,
    archive,
    oracleArgs,
    cargoOut = "",
    cargoLog = "",
    diagnostics = "",
    readers = 0,
    sdkCalls = 0;
  const exec = async (exe, args, options = {}) => {
    if (win(exe) === win(p.cargo)) {
      assertConfiguration(root);
      for (const name of ["config", "config.toml"])
        need(
          !fs.existsSync(path.join(env.CARGO_HOME, name)),
          "unknown-cargo-configuration",
        );
    }
    const x = await runner.run(exe, args, { env, cwd: root, ...options });
    diagnostics += x.stderr;
    return x;
  };
  const usedNonces=new Set();result.reader_traces=[];
  const verifySources=()=>need(isDeepStrictEqual(controllers,sealControllers(root,budget)),'controller-source-drift');
  const reader=async mode=>{
    need(++readers<=3,'reader-count');
    const reservation=budget.readerGrant(),grant=reservation.grant;
    const nonce=crypto.randomBytes(16).toString('hex');need(!usedNonces.has(nonce),'nonce-reuse');usedNonces.add(nonce);
    const decoder=new Decoder({nonce,mode,grant,ledger:budget});
    const request=path.join(scratch,'requests',nonce+'.json'),output=path.join(scratch,'outputs',nonce+'.json');
    let sourceValid=false,outputValid=false,x,data,observation={metrics:null,metrics_status:'unavailable'};
    try{
      fs.writeFileSync(request,JSON.stringify([2,'pinned-windows-candidate-v1',mode,nonce,grant,policySha]),{flag:'wx'});
      x=await exec(powershell,['-NoProfile','-NonInteractive','-WorkingDirectory',scratch,'-File',path.join(root,'scripts/qualification/pair-progress-windows-build.ps1'),'-Nonce',nonce],{
        env:{...runtime,ORK_PROGRESS_READER:'1',ORK_PROGRESS_SCRATCH:scratch,ORK_PROGRESS_WORKSPACE:root},cwd:scratch,
        timeout:mode==='preflight'?60000:120000,stdoutLimit:4096,decoder
      });
      decoder.end();budget.check();verifySources();sourceValid=true;
      if(!decoder.failure&&decoder.terminal==='ok'&&x.code===0){
        const held=readOutput(output,budget);data=validateOutput(held.value,decoder);outputValid=true;
        observation={metrics:held.value.metrics,metrics_status:'admitted',output_sha256:held.sha256,output_bytes:held.bytes};
      }
    }finally{
      const disposition=settle(decoder,{code:x?.code,drained:runner.drained,sourceValid,outputValid});
      reservation.finish(disposition.charged);
      result.reader_traces.push({...disposition,mode,nonce,...observation});
      if(disposition.unknown_work)result.hash_work_unknown=true;
      if(!disposition.reader_complete){runner.stopped=true;result.success=false;}
    }
    need(outputValid,'reader-unavailable');return data;
  };
  const sdkPass = async () => {
    need(++sdkCalls <= 2, "sdk-pass-count");
    const limits = sdk.remaining(),
      finish = budget.reserve(limits.hash);
    let completed = false;
    try {
      const x = await exec(
        python,
        [
          "-I",
          "-S",
          "-B",
          path.join(
            root,
            "scripts/qualification/pair-progress-windows-build-sdk.py",
          ),
        ],
        {
          env: {
            ...runtime,
            ORK_PROGRESS_SDK: "1",
            ORK_PROGRESS_SDK_API: String(limits.api),
            ORK_PROGRESS_SDK_READ: String(limits.read),
            ORK_PROGRESS_SDK_HASH: String(limits.hash),
          },
          timeout: 30000,
        },
      );
      need(Buffer.byteLength(x.stdout) <= 8192, "sdk-report-limit");
      const pass = JSON.parse(x.stdout);
      need(
        Number.isSafeInteger(pass.hashed_bytes) &&
          pass.hashed_bytes >= 0 &&
          pass.hashed_bytes <= limits.hash,
        "sdk-work-unknown",
      );
      finish(pass.hashed_bytes);
      completed = true;
      result.sdk_last_report = {
        status: pass.status,
        api_calls: pass.api_calls,
        read_bytes_charged: pass.read_bytes_charged,
        hashed_bytes: pass.hashed_bytes,
        open_handles: pass.open_handles,
      };
      for (const [key, cap] of [
        ["api_calls", limits.api],
        ["read_bytes_charged", limits.read],
        ["open_handles", 3],
      ])
        need(
          Number.isSafeInteger(pass[key]) && pass[key] >= 0 && pass[key] <= cap,
          "sdk-work-unknown",
        );
      result.sdk_observed = {
        api_calls: sdk.api + pass.api_calls,
        read_bytes_charged: sdk.read + pass.read_bytes_charged,
        hashed_bytes: sdk.hash + pass.hashed_bytes,
        open_handles: pass.open_handles,
      };
      need(x.code === 0, "sdk-unavailable");
      need(
        pass.runtime?.version === "3.12.10" &&
          pass.runtime?.implementation === "cpython" &&
          pass.runtime?.bits === 64 &&
          win(pass.runtime?.path ?? "") === win(python),
        "python-runtime",
      );
      sdk.add(pass);
      result.controllers.python.observed = pass.runtime;
      result.sdk_passes = sdk.passes.length;
    } catch (error) {
      if (error.message === "sdk-work-unknown") result.sdk_work_unknown = true;
      throw error;
    } finally {
      if (!completed) {
        finish(limits.hash);
        result.sdk_work_unknown = true;
      }
    }
  };
  try {
    Object.assign(
      result,
      await runStages({
        preflight: async () => {
          controllers = sealControllers(root, budget);
          result.controller_sources = controllers;
          for (const [f, h] of Object.entries(policy.source_pins))
            need(
              boundedHash(path.join(root, f), budget) === h,
              "source-input-drift",
            );
          assertConfiguration(root);
          assertLock(
            boundedRead(
              path.join(root, "crates/orkworksd/Cargo.lock"),
              budget,
              MiB,
            ).toString("utf8"),
          );
          await dispatchHistory(e);
          result.dispatch_history_count = 1;
          const preflight = await reader("preflight");
          result.tools = preflight.tools;
          validateTools(preflight);
          result.controllers = {
            node: {
              path: process.execPath,
              version: process.version,
              sha256: boundedHash(process.execPath, budget),
            },
            powershell: {
              path: powershell,
              version: preflight.powershell_version,
              sha256: boundedHash(powershell, budget),
            },
            python: {
              path: python,
              required_version: "3.12.10",
              architecture: "x64",
              sha256: boundedHash(python, budget),
            },
          };
          need(
            typeof preflight.powershell_version === "string" &&
              /^7\./.test(preflight.powershell_version),
            "powershell-version",
          );
        },
        sdkBefore: sdkPass,
        versions: async () => {
          for (const [exe, args] of [
            [p.cargo, ["-Vv"]],
            [p.rustc, ["-vV"]],
          ]) {
            const x = await exec(exe, args);
            need(
              x.code === 0 &&
                /^release: 1\.100\.0$/m.test(x.stdout) &&
                /^host: x86_64-pc-windows-msvc$/m.test(x.stdout),
              "rust-version-mismatch",
            );
          }
        },
        fetch: async () => {
          const x = await exec(
            p.cargo,
            [
              "fetch",
              "--locked",
              "--target",
              "x86_64-pc-windows-msvc",
              "--manifest-path",
              "crates/orkworksd/Cargo.toml",
              "--config",
              "net.retry=0",
            ],
            { timeout: 240000 },
          );
          need(x.code === 0, "fetch-failed");
        },
        sources: async () => {
          manifest = validateManifest(
            await reader("before"),
            Object.keys(roots),
          );
          const byPackage = new Map(manifest.crates.map((c) => [c.package, c]));
          for (const [key, sub] of [
            ["libgit2-sys@0.18.8+1.9.7", "libgit2/src"],
            ["libssh2-sys@0.3.1", "libssh2/src"],
            ["ring@0.17.14", "pregenerated"],
          ]) {
            const c = byPackage.get(key);
            need(
              c &&
                manifest.directories.some(
                  (d) =>
                    d.root === "registry" && d.file === c.directory + "/" + sub,
                ),
              "missing-packaged-source",
            );
          }
          const c = byPackage.get("libsqlite3-sys@0.30.1");
          need(
            c?.files["sqlite3/sqlite3.c"] ===
              "c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d",
            "sqlite-source-drift",
          );
          sqlite = path.join(roots.registry, c.directory);
          need(
            !manifest.files.some(
              (f) =>
                f.root === "registry" && f.file.includes("/ring-0.17.14/.git/"),
            ),
            "unpackaged-ring",
          );
        },
        sealBefore: async () => {
          text("inputs.json", {
            source_pins: policy.source_pins,
            environment: env,
            tools: result.tools,
            controllers: result.controllers,
            controller_sources: controllers,
            manifest,
            sdk_before: sdk.passes[0],
          });
          need(
            fs.statSync(files["inputs.json"]).size <= 64 * MiB,
            "manifest-limit",
          );
          result.input_manifest_sha256 = boundedHash(
            files["inputs.json"],
            budget,
            64 * MiB,
          );
        },
        build: async () => {
          const x = await exec(
            p.cargo,
            [
              "build",
              "--locked",
              "--offline",
              "--release",
              "--target",
              "x86_64-pc-windows-msvc",
              "--bin",
              "orkworksd",
              "--jobs",
              "2",
              "--manifest-path",
              "crates/orkworksd/Cargo.toml",
              "-vv",
              "--message-format=json",
            ],
            { timeout: 2100000 },
          );
          cargoOut = x.stdout;
          cargoLog = x.stderr;
          save(
            "cargo.jsonl",
            redactDiagnostics(cargoOut, {
              source: root,
              scratch,
              home: e.USERPROFILE,
            }),
          );
          need(x.code === 0, "build-failed");
        },
        prepareOracle: async () => {
          invocation = sqliteInvocation(
            cargoLog + "\n" + cargoOut,
            policy.paths["cl.exe"],
          );
          const source = path.join(sqlite, "sqlite3/sqlite3.c");
          assertSqliteSource(invocation, sqlite, source);
          const fo = invocation.filter((a) => /^[-/]Fo/.test(a));
          need(fo.length === 1, "sqlite-object-unavailable");
          object = owned(
            path.win32.resolve(sqlite, fo[0].slice(3)),
            env.CARGO_TARGET_DIR,
          );
          archive = path.join(path.dirname(object), "sqlite3.lib");
          files["sqlite3.lib"] = archive;
          oracleArgs = preprocessArguments(
            invocation,
            save("oracle.c", oracleSource(source)),
          );
          text("compiler.json", {
            executable: policy.paths["cl.exe"],
            argv: invocation,
            cwd: sqlite,
            environment: env,
            oracle_argv: oracleArgs,
          });
          result.sqlite = {
            source_sha256: boundedHash(source, budget),
            object_sha256: boundedHash(object, budget),
            archive_sha256: boundedHash(archive, budget),
          };
        },
        oracle: async () => {
          const x = await exec(policy.paths["cl.exe"], oracleArgs, {
            cwd: sqlite,
            timeout: 120000,
            stdoutLimit: 32 * MiB,
          });
          need(Buffer.byteLength(x.stdout) <= 32 * MiB, "oracle-output-limit");
          save("oracle.i", x.stdout);
          need(
            x.code === 0 && !/warning D9002/.test(x.stderr),
            "oracle-failed",
          );
        },
        inspect: async () => {
          const candidate = path.join(
              env.CARGO_TARGET_DIR,
              "x86_64-pc-windows-msvc/release/orkworksd.exe",
            ),
            map = path.join(working, "candidate.map");
          files["orkworksd.exe"] = candidate;
          files["candidate.map"] = map;
          const rlib = owned(
            sqliteRlib(cargoOut, env.CARGO_TARGET_DIR),
            env.CARGO_TARGET_DIR,
          );
          files["libsqlite3_sys.rlib"] = rlib;
          const member = path.basename(object),
            custody = {
              object_sha256: boundedHash(object, budget),
              native_archive_sha256: boundedHash(archive, budget),
              rlib_sha256: boundedHash(rlib, budget),
              rlib,
              member,
              extractions: [],
            };
          need(
            custody.object_sha256 === result.sqlite.object_sha256 &&
              custody.native_archive_sha256 === result.sqlite.archive_sha256,
            "sqlite-artifact-drift",
          );
          let listingText = "";
          for (const [label, input] of [
            ["native", archive],
            ["rust", rlib],
          ]) {
            const list = await exec(policy.paths["lib.exe"], ["/LIST", input]);
            need(list.code === 0, "archive-list-unavailable");
            listingText += list.stdout + list.stderr + "\n";
            const selected = archiveMember(list.stdout, member),
              extracted = path.join(working, label + "-sqlite.obj");
            const x = await exec(policy.paths["lib.exe"], [
              input,
              "/EXTRACT:" + selected,
              "/OUT:" + extracted,
            ]);
            need(x.code === 0, "archive-extraction-unavailable");
            const hash = boundedHash(extracted, budget);
            need(hash === custody.object_sha256, "sqlite-member-byte-mismatch");
            custody.extractions.push({
              archive: input,
              member: selected,
              sha256: hash,
            });
            fs.rmSync(extracted);
          }
          save("archive-members.txt", listingText);
          const deps = await exec(policy.paths["dumpbin.exe"], [
            "/DEPENDENTS",
            candidate,
          ]);
          save("dependencies.txt", deps.stdout + deps.stderr);
          need(
            deps.code === 0 &&
              !/\b(?:sqlite3?|sqlcipher)\.dll\b/i.test(
                deps.stdout + deps.stderr,
              ),
            "external-sqlite-import",
          );
          const mapText = boundedRead(map, budget, 64 * MiB).toString("utf8");
          need(
            /^\s*orkworksd(?:\.exe)?\s*$/m.test(mapText.split(/Timestamp/i)[0]),
            "candidate-map-module-mismatch",
          );
          custody.map_rows = assertSqliteMap(mapText, rlib, member);
          text("link-custody.json", custody);
          result.candidate = {
            sha256: boundedHash(candidate, budget),
            bytes: fs.statSync(candidate).size,
          };
          result.linkage = {
            archive_sha256: custody.native_archive_sha256,
            rlib_sha256: custody.rlib_sha256,
            map_sha256: boundedHash(map, budget),
            dependency_report_sha256: boundedHash(
              files["dependencies.txt"],
              budget,
            ),
          };
        },
        sdkAfter: sdkPass,
        sealAfter: async () => {
          sameManifest(
            manifest,
            validateManifest(await reader("after"), Object.keys(roots)),
          );

          budget.check();
        },
      }),
    );
  } catch (error) {
    result.reason = /^[a-z0-9-]{1,80}$/.test(error.message ?? "")
      ? error.message
      : "unavailable";
    result.success = false;
  }
  if(!result.success){runner.stopped=true;budget.finalDeadline=Math.min(budget.finalDeadline,budget.clock()+120000);if(!publicationAvailable(budget,result)){unavailablePublication(result);return;}}
  try{if(runner.drained&&controllers)verifySources();else throw Error('source-final-unavailable');}
  catch{result.success=false;result.reason='source-final-unavailable';}
  if(!result.success&&!publicationAvailable(budget,result)){unavailablePublication(result);return;}
  // Snapshot evidence before removing all owned working inputs. Never clean up a live child tree.
  const data = {};
  try {
    need(result.success,"receipt-only");budget.check();
    save(
      "diagnostics.txt",
      redactDiagnostics(diagnostics, {
        source: root,
        scratch,
        home: e.USERPROFILE,
      }),
    );
    let total = 0;
    for (const [name, file] of Object.entries(files)) {
      if (!fs.existsSync(file)) continue;
      const n = fs.lstatSync(file).size;
      total += n;
      need(total + 65536 + 32768 <= 256 * MiB, "evidence-limit");
      data[name] = boundedRead(file, budget, 256 * MiB);
    }
  } catch {
    result.success = false;
    if(result.success)result.reason="evidence-unavailable";
    result.reason ||= "evidence-unavailable";
    for (const n of Object.keys(data)) delete data[n];
  }
  if (runner.drained) {
    try {
      fs.rmSync(working, { recursive: true });
      for(const f of ['requests','outputs'])fs.rmSync(path.join(scratch,f),{recursive:true});
      result.cleanup = "owned-scratch-removed";
    } catch {
      result.success = false;
      result.cleanup = "unavailable";
    }
  } else {
    result.success = false;
    result.cleanup = "unavailable-live-child";
  }
  Object.assign(result, authority(result.success));
  const sdkRecord =
    sdk.passes.length === 2
      ? sdk.finish(result.success)
      : {
          schema: 3,
          evidence_kind: "sdk-positive-build-bracket",
          success: false,
          positive_record_observed: false,
          inventory_complete: false,
          passes: sdk.passes,
          api_calls:
            result.sdk_observed?.api_calls ?? (sdkCalls === 0 ? 0 : null),
          read_bytes_charged:
            result.sdk_observed?.read_bytes_charged ??
            (sdkCalls === 0 ? 0 : null),
          hashed_bytes:
            result.sdk_observed?.hashed_bytes ?? (sdkCalls === 0 ? 0 : null),
          open_handles:
            result.sdk_observed?.open_handles ?? (sdkCalls === 0 ? 0 : null),
          enumerated_entries: 0,
          work_unknown: result.sdk_work_unknown === true,
        };
  data["sdk-record.json"] = Buffer.from(JSON.stringify(sdkRecord));
  result.reader_debit_basis='payload and metadata passes, expanded-read precharges, conservative row bounds and full encoded-output ceiling';
  result.commands = runner.commands;
  result.reader_invocations = readers;
  result.sdk_invocations = sdkCalls;
  result.forced_terminations = runner.kills;
  result.ended_utc = new Date().toISOString();
  publishBuffers(evidence, result, data, budget, 256 * MiB, sdk);
  if (!result.success) process.exitCode = 1;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
)
  main().catch(error => {
    console.error(error.message==='publication-time-limit'?'Progress candidate publication unavailable: reserve expired; no artifact promised':'Progress candidate unavailable; no retry authorized');
    process.exitCode = 1;
  });
