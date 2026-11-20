import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchExecution,checkProposalBinding,checkExecutionApproval} from './research-execution-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-execution.py',import.meta.url))]);
test('generator supplies proposal, publication effects, approval and rejection views',()=>{
 const p=JSON.parse(generate());assert.equal(p.effects?.length,6);assert.equal(p.proposal.denialCases.unqualifiedProfile.expected,'approval-unavailable-no-fallback');
});
test('missing execution packaging is rejected',()=>assert.throws(()=>validateResearchExecution({})));

import {readFileSync} from 'node:fs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=readFileSync(new URL('research-execution-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
const domain=(n,b)=>{const c=clone(b);return hash(n+'\n'+canonical(c));};
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);
 assert.deepEqual(validateResearchExecution(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change,reseal] of [
 ['research binding drops the summary',p=>{delete p.proposal.source.researchBinding.summary},'proposal'],
 ['multi-task batches',p=>{p.proposal.source.batches=[['execution-task'],['execution-task-2']]},null],
 ['proposal reuses the research group',p=>{p.proposal.source.task.group.groupId='review-group'},null],
 ['plan digest drifts',p=>{p.proposal.source.planDefinitionDigest=hash('drifted')},null],
 ['publication leaves currentPlan set',p=>{p.publicationEffects.runAfter.value.body.currentPlan=p.proposal.binding.completedPlan},'run'],
 ['pendingPlan unbound',p=>{p.publicationEffects.runAfter.value.body.pendingPlan=null},'run'],
 ['plan control activates without approval',p=>{p.publicationEffects.planControl.value.body.status='active'},null],
 ['approval digest drifts',p=>{p.approval.row.value.digest=hash('drifted')},null],
 ['grant labels serialized as a credential',p=>{p.approval.grantLabels={grantExisted:'serialized-token',expected:'label-assertions-only/never-serialized/never-restored'}},null],
 ['rejection receipt reports a planVersion',p=>{p.rejection.row.value.receipt.planVersion=2},'reject'],
 ['rejected plan stays proposed',p=>{p.rejection.planAfter.value.body.status='proposed'},null],
 ['rejection resumes the run',p=>{p.rejection.runAfter.value.body.state='awaiting_execution_approval';p.rejection.runAfter.value.body.pauseReason=null},'run'],
 ['approval body reviewContextId drifts',p=>{p.approval.row.value.body.reviewContextId=hash('drifted context')},null],
 ['rejection record digest zeroed',p=>{p.rejection.row.value.digest='0'.repeat(64)},null],
 ['task control keyed under the old plan',p=>{p.publicationEffects.taskControl.key={...p.publicationEffects.taskControl.key,planId:'prefix-plan'}},null],
 ['binding keeps the research plan identity',p=>{p.proposal.source.parentPlanBinding.planId='prefix-plan'},null],
 ['rejection captures a non-paused prestate',p=>{p.rejection.row.value.body.readSet.run.value.body.state='awaiting_execution_approval';p.rejection.row.value.body.readSet.run.value.body.pauseReason=null},null],
 ['rejection keeps pendingPlan bound',p=>{p.rejection.runAfter.value.body.pendingPlan=p.publicationEffects.planControl.value.body.plan},'run'],
 ['run bearer approves',p=>{p.approval.preconditions.resumedParent='none';},null],
]) {test(`mutation ${name} is rejected`,()=>{
 const p=packet();change(p);
 if(reseal==='proposal'){const s=p.proposal.source;delete s.planDefinitionDigest;s.planDefinitionDigest=domain('orkworks.research-execution-source-plan.v1',s);}
 assert.throws(()=>validateResearchExecution(p),e=>{assert.ok(e instanceof Error);return true;});
});}
test('supplied-fact oracles separate publication guards from approval authority',()=>{
 const p=packet();
 assert.equal(checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:p.proposal.binding.input,unansweredClarification:null,activeGrant:false,stageContinueRequired:false}),'published');
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:null,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:p.proposal.binding.input,unansweredClarification:null,activeGrant:false,stageContinueRequired:false}),/stale summary denied/);
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:{planId:'other'},
  currentInput:p.proposal.binding.input,unansweredClarification:null,activeGrant:false,stageContinueRequired:false}),/stale completed plan denied/);
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:p.proposal.binding.input,unansweredClarification:{questionId:'q'},activeGrant:false,stageContinueRequired:false}),/no current unanswered clarification/);
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:p.proposal.binding.input,unansweredClarification:null,activeGrant:true,stageContinueRequired:false}),/one-grant-at-a-time guard/);
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:{...p.proposal.binding.input,inputRevision:9},unansweredClarification:null,activeGrant:false,stageContinueRequired:false}),/stale input denied/);
 assert.throws(()=>checkProposalBinding(p.proposal.source,{authority:'supplied-current-parent',
  currentSummary:p.proposal.binding.summary,completedResearchPlan:p.proposal.binding.completedPlan,
  currentInput:p.proposal.binding.input,unansweredClarification:null,activeGrant:false,stageContinueRequired:true}),/stage fence/);
 assert.throws(()=>checkExecutionApproval({authority:'supplied-current-Main',approver:'electron-main',
  resumedParent:'supplied-eligible',profileQualification:'supplied',stageContinueRequired:true,
  grantLabels:p.approval.grantLabels}),/stage fence/);
 const labels=checkExecutionApproval({authority:'supplied-current-Main',approver:'electron-main',
  resumedParent:'supplied-eligible',profileQualification:'supplied',stageContinueRequired:false,
  grantLabels:p.approval.grantLabels});
 assert.equal(labels.expected,'label-assertions-only/never-serialized/never-restored');
 assert.throws(()=>checkExecutionApproval({authority:'supplied-current-Main',approver:'run-bearer',
  resumedParent:'supplied-eligible',profileQualification:'supplied',stageContinueRequired:false,
  grantLabels:p.approval.grantLabels}),/never approves/);
 assert.throws(()=>checkExecutionApproval({authority:'supplied-current-Main',approver:'electron-main',
  resumedParent:'none',profileQualification:'supplied',stageContinueRequired:false,
  grantLabels:p.approval.grantLabels}),/resumed eligible/);
});
test('literal bounds and domains are pinned',()=>{
 const p=packet();
 assert.equal(p.proposal.domain,'orkworks.research-execution-source-plan.v1');
 assert.equal(p.approval.request.action,'orkworks.research-execution-plan.approve.v1');
 assert.equal(p.rejection.request.kind,'reject-execution-proposal');
});
