import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {buildActionsOverview} from '../src/actionsOverview.ts';
import {unverifiedRecommendation} from './fixtures/unverifiedAdvice.ts';

const require=createRequire(import.meta.url);
const result=await build({entryPoints:[new URL('../src/components/ActionsOverview.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const module={exports:{}};
new Function('require','module','exports',result.outputFiles[0].text)(require,module,module.exports);
const Actions=module.exports.default;
const render=(model,inspected)=>renderToStaticMarkup(createElement(Actions,{model,inspected,focusRequest:0,onInspectAction:()=>{},onFocusHandled:()=>{},onOpenSession:()=>{},onOpenRecommendation:()=>{}}));

test('Actions warns before unverified row and detail prose, and hides stale copied prose',()=>{
 const item=unverifiedRecommendation();item.title='Generated title';
 const model=buildActionsOverview([],'ready',[item],'ready');
 const html=render(model,model.rows[0]);
 const warning='Checks remain unverified. Run the proposed validation before relying on it.';
 assert.ok(html.indexOf(warning)>=0&&html.indexOf(warning)<html.indexOf(item.title));
 const detail=html.slice(html.indexOf('data-actions-detail'));
 assert.ok(detail.indexOf(warning)>=0&&detail.indexOf(warning)<detail.indexOf('Tests passed.'));
 const stale=render(buildActionsOverview([],'ready',[],'ready'),model.rows[0]);
 assert.match(stale,/unavailable or changed/);assert.doesNotMatch(stale,/Generated title|Tests passed/);
});
