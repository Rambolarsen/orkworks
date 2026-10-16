import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
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
  assert.equal(proof.identity.version, 'brain-1');
  assert.equal(proof.identity.sequence, 1);
  assert.equal(proof.identity.bundleSha256, '9d7812815ea4bdae1da3388f2d8ceb22784c9559998ffb76d4a3ee9f20336a3e');
  const manifestBytes = Buffer.from(proof.activation.manifestEnvelopeBase64, 'base64');
  assert.equal(createHash('sha256').update(manifestBytes).digest('hex'),
    '13dc8acab5408965e2f2cfa6f2fc246c53fb4d23b42f9373c54b43f1c3842d19');
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
