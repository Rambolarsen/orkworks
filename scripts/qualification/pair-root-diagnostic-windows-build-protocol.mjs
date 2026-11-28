// Isolated v3 wire. A checkpoint never admits an input or refunds unknown work.
export {WorkBudget, parseStrict} from './pair-progress-windows-build-protocol.mjs';
const need = (value, reason) => { if (!value) throw Error(reason); };
export const reasons = Object.freeze(['bad-request','input-denied','hash-limit','input-limit','identity-drift','format-denied','output-limit','reader-unavailable']);
const failures = new Set(['stream-limit','report-limit','after-terminal','encoding','line-limit','time-limit','framing','binding','record-limit','counter','phase-order','sample-order','sample-counter','missing-sample','trace-limit','terminal','partial-eof','missing-terminal','process-unavailable','process-drain-unavailable','reader-stderr','stdout-limit','source-invalid','output-invalid']);
const phases = Object.freeze([[0,7],[1,7],[2,7],[3,7],...[0,1,2,3,4,5,6].flatMap(root=>[[4,root],[5,root]]),[6,7],[7,7],[8,7]].map(Object.freeze));
const minimumSamples = (files, dirs) => files >= 65536 ? 11 : files >= 4096 ? 10 : files >= 256 ? 9 : files >= 16 ? 8 : files ? 7 : dirs > 1 ? 3 : dirs ? 2 : 0;

export class Decoder {
 constructor({nonce,mode,grant,ledger}) {
  need(/^[a-f0-9]{32}$/.test(nonce)&&['preflight','before','after'].includes(mode)&&Number.isSafeInteger(grant)&&grant>=0&&grant<=34359738368,'bad-request');
  Object.assign(this,{nonce,mode,grant,ledger});
  this.expected=mode==='preflight'?[[0,7],[1,7],[6,7],[7,7],[8,7]]:phases;
  this.position=0;this.sample=0;this.rootOpen=null;this.msvcClosed=false;this.trace=[];this.counters=[0,0,0,0];
  this.tail=Buffer.alloc(127);this.tailLength=0;this.tailBytes=0;this.received=0;this.failure=null;this.terminal=null;this.ended=false;
 }
 fail(reason) {
  if(!this.failure){this.failure=failures.has(reason)?reason:'reader-unavailable';this.tailBytes=this.tailLength;this.tail=Buffer.alloc(0);this.tailLength=0;}
 }
 feed(chunk) {
  try{this.ledger.stream(chunk.length);}catch{this.fail('stream-limit');return;}
  this.received+=chunk.length;
  if(this.received>4096){this.fail('report-limit');return;}
  if(this.failure)return;
  for(const byte of chunk){
   if(this.terminal||this.ended){this.fail('after-terminal');break;}
   if(byte===10){try{this.record(this.tail.subarray(0,this.tailLength).toString('ascii'));this.tailLength=0;}catch(e){this.fail(e.message);break;}}
   else {if(byte<32||byte>126){this.fail('encoding');break;}if(this.tailLength>=127){this.fail('line-limit');break;}this.tail[this.tailLength++]=byte;}
  }
 }
 record(raw) {
  this.ledger.active();let t;try{t=JSON.parse(raw);}catch{throw Error('framing');}
  need(Array.isArray(t)&&t.length===10&&JSON.stringify(t)===raw,'framing');
  need(t[0]===3&&t[1]===this.nonce&&t[2]===this.trace.length+1,'binding');
  need(this.trace.length<32,'record-limit');
  need(t.filter((_,i)=>i!==1).every(Number.isSafeInteger)&&t.filter((_,i)=>i!==1).every(v=>v>=0),'counter');
  const [,,,kind,root,h,f,d,a,detail]=t;
  need(h<=this.grant&&f+a<=200000&&d+(a?2:0)<=50000&&a<=345&&[h,f,d,a].every((v,i)=>v>=this.counters[i]),'counter');
  need(this.mode!=='preflight'||(f===0&&d===0&&a===0),'counter');
  if(kind===10){
   need(this.mode!=='preflight'&&this.rootOpen===0&&root===0&&detail===this.sample+1&&detail<=11,'sample-order');
   const valid=detail<=2?f===0&&d===0:detail===3?f===0&&d===1:detail<=6?f===0&&d>=1:detail===7?f===1:f===[16,256,4096,65536][detail-8];
   need(valid,'sample-counter');this.sample=detail;
  }else if(kind===9){
   need(root===7&&detail>=1&&detail<=8,'terminal');
   if(!this.msvcClosed)need(this.sample>=minimumSamples(f,d),'missing-sample');
  }else{
   const expected=this.expected[this.position];
   need(expected&&kind===expected[0]&&root===expected[1]&&detail===0,'phase-order');
   if(kind===5&&root===0)need(d>=1&&this.sample>=minimumSamples(f,d),'missing-sample');
   if(kind===4)this.rootOpen=root;if(kind===5){this.rootOpen=null;if(root===0)this.msvcClosed=true;}
   this.position++;
  }
  const row=Object.freeze([this.ledger.elapsed(),Object.freeze([...t])]);
  need(Number.isSafeInteger(row[0])&&row[0]>=0&&row[0]<=2520000&&Buffer.byteLength(JSON.stringify(row))<=144,'trace-limit');
  this.trace.push(row);this.counters=[h,f,d,a];
  if(kind===8||kind===9)this.terminal=kind===8?'ok':'error';
 }
 validateSamples(data) {
  if(this.mode==='preflight'){need(this.sample===0,'sample-order');return true;}
  const files=data.files.filter(row=>row.root==='msvc').length, dirs=data.directories.filter(row=>row.root==='msvc').length;
  need(dirs>=1&&this.sample===minimumSamples(files,dirs),'missing-sample');return true;
 }
 end(){if(this.tailLength)this.fail('partial-eof');this.ended=true;if(!this.terminal)this.fail('missing-terminal');return !this.failure;}
}

export function settle(decoder,{code,drained,outputValid=false,sourceValid=false}) {
 decoder.end();
 const closed=!decoder.failure&&drained&&((decoder.terminal==='ok'&&code===0)||(decoder.terminal==='error'&&Number.isInteger(code)&&code!==0));
 const known=closed&&sourceValid&&(decoder.terminal==='error'||outputValid);
 const reason=decoder.failure||(!drained?'undrained':!closed?'bad-close':!sourceValid?'source-invalid':decoder.terminal==='error'?reasons[decoder.trace.at(-1)[1][9]-1]:!outputValid?'output-invalid':'ok');
 return Object.freeze({reader_complete:!!(known&&decoder.terminal==='ok'),charged:known?decoder.counters[0]:decoder.grant,unknown_work:!known,reason,drained:!!drained,tail_bytes:decoder.tailBytes,trace:Object.freeze([...decoder.trace])});
}

export function encodeResult(result) {
 const traces=result.reader_traces??[];need(Array.isArray(traces)&&traces.length<=3,'trace-limit');
 let rows=0;const nonces=new Set();
 const metadata=traces.map(({trace,...rest})=>{
  need(Array.isArray(trace),'trace-limit');rows+=trace.length;
  const base=['reader_complete','charged','unknown_work','reason','drained','tail_bytes','mode','nonce','metrics','metrics_status'];
  const fields=rest.metrics_status==='admitted'?[...base,'output_sha256','output_bytes']:base;
  need(Object.keys(rest).length===fields.length&&Object.keys(rest).every(key=>fields.includes(key)),'trace-metadata-fields');
  need(['preflight','before','after'].includes(rest.mode)&&/^[a-f0-9]{32}$/.test(rest.nonce)&&!nonces.has(rest.nonce),'trace-binding');nonces.add(rest.nonce);
  need(typeof rest.reader_complete==='boolean'&&typeof rest.unknown_work==='boolean'&&typeof rest.drained==='boolean'&&Number.isSafeInteger(rest.charged)&&rest.charged>=0&&rest.charged<=34359738368&&Number.isSafeInteger(rest.tail_bytes)&&rest.tail_bytes>=0&&rest.tail_bytes<=127,'trace-metadata-values');
  need([...reasons,...failures,'undrained','bad-close','ok'].includes(rest.reason),'trace-metadata-reason');
  if(rest.reader_complete){
   const m=rest.metrics;
   need(!rest.unknown_work&&rest.drained&&rest.metrics_status==='admitted'&&/^[a-f0-9]{64}$/.test(rest.output_sha256)&&Number.isSafeInteger(rest.output_bytes)&&rest.output_bytes>0&&rest.output_bytes<=67108864,'trace-metadata-output');
   need(m&&Object.keys(m).length===5&&['strategy','scratch_allocations','scratch_capacity_bytes','maximum_leases','hash_bytes'].every(k=>Object.hasOwn(m,k))&&m.strategy==='reuse'&&m.scratch_allocations===1&&m.scratch_capacity_bytes===1048576&&m.maximum_leases===1&&Number.isSafeInteger(m.hash_bytes)&&m.hash_bytes>=0&&m.hash_bytes<=rest.charged,'trace-metadata-metrics');
  }else need(rest.metrics_status==='unavailable'&&rest.metrics===null,'trace-metadata-output');
  need(trace.length<=32&&trace.every(row=>Buffer.byteLength(JSON.stringify(row))<=144),'trace-limit');return rest;
 });
 need(rows<=69&&Buffer.byteLength(JSON.stringify(metadata))<=2048,'trace-metadata-limit');
 need(Buffer.byteLength(JSON.stringify(traces))<=18432,'trace-limit');
 const bytes=Buffer.from(JSON.stringify(result)+'\n');need(bytes.length<=65536,'result-limit');return bytes;
}
