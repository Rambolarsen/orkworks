import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
const require=createRequire(import.meta.url);
const result=await build({entryPoints:[new URL('../src/components/BranchDocumentSteps.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const module={exports:{}};new Function('require','module','exports',result.outputFiles[0].text)(require,module,module.exports);
const Steps=module.exports.default;
const data={branch:'task',sharedSessions:2,steps:[{kind:'design',available:true,title:'Design the work',fileName:'design.md',source:'git_history',pinned:false},{kind:'implementation',available:true,title:'Build the work',fileName:'plan.md',source:'agent_reported',pinned:true}]};
const render=(documents=data)=>renderToStaticMarkup(createElement(Steps,{sessionId:'session',documents,onOpen(){},onUseLatest:async()=>{}}));
test('displays both ordered documents, sharing context and explicit pin without approval gates',()=>{
 const html=render();assert.ok(html.indexOf('Design / specification')<html.indexOf('Implementation plan'));
 assert.match(html,/Design the work/);assert.match(html,/Build the work/);assert.match(html,/Shared by 2 sessions/);
 assert.match(html,/Found in Git history/);assert.match(html,/Pinned/);assert.match(html,/Use latest/);
 assert.doesNotMatch(html,/Approve|Approved|Complete step|Continue execution/);
});
test('keeps a missing implementation step visible and disabled without hiding the design',()=>{
 const html=render({...data,steps:[data.steps[0],{kind:'implementation',available:false,pinned:false}]});
 assert.match(html,/Design the work/);assert.match(html,/Implementation plan/);assert.match(html,/No document found yet/);
 assert.match(html,/<button[^>]*disabled[^>]*data-document-kind="implementation"|<button[^>]*data-document-kind="implementation"[^>]*disabled/);
});
test('missing pinned documents stay visibly unavailable and can return to latest',()=>{
 const html=render({...data,notice:'Branch history unavailable',steps:[{...data.steps[1],available:false}]});
 assert.match(html,/Selected document unavailable/);assert.match(html,/Use latest/);assert.match(html,/Branch history unavailable/);
});
