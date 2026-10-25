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
