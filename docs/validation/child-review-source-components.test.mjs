// Documentation-only child source vector assay. Never import into production.
// Synthetic identities/outcomes prove structural bytes only: no event, custody,
// native identity or eligibility evidence of any kind.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {
  assertReadBudgetCoverage, canonical, domain, makeContext, renderChildInstructions,
  retainedReviewInputWire, validateReviewTargetInput, verifyChildChain,
} from './child-review-source-components.mjs';

const sha = x => createHash('sha256').update(x).digest('hex');
const catalog = JSON.parse(readFileSync(new URL('./child-review-source-vectors.json',
  import.meta.url)));
const vectors = new Map(catalog.vectors.map(v => [v.id, v]));
const value = id => JSON.parse(vectors.get(id).canonical);
const DOMAIN_BY_ID = {
  'worker-approval-source': 'orkworks.child-source-plan-approval.v1',
  'reviewer-approval-source': 'orkworks.child-source-plan-approval.v1',
  'worker-assignment-source': 'orkworks.child-assignment-source.v1',
  'reviewer-assignment-source': 'orkworks.child-assignment-source.v1',
};

test('catalog shape, canonical wire bytes and digest pins', () => {
  assert.equal(catalog.schemaVersion, 1);
  assert.equal(catalog.classification, 'synthetic-unverified-source-bytes-only');
  assert.equal(catalog.vectors.length, 24);
  for (const v of catalog.vectors) {
    const parsed = JSON.parse(v.canonical);
    assert.equal(canonical(parsed), v.canonical, `${v.id} canonical wire bytes`);
    assert.equal(sha(v.canonical), v.sha256, `${v.id} byte digest pin`);
    const name = DOMAIN_BY_ID[v.id];
    if (!name) continue;
    assert.equal(domain(name, parsed), v.domainDigest, `${v.id} domain digest`);
  }
});

function workerChain(overrides = {}) {
  return {
    plan: value('worker-source-plan'), configuration: value('worker-configuration'),
    approvalRequest: value('worker-approval-request'),
    approvalSource: value('worker-approval-source'),
    receipt: value('worker-approval-receipt'), reservation: value('worker-reservation'),
    allocation: value('worker-allocation'), attachment: value('worker-attachment'),
    assignmentSource: value('worker-assignment-source'), ...overrides,
  };
}
function reviewerBase(overrides = {}) {
  const snapshot = value('reviewer-input-snapshot');
  const targetResult = value('target-result');
  return {
    plan: value('reviewer-source-plan'), configuration: value('reviewer-configuration'),
    approvalRequest: value('reviewer-approval-request'),
    approvalSource: value('reviewer-approval-source'),
    receipt: value('reviewer-approval-receipt'), reservation: value('reviewer-reservation'),
    allocation: value('reviewer-allocation'), attachment: value('reviewer-attachment'),
    assignmentSource: value('reviewer-assignment-source'),
    inputSnapshot: snapshot, targetConfiguration: value('worker-configuration'),
    targetResult,
    expectedTarget: {assignmentIdentity: targetResult.assignmentIdentity,
      resultDigest: snapshot.target.resultDigest}, ...overrides,
  };
}

test('worker retained-source chain', () => {
  const ctx = makeContext(catalog, 'worker');
  const result = verifyChildChain(workerChain(), ctx);
  assert.equal(result.configurationDigest, value('worker-configuration').configurationDigest);
  assert.equal(result.approvalRowDigest,
    domain('orkworks.child-source-plan-approval.v1', value('worker-approval-source')));
  assert.equal(result.reservationDigest, value('worker-reservation').reservationDigest);
  assert.equal(result.attachmentDigest, value('worker-attachment').attachmentDigest);
});

test('reviewer retained-source chain with input snapshot', () => {
  const ctx = makeContext(catalog, 'reviewer');
  const verified = verifyChildChain(reviewerBase(), ctx);
  assert.equal(verified.configurationDigest,
    value('reviewer-configuration').configurationDigest);
  const snapshot = value('reviewer-input-snapshot');
  assert.equal(verified.snapshotKeys.length, 1, 'single evidence source');
  assert.equal(verified.snapshotKeys[0],
    domain('orkworks.assignment-evidence-source.v1',
      snapshot.evidence[0].source), 'source key join');
});

test('framing and retained-input wire bytes', () => {
  const worker = value('worker-configuration');
  const reviewer = value('reviewer-configuration');
  const renderVector = id => JSON.parse(vectors.get(id).canonical);
  assert.equal(renderChildInstructions(worker), renderVector('worker-render'),
    'worker render bytes');
  assert.equal(renderChildInstructions(reviewer), renderVector('reviewer-render'),
    'reviewer render bytes');
  const snapshot = value('reviewer-input-snapshot');
  assert.equal(retainedReviewInputWire(snapshot),
    renderVector('reviewer-retained-wire'), 'retained input wire');
  const composed = Buffer.byteLength(reviewer.renderedInstructions +
    retainedReviewInputWire(snapshot));
  assert(composed <= 262144, 'composed reviewer startup ceiling');
  assert(renderChildInstructions(worker).startsWith(
    'ORKWORKS CHILD INSTRUCTIONS v1\n'), 'child render header');
});

test('candidate/result source join', () => {
  const result = value('target-result');
  const candidate = {schemaVersion: 1, assignmentIdentity: result.assignmentIdentity,
    predecessor: result.predecessor, authorityGeneration: result.authorityGeneration,
    outputs: result.outputs};
  assert.equal(result.candidateDigest,
    domain('orkworks.assignment-result-candidate.v1', candidate), 'candidate digest join');
  assert.equal(domain('orkworks.assignment-result.v1', result),
    value('reviewer-input-snapshot').target.resultDigest, 'result digest join');
});

function reseal(x, field, name) {
  const body = {...x};
  delete body[field];
  return {...x, [field]: domain(name, body)};
}
const resealConfiguration = c => {
  const rendered = renderChildInstructions(c);
  return reseal({...c, renderedInstructions: rendered,
    renderedInstructionsDigest: sha(rendered)}, 'configurationDigest',
  'orkworks.assignment-configuration.v3');
};
const resealSnapshot = s => reseal(s, 'inputDigest', 'orkworks.child-review-input.v1');

// Structural and join rejections. Mutated digest-sealed records reject through
// their own digest mismatch unless explicitly resealed; open records reject
// through recorded joins. None confers authority.
const NEGATIVES = [
  ['plan unknown field', () => workerChain({plan: {...value('worker-source-plan'), extra: 1}}),
    /closed fields/],
  ['plan multi-task batch', () => workerChain({plan: reseal(
    {...value('worker-source-plan'), batches: [['task-1', 'task-2']]},
    'planDefinitionDigest', 'orkworks.child-source-plan.v1')}), /closed arrays|single-task/],
  ['plan parallelism', () => workerChain({plan: reseal(
    {...value('worker-source-plan'), maxParallelChildren: 2}, 'planDefinitionDigest',
    'orkworks.child-source-plan.v1')}), /selected slice parallelism/],
  ['plan oversized scope text', () => workerChain({plan: reseal(
    {...value('worker-source-plan'), scopeText: 'x'.repeat(4097)},
    'planDefinitionDigest', 'orkworks.child-source-plan.v1')}), /bounded UTF-8/],
  ['plan dirty base commit', () => workerChain({plan: reseal(
    {...value('worker-source-plan'), task: {...value('worker-source-plan').task,
      group: {...value('worker-source-plan').task.group, baseCommit: 'b'.repeat(39)}}},
    'planDefinitionDigest', 'orkworks.child-source-plan.v1')}), /Git object base commit/],
  ['worker tools drift', () => workerChain({configuration: resealConfiguration(
    {...value('worker-configuration'), permissions: {...value('worker-configuration').permissions,
      requested: {...value('worker-configuration').permissions.requested,
        tools: ['orkworks.evaluation.submit.v1', 'orkworks.evaluation.artifact.read.v1',
          'orkworks.evaluation.receipt.read.v1', 'orkworks.result.receipt.read.v1',
          'orkworks.result.submit.v1']}}})}), /exact verification tool profile/],
  ['worker filesystem scope', () => workerChain({configuration: resealConfiguration(
    {...value('worker-configuration'), assignment: {...value('worker-configuration').assignment,
      worktreeScope: {readPaths: ['src/x.ts'], writePaths: []}}})}),
  /retained-data child has no live filesystem scope/],
  ['worker coordination actions', () => workerChain({configuration: resealConfiguration(
    {...value('worker-configuration'), permissions: {...value('worker-configuration').permissions,
      requested: {...value('worker-configuration').permissions.requested,
        coordinationActions: [{actionId: 'orkworks.coordination.plan.propose.v1',
          workspaceId: 'workspace-1', runId: 'a'.repeat(64) + '-1',
          parentSessionId: 'parent-1'}]}}})}), /retained-data access list/],
  ['worker review input', () => workerChain({configuration: resealConfiguration(
    {...value('worker-configuration'), reviewInput: {
      schemaVersion: 1, kind: 'accepted-child-result',
      target: {assignmentIdentity: value('worker-assignment-source').assignmentIdentity,
        resultRevision: 1, resultDigest: '0'.repeat(64)},
      outputDeclarationIds: [], evidenceSources: [],
      evaluationOutputDeclarationId: 'evaluation', maxReadBytes: 1}})}), /worker review input/],
  ['native delegation attempt', () => workerChain({configuration: resealConfiguration(
    {...value('worker-configuration'), permissions: {...value('worker-configuration').permissions,
      requested: {...value('worker-configuration').permissions.requested,
        nativeDelegation: 'allowed'}}})}), /child native delegation denied/],
  ['allocation group drift', () => workerChain({allocation: reseal(
    {...value('worker-allocation'), group: {...value('worker-allocation').group,
      groupId: 'group-foreign'}}, 'allocationDigest',
    'orkworks.child-source-allocation.v1')}), /allocation planned group join/],
  ['primary worktree substitution', () => workerChain({allocation: reseal(
    {...value('worker-allocation'), allocatedWorktree:
      value('worker-configuration').sourceWorktreeBinding}, 'allocationDigest',
    'orkworks.child-source-allocation.v1')}), /worktree allocation kind/],
  ['attachment identity reservation drift', () => workerChain({attachment: reseal(
    {...value('worker-attachment'), assignmentIdentity:
      {...value('worker-attachment').assignmentIdentity, reservationId: 'reservation-x'}},
    'attachmentDigest', 'orkworks.child-source-attachment.v1')}),
  /identity reservation join/],
  ['frozen task version drift', () => {
    const reservation = reseal({...value('worker-reservation'), assignmentTaskVersion: 2},
      'reservationDigest', 'orkworks.child-source-reservation.v1');
    const reservationRef2 = {task: reservation.task,
      reservationId: reservation.reservationId,
      reservationDigest: reservation.reservationDigest};
    const allocation = reseal({...value('worker-allocation'),
      reservation: reservationRef2}, 'allocationDigest',
    'orkworks.child-source-allocation.v1');
    const allocationRef2 = {reservation: reservationRef2,
      allocationId: allocation.allocationId,
      allocationDigest: allocation.allocationDigest};
    const attachment = reseal({...value('worker-attachment'),
      reservation: reservationRef2, allocation: allocationRef2}, 'attachmentDigest',
    'orkworks.child-source-attachment.v1');
    return workerChain({reservation, allocation, attachment,
      assignmentSource: {...value('worker-assignment-source'),
        reservation: reservationRef2, allocation: allocationRef2}});
  }, /frozen task version join/],
  ['changed approval retry bytes', () => workerChain({approvalSource:
    {...value('worker-approval-source'), requestDigest: '0'.repeat(64)}}),
  /approval request digest join/],
  ['attachment allocation digest drift', () => workerChain({attachment: reseal(
    {...value('worker-attachment'),
      allocation: {...value('worker-attachment').allocation,
        allocationDigest: '0'.repeat(64)}}, 'attachmentDigest',
    'orkworks.child-source-attachment.v1')}), /attachment allocation reference join/],
  ['attachment identity drift', () => workerChain({attachment: reseal(
    {...value('worker-attachment'),
      assignmentIdentity: {...value('worker-assignment-source').assignmentIdentity,
        childSessionId: 'child-9'}}, 'attachmentDigest',
    'orkworks.child-source-attachment.v1')}), /attachment identity/],
  ['snapshot unknown field', () => reviewerBase({inputSnapshot:
    {...value('reviewer-input-snapshot'), bearer: 'not-a-grant'}}), /closed fields/],
  ['assignment source approval drift', () => workerChain({assignmentSource: {
    ...value('worker-assignment-source'),
    approval: {...value('worker-assignment-source').approval,
      approvalDigest: '0'.repeat(64)}}}), /assignment source approval join/],
  ['plan prompt drift', () => workerChain({plan: reseal(
    {...value('worker-source-plan'),
      task: {...value('worker-source-plan').task,
        initialPrompt: 'Unrelated prompt bytes.'}}, 'planDefinitionDigest',
    'orkworks.child-source-plan.v1')}), /plan initial prompt join/],
  ['snapshot result unbound to reviewed target', () => reviewerBase(
    {inputSnapshot:
      {...value('reviewer-input-snapshot'),
        result: {...value('reviewer-input-snapshot').result, resultRevision: 2}}}),
  /snapshot retained result join/],
  ['snapshot credentials excluded', () => reviewerBase({inputSnapshot:
    {...value('reviewer-input-snapshot'), bearer: 'not-a-grant'}}), /closed fields/],
  ['reservation approval digest drift', () => workerChain({reservation: reseal(
    {...value('worker-reservation'),
      approval: {...value('worker-reservation').approval, approvalDigest: '0'.repeat(64)}},
    'reservationDigest', 'orkworks.child-source-reservation.v1')}),
  /reservation approval join/],
  ['snapshot task key drift', () => reviewerBase({inputSnapshot:
    {...value('reviewer-input-snapshot'), task: {...value('reviewer-input-snapshot').task,
      taskId: 'task-9'}}}), /snapshot task join/],
];
for (const [name, build, expected] of NEGATIVES) {
  test(`rejects: ${name}`, () => {
    const chain = build();
    const role = chain.inputSnapshot || (chain.configuration &&
      chain.configuration.configurationId === 'configuration-reviewer-1') ?
      'reviewer' : 'worker';
    assert.throws(() => verifyChildChain(chain, makeContext(catalog, role)), expected);
  });
}

test('rejects stale plan definition bytes in the retained chain', () => {
  const resealDomain = (x, field, domainName) => {
    const body = {...x};
    delete body[field];
    return {...x, [field]: domain(domainName, body)};
  };
  const plan = value('reviewer-source-plan');
  const drifted = {...plan, task: {...plan.task,
    input: {...plan.task.input, inputDigest: 'f'.repeat(64)}}};
  const resealedPlan = resealDomain(drifted, 'planDefinitionDigest',
    'orkworks.child-source-plan.v1');
  const chain = reviewerBase({plan: resealedPlan});
  assert.throws(() => verifyChildChain(chain, makeContext(catalog, 'reviewer')),
    /request plan join/);
});

test('rejects a read budget above the 256 KiB ceiling', () => {
  const rc = value('reviewer-configuration');
  const snapshot = value('reviewer-input-snapshot');
  assert.throws(() => validateReviewTargetInput(
    {...rc.reviewInput, maxReadBytes: 262145},
    {run: catalog.context.run, expectedTarget: snapshot.target,
      evaluationDeclarationId: 'evaluation'}), /256 KiB ceiling/);
});

test('rejects a read budget below the retained input coverage', () => {
  const rc = value('reviewer-configuration');
  const snapshot = value('reviewer-input-snapshot');
  assert.throws(() => assertReadBudgetCoverage(
    {...rc.reviewInput, maxReadBytes: 1}, snapshot),
  /approved read budget covers/);
  assert(assertReadBudgetCoverage(rc.reviewInput, snapshot) > 0,
    'positive coverage for the supplied snapshot');
});

test('rejects an unauthenticated target configuration substitution', () => {
  const chain = reviewerBase({targetConfiguration: value('reviewer-configuration')});
  assert.throws(() => verifyChildChain(chain, makeContext(catalog, 'reviewer')),
    /retained artifact unavailable|target configuration digest join/);
});

function reviewerWithDeliveryOrder(deliveryOrder, settingsMutation) {
  const settingsIndex = catalog.context.artifactSets.reviewer
    .findIndex(a => JSON.parse(a.content).kind === 'child-startup-settings');
  const originalSettings = JSON.parse(
    catalog.context.artifactSets.reviewer[settingsIndex].content);
  const mutatedSettings = settingsMutation ?
    settingsMutation(originalSettings) :
    {...originalSettings, deliveryOrder};
  const mutatedCanonical = canonical(mutatedSettings);
  const ctx = makeContext(catalog, 'reviewer');
  ctx.retainedArtifacts = catalog.context.artifactSets.reviewer.map((a, i) => i ===
    settingsIndex ? {content: mutatedCanonical, contentDigest: sha(mutatedCanonical)} : a);
  const configuration = structuredClone(value('reviewer-configuration'));
  configuration.capabilityEvidence = reseal(
    {...configuration.capabilityEvidence,
      effectiveSettingsDigest: sha(mutatedCanonical),
      references: configuration.capabilityEvidence.references.map((r, i) =>
        i === 0 ? {...r, contentDigest: sha(mutatedCanonical)} : r)},
    'evidenceDigest', 'orkworks.capability-evidence.v1');
  configuration.harness = Object.fromEntries(
    ['harnessId', 'definitionDigest', 'adapterId', 'adapterVersion', 'executableIdentity',
      'toolVersion', 'platform', 'instructionMechanism', 'effectiveSettingsDigest',
      'evidenceId', 'evidenceDigest'].map(k => [k, configuration.capabilityEvidence[k]]));
  return {ctx, chain: reviewerBase({configuration: resealConfiguration(configuration)})};
}

test('rejects missing retained review-input delivery with consistent bytes', () => {
  const {ctx, chain} = reviewerWithDeliveryOrder(
    [{'kind': 'automatic', 'precedence': 0}, {'kind': 'supplied'}]);
  assert.throws(() => verifyChildChain(chain, ctx),
    /one retained review-input delivery/);
});

function withMutatedArtifact(role, kind, mutateArtifact) {
  const artifactIndex = catalog.context.artifactSets[role]
    .findIndex(a => a.content.startsWith('{') && JSON.parse(a.content).kind === kind);
  assert(artifactIndex >= 0, 'artifact selected');
  const original = JSON.parse(catalog.context.artifactSets[role][artifactIndex].content);
  const mutated = mutateArtifact(original);
  const mutatedCanonical = canonical(mutated);
  const ctx = makeContext(catalog, role);
  ctx.retainedArtifacts = catalog.context.artifactSets[role].map((a, i) => i ===
    artifactIndex ? {content: mutatedCanonical, contentDigest: sha(mutatedCanonical)} : a);
  return {ctx, original, index: artifactIndex};
}

test('rejects malformed model-policy settings bindings with consistent bytes', () => {
  const {ctx, index} = withMutatedArtifact('reviewer', 'child-model-policy',
    policy => ({...policy, settings: true}));
  const configuration = structuredClone(value('reviewer-configuration'));
  configuration.capabilityEvidence = reseal(
    {...configuration.capabilityEvidence,
      references: configuration.capabilityEvidence.references.map((r, i) =>
        i === index ? {...r, contentDigest:
          ctx.retainedArtifacts[index].contentDigest} : r)},
    'evidenceDigest', 'orkworks.capability-evidence.v1');
  configuration.harness = Object.fromEntries(
    ['harnessId', 'definitionDigest', 'adapterId', 'adapterVersion', 'executableIdentity',
      'toolVersion', 'platform', 'instructionMechanism', 'effectiveSettingsDigest',
      'evidenceId', 'evidenceDigest'].map(k => [k, configuration.capabilityEvidence[k]]));
  configuration.model = {...configuration.model,
    policyDigest: sha(ctx.retainedArtifacts[index].content)};
  const chain = reviewerBase({configuration: resealConfiguration(configuration)});
  assert.throws(() => verifyChildChain(chain, ctx),
    /policy binding name|closed fields|bounded array/);
});

test('rejects automatic delivery beyond the composed startup ceiling', () => {
  const worker = structuredClone(value('worker-configuration'));
  const rule = structuredClone(worker.rules[0]);
  rule.content = 'x'.repeat(140000);
  rule.contentDigest = sha(rule.content);
  const rules = [rule];
  const originalSettings = JSON.parse(catalog.context.artifactSets.worker
    .find(a => JSON.parse(a.content).kind === 'child-startup-settings').content);
  const mutatedSettings = {...originalSettings,
    discoveredInstructions: [{snapshotKind: 'rule', snapshotId: rule.id,
      contentDigest: rule.contentDigest, precedence: 0}]};
  const {ctx} = withMutatedArtifact('worker', 'child-startup-settings', () => mutatedSettings);
  worker.rules = rules;
  worker.requirementManifests = worker.requirementManifests.map(m => m.sourceId ===
    rule.id ? {...m, sourceDigest: rule.contentDigest} : m);
  worker.skills = worker.skills.map(s => s.id === 'fixture-skill' ?
    {...s, requirementSources: s.requirementSources.map(rs => rs.sourceId === rule.id ?
      {...rs, sourceDigest: rule.contentDigest} : rs)} : s);
  worker.capabilityEvidence = reseal(
    {...worker.capabilityEvidence,
      effectiveSettingsDigest: sha(canonical(mutatedSettings)),
      references: worker.capabilityEvidence.references.map((r, i) =>
        i === 0 ? {...r, contentDigest: sha(canonical(mutatedSettings))} : r)},
    'evidenceDigest', 'orkworks.capability-evidence.v1');
  worker.harness = Object.fromEntries(
    ['harnessId', 'definitionDigest', 'adapterId', 'adapterVersion', 'executableIdentity',
      'toolVersion', 'platform', 'instructionMechanism', 'effectiveSettingsDigest',
      'evidenceId', 'evidenceDigest'].map(k => [k, worker.capabilityEvidence[k]]));
  const chain = workerChain({configuration: resealConfiguration(worker)});
  assert.throws(() => verifyChildChain(chain, ctx),
    /composed startup ceiling includes every delivered occurrence/);
});

test('rejects delivery digest drift from the retained input snapshot', () => {
  const {ctx, chain} = reviewerWithDeliveryOrder(undefined, settings => ({
    ...settings,
    deliveryOrder: settings.deliveryOrder.map(d =>
      d.kind === 'review-input' ? {...d, inputDigest: 'f'.repeat(64)} : d),
  }));
  assert.throws(() => verifyChildChain(chain, ctx),
    /review-input delivery digest join/);
});
