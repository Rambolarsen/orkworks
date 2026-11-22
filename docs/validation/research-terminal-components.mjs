// Decoded finite documentation samples and supplied-fact oracles only.
// No wire decoder, publisher, credential validator, runtime reducer or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateResearchContinuation} from './research-continuation-components.mjs';
import {validateResearchCompletion} from './research-completion-components.mjs';
import {validateResearchExecution} from './research-execution-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const continuationRaw=readFileSync(new URL('research-continuation-components.json',import.meta.url));
const completionRaw=readFileSync(new URL('research-completion-components.json',import.meta.url));
const executionRaw=readFileSync(new URL('research-execution-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw),continuation=JSON.parse(continuationRaw),
 completion=JSON.parse(completionRaw),execution=JSON.parse(executionRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),digest=(n,x)=>hash(n+'\n'+canonical(x));
const caps={research_turn_completions:65536,research_coordination_receipts:16384,
 research_lifecycle_records:2097152,research_run_controls:1048576,research_lifecycle_state:16384,
 research_plan_controls:16384,research_collections:16384,research_expectations:16384,
 research_session_states:16384,research_task_controls:16384};
export function measureTerminalRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function validateTerminalWidths(r,jobBytes){
 assert(Object.hasOwn(caps,r.table),'supported family');
 if(r.table==='research_lifecycle_records')assert(size(r.value.receipt)<=4096,'receipt limit');
 if(r.table==='research_coordination_receipts')assert(size(r.value.receipt)<=4096,'coordination receipt limit');
 assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');}
function checkOutcomeVerdict(outcome){
 const anyConflict=outcome.questionStatuses.some(s=>s.status==='conflict');
 const allConsistent=outcome.questionStatuses.every(s=>s.status==='consistent');
 return anyConflict?'conflict':allConsistent?'consistent':'uncertain';}
export function validateResearchTerminal(p){
 exact(p,'prefixSha256 continuationSha256 completionSha256 executionSha256 assumptions prestate turn coordination completion terminal retirement liveChild failedExecution accounting catalog effects measurements');
 assert.equal(p.prefixSha256,hash(prefixRaw));assert.equal(p.continuationSha256,hash(continuationRaw));
 assert.equal(p.completionSha256,hash(completionRaw));assert.equal(p.executionSha256,hash(executionRaw));
 validateResearchPrefix(prefix);validateResearchContinuation(continuation);
 validateResearchCompletion(completion);validateResearchExecution(execution);
 same(p.assumptions,[
  'separate decoded examples, not a store or legal event history',
  'reviewer/execution child authority, turn-authentication, launch chain and current Main shown context supplied, never authenticated here',
  'grants are volatile server-held facts represented as label assertions only, never persisted or serialized',
  'capability/profile qualification, complete memberships, original H/C/proofs, ledgers and actual effects unqualified'],'qualification boundary');
 // prestate: synthetic execution-proposal context labeled against that contract
 const pre=p.prestate;
 assert.equal(pre.labeledAgainst,'research-execution-proposal contract accepted choices');
 assert.equal(pre.postLaunchState,'awaiting_execution_approval');
 assert.equal(pre.postLaunchStateLabel,'supplied-launch-chain-state-writer-remains-separately-gated','supplied post-launch state label');
 assert.equal(pre.executionTaskControl.value.body.task.planId,'execution-plan','execution task identity');
 // turn gate: accepted variant, synthetic runtime, digest exclusion
 const turn=p.turn.body;
 assert.equal(turn.evidence.kind,'child_terminal_without_receipt','accepted turn variant');
 assert.equal(p.turn.variant,'child_terminal_without_receipt/synthetic-runtime/supplied-handler-labeling','supplied-origin labeling');
 assert(!('turnDigest' in turn),'accepted turn body carries no digest field');
 const turnRow=p.turn.row;
 assert.deepEqual(Object.keys(turnRow.value).sort(),['body','digest','lastCommit','schemaVersion'],'turn family value shape');
 assert.equal(turnRow.value.digest,digest('orkworks.research-source-turn-completion.v1',clone(turn)),'turn family digest over the exact body');
 assert.equal(turn.acceptedTaskVersion,turn.expectedTaskVersion+1,'accepted version follows the expected version');
 assert.deepEqual(turnRow.key,turn.task,'turn family keyed by the execution task');
 const terminalSnapshot=turn.evidence.session;
 assert.deepEqual(Object.keys(terminalSnapshot).sort(),['key','table','value'],'terminal evidence session is a typed read');
 assert.equal(terminalSnapshot.value.body.state,'terminal','terminal snapshot state');
 assert(terminalSnapshot.value.body.runtime,'terminal snapshot retains the runtime');
 // coordination outcome under the ordinary-child result/turn boundary
 const coordination=p.coordination;
 assert.equal(coordination.row.value.request.outcome,'completed');
 assert.deepEqual(Object.keys(coordination.row.value).sort(),['lastCommit','receipt','request','requestDigest','schemaVersion'],'flat coordination family value');
 assert(!('requestDigest' in coordination.row.value.request),'accepted coordination request carries no digest field');
 const requestForHash=clone(coordination.row.value.request);delete requestForHash.requestDigest;
 assert.equal(coordination.row.value.requestDigest,digest('orkworks.research-source-coordination-request.v1',requestForHash),'coordination request digest');
 assert.deepEqual(coordination.row.value.receipt.acceptedTaskVersion,turn.acceptedTaskVersion+1,'receipt advances the task version once after the turn');
 // completion trigger: accepted coordination form, same-C machinery
 const completionRequest=p.completion.request,trigger=completionRequest.trigger;
 const lifecycleRow=p.completion.row;
 const storedRequest=lifecycleRow.value.body.request;
 assert.deepEqual(storedRequest,clone(completionRequest),'stored lifecycle request equals the validated request');
 const storedTrigger=storedRequest.trigger;
 assert.deepEqual(storedTrigger,{kind:'coordination',task:turn.task,acceptedTaskVersion:coordination.row.value.receipt.acceptedTaskVersion,
  requestDigest:coordination.row.value.requestDigest},'coordination trigger pins the accepted outcome');
 assert.equal(completionRequest.kind,'plan-completion');
 assert.deepEqual(completionRequest.plan,p.prestate.executionPlan,'completion under the execution PlanRef');
 const row=p.completion.row;
 const bodyForHash=clone(row.value.body);delete bodyForHash.digest;
 assert.equal(row.value.digest,digest('orkworks.research-lifecycle-decision.v1',bodyForHash),'lifecycle digest excludes only digest');
 assert.deepEqual(row.value.body.readSet.run.value.body.pendingPlan,p.prestate.executionPlan,'captured pendingPlan is the execution plan');
  assert.equal(completionRequest.expectedPlanVersion,1,'expected plan version matches the captured prestate');
 assert.equal(row.value.receipt.planVersion,2,'completion receipt reports the resulting plan version');
 assert(['coordination','recovery'].includes(trigger.kind),'accepted completion trigger union');
 assert.equal(row.value.receipt.runVersion,row.value.body.readSet.run.value.body.runVersion+1,'no second completion record');
 // absorbing terminal transition
 const before=p.terminal.runBefore.value.body,after=p.terminal.runAfter.value.body;
 assert.equal(after.state,'complete','absorbing terminal value');
 assert.equal(after.runVersion,53,'one coalesced advance');
 assert.equal(after.pauseReason,null,'pause-only fields nulled');
 assert.equal(after.resumeTarget,null,'pause-only fields nulled');
 assert.equal(before.stageContinueRequired,false,'stage gate false in the completing prestate');
 assert.equal(after.stageContinueRequired,false,'raw stage gate value preserved (false stays false)');
 assert.deepEqual(after.blockers,[],'raw blocker values preserved');
 assert.deepEqual(after.currentInput,before.currentInput,'current input unchanged by the terminal transition');
 assert.equal(after.parentRuntime,null,'parent runtime pointer cleared');
 assert.equal(after.pendingPlan,null,'pendingPlan cleared');
 assert.deepEqual(after.planReferences,before.planReferences,'planReferences retained');
 // selector preservation
 const selectorAfter=p.terminal.selectorAfter.value.body;
 assert.deepEqual(selectorAfter.completedResearchPlan,p.terminal.selectorBefore.value.body.completedResearchPlan,
  'completedResearchPlan unchanged by execution completion');
 assert.equal(selectorAfter.completedResearchPlan.planId,'prefix-plan','selector keeps the research plan');
 assert.deepEqual(selectorAfter.currentSummary,p.terminal.selectorBefore.value.body.currentSummary,'current summary binding unchanged');
 // terminal plan control
 assert.equal(p.terminal.planAfter.value.body.status,'complete','execution plan complete');
 assert.equal(p.terminal.planAfter.value.body.planVersion,2,'plan version advanced once');
 // grant revocation labels
 const grants=p.terminal.grantRevocation;
 assert.equal(grants.labels.expected,'label-assertions-only/never-serialized/never-restored','grant label rule');
 assert.equal(grants.labels.grantExisted,'supplied','grant labels are assertions, never serialized credentials');
 assert.deepEqual(Object.keys(grants.retainedApprovalFacts).sort(),['approvalDigest','approvalId','commitRef','planDigest'],
  'retained approval facts as historical bindings');
 // waits and terminal mapping labels
 assert.deepEqual(p.terminal.waits,{runClosed:true,expected:'effect-label-not-a-record/no-reconnect'});
 assert.equal(p.terminal.noTerminalReportRunV1.expected,'terminal-state-only-in-research_run_controls/display-label-not-qualification');
 assert(after.profile.startsWith('single-task-retained-file-research-execution-v1'),'terminal state lives in research_run_controls');
 // retirement
 const retirement=p.retirement;
 assert.equal(retirement.collection.value.body.active,false,'collection retired');
 assert.equal(retirement.expectation.value.body.collecting,false,'expectation retired');
 assert.deepEqual(retirement.expectation.value.body.selectedReport,prefix.reportAssumption.source,
  'selected reports retained');
 assert.equal(retirement.expected,'accepted-finish-pattern/selected-reports-retained/no-ledger-fabrication');
 // live child capacity
 assert.equal(p.liveChild.sessionState.value.body.state,'running','live child still running');
 assert.equal(p.liveChild.expected,'no-capacity-release-for-nonterminal-children','capacity rule');
 // failed/blocked execution never completes
 assert.equal(p.failedExecution.runAfter.state,'blocked','failed execution keeps the run nonterminal');
 assert.deepEqual(p.failedExecution.runAfter.currentPlan,p.prestate.executionPlan,'failed outcome retains the plan');
 assert.equal(p.failedExecution.expected,'failed-execution-never-completes/recovery-revision-unclosed');
 // accounting/catalog
 assert.equal(p.accounting.terminalRecordPool,'child-control-retained-bytes/8MiB-run-64MiB-workspace');
 assert.equal(p.accounting.planSourcePool,'child-source-retained-bytes/charged-at-creation/unchanged-here');
 assert.equal(p.catalog.candidate,63);assert.equal(p.catalog.expected,'no-new-families-or-kinds/new-rows-not-names');
 // effect labels
 same(p.effects.map(x=>x.id),['turn-gate','coordination','plan-completion','terminal-transition','selector',
  'terminal-mapping','live-child','failed-execution'],'effect label set');
 const rows=[p.turn.row,p.coordination.row,p.completion.row,p.terminal.runAfter,p.terminal.selectorAfter,
  p.terminal.planAfter,p.retirement.collection,p.retirement.expectation,
  p.prestate.executionTaskControl,p.prestate.sessionState,p.liveChild.sessionState];
 const measurements=rows.map(r=>{validateTerminalWidths(r,size(r));return {table:r.table,key:r.key,...measureTerminalRow(r)};});
 same(p.measurements,measurements,'exact key plus committed occurrence projection');return measurements;}
// All facts below are supplied assertions. These functions do not establish
// credentials, complete inventory, actual outcome, currentness or source proof.
export function checkTerminalCompletion(f){
 if(f.trigger!=='coordination'&&f.trigger!=='recovery')throw new Error('unsupported completion trigger');
 if(f.stageContinueRequired)throw new Error('stage-cancelled plan never completes');
 assert.equal(f.grantRevoked,'supplied','grant revocation label');
 return f.state==='complete'?'terminal':'nonterminal';}
export function checkTerminalRetry(f){
 if(f.afterTerminal&&f.parentLevel)throw new Error('parent-level post-completion retry denied by the preserved authentication rules');
 assert.equal(f.sourceLevelIdempotence,true,'retained original outcome is the durable resolution');
 return clone(f.originalReceipt);}
