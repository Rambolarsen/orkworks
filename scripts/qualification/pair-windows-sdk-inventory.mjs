import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { pathToFileURL } from "node:url";
import { assertFirstRun } from "./pair-windows-metadata-evidence.mjs";
import { runBounded } from "./pair-windows-toolchain-inventory.mjs";
const workflow = "pair-windows-sdk-inventory.yml";
const cap = 256 * 1024, maxHash = 8 * 1024 ** 2;
const python = "C:\\hostedtoolcache\\windows\\Python\\3.12.10\\x64\\python.exe";
const roots = ["SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall", "SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall"];
const need = (v, reason) => { if (!v) throw Error(reason); };
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const text = (s, n) => typeof s === "string" && s.length > 0 && s.length <= n && /^[\x20-\x7e]+$/.test(s);
const kitRoot = s => typeof s === "string" && s.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase() === "c:/program files (x86)/windows kits/10";
// Both passes read at most the sealed source length, never a growing whole file.
export function sourceHash(file, expected, charge = () => {}) {
  need(Number.isSafeInteger(expected) && expected > 0 && expected <= 1024 ** 2, "source-limit");
  const named = fs.lstatSync(file);
  need(named.isFile() && !named.isSymbolicLink(), "source-type");
  const fd = fs.openSync(file, fs.constants.O_RDONLY | (fs.constants.O_NOFOLLOW ?? 0));
  try {
    const before = fs.fstatSync(fd);
    need(before.isFile() && before.size === expected, "source-length");
    const data = Buffer.alloc(expected);
    let offset = 0;
    while (offset < expected) {
      const n = fs.readSync(fd, data, offset, expected-offset, offset);
      need(n > 0, "source-short-read"); offset += n;
    }
    const after = fs.fstatSync(fd), current = fs.lstatSync(file);
    need(after.size === expected && current.isFile() && !current.isSymbolicLink() && current.size === expected && before.dev === current.dev && before.ino === current.ino, "source-drift");
    charge(data.length);
    return sha(data);
  } finally { fs.closeSync(fd); }
}
// Project even unavailable observations: rejected/unknown fields are never receipts.
export function retained(data) {
  const out = {schema:1,success:data?.success === true,passes:[]};
  const reasons = new Set(["time-limit","api-limit","read-limit","hash-limit","value-format","platform","registry-open","registry-close","registry-info","registry-name","registry-value","entry-limit","sdk-root","duplicate-name","match-limit","registry-drift","observation-drift","sdk-service","sdk-ambiguous","registry-unavailable","runtime-denied","hash-budget"]);
  if(reasons.has(data?.reason)) out.reason=data.reason;
  for (const [name, maximum] of [["api_calls",10000],["open_handles",4096],["read_bytes_charged",4*1024**2],["hashed_bytes",maxHash]])
    if(Number.isSafeInteger(data?.[name]) && data[name]>=0 && data[name]<=maximum) out[name]=data[name];
  if(data?.runtime?.python === "3.12.10" && data?.runtime?.arch === "X64" && typeof data?.runtime?.executable === "string" && path.win32.normalize(data.runtime.executable).toLowerCase() === python.toLowerCase()) out.runtime={python:"3.12.10",arch:"X64",executable:python};
  for (const pass of Array.isArray(data?.passes) ? data.passes.slice(0,2) : []) {
    const p = {status:["complete","collecting"].includes(pass?.status) ? pass.status : "unavailable",roots:[],products:[]};
    if(kitRoot(pass?.kits_root)) p.kits_root="C:/Program Files (x86)/Windows Kits/10/";
    if(/^[a-f0-9]{64}$/.test(pass?.digest??"")) p.digest=pass.digest;
    for(const r of Array.isArray(pass?.roots) ? pass.roots.slice(0,2) : []) {
      if(!roots.includes(r?.path)) continue;
      const root={path:r.path};
      for(const name of ["count","after_count"]) if(Number.isSafeInteger(r[name]) && r[name]>=0 && r[name]<=0xffffffff) root[name]=r[name];
      for(const name of ["before_write","after_write"]) if(typeof r[name]==="string" && /^[0-9]{1,20}$/.test(r[name])) root[name]=r[name];
      p.roots.push(root);
    }
    for(const x of Array.isArray(pass?.products) ? pass.products.slice(0,32) : [])
      if(text(x?.name,256) && x.name.toLowerCase().includes("windows software development kit") && text(x?.version,128) && typeof x?.key==="string" && roots.some(root=>x.key.startsWith(root+"/") && /^[^/\\\x00]{1,255}$/.test(x.key.slice(root.length+1)))) p.products.push({key:x.key,name:x.name,version:x.version});
    out.passes.push(p);
  }
  return out;
}
export function assertDispatch(e, platform) {
  need(platform === "win32" && e.GITHUB_ACTIONS === "true" && e.GITHUB_REPOSITORY === "Rambolarsen/orkworks" && e.GITHUB_EVENT_NAME === "workflow_dispatch" && e.GITHUB_RUN_ATTEMPT === "1" && /^[a-f0-9]{40}$/.test(e.EXPECTED_SHA ?? "") && e.GITHUB_SHA === e.EXPECTED_SHA && e.GITHUB_WORKFLOW_REF === "Rambolarsen/orkworks/.github/workflows/" + workflow + "@refs/heads/main", "dispatch-denied");
}
export function childEnvironment(parent, hashBudget) {
  const env = {};
  for (const name of ["SystemRoot", "WINDIR", "TEMP", "TMP"]) if (parent[name]) env[name] = parent[name];
  return {...env, PATH:"C:\\Windows\\System32", ORK_PAIR_SDK_CHILD:"1", ORK_PAIR_SDK_HASH_BUDGET:String(hashBudget)};
}
export function problems(data) {
  const errors = [], add = (value, reason) => { if (!value) errors.push(reason); };
  add(data?.schema === 1 && data?.success === true, "collector-unavailable");
  for (const [key, limit] of [["api_calls",10000],["read_bytes_charged",4*1024**2],["hashed_bytes",maxHash]]) add(Number.isSafeInteger(data?.[key]) && data[key] > 0 && data[key] <= limit, key + "-limit");
  add(data?.open_handles === 0, "registry-handles");
  add(data?.runtime?.python === "3.12.10" && data?.runtime?.arch === "X64" && typeof data?.runtime?.executable === "string" && path.win32.normalize(data.runtime.executable).toLowerCase() === python.toLowerCase(), "runtime-identity");
  add(Array.isArray(data?.passes) && data.passes.length === 2, "passes-incomplete");
  if (Array.isArray(data?.passes)) for (const pass of data.passes) {
    add(pass?.status === "complete" && /^[a-f0-9]{64}$/.test(pass?.digest ?? ""), "pass-incomplete");
    add(kitRoot(pass?.kits_root), "sdk-root");
    add(Array.isArray(pass?.roots) && pass.roots.length === 2, "registry-roots");
    if (Array.isArray(pass?.roots)) {
      add(pass.roots.every((r,i)=>r?.path === roots[i] && Number.isSafeInteger(r?.count) && r.count >= 0 && r.count <= 1024 && r.after_count === r.count && typeof r?.before_write === "string" && /^[0-9]{1,20}$/.test(r.before_write) && r.before_write === r.after_write), "registry-observation");
      add(pass.roots.reduce((n,r)=>n+r.count,0) <= 1024, "entry-limit");
    }
    add(Array.isArray(pass?.products) && pass.products.length > 0 && pass.products.length <= 32, "sdk-products");
    if (Array.isArray(pass?.products)) {
      const keys = new Set();
      for (const p of pass.products) {
        add(text(p?.name,256) && p.name.toLowerCase().includes("windows software development kit") && text(p?.version,128), "foreign-product");
        add(typeof p?.key === "string" && roots.some(root=>p.key.startsWith(root+"/") && /^[^/\\\x00]{1,255}$/.test(p.key.slice(root.length+1))), "foreign-key");
        add(!keys.has(p?.key), "duplicate-product"); keys.add(p?.key);
      }
      add(pass.products.some(p=>p?.version === "10.1.26100.9457"), "sdk-service");
    }
  }
  if (data?.passes?.length === 2) add(data.passes[0]?.digest === data.passes[1]?.digest && JSON.stringify(data.passes[0]?.products) === JSON.stringify(data.passes[1]?.products), "observation-drift");
  return [...new Set(errors)];
}
export async function collectOnce(run) {
  try {
    const out = await run();
    // Never export raw stderr, which may include unrelated runtime configuration.
    const diagnostics = out.failure ? "controller-" + out.failure : "";
    if (out.failure) return {success:false,reason:"controller-failed",controller_failure:out.failure,diagnostics};
    const inventory = JSON.parse(out.stdout), errors = problems(inventory);
    const success = out.code === 0 && errors.length === 0;
    const projected=retained(inventory); projected.success=success;
    return {success,controller_exit:out.code,inventory:projected,problems:errors,reason:success ? undefined : "sdk-unavailable",diagnostics};
  } catch { return {success:false,reason:"inventory-unavailable",diagnostics:""}; }
}
export function publish(directory, result, inventory, diagnostics, limit=cap) {
  let files = {"result.json":JSON.stringify(result,null,2),"diagnostics.txt":diagnostics};
  if (inventory !== undefined) files["sdk-inventory.json"] = inventory;
  if (["stdout-limit","stderr-limit"].includes(result.controller_failure) || Object.values(files).reduce((n,s)=>n+Buffer.byteLength(s),0)>limit) {
    result.success = false; result.reason = "evidence-limit";
    files = {"result.json":JSON.stringify({schema:1,success:false,reason:"evidence-limit",run_id:result.run_id,implementation_sha:result.implementation_sha,controller_invocations:result.controller_invocations,inspected_tool_executions:0,fetch_invocations:0,build_invocations:0,target_executions:0,runtime_qualified:false,replacement_pin_accepted:false,cleanup:result.cleanup})};
  }
  for (const [name,value] of Object.entries(files)) fs.writeFileSync(path.join(directory,name),value);
}
async function main() {
  assertDispatch(process.env,process.platform);
  const root = fs.realpathSync(process.cwd());
  need(root === fs.realpathSync(process.env.GITHUB_WORKSPACE), "wrong-checkout");
  const scratch = path.join(process.env.RUNNER_TEMP,"ork-pair-sdk-inventory");
  fs.mkdirSync(scratch);
  const work=path.join(scratch,"working"), evidence=path.join(scratch,"evidence");
  fs.mkdirSync(work); fs.mkdirSync(evidence);
  const result = {schema:1,proposal_merge:"20bac1523d09e4aa89d59a3f703475a36756dccd",implementation_sha:process.env.EXPECTED_SHA,run_id:process.env.GITHUB_RUN_ID,started_utc:new Date().toISOString(),success:false,controller_invocations:0,inspected_tool_executions:0,fetch_invocations:0,build_invocations:0,target_executions:0,runtime_qualified:false,replacement_pin_accepted:false};
  let inventory, diagnostics="", charged=0;
  const charge = n => { need(charged+n<=maxHash,"hash-budget"); charged+=n; };
  try {
    const response = await fetch("https://api.github.com/repos/Rambolarsen/orkworks/actions/workflows/"+workflow+"/runs?event=workflow_dispatch&per_page=100",{headers:{Authorization:"Bearer "+process.env.GITHUB_TOKEN,Accept:"application/vnd.github+json"},signal:AbortSignal.timeout(30000)});
    need(response.ok,"history-unavailable");
    assertFirstRun(await response.json(),process.env.GITHUB_RUN_ID,process.env.EXPECTED_SHA);
    result.dispatch_history_count=1;
    need(process.env.RUNNER_OS==="Windows" && process.env.RUNNER_ARCH==="X64" && /^[a-zA-Z0-9_-]+$/.test(process.env.ImageOS??"") && /^[0-9]+(?:\.[0-9]+){1,3}$/.test(process.env.ImageVersion??""),"runner-identity");
    result.runner={os:process.env.RUNNER_OS,arch:process.env.RUNNER_ARCH,image_os:process.env.ImageOS,image_version:process.env.ImageVersion};
    const files=["pair-windows-sdk-inventory.mjs","pair-windows-sdk-inventory.py","pair-windows-metadata-evidence.mjs","pair-windows-toolchain-inventory.mjs"];
    const sources=files.map(name=>({file:"scripts/qualification/"+name}));
    let size=0;
    for (const source of sources) {
      const stat=fs.lstatSync(path.join(root,source.file));
      need(stat.isFile() && !stat.isSymbolicLink(),"controller-source"); size+=stat.size; source.bytes=stat.size;
    }
    need(size<=1024**2,"source-limit");
    for (const source of sources) source.sha256=sourceHash(path.join(root,source.file),source.bytes,charge);
    result.controllers=sources;
    result.controller_program=python;
    result.controller_invocations=1;
    const out=await collectOnce(()=>runBounded(python,["-I","-S","-B",path.join(root,"scripts/qualification/pair-windows-sdk-inventory.py")],{env:childEnvironment(process.env,maxHash-2*size-cap),cwd:work,timeout:60000,stdoutLimit:192*1024,stderrLimit:8192}));
    diagnostics=out.diagnostics; delete out.diagnostics;
    if(out.inventory) inventory=JSON.stringify(out.inventory);
    delete out.inventory; Object.assign(result,out);
    if(inventory!==undefined) {
      const bytes=Buffer.byteLength(inventory);
      const hashed=JSON.parse(inventory).hashed_bytes;
      need(Number.isSafeInteger(hashed) && hashed>=0 && hashed+2*size+bytes<=maxHash,"hash-budget");
      charge(hashed); charge(bytes);
      result.inventory_bytes=bytes; result.inventory_sha256=sha(inventory);
    }
    for(const source of sources) need(sourceHash(path.join(root,source.file),source.bytes,charge)===source.sha256,"controller-drift");
  } catch(e) { result.success=false;result.reason=String(e.message).replace(/[^a-z-]/g,"").slice(0,80)||"unavailable"; }
  finally {
    try { fs.rmSync(work,{recursive:true,force:true}); result.cleanup="owned-working-scratch-removed"; }
    catch { result.success=false;result.cleanup="unavailable"; }
    result.total_hashed_bytes=charged;
    result.ended_utc=new Date().toISOString();
    publish(evidence,result,inventory,diagnostics);
  }
  if(!result.success) process.exitCode=1;
}
if(process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) main().catch(()=>{console.error("SDK inventory unavailable; no retry authorized");process.exitCode=1;});
