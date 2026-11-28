// Isolated live v2 wire domain. Progress alone never admits candidate inputs.
import { performance } from 'node:perf_hooks';
export const MiB = 1024 * 1024;
const roots = ['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'];
export const phases = Object.freeze([
 ['start',''],['ready',''],['archives-begin',''],['archives-end',''],
 ...roots.flatMap(r=>[['root-begin',r],['root-end',r]]),
 ['serialize-begin',''],['serialize-end',''],['ok','']
].map(Object.freeze));
const reasons = new Set(['bad-request','input-denied','hash-limit','input-limit','identity-drift','format-denied','output-limit','reader-unavailable']);
export class WorkBudget {
 constructor({hashLimit=32*1024*MiB,streamLimit=64*MiB,seconds=2400,clock=()=>performance.now()}={}) {
  if(!Number.isSafeInteger(hashLimit)||hashLimit<0||hashLimit>32*1024*MiB||!Number.isSafeInteger(streamLimit)||streamLimit<0||streamLimit>64*MiB||seconds<=0||seconds>2400)throw Error('budget-range');
  Object.assign(this,{hashLimit,streamLimit,clock});this.started=clock();this.deadline=this.started+seconds*1000;this.finalDeadline=this.deadline+120000;
  this.hashed=0;this.reserved=0;this.streamed=0;this.parentUsed=0;this.readerOutstanding=false;this.failurePhase=false;
 }
 elapsed(){return Math.floor(this.clock()-this.started);}
 active(){this.check();}
 check(){if(this.failurePhase||this.clock()>=this.deadline)throw Error('time-limit');}
 emergency(){this.failurePhase=true;if(this.clock()>=this.finalDeadline)throw Error('publication-time-limit');}
 hash(n){
  if(!Number.isSafeInteger(n)||n<0||n>this.remaining())throw Error('hash-limit');
  if(this.readerOutstanding&&this.parentUsed+n>129*MiB)throw Error('admission-work-limit');
  this.hashed+=n;if(this.readerOutstanding)this.parentUsed+=n;
 }
 reserve(n){
  this.check();if(!Number.isSafeInteger(n)||n<0||n>this.remaining())throw Error('hash-limit');
  this.reserved+=n;let done=false;
  return actual=>{if(done||!Number.isSafeInteger(actual)||actual<0||actual>n)throw Error('reservation-invalid');done=true;this.reserved-=n;this.hashed+=actual;};
 }
 readerGrant(){
  this.check();if(this.readerOutstanding||this.reserved)throw Error('reservation-outstanding');
  const grant=this.remaining()-132*MiB;if(grant<=0)throw Error('hash-limit');
  const settle=this.reserve(grant);this.readerOutstanding=true;this.parentUsed=0;
  return {grant,finish:actual=>{settle(actual);this.readerOutstanding=false;this.parentUsed=0;}};
 }
 stream(n){if(!Number.isSafeInteger(n)||n<0)throw Error('stream-limit');this.streamed+=n;if(this.streamed>this.streamLimit)throw Error('stream-limit');}
 remaining(){return this.hashLimit-this.hashed-this.reserved;}
}
export class Decoder {
 constructor({nonce,mode,grant,ledger,source='child'}) {
  if(!/^[a-f0-9]{32}$/.test(nonce)||!['preflight','before','after'].includes(mode)||!Number.isSafeInteger(grant)||grant<0||grant>32*1024*MiB) throw Error('bad-request');
  Object.assign(this,{nonce,mode,grant,ledger,source});
  this.trace=[]; this.tail=Buffer.alloc(191); this.tailLength=0; this.tailBytes=0;
  this.received=0; this.failure=null; this.terminal=null; this.counters=[0,0,0,0]; this.ended=false;
  this.expected=mode==='preflight'?[['start',''],['ready',''],['serialize-begin',''],['serialize-end',''],['ok','']]:phases;
 }
 fail(reason) {
  if(!this.failure) {this.failure=reason; this.tailBytes=this.tailLength; this.tail=Buffer.alloc(0); this.tailLength=0;}
 }
 feed(chunk) {
  // Charge the whole received chunk even when parsing previously failed.
  try {this.ledger.stream(chunk.length);} catch {this.fail('stream-limit');return;}
  this.received+=chunk.length;
  if(this.received>4096) {this.fail('report-limit');return;}
  if(this.failure)return;
  for(const byte of chunk) {
   if(this.terminal || this.ended) {this.fail('after-terminal');break;}
   if(byte===10) {
    try {this.record(this.tail.subarray(0,this.tailLength).toString('ascii'));this.tailLength=0;}
    catch(e) {this.fail(e.message);break;}
   } else {
    if(byte<32||byte>126) {this.fail('encoding');break;}
    if(this.tailLength>=191) {this.fail('line-limit');break;}
    this.tail[this.tailLength++]=byte;
   }
  }
 }
 record(raw) {
  this.ledger.active();
  let t; try {t=JSON.parse(raw);} catch {throw Error('framing');}
  if(!Array.isArray(t)||![9,10].includes(t.length)||JSON.stringify(t)!==raw) throw Error('framing');
  if(t[0]!==2||t[1]!==this.nonce||t[2]!==this.trace.length+1) throw Error('binding');
  if(this.trace.length>=21) throw Error('record-limit');
  if(!t.slice(5,9).every(Number.isSafeInteger)||t.slice(5,9).some((x,i)=>x<0||x<this.counters[i])) throw Error('counter');
  const [h,f,d,a]=t.slice(5,9);
  if(h>this.grant||f+a>200000||d+(a?2:0)>50000||a>345 || (this.mode==='preflight'&&(f||d||a))) throw Error('counter');
  if(t[3]==='error') {
   if(t.length!==10||t[4]!==''||!reasons.has(t[9])) throw Error('terminal');
  } else {
   const expected=this.expected[this.trace.length];
   if(t.length!==9||!expected||t[3]!==expected[0]||t[4]!==expected[1]) throw Error('phase-order');
  }
  const row=Object.freeze([this.ledger.elapsed(),Object.freeze([...t])]);
  if(Buffer.byteLength(JSON.stringify(row))>256) throw Error('trace-limit');
  this.trace.push(row); this.counters=[h,f,d,a];
  if(t[3]==='ok'||t[3]==='error')this.terminal=t[3];
 }
 end() {
  if(this.tailLength) this.fail('partial-eof');
  this.ended=true;
  if(!this.terminal)this.fail('missing-terminal');
  return !this.failure;
 }
}
// Pure settlement; the ledger owns exactly-once reservation settlement.
export function settle(decoder,{code,drained,outputValid=false,sourceValid=true}) {
 decoder.end();
 const closed=!decoder.failure&&drained&&((decoder.terminal==='ok'&&code===0)||(decoder.terminal==='error'&&Number.isInteger(code)&&code!==0));
 const known=closed&&sourceValid&&(decoder.terminal==='error'||outputValid);
 return Object.freeze({reader_complete:!!(known&&decoder.terminal==='ok'),
  charged:known?decoder.counters[0]:decoder.grant,unknown_work:!known,
  reason:decoder.failure||(!drained?'undrained':!closed?'bad-close':!sourceValid?'source-invalid':decoder.terminal==='error'?decoder.trace.at(-1)[1][9]:!outputValid?'output-invalid':'ok'),
  drained:!!drained,tail_bytes:decoder.tailBytes,trace:Object.freeze([...decoder.trace])});
}
export function encodeResult(result) {
 if(Buffer.byteLength(JSON.stringify(result.reader_traces??result.traces??[]))>18432) throw Error('trace-limit');
 const bytes=Buffer.from(JSON.stringify(result)+'\n');
 if(bytes.length>65536) throw Error('result-limit');
 return bytes;
}
// Parse complete JSON while independently rejecting duplicate object keys at every depth.
export function parseStrict(raw) {
 const value=JSON.parse(raw);let i=0;
 const white=()=>{while(/\s/.test(raw[i]??'')&&i<raw.length)i++;};
 const string=()=>{const begin=i++;while(i<raw.length){if(raw[i]==='\\'){i+=2;continue;}if(raw[i++]==='"')return JSON.parse(raw.slice(begin,i));}throw Error('json-string');};
 const scan=()=>{white();if(raw[i]==='{'){i++;white();const keys=new Set();if(raw[i]==='}'){i++;return;}for(;;){white();const key=string();if(keys.has(key))throw Error('duplicate-key');keys.add(key);white();i++;scan();white();if(raw[i++]==='}')return;}}
 else if(raw[i]==='['){i++;white();if(raw[i]===']'){i++;return;}for(;;){scan();white();if(raw[i++]===']')return;}}
 else if(raw[i]==='"')string();else {while(i<raw.length&&!/[\s,}\]]/.test(raw[i]))i++;}};
 scan();white();if(i!==raw.length)throw Error('json-trailing');return value;
}
