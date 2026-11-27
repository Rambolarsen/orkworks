import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import {spawn,spawnSync} from "node:child_process";
import {pathToFileURL} from "node:url";
import {isDeepStrictEqual} from "node:util";
import {assertFirstRun,assertConfiguration,assertLock} from "./pair-windows-metadata-evidence.mjs";
import {buildEnvironment,sqliteInvocation,assertSqliteSource,preprocessArguments,sqliteRlib,archiveMember,assertSqliteMap,oracleSource,packageBuildScript} from "./pair-fixed-windows-build.mjs";
const need=(v,r)=>{if(!v)throw Error(r);};
const sha=b=>crypto.createHash("sha256").update(b).digest("hex");
const MiB=1024**2,GiB=1024**3;
const policy=JSON.parse(fs.readFileSync(new URL("./pair-replacement-windows-build.inputs.json",import.meta.url)));
const workflow="pair-replacement-windows-build.yml";
const python="C:\\hostedtoolcache\\windows\\Python\\3.12.10\\x64\\python.exe";
const powershell="C:\\Program Files\\PowerShell\\7\\pwsh.exe";
const sdkKey="SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall/{45e335f3-c807-43b8-9ed1-43666cfcf939}";
const sdkName="Windows Software Development Kit - Windows 10.0.26100.9457";
const kit="C:/Program Files (x86)/Windows Kits/10/";
const authority=complete=>({candidate_evidence_complete:complete===true,runtime_qualified:false,production_pin_accepted:false,target_executions:0});
const allowed=["inputs.json","cargo.jsonl","diagnostics.txt","compiler.json","oracle.c","oracle.i","sqlite3.lib","libsqlite3_sys.rlib","orkworksd.exe","candidate.map","dependencies.txt","archive-members.txt","link-custody.json","sdk-record.json"];
export class WorkBudget {
  constructor({hashLimit=32*GiB,streamLimit=64*MiB,seconds=2400}={}){need(hashLimit>=0&&hashLimit<=32*GiB&&streamLimit>=0&&streamLimit<=64*MiB,"budget-range");this.hashLimit=hashLimit;this.streamLimit=streamLimit;this.hashed=0;this.reserved=0;this.streamed=0;this.deadline=Date.now()+seconds*1000;}
  check(){need(Date.now()<this.deadline,"time-limit");}
  hash(n){need(Number.isSafeInteger(n)&&n>=0&&this.hashed+this.reserved+n<=this.hashLimit,"hash-limit");this.hashed+=n;}
  reserve(n){this.hash(n);this.hashed-=n;this.reserved+=n;let done=false;return actual=>{need(!done&&Number.isSafeInteger(actual)&&actual>=0&&actual<=n,"reservation-invalid");done=true;this.reserved-=n;this.hashed+=actual;};}
  stream(n){need(this.streamed+n<=this.streamLimit,"stream-limit");this.streamed+=n;}
  remaining(){return this.hashLimit-this.hashed-this.reserved;}
}
function safeName(file){const full=path.resolve(file);for(let p=full;;p=path.dirname(p)){const s=fs.lstatSync(p);need(!s.isSymbolicLink(),"linked-input");if(path.dirname(p)===p)break;}need(path.resolve(fs.realpathSync(full)).toLowerCase()===full.toLowerCase(),"foreign-input");return full;}
function withSealed(file,budget,maximum,reader){
  budget.check();const full=safeName(file),named=fs.lstatSync(full,{bigint:true});need(named.isFile()&&named.size<=BigInt(maximum),"file-limit");
  const expected=Number(named.size);budget.hash(expected);const fd=fs.openSync(full,fs.constants.O_RDONLY|(fs.constants.O_NOFOLLOW??0));
  try{const before=fs.fstatSync(fd,{bigint:true});need(before.isFile()&&before.size===named.size&&before.ino===named.ino&&before.dev===named.dev,"file-drift");const result=reader(fd,expected);const after=fs.fstatSync(fd,{bigint:true}),current=fs.lstatSync(full,{bigint:true});need(!current.isSymbolicLink()&&current.isFile()&&after.size===before.size&&current.size===before.size&&current.ino===before.ino&&current.dev===before.dev&&after.mtimeNs===before.mtimeNs&&after.ctimeNs===before.ctimeNs,"file-drift");safeName(full);return result;}finally{fs.closeSync(fd);}
}
function chunks(fd,size,budget,consume){const buffer=Buffer.alloc(Math.min(MiB,size));let offset=0;while(offset<size){budget.check();const n=fs.readSync(fd,buffer,0,Math.min(buffer.length,size-offset),offset);need(n>0,"file-short-read");consume(buffer.subarray(0,n),offset);offset+=n;}}
export function boundedRead(file,budget,maximum=512*MiB){return withSealed(file,budget,maximum,(fd,size)=>{const data=Buffer.alloc(size);chunks(fd,size,budget,(bytes,offset)=>bytes.copy(data,offset));return data;});}
export function boundedHash(file,budget,maximum=512*MiB){return withSealed(file,budget,maximum,(fd,size)=>{const h=crypto.createHash("sha256");chunks(fd,size,budget,bytes=>h.update(bytes));return h.digest("hex");});}
export class SdkLedger {
  constructor(budget){this.budget=budget;this.passes=[];this.api=0;this.read=0;this.hash=0;}
  remaining(){return{api:32-this.api,read:4096-this.read,hash:Math.min(32768,65536-32768-this.hash)};}
  add(p){need(this.passes.length<2,"sdk-pass-count");need(p?.schema===3&&p.evidence_kind==="sdk-positive-build-bracket"&&p.status==="complete"&&p.inventory_complete===false&&p.enumerated_entries===0&&p.open_handles===0,"sdk-unavailable");need(p.api_calls===13&&p.read_bytes_charged===1358&&Number.isSafeInteger(p.hashed_bytes)&&p.hashed_bytes>0&&p.hashed_bytes<=9478,"sdk-counters");need(p.kits_root===kit&&isDeepStrictEqual(p.record,{key:sdkKey,name:sdkName,version:"10.1.26100.9457"}),"sdk-record");need(Array.isArray(p.before_write)&&p.before_write.length===2&&p.before_write.every(s=>typeof s==="string"&&/^[0-9]{1,20}$/.test(s))&&isDeepStrictEqual(p.before_write,p.after_write)&&/^[a-f0-9]{64}$/.test(p.digest??""),"sdk-marker");need(this.api+p.api_calls<=32&&this.read+p.read_bytes_charged<=4096&&this.hash+p.hashed_bytes+32768<=65536,"sdk-budget");if(this.passes.length)need(p.digest===this.passes[0].digest&&isDeepStrictEqual(p.before_write,this.passes[0].before_write),"sdk-drift");const pass={status:"complete",before_write:p.before_write,after_write:p.after_write,kits_root:kit,record:p.record,digest:p.digest};this.api+=p.api_calls;this.read+=p.read_bytes_charged;this.hash+=p.hashed_bytes;this.passes.push(pass);}
  finish(complete=true){need(this.passes.length===2,"sdk-incomplete");return{schema:3,evidence_kind:"sdk-positive-build-bracket",success:complete,positive_record_observed:complete,inventory_complete:false,passes:this.passes,api_calls:this.api,read_bytes_charged:this.read,hashed_bytes:this.hash,enumerated_entries:0,open_handles:0};}
  receipt(n){need(n<=32768&&this.hash+n<=65536,"sdk-receipt-limit");this.hash+=n;}
}
const stages=["preflight","sdkBefore","versions","fetch","sources","sealBefore","build","prepareOracle","oracle","inspect","sdkAfter","sealAfter"];
export async function runStages(io){const r={schema:2,evidence_kind:"replacement-sidecar-candidate",success:false,...authority(false),fetch_invocations:0,build_invocations:0,oracle_invocations:0};try{for(const stage of stages){r.stage=stage;const counter={fetch:"fetch_invocations",build:"build_invocations",oracle:"oracle_invocations"}[stage];if(counter)r[counter]++;await io[stage]();}r.stage="complete";r.success=true;Object.assign(r,authority(true));}catch(e){r.reason=/^[a-z0-9-]{1,80}$/.test(e.message??"")?e.message:"stage-unavailable";}return r;}
export class CommandRunner {
  constructor(budget){this.budget=budget;this.stopped=false;this.drained=true;this.kills=0;this.commands=[];}
  async run(exe,args,{env,cwd,timeout=30000}={}){
    need(!this.stopped,"runner-stopped");this.budget.check();const duration=Math.min(timeout,this.budget.deadline-Date.now());need(duration>0,"time-limit");this.commands.push({exe,args,cwd});this.drained=false;
    return await new Promise((resolve,reject)=>{const c=spawn(exe,args,{env,cwd,stdio:["ignore","pipe","pipe"],windowsHide:true});let failure,closed=false;const out=[],err=[];
      const stop=reason=>{if(failure)return;failure=reason;this.stopped=true;if(!closed&&c.exitCode===null&&c.signalCode===null){if(process.platform==="win32"&&c.pid&&this.kills===0){this.kills++;spawnSync("C:\\Windows\\System32\\taskkill.exe",["/PID",String(c.pid),"/T","/F"],{env:{SystemRoot:"C:\\Windows",PATH:"C:\\Windows\\System32"},stdio:"ignore",timeout:10000,windowsHide:true});}c.kill("SIGKILL");}};
      const timer=setTimeout(()=>stop("time-limit"),duration);
      for(const[stream,parts]of[[c.stdout,out],[c.stderr,err]])stream.on("data",b=>{if(failure)return;try{this.budget.stream(b.length);parts.push(b);}catch{stop("stream-limit");}});
      c.on("error",()=>stop("process-unavailable"));c.on("close",code=>{closed=true;this.drained=true;clearTimeout(timer);if(failure)reject(Error(failure));else resolve({code,stdout:Buffer.concat(out).toString("utf8"),stderr:Buffer.concat(err).toString("utf8")});});
    });
  }
}
export function publishEvidence(directory,result,files,budget,limit=256*MiB,sdk){
  // Snapshot bounded owned files before deleting working paths. Buffers are immutable copies.
  const data={};let total=0;try{for(const[name,file]of Object.entries(files)){need(allowed.includes(name),"foreign-evidence");const stat=fs.lstatSync(file);need(stat.isFile()&&!stat.isSymbolicLink()&&total+stat.size+65536<=limit,"evidence-limit");data[name]=boundedRead(file,budget,limit);total+=data[name].length;}return publishBuffers(directory,result,data,budget,limit,sdk);}catch{result.success=false;Object.assign(result,authority(false));result.reason="evidence-limit";fs.writeFileSync(path.join(directory,"result.json"),JSON.stringify({schema:2,success:false,...authority(false),reason:"evidence-limit"}));}
}
function publishBuffers(directory,result,data,budget,limit=256*MiB,sdk){
  Object.assign(result,authority(result.success));let files={};try{let total=0;for(const[name,bytes]of Object.entries(data)){need(allowed.includes(name),"foreign-evidence");total+=bytes.length;need(total+65536<=limit,"evidence-limit");if(name==="sdk-record.json")sdk?.receipt(bytes.length);budget.hash(bytes.length);files[name]={bytes:bytes.length,sha256:sha(bytes)};}result.files=files;result.hash_bytes_charged=budget.hashed;result.stream_bytes=budget.streamed;const text=JSON.stringify(result,null,2);need(Buffer.byteLength(text)<=65536&&total+Buffer.byteLength(text)<=limit,"evidence-limit");for(const[name,bytes]of Object.entries(data))fs.writeFileSync(path.join(directory,name),bytes);fs.writeFileSync(path.join(directory,"result.json"),text);}catch{result.success=false;Object.assign(result,authority(false));result.reason="evidence-limit";fs.writeFileSync(path.join(directory,"result.json"),JSON.stringify({schema:2,success:false,...authority(false),reason:"evidence-limit",run_id:result.run_id,implementation_sha:result.implementation_sha,cleanup:result.cleanup}));}
}
export function assertDispatch(e,platform){need(platform==="win32"&&e.GITHUB_ACTIONS==="true"&&e.GITHUB_REPOSITORY==="Rambolarsen/orkworks"&&e.GITHUB_EVENT_NAME==="workflow_dispatch"&&e.GITHUB_RUN_ATTEMPT==="1"&&/^[a-f0-9]{40}$/.test(e.EXPECTED_SHA??"")&&e.GITHUB_SHA===e.EXPECTED_SHA&&e.GITHUB_WORKFLOW_REF==="Rambolarsen/orkworks/.github/workflows/"+workflow+"@refs/heads/main","dispatch-denied");}
// Live entry point is added only after the bounded external-reader fixtures pass.
