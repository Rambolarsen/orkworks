import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { READINESS_CONCEPT_IDS } from '../electron/knowledgeReadiness.ts';
import { KnowledgeCache } from '../electron/knowledgeCache.ts';
import { verifyKnowledgeActivation } from '../electron/knowledgeProof.ts';

const resources = path.resolve(import.meta.dirname, '../resources/knowledge');
const starterPath = path.join(resources, 'starter.json');
const publicKey = readFileSync(path.join(resources, 'public-key.pem'), 'utf8');

test('packaged starter has eligible publisher-signed privacy and manifest proof', () => {
  const proof = verifyKnowledgeActivation(readFileSync(starterPath), publicKey);
  assert.equal(proof.bundle.privacyPolicyVersion, 1);
  assert.equal(proof.identity.version, 'brain-3');
  assert.equal(proof.identity.sequence, 3);
  assert.equal(proof.identity.bundleSha256, 'ac2d1750c76163cad6226eb94ee316cae46788623380589025753adca69fa511');
  const manifestBytes = Buffer.from(proof.activation.manifestEnvelopeBase64, 'base64');
  assert.equal(createHash('sha256').update(manifestBytes).digest('hex'),
    '4e028762fae8a8dca9ebd346210dc965295e43ea43ed6c760ca545de9ea48eb3');
  assert.ok(proof.bundle.capabilities.includes('taskmaster-assessment-v1'));
  assert.equal(proof.identity.publicKeySha256, 'a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd');
  assert.ok(proof.bundle.capabilities.includes('taskmaster-readiness-v1'));
  assert.deepEqual(proof.bundle.readinessRubric?.concepts.map(row => row.conceptId), [...READINESS_CONCEPT_IDS]);
  assert.ok(proof.bundle.readinessRubric?.concepts.every(row => row.levels.length === 5));
  assert.ok(proof.bundle.pages.some(page => page.id === 'playbooks/bounded-workflow-assessment.md'));
});

test('a fresh offline installation loads the signed packaged starter without a cache', async () => {
  const directory = mkdtempSync(path.join(tmpdir(), 'orkworks-starter-'));
  try {
    const cache = new KnowledgeCache({ directory, publicKey, starterPath });
    const proof = await cache.load();
    assert.ok(proof, 'offline starter must establish independently verified knowledge');
    assert.ok(proof.bundle.capabilities.includes('taskmaster-readiness-v1'));
    assert.equal(proof.bundle.readinessRubric?.concepts.length, 14);
    assert.equal(proof.bundle.privacyPolicyVersion, 1);
    assert.equal(proof.identity.publicKeySha256, 'a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
