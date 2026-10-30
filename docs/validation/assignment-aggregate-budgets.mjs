// Offline arithmetic only. Never trust these inputs as store-derived usage.
// No authority, ownership, inventory, receipt, source or transaction validation.
import assert from 'node:assert/strict';
const scopes=['assignment','run','workspace'];
const caps=[67108864n,134217728n,268435456n];
const resources=['records','payloads'];
const max=18446744073709551615n;
function counter(value){
  assert(typeof value==='string' && /^(0|[1-9][0-9]{0,19})$/.test(value),'canonical counter TEXT');
  const n=BigInt(value);assert(n<=max,'counter u64 range');return n;
}
function fields(value,keys){
  assert(value!==null && typeof value==='object' && !Array.isArray(value),'totals record');
  assert.deepEqual(Object.keys(value).sort(),[...keys].sort(),'closed totals fields');
}
export function comparePublicationTotals(usage,delta){
  fields(usage,resources);fields(delta,resources);
  const exceeded=[];
  // Validate every field before returning a capacity classification.
  for(const resource of resources){
    fields(usage[resource],scopes);fields(delta[resource],scopes);
    for(const [i,scope] of scopes.entries()){
      const total=counter(usage[resource][scope])+counter(delta[resource][scope]);
      assert(total<=max,'counter addition overflow');
      if(total>caps[i])exceeded.push(`${resource}.${scope}`);
    }
  }
  return {fits:exceeded.length===0,exceeded,reason:exceeded.length===0?null:
    exceeded[0].startsWith('records.')?'assignment-record-capacity-exhausted':'result-payload-capacity-exhausted'};
}
