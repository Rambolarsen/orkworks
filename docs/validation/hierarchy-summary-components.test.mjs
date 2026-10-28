import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {decodeSummary, validateRequest, validateResponse, viewedIdentity} from './hierarchy-summary-components.mjs';
const catalog = JSON.parse(readFileSync(new URL('./hierarchy-summary-vectors.json', import.meta.url)));
const vector = id => {
  const v = catalog.vectors.find(v => v.id === id);
  return {context: structuredClone(v.context), request: JSON.parse(v.rendererRequest.canonicalUtf8), response: JSON.parse(v.response.canonicalUtf8)};
};
const pass = {explicitOpen:true, visible:true, completeMaterial:true, painted:true, sameRequest:true};

for (const v of catalog.vectors) test(`${v.id}: independent canonical UTF-8 vectors pass both request boundaries and complete response`, () => {
  for (const [name, limit] of [['rendererRequest',4096], ['mainRequest',4096], ['response',4194304]]) {
    const raw = Buffer.from(v[name].canonicalUtf8);
    assert.equal(raw.length, v[name].utf8Bytes);
    assert.equal(createHash('sha256').update(raw).digest('hex'), v[name].sha256);
    const decoded = decodeSummary(raw, limit);
    const request = JSON.parse(v.rendererRequest.canonicalUtf8);
    if (name === 'response') assert.equal(validateResponse(decoded, request, v.context), decoded);
    else assert.equal(validateRequest(decoded, v.context, name === 'mainRequest' ? 'main' : 'renderer'), decoded);
  }
});

for (const [name, raw] of [
  ['duplicate keys','{"a":1,"a":2}'], ['escaped duplicate keys','{"a":1,"\\u0061":2}'],
  ['fraction','{"a":1.0}'], ['exponent','{"a":1e0}'], ['negative zero','{"a":-0}'],
  ['unsafe integer','{"a":9007199254740993}'], ['unpaired surrogate','{"a":"\\ud800"}'],
  ['depth 33','['.repeat(33)+'0'+']'.repeat(33)], ['trailing data','{}{}'],
]) test(`wire rejects ${name} before schema adoption`, () => assert.throws(() => decodeSummary(Buffer.from(raw),4194304)));
test('wire rejects invalid UTF-8', () => assert.throws(() => decodeSummary(Buffer.from([0xff]),4096)));
test('wire accepts whitespace, property order and escaped Unicode without requiring canonical transport', () => {
  assert.deepEqual(decodeSummary(Buffer.from(' { "b": 1, "a": "\\u00e6" } '),4096), {b:1,a:'æ'});
});
test('wire enforces inclusive UTF-8 request, response and depth limits', () => {
  for (const limit of [4096,4194304]) {
    const raw = Buffer.from('"'+'æ'.repeat((limit-2)/2)+'"');
    assert.equal(raw.length,limit);
    assert.equal(typeof decodeSummary(raw,limit),'string');
    assert.throws(() => decodeSummary(Buffer.concat([raw,Buffer.from(' ')]),limit));
  }
  assert.doesNotThrow(() => decodeSummary(Buffer.from('['.repeat(32)+'0'+']'.repeat(32)),4096));
});

for (const [name, mutate] of [
  ['renderer-supplied workspace', r=>r.workspaceId='w'], ['bearer', r=>r.token='secret'],
  ['path',r=>r.path='/tmp/x'], ['digest-only selector',r=>r.selection={resultDigest:'a'.repeat(64)}],
  ['unknown request field',r=>r.extra=true], ['numeric summary revision',r=>r.selection.summaryRevision=1],
  ['leading-zero revision',r=>r.selection.summaryRevision='01'], ['exhausted revision',r=>r.selection.summaryRevision='2147483648'],
  ['missing frozen task version',r=>delete r.selection.assignmentIdentity.taskVersion],
  ['wrong frozen task version',r=>r.selection.assignmentIdentity.taskVersion=2],
  ['wrong accepted assignment',r=>r.selection.accepted.reference.assignmentIdentity.taskId='other'],
  ['string snapshot',r=>r.expectedSnapshotVersion='1'], ['unknown accepted tag',r=>r.selection.accepted.kind='latest'],
]) test(`request rejects ${name}`, () => {
  const {request,context}=vector('r2-required-stale'); mutate(request);
  assert.throws(() => validateRequest(request,context));
});
test('main must derive the exact workspace and generation',()=>{
  const {request,context}=vector('meets');
  const main={...request,workspaceId:context.workspaceId,sidecarGeneration:context.sidecarGeneration};
  main.sidecarGeneration='0'.repeat(64);
  assert.throws(()=>validateRequest(main,context,'main'));
});
for (const [field,value] of [['fresh',false],['snapshotVersion',2],['workspaceId','other'],['runId','other']]) test(`request cannot bypass ${field} context fence`,()=>{
  const {request,context}=vector('meets');context[field]=value;assert.throws(()=>validateRequest(request,context));
});

for (const [name, mutate, seed='meets'] of [
  ['unknown response field',r=>r.token='forged'], ['unknown detail field',r=>r.detail.instructions='hidden'],
  ['wrong lifetime',r=>r.sidecarGeneration='0'.repeat(64)], ['wrong snapshot',r=>r.snapshotVersion++],
  ['retargeted summary',r=>r.selection.summaryRevision='2'], ['material assignment drift',r=>r.detail.material.assignmentIdentity.taskId='other'],
  ['material accepted drift',r=>r.detail.material.accepted.reference.resultRevision=2],
  ['criteria digest drift',r=>r.detail.criteria[0].description='changed'], ['rubric digest drift',r=>r.detail.rubric.dimensions[0].description='changed'],
  ['missing required outcome',r=>r.detail.outcomes.pop()], ['reordered outcomes',r=>r.detail.outcomes.reverse()],
  ['missing annotation',r=>r.detail.outcomes[0].assessments.pop()], ['unknown annotation digest',r=>r.detail.outcomes[0].assessments[0].evaluationDigest='0'.repeat(64)],
  ['missing overview evaluation',r=>r.detail.evaluations=[]], ['duplicate digest references',r=>r.detail.evaluations[1].evaluationDigest=r.detail.evaluations[0].evaluationDigest,'ratings-0-2'],
  ['promoted stale claim',r=>r.detail.outcomes.find(o=>o.target.id==='c1').assessments[0].effective='unsatisfied','r2-required-stale'],
  ['false full assessment',r=>r.detail.outcomes.find(o=>o.target.id==='c1').fullyAssessed=true,'r2-required-stale'],
  ['wrong claim domain',r=>r.detail.outcomes.find(o=>o.target.id==='c1').assessments[0].claimed=3],
  ['unknown reason',r=>r.detail.outcomes[0].assessments[0].reasons=['guess']],
  ['duplicate reasons',r=>r.detail.outcomes[0].assessments[0].reasons=['invalidated','invalidated']],
  ['negative completeness',r=>r.detail.completeness.satisfied=-1], ['invented denominator',r=>r.detail.completeness.percentage.denominator=1],
  ['shrunken completeness',r=>r.detail.completeness={required:1,satisfied:1,unsatisfied:0,unassessed:0,percentage:{kind:'assessed',numerator:1,denominator:1}}],
  ['assessed incomplete percentage',r=>r.detail.completeness.percentage={kind:'assessed',numerator:0,denominator:2},'r2-required-stale'],
  ['averaged quality',r=>r.detail.quality={kind:'rating',value:1},'ratings-0-2'],
  ['threshold disagreement as pass',r=>r.detail.quality={kind:'rating',value:3},'ratings-2-3'],
  ['quality 3 over bad dimension',r=>r.detail.quality={kind:'rating',value:3},'rating-3-below-standard'],
  ['missing quality becomes zero',r=>r.detail.quality={kind:'rating',value:0},'missing-dimension-rating-1'],
  ['missing finding source',r=>r.detail.findingSources=[],'current-finding'],
  ['unknown finding origin',r=>r.detail.findingSources[0].originEvaluationDigest='0'.repeat(64),'current-finding'],
  ['wrong finding reviewer',r=>r.detail.findingSources[0].reviewerIdentity.userReviewId='someone-else','current-finding'],
  ['missing effective dispute',r=>r.detail.findingSources[0].disputeEvaluationDigests=[],'disputed-finding'],
  ['duplicate dispute',r=>r.detail.findingSources[0].disputeEvaluationDigests.push(r.detail.findingSources[0].disputeEvaluationDigests[0]),'disputed-finding'],
]) test(`response rejects ${name}`,()=>{
  const {response,request,context}=vector(seed);mutate(response);
  assert.throws(()=>validateResponse(response,request,context));
});

test('R2 required-stale and optional-stale projections explain different current required work',()=>{
  const a=vector('r2-required-stale'), b=vector('r2-optional-stale');
  validateResponse(a.response,a.request,a.context);validateResponse(b.response,b.request,b.context);
  assert.deepEqual(a.response.detail.evaluations,b.response.detail.evaluations);
  assert.equal(a.response.detail.material.overall,'needs-rework');assert.equal(b.response.detail.material.overall,'needs-rework');
  assert.equal(a.response.detail.completeness.percentage.kind,'unassessed');
  assert.deepEqual(b.response.detail.completeness.percentage,{kind:'assessed',numerator:0,denominator:2});
  assert.equal(a.response.detail.outcomes.find(o=>o.target.id==='c1').fullyAssessed,false);
  assert.equal(b.response.detail.outcomes.find(o=>o.target.id==='c1').fullyAssessed,true);
});

test('only explicit, current, complete, painted material presentation yields a local viewed identity',()=>{
  const {response,request,context}=vector('meets');
  const identity=viewedIdentity(response,request,context,pass);
  assert.deepEqual(identity,{workspaceId:'w',runId:'run',assignmentIdentity:request.selection.assignmentIdentity,
    resultRevision:1,resultDigest:request.selection.accepted.reference.resultDigest,summaryRevision:'1'});
  for(const field of Object.keys(pass)) assert.equal(viewedIdentity(response,request,context,{...pass,[field]:false}),null,field);
  for(const [field,value] of [['fresh',false],['snapshotVersion',2],['workspaceId','other'],['sidecarGeneration','0'.repeat(64)]])
    assert.equal(viewedIdentity(response,request,{...context,[field]:value},pass),null,field);
});
test('no accepted result or incomplete/failed response cannot mark viewed',()=>{
  const {response,request,context}=vector('root-no-result');assert.equal(viewedIdentity(response,request,context,pass),null);
  for(const result of [null,{code:'unavailable'}, {...response,detail:null}]) assert.equal(viewedIdentity(result,request,context,pass),null);
});
test('detail-only rating changes invalidate the old snapshot but retain the same material viewed identity',()=>{
  const a=vector('rating-change-0'),b=vector('rating-change-2');
  assert.deepEqual(a.response.detail.material,b.response.detail.material);
  assert.throws(()=>validateResponse(a.response,a.request,b.context));
  assert.deepEqual(viewedIdentity(a.response,a.request,a.context,pass),viewedIdentity(b.response,b.request,b.context,pass));
});
test('A→B→A with a new summary revision does not reuse the earlier viewed identity',()=>{
  const a=vector('meets'),b=structuredClone(a);b.request.selection.summaryRevision='3';b.context.selection.summaryRevision='3';b.response.selection.summaryRevision='3';
  assert.notDeepEqual(viewedIdentity(a.response,a.request,a.context,pass),viewedIdentity(b.response,b.request,b.context,pass));
});

test('fully assessed material cannot call an unsatisfied annotation meets',()=>{
  const {response,request,context}=vector('r2-optional-stale');
  response.detail.material.requiredOutcomes.find(o=>o.target.id==='c1').disposition='meets';
  response.detail.completeness={required:2,satisfied:1,unsatisfied:1,unassessed:0,percentage:{kind:'assessed',numerator:1,denominator:2}};
  assert.throws(()=>validateResponse(response,request,context));
});

test('required arrays, text and annotation counts cannot silently exceed the declared caps',()=>{
  for(const [field,maximum] of [['criteria',32],['evaluations',16],['outcomes',49],['findingSources',512]]){
    const {response,request,context}=vector('meets');response.detail[field]=Array(maximum+1).fill(response.detail[field][0]??{});
    assert.throws(()=>validateResponse(response,request,context),field);
  }
  const {response,request,context}=vector('meets');
  response.detail.criteria[0].description='æ'.repeat(1025);
  assert.throws(()=>validateResponse(response,request,context));
});

test('nonparticipating historical streams cannot add an unassessed vote or erase a current failure',()=>{
  const {response,request,context}=vector('failure-plus-unassessed');
  const c1=response.detail.outcomes.find(o=>o.target.id==='c1');
  const history=c1.assessments.find(a=>a.effective==='unassessed');
  history.participates=false;history.reasons=['invalidated'];c1.fullyAssessed=true;
  response.detail.completeness={required:2,satisfied:1,unsatisfied:1,unassessed:0,percentage:{kind:'assessed',numerator:1,denominator:2}};
  assert.equal(validateResponse(response,request,context),response);
});

test('an Unassessed contributor is not an assessed conflict and cannot erase known required failure material',()=>{
  const {response}=vector('failure-plus-unassessed');
  assert.equal(response.detail.material.requiredOutcomes.find(o=>o.target.id==='c1').disposition,'needs-rework');
});

test('approved rubric dimension order is preserved instead of sorted by the display checker',()=>{
  const {response,request,context}=vector('meets');
  const d=response.detail;
  d.rubric.dimensions.unshift({id:'z',description:'Declared first.'});
  // Literal canonical approved snapshot, independent of the checker serializer.
  d.material.rubricSnapshotDigest=createHash('sha256').update('orkworks.rubric-snapshot.v1\n'+
    '{"dimensions":[{"description":"Declared first.","id":"z"},{"description":"Correctness.","id":"d"}],"evaluatorRole":"review","id":"rubric","version":1}').digest('hex');
  const model=structuredClone(d.outcomes.find(o=>o.target.id==='d'));model.target.id='z';
  const material=structuredClone(d.material.requiredOutcomes.find(o=>o.target.id==='d'));material.target.id='z';
  // Material target bytes order by id within these two dimension targets.
  const ratingIndex=d.outcomes.findIndex(o=>o.target.kind==='rating');
  d.outcomes.splice(ratingIndex,0,model);d.material.requiredOutcomes.splice(ratingIndex,0,material);
  assert.equal(validateResponse(response,request,context),response);
});

test('disputed finding retains its origin’s adverse claim while effective target becomes Unassessed',()=>{
  const {response}=vector('disputed-finding');
  const source=response.detail.findingSources[0];
  const annotation=response.detail.outcomes.find(o=>o.target.id==='c1').assessments.find(a=>a.evaluationDigest===source.originEvaluationDigest);
  assert.equal(annotation.claimed,'unsatisfied');assert.equal(annotation.effective,'unassessed');
  assert(annotation.reasons.includes('declared-conflict'));
});

for(const reason of ['missing-evidence','evidence-unavailable','stale-evidence']) test(`unsupported ${reason} cannot retain an assessed effective claim`,()=>{
  const {response,request,context}=vector('meets');
  response.detail.outcomes[0].assessments[0].reasons=[reason];
  assert.throws(()=>validateResponse(response,request,context));
});
test('not fully assessed support cannot retain meets material',()=>{
  const {response,request,context}=vector('r2-required-stale');
  response.detail.material.requiredOutcomes.find(o=>o.target.id==='c1').disposition='meets';
  assert.throws(()=>validateResponse(response,request,context));
});

test('criteria retain the owning stable-ID order even with a recomputed snapshot digest',()=>{
  const {response,request,context}=vector('meets');
  response.detail.criteria.reverse();
  response.detail.material.criteriaSnapshotDigest=createHash('sha256').update('orkworks.criteria-snapshot.v1\n'+JSON.stringify(response.detail.criteria)).digest('hex');
  assert.throws(()=>validateResponse(response,request,context));
});
test('an approved criterion description is a required source field, including on optional criteria',()=>{
  const {response,request,context}=vector('meets');
  delete response.detail.criteria.find(c=>c.requirement==='optional').description;
  response.detail.material.criteriaSnapshotDigest=createHash('sha256').update('orkworks.criteria-snapshot.v1\n'+JSON.stringify(response.detail.criteria)).digest('hex');
  assert.throws(()=>validateResponse(response,request,context));
});
