import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import * as React from 'react';
const require=createRequire(import.meta.url);
const compiled=await build({entryPoints:[new URL('../src/components/ReadinessPanel.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const ids=['context-discovery','progressive-disclosure','instruction-scoping','architecture-awareness','domain-knowledge','environment-reproducibility','tool-accessibility','task-decomposition','verification','observability','change-safety','feedback-loops','durable-memory','human-oversight'];
const snapshot=()=>({id:'one',startedAt:'2026-10-25T00:00:00.000Z',completedAt:'2026-10-25T00:00:00.000Z',redacted:false,data:{workspacePath:'/repo',inputFingerprint:'a'.repeat(64),comparisonFingerprint:'b'.repeat(64),rubricDigest:'c'.repeat(64),summary:'Observed guidance',concepts:ids.map(conceptId=>({conceptId,maturity:2,confidence:'medium',repositoryFactHashes:['d'.repeat(64)],rubricPageId:'playbooks/rubric.md',rationale:'Documented guidance',limits:[],nextTransition:'Workflow evidence',unassessedReason:null})),repositoryEvidence:[],knowledgeEvidence:[],coverage:14,rawTotal:28,overallScore:50,git:{headOid:null,dirty:null},inferenceIdentity:{},knowledgeIdentity:{}}});
const status=(snapshots=[])=>({workspacePath:'/repo',run:null,snapshots,freshness:'current',retentionLimit:100,reasonCode:null});
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{promise,resolve,reject};};
function nodes(tree){if(Array.isArray(tree))return tree.flatMap(nodes);if(!tree||typeof tree!=='object')return[];return[tree,...nodes(tree.props?.children)];}
async function fixture(t,initial,props={}){
 const queue=[Promise.resolve(initial)];let tick;
 const prior=globalThis.window;globalThis.window={setInterval:f=>{tick=f;return 1},clearInterval:()=>{},orkworks:{getReadinessStatus:()=>queue.shift(),requestReadinessAssessment:async()=>({outcome:'unavailable',runId:null,reasonCode:null})}};t.after(()=>globalThis.window=prior);
 let stateIndex=0,effectIndex=0;const values=[],deps=[],effects=[];
 const hooks={...React,useRef:initial=>hooks.useState({current:initial})[0],useState(initial){const i=stateIndex++;if(!(i in values))values[i]=initial;return[values[i],next=>values[i]=typeof next==='function'?next(values[i]):next];},useEffect(callback,next){const i=effectIndex++;if(!deps[i]||next.some((x,j)=>x!==deps[i][j]))effects.push(callback);deps[i]=next;}};
 const module={exports:{}};new Function('require','module','exports',compiled.outputFiles[0].text)(id=>id==='react'?hooks:require(id),module,module.exports);
 const onBusyChange=()=>{};const render=()=>{stateIndex=0;effectIndex=0;return module.exports.default({workspacePath:'/repo',disabled:false,onBusyChange,...props});};
 const settle=async()=>{while(effects.length)effects.shift()();await new Promise(setImmediate);};render();await settle();
 return{render,settle,queue,tick:()=>tick(),summary:()=>nodes(render()).find(node=>node.props?.snapshot)?.props.snapshot};
}
test('older responses cannot restore a score after a newer redacted history',async t=>{
 const view=await fixture(t,status());const old=deferred(),newer=deferred();view.queue.push(old.promise,newer.promise);view.tick();view.tick();
 newer.resolve(status([{...snapshot(),redacted:true,data:null}]));await view.settle();assert.equal(view.summary().redacted,true);
 old.resolve(status([snapshot()]));await view.settle();assert.equal(view.summary().redacted,true);
});
test('an unavailable history read clears retained source-derived presentation',async t=>{
 const view=await fixture(t,status([snapshot()]));assert.equal(view.summary().data.overallScore,50);const error=deferred();view.queue.push(error.promise);view.tick();error.reject(Error('access changed'));await view.settle();assert.equal(view.summary(),undefined);
});

function displayedText(tree){return nodes(tree).flatMap(node=>{const children=node.props?.children;return(Array.isArray(children)?children:[children]).filter(x=>typeof x==='string');}).join(' ');}
test('failed readiness shows safe stage and cause while retaining earlier score',async t=>{
 const initial=status([snapshot()]);initial.run={id:'failed',state:'failed',phase:null,startedAt:'2026-10-25T00:00:00Z',completedAt:'2026-10-25T00:01:00Z',errorCode:'scoring_provider_timeout'};
 const view=await fixture(t,initial);const text=displayedText(view.render());assert.match(text,/scoring/i);assert.match(text,/timed out/i);assert.equal(view.summary().data.overallScore,50);
});
test('unknown readiness error codes never become displayed provider text',async t=>{
 const initial=status();initial.run={id:'failed',state:'failed',phase:null,startedAt:'2026-10-25T00:00:00Z',completedAt:'2026-10-25T00:01:00Z',errorCode:'private-account-secret'};
 const view=await fixture(t,initial);assert.doesNotMatch(displayedText(view.render()),/private-account-secret/);
});

for(const [code,expected] of [['evidence_inputs_changed',/inputs changed/i],['publication_metadata_unavailable',/history could not be read or saved/i],['assessment_failed',/try assess readiness again/i]]){
 test(`readiness describes ${code} without exposing the code`,async t=>{
  const initial=status();initial.run={id:'failed',state:code==='evidence_inputs_changed'?'interrupted':'failed',phase:null,startedAt:'2026-10-25T00:00:00Z',completedAt:'2026-10-25T00:01:00Z',errorCode:code};
  const view=await fixture(t,initial);const text=displayedText(view.render());assert.match(text,expected);assert.doesNotMatch(text,new RegExp(code));
 });
}


test('combined assessment is primary and scoring alone remains available',async t=>{
 const view=await fixture(t,status());
 const buttons=nodes(view.render()).filter(node=>node.type==='button');
 assert.equal(buttons[0].props.children,'Assess and suggest');
 assert.ok(buttons.some(button=>button.props.children==='Assess readiness'));
 assert.match(displayedText(view.render()),/four model calls/i);
});


test('combined action invokes only its privileged method and reports the scheduled run',async t=>{
 const view=await fixture(t,status());let combined=0,score=0;
 window.orkworks.requestReadinessSuggestion=async()=>{combined++;return{outcome:'scheduled',runId:'combined',reasonCode:null};};
 window.orkworks.requestReadinessAssessment=async()=>{score++;return{outcome:'scheduled',runId:'score',reasonCode:null};};
 view.queue.push(Promise.resolve(status()));
 const button=nodes(view.render()).find(node=>node.type==='button'&&node.props.children==='Assess and suggest');
 button.props.onClick();await view.settle();
 assert.equal(combined,1);assert.equal(score,0);
 assert.match(displayedText(view.render()),/Assessment and suggestion requested/);
});
test('combined no-proposal reason comes only from its linked current assessment',async t=>{
 const initial=status([snapshot()]);initial.suggestionRun={id:'combo',state:'succeeded',phase:'finished',readinessRunId:'one',assessmentId:'assessment-one',recommendationId:null,reasonCode:'no_proposal',noProposalReason:'The supplied guidance already covers this workflow.'};
 const view=await fixture(t,initial);assert.match(displayedText(view.render()),/already covers this workflow/);
});
test('suggestion-stage failure retains the saved score and describes the failure',async t=>{
 const initial=status([snapshot()]);initial.suggestionRun={id:'combo',state:'failed',phase:'finished',readinessRunId:'one',assessmentId:'assessment-one',recommendationId:null,reasonCode:'suggestion_failed',noProposalReason:null};
 const view=await fixture(t,initial);assert.equal(view.summary().data.overallScore,50);assert.match(displayedText(view.render()),/saved score is retained/i);
});


test('history selectors preserve zero scores, redaction labels and baseline reset', async t => {
 const recent = snapshot(), older = snapshot();
 recent.id = 'recent'; older.id = 'older';
 Object.assign(older.data, { overallScore: 0, rawTotal: 0 });
 older.data.concepts.forEach(row => row.maturity = 0);
 const redacted = { ...snapshot(), id: 'redacted', redacted: true, data: null };
 const view = await fixture(t, status([recent, older, redacted]));
 const selects = () => nodes(view.render()).filter(node => node.type === 'select');
 const summary = () => nodes(view.render()).find(node => node.props?.snapshot).props;
 assert.equal(summary().baseline.id, 'older');
 for (const select of selects()) {
  const options = nodes(select).filter(node => node.type === 'option');
  assert.match(options.find(option => option.props.value === 'older').props.children.join(''), /0\/100/);
  assert.match(options.find(option => option.props.value === 'redacted').props.children.join(''), /Redacted/);
 }
 selects()[1].props.onChange({ target: { value: '' } });
 assert.equal(summary().baseline, undefined);
 selects()[0].props.onChange({ target: { value: 'older' } });
 assert.equal(summary().snapshot.id, 'older');
 assert.equal(summary().baseline, undefined);
 selects()[0].props.onChange({ target: { value: 'recent' } });
 assert.equal(summary().baseline.id, 'older');
});

test('standalone assessment remains separate from the combined request', async t => {
 const view = await fixture(t, status());
 let score = 0;
 window.orkworks.requestReadinessAssessment = async () => { score++; return { outcome: 'scheduled' }; };
 window.orkworks.requestReadinessSuggestion = async () => { assert.fail('Combined action must not be invoked'); };
 view.queue.push(Promise.resolve(status()));
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'Assess readiness').props.onClick();
 await view.settle();
 assert.equal(score, 1);
 assert.match(displayedText(view.render()), /Readiness assessment requested/);
});

test('a completed suggestion is surfaced once across status refreshes', async t => {
 const initial = status();
 initial.suggestionRun = { id: 'combo', state: 'succeeded', phase: 'finished', readinessRunId: 'one', assessmentId: 'assessment-one', recommendationId: 'recommendation-one', reasonCode: null, noProposalReason: null };
 const surfaced = [];
 const view = await fixture(t, initial, { onSuggestion: id => surfaced.push(id) });
 view.render(); await view.settle();
 assert.deepEqual(surfaced, ['recommendation-one']);
 view.queue.push(Promise.resolve(structuredClone(initial))); view.tick(); await view.settle();
 view.render(); await view.settle();
 assert.deepEqual(surfaced, ['recommendation-one']);
});


test('each source click reopens its saved assessment without polling overriding manual selection', async t => {
 const source = snapshot(), other = snapshot(); other.id = 'other';
 const initial = status([other, source]);
 let scrolls = 0;
 const prior = globalThis.document;
 globalThis.document = { getElementById: () => ({ scrollIntoView: () => scrolls++ }) };
 t.after(() => globalThis.document = prior);
 const props = { requestedSnapshot: { snapshotId: source.id } };
 const view = await fixture(t, initial, props);
 view.render(); await view.settle();
 assert.equal(view.summary().id, source.id);
 const chooseOther = () => nodes(view.render()).find(node => node.type === 'select').props.onChange({ target: { value: other.id } });
 chooseOther(); assert.equal(view.summary().id, other.id);
 props.requestedSnapshot = { snapshotId: source.id };
 view.render(); await view.settle();
 assert.equal(view.summary().id, source.id);
 assert.equal(scrolls, 2);
 chooseOther();
 view.queue.push(Promise.resolve(structuredClone(initial))); view.tick(); await view.settle();
 view.render(); await view.settle();
 assert.equal(view.summary().id, other.id);
 assert.equal(scrolls, 2);
});
