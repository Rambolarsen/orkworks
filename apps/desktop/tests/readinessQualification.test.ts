import test from 'node:test';
import assert from 'node:assert/strict';
import { readinessQualification, type ReadinessData } from '../src/readiness.ts';

const profile = () => ({
  inferenceIdentity: { selection: { provider: 'codex', model: 'gpt-6.1-sol', reasoningEffort: 'low' }, custom: null, native: { profile: 'Codex' } },
  knowledgeIdentity: { version: 'brain-3', bundle_sha256: 'ac2d1750c76163cad6226eb94ee316cae46788623380589025753adca69fa511' },
  rubricDigest: '9f2c7c21b0edfa0e96fd806d9bcc50e2dd983dbf3ab2a45cf5395b0e1aea1a5e',
  versions: { collector: 3, prompt: 'taskmaster-readiness-v2', discoverySchema: 1, finalSchema: 1, scoring: 1 },
});
const notice = (value: ReturnType<typeof profile>) => readinessQualification(value as ReadinessData);

test('the reviewed collector-3 prompt-v2 profile has a bounded fixture qualification', () => {
  assert.match(notice(profile()), /passed.*fixture qualification/i);
  assert.match(notice(profile()), /does not establish accuracy for every repository/i);
});

test('qualification stays scoped to exact model, effort, transport, prompt, collector, rubric and guidance', () => {
  const changes: [string, (value: ReturnType<typeof profile>) => void][] = [
    ['model', v => { v.inferenceIdentity.selection.model = 'another-model'; }],
    ['effort', v => { v.inferenceIdentity.selection.reasoningEffort = 'medium'; }],
    ['provider', v => { v.inferenceIdentity.selection.provider = 'custom'; }],
    ['native profile', v => { v.inferenceIdentity.native.profile = 'ClaudeCode'; }],
    ['prompt', v => { v.versions.prompt = 'taskmaster-readiness-v1'; }],
    ['collector', v => { v.versions.collector = 2; }],
    ['rubric', v => { v.rubricDigest = 'a'.repeat(64); }],
    ['guidance', v => { v.knowledgeIdentity.bundle_sha256 = 'a'.repeat(64); }],
    ['discovery schema', v => { v.versions.discoverySchema = 2; }],
    ['final schema', v => { v.versions.finalSchema = 2; }],
    ['scoring', v => { v.versions.scoring = 2; }],
  ];
  for (const [name, change] of changes) {
    const value = profile();
    change(value);
    assert.match(notice(value), /no retained readiness qualification/i, name);
  }
  const custom = { ...profile(), inferenceIdentity: { ...profile().inferenceIdentity, custom: { definition: 'override' } } };
  assert.match(readinessQualification(custom as unknown as ReadinessData), /no retained readiness qualification/i);
});

test('original collector-2 qualification remains separate from the failed collector-3 prompt-v1 profile', () => {
  const original = profile();
  original.versions = { ...original.versions, collector: 2, prompt: 'taskmaster-readiness-v1' };
  assert.match(notice(original), /passed the original readiness fixture qualification/i);
  original.rubricDigest = 'a'.repeat(64);
  assert.match(notice(original), /no retained readiness qualification/i);
  original.rubricDigest = profile().rubricDigest;
  original.versions.collector = 3;
  assert.match(notice(original), /no retained readiness qualification/i);
});
