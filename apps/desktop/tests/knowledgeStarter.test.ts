import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { KnowledgeCache } from '../electron/knowledgeCache.ts';
import { verifyKnowledgeActivation } from '../electron/knowledgeProof.ts';

const resources = path.resolve(import.meta.dirname, '../resources/knowledge');
const starterPath = path.join(resources, 'starter.json');
const publicKey = readFileSync(path.join(resources, 'public-key.pem'), 'utf8');

test('packaged starter has eligible publisher-signed privacy and manifest proof', () => {
  const proof = verifyKnowledgeActivation(readFileSync(starterPath), publicKey);
  assert.equal(proof.bundle.privacyPolicyVersion, 1);
  assert.ok(proof.bundle.capabilities.includes('taskmaster-assessment-v1'));
  assert.equal(proof.identity.publicKeySha256, 'a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd');
  assert.equal(proof.bundle.pages.length, 9);
  assert.ok(proof.bundle.pages.some(page => page.id === 'playbooks/bounded-workflow-assessment.md'));
});

test('a fresh offline installation loads the signed packaged starter without a cache', async () => {
  const directory = mkdtempSync(path.join(tmpdir(), 'orkworks-starter-'));
  try {
    const cache = new KnowledgeCache({ directory, publicKey, starterPath });
    const proof = await cache.load();
    assert.ok(proof, 'offline starter must establish independently verified knowledge');
    assert.equal(proof.bundle.privacyPolicyVersion, 1);
    assert.equal(proof.identity.publicKeySha256, 'a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
