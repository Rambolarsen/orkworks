import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchTerminal,checkTerminalCompletion,checkTerminalRetry} from './research-terminal-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-terminal.py',import.meta.url))]);
test('generator supplies prestate, turn gate, coordination, completion, terminal transition, retirement and failure views',()=>{
 const p=JSON.parse(generate());assert.equal(p.effects?.length,8);assert.equal(p.catalog.candidate,63);
});
test('missing terminal packaging is rejected',()=>assert.throws(()=>validateResearchTerminal({})));

import {readFileSync} from 'node:fs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=readFileSync(new URL('research-terminal-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
const domain=(n,b)=>{const c=clone(b);return hash(n+'\n'+canonical(c));};
const origin2=label=>({storageEpoch:'f'.repeat(64),commitSequence:'9',changeDigest:hash('UNQUALIFIED diverged: '+label)});
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);
 assert.deepEqual(validateResearchTerminal(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change] of [
 ['terminal state reversal',p=>{p.terminal.runAfter.value.body.state='paused'},null],
 ['pause fields preserved as nonnull',p=>{p.terminal.runAfter.value.body.pauseReason='parent_ended'},null],
 ['second completion record',p=>{p.completion.row.value.receipt.runVersion=54},null],
 ['selector rewritten to the execution plan',p=>{p.terminal.selectorAfter.value.body.completedResearchPlan={planId:'execution-plan'}},'selector'],
 ['TerminalReportRunV1 cast',p=>{p.terminal.runAfter.value.body.profile='terminal-report-run-v1'},null],
 ['capacity released for the running child',p=>{p.liveChild.sessionState.value.body.state='ended'},null],
 ['failed execution completes the run',p=>{p.failedExecution.runAfter.state='complete'},null],
 ['turn evidence replaced by a report receipt',p=>{p.turn.body.evidence={kind:'codex_stop_receipt',runtime:p.prestate.sessionState.value.body.runtime}},'turn'],
 ['coordination outcome flipped to blocked',p=>{p.coordination.row.value.request.outcome='blocked'},'coordination'],
 ['unsupported trigger kind',p=>{p.completion.request.trigger.kind='scope-rejection'},null],
 ['grant label serialized as credential',p=>{p.terminal.grantRevocation.labels={grantExisted:'token',expected:'label-assertions-only/never-serialized/never-restored'}},null],
 ['collection stays active after completion',p=>{p.retirement.collection.value.body.active=true},'collection'],
]) {test(`mutation ${name} is rejected`,()=>{
 const p=packet();change(p);
 if(name==='turn evidence replaced by a report receipt'){const t=p.turn.body;delete t.turnDigest;t.turnDigest=domain('orkworks.research-source-turn-completion.v1',t);}
 assert.throws(()=>validateResearchTerminal(p),e=>{assert.ok(e instanceof Error);return true;});
});}
test('supplied-fact helpers separate completion admission from terminal retries',()=>{
 const p=packet();
 assert.equal(checkTerminalCompletion({trigger:'coordination',stageContinueRequired:false,
  taskStatus:'reported_complete',grantRevoked:'supplied',state:'complete'}),'terminal');
 assert.throws(()=>checkTerminalCompletion({trigger:'scope-rejection',stageContinueRequired:false,
  taskStatus:'reported_complete',grantRevoked:'supplied',state:'complete'}),/unsupported completion trigger/);
 assert.throws(()=>checkTerminalCompletion({trigger:'coordination',stageContinueRequired:true,
  taskStatus:'reported_complete',grantRevoked:'supplied',state:'complete'}),/stage-cancelled plan never completes/);
 assert.equal(checkTerminalCompletion({trigger:'coordination',stageContinueRequired:false,
  taskStatus:'running',grantRevoked:'supplied',state:'assessing'}),'nonterminal');
 assert.throws(()=>checkTerminalRetry({afterTerminal:true,parentLevel:true,sourceLevelIdempotence:true,
  originalReceipt:p.completion.row.value.receipt}),/parent-level post-completion retry denied/);
 const receipt=checkTerminalRetry({afterTerminal:true,parentLevel:false,sourceLevelIdempotence:true,
  originalReceipt:p.completion.row.value.receipt});
 assert.deepEqual(receipt,p.completion.row.value.receipt,'source-level idempotence returns the original outcome');
});
test('literal bounds, domains and catalog counts are pinned',()=>{
 const p=packet();
 assert.equal(p.catalog.candidate,63);
 assert.equal(p.accounting.terminalRecordPool,'child-control-retained-bytes/8MiB-run-64MiB-workspace');
 assert.equal(p.prestate.postLaunchStateLabel,'supplied-launch-chain-state-writer-remains-separately-gated');
});
