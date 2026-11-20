// Decoded finite documentation samples and supplied-fact oracles only.
// No wire decoder, publisher, credential validator, runtime reducer or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateResearchCompletion} from './research-completion-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const completionRaw=readFileSync(new URL('research-completion-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw),completion=JSON.parse(completionRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),digest=(n,x)=>hash(n+'\n'+canonical(x));
const caps={research_execution_plan_objects:2097152,research_execution_plan_approvals:16384,
 research_run_controls:1048576,research_plan_controls:16384,research_task_controls:16384,research_lifecycle_records:2097152};
export function measureExecutionRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function validateExecutionWidths(r,jobBytes){
 assert(Object.hasOwn(caps,r.table),'supported family');
 if(r.table==='research_execution_plan_approvals')assert(size(r.value.receipt)<=4096,'receipt limit');
 if(r.table==='research_lifecycle_records')assert(size(r.value.receipt)<=4096,'lifecycle receipt limit');
 assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');}
export function validateResearchExecution(p){
 exact(p,'prefixSha256 completionSha256 assumptions proposal publicationEffects approval rejection effects measurements');
 assert.equal(p.prefixSha256,hash(prefixRaw));assert.equal(p.completionSha256,hash(completionRaw));
 validateResearchPrefix(prefix);validateResearchCompletion(completion);
 same(p.assumptions,[
  'separate decoded examples, not a store or legal event history',
  'current Main/parent authority, epoch, owner, lease, health, rate and shown context supplied, never authenticated here',
  'grants are volatile server-held facts represented as label assertions only, never persisted or serialized',
  'capability/profile qualification, launch-chain admission, complete memberships, original H/C/proofs, ledgers and actual effects unqualified'],'qualification boundary');
 // proposal source: accepted shape, digest domain, exact binding
 const proposal=p.proposal.source;
 assert.equal(proposal.kind,'single-task-execution-source');
 assert(!('reviewTask' in proposal),'review-only variant not used here');
 assert.deepEqual(Object.keys(proposal).sort(),['batches','kind','maxParallelChildren','parentPlanBinding','planDefinitionDigest','reasons','researchBinding','schemaVersion','scopeText','task'],'closed proposal members');
 assert.deepEqual(proposal.researchBinding.summary,completion.summaries[1].selectorAfter.value.body.currentSummary,'exact current summary');
 assert.deepEqual(proposal.researchBinding.completedPlan,completion.completions[0].row.value.body.request.plan,'exact completed research plan');
 assert.deepEqual(proposal.researchBinding.input,completion.summaries[1].row.value.body.input,'exact current input ref');
 assert.equal(proposal.parentPlanBinding.planId,'execution-plan','binding rebuilt for the execution identity');
 assert.equal(proposal.parentPlanBinding.planRevision,1,'fresh plan revision');
 assert.equal(proposal.parentPlanBinding.preparationRevision,proposal.researchBinding.input.preparationRevision,'binding resolves the current manifest');
 assert.equal(proposal.parentPlanBinding.inputDigest,proposal.researchBinding.input.inputDigest,'binding pins the current input digest');
 assert.deepEqual(proposal.batches,[['execution-task']],'exactly one declared task');
 assert.equal(proposal.maxParallelChildren,1);
 assert.equal(proposal.task.group.groupId,'execution-group','independent new group');
 assert(proposal.scopeText.length>0&&Buffer.byteLength(proposal.scopeText)<=4096,'bounded scopeText');
 assert(proposal.reasons.length>=1&&proposal.reasons.length<=4,'bounded reasons');
 assert.equal(p.proposal.domain,'orkworks.research-execution-source-plan.v1');
 const proposalForHash=clone(proposal);delete proposalForHash.planDefinitionDigest;
 assert.equal(proposal.planDefinitionDigest,digest('orkworks.research-execution-source-plan.v1',proposalForHash),'plan digest excludes only planDefinitionDigest');
 assert(size(proposal)<=65536,'64 KiB plan body bound');
 exact(p.proposal.publication,'schemaVersion expectedRunVersion plan','offline publication input, non-retained');
 // publication effects: plan/task controls proposed v1, planReferences append, pendingPlan bind
 const runAfter=p.publicationEffects.runAfter.value.body;
 assert.equal(runAfter.state,'awaiting_execution_approval','positive writer for the retained handoff value');
 assert.deepEqual(runAfter.planReferences,[completion.completions[0].row.value.body.request.plan,p.publicationEffects.planControl.value.body.plan],'planRef appended once');
 assert.deepEqual(runAfter.pendingPlan,p.publicationEffects.planControl.value.body.plan,'pendingPlan binding');
 assert.equal(runAfter.currentPlan,null,'currentPlan stays cleared (accepted completion effect unchanged)');
 assert.equal(runAfter.runVersion,51,'one runVersion advance');
 assert.equal(p.publicationEffects.planControl.value.body.status,'proposed','accepted status union value');
 assert.deepEqual(p.publicationEffects.planControl.value.body.task,clone(p.publicationEffects.taskControl.value.body.task),
  'plan control binds the execution-plan task');
 assert.deepEqual(p.publicationEffects.taskControl.key,clone(p.publicationEffects.planControl.value.body.task),'task control keyed by the execution task');
 assert.equal(p.publicationEffects.taskControl.value.body.task.planId,'execution-plan','task key built from the execution plan identity');
 assert.equal(p.publicationEffects.taskControl.value.body.taskVersion,1,'task control at version 1');
 const gated=p.publicationEffects.gated;
 assert.equal(gated.blocked.state,'blocked','gated state retained with the pending recorded');
 assert.deepEqual(p.publicationEffects.paused,{state:'paused',pauseReason:'sidecar_replaced',resumeTarget:'assessing',
  expected:'resumeTarget-assessing-while-paused-before-this-state'},'paused before this state keeps its own target');
 // approval: exact shown bytes, digest domains, closed family members, label grants
 const approvalRequest=p.approval.request,source=p.approval.source;
 assert.equal(approvalRequest.action,'orkworks.research-execution-plan.approve.v1');
 const sourceForHash=clone(source);delete sourceForHash.requestDigest;
 assert.equal(source.requestDigest,digest('orkworks.research-execution-plan-request.v1',approvalRequest),'request digest excludes only requestDigest');
 const approvalRow=p.approval.row;
 assert.deepEqual(p.proposal.objectRow.value.body,clone(proposal),'retained proposal body equals the validated source');
 exact(approvalRow.value,'schemaVersion body digest receipt lastCommit','closed approval family value');
 assert.deepEqual(approvalRow.value.body,clone(source),'retained approval body equals the validated source');
 assert.equal(approvalRow.value.digest,digest('orkworks.research-execution-plan-approval.v1',clone(approvalRow.value.body)),'approval digest over the retained body');
 assert.deepEqual(approvalRow.value.receipt,{schemaVersion:1,plan:approvalRequest.plan,approvalId:'execution-approval-1',
  approvalDigest:approvalRow.value.digest,requestDigest:source.requestDigest,commit:approvalRow.value.receipt.commit},'receipt shape');
 assert(size(approvalRow.value.receipt)<=4096,'receipt bound');
 assert.equal(p.approval.planAfter.value.body.status,'active','approval activates the plan control');
 assert.deepEqual(p.approval.grantLabels,{grantExisted:'supplied',grantRevokedAtCompletion:'supplied',grantFencing:'supplied',
  expected:'label-assertions-only/never-serialized/never-restored'},'grant label assertions');
 const approvalRunAfter=p.approval.runAfter.value.body;
 assert.deepEqual(approvalRunAfter.currentPlan,approvalRequest.plan,'approval binds currentPlan to the execution ref');
 assert.equal(approvalRunAfter.pendingPlan,null,'approval clears pendingPlan');
 assert.equal(approvalRunAfter.runVersion,52,'approval advances runVersion once');
 assert.equal(p.approval.preconditions.resumedParent,'supplied-eligible','new approvals need a resumed eligible parent');
 // rejection: lifecycle machinery, cancelled proposed plan, pendingPlan cleared, pause retained
 const rejectRequest=p.rejection.request;
 assert.equal(rejectRequest.kind,'reject-execution-proposal');
 exact(rejectRequest,'schemaVersion kind run expectedRunVersion input decisionId proposal reason reviewContextId sidecarGeneration','closed rejection request');
 const rejectRow=p.rejection.row;
 assert.deepEqual(rejectRow.value.body.request,clone(rejectRequest),'retained request bytes');
 const rejectForHash=clone(rejectRow.value.body);delete rejectForHash.digest;
 assert.equal(rejectRow.value.digest,digest('orkworks.research-lifecycle-decision.v1',rejectForHash),'rejection digest over the retained body');
 assert.deepEqual(rejectRow.value.body.readSet.pendingPlan.value.body,clone(p.publicationEffects.planControl.value.body),
  'captured proposed plan read');
 assert.equal(rejectRow.value.receipt.planVersion,null,'rejection receipt planVersion stays null');
 const rejectRunAfter=p.rejection.runAfter.value.body;
 assert.equal(rejectRunAfter.pendingPlan,null,'pendingPlan cleared');
 assert.equal(rejectRunAfter.runVersion,52,'one runVersion advance');
 assert.equal(rejectRunAfter.state,'paused','interrupted parent stays paused through rejection');
 assert.equal(rejectRunAfter.pauseReason,'parent_ended','pause reason retained');
 assert.equal(rejectRunAfter.resumeTarget,'awaiting_execution_approval','coordination target retained across pauses');
 assert.equal(rejectRow.value.body.readSet.run.value.body.state,'paused','captured read set is the paused prestate');
 assert.equal(rejectRow.value.body.readSet.run.value.body.pauseReason,'parent_ended','captured pause reason');
 assert.equal(p.rejection.pausedCase.expected,'captured-paused-prestate-preserved-through-rejection/same-reason-and-target','pause-retention rule demonstrated');
 assert.equal(p.rejection.planAfter.value.body.status,'cancelled','rejected proposal marked cancelled');
 assert.equal(p.rejection.planAfter.value.body.planVersion,2,'rejected plan version advanced once');
 assert.equal(p.rejection.replacement.expected,'new-proposal-bound-to-current-context/never-a-mutation','replacement rule');
 // effect labels
 same(p.effects.map(x=>x.id),['publication','gated-state','approval','rejection','invalidation','denials'],'effect label set');
 const rows=[p.proposal.objectRow,p.publicationEffects.runAfter,p.publicationEffects.planControl,p.publicationEffects.taskControl,
  p.approval.row,p.approval.planAfter,p.approval.runAfter,p.rejection.row,p.rejection.runAfter,p.rejection.planAfter];
 const measurements=rows.map(r=>{validateExecutionWidths(r,size(r));return {table:r.table,key:r.key,...measureExecutionRow(r)};});
 same(p.measurements,measurements,'exact key plus committed occurrence projection');return measurements;}
// All facts below are supplied assertions. These functions do not establish
// credentials, complete inventory, actual outcome, currentness or source proof.
export function checkProposalBinding(proposal,facts){
 assert.equal(facts.authority,'supplied-current-parent','planning authority before publication');
 assert.equal(facts.stageContinueRequired,false,'stage fence makes publication unavailable');
 assert.deepEqual(proposal.researchBinding.summary,facts.currentSummary,'stale summary denied');
 assert.deepEqual(proposal.researchBinding.completedPlan,facts.completedResearchPlan,'stale completed plan denied');
 assert.deepEqual(proposal.researchBinding.input,facts.currentInput,'stale input denied');
 assert.equal(facts.unansweredClarification,null,'no current unanswered clarification');
 assert.equal(facts.activeGrant,false,'one-grant-at-a-time guard');
 return 'published';}
export function checkExecutionApproval(f,proposal){
 assert.equal(f.authority,'supplied-current-Main','Main authority and shown context before lookup');
 assert.equal(f.stageContinueRequired,false,'stage fence makes approval unavailable');
 assert.notEqual(f.approver,'run-bearer','the run bearer alone never approves');
 assert.equal(f.resumedParent,'supplied-eligible','actually resumed eligible bound parent');
 assert.equal(f.profileQualification,'supplied','labelled capability fact, no fallback');
 assert.deepEqual(proposal.researchBinding.summary,f.currentSummary,'stale summary denies approval');
 assert.deepEqual(proposal.researchBinding.completedPlan,f.completedResearchPlan,'stale completed plan denies approval');
 assert.deepEqual(proposal.researchBinding.input,f.currentInput,'stale input denies approval');
 assert.equal(f.unansweredClarification,null,'no current unanswered clarification');
 return f.grantLabels;}
