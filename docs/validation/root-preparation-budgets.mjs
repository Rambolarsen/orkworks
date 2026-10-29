// Documentation-only initial-profile budget inspection. Never import into runtime.
// All source/context/native authority remains unverified; no mutation is performed.
import assert from 'node:assert/strict';
import {validateEffectsState} from './root-preparation-effects.mjs';
const LIMITS={runPreparationBytes:8388608n,workspacePreparationBytes:134217728n,workspaceRunCount:32n,
  proofBytes:67108864n,proofCount:4096n,logicalBytes:536870912n,logicalRows:8192n};
const MAX=9223372036854775807n;
const counter=x=>{assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,18})$/.test(x),'canonical budget Counter');const n=BigInt(x);assert(n<=MAX,'budget Counter range');return n;};
const add=(a,b)=>{const n=a+b;assert(n<=MAX,'checked budget sum');return n;};
function canonical(x){
  if(Array.isArray(x))return '['+x.map(canonical).join(',')+']';
  if(x&&typeof x==='object')return '{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}';
  return JSON.stringify(x);
}
const bytes=x=>BigInt(Buffer.byteLength(canonical(x)));
const PREP=['root_drafts','root_preparations','run_states','parent_attachments','preparation_artifacts'];
export function compareBudgetTotals(usage){
  assert(usage&&typeof usage==='object'&&!Array.isArray(usage),'budget usage record');
  assert.deepEqual(Object.keys(usage).sort(),Object.keys(LIMITS).sort(),'closed budget totals');
  const exceeded=Object.keys(LIMITS).filter(k=>counter(usage[k])>LIMITS[k]);
  return {classification:'budget-arithmetic-only',fits:exceeded.length===0,exceeded};
}
export function measureBudgetState(state,contexts,fixtureCaps){
  // Validate complete original-source, row/proof and shared-ledger membership first.
  // This inherited checker is intentionally restricted to the no-pruning fixture.
  validateEffectsState(state,contexts,fixtureCaps);
  let logicalBytes=0n,proofBytes=0n,proofCount=0n,workspacePreparationBytes=0n;
  const runs=new Map();
  for(const encoded of state.rows){
    const key=JSON.parse(encoded.keyCanonical),value=JSON.parse(encoded.valueCanonical);
    const full=add(BigInt(Buffer.byteLength(encoded.keyCanonical)),BigInt(Buffer.byteLength(encoded.valueCanonical)));
    logicalBytes=add(logicalBytes,full);
    if(encoded.table==='commit_proof'){proofBytes=add(proofBytes,full);proofCount=add(proofCount,1n);}
    if(!PREP.includes(encoded.table))continue;
    const source=encoded.table==='root_drafts'?value.draft:encoded.table==='root_preparations'?value.bootstrap:
      encoded.table==='run_states'?value.state:encoded.table==='parent_attachments'?value.attachment:key;
    const owner={workspaceId:source.workspaceId,runId:source.runId};
    const identity=canonical(owner),entry=runs.get(identity)??{...owner,bytes:0n};
    delete value.lastCommit;
    if(encoded.table==='root_preparations')delete value.receipt.commit;
    const charge=add(bytes(key),bytes(value));entry.bytes=add(entry.bytes,charge);runs.set(identity,entry);
    workspacePreparationBytes=add(workspacePreparationBytes,charge);
  }
  const runList=[...runs.values()].sort((a,b)=>Buffer.compare(Buffer.from(canonical({workspaceId:a.workspaceId,runId:a.runId})),Buffer.from(canonical({workspaceId:b.workspaceId,runId:b.runId}))));
  const usage={runPreparationBytes:String(runList.reduce((max,r)=>r.bytes>max?r.bytes:max,0n)),workspacePreparationBytes:String(workspacePreparationBytes),
    workspaceRunCount:String(runs.size),proofBytes:String(proofBytes),proofCount:String(proofCount),logicalBytes:String(logicalBytes),logicalRows:String(state.rows.length)};
  return {classification:'synthetic-logical-budget-inspection-only',usage,runs:runList.map(r=>({...r,bytes:String(r.bytes)})),comparison:compareBudgetTotals(usage)};
}
