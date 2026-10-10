import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { parseStrictJson } from '../electron/strictJson.ts';
import {
  createKnowledgeActivation,
  selectKnowledgeEntry,
  verifyKnowledgeActivation,
  verifyKnowledgeManifest,
} from '../electron/knowledgeProof.ts';

const fixtureRoot = path.resolve(import.meta.dirname, '../../../tests/fixtures/taskmaster-knowledge');
const fixture = (name: string): Uint8Array => readFileSync(path.join(fixtureRoot, name));
const fixtureText = (name: string): string => fixture(name).toString();
const testPublicKey = fixtureText('test-public-key.pem');

test('matching signed policy and attestation admit synthetic knowledge', () => {
  const result = verifyKnowledgeActivation(fixture('valid-activation.json'), testPublicKey);

  assert.equal(result.bundle.privacyPolicyVersion, 1);
  assert.equal(result.identity.bundleSha256, fixtureText('valid-envelope.sha256').trim());
  assert.equal(result.identity.publicKeySha256, fixtureText('test-public-key-spki.sha256').trim());
  assert.equal(result.identity.formatVersion, 1);
  assert.equal(result.identity.version, 'synthetic-1');
  assert.equal(result.identity.sequence, 1);
  assert.equal(result.bundle.pages[0].id, 'concepts/reliable-changes.md');
});

test('an unsigned bundle cannot activate knowledge', () => {
  assert.throws(() => verifyKnowledgeActivation(fixture('unsigned-bundle.json'), testPublicKey));
});

test('the verifier rejects wrong key, signature, digest, and policy attestations', () => {
  for (const name of ['wrong-key.json', 'wrong-signature.json', 'wrong-digest.json', 'wrong-policy.json']) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('the verifier rejects duplicate fields, malformed UTF-8, and noncanonical base64', () => {
  for (const name of [
    'duplicate-envelope-field.json',
    'duplicate-payload-field.json',
    'unknown-envelope-field.json',
    'unknown-activation-field.json',
    'malformed-utf8.json',
    'noncanonical-base64.json',
  ]) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('invalid page metadata, unsafe IDs, and parent cycles cannot be activated', () => {
  for (const name of [
    'invalid-metadata.json',
    'invalid-page-digest.json',
    'missing-related-page.json',
    'duplicate-related-ids.json',
    'unsupported-page-type.json',
    'invalid-provenance-credentials.json',
    'invalid-provenance-fragment.json',
    'invalid-provenance-http.json',
    'unsafe-id.json',
    'unsafe-related-id.json',
    'parent-cycle.json',
  ]) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('page ID grammar rejects duplicate IDs and every unsafe path spelling', () => {
  const unsafeIds = [
    'space', 'colon', 'control', 'unicode', 'percent', 'backslash', 'emptySegment', 'dotSegment',
    'dotdotSegment', 'leadingDotSegment', 'dotFileSegment', 'uppercaseExtension', 'leadingSlash',
    'leadingHyphen', 'tooLong',
  ];
  for (const label of unsafeIds) {
    assert.throws(() => verifyKnowledgeActivation(fixture(`invalid-id-${label}.json`), testPublicKey), label);
  }
  assert.throws(() => verifyKnowledgeActivation(fixture('duplicate-page-ids.json'), testPublicKey));
  const maximum = verifyKnowledgeActivation(fixture('maximum-id-length.json'), testPublicKey);
  assert.equal(Buffer.byteLength(maximum.bundle.pages[0].id, 'ascii'), 256);
});

test('manifest selection ranks only the supported format and privacy policy', () => {
  const manifest = verifyKnowledgeManifest(fixture('ranked-manifest-envelope.json'), testPublicKey);

  assert.equal(selectKnowledgeEntry(manifest).version, 'supported-newer');
});

test('same bundle with a newer valid manifest is idempotent at bundle identity', () => {
  const original = verifyKnowledgeActivation(fixture('valid-activation.json'), testPublicKey);
  const reattested = verifyKnowledgeActivation(fixture('same-bundle-newer-manifest.json'), testPublicKey);

  assert.deepEqual(reattested.identity, original.identity);
  assert.notDeepEqual(reattested.activation, original.activation);
});

test('verified data is immutable against post-verification mutation', () => {
  const result = verifyKnowledgeActivation(fixture('valid-activation.json'), testPublicKey);

  assert.equal(Object.isFrozen(result), true);
  assert.equal(Object.isFrozen(result.bundle), true);
  assert.equal(Object.isFrozen(result.bundle.pages), true);
  assert.equal(Object.isFrozen(result.bundle.pages[0]), true);
  assert.throws(() => {
    (result.bundle.pages[0] as { title: string }).title = 'changed';
  });
});

test('activation helper preserves original envelope bytes and the exact wire fields', () => {
  const activation = createKnowledgeActivation(fixture('valid-bundle-envelope.json'), fixture('valid-manifest-envelope.json'));

  assert.deepEqual(Object.keys(activation).sort(), [
    'activationFormatVersion',
    'bundleEnvelopeBase64',
    'manifestEnvelopeBase64',
  ]);
  assert.equal(Buffer.from(activation.bundleEnvelopeBase64, 'base64').compare(Buffer.from(fixture('valid-bundle-envelope.json'))), 0);
});

test('the shared malformed syntax corpus enforces lexical JSON constraints', () => {
  for (const name of [
    'escaped-duplicate-key.json',
    'integer-overflow.json',
    'lone-surrogate.json',
    'fractional-number.json',
    'exponent-number.json',
    'negative-zero.json',
    'too-deep.json',
  ]) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('activation and envelope bounds accept two maximum envelopes and reject overflow', () => {
  assert.equal(fixture('maximum-bundle-envelope.json').byteLength, 2 * 1024 * 1024);
  assert.equal(fixture('maximum-manifest-envelope.json').byteLength, 2 * 1024 * 1024);
  const maximum = verifyKnowledgeActivation(fixture('maximum-envelopes-activation.json'), testPublicKey);
  assert.equal(maximum.bundle.version, 'maximum-envelope');
  assert.throws(() => verifyKnowledgeActivation(fixture('activation-overflow.json'), testPublicKey));
  assert.throws(() => verifyKnowledgeActivation(fixture('envelope-overflow.json'), testPublicKey));
  assert.throws(() => createKnowledgeActivation(fixture('envelope-overflow-envelope.json'), fixture('valid-manifest-envelope.json')));
});

test('page count, content, text, relationships, applicability, and provenance limits are inclusive', () => {
  const result = verifyKnowledgeActivation(fixture('maximum-page-limits.json'), testPublicKey);
  assert.equal(result.bundle.pages.length, 256);
  assert.equal(Buffer.byteLength(result.bundle.pages[0].content, 'utf8'), 64 * 1024);
  assert.equal(result.bundle.pages[0].title.length, 512);
  assert.equal(result.bundle.pages[0].relatedIds.length, 256);
  assert.equal(result.bundle.pages[0].applicability?.length, 32);
  assert.equal(result.bundle.pages[0].provenance?.length, 16);

  for (const name of [
    'content-overflow.json',
    'title-overflow.json',
    'status-overflow.json',
    'applicability-overflow.json',
    'provenance-overflow.json',
    'page-count-overflow.json',
  ]) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('capability bounds accept 16 safe unique markers and reject overflow', () => {
  const maximum = verifyKnowledgeActivation(fixture('maximum-capabilities.json'), testPublicKey);
  assert.equal(maximum.bundle.capabilities.length, 16);
  assert.throws(() => verifyKnowledgeActivation(fixture('capability-overflow.json'), testPublicKey));
});

test('manifest accepts 1,000 entries and rejects overflow and duplicate identities', () => {
  const maximum = verifyKnowledgeManifest(fixture('maximum-manifest-entries-envelope.json'), testPublicKey);
  assert.equal(maximum.bundles.length, 1000);
  assert.equal(selectKnowledgeEntry(maximum).sequence, 1000);
  for (const name of ['manifest-entry-overflow-envelope.json', 'duplicate-manifest-entry-envelope.json']) {
    assert.throws(() => verifyKnowledgeManifest(fixture(name), testPublicKey), name);
  }
});

test('strict JSON rejects escaped duplicate keys, unsafe numbers, fractions, exponents, and negative zero', () => {
  for (const json of [
    '{"a":1,"\\u0061":2}',
    '{"n":9007199254740992}',
    '{"n":1.0}',
    '{"n":1e0}',
    '{"n":-0}',
  ]) {
    assert.throws(() => parseStrictJson(Buffer.from(json)));
  }
});

test('strict JSON rejects lone surrogate escapes and nesting beyond 32 levels', () => {
  assert.throws(() => parseStrictJson(Buffer.from('{"text":"\\ud800"}')));
  assert.throws(() => parseStrictJson(Buffer.from(`${'['.repeat(33)}0${']'.repeat(33)}`)));
  assert.deepEqual(parseStrictJson(Buffer.from(`${'['.repeat(32)}0${']'.repeat(32)}`)), JSON.parse(`${'['.repeat(32)}0${']'.repeat(32)}`));
});

test('strict JSON rejects malformed UTF-8 and bytes over the caller bound', () => {
  assert.throws(() => parseStrictJson(Uint8Array.of(0xff)));
  assert.throws(() => parseStrictJson(Buffer.from('{}'), 1));
});
