// Synthetic-only protocol; no filesystem, tool discovery or admission authority.
import { performance } from 'node:perf_hooks';
export const MiB = 1024 * 1024;
const roots = ['msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source'];
export const phases = Object.freeze([
 ['start',''],['ready',''],['archives-begin',''],['archives-end',''],
 ...roots.flatMap(r=>[['root-begin',r],['root-end',r]]),
 ['serialize-begin',''],['serialize-end',''],['ok','']
].map(Object.freeze));
const reasons = new Set(['bad-request','input-denied','hash-limit','input-limit','identity-drift','format-denied','output-limit','fixture-assertion','reader-unavailable']);
export class Ledger {
 constructor() {
  this.started=performance.now(); this.hashBytes=0; this.reserved=0;
  this.childBytes=0; this.injectedBytes=0; this.vectors=0; this.vectorBytes=0;
  this.nativeChildren=0; this.nodeChildren=0; this.terminations=0;
  this.unknownWork=false; this.modelBytes=0; this.modelUnknownBytes=0; this.scratchBytes=0;
  this.reservations=new Map();
 }
 elapsed() { return Math.floor(performance.now()-this.started); }
 active() { if(this.elapsed()>=120000) throw Error('active-timeout'); }
 beginVector() { this.active(); if(++this.vectors>64) throw Error('vector-limit'); this.vectorBytes=0; }
 stream(n,source) {
  if(!Number.isSafeInteger(n)||n<0) throw Error('stream-limit');
  if(source==='injected') {this.injectedBytes+=n; this.vectorBytes+=n;}
  else if(source==='child') this.childBytes+=n;
  else throw Error('stream-source');
  if(this.childBytes+this.injectedBytes>MiB || this.injectedBytes>512*1024 || (source==='injected'&&this.vectorBytes>8192)) throw Error('stream-limit');
 }
 chargeHash(n,model=false) {
  if(!Number.isSafeInteger(n)||n<0||this.hashBytes+this.reserved+n>64*MiB) throw Error('hash-limit');
  this.hashBytes+=n; if(model)this.modelBytes+=n;
 }
 reserve(n,model=false) {
  this.active(); if(!Number.isSafeInteger(n)||n<0||this.hashBytes+this.reserved+n>64*MiB) throw Error('hash-limit');
  const key=Symbol(); this.reservations.set(key,{grant:n,model}); this.reserved+=n; return key;
 }
 settle(key,used,unknown=false,model=false) {
  if(!this.reservations.has(key)) throw Error('duplicate-settlement');
  const entry=this.reservations.get(key),grant=entry.grant;model ||= entry.model;
  if(!Number.isSafeInteger(used)||used<0||used>grant) throw Error('hash-limit');
  this.reservations.delete(key); this.reserved-=grant;
  this.hashBytes+=unknown?grant:used; this.unknownWork ||= unknown&&!model; if(model){this.modelBytes+=unknown?grant:used;if(unknown)this.modelUnknownBytes+=grant;}
 }
 scratch(n) {
  if(!Number.isSafeInteger(n)||n<0||this.scratchBytes+n>8*MiB) throw Error('scratch-limit');
  this.scratchBytes+=n;
 }
}
export class Decoder {
 constructor({nonce,mode,grant,ledger,source='child'}) {
  if(!/^[a-f0-9]{32}$/.test(nonce)||!['reference','reuse','guards','transport'].includes(mode)||!Number.isSafeInteger(grant)||grant<0||grant>64*MiB) throw Error('bad-request');
  Object.assign(this,{nonce,mode,grant,ledger,source});
  this.trace=[]; this.tail=Buffer.alloc(191); this.tailLength=0; this.tailBytes=0;
  this.received=0; this.failure=null; this.terminal=null; this.counters=[0,0,0,0]; this.ended=false;
  this.expected=mode==='transport'?[['start','']]:mode==='guards'?[['start',''],['ready',''],['ok','']]:phases;
 }
 fail(reason) {
  if(!this.failure) {this.failure=reason; this.tailBytes=this.tailLength; this.tail=Buffer.alloc(0); this.tailLength=0;}
 }
 feed(chunk) {
  // Charge the whole received chunk even when parsing previously failed.
  try {this.ledger.stream(chunk.length,this.source);} catch {this.fail('stream-limit');return;}
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
  if(t[0]!==1||t[1]!==this.nonce||t[2]!==this.trace.length+1) throw Error('binding');
  if(this.trace.length>=21) throw Error('record-limit');
  if(!t.slice(5,9).every(Number.isSafeInteger)||t.slice(5,9).some((x,i)=>x<0||x<this.counters[i])) throw Error('counter');
  const [h,f,d,a]=t.slice(5,9);
  if(h>this.grant||f+a>2048||d+(a?2:0)>256||a>1 || (['guards','transport'].includes(this.mode)&&(f||d||a))) throw Error('counter');
  if(t[3]==='error') {
   if(t.length!==10||t[4]!==''||!reasons.has(t[9])||this.mode==='transport') throw Error('terminal');
  } else {
   const expected=this.expected[this.trace.length];
   if(t.length!==9||!expected||t[3]!==expected[0]||t[4]!==expected[1]) throw Error('phase-order');
   if(this.mode==='transport'&&h!==0) throw Error('counter');
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
  drained:!!drained,tail_bytes:decoder.tailBytes,trace:decoder.trace});
}
export function encodeResult(result) {
 if(Buffer.byteLength(JSON.stringify(result.traces??[]))>18432) throw Error('trace-limit');
 const bytes=Buffer.from(JSON.stringify(result)+'\n');
 if(bytes.length>65536) throw Error('result-limit');
 return bytes;
}
