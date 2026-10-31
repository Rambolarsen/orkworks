import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
const require=createRequire(import.meta.url);
const result=await build({entryPoints:[new URL('../src/components/ReadinessSummary.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const module={exports:{}};new Function('require','module','exports',result.outputFiles[0].text)(require,module,module.exports);const Summary=module.exports.default;
const row={conceptId:'verification',maturity:3,confidence:'medium',repositoryFactHashes:['a'.repeat(64)],rubricPageId:'playbooks/rubric.md',rationale:'Documented checks',limits:['Runtime behavior remains unverified'],nextTransition:'Retained failure evidence',unassessedReason:null};
const snapshot=(id,score)=>({id,startedAt:'2026-10-25T00:00:00.000Z',completedAt:'2026-10-25T00:00:00.000Z',redacted:false,data:{coverage:14,rawTotal:42,overallScore:score,comparisonFingerprint:'same',summary:'Observed readiness',concepts:[row],git:{headOid:'abc123',dirty:true},repositoryEvidence:[{path:'README.md',sha256:'a'.repeat(64),excerpt:'A saved excerpt',observedAt:'now'}],knowledgeEvidence:[],inferenceIdentity:{selection:{provider:'codex',model:'selected'}},knowledgeIdentity:{version:'brain-2'}}});
test('summary shows overall score, comparable delta, individual evidence and historical provenance',()=>{
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:snapshot('new',75),baseline:snapshot('old',50),freshness:'changed'}));
 assert.match(html,/75/);assert.match(html,/25/);assert.match(html,/Verification/);assert.match(html,/3.*4/);assert.match(html,/medium confidence/);assert.match(html,/A saved excerpt/);assert.match(html,/Changes since assessment/);assert.match(html,/uncommitted changes/);
});
test('partial and redacted snapshots cannot display an overall number',()=>{
 const partial=snapshot('partial',null);partial.data.coverage=13;partial.data.rawTotal=null;
 assert.match(renderToStaticMarkup(createElement(Summary,{snapshot:partial,freshness:'current'})),/13.*14 areas assessed/);
 const redacted={...partial,redacted:true,data:null};const html=renderToStaticMarkup(createElement(Summary,{snapshot:redacted,freshness:'redacted'}));assert.match(html,/redacted/);assert.doesNotMatch(html,/Documented checks|75/);
});

test('identical assessed inputs are labeled as model variability rather than repository progress',()=>{
 const current=snapshot('new',75),baseline=snapshot('old',50);current.data.inputFingerprint='a'.repeat(64);baseline.data.inputFingerprint='a'.repeat(64);
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:current,baseline,freshness:'current'}));assert.match(html,/Same assessed inputs/);assert.match(html,/model variability/);
});


test('next improvement needs and their limits are visible outside collapsed details',()=>{
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:snapshot('new',75),freshness:'current'}));
 const visible=html.replace(/<details[\s\S]*?<\/details>/g,'');
 assert.match(visible,/What to improve next/);
 assert.match(visible,/Retained failure evidence/);
 assert.match(visible,/Runtime behavior remains unverified/);
});
test('Luna medium qualification failure is visible beside a historical score',()=>{
 const saved=snapshot('saved',46);saved.data.inferenceIdentity.selection={provider:'codex',model:'gpt-6-luna',reasoningEffort:'medium'};
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:saved,freshness:'changed'}));
 assert.match(html,/failed.*readiness qualification/i);assert.match(html,/46/);
});


test('unassessed priority areas explain the missing evidence beside the score',()=>{
 const saved=snapshot('partial',null);saved.data.concepts=[{...row,maturity:null,confidence:null,unassessedReason:'The supplied excerpts do not establish this capability.'}];saved.data.coverage=0;saved.data.rawTotal=null;
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:saved,freshness:'current'}));
 const visible=html.replace(/<details[\s\S]*?<\/details>/g,'');
 assert.match(visible,/Not assessed/);assert.match(visible,/do not establish this capability/);
});


test('individual score details show only cited evidence and suppress incompatible deltas', () => {
 const current = snapshot('new', 75), baseline = snapshot('old', 50);
 current.data.repositoryEvidence = [
  { factId: 'a'.repeat(64), path: 'README.md', excerpt: 'Cited excerpt', observedAt: 'now' },
  { factId: 'b'.repeat(64), path: 'other.md', excerpt: 'Unrelated excerpt', observedAt: 'now' },
 ];
 baseline.data.comparisonFingerprint = 'different';
 baseline.data.concepts = [{ ...row, maturity: 1, confidence: 'low' }];
 const html = renderToStaticMarkup(createElement(Summary, { snapshot: current, baseline, freshness: 'current' }));
 const area = html.match(/<details class="readiness-area">([\s\S]*?)<\/details>/)[1];
 assert.match(area, /Cited excerpt/);
 assert.doesNotMatch(area, /Unrelated excerpt|<span>\+2<\/span>/);
 assert.match(area, /previously low/);
 assert.match(html, /Assessment conditions changed/);
});


test('budget-limited evidence is visible beside the saved score',()=>{
 const value=snapshot('limited',75);
 value.data.inputManifest={collector:{collectorVersion:4,preferredFileIds:[0,1],requestedFileIds:[1],selectionOmissions:[{id:0,reason:'evidence_limit'}]}};
 const html=renderToStaticMarkup(createElement(Summary,{snapshot:value,freshness:'current'}));
 assert.match(html,/1 evidence selection did not fit/);
 assert.match(html,/Unread evidence remains unknown/);
});
