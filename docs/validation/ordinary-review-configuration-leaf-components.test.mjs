import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {encodeCatalog,decodeCatalog,encodeIntegration,decodeIntegration,catalogDigest,integrationDigest,measureCapture,validateLeafJoin} from './ordinary-review-configuration-leaf-components.mjs';
const v=JSON.parse(readFileSync(new URL('./ordinary-review-configuration-leaf-vectors.json',import.meta.url)));
const copy=structuredClone,json=x=>Buffer.from(JSON.stringify(x));
const catalog=()=>copy(v.catalog),integration=()=>copy(v.integration),prepared=()=>copy(v.prepared),max='18446744073709551615';
function setAt(x,path,fn){let at=x;for(const key of path.slice(0,-1))at=at[key];const key=path.at(-1);at[key]=fn(at[key]);}
function facts(c=catalog(),i=integration(),p=prepared()){
 const owners=Object.fromEntries(['workspace','proposal','source','artifact','configuration'].map(k=>[k,Object.freeze({})]));
 const resourceOwners=Object.fromEntries(i.resources.map(r=>[r.id,Object.freeze({})]));
 const preparedEntry={prepared:copy(p),draft:null,preparedAtMs:1000,expiresAtMs:31000,owners};
 const preparedCurrent={...Object.fromEntries(['workspace','proposal','source','artifact','configuration'].map(k=>[k,copy(p[k])])),owners:{...owners},status:'proposed',targetSessionId:null,sourceEligible:true,configurationQualified:true,nowMs:2000};
 return {expectedInventory:copy(v.expectedInventory),captured:{catalog:copy(c),integration:copy(i),prepared:copy(p),configurationOwner:owners.configuration,resourceOwners:{...resourceOwners},preparedEntry},current:{catalog:copy(c),integration:copy(i),configurationOwner:owners.configuration,resourceOwners:{...resourceOwners},preparedCurrent,sourceFacts:{stockToolIds:copy(v.expectedStockToolIds),stockResolverQualified:true,inventoryQualified:true,fragmentsQualified:true,literalCommandQualified:true,userDocumentState:'absent',requiresLateArgv:false,requiresPromptAuthority:false}}};
}
test('independent complete golden leaf bytes/digests and source byte count',()=>{
 const c=catalog(),i=integration();assert.equal(encodeCatalog(c).toString(),v.canonicalCatalog);assert.equal(encodeIntegration(i,c.platform).toString(),v.canonicalIntegration);
 assert.equal(catalogDigest(c),v.catalogDigest);assert.equal(integrationDigest(i,c.platform),v.integrationDigest);
 assert.deepEqual(decodeCatalog(Buffer.from(v.canonicalCatalog)),c);assert.deepEqual(decodeIntegration(Buffer.from(v.canonicalIntegration),c.platform),i);
 assert.equal(Buffer.from(c.builtinDocumentBase64,'base64').length,v.bytes.builtin);
 assert.deepEqual(measureCapture(c,i),{catalogBytes:v.bytes.catalog,integrationBytes:v.bytes.integration,totalBytes:v.bytes.catalog+v.bytes.integration});
});
for(const [name,path,fn] of [
 ['schema',['schemaVersion'],()=>2],['kind',['kind'],()=> 'other'],['owner UUID',['ownerInstanceId'],()=> 'invalid'],['zero owner',['ownerRevision'],()=> '0'],['exhausted owner',['ownerRevision'],()=> max],['counter syntax',['ownerRevision'],()=> '01'],['overflow',['ownerRevision'],()=> '18446744073709551616'],['build digest',['sidecarBuildId'],()=> 'E'.repeat(64)],['platform',['platform'],()=> 'other'],['generic',['requestedToolId'],()=> 'generic-shell'],['alias',['requestedToolId'],()=> 'gh-copilot'],['ID',['requestedToolId'],()=> '../x'],['present file',['userDocumentState'],()=> 'present'],['invalid file',['userDocumentState'],()=> 'invalid'],['resolver',['resolverPolicy'],()=> 'fallback'],['base64 spelling',['builtinDocumentBase64'],s=>s+'\n'],['relative program',['resolvedCommand','program'],()=> 'tool'],['sparse argv',['resolvedCommand','argv'],()=>Array(1)],['unknown command key',['resolvedCommand'],x=>({...x,env:{}})],['control',['modelId'],()=> 'x\0y'],['surrogate',['modelId'],()=> '\ud800']])
 test(`catalog denies ${name}`,()=>{const c=catalog();setAt(c,path,fn);assert.throws(()=>encodeCatalog(c));});
for(const [name,path,fn] of [
 ['schema',['schemaVersion'],()=>2],['owner zero',['ownerRevision'],()=> '0'],['owner exhausted',['ownerRevision'],()=>max],['workspace',['workspaceId'],()=> 'bad'],['zero workspace generation',['workspaceGeneration'],()=> '0'],['catalog digest',['catalogDigest'],()=> 'bad'],['binding',['adapter','binding'],()=> 'custom'],['representative',['adapter','representativeToolId'],()=>null],['policy',['adapter','policyVersion'],()=> 'other'],['active missing',['activeSelection','toolIds'],()=> []],['active duplicate',['activeSelection','toolIds'],()=> ['codex','codex']],['active order',['activeSelection','toolIds'],()=> ['opencode','codex']],['negative revision',['activeSelection','revision'],()=> '-1'],['unknown role',['inventory',0,'role'],()=> 'other'],['unknown presence',['inventory',0,'presence'],()=> 'optional'],['required absence',['resources',0],x=>({...x,state:'absent',bytesBase64:null})],['missing record',['resources'],x=>x.slice(1)],['extra record',['resources'],x=>[...x,x[0]]],['role substitution',['resources',0,'role'],()=> 'source_asset'],['symlink',['resources',0,'state'],()=> 'symlink'],['unreadable',['resources',0,'state'],()=> 'unreadable'],['absent bytes',['resources',2,'bytesBase64'],()=> 'YQ=='],['regular null',['resources',0,'bytesBase64'],()=>null],['base64 URL',['resources',0,'bytesBase64'],()=> '_w=='],['base64 padding',['resources',0,'bytesBase64'],()=> 'YR=='],['native selector',['runtimePolicy','worktreeInstructions'],()=> 'opencode-scoped-v1'],['prompt generation',['runtimePolicy','promptHook'],()=> 'qualified'],['relative resource',['resources',0,'path'],()=> 'relative'],['sparse inventory',['inventory'],()=>Array(1)],['sparse resources',['resources'],()=>Array(1)]])
 test(`integration denies ${name}`,()=>{const i=integration();setAt(i,path,fn);assert.throws(()=>encodeIntegration(i,v.catalog.platform));});
for(const [name,raw] of Object.entries({duplicate:v.canonicalCatalog.replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'),whitespace:v.canonicalCatalog+'\n',escaped:v.canonicalCatalog.replace('schemaVersion','schema\\u0056ersion'),bom:'\ufeff'+v.canonicalCatalog,invalidUTF8:Buffer.from([0xff]),rawCap:Buffer.alloc(131073,32),reordered:JSON.stringify({kind:v.catalog.kind,...v.catalog})}))
 test(`canonical catalog decoder denies ${name}`,()=>assert.throws(()=>decodeCatalog(Buffer.from(raw))));
test('zero active-selection revision, pre-exhaustion owner and empty regular bytes are valid',()=>{
 const c=catalog();c.ownerRevision='18446744073709551614';assert.doesNotThrow(()=>encodeCatalog(c));
 const i=integration();i.activeSelection.revision=max;i.ownerRevision=c.ownerRevision;i.resources[2].state='regular';i.resources[2].bytesBase64='';assert.deepEqual(decodeIntegration(encodeIntegration(i,c.platform),c.platform),i);
});
test('raw bytes preserve whitespace and binary values rather than normalized JSON or a previous hash',()=>{
 const c=catalog(),before=catalogDigest(c);c.builtinDocumentBase64=Buffer.concat([Buffer.from(c.builtinDocumentBase64,'base64'),Buffer.from('\n')]).toString('base64');assert.notEqual(catalogDigest(c),before);
 const i=integration();const absent=integrationDigest(i,c.platform);i.resources[2].state='regular';i.resources[2].bytesBase64='';assert.notEqual(integrationDigest(i,c.platform),absent);
 i.resources[2].bytesBase64=Buffer.from([0,255,1]).toString('base64');assert.notEqual(integrationDigest(i,c.platform),absent);
});
test('valid synthetic leaf-to-prepared joins use both actual codecs and independent expected inventory',()=>{
 const c=catalog(),i=integration(),p=prepared();assert.equal(validateLeafJoin(c,i,p,facts()),true);
});
for(const key of ['stockResolverQualified','inventoryQualified','fragmentsQualified','literalCommandQualified']) test(`unknown ${key} never qualifies`,()=>{
 for(const x of [undefined,false,'true',1,{}]){const f=facts();f.current.sourceFacts[key]=x;assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);}
});
for(const [key,values] of Object.entries({userDocumentState:[undefined,'present','invalid','unreadable'],requiresLateArgv:[undefined,true,'false'],requiresPromptAuthority:[undefined,true,'false']}))
 test(`explicit ${key} refusal`,()=>{for(const x of values){const f=facts();f.current.sourceFacts[key]=x;assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);}});
test('same labels with replaced configuration or resource owner deny',()=>{
 let f=facts();f.current.configurationOwner=Object.freeze({});assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);
 for(const id of integration().resources.map(x=>x.id)){f=facts();f.current.resourceOwners[id]=Object.freeze({});assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);}
});
for(const [name,target,path,fn] of [
 ['catalog epoch','catalog',['ownerRevision'],()=> '2'],['build','catalog',['sidecarBuildId'],()=> 'f'.repeat(64)],['stock bytes','catalog',['builtinDocumentBase64'],s=>Buffer.from('other').toString('base64')],['integration epoch','integration',['ownerRevision'],()=> '2'],['active selection','integration',['activeSelection','revision'],()=> '1'],['resource bytes','integration',['resources',0,'bytesBase64'],()=> 'YQ=='],['owned fragment','integration',['resources',2],x=>({...x,state:'regular',bytesBase64:'YQ=='})],['late argv','integration',['finalCommand','argv'],a=>[...a,'--session','new']],['source location','integration',['finalCommand','cwd'],()=> '/other']])
 test(`current ${name} invalidates old leaves`,()=>{const f=facts();setAt(f.current[target],path,fn);assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);});
test('weakening inventory on both arrays cannot bypass independent expected membership',()=>{
 const c=catalog(),i=integration(),p=prepared();i.inventory[0].presence='observed';i.resources[0].state='absent';i.resources[0].bytesBase64=null;p.configuration.catalogDigest=catalogDigest(c);p.configuration.integrationDigest=integrationDigest(i,c.platform);const f=facts(c,i,p);assert.equal(validateLeafJoin(c,i,p,f),false);
});

function bindDigests(c,i,p){i.catalogDigest=catalogDigest(c);p.configuration.catalogDigest=i.catalogDigest;p.configuration.integrationDigest=integrationDigest(i,c.platform);}
test('none adapter with an empty expected read set has no phantom owner-map key',()=>{
 const c=catalog(),i=integration(),p=prepared();c.requestedToolId='antigravity';i.selectedToolId='antigravity';i.activeSelection.toolIds=['antigravity'];i.adapter.binding='none';i.adapter.representativeToolId=null;i.inventory=[];i.resources=[];p.configuration.codingToolId='antigravity';p.proposal.proposedDraft.codingToolId='antigravity';bindDigests(c,i,p);
 const f=facts(c,i,p);f.expectedInventory=[];assert.equal(validateLeafJoin(c,i,p,f),true);
});
for(const [name,change] of [
 ['epoch',(c,i,p)=>{c.ownerRevision='2';i.ownerRevision='2';}],
 ['model',(c,i,p)=>c.modelId='different-model'],
 ['tool',(c,i,p)=>{c.requestedToolId='aider';i.selectedToolId='aider';i.activeSelection.toolIds=['aider','codex'];}],
 ['workspace',(c,i,p)=>i.workspaceGeneration='2'],
 ['platform',(c,i,p)=>c.platform='macos_arm64'],
 ['final program',(c,i,p)=>i.finalCommand.program='/other/tool'],
 ['final argv',(c,i,p)=>i.finalCommand.argv.push('--other')],
 ['final cwd',(c,i,p)=>i.finalCommand.cwd='/other'],
 ['resolved cwd',(c,i,p)=>c.resolvedCommand.cwd='/other'],
 ]) test(`valid but mismatched ${name} cannot join prepared configuration`,()=>{
 const c=catalog(),i=integration(),p=prepared();change(c,i,p);bindDigests(c,i,p);assert.equal(validateLeafJoin(c,i,p,facts(c,i,p)),false);
});
test('configuration owner is the exact parent owner, not just a mutually matching foreign token',()=>{
 const f=facts();f.captured.configurationOwner=f.current.configurationOwner=Object.freeze({});assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);
});
test('parent frozen draft/expiry/current-source checks remain active',()=>{
 for(const change of [f=>f.current.preparedCurrent.nowMs=31000,f=>f.captured.preparedEntry.draft={...prepared().proposal.proposedDraft,userInstruction:'different'},f=>f.current.preparedCurrent.source.runGeneration='2']){const f=facts();change(f);assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);}
});
test('unknown or retired active IDs and missing current facts do not qualify',()=>{
 const c=catalog(),i=integration(),p=prepared();i.activeSelection.toolIds=['codex','unknown-tool'];bindDigests(c,i,p);assert.equal(validateLeafJoin(c,i,p,facts(c,i,p)),false);
 i.activeSelection.toolIds=['codex','gemini'];assert.throws(()=>encodeIntegration(i,c.platform));
 const f=facts();delete f.current.resourceOwners['asset-source'];assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);
});
function leaves(x,path=[]){if(x===null || typeof x!=='object')return [path];if(Array.isArray(x))return [path,...x.flatMap((v,k)=>leaves(v,[...path,k]))];return Object.keys(x).flatMap(k=>leaves(x[k],[...path,k]));}
function mutated(x){if(typeof x==='string')return /^[0-9]+$/.test(x)?String(BigInt(x)+1n):/^[0-9a-f]{64}$/.test(x)?(x[0]==='a'?'b':'a')+x.slice(1):x+'x';if(typeof x==='number')return x+1;return Array.isArray(x)?['unexpected']:'unexpected';}
for(const [name,value,digest] of [['catalog',v.catalog,catalogDigest],['integration',v.integration,x=>integrationDigest(x,v.catalog.platform)]])
 for(const path of leaves(value))test(`commit ${name} field ${path.join('.')}`,()=>{
  const changed=copy(value);setAt(changed,path,mutated);let actual;try{actual=digest(changed);}catch{return;}assert.notEqual(actual,v[name+'Digest']);
 });
function objects(x,path=[]){if(x===null || typeof x!=='object')return [];if(Array.isArray(x))return x.flatMap((v,k)=>objects(v,[...path,k]));return [{path,keys:Object.keys(x)},...Object.keys(x).flatMap(k=>objects(x[k],[...path,k]))];}
for(const [name,value,encode] of [['catalog',v.catalog,encodeCatalog],['integration',v.integration,x=>encodeIntegration(x,v.catalog.platform)]])
 for(const {path,keys} of objects(value))test(`closed ${name} object ${path.join('.')||'root'}`,()=>{
  for(const key of [...keys,'extra']){const x=copy(value);let at=x;for(const k of path)at=at[k];if(key==='extra')at.extra=1;else delete at[key];assert.throws(()=>encode(x));}
 });
function fullCatalog(){
 const c=catalog(),raw=Buffer.from(c.builtinDocumentBase64,'base64');c.builtinDocumentBase64=Buffer.concat([raw,Buffer.alloc(65536-raw.length,32)]).toString('base64');c.resolvedCommand.argv=Array(64).fill('');
 let gap=131072-json(c).length;assert(gap>=0 && gap<=65536);for(let n=0;gap;n++){const size=Math.min(1024,gap);c.resolvedCommand.argv[n]='x'.repeat(size);gap-=size;}
 assert.equal(json(c).length,131072);return c;
}
function fullIntegration(c=catalog()){
 const i=integration();i.catalogDigest=catalogDigest(c);i.resources[0].bytesBase64=Buffer.alloc(262144,120).toString('base64');i.resources[1].bytesBase64='';
 const available=524288-json(i).length;assert(available>0);const rawSize=Math.floor(available/4)*3;assert(rawSize<=262144);i.resources[1].bytesBase64=Buffer.alloc(rawSize,120).toString('base64');
 const gap=524288-json(i).length;assert(gap>=0 && gap<4);i.resources[1].path+='x'.repeat(gap);assert.equal(json(i).length,524288);return i;
}
test('raw builtin 64KiB and resource 256KiB limits include every byte',()=>{
 const c=catalog();c.builtinDocumentBase64=Buffer.alloc(65536,32).toString('base64');assert.doesNotThrow(()=>encodeCatalog(c));c.builtinDocumentBase64=Buffer.alloc(65537,32).toString('base64');assert.throws(()=>encodeCatalog(c));
 const i=integration();i.resources[0].bytesBase64=Buffer.alloc(262144,32).toString('base64');assert.doesNotThrow(()=>encodeIntegration(i,v.catalog.platform));i.resources[0].bytesBase64=Buffer.alloc(262145,32).toString('base64');assert.throws(()=>encodeIntegration(i,v.catalog.platform));
});
test('standalone catalog128KiB and integration512KiB wrappers reject one extra byte',()=>{
 const c=fullCatalog();assert.equal(encodeCatalog(c).length,131072);const i=fullIntegration(c);assert.equal(encodeIntegration(i,c.platform).length,524288);
 assert.deepEqual(measureCapture(c,i),{catalogBytes:131072,integrationBytes:524288,totalBytes:655360});
 const tooC=copy(c);tooC.modelId+='x';assert.throws(()=>encodeCatalog(tooC));const tooI=copy(i);tooI.resources[1].path+='x';assert.throws(()=>encodeIntegration(tooI,c.platform));
 // These are valid wrapper domains, not qualified stock-resolution or #1922 config maxima.
 const p=prepared();p.configuration.argv=copy(c.resolvedCommand.argv);assert.equal(validateLeafJoin(c,i,p,facts(c,i,p)),false);
});
test('individual UTF-8 paths, model/argv and count boundaries',()=>{
 const c=catalog();c.modelId='m'.repeat(256);c.resolvedCommand.program='/'+'x'.repeat(4095);c.resolvedCommand.cwd='/'+'r'.repeat(4095);c.resolvedCommand.argv=Array(64).fill('');assert.doesNotThrow(()=>encodeCatalog(c));
 for(const path of [['modelId'],['resolvedCommand','program'],['resolvedCommand','cwd']]){const x=copy(c);setAt(x,path,s=>s+'x');assert.throws(()=>encodeCatalog(x));}
 c.resolvedCommand.argv.push('');assert.throws(()=>encodeCatalog(c));c.resolvedCommand.argv=['é'.repeat(512)];assert.doesNotThrow(()=>encodeCatalog(c));c.resolvedCommand.argv[0]+='x';assert.throws(()=>encodeCatalog(c));
 const i=integration();i.inventory=Array(65).fill(i.inventory[0]);assert.throws(()=>encodeIntegration(i,c.platform));
});
test('integration decoder rejects noncanonical/oversized input and missing platform',()=>{
 assert.throws(()=>decodeIntegration(Buffer.from(v.canonicalIntegration+'\n'),v.catalog.platform));assert.throws(()=>decodeIntegration(Buffer.alloc(524289,32),v.catalog.platform));assert.throws(()=>decodeIntegration(Buffer.from(v.canonicalIntegration)));
});

test('regular resource payload changes are included without relying on a state change',()=>{
 const i=integration(),digest=integrationDigest(i,v.catalog.platform);i.resources[0].bytesBase64='YQ==';assert.notEqual(integrationDigest(i,v.catalog.platform),digest);
});
for(const key of ['catalog','integration'])test(`captured ${key} is not replaceable while current bytes remain equal`,()=>{
 const f=facts();if(key==='catalog')f.captured.catalog.sidecarBuildId='f'.repeat(64);else f.captured.integration.activeSelection.revision='1';
 assert.equal(validateLeafJoin(catalog(),integration(),prepared(),f),false);
});
test('catalog-to-integration and configuration digest relations cannot be rebound independently',()=>{
 const c=catalog(),i=integration(),p=prepared();i.catalogDigest='f'.repeat(64);p.configuration.integrationDigest=integrationDigest(i,c.platform);assert.equal(validateLeafJoin(c,i,p,facts(c,i,p)),false);
 for(const key of ['catalogDigest','integrationDigest']){const q=prepared();q.configuration[key]='f'.repeat(64);assert.equal(validateLeafJoin(catalog(),integration(),q,facts(catalog(),integration(),q)),false);}
});
