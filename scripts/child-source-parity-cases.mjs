// Test-only bridge: run the independent documentation assay unchanged, capturing
// each complete-packet verdict for differential Rust validation. No live I/O.
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const assay=pathToFileURL(resolve(import.meta.dirname,'../docs/validation/child-source-components.test.mjs'));
const checker=new URL('./child-source-components.mjs',assay);
let source=readFileSync(assay,'utf8');
const testImport="import {test} from 'node:test';";
const checkerImport="import {verifyChildSources, decodeChildSource, renderChildInstructions, declaredStartup, wholeBodySourceDigest} from './child-source-components.mjs';";
if(!source.includes(testImport)||!source.includes(checkerImport)) throw Error('assay bridge imports changed');
source=source.replace(testImport,`let current; let tests=0; const cases=[]; const queued=[]; function test(name,fn){queued.push([name,fn]);}`)
 .replace(checkerImport,`import {verifyChildSources as verify, decodeChildSource, renderChildInstructions, declaredStartup, wholeBodySourceDigest} from ${JSON.stringify(checker.href)};
 function verifyChildSources(p){let accepted=false;try{const result=verify(p);accepted=true;return result;}finally{const {run,parentInput,worker,reviewer,snapshot,reportContext}=p;cases.push({name:current,accepted,packet:{run,parentInput,worker,reviewer,snapshot,reportContext}});}}`)
 .replaceAll('import.meta.url',JSON.stringify(assay.href));
source+=`
for(const [name,fn] of queued){current=name; tests++; fn();}
// Additional full-context metadata boundaries, arranged with the existing
// assay's resealing helpers. These do not change its pinned 129-test count.
function metadataPacket(length,padding=0){
 const p=structuredClone(packet),o=p.reviewer;
 const path={encoding:'unix-bytes',base64:Buffer.from('/'+'x'.repeat(length-1)).toString('base64')};
 o.plan.task.group.absolutePath=path;o.allocation.group.absolutePath=path;
 o.allocation.allocatedWorktree.canonicalRoot=path;o.allocation.allocatedWorktree.canonicalPrivateGitDirectory=path;
 if(padding){const old=o.allocation.allocationId;replaceDigest(o,old,old+'x'.repeat(padding));}
 reseal(o.allocation.allocatedWorktree,'worktreeBindingDigest','orkworks.worktree-binding.v1');refreshOwner(o);return p;
}
function metadataSize(p){const o=p.reviewer;return Buffer.byteLength(wire({key:o.allocation.reservation,value:{schemaVersion:1,body:o.allocation,lastCommit:o.receipt.commit}}));}
let low=1,high=4096;
while(low<high){const mid=Math.ceil((low+high)/2);if(metadataSize(metadataPacket(mid))<=16384)low=mid;else high=mid-1;}
const padding=16384-metadataSize(metadataPacket(low));
for(const extra of [0,1]){
 const p=metadataPacket(low,padding+extra);assert.equal(metadataSize(p),16384+extra);
 current='complete metadata wrapper '+(extra?'successor rejected':'inclusive maximum accepted');
 if(extra)assert.throws(()=>verifyChildSources(p),/metadata complete wrapper 16 KiB/);else assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
}
process.stdout.write(JSON.stringify({tests,cases}));
`;
try {await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));} catch(error) {console.error(error.message); process.exitCode=1;}
