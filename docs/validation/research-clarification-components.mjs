// Decoded finite documentation samples and supplied-fact oracles only.
// No wire decoder, publisher, credential validator, runtime reducer or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),digest=(n,x)=>hash(n+'\n'+canonical(x));
const caps={objects:2097152,research_clarification_requests:16384,research_clarification_answers:32768,research_run_controls:1048576};
const decisionVariants=new Set(['research','clarification','execution']);
export function measureClarificationRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function validateClarificationWidths(r,jobBytes){
 assert(Object.hasOwn(caps,r.table),'supported family');
 if(r.table==='research_clarification_requests')assert(size(r.value.body)<=16384,'request wrapper limit');
 if(r.table==='research_clarification_answers')assert(size(r.value.receipt)<=4096,'receipt limit');
 if(r.table==='research_run_controls')assert(size(r.value.body)<=1048576,'run control limit');
 assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');}
function checkDecisionUnion(decision){
 assert(decisionVariants.has(decision.researchNeeded===true?'research':decision.researchNeeded===false&&decision.clarificationNeeded===true?'clarification':'execution'),'decision variant');
 if(decision.clarificationNeeded===true){
  assert.equal(decision.researchNeeded,false,'clarification variant fixes researchNeeded false');
  assert(decision.inputRevision>=1&&decision.inputRevision<=64,'bounded revision');
  assert(decision.question.questionText.length>0&&Buffer.byteLength(decision.question.questionText)<=2048,'bounded question text');
  assert(decision.question.whyItMatters.length>0&&Buffer.byteLength(decision.question.whyItMatters)<=1024,'bounded whyItMatters');
  assert(decision.question.suggestions.length<=4,'at most four suggestions');
  for(const s of decision.question.suggestions)assert(s.length>0&&Buffer.byteLength(s)<=512,'bounded suggestion');}}
function checkProducer(producer,facts){
 if(producer.policy==='research-clarification-answer-v1'){
  assert.equal(producer.policy,facts.expectedOrigin,'user-origin producer, never a runtime cast');
  return 'user-origin';}
 assert(producer.kind==='parent'||producer.kind==='child','runtime producer kind');
 return 'runtime';}
export function validateResearchClarification(p){
 exact(p,'prefixSha256 assumptions assessment answer freshAssessment changedAnswer exhaustion resume effects measurements');
 assert.equal(p.prefixSha256,hash(prefixRaw));
 validateResearchPrefix(prefix);
 same(p.assumptions,[
  'separate decoded examples, not a store or legal event history and independent of the prefix packet revision numbering',
  'current Main authority, epoch, owner, lease, health, rate and shown context supplied, never authenticated here',
  'sidecar-derived user-origin provenance, runtime eligibility and complete publication facts supplied and unqualified',
  'complete source/consumer/index memberships, original H/C/proofs, ledgers and actual effects unqualified'],'qualification boundary');
 // assessment manifest: fresh clarification decision, exact digest, parent producer
 const manifest=p.assessment.manifest.value.body;
 assert.equal(manifest.preparationRevision,2);assert.equal(manifest.inputRevision,2);
 checkDecisionUnion(manifest.decision);
 assert.equal(manifest.decision.inputRevision,manifest.inputRevision,'fresh decision at its own revision');
 const manifestForHash=clone(manifest);delete manifestForHash.inputDigest;
 assert.equal(manifest.inputDigest,digest('orkworks.preparation-input.v1',manifestForHash),'input digest excludes only inputDigest');
 assert.deepEqual(manifest.records, [{kind:'configuration',reference:manifest.records[0].reference}],'no research records in a clarification manifest');
 assert.equal(p.assessment.manifest.value.producer.kind,'parent','assessment producer is the parent runtime');
 // request row and pending binding
 const request=p.assessment.request.value.body;
 assert.equal(request.inputRevision,manifest.inputRevision,'request binds the asking manifest');
 const requestForHash=clone(request);delete requestForHash.requestDigest;
 assert.equal(request.requestDigest,digest('orkworks.research-clarification-request.v1',requestForHash),'request digest excludes only requestDigest');
 assert.deepEqual(request.question,manifest.decision.question,'byte-equal question');
 assert.equal(p.assessment.request.key.questionVersion,1,'first question version');
 const pending=p.assessment.pending;
 assert.deepEqual(pending,{questionId:'goal-scope',questionVersion:1,requestDigest:request.requestDigest},'exact pending binding');
 const runAfterAsk=p.assessment.runAfter.value.body;
 assert.equal(runAfterAsk.state,'awaiting_clarification','asking selects awaiting_clarification when eligible');
 assert.deepEqual(runAfterAsk.pendingClarification,pending,'pending binding on the run control');
 assert.deepEqual(runAfterAsk.currentInput,{...p.assessment.runAfter.value.body.currentInput},'sole current-input pointer');
 assert.equal(runAfterAsk.runVersion,3,'one runVersion advance');
 const gated=p.assessment.gated;
 assert.equal(gated.blocked.state,'blocked','higher gate governs the asked question');
 assert.deepEqual(gated.blocked.pendingClarification,pending,'pending recorded under the higher gate');
 assert.equal(gated.stageFence.expected,'clarification-mutations-rejected-under-stage-fence','stage fence label');
 // answer: origin, digest, receipt, family row
 const answer=p.answer.body;
 assert.equal(answer.policy===undefined,true,'origin is a member, not the body kind');
 const answerForHash=clone(answer);delete answerForHash.answerDigest;
 assert.equal(answer.answerDigest,digest('orkworks.research-clarification-answer.v1',answerForHash),'answer digest excludes only answerDigest');
 assert.equal(answer.origin.policy,'research-clarification-answer-v1','user-origin policy');
 assert.equal(answer.requestDigest,request.requestDigest,'answer resolves the exact request');
 const answerRow=p.answer.row;
 exact(answerRow.value,'schemaVersion request answer receipt lastCommit','closed answers family value');
 assert.deepEqual(answerRow.value.request,{run:{...runKey()},questionId:'goal-scope',questionVersion:1,requestDigest:request.requestDigest},'request ref');
 assert.equal(answerRow.value.receipt.inputRevision,3,'newly published input revision');
 assert(size(answerRow.value.receipt)<=4096,'receipt bound');
 assert.equal(p.answer.manifest.value.body.records.length,2,'answer manifest gains exactly one record');
 const answerRecord=p.answer.manifest.value.body.records[1];
 assert.equal(answerRecord.kind,'clarification-answer','one clarification-answer record variant');
 assert.deepEqual(answerRecord.reference,{run:{...runKey()},questionId:'goal-scope',questionVersion:1,
  answerId:'answer-1',answerDigest:answer.answerDigest},'full answer ref');
 const answerManifestForHash=clone(p.answer.manifest.value.body);delete answerManifestForHash.inputDigest;
 assert.equal(p.answer.manifest.value.body.inputDigest,digest('orkworks.preparation-input.v1',answerManifestForHash),'answer manifest resealed');
 assert.equal(checkProducer(p.answer.manifest.value.producer,{expectedOrigin:'research-clarification-answer-v1'}),'user-origin','answer manifest producer is the user origin');
 assert.deepEqual(p.answer.manifest.value.body.decision,manifest.decision,'copied byte-unchanged decision');
 const runAfterAnswer=p.answer.runAfter.value.body;
 assert.equal(runAfterAnswer.state,'assessing','assessing when runtime active and no higher gate');
 assert.equal(runAfterAnswer.pendingClarification,null,'pending cleared');
 assert.equal(runAfterAnswer.runVersion,4,'one coalesced runVersion advance');
 // fresh assessment: retained answer entries, fresh decision, parent producer
 const fresh=p.freshAssessment.manifest.value.body;
 assert.equal(fresh.preparationRevision,4);assert.equal(fresh.inputRevision,4);
 assert.equal(fresh.decision.inputRevision,4,'fresh decision at its own revision');
 assert.equal(fresh.decision.researchNeeded,true,'fresh parent assessment decides research need');
 assert.deepEqual(fresh.records.filter(r=>r.kind==='clarification-answer').map(r=>r.reference),
  p.freshAssessment.retainedAnswers.map(x=>x.reference??x),'applicable clarification-answer entries retained');
 assert.equal(p.freshAssessment.manifest.value.producer.kind,'parent','fresh assessment producer is the current parent runtime');
 // state alternatives
 for(const alt of p.answer.stateAlternatives)
  assert(alt.expected.length>0,'state alternative carries its gate label');
 const paused=p.answer.stateAlternatives.find(x=>x.id==='paused-retained');
 assert.equal(paused.resumeTarget,'assessing','resumeTarget is assessing while paused during clarification');
 // changed answer: re-ask at questionVersion+1
 const reask=p.changedAnswer.request.value.body;
 assert.equal(p.changedAnswer.request.key.questionVersion,2,'re-ask increments questionVersion');
 assert.deepEqual(reask.question,manifest.decision.question,'same question re-asked, never a recycled id');
 assert.equal(p.changedAnswer.previous.questionVersion,1,'original request version retained');
 assert.equal(p.changedAnswer.expected,'changed-answer-re-ask-at-questionVersion-plus-one/original-retained');
 // exhaustion and resume labels
 assert.deepEqual(p.exhaustion,{pairs:32,revisions:64,expected:'atomic-denial/no-wrap-reset-pruning'});
 assert.deepEqual(p.resume.restoredBinding,pending,'resume restores the exact pending binding');
 // effect labels
 same(p.effects.map(x=>x.id),['assessment','answer-admission','answer-outcome','consumer-invalidation',
  'fresh-assessment','changed-answer','exhaustion','resume'],'effect label set');
 const rows=[p.assessment.manifest,p.assessment.request,p.assessment.runAfter,p.answer.manifest,p.answer.runAfter,
  p.freshAssessment.manifest,p.changedAnswer.request];
 const measurements=rows.map(r=>{validateClarificationWidths(r,size(r));return {table:r.table,key:r.key,...measureClarificationRow(r)};});
 same(p.measurements,measurements,'exact key plus committed occurrence projection');return measurements;}
function runKey(){return {workspaceId:'workspace-1',runId:'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1'};}
// All facts below are supplied assertions. These functions do not establish
// credentials, complete inventory, actual outcome, currentness or source proof.
export function checkAnswerAdmission(answersRow,requestsRow,facts){
 assert.equal(facts.authority,'supplied-current-Main','Main authority and shown context before lookup');
 assert.equal(facts.stageContinueRequired,false,'stage fence rejects clarification mutations');
 if(facts.retainedIdentical){same(facts.request,requestsRow.value.body,'identical request bytes');
  assert.equal(facts.currentnessRequired,false,'original receipt returns before live CAS');
  return clone(answersRow.value.receipt);}
 assert.deepEqual(facts.pending,{...facts.pending&&{questionId:facts.pending.questionId,questionVersion:facts.pending.questionVersion,requestDigest:requestsRow.value.body.requestDigest}},'current unanswered pending request');
 assert.equal(facts.expectedInputRevision,facts.currentInputRevision,'stale input rejected');
 return clone(answersRow.value.receipt);}
export function checkAnswerState(f){
 if(['complete','cancelled'].includes(f.state))return f.state;
 if(f.stageContinueRequired)return 'paused';
 if(f.blockers>0||f.allocation!=='settled')return 'blocked';
 if(f.resumeApprovalRequired)return 'awaiting_resume_approval';
 assert.equal(f.parent,'supplied-active','fresh assessment needs the active eligible parent');
 return 'assessing';}
