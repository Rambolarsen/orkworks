// Decoded finite documentation samples and supplied-fact oracles only.
// No wire decoder, publisher, credential validator, runtime reducer or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateResearchStartup} from './research-startup-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const startupRaw=readFileSync(new URL('research-startup-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw),startup=JSON.parse(startupRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),digest=(n,x)=>hash(n+'\n'+canonical(x));
const fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const chain=prefix.chain,scope=chain.startup.scope,cfg=chain.startup.configuration;
const runKey=fields(chain.startup.context.run,'workspaceId runId');
const researcherTask=clone(scope.task);
const reviewerTask={...researcherTask,taskId:'review-task'};
const report=clone(prefix.reportAssumption.report),reportSource=clone(prefix.reportAssumption.source);
const configRef=fields(chain.plan.task.configuration,'configurationId configurationDigest');
const planRef=clone(chain.assignment.plan);
const scopeRef={task:clone(researcherTask),scopeDigest:scope.scopeDigest};
const caps={objects:2097152,research_reviews:32768,research_summaries:16384,research_expectations:16384};
export function measureReviewRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function validateReviewWidths(r,jobBytes){
 assert(Object.hasOwn(caps,r.table),'supported family');
 if(r.table==='research_reviews')assert(size(r.value.receipt)<=4096,'receipt limit');
 if(r.table==='research_summaries')assert(size(r.value.body)<=8192,'summary body limit');
 if(r.table==='objects')assert(r.value.body.task.taskId==='review-task','snapshot task identity');
 assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');}
export function validateResearchReview(p){
 exact(p,'prefixSha256 startupSha256 assumptions twoTaskPlan reviewerConfiguration collectionShapes reviewInput reviewAdmission staleReview summaryCitation effects measurements');
 assert.equal(p.prefixSha256,hash(prefixRaw));assert.equal(p.startupSha256,hash(startupRaw));
 validateResearchPrefix(prefix);validateResearchStartup(startup);
 same(p.assumptions,[
  'separate decoded examples, not a store or legal event history',
  'reviewer child authority, generation, lease, rate and current Main shown context supplied, never authenticated here',
  'actual capture origin, UI approval, reservation/allocation/attachment/runtime authority and complete publication unqualified',
  'complete source/consumer/index memberships, original H/C/proofs, ledgers and actual effects unqualified'],'qualification boundary');
 // two-task plan: accepted optional reviewTask, batches, digest domain
 const plan=clone(p.twoTaskPlan.body);
 assert.equal(plan.kind,'single-task-retained-file-research');
 assert.equal(plan.reviewTask.taskId,'review-task');
 assert.deepEqual(plan.reviewTask.scope,scopeRef,'reviewer consumes the researcher task scope');
 assert.deepEqual(plan.reviewTask.dependsOn,researcherTask,'dependency is the researcher TaskKey');
 assert.deepEqual(plan.task,clone(chain.plan.task),'researcher task unchanged');
 assert.deepEqual(plan.batches,[['prefix-task'],['review-task']],'dependency-ordered batches');
 assert.equal(plan.maxParallelChildren,1);
 assert.deepEqual(plan.reviewTask.group,{groupId:'review-group',branchName:'orkworks/review-group',
  absolutePath:{base64:'L3Jldmlldy9ncm91cA==',encoding:'unix-bytes'},baseCommit:cfg.assignment.baseCommit},'independent group');
 assert.equal(p.twoTaskPlan.domain,'orkworks.research-source-plan.v1');
 const planForHash=clone(plan);delete planForHash.planDefinitionDigest;
 assert.equal(plan.planDefinitionDigest,digest('orkworks.research-source-plan.v1',planForHash),'plan digest excludes only planDefinitionDigest');
 // ordering denial: the delivery entry carries no digest
 const bad=p.twoTaskPlan.orderingDenial.settings;
 assert(bad.deliveryOrder.some(d=>d.kind==='research-review-input'&&d.inputDigest),'denial embeds a digest');
 assert.equal(p.twoTaskPlan.orderingDenial.expected,'reject-config-embedding-snapshot-digest/digest-free-entry-required');
 const ok=p.twoTaskPlan.orderingDenial.settingsOk;
 assert(ok.deliveryOrder.some(d=>d.kind==='research-review-input'&&!('inputDigest' in d)),'accepted entry is digest-free');
 assert.deepEqual(p.reviewerConfiguration.settingsDeliveryOrder,ok.deliveryOrder,'packet carries the accepted order');
 // reviewer configuration: accepted v3 specialization with recomputed digests
 const rc=p.reviewerConfiguration.body;
 assert.equal(rc.taskId,'review-task');assert.equal(rc.configurationId,'review-config');
 assert.deepEqual(rc.permissions.requested.tools,rc.permissions.effective.tools);
 assert.deepEqual(rc.permissions.effective.tools,['orkworks.research-review.context.read.v1',
  'orkworks.research-review.receipt.read.v1','orkworks.research-review.submit.v1'],'sorted review tool set');
 assert.deepEqual(rc.assignment.dependencies,[researcherTask],'full-TaskKey dependency entry');
 assert.equal(rc.assignment.worktreeGroupId,'review-group');
 assert(rc.renderedInstructions.startsWith('ORKWORKS CHILD INSTRUCTIONS v1\n'),'unchanged framing header');
 assert.equal(rc.renderedInstructionsDigest,hash(rc.renderedInstructions),'instruction digest over rendered bytes');
 const resealed=clone(rc);const expected=rc.configurationDigest;delete resealed.configurationDigest;
 assert.equal(expected,digest('orkworks.assignment-configuration.v3',resealed),'configuration digest excludes only configurationDigest');
 assert.equal(rc.harness.effectiveSettingsDigest,hash(canonical(ok)),'settings digest binds the digest-free delivery entry');
 assert(size(rc)<=1048576,'configuration bound');
 assert.equal(p.reviewerConfiguration.composedStartupLimit,262144,'startup bound retained, not a fit promise');
 // collection shapes
 assert.equal(p.collectionShapes.oneTask.reviewTask,null,'one-task shape');
 assert.equal(p.collectionShapes.twoTask.reviewTask.taskId,'review-task','two-task shape');
 assert.deepEqual(p.collectionShapes.expectations[0].value.body.selectedReport,reportSource,'prestate selected report');
 assert.equal(p.collectionShapes.expectations[0].value.body.collecting,true,'source-owned collecting state');
 // snapshot: exact target binding, questions, cited excerpts only
 const snap=p.reviewInput.snapshot.value.body;
 assert.equal(snap.task.taskId,'review-task');
 const snapForHash=clone(snap);delete snapForHash.inputDigest;
 assert.equal(snap.inputDigest,digest('orkworks.research-review-input.v1',snapForHash),'snapshot digest excludes only inputDigest');
 const expectedTarget={...reportSource,reportIdentity:{...reportSource.reportIdentity,reportVersion:2},
  sizeBytes:size(snap.report),contentDigest:snap.report.contentDigest};
 assert.deepEqual(snap.target,expectedTarget,'target equals the selected report evidence source');
 const reportForHash=clone(snap.report);delete reportForHash.contentDigest;
 assert.equal(snap.report.contentDigest,digest('orkworks.research-report.v1',reportForHash),'reviewed report resealed');
 assert.deepEqual(snap.scope,scopeRef,'approved scope reference');
 assert.deepEqual(snap.questions,clone(scope.questions),'complete approved question set delivered');
 assert(snap.citedExcerpts.length===1&&snap.citedExcerpts[0].reference.reference.id==='e-a','cited excerpts only');
 assert.equal(snap.citedExcerpts[0].contentDigest,snap.citedExcerpts[0].reference.reference.excerptDigest,'cited excerpt digest');
 assert(size({key:p.reviewInput.snapshot.key,value:p.reviewInput.snapshot.value.body})<=262144,'snapshot bound');
 assert.deepEqual(p.reviewInput.emptyCase.citedExcerpts,[],'empty-citedExcerpts case present');
 // outcome and family row
 const outcome=p.reviewAdmission.outcome;
 assert.deepEqual(outcome.questionStatuses.map(x=>x.questionId),scope.questions.map(q=>q.questionId),'one status per approved question');
 assert.equal(outcome.verdict,'consistent');
 const outcomeForHash=clone(outcome);delete outcomeForHash.reviewDigest;
 assert.equal(outcome.reviewDigest,digest('orkworks.research-review-outcome.v1',outcomeForHash),'outcome digest excludes only reviewDigest');
 assert(size(outcome)<=16384,'outcome body bound');
 const row=p.reviewAdmission.row;
 assert.equal(row.table,'research_reviews');assert.deepEqual(row.key,reviewerTask,'family keyed by reviewer TaskKey');
 assert.equal(row.value.request.review.reviewDigest,outcome.reviewDigest,'no duplicated wrapper body');
 assert.deepEqual(row.value.request.review,clone(outcome),'request carries the complete outcome');
 const requestForHash=clone(row.value.request);delete requestForHash.requestDigest;
 assert.equal(row.value.requestDigest,digest('orkworks.research-review-request.v1',requestForHash),'request digest excludes only requestDigest');
 assert.deepEqual(row.value.receipt,{schemaVersion:1,task:reviewerTask,reviewDigest:outcome.reviewDigest,
  acceptedTaskVersion:8,commit:row.value.receipt.commit},'receipt shape');
 assert(size(row.value.receipt)<=4096,'receipt bound');
 assert(size({key:row.key,value:row.value})<=caps.research_reviews,'wrapper bound');
 // summary citation: typed review evidence source
 const evidence=p.summaryCitation.evidence;
 assert.equal(evidence.sourceKind,'research-review');
 assert.deepEqual(evidence.reviewIdentity,{...runKey,taskId:'review-task',reservationId:'review-reservation',
  childSessionId:'review-child',configurationDigest:rc.configurationDigest,
  sidecarGeneration:outcome.sidecarGeneration,launchGeneration:outcome.launchGeneration},'review identity members');
 assert.equal(evidence.sizeBytes,size(outcome),'exact stored outcome body length');
 assert.equal(evidence.contentDigest,outcome.reviewDigest,'contentDigest equals reviewDigest');
 const summary=p.summaryCitation.row.value.body;
 assert.equal(summary.summaryVersion,3);
 assert.deepEqual(summary.researchPlan,planRef,'completed research plan ref');
 assert.deepEqual(summary.reviewReferences,[evidence],'typed review reference cited');
 assert.deepEqual(summary.reportReferences,[reportSource],'report references unchanged');
 assert.equal(p.summaryCitation.row.value.digest,digest('orkworks.research-summary.v1',summary),'summary digest');
 assert(size(summary)<=8192,'summary bound');
 // stale review and reviewer-only replacement
 assert.deepEqual(p.staleReview.expectationAfter.value.body.task,researcherTask,'expectation task unchanged');
 assert.deepEqual(p.staleReview.expectationAfter.value.body.selectedReport,p.reviewAdmission.row.value.request.review.reviewedReport,
  'expectation follows the corrected selection without reviewer-row mutation');
 assert.equal(p.staleReview.expectationAfter.value.body.expectationVersion,3,'coalesced selection version');
 const replacement=p.staleReview.replacementPlan;
 assert.equal(replacement.variant,'reviewer-only-replacement');
 assert(!('task' in replacement)||!replacement.task||replacement.task===undefined||replacement.taskId===undefined,'no researcher task member');
 assert.deepEqual(replacement.researchDependency.task,researcherTask,'original researcher TaskKey carried');
 assert.deepEqual(replacement.researchDependency.scope,scopeRef,'original scope carried');
 assert.deepEqual(replacement.researchDependency.configuration,configRef,'original configuration carried');
 assert.deepEqual(replacement.batches,[['review-task-2']],'reviewer-only batches');
 const replacementForHash=clone(replacement);delete replacementForHash.planDefinitionDigest;
 assert.equal(replacement.planDefinitionDigest,digest('orkworks.research-source-plan.v1',replacementForHash),'replacement digest excludes only planDefinitionDigest');
 assert.equal(p.staleReview.sameReportCase.expected,'replacement-for-unchanged-report/owner-disposition-only/no-automatic-rereview');
 // effect labels
 same(p.effects.map(x=>x.id),['plan-approval','reviewer-reservation','review-admission','review-publication',
  'conflict','report-correction','replacement','summary-citation'],'effect label set');
 const rows=[p.reviewInput.snapshot,p.reviewAdmission.row,p.summaryCitation.row,
  p.collectionShapes.expectations[0],p.staleReview.expectationAfter];
 const measurements=rows.map(r=>{validateReviewWidths(r,size(r));return {table:r.table,key:r.key,...measureReviewRow(r)};});
 same(p.measurements,measurements,'exact key plus committed occurrence projection');return measurements;}
// All facts below are supplied assertions. These functions do not establish
// credentials, complete inventory, actual outcome, currentness or source proof.
export function checkReviewAdmission(row,facts){
 assert.equal(facts.authority,'supplied-current-child','generation/lease/health/rate before lookup');
 assert.equal(facts.targetCurrent,true,'selected report identity/version/digest equals target');
 same(row.value.request.review.reviewedReport,facts.selectedReport,'stale target denied');
 assert.equal(row.value.receipt.acceptedTaskVersion,facts.taskVersion,'live CAS read, never assumed');
 return clone(row.value.receipt);}
export function checkSummaryReviewReference(summary,facts){
 if(summary.reviewReferences.length===0){assert.equal(facts.requiredReview,'proven-none','empty only with proven membership');return 'empty';}
 for(const ref of summary.reviewReferences){
  assert.equal(ref.sourceKind,'research-review','typed review source');
  assert.equal(ref.contentDigest,facts.reviewDigest,'bound to the exact outcome');
  assert.equal(facts.reviewCurrent,true,'currentness revalidated at publication');
  assert.notEqual(facts.verdict,'conflict','conflicting review never satisfies the required-review gate');}
 return 'cited';}
