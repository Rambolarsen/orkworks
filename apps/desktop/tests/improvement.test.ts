import test from 'node:test';
import assert from 'node:assert/strict';
import { canUseImprovementPrimary, improvementHandoffLabel, improvementStatusForWorkspace, parseImprovementStatus, readImprovementIntent } from '../src/improvement.ts';
import { taskmasterRequest } from '../electron/taskmasterSettings.ts';
import { conceptIds } from '../src/readiness.ts';

const status = (overrides: Record<string, unknown> = {}) => ({
  workspacePath: '/repo', available: true, reasonCode: null,
  run: { id: 'r1', state: 'succeeded', phase: null, startedAt: '2026-11-12T10:00:00Z', completedAt: '2026-11-12T10:01:00Z', errorCode: null, report: null },
  ...overrides,
});

test('parses unavailable status without inventing a report', () => {
  assert.deepEqual(parseImprovementStatus(status({ available: false, reasonCode: 'knowledge_unavailable', run: null })), {
    workspacePath: '/repo', available: false, reasonCode: 'knowledge_unavailable', run: null,
  });
  assert.equal(canUseImprovementPrimary(parseImprovementStatus(status({ available: false, run: null }))), false);
  assert.equal(canUseImprovementPrimary(null), false);
  assert.equal(canUseImprovementPrimary(parseImprovementStatus(status())), true);
});

test('fails closed on unknown state and malformed report data', () => {
  assert.throws(() => parseImprovementStatus(status({ run: { ...status().run, state: 'unknown' } })), /Invalid improvement/);
  assert.throws(() => parseImprovementStatus(status({ run: { ...status().run, report: { summary: 4 } } })), /Invalid improvement/);
});

test('ignores status from a stale workspace at the consumer boundary', () => {
  assert.equal(improvementStatusForWorkspace(parseImprovementStatus(status()), '/repo')?.workspacePath, '/repo');
  assert.equal(improvementStatusForWorkspace(parseImprovementStatus(status()), '/other'), null);
});

test('status is an authenticated read and start/cancel use only their frozen routes', async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const fetcher = async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    return new Response(JSON.stringify({ outcome: 'cancel_requested', runId: 'run-1', reasonCode: null }));
  };
  await taskmasterRequest(1234, 'secret', 'improvement', undefined, fetcher);
  await taskmasterRequest(1234, 'secret', 'start-improvement', undefined, fetcher);
  await taskmasterRequest(1234, 'secret', 'cancel-improvement', { runId: 'run-1' }, fetcher);
  assert.deepEqual(calls.map(({ url, init }) => [url, init?.method]), [
    ['http://127.0.0.1:1234/taskmaster/improvement', 'GET'],
    ['http://127.0.0.1:1234/taskmaster/improvement', 'POST'],
    ['http://127.0.0.1:1234/taskmaster/improvement/cancel', 'POST'],
  ]);
  assert.equal(calls[0].init?.headers && (calls[0].init.headers as Record<string, string>)['x-orkworks-open-plan-token'], 'secret');
  await assert.rejects(taskmasterRequest(1234, 'secret', 'start-improvement', { force: true }, fetcher));
});

test('status/history reads do not dispatch inference', async () => {
  let calls = 0;
  await taskmasterRequest(1234, 'secret', 'improvement', undefined, async () => { calls++; return new Response('{}'); });
  assert.equal(calls, 1);
});

test('improvement transport forwards the owning workspace cancellation signal', async () => {
  const controller = new AbortController();
  let received: AbortSignal | null = null;
  await taskmasterRequest(1234, 'secret', 'improvement', undefined, async (_url, init) => {
    received = init?.signal as AbortSignal;
    return new Response('{}');
  }, undefined, controller.signal);
  assert.notEqual(received, controller.signal);
  assert.equal((received as AbortSignal | null)?.aborted, false);
  controller.abort();
  assert.equal((received as AbortSignal | null)?.aborted, true);
});

test('a no-proposal report keeps its explanation separate from readiness score', () => {
  const value: any = status({ run: { ...status().run, report: {
    summary: 'Evidence was incomplete.', concepts: conceptIds.map(conceptId => ({ conceptId, maturity: null, confidence: null, repositoryFactHashes: [], rubricPageId: 'rubric', rationale: 'No evidence', limits: [], nextTransition: '', unassessedReason: 'No permitted evidence' })), opportunities: [], preferredOpportunityId: null,
    noProposalReason: 'No supported change was found.', limitations: [], coverage: 0, overallScore: null,
    recommendationId: null, historySnapshotId: null, historyError: null, repositoryEvidence: [], knowledgeEvidence: [],
  } } });
  const parsed=parseImprovementStatus(value);
  assert.equal(parsed.run?.report?.noProposalReason, 'No supported change was found.');
  assert.equal(parsed.run?.report?.overallScore, null);
});

test('investigation provenance uses investigation wording and legacy provenance keeps implementation wording', () => {
  assert.equal(improvementHandoffLabel('investigation'), 'Investigate with AI');
  assert.equal(improvementHandoffLabel('implementation'), 'Fix with AI');
  assert.equal(improvementHandoffLabel(undefined), 'Fix with AI');
  assert.equal(readImprovementIntent({}), undefined);
  assert.equal(readImprovementIntent({ improvementReference: { version: 1, intent: 'investigation' } }), 'investigation');
  assert.equal(readImprovementIntent({ improvementReference: { version: 2, intent: 'investigation' } }), null);
  assert.equal(readImprovementIntent({ improvementReference: { version: 1, intent: 'unknown' } }), null);
});

const emptyReport = () => ({summary:'Limited evidence.', concepts:conceptIds.map(conceptId=>({conceptId,maturity:null,confidence:null,repositoryFactHashes:[],rubricPageId:'rubric',rationale:'No evidence',limits:[],nextTransition:'Evidence of usable guidance',unassessedReason:'Evidence is unavailable'})),opportunities:[],preferredOpportunityId:null,noProposalReason:'No supported next step.',limitations:[],coverage:0,overallScore:null,recommendationId:null,historySnapshotId:null,historyError:null,repositoryEvidence:[],knowledgeEvidence:[]});

test('recommendations without assessment provenance retain the legacy fix action',()=>{
  assert.equal(readImprovementIntent(undefined),undefined);
  assert.equal(readImprovementIntent(null),undefined);
});
test('public range completeness uses UTF-8 bytes for non-ASCII evidence',()=>{
  const report:any=emptyReport();
  report.repositoryEvidence=[{path:'AGENTS.md',offset:0,fileSize:4,sha256:'a'.repeat(64),text:'åbc',complete:true}];
  assert.doesNotThrow(()=>parseImprovementStatus(status({run:{...status().run,report}})));
});
test('scores and coverage must agree with the capability rows',()=>{
  const report:any=emptyReport();report.coverage=14;report.overallScore=48;
  assert.throws(()=>parseImprovementStatus(status({run:{...status().run,report}})),/Invalid improvement/);
});


test('improvement transport stops and cancels an oversized response before EOF', async () => {
  let pulls = 0, cancelled = false;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      pulls++;
      if (pulls === 1) controller.enqueue(new Uint8Array(256 * 1024));
      else if (pulls === 2) controller.enqueue(new Uint8Array(1));
      else { controller.enqueue(new Uint8Array(100)); controller.close(); }
    },
    cancel() { cancelled = true; },
  }, { highWaterMark: 0 });
  await assert.rejects(taskmasterRequest(1234, 'secret', 'improvement', undefined,
    async () => new Response(stream)), /response too large/);
  assert.equal(cancelled, true);
  assert.equal(pulls, 2);
});
