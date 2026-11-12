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
function nodes(tree){if(Array.isArray(tree))return tree.flatMap(nodes);if(!tree||typeof tree!=='object')return[];return[tree,...nodes(typeof tree.type==='function'?tree.type(tree.props):tree.props?.children)];}
async function fixture(t,initial,props={}){
 const queue=[Promise.resolve(initial)];let tick;
 let improvementStatus={workspacePath:'/repo',available:false,reasonCode:'not_qualified',run:null};
 const prior=globalThis.window;globalThis.window={setInterval:f=>{tick=f;return 1},clearInterval:()=>{},orkworks:{getReadinessStatus:()=>queue.shift(),getImprovementStatus:async()=>improvementStatus,requestImprovement:async()=>({outcome:'unavailable',runId:null,reasonCode:'not_qualified'}),cancelImprovement:async()=>({outcome:'cancel_requested',runId:'improvement',reasonCode:null}),requestReadinessAssessment:async()=>({outcome:'unavailable',runId:null,reasonCode:null})}};t.after(()=>globalThis.window=prior);
 let stateIndex=0,effectIndex=0;const values=[],deps=[],effects=[];
 const hooks={...React,useRef:initial=>hooks.useState({current:initial})[0],useState(initial){const i=stateIndex++;if(!(i in values))values[i]=initial;return[values[i],next=>values[i]=typeof next==='function'?next(values[i]):next];},useEffect(callback,next){const i=effectIndex++;if(!deps[i]||next.some((x,j)=>x!==deps[i][j]))effects.push(callback);deps[i]=next;}};
 const module={exports:{}};new Function('require','module','exports',compiled.outputFiles[0].text)(id=>id==='react'?hooks:require(id),module,module.exports);
 const onBusyChange=()=>{};const render=(nextProps={})=>{stateIndex=0;effectIndex=0;return module.exports.default({workspacePath:'/repo',disabled:false,onBusyChange,...props,...nextProps});};
 const settle=async()=>{while(effects.length)effects.shift()();await new Promise(setImmediate);};render();await settle();
 return{render,settle,queue,tick:()=>tick(),summary:()=>nodes(render()).find(node=>node.props?.snapshot)?.props.snapshot,setImprovementStatus:value=>improvementStatus=value};
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

for(const [code,expected] of [['evidence_inputs_changed',/inputs changed/i],['publication_metadata_unavailable',/history could not be read or saved/i],['assessment_failed',/try check agent readiness again/i]]){
 test(`readiness describes ${code} without exposing the code`,async t=>{
  const initial=status();initial.run={id:'failed',state:code==='evidence_inputs_changed'?'interrupted':'failed',phase:null,startedAt:'2026-10-25T00:00:00Z',completedAt:'2026-10-25T00:01:00Z',errorCode:code};
  const view=await fixture(t,initial);const text=displayedText(view.render());assert.match(text,expected);assert.doesNotMatch(text,new RegExp(code));
 });
}


test('combined assessment is primary and scoring alone remains available',async t=>{
 const view=await fixture(t,status());
 const buttons=nodes(view.render()).filter(node=>node.type==='button');
 assert.equal(buttons[0].props.children,'Find next improvement');
 assert.ok(buttons.some(button=>button.props.children==='Check agent readiness'));
 assert.match(buttons[0].props.title,/four model calls/i);
});

test('qualified improvement alone becomes the primary action and discloses six calls',async t=>{
 const view=await fixture(t,status());
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:null});
 view.tick();await view.settle();
 const button=nodes(view.render()).find(node=>node.type==='button'&&node.props.children==='Find next improvement');
 assert.match(button.props.title,/six model calls/i);
 let starts=0;window.orkworks.requestImprovement=async()=>{starts++;return{outcome:'scheduled',runId:'run-1',reasonCode:null};};
 button.props.onClick();await view.settle();assert.equal(starts,1);
});

test('an active improvement can be cancelled without calling the legacy assessment',async t=>{
 const view=await fixture(t,status());
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'improvement',state:'running',phase:'assessing',startedAt:'2026-11-12T10:00:00Z',completedAt:null,errorCode:null,report:null}});
 view.tick();await view.settle();
 let cancelled=0;window.orkworks.cancelImprovement=async id=>{cancelled++;assert.equal(id,'improvement');return{outcome:'cancel_requested',runId:id,reasonCode:null};};
 const button=nodes(view.render()).find(node=>node.type==='button'&&node.props.children==='Cancel investigation');
 assert.ok(button);button.props.onClick();await view.settle();assert.equal(cancelled,1);
});


test('combined action invokes only its privileged method and reports the scheduled run',async t=>{
 const view=await fixture(t,status());let combined=0,score=0;
 window.orkworks.requestReadinessSuggestion=async()=>{combined++;return{outcome:'scheduled',runId:'combined',reasonCode:null};};
 window.orkworks.requestReadinessAssessment=async()=>{score++;return{outcome:'scheduled',runId:'score',reasonCode:null};};
 view.queue.push(Promise.resolve(status()));
 const button=nodes(view.render()).find(node=>node.type==='button'&&node.props.children==='Find next improvement');
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
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'Check agent readiness').props.onClick();
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

test('the compact overview opens its saved report and comparison history without a new assessment', async t => {
 const recent = snapshot(), older = snapshot(); older.id = 'older';
 const view = await fixture(t, status([recent, older]));
 window.orkworks.requestReadinessAssessment = async () => assert.fail('Viewing a report must not assess');
 window.orkworks.requestReadinessSuggestion = async () => assert.fail('Viewing history must not seek a suggestion');
 const report = () => nodes(view.render()).find(node => node.props?.id === 'readiness-report');
 const history = () => nodes(view.render()).find(node => node.props?.className === 'readiness-history-controls');
 assert.equal(report()?.props.hidden, true);
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'View report').props.onClick();
 assert.equal(report().props.hidden, false);
 assert.equal(history().props.hidden, true);
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'History').props.onClick();
 assert.equal(history().props.hidden, false);
 const select = nodes(history()).find(node => node.type === 'select');
 select.props.onChange({ target: { value: 'older' } });
 assert.equal(view.summary().id, 'older');
 assert.equal(view.summary().data.overallScore, 50);
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'View report').props.onClick();
 assert.equal(report().props.hidden, true);
 nodes(view.render()).find(node => node.type === 'button' && node.props.children === 'History').props.onClick();
 assert.equal(report().props.hidden, false);
 assert.equal(history().props.hidden, false, 'History opens on the first click even after the report was collapsed');
});

test('the owner can place readiness actions in its toolbar without changing their privileged routing', async t => {
 const view = await fixture(t, status(), { renderActions: actions => React.createElement('nav', { 'aria-label': 'Assessments' }, actions.findImprovement, actions.checkReadiness) });
 const toolbar = nodes(view.render()).find(node => node.props?.['aria-label'] === 'Assessments');
 assert.ok(toolbar, 'The toolbar must receive the actual readiness controls');
 let combined = 0, score = 0;
 window.orkworks.requestReadinessSuggestion = async () => { combined++; return { outcome: 'scheduled' }; };
 window.orkworks.requestReadinessAssessment = async () => { score++; return { outcome: 'scheduled' }; };
 view.queue.push(Promise.resolve(status()));
 nodes(toolbar).find(node => node.type === 'button' && node.props.children === 'Find next improvement').props.onClick();
 await view.settle();
 assert.equal(combined, 1);
 assert.equal(score, 0);
});

test('disabling readiness during a pending request does not leave controls stuck after cancellation', async t => {
 const props = { enabled: true };
 const view = await fixture(t, status(), props);
 const pending = deferred();
 window.orkworks.requestReadinessSuggestion = () => pending.promise;
 const primary = () => nodes(view.render()).find(node => node.type === 'button' && node.props.className === 'recommendation-primary');
 primary().props.onClick();
 assert.equal(primary().props.disabled, true);
 props.enabled = false; view.render(); await view.settle();
 props.enabled = true; view.queue.push(Promise.resolve(status())); view.render(); await view.settle();
 assert.equal(primary().props.disabled, false, 'Cancelling workspace selection must reenable readiness');
 pending.resolve({ outcome: 'scheduled', runId: 'old-workspace-attempt' }); await view.settle();
 assert.doesNotMatch(displayedText(view.render()), /Assessment and suggestion requested/);
 assert.equal(primary().props.disabled, false);
});


test('an already-running response does not remain after a successful idle refresh', async t => {
 const view = await fixture(t, status([snapshot()]));
 window.orkworks.requestReadinessSuggestion = async () => ({ outcome: 'already_running' });
 view.queue.push(Promise.resolve(status([snapshot()])));
 nodes(view.render()).find(n => n.type === 'button' && n.props.children === 'Find next improvement').props.onClick();
 await view.settle(); view.render(); await view.settle();
 assert.doesNotMatch(displayedText(view.render()), /Another Taskmaster analysis is active/);
});

test('live external activity is separate from a timestamped previous failure and clears when idle', async t => {
 const props = { analysisActive: true };
 const initial = status([snapshot()]);
 initial.run = { id: 'failed', state: 'failed', phase: null, startedAt: '2026-10-26T00:00:00Z', completedAt: '2026-10-26T00:01:00Z', errorCode: 'evidence_evidence_limit' };
 const view = await fixture(t, initial, props);
 const live = nodes(view.render()).filter(n => n.props?.role === 'status');
 assert.ok(live.some(n => /Another Taskmaster analysis is active/.test(displayedText(n))));
 assert.ok(live.every(n => !/exceeded|failed/.test(displayedText(n))), 'Historical errors must not be announced as current activity');
 const notice = nodes(view.render()).find(n => n.props?.className === 'readiness-failure');
 assert.ok(notice, 'A previous failure gets a separate notice');
 assert.ok(nodes(notice).some(n => n.type === 'time' && n.props.dateTime === initial.run.completedAt));
 assert.match(displayedText(notice), /saved assessment is retained/i);
 assert.ok(nodes(notice).some(n => n.type === 'details'), 'Technical failure detail is expandable');
 assert.doesNotMatch(displayedText(notice), /check taskmaster settings/i);
 props.analysisActive = false;
 view.render(); await view.settle();
 assert.doesNotMatch(displayedText(view.render()), /Another Taskmaster analysis is active/);
});

test('a request confirmation clears when polling observes its run', async t => {
 const view = await fixture(t, status());
 window.orkworks.requestReadinessAssessment = async () => ({ outcome: 'scheduled' });
 view.queue.push(Promise.resolve(status()));
 nodes(view.render()).find(n => n.type === 'button' && n.props.children === 'Check agent readiness').props.onClick();
 await view.settle(); view.render(); await view.settle();
 assert.match(displayedText(view.render()), /Readiness assessment requested/);
 const active = status();
 active.run = { id: 'new', state: 'running', phase: 'reading_evidence', startedAt: '2026-11-02T00:00:00Z', completedAt: null, errorCode: null };
 view.queue.push(Promise.resolve(active)); view.tick(); await view.settle(); view.render(); await view.settle();
 assert.doesNotMatch(displayedText(view.render()), /Readiness assessment requested/);
 assert.match(displayedText(view.render()), /Reading evidence/);
});

test('a failed combined score shows the linked cause once without generic settings advice', async t => {
 const initial = status([snapshot()]);
 initial.run = { id: 'score-failed', state: 'failed', phase: null, startedAt: '2026-10-26T00:00:00Z', completedAt: '2026-10-26T00:01:00Z', errorCode: 'evidence_evidence_limit' };
 initial.suggestionRun = { id: 'combined-failed', state: 'failed', phase: 'finished', readinessRunId: 'score-failed', assessmentId: null, recommendationId: null, reasonCode: 'scoring_failed' };
 const view = await fixture(t, initial);
 assert.equal(nodes(view.render()).filter(n => n.props?.className === 'readiness-failure').length, 1);
 assert.doesNotMatch(displayedText(view.render()), /check Taskmaster settings/i);
});

test('saved score provenance is visible for unknown currentness and redacted scores stay hidden', async t => {
 const initial = status([snapshot()]); initial.freshness = 'unknown';
 const view = await fixture(t, initial);
 const text = displayedText(view.render());
 assert.match(text, /Saved/); assert.match(text, /Currentness unknown/);
 const badge = nodes(view.render()).find(n => n.props?.className?.includes('readiness-freshness'));
 assert.ok(badge);
 const redacted = status([{ ...snapshot(), redacted: true, data: null }]); redacted.freshness = 'redacted';
 view.queue.push(Promise.resolve(redacted)); view.tick(); await view.settle();
 assert.ok(!nodes(view.render()).some(n => n.type === 'strong' && n.props.children === 50));
 assert.doesNotMatch(displayedText(view.render()), /Inputs unchanged/);
});


for (const [errorCode, guidance, irrelevant] of [
 ['evidence_evidence_limit', /evidence access and exclusions/, /updated app build|model provider/],
 ['scoring_provider_authentication', /Sign in to the model provider/, /updated app build/],
 ['scoring_provider_timeout', /provider is available/, /Sign in/],
 ['publication_metadata_unavailable', /local storage access/, /Taskmaster settings/],
 ['evidence_evidence_unavailable', /repository access/, /updated app build/],
 ['scoring_provider_model', /model and coding tool configuration/, /local storage access/],
 ['scoring_prompt_limit', /context access and exclusions/, /Sign in/],
]) test(`failure recovery for ${errorCode} addresses the cause`, async t => {
 const initial = status();
 initial.run = { id: 'failed', state: 'failed', phase: null, startedAt: '2026-10-26T00:00:00Z', completedAt: '2026-10-26T00:01:00Z', errorCode };
 const view = await fixture(t, initial);
 const notice = nodes(view.render()).find(n => n.props?.className === 'readiness-failure');
 const recovery = nodes(notice).find(n => n.type === 'p' && !n.props.className);
 assert.match(displayedText(recovery), guidance);
 assert.doesNotMatch(displayedText(recovery), irrelevant);
});


test('preferred improvement renders its cited repository and Brain evidence',async t=>{
 const view=await fixture(t,status());
 const hash='a'.repeat(64);
 const report={summary:'One supported opportunity.',concepts:ids.map(conceptId=>({conceptId,maturity:null,confidence:null,repositoryFactHashes:[],rubricPageId:'rubric',rationale:'Limited evidence',limits:[],nextTransition:'Evidence of stronger coverage',unassessedReason:'Insufficient evidence'})),opportunities:[{id:'entry-link',targetSurface:'instructions',title:'Scoped entry point',summary:'Link scoped instructions.',mechanism:'The entry point lacks the scoped link.',benefit:'Scoped constraints become discoverable.',evidenceStrength:'supported_hypothesis',effort:'low',risk:'low',reversibility:'high',repositoryFactHashes:[hash],knowledgePageIds:['concepts/context-discovery.md'],validation:'Link resolution remains unverified.',rollback:'Restore the previous entry point.'}],preferredOpportunityId:'entry-link',noProposalReason:null,limitations:[],coverage:0,overallScore:null,recommendationId:null,historySnapshotId:null,historyError:null,repositoryEvidence:[{path:'AGENTS.md',offset:0,fileSize:24,sha256:hash,text:'Repository entry facts.',complete:false}],knowledgeEvidence:[{pageId:'concepts/context-discovery.md',title:'Context discovery',status:'active',bundleVersion:'brain-test',sha256:'b'.repeat(64),excerpt:'Scoped instructions need a discoverable entry point.'}]};
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'evidence-run',state:'succeeded',phase:null,startedAt:'2026-11-12T10:00:00Z',completedAt:'2026-11-12T10:01:00Z',errorCode:null,report}});
 view.tick();await view.settle();
 const text=displayedText(view.render());
 assert.match(text,/AGENTS.md/);
 assert.match(text,/Context discovery/);
 assert.match(text,/Insufficient evidence/);
 assert.match(text,/Repository entry facts/);
 assert.match(text,/concepts\/context-discovery.md/);
 assert.match(text,/Scoped instructions need a discoverable entry point/);
});


test('pending recommendation publication keeps all assessment actions busy',async t=>{
 const busy=[];const view=await fixture(t,status(),{onBusyChange:value=>busy.push(value)});
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'pending',state:'pending_recommendation',phase:null,startedAt:'2026-11-12T10:00:00Z',completedAt:null,errorCode:null,report:null}});
 view.tick();await view.settle();const tree=view.render();await view.settle();
 assert.equal(busy.at(-1),true);
 assert.ok(nodes(tree).filter(node=>node.type==='button').every(button=>button.props.disabled));
 assert.match(displayedText(tree),/Saving.*recommendation/i);
});

test('a completed improvement surfaces its linked recommendation once',async t=>{
 const surfaced=[];const view=await fixture(t,status(),{onSuggestion:id=>surfaced.push(id)});
 const report={...snapshot().data,opportunities:[],preferredOpportunityId:null,noProposalReason:'No supported proposal.',limitations:[],recommendationId:'recommendation-1',historySnapshotId:null,historyError:null};
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'completed',state:'succeeded',phase:null,startedAt:'2026-11-12T10:00:00Z',completedAt:'2026-11-12T10:01:00Z',errorCode:null,report}});
 view.tick();await view.settle();view.render();await view.settle();view.render();await view.settle();
 assert.deepEqual(surfaced,['recommendation-1']);
});

test('older improvement reads cannot restore evidence after a newer redaction',async t=>{
 const view=await fixture(t,status());const older=deferred(),newer=deferred();let calls=0;
 window.orkworks.getImprovementStatus=()=>++calls===1?older.promise:newer.promise;
 view.tick();view.tick();newer.resolve({workspacePath:'/repo',available:false,reasonCode:'access_denied',run:null});await view.settle();
 older.resolve({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'stale',state:'running',phase:'assessing',startedAt:'2026-11-12T10:00:00Z',completedAt:null,errorCode:null,report:null}});await view.settle();
 assert.doesNotMatch(displayedText(view.render()),/Assessing opportunities/);
});


test('workspace switch synchronously hides the former improvement report before effects',async t=>{
 const view=await fixture(t,status());
 const report={...snapshot().data,summary:'Private former workspace assessment',opportunities:[],preferredOpportunityId:null,noProposalReason:'Private former workspace reason',limitations:[],recommendationId:null,historySnapshotId:null,historyError:null};
 view.setImprovementStatus({workspacePath:'/repo',available:true,reasonCode:null,run:{id:'former',state:'succeeded',phase:null,startedAt:'2026-11-12T10:00:00Z',completedAt:'2026-11-12T10:01:00Z',errorCode:null,report}});
 view.tick();await view.settle();assert.match(displayedText(view.render()),/Private former workspace/);
 assert.doesNotMatch(displayedText(view.render({workspacePath:'/other'})),/Private former workspace/);
});
