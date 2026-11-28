import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {isDeepStrictEqual} from 'node:util';
import {boundedHash,readOutput,validateManifest,validateTools,CommandRunner} from './pair-progress-windows-build.mjs';
import {settle,encodeResult} from './pair-root-diagnostic-windows-build-protocol.mjs';
export {CommandRunner,readOutput};
const MiB=1024**2;
const need=(value,reason)=>{if(!value)throw Error(reason);};
const policySha='1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6';
export const sourceFiles=Object.freeze([
 'scripts/qualification/pair-root-diagnostic-windows-build.mjs',
 'scripts/qualification/pair-root-diagnostic-windows-build-protocol.mjs',
 'scripts/qualification/pair-root-diagnostic-windows-build.ps1',
 'scripts/qualification/pair-root-diagnostic-windows-build-sdk.py',
 'scripts/qualification/pair-root-diagnostic-windows-build.inputs.json',
 'scripts/qualification/pair-progress-windows-build.mjs',
 'scripts/qualification/pair-progress-windows-build-protocol.mjs',
 'scripts/qualification/pair-progress-windows-build.inputs.json',
 'scripts/qualification/pair-progress-windows-build-sdk.py',
 'scripts/qualification/pair-fixed-windows-build.mjs',
 'scripts/qualification/pair-fixed-windows-build.inputs.json',
 'scripts/qualification/pair-windows-metadata-evidence.mjs',
 'scripts/qualification/pair-windows-sdk-positive-record.py',
]);
export function sealSources(root,budget) {
 let bytes=0;const seal={};
 for(const name of sourceFiles){
  const file=path.join(root,name),before=fs.lstatSync(file,{bigint:true});
  bytes+=Number(before.size);need(bytes<=MiB,'controller-source-limit');
  const digest=boundedHash(file,budget,MiB),after=fs.lstatSync(file,{bigint:true});
  need(before.dev===after.dev&&before.ino===after.ino&&before.size===after.size&&before.mtimeNs===after.mtimeNs&&before.ctimeNs===after.ctimeNs,'source-drift');
  if(name.endsWith('pair-root-diagnostic-windows-build.inputs.json'))need(Number(before.size)===43200&&digest===policySha,'policy-binding');
  seal[name]=Object.freeze({id:String(before.dev)+':'+String(before.ino),bytes:Number(before.size),sha256:digest});
 }
 return Object.freeze(seal);
}
function keys(value,expected){need(value&&typeof value==='object'&&!Array.isArray(value)&&isDeepStrictEqual(Object.keys(value).sort(),[...expected].sort()),'output-fields');}
export function validateOutput(value,decoder) {
 keys(value,['schema','evidence_kind','mode','nonce','policy_sha256','data','metrics']);
 need(value.schema===2&&value.evidence_kind==='root-diagnostic-reader-output'&&value.mode===decoder.mode&&value.nonce===decoder.nonce&&value.policy_sha256===policySha,'output-binding');
 keys(value.metrics,['strategy','scratch_allocations','scratch_capacity_bytes','maximum_leases','hash_bytes']);
 const metrics=value.metrics;need(metrics.strategy==='reuse'&&metrics.scratch_allocations===1&&metrics.scratch_capacity_bytes===MiB&&metrics.maximum_leases===1&&Number.isSafeInteger(metrics.hash_bytes)&&metrics.hash_bytes>=0&&metrics.hash_bytes<=decoder.counters[0],'output-metrics');
 need(value.data?.hash_bytes===decoder.counters[0],'reader-report-drift');
 if(decoder.mode==='preflight'){
  keys(value.data,['success','tools','powershell_version','hash_bytes']);validateTools(value.data);need(/^7\./.test(value.data.powershell_version??''),'powershell-version');
 }else{
  keys(value.data,['success','files','directories','file_bytes','file_count','directory_count','hash_bytes','crates','archive_cache_id']);
  validateManifest(value.data,['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source']);
  need(value.data.files.length===decoder.counters[1]&&value.data.directories.length===decoder.counters[2]&&value.data.crates.length===decoder.counters[3],'reader-count-drift');
 }
 decoder.validateSamples(value.data);return value.data;
}
export function admitReader({decoder,close,reservation,budget,sourceRoot,seal,output}) {
 let sourceValid=false,outputValid=false,data;
 const observation={mode:decoder.mode,nonce:decoder.nonce,metrics:null,metrics_status:'unavailable'};
 decoder.end();
 try {
  try{budget.check();}catch{decoder.fail('time-limit');budget.emergency();}
  sourceValid=isDeepStrictEqual(sealSources(sourceRoot,budget),seal);
  if(sourceValid&&!decoder.failure&&decoder.terminal==='ok'&&close.code===0&&close.drained){
   budget.check();const held=readOutput(output,budget);data=validateOutput(held.value,decoder);budget.check();outputValid=true;
   Object.assign(observation,{metrics:held.value.metrics,metrics_status:'admitted',output_sha256:held.sha256,output_bytes:held.bytes});
  }
 }catch{data=undefined;outputValid=false;}
 const disposition=Object.freeze({...settle(decoder,{...close,sourceValid,outputValid}),...observation});
 reservation.finish(disposition.charged);
 return Object.freeze({disposition,data:outputValid?data:undefined});
}
export function receipt(traces) {
 return encodeResult({schema:4,evidence_kind:'root-diagnostic-sidecar-candidate',success:false,candidate_evidence_complete:false,runtime_qualified:false,production_pin_accepted:false,target_executions:0,reader_traces:traces});
}
export function main(){throw Error('live-entry-disabled');}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 try{main();}catch{console.error(JSON.stringify({reason:'live-entry-disabled',runtime_qualified:false,production_pin_accepted:false,target_executions:0}));process.exitCode=1;}
}
