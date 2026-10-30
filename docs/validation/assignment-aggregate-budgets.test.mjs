// Documentation arithmetic only: catches off-by-one, pooling, skipped validation and overflow.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {comparePublicationTotals} from './assignment-aggregate-budgets.mjs';
const catalog=JSON.parse(readFileSync(new URL('./assignment-aggregate-budgets.json',import.meta.url)));
const zero=()=>({records:{assignment:'0',run:'0',workspace:'0'},payloads:{assignment:'0',run:'0',workspace:'0'}});
for(const vector of catalog.vectors)test(vector.name,()=>{
  const input=structuredClone(vector);
  assert.deepEqual(comparePublicationTotals(input.usage,input.delta),vector.expected);
  assert.deepEqual(input,vector,'comparison must not mutate captured totals');
});
for(const bad of ['01','-1','+1','1.0',' 1','1e3','18446744073709551616',1,null])test(`reject malformed counter ${JSON.stringify(bad)}`,()=>{
  const usage=zero();usage.records.run=bad;
  assert.throws(()=>comparePublicationTotals(usage,zero()),/counter/);
});
for(const where of ['usage','delta'])for(const change of ['missing-resource','extra-resource','missing-scope','extra-scope'])test(`${where} rejects ${change}`,()=>{
  const usage=zero(),delta=zero(),value=where==='usage'?usage:delta;
  if(change==='missing-resource')delete value.records;
  if(change==='extra-resource')value.receipts={assignment:'0',run:'0',workspace:'0'};
  if(change==='missing-scope')delete value.records.run;
  if(change==='extra-scope')value.records.epoch='0';
  assert.throws(()=>comparePublicationTotals(usage,delta));
});
test('checked addition rejects u64 overflow before a capacity result',()=>{
  const usage=zero(),delta=zero();usage.records.assignment='18446744073709551615';delta.records.assignment='1';
  assert.throws(()=>comparePublicationTotals(usage,delta),/overflow/);
});
test('later malformed payload cannot be hidden by earlier exhausted records',()=>{
  const usage=zero();usage.records.assignment='67108865';usage.payloads.workspace='01';
  assert.throws(()=>comparePublicationTotals(usage,zero()),/counter/);
});
test('maximum u64 is valid counter TEXT but exceeds policy',()=>{
  const usage=zero();usage.payloads.workspace='18446744073709551615';
  assert.deepEqual(comparePublicationTotals(usage,zero()),{fits:false,exceeded:['payloads.workspace'],reason:'result-payload-capacity-exhausted'});
});
