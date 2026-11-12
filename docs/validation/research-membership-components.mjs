// Offline finite reference/effect comparisons only. No store/reducer/publisher.
import assert from 'node:assert/strict';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {measureDiscoveryTrace} from './child-membership-components.mjs';
const same=(a,b,label='exact research membership component')=>assert.deepEqual(a,b,label);
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const size=x=>Buffer.byteLength(canonical(x));
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const order=(a,b)=>Buffer.compare(Buffer.from(canonical(a)),Buffer.from(canonical(b)));
const unique=xs=>[...new Map(xs.map(x=>[canonical(x),x])).values()].sort(order);
const meta=(table,key)=>({kind:'metadata',table,key});
const object=(kind,identity,digest)=>({kind:'object',reference:{key:{kind,identity},digest}});
const proof=commit=>({kind:'commit-proof',commit});
const inputKey=x=>pick(x,'workspaceId runId preparationRevision inputRevision');
const runtime=x=>meta('research_runtime_attachments',pick(x,'kind workspaceId runId sessionId sidecarGeneration launchGeneration'));
const entry=(id,consumer,targets)=>({id,consumer,targets:unique(targets)});
function reverse(entries){
  const out=new Map();
  for(const e of entries)for(const target of e.targets){const k=canonical(target);if(!out.has(k))out.set(k,{target,consumers:[]});out.get(k).consumers.push(e.consumer);}
  return [...out.values()].map(x=>({target:x.target,consumers:unique(x.consumers)})).sort((a,b)=>order(a.target,b.target));
}
function environment(p){
  validateResearchPrefix(p);
  const ch=p.chain,s=ch.startup,a=ch.assignment,run=s.context.run,t=s.scope.task,rk=pick(run,'workspaceId runId');
  const root=meta('root_preparations',rk),seed=meta('run_states',rk);
  const plan=object('research-source-plan',a.plan,ch.plan.planDefinitionDigest),cfg=object('research-source-configuration',a.configuration,s.configuration.configurationDigest),scope=object('research-source-scope',a.scope,s.scope.scopeDigest);
  const caps=s.captures.map(c=>object('research-source-capture',{task:t,...pick(c,'sourceId captureDigest')},c.captureDigest));
  const evidence=s.evidenceObjects.map(e=>object('research-source-evidence',{task:t,...pick(e,'referenceId contentDigest')},domain('orkworks.research-source-evidence.v1',e)));
  const assignment=object('research-assignment-source',a.assignmentIdentity,ch.fingerprints.assignmentSourceDigest);
  const report=object('research-report',p.reportAssumption.source.reportIdentity,p.reportAssumption.report.contentDigest);
  const approval=meta('research_plan_approvals',{plan:a.plan,approvalId:a.approval.approvalId});
  const reservation=meta('research_reservations',t),allocation=meta('research_allocations',a.reservation),attachment=meta('research_attachments',a.assignmentIdentity.childSessionId),task=meta('research_task_controls',t);
  const [parent,child]=p.runtimeHistory.map(r=>runtime(r.key));
  const input=b=>meta('research_control_inputs',inputKey(b));
  return {ch,s,a,run,t,rk,root,seed,plan,cfg,scope,caps,evidence,assignment,report,approval,reservation,allocation,attachment,task,parent,child,input};
}
function sourceEntries(p,e){
  const {ch,s,root,plan,cfg,scope,caps,evidence,assignment,report,approval,reservation,allocation,attachment,child,input}=e;
  return [entry('plan',plan,[root,cfg,scope,input(p.inputs[0].value.body)]),entry('configuration',cfg,[root,plan,scope,...evidence]),entry('scope',scope,caps),
    ...s.captures.map((c,i)=>entry('capture-'+i,caps[i],[root,plan,cfg,scope])),
    ...s.evidenceObjects.map((v,i)=>entry('evidence-'+i,evidence[i],[cfg])),
    entry('assignment',assignment,[root,plan,cfg,scope,approval,reservation,allocation,attachment]),entry('report',report,[assignment,scope,child])];
}
function historyEntries(p,e){
  const {ch,run,root,plan,cfg,scope,caps,assignment,report,approval,reservation,allocation,attachment,parent,input}=e;
  const entries=[];
  for(const r of ch.metadata){
    let targets;
    switch(r.table){
      case 'research_plan_approvals':targets=[plan,proof(r.value.receipt.commit)];break;
      case 'research_reservations':targets=[plan,cfg,scope,approval];break;
      case 'research_allocations':targets=[reservation,plan];break;
      case 'research_attachments':targets=[reservation,allocation,assignment];break;
      default:assert.fail('closed source metadata recipe');
    }
    entries.push(entry(r.table,meta(r.table,r.key),[...targets,proof(r.value.lastCommit)]));
  }
  for(const r of p.inputs){
    const b=r.value.body,targets=[root,runtime(r.value.producer),proof(r.value.lastCommit)];
    for(const record of b.records){
      switch(record.kind){
        case 'configuration':targets.push(cfg);break;
        case 'research-scope':targets.push(scope);break;
        case 'research-capture':targets.push(caps.find(c=>canonical(c.reference.key.identity)===canonical(record.reference)));break;
        case 'research-report':targets.push(report);break;
        default:assert.fail('closed research input recipe');
      }
    }
    if(b.decision.inputRevision<b.inputRevision)targets.push(input(p.inputs[0].value.body),parent);
    entries.push(entry('input-'+b.inputRevision,input(b),targets));
  }
  for(const r of p.runtimeHistory){
    const targets=[root,proof(r.value.lastCommit)];
    if(r.key.kind==='parent')targets.push(meta('parent_attachments',run.parentSessionId));else targets.push(assignment,attachment,reservation);
    entries.push(entry('runtime-'+r.key.kind,runtime(r.key),targets));
  }
  return entries;
}
function registryComponents(p,e){
  const {a,report,assignment}=e;
  const before=p.views.find(v=>v.id==='before-selection'),tv=before.rows.find(r=>r.table==='research_task_controls').value.body.taskVersion;
  const stream=pick(a.assignmentIdentity,'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest');
  const request={schemaVersion:1,operation:'submitResearchReport',stream,expectedTaskVersion:tv,expectedRunVersion:null,expectedReportHead:null,report:p.reportAssumption.report};
  const headRow={table:'report_heads',key:stream,value:{schemaVersion:1,head:p.reportAssumption.head,lastCommit:p.context.unqualifiedCommit}};
  const receiptRow={table:'report_receipts',key:{stream,reportVersion:1},value:{schemaVersion:1,...pick(request,'operation expectedTaskVersion expectedRunVersion expectedReportHead'),requestDigest:domain('orkworks.research-report-request.v1',request),response:{schemaVersion:1,acceptedHead:p.reportAssumption.head},lastCommit:p.context.unqualifiedCommit}};
  assert(size(request)<=32768,'normalized request 32 KiB');assert(size(headRow.value)<=512,'complete head value 512 bytes');assert(size({key:receiptRow.key,value:receiptRow.value})<=2048,'complete receipt 2 KiB');
  return {components:{request,headRow,receiptRow},entries:[headRow,receiptRow].map(r=>entry(r.table,meta(r.table,r.key),[report,assignment,proof(r.value.lastCommit)]))};
}
function viewEntries(v,e){
  const {root,seed,plan,cfg,scope,report,approval,reservation,allocation,attachment,assignment,task,parent,child,rk,input}=e;
  return v.rows.map(r=>{
    const b=r.value.body,targets=[proof(r.value.lastCommit)];
    switch(r.table){
      case 'research_run_controls':
        targets.push(root,b.currentInput.inputRevision===1?seed:input(b.currentInput));
        if(b.planReferences.length)targets.push(plan);if(b.parentRuntime!==null)targets.push(parent);break;
      case 'research_plan_controls':
        targets.push(plan,cfg,task);if(b.latestApproval!==null)targets.push(approval);break;
      case 'research_task_controls':
        targets.push(plan,cfg);if(b.reservation!==null)targets.push(reservation);break;
      case 'research_launch_controls':targets.push(plan,cfg,reservation,approval,parent,allocation,attachment,child);break;
      case 'research_session_states':
        targets.push(reservation,attachment,assignment);if(b.runtime!==null)targets.push(child);break;
      case 'research_collections':targets.push(meta('research_run_controls',rk),task);break;
      case 'research_expectations':
        targets.push(meta('research_collections',rk),cfg,scope);if(b.reservation!==null)targets.push(reservation);if(b.selectedReport!==null)targets.push(report);break;
      default:assert.fail('closed view membership recipe');
    }
    return entry(r.table,meta(r.table,r.key),targets);
  });
}
function comparison(p,e){
  const rows=id=>p.views.find(v=>v.id===id).rows;
  const before=rows('before-selection'),after=rows('selected-source'),changes=[],unchanged=[];
  for(const old of before){
    const next=after.find(r=>r.table===old.table&&canonical(r.key)===canonical(old.key));assert(next);
    const location=meta(old.table,old.key);
    if(canonical(old.value)===canonical(next.value))unchanged.push(location);else changes.push({location,before:old.value,after:next.value});
  }
  changes.push({location:e.input(p.inputs[1].value.body),before:null,after:p.inputs[1].value});
  return {beforeView:'before-selection',afterView:'selected-source',changes,unchanged};
}
export function validateResearchMembership(p,prefix){
  exact(p,'classification prefixFingerprint groups registryComponents standaloneSourceExample effectComparison discoveryAssumption discoveryTrace prospectiveOwnerAssumption');
  same(p.classification,'conditional-research-membership-components-only');const e=environment(prefix);
  same(p.prefixFingerprint,hash(canonical(prefix)),'unchanged prefix fingerprint');
  const registry=registryComponents(prefix,e);same(p.registryComponents,registry.components,'synthetic original registry join');
  const expected=[{id:'sources',entries:sourceEntries(prefix,e)},{id:'history',entries:historyEntries(prefix,e)},
    {id:'registry',entries:registry.entries},...prefix.views.map(v=>({id:v.id,entries:viewEntries(v,e)}))];
  same(p.groups.length,expected.length);let entries=0,occurrences=0;
  for(const [i,g] of p.groups.entries()){
    exact(g,'id entries reverse');same(g.id,expected[i].id);same(g.entries,expected[i].entries,'exact forward membership');
    same(g.reverse,reverse(expected[i].entries),'exact reverse membership');entries+=g.entries.length;occurrences+=g.entries.reduce((n,x)=>n+x.targets.length,0);
  }
  same(p.standaloneSourceExample,entry('separate-evidence-source-example',object('assignment-evidence-source',prefix.reportAssumption.source,prefix.reportAssumption.sourceKey),[e.report]),'separate source example');
  same(p.effectComparison,comparison(prefix,e),'local value comparison');
  same(p.prospectiveOwnerAssumption,[{collection:e.rk,task:e.t}],'supplied one-local-collection sample');
  const discovery=measureDiscoveryTrace(p.discoveryTrace,p.discoveryAssumption);
  return {classification:'conditional-research-membership-components-only',entries,occurrences,localChanges:p.effectComparison.changes.length,
    forwardBytes:expected.reduce((n,g)=>n+size(g.entries),0),comparisonBytes:size(p.effectComparison),discovery};
}
