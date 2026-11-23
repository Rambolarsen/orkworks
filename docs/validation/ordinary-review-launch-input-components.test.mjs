import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {encodeProposal,decodeProposal,encodeConfiguration,decodeConfiguration,encodePrepared,decodePrepared,preparedDigest,encodePreviewRequest,decodePreviewRequest,validateJoin} from './ordinary-review-launch-input-components.mjs';
const v=JSON.parse(readFileSync(new URL('./ordinary-review-launch-input-vectors.json',import.meta.url)));
const copy=structuredClone, json=x=>Buffer.from(JSON.stringify(x));
const proposal=()=>copy(v.proposal), configuration=()=>copy(v.configuration), prepared=()=>copy(v.prepared);
const request=()=>({schemaVersion:1,expectedProposalRevision:'1',draft:copy(v.proposal.proposedDraft)});
const max='18446744073709551615';
function setAt(value,path,update){let node=value;for(const key of path.slice(0,-1))node=node[key];const key=path.at(-1);node[key]=update(node[key]);}
function facts(p=prepared(),draftInput=null) {
 const owners=Object.fromEntries(['workspace','proposal','source','artifact','configuration'].map(k=>[k,Object.freeze({})]));
 const entry={prepared:copy(p),draft:copy(draftInput),preparedAtMs:1000,expiresAtMs:31000,owners};
 const current={...Object.fromEntries(['workspace','proposal','source','artifact','configuration'].map(k=>[k,copy(p[k])])),owners:{...owners},status:'proposed',targetSessionId:null,sourceEligible:true,configurationQualified:true,nowMs:2000};
 return {entry,current};
}
test('independent literal bytes, complete byte counts and domain-separated SHA-256',()=>{
 for(const [value,encode,decode,key] of [[proposal(),encodeProposal,decodeProposal,'Proposal'],[configuration(),encodeConfiguration,decodeConfiguration,'Configuration'],[prepared(),encodePrepared,decodePrepared,'Prepared']]) {
  const bytes=encode(value);assert.equal(bytes.toString(),v['canonical'+key]);assert.equal(bytes.length,v.bytes[key.toLowerCase()]);assert.deepEqual(decode(bytes),value);
 }
 assert.equal(preparedDigest(prepared()),v.digest);
 assert.deepEqual(decodePreviewRequest(encodePreviewRequest(request())),request());
});
for(const [name,raw] of Object.entries({duplicate:v.canonicalPrepared.replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'),whitespace:v.canonicalPrepared+'\n',escaped:v.canonicalPrepared.replace('schemaVersion','schema\\u0056ersion'),bom:'\ufeff'+v.canonicalPrepared,invalidUTF8:Buffer.from([0xff]),rawOverflow:Buffer.alloc(49153,32),reordered:JSON.stringify({previewId:v.prepared.previewId,...v.prepared})}))
 test(`canonical decoder denies ${name}`,()=>assert.throws(()=>decodePrepared(Buffer.from(raw))));
for(const [name,path,change] of [
 ['unknown',[],p=>p.extra=1],['schema',['schemaVersion'],()=>2],['intent',['proposal','intent'],()=> 'start_fix_session'],
 ['zero',['proposal','proposalRevision'],()=> '0'],['overflow',['proposal','proposalRevision'],()=> '18446744073709551616'],['leading-zero',['proposal','proposalRevision'],()=> '01'],['numeric',['source','runGeneration'],()=>1],
 ['uuid',['previewId'],()=> 'not-uuid'],['uuid-version',['previewId'],x=>x.replace('-4111-','-1111-')],['opaque',['proposal','recommendationId'],()=> '../other'],
 ['depth',['proposal','chain','depth'],()=>3],['fraction',['proposal','chain','depth'],()=>0.5],['negative-zero',['proposal','chain','depth'],()=> -0],
 ['profile',['configuration','profile'],()=> 'orchestrated-child'],['generic-shell',['configuration','codingToolId'],()=> 'generic-shell'],['skills',['configuration','skills'],()=> ['skill']],['images',['configuration','images'],()=> ['image']],['effort',['configuration','reasoningEffort'],()=> 'high'],['sparse-argv',['configuration','argv'],()=>Array(1)],
 ['platform',['configuration','platform'],()=> 'unknown'],['relative-executable',['configuration','executable'],()=> 'bin/tool'],['extra-env',['configuration'],c=>({...c,env:{}})],
 ['runtime',['source','runtimeInstanceId'],()=> '00000009-1111-4111-8111-111111111111'],['source-session',['source','sessionId'],()=> 'other'],['cwd',['source','effectiveWorktree'],()=> '/other'],['lifecycle',['source','lifecycle'],()=> 'ended'],['organization',['source','organization'],()=> 'archived'],
 ['head',['source','git','head'],()=> ({kind:'tag',reference:'main'})],['commit',['source','git','head'],()=> ({kind:'detached',commit:'ABC'})],['branch-join',['artifact','branchIdentity'],()=> 'other'],['kind',['artifact','kind'],()=> 'plan'],['generation-exhausted',['artifact','generation'],()=> max],['pin-type',['artifact','pinned'],()=> 'false'],
 ['nul',['userInstruction'],()=> 'a\0b'],['escape-control',['userInstruction'],()=> 'a\x1bb'],['DEL',['userInstruction'],()=> 'a\x7fb'],['surrogate',['userInstruction'],()=> '\ud800'],['path-control',['workspace','canonicalRoot'],()=> '/a\nb'],
 ]) test(`closed prepared codec denies ${name}`,()=>{const p=prepared();if(path.length)setAt(p,path,change);else change(p);assert.throws(()=>encodePrepared(p));});
test('zero artifact epoch and maximum positive counters are distinct valid domains',()=>{
 const p=prepared();p.proposal.proposalRevision=max;p.proposal.request.requestRevision=max;p.proposal.sourceRuntime.runGeneration=max;p.source.runGeneration=max;p.source.eligibilityRevision=max;p.workspace.workspaceGeneration=max;p.configuration.ownerRevision=max;p.configuration.capabilityEvidenceRevision=max;p.artifact.generation='18446744073709551614';
 assert.deepEqual(decodePrepared(encodePrepared(p)),p);
});
test('Unicode, CRLF, tab and non-BMP are preserved without normalization',()=>{
 const p=prepared();assert.equal(decodePrepared(encodePrepared(p)).userInstruction,p.userInstruction);
 const q=prepared();q.userInstruction=q.userInstruction.normalize('NFC');assert.notEqual(preparedDigest(q),v.digest);
 q.userInstruction=p.userInstruction.replaceAll('\r\n','\n');assert.notEqual(preparedDigest(q),v.digest);
});
test('detached head preserves exact commit, source-only field is not normalized into a branch',()=>{
 for(const width of [40,64]) {const p=prepared();p.source.git.head={kind:'detached',commit:'e'.repeat(width)};assert.deepEqual(decodePrepared(encodePrepared(p)),p);}
});
test('explicit edited draft is distinct from the original proposal and joins exactly',()=>{
 const p=prepared();p.configuration.modelId='edited-model';p.configuration.argv=['--model','edited-model'];p.userInstruction='Edited review instruction';const {entry,current}=facts(p,{codingToolId:p.configuration.codingToolId,modelId:p.configuration.modelId,userInstruction:p.userInstruction});assert.equal(validateJoin(p,entry,current),true);assert.notEqual(preparedDigest(p),v.digest);
 entry.draft.userInstruction+=' changed';assert.equal(validateJoin(p,entry,current),false);
});
test('original inspected entry joins only exact current synthetic owners and values',()=>{
 const p=prepared(),{entry,current}=facts(p);assert.equal(validateJoin(p,entry,current),true);
 const before=copy(p);validateJoin(p,entry,current);assert.deepEqual(p,before);
});
for(const key of ['workspace','proposal','source','artifact','configuration']) test(`same labels cannot replace ${key} owner`,()=>{
 const p=prepared(),{entry,current}=facts(p);current.owners[key]=Object.freeze({});assert.equal(validateJoin(p,entry,current),false);
});
for(const [name,path,value] of [
 ['proposal revision',['proposal','proposalRevision'],'2'],['source generation',['source','runGeneration'],'2'],['lifecycle',['source','lifecycle'],'ended'],['source cwd',['source','effectiveWorktree'],'/other'],['Git directory',['source','git','commonDirectory'],'/other/.git'],['Git head',['source','git','head','reference'],'other'],['artifact kind',['artifact','kind'],'implementation'],['pin',['artifact','pinned'],true],['epoch',['artifact','generation'],'2'],['capture',['artifact','ownerCaptureId'],'00000009-1111-4111-8111-111111111111'],['configuration revision',['configuration','ownerRevision'],'2'],['argv',['configuration','argv'],['different']],['sidecar',['workspace','sidecarInstanceId'],'00000009-1111-4111-8111-111111111111']])
 test(`current ${name} change denies original entry`,()=>{const p=prepared(),{entry,current}=facts(p);setAt(current,path,()=>value);assert.equal(validateJoin(p,entry,current),false);});
for(const [key,values] of Object.entries({sourceEligible:[undefined,false,'true',1,{}],configurationQualified:[undefined,false,'true',1,{}],status:[undefined,'accepted','completed'],targetSessionId:[undefined,'other'],nowMs:[undefined,NaN,-1,999,31000,32000]}))
 test(`unknown/malformed ${key} is never positive proof`,()=>{for(const value of values){const p=prepared(),{entry,current}=facts(p);current[key]=value;assert.equal(validateJoin(p,entry,current),false);}});
test('source/admission orders remain logical: current change before comparison denies; after comparison cannot undo its result',()=>{
 const p=prepared(),{entry,current}=facts(p);const admitted=validateJoin(p,entry,current);current.artifact.pinned=true;assert.equal(admitted,true);assert.equal(validateJoin(p,entry,current),false);
});

test('exhausted proposal revision remains decodable history but cannot join fresh inspection',()=>{
 const p=prepared();p.proposal.proposalRevision=max;const {entry,current}=facts(p);
 assert.doesNotThrow(()=>decodePrepared(encodePrepared(p)));assert.equal(validateJoin(p,entry,current),false);
});
test('an edited instruction cannot masquerade as the original null-draft path',()=>{
 const p=prepared();p.userInstruction='Edited';const {entry,current}=facts(p);entry.draft=null;
 assert.equal(validateJoin(p,entry,current),false);
});
function leafPaths(value,path=[]) {
 if(value===null || typeof value!=='object' || Array.isArray(value))return [path];
 return Object.keys(value).flatMap(k=>leafPaths(value[k],[...path,k]));
}
for(const path of leafPaths(v.prepared)) test(`commit every leaf: ${path.join('.')}`,()=>{
 const p=prepared();setAt(p,path,value=>typeof value==='string'?(/^[0-9]+$/.test(value)?String(BigInt(value)+1n):/^[0-9a-f]{64}$/.test(value)?(value[0]==='a'?'b':'a')+value.slice(1):value+'x'):typeof value==='number'?value+1:typeof value==='boolean'?!value:Array.isArray(value)?['unexpected']:'unexpected');
 let digest;try {digest=preparedDigest(p);}catch{return;}assert.notEqual(digest,v.digest);
});
function objectPaths(value,path=[]) {
 if(value===null || typeof value!=='object' || Array.isArray(value))return [];
 return [{path,keys:Object.keys(value)},...Object.keys(value).flatMap(k=>objectPaths(value[k],[...path,k]))];
}
for(const {path,keys} of objectPaths(v.prepared)) test(`closed nested keys: ${path.join('.')||'prepared'}`,()=>{
 for(const key of keys){const p=prepared();let node=p;for(const k of path)node=node[k];delete node[key];assert.throws(()=>encodePrepared(p));}
 const p=prepared();let node=p;for(const k of path)node=node[k];node.extra=1;assert.throws(()=>encodePrepared(p));
});
// Constructions use JSON byte counts independently of the implementation encoder.
function fillInstruction(value,setter,target) {
 setter(value,'');const available=target-json(value).length;assert(available>0);
 const body='\n'.repeat(Math.floor(available/2))+'x'.repeat(available%2);assert(Buffer.byteLength(body)<=16384);setter(value,body);assert.equal(json(value).length,target);return value;
}
function fullConfiguration(){
 const c=configuration();c.argv=[...Array(11).fill('x'.repeat(1024)),''];
 const gap=12288-json(c).length;assert(gap>=0 && gap<=1024);c.argv[11]='x'.repeat(gap);assert.equal(json(c).length,12288);return c;
}
test('valid exact proposal 24KiB and one-byte overflow',()=>{
 const p=fillInstruction(proposal(),(p,s)=>p.proposedDraft.userInstruction=s,24576);
 assert.equal(encodeProposal(p).length,24576);p.proposedDraft.userInstruction+='x';assert.throws(()=>encodeProposal(p));
});
test('valid exact configuration 12KiB and one-byte overflow',()=>{
 const c=fullConfiguration();assert.equal(encodeConfiguration(c).length,12288);c.argv[11]+='x';assert.throws(()=>encodeConfiguration(c));
});
test('valid exact prepared 48KiB and one-byte overflow',()=>{
 let p=prepared();p.configuration=fullConfiguration();p.proposal.proposedDraft.userInstruction='p'.repeat(12288);
 p=fillInstruction(p,(p,s)=>p.userInstruction=s,49152);
 assert.equal(encodePrepared(p).length,49152);
 const d={codingToolId:p.configuration.codingToolId,modelId:p.configuration.modelId,userInstruction:p.userInstruction};
 assert(encodePreviewRequest({schemaVersion:1,expectedProposalRevision:p.proposal.proposalRevision,draft:d}).length<=24576);
 const {entry,current}=facts(p,d);assert.equal(validateJoin(p,entry,current),true);
 p.userInstruction+='x';assert.throws(()=>encodePrepared(p));
});
test('valid exact canonical request 24KiB; raw and encoded overflow deny',()=>{
 const r=fillInstruction(request(),(r,s)=>r.draft.userInstruction=s,24576);
 assert.equal(encodePreviewRequest(r).length,24576);assert.deepEqual(decodePreviewRequest(json(r)),r);
 r.draft.userInstruction+='x';assert.throws(()=>encodePreviewRequest(r));assert.throws(()=>decodePreviewRequest(json(r)));
 assert.deepEqual(decodePreviewRequest(encodePreviewRequest({...request(),draft:null})),{...request(),draft:null});
});
test('individual instruction/request/model/argv/ID limits count UTF-8 bytes',()=>{
 const p=proposal();p.proposedDraft.userInstruction='x'.repeat(16384);p.request.text='r'.repeat(2048);p.proposedDraft.modelId='m'.repeat(256);p.recommendationId='r'.repeat(128);assert.doesNotThrow(()=>encodeProposal(p));
 for(const path of [['proposedDraft','userInstruction'],['request','text'],['proposedDraft','modelId'],['recommendationId']]){const q=copy(p);setAt(q,path,s=>s+'x');assert.throws(()=>encodeProposal(q));}
 const c=configuration();c.argv=Array(64).fill('');assert.doesNotThrow(()=>encodeConfiguration(c));c.argv.push('');assert.throws(()=>encodeConfiguration(c));
 c.argv=['é'.repeat(512)];assert.doesNotThrow(()=>encodeConfiguration(c));c.argv[0]+='x';assert.throws(()=>encodeConfiguration(c));
 const q=proposal();q.proposedDraft.userInstruction='🛠'.repeat(4096);assert.doesNotThrow(()=>encodeProposal(q));q.proposedDraft.userInstruction+='x';assert.throws(()=>encodeProposal(q));
});
test('Windows path form and argument order/empty values are exact logical bytes',()=>{
 const p=prepared();p.configuration.platform='windows_x64';p.configuration.executable='C:\\fixture\\reviewer.exe';p.workspace.canonicalRoot='C:\\fixture\\repo';p.source.effectiveWorktree=p.workspace.canonicalRoot;p.source.git.commonDirectory='C:\\fixture\\repo\\.git';
 p.configuration.argv=['','--model','fixture-model'];assert.deepEqual(decodePrepared(encodePrepared(p)),p);
 const digest=preparedDigest(p);p.configuration.argv.reverse();assert.notEqual(preparedDigest(p),digest);
 p.configuration.executable='\\drive-relative';assert.throws(()=>encodePrepared(p));
});
test('all strict request fields and raw representation are bounded',()=>{
 for(const r of [{...request(),extra:1},{...request(),expectedProposalRevision:'0'},{...request(),draft:{...request().draft,sourceSessionId:'other'}}])assert.throws(()=>encodePreviewRequest(r));
 assert.throws(()=>decodePreviewRequest(Buffer.from('{"schemaVersion":1,"schemaVersion":1,"expectedProposalRevision":"1","draft":null}')));
});

for(const [key,path] of [['workspace',['workspace','workspaceGeneration']],['proposal',['proposal','proposalRevision']],['source',['source','eligibilityRevision']],['artifact',['artifact','pinned']],['configuration',['configuration','ownerRevision']]])
 test(`caller cannot substitute captured ${key} with old current facts`,()=>{const p=prepared(),{entry,current}=facts(p);setAt(p,path,x=>typeof x==='boolean'?!x:'2');assert.equal(validateJoin(p,entry,current),false);});
for(const key of ['codingToolId','modelId','userInstruction']) test(`selected edited draft must join ${key}`,()=>{
 const p=prepared(),d=copy(p.proposal.proposedDraft),{entry,current}=facts(p,d);entry.draft[key]+='-changed';assert.equal(validateJoin(p,entry,current),false);
});
test('expiry and simulated owner facts never renew or become implicit truth',()=>{
 for(const change of [e=>delete e.draft,e=>e.expiresAtMs++,e=>e.preparedAtMs=-1,e=>e.owners.source='same-label',e=>delete e.owners.configuration]){
  const p=prepared(),{entry,current}=facts(p);change(entry);assert.equal(validateJoin(p,entry,current),false);
 }
});
test('native path, reference, branch and selection domain boundaries are literal values only',()=>{
 const p=prepared();p.configuration.executable='/'+'x'.repeat(4095);p.workspace.canonicalRoot='/'+'w'.repeat(4095);p.source.effectiveWorktree=p.workspace.canonicalRoot;p.source.git.commonDirectory='/'+'g'.repeat(4095);p.source.git.head.reference='b'.repeat(1024);p.artifact.branchIdentity=p.source.git.head.reference;p.artifact.anchor='a'.repeat(2048);p.artifact.reference='r'.repeat(2048);p.artifact.selectionId='s'.repeat(128);
 assert.doesNotThrow(()=>encodePrepared(p));
 for(const path of [['configuration','executable'],['source','git','commonDirectory'],['artifact','anchor'],['artifact','reference'],['artifact','selectionId']]){const q=copy(p);setAt(q,path,s=>s+'x');assert.throws(()=>encodePrepared(q));}
});
