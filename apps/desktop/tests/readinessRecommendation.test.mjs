import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
const require=createRequire(import.meta.url);
const result=await build({entryPoints:[new URL('../src/components/ReadinessRecommendation.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const module={exports:{}};new Function('require','module','exports',result.outputFiles[0].text)(require,module,module.exports);const Link=module.exports.default;
const id='e1426ee7-55aa-4e7b-8811-d20b8be42473';
const reference={snapshotId:id,rubricDigest:'a'.repeat(64),conceptIds:['verification']};
const recommendation={workspaceId:'/repo',assessment:{readinessReference:reference}};
const source={id,redacted:false,completedAt:'2026-10-30T00:00:00Z',data:{workspacePath:'/repo',rubricDigest:reference.rubricDigest}};
const render=(snapshots,rec=recommendation)=>renderToStaticMarkup(createElement(Link,{recommendation:rec,snapshots,onOpen:()=>{}}));
test('linked recommendations name the capability and offer the exact retained source',()=>{
 const html=render([source]);assert.match(html,/Verification/);assert.match(html,/View saved assessment/);assert.doesNotMatch(html,/score gain|points/);
});
test('evicted, unrelated or redacted sources cannot be opened',()=>{
 for(const sources of [[],[{...source,id:'other'}],[{...source,redacted:true}], [{...source,data:{...source.data,workspacePath:'/other'}}]]) {
  const html=render(sources);assert.match(html,/Saved assessment unavailable/);assert.doesNotMatch(html,/<button/);
 }
});
test('redacted or malformed capability provenance is never displayed',()=>{
 assert.equal(render([source],{...recommendation,assessment:{...recommendation.assessment,accessRedacted:true}}),'');
 assert.equal(render([source],{...recommendation,assessment:{readinessReference:{...reference,conceptIds:['invented']}}}),'');
});
