// Offline logical witnesses. No SQL, authority, native process or fence operations.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {validateEffectsState, validateEffectsTransition, validateArtifactOccurrences, inspectLogicalPreparationReceipt} from './root-preparation-effects.mjs';
const read = name => readFileSync(new URL(name, import.meta.url));
const catalog = JSON.parse(read('root-preparation-effects.json'));
const state = name => structuredClone(catalog.states.find(s => s.name === name));
const command = name => structuredClone(catalog.commands.find(c => c.afterState === name));
const caps = name => structuredClone(catalog.caps[name ?? 'fitting']);
const contexts = () => structuredClone(catalog.contexts);
const sha = x => createHash('sha256').update(x).digest('hex');
function canonical(x) {
  if (Array.isArray(x)) return '[' + x.map(canonical).join(',') + ']';
  if (x && typeof x === 'object') return '{' + Object.keys(x).sort().map(k => JSON.stringify(k) + ':' + canonical(x[k])).join(',') + '}';
  return JSON.stringify(x);
}
function changeRow(s, table, mutate, predicate = () => true) {
  const row = s.rows.find(r => r.table === table && predicate(JSON.parse(r.keyCanonical), JSON.parse(r.valueCanonical)));
  assert(row, 'test row exists');
  const value = JSON.parse(row.valueCanonical); mutate(value, row, s);
  row.valueCanonical = canonical(value);
}
const target = JSON.parse(catalog.contexts.target.draftCanonical);
const other = JSON.parse(catalog.contexts.other.draftCanonical);
const targetQuota = resource => key => key.resource === resource && key.owner.kind === 'run' && key.owner.runId === target.runId;
for (const s of catalog.states) test(`${s.name}: independent full-state bytes and commitments`, () => {
  const c = caps(s.name === 'target-never-initialized' ? 'summary-full' : 'fitting');
  assert.equal(validateEffectsState(s, contexts(), c), s.stateSha256);
});
for (const c of catalog.commands) test(`${c.afterState}: exact independent effect set and command charge`, () => {
  assert.deepEqual(validateEffectsTransition(state(c.beforeState), state(c.afterState), c, contexts(), caps(c.caps)), {
    classification: 'synthetic-logical-effects-only-not-runtime-admission', changeDigest: c.changeDigest,
    changedRows: c.changedRows, commandBytes: c.commandBytes});
});
test('exact source-catalog inputs are preserved', () => {
  assert.equal(sha(read('root-bootstrap-vectors.json')), catalog.inputs.bootstrapCatalogSha256);
  assert.equal(sha(read('root-preparation-join-vectors.json')), catalog.inputs.joinCatalogSha256);
});
for (const [name, table, mutate] of [
  ['artifact byte drift', 'preparation_artifacts', x => x.contentBase64 = Buffer.from('different').toString('base64')],
  ['artifact size drift', 'preparation_artifacts', x => x.sizeBytes = '0'],
  ['artifact counter spelling', 'preparation_artifacts', x => x.sizeBytes = '01'],
  ['artifact extra field', 'preparation_artifacts', x => x.objectStore = 'unreviewed'],
  ['artifact invalid base64', 'preparation_artifacts', x => x.contentBase64 = '!!!'],
  ['initial input digest drift', 'run_states', x => x.state.inputDigest = '0'.repeat(64)],
  ['initial state starts runtime', 'run_states', x => x.state.parentRuntime = {sidecarGeneration:'a'.repeat(64),launchGeneration:'b'.repeat(64)}],
  ['parent adopted as attached', 'parent_attachments', x => x.attachment.kind = 'attached'],
  ['parent extra generation', 'parent_attachments', x => x.attachment.launchGeneration = null],
  ['summary fabricated pass', 'summary_state', x => x.row.state.material.overall = 'meets-requirements'],
  ['source approval drift', 'root_preparations', x => x.approval.approvalId = 'other-approval'],
  ['source receipt link drift', 'root_preparations', x => x.receipt.commit.changeDigest = '0'.repeat(64)],
  ['shared ledger reset', 'quota_usage', x => x.used = '0'],
]) test(`state rejects ${name}`, () => {
  const s = state('target-admitted');
  const predicate = table === 'quota_usage' ? key => key.resource === 'preparation-retained-bytes' && key.owner.kind === 'workspace' : () => true;
  changeRow(s, table, mutate, predicate);
  assert.throws(() => validateEffectsState(s, contexts(), caps()));
});
for (const [name, mutate] of [
  ['missing artifact', s => s.rows.splice(s.rows.findIndex(r => r.table === 'preparation_artifacts'),1)],
  ['duplicate artifact key', s => s.rows.push(structuredClone(s.rows.find(r => r.table === 'preparation_artifacts')))],
  ['missing shared ledger', s => s.rows.splice(s.rows.findIndex(r => r.table === 'quota_usage'),1)],
  ['missing proof needed by unchanged artifact', s => s.rows.splice(s.rows.findIndex(r => r.table === 'commit_proof' && JSON.parse(r.keyCanonical) === '4'),1)],
  ['missing parent membership', s => s.rows.splice(s.rows.findIndex(r => r.table === 'parent_attachments'),1)],
  ['unlisted storage table', s => s.rows.push({table:'objects',keyCanonical:'{}',valueCanonical:'{}',rowDigest:'0'.repeat(64)})],
]) test(`state rejects ${name}`, () => {
  const s = state('target-admitted'); mutate(s);
  assert.throws(() => validateEffectsState(s, contexts(), caps()));
});
for (const [name, mutate] of [
  ['missing change', m => m.changes.pop()],
  ['extra change', m => m.changes.push({table:'root_drafts',key:'extra',beforeDigest:null,afterDigest:'0'.repeat(64)})],
  ['wrong before commitment', m => m.changes[0].beforeDigest = '0'.repeat(64)],
  ['wrong predecessor', m => m.predecessor.changeDigest = '0'.repeat(64)],
]) test(`transition rejects ${name} even after manifest digest repair`, () => {
  const c = command('target-admitted'), m = JSON.parse(c.manifestCanonical); mutate(m);
  c.manifestCanonical = canonical(m); c.changeDigest = sha('orkworks.hierarchy-storage-change.v3\n'+c.manifestCanonical);
  assert.throws(() => validateEffectsTransition(state(c.beforeState), state(c.afterState), c, contexts(), caps()));
});
for (const [name, mutate] of [
  ['understated bytes', c => c.commandBytes--],
  ['understated rows', c => c.changedRows--],
  ['wrong command kind', c => c.commandKind = 'root-draft-cancel'],
  ['wrong source owner', c => c.subject = 'other'],
  ['fabricated summary branch', c => c.summaryBranch = 'never-initialized'],
]) test(`transition rejects ${name}`, () => {
  const c = command('target-admitted'); mutate(c);
  assert.throws(() => validateEffectsTransition(state('target-draft'), state('target-admitted'), c, contexts(), caps()));
});
test('a coherent initialized state is rejected as the never-initialized command result', () => {
  const c = command('target-never-initialized');
  assert.throws(() => validateEffectsTransition(state(c.beforeState), state('target-admitted'), c, contexts(), caps(c.caps)));
});
test('summary capacity alone may select unavailable; command capacity cannot split admission', () => {
  const c = command('target-never-initialized');
  const limited = caps('summary-full'); limited.commandBytes = c.commandBytes - 1;
  assert.throws(() => validateEffectsTransition(state(c.beforeState), state(c.afterState), c, contexts(), limited));
});
test('missing workspace ledger cannot be treated as zero during preparation', () => {
  const c = command('target-draft'), before = state(c.beforeState);
  before.rows = before.rows.filter(r => !(r.table === 'quota_usage' && JSON.parse(r.keyCanonical).resource === 'preparation-retained-bytes'));
  assert.throws(() => validateEffectsTransition(before, state(c.afterState), c, contexts(), caps()));
});
test('unrelated source/membership/artifact values survive every target branch byte-for-byte', () => {
  const before = state('target-draft');
  const own = r => {
    const v = JSON.parse(r.valueCanonical), k = JSON.parse(r.keyCanonical);
    return (r.table === 'root_preparations' && v.bootstrap.runId === other.runId)
      || (r.table === 'run_states' && v.state.runId === other.runId)
      || (r.table === 'parent_attachments' && v.attachment.runId === other.runId)
      || (r.table === 'preparation_artifacts' && k.runId === other.runId)
      || (r.table === 'quota_usage' && k.owner.kind === 'run' && k.owner.runId === other.runId);
  };
  assert(before.rows.filter(own).length > 0);
  for (const name of ['target-admitted','target-never-initialized','target-cancelled'])
    assert.deepEqual(state(name).rows.filter(own),before.rows.filter(own));
});
test('draft-to-source transfer preserves artifact bytes and original proof links', () => {
  const filter = s => s.rows.filter(r => r.table === 'preparation_artifacts' && JSON.parse(r.keyCanonical).runId === target.runId);
  assert.deepEqual(filter(state('target-admitted')), filter(state('target-draft')));
  const refs = filter(state('target-draft')).map(r => JSON.parse(r.valueCanonical).contentDigest);
  assert(new Set(refs).size < refs.length, 'fixture contains separately charged equal-content occurrences');
});
test('cancel releases only the unadmitted target run while preserving burned-identity obligations', () => {
  const after = state('target-cancelled');
  assert(!after.rows.some(r => r.table === 'quota_usage' && targetQuota('preparation-retained-bytes')(JSON.parse(r.keyCanonical))));
  assert(!after.rows.some(r => r.table === 'root_drafts'));
  assert(after.rows.some(r => r.table === 'root_preparations'));
  assert.equal(catalog.unverified.includes('durable admission fence and native revocation ordering'), true);
});
test('logical receipt inspection returns original bytes and does not alter state', () => {
  const s = state('target-admitted'), before = canonical(s);
  const receipt = inspectLogicalPreparationReceipt(s,'target',contexts(),caps());
  const row = s.rows.find(r => r.table === 'root_preparations' && JSON.parse(r.valueCanonical).bootstrap.runId === target.runId);
  assert.equal(receipt, canonical(JSON.parse(row.valueCanonical).receipt));
  assert.equal(canonical(s),before);
  assert.throws(() => inspectLogicalPreparationReceipt(state('target-cancelled'),'target',contexts(),caps()));
});
test('artifact component counts binary bytes per reference, including equal content twice', () => {
  const raw=Buffer.alloc(524288,255),digest=sha(raw),owner={workspaceId:'workspace',runId:'run'};
  const refs=['a','b'].map(id=>({id,kind:'fixture',sourceReference:'synthetic:'+id,contentDigest:digest}));
  const rows=refs.map(r=>({table:'preparation_artifacts',key:{...owner,referenceId:r.id},value:{schemaVersion:1,contentBase64:raw.toString('base64'),sizeBytes:String(raw.length),contentDigest:digest,
    lastCommit:{storageEpoch:'a'.repeat(64),commitSequence:'1',changeDigest:'b'.repeat(64)}}}));
  assert.equal(validateArtifactOccurrences(rows,refs,owner),1048576);
  const extra={...refs[0],id:'c'};
  assert.throws(()=>validateArtifactOccurrences([...rows,{...structuredClone(rows[0]),key:{...owner,referenceId:'c'}}],[...refs,extra],owner));
});


// Repair the latest commitment graph so semantic corruptions cannot be rejected
// merely because a stale outer fingerprint happens to differ.
function repairLatest(s){
  const all=s.rows.map(r=>({table:r.table,key:JSON.parse(r.keyCanonical),value:JSON.parse(r.valueCanonical)}));
  const project=r=>{const v=structuredClone(r.value);if(!['store_header','commit_proof'].includes(r.table))delete v.lastCommit;
    if(r.table==='root_preparations')delete v.receipt.commit;
    if(r.table==='summary_state'&&v.checkpoint)delete v.checkpoint.applied;return v;};
  const rowDigest=r=>sha('orkworks.hierarchy-storage-row.v3\n'+canonical({table:r.table,key:r.key,body:project(r)}));
  const header=all.find(r=>r.table==='store_header'),seq=header.value.latest.commitSequence;
  const proof=all.find(r=>r.table==='commit_proof'&&r.key===seq);
  for(const change of proof.value.body.changes)if(change.afterDigest!==null){
    const r=all.find(r=>r.table===change.table&&canonical(r.key)===canonical(change.key));
    assert(r);change.afterDigest=rowDigest(r);
  }
  const digest=sha('orkworks.hierarchy-storage-change.v3\n'+canonical(proof.value.body));
  proof.value.changeDigest=digest;
  const update=link=>{if(link?.commitSequence===seq)link.changeDigest=digest;};
  update(header.value.latest);
  for(const r of all){update(r.value.lastCommit);if(r.table==='root_preparations')update(r.value.receipt.commit);
    if(r.table==='summary_state'&&r.value.checkpoint)update(r.value.checkpoint.applied);}
  for(const [i,r] of all.entries())Object.assign(s.rows[i],{keyCanonical:canonical(r.key),valueCanonical:canonical(r.value),rowDigest:rowDigest(r)});
  s.stateSha256=sha(canonical(s.rows));return s;
}
for(const [name,table,mutate,predicate,message] of [
  ['fabricated initial pass','summary_state',v=>v.row.state.material.overall='meets-requirements',k=>k.runId===target.runId,/initial summary exact bytes/],
  ['lost initial manifest identity','run_states',v=>v.initialInput.runId='other',k=>k.runId===target.runId,/initial input exact source binding/],
  ['started parent runtime','run_states',v=>v.state.parentRuntime={sidecarGeneration:'a'.repeat(64),launchGeneration:'b'.repeat(64)},k=>k.runId===target.runId,/initial state exact membership/],
  ['changed shared usage','quota_usage',v=>v.used='0',k=>k.owner.kind==='workspace'&&k.resource==='preparation-retained-bytes',/reconciled shared ledger/],
])test(`self-consistent commitment graph still rejects ${name}`,()=>{
  const s=state('target-admitted');changeRow(s,table,mutate,predicate);repairLatest(s);
  assert.throws(()=>validateEffectsState(s,contexts(),caps()),message);
});
test('self-consistent replaced source identity still fails original frozen bytes',()=>{
  const s=state('target-admitted');
  changeRow(s,'root_preparations',v=>{
    v.bootstrap.bootstrapId='substituted-bootstrap';
    const seal=(x,field,domain)=>{const body={...x};delete body[field];x[field]=sha(domain+'\n'+canonical(body));};
    seal(v.bootstrap,'bootstrapConfigurationDigest','orkworks.orchestrator-bootstrap.v2');
    v.runDefinition.bootstrapConfigurationDigest=v.bootstrap.bootstrapConfigurationDigest;
    seal(v.runDefinition,'runDefinitionDigest','orkworks.orchestration-run.v1');
    const d={...structuredClone(target),bootstrap:v.bootstrap,runDefinition:v.runDefinition};
    seal(d,'draftDigest','orkworks.root-preparation-draft.v1');v.approval.draftDigest=d.draftDigest;
    const a={...JSON.parse(catalog.contexts.target.approvalCanonical),draftDigest:d.draftDigest};
    const requestDigest=sha('orkworks.root-preparation-request.v1\n'+canonical(a));v.approval.requestDigest=requestDigest;
    Object.assign(v.receipt,{draftDigest:d.draftDigest,requestDigest,bootstrapConfigurationDigest:v.bootstrap.bootstrapConfigurationDigest,runDefinitionDigest:v.runDefinition.runDefinitionDigest});
    Object.assign(v.receipt.rootAssignmentIdentity,{bootstrapId:v.bootstrap.bootstrapId,bootstrapConfigurationDigest:v.bootstrap.bootstrapConfigurationDigest});
  },k=>k.runId===target.runId);
  repairLatest(s);
  assert.throws(()=>validateEffectsState(s,contexts(),caps()),/frozen draft bytes/);
});
test('a fully committed never-initialized marker cannot be selected when all summary capacities fit',()=>{
  const c=command('target-never-initialized');
  assert.throws(()=>validateEffectsTransition(state(c.beforeState),state(c.afterState),c,contexts(),caps()),/capacity-derived summary branch/);
});
test('complete command accepts its exact measured ceiling and rejects one byte less',()=>{
  const c=command('target-admitted'),limit=caps();limit.commandBytes=c.commandBytes;
  assert.doesNotThrow(()=>validateEffectsTransition(state(c.beforeState),state(c.afterState),c,contexts(),limit));
  limit.commandBytes--;
  assert.throws(()=>validateEffectsTransition(state(c.beforeState),state(c.afterState),c,contexts(),limit),/whole command capacity/);
});
test('encoded artifact value bound includes complete JSON and links',()=>{
  const owner={workspaceId:'workspace',runId:'run'},raw=Buffer.alloc(1),digest=sha(raw);
  const refs=[{id:'a',kind:'fixture',sourceReference:'synthetic:a',contentDigest:digest}];
  const r={table:'preparation_artifacts',key:{...owner,referenceId:'a'},value:{schemaVersion:1,contentBase64:'A'.repeat(2097152),sizeBytes:'1',contentDigest:digest,
    lastCommit:{storageEpoch:'a'.repeat(64),commitSequence:'1',changeDigest:'b'.repeat(64)}}};
  assert.throws(()=>validateArtifactOccurrences([r],refs,owner),/artifact encoded row ceiling/);
});


test('self-consistent latest proof cannot name a fabricated retained predecessor digest',()=>{
  const s=state('target-admitted');
  changeRow(s,'commit_proof',v=>v.body.predecessor.changeDigest='0'.repeat(64),key=>key==='5');
  repairLatest(s);
  assert.throws(()=>validateEffectsState(s,contexts(),caps()),/retained predecessor proof/);
});
