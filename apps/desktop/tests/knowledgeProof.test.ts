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
    'invalid-parent-null.json',
    'invalid-applicability-null.json',
    'invalid-provenance-null.json',
    'invalid-page-digest.json',
    'missing-related-page.json',
    'duplicate-related-ids.json',
    'unsupported-page-type.json',
    'invalid-provenance-credentials.json',
    'invalid-provenance-fragment.json',
    'invalid-provenance-empty-fragment.json',
    'invalid-provenance-empty-credentials.json',
    'invalid-provenance-uppercase-scheme.json', 'invalid-provenance-uppercase-host.json', 'invalid-provenance-default-port.json',
    'invalid-provenance-missing-root-slash.json', 'invalid-provenance-dot-segment.json', 'invalid-provenance-lowercase-escape.json',
    'invalid-provenance-invalid-escape.json', 'invalid-provenance-short-escape.json', 'invalid-provenance-backslash.json',
    'invalid-provenance-unicode-host.json', 'invalid-provenance-unicode-path.json', 'invalid-provenance-space.json',
    'invalid-provenance-control.json', 'invalid-provenance-empty-url.json', 'provenance-url-overflow.json',
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
  const maximumActivation = Buffer.from(JSON.stringify({
    activationFormatVersion: 1,
    bundleEnvelopeBase64: Buffer.from(fixture('maximum-bundle-envelope.json')).toString('base64'),
    manifestEnvelopeBase64: Buffer.from(fixture('maximum-manifest-envelope.json')).toString('base64'),
  }));
  assert.ok(maximumActivation.byteLength < 6 * 1024 * 1024);
  const exactlyMaximum = Buffer.concat([
    maximumActivation,
    Buffer.alloc(6 * 1024 * 1024 - maximumActivation.byteLength, 0x20),
  ]);
  assert.equal(exactlyMaximum.byteLength, 6 * 1024 * 1024);
  const maximum = verifyKnowledgeActivation(exactlyMaximum, testPublicKey);
  assert.equal(maximum.bundle.version, 'maximum-envelope');

  const oneByteOverMaximum = Buffer.concat([exactlyMaximum, Buffer.from(' ')]);
  assert.equal(oneByteOverMaximum.byteLength, 6 * 1024 * 1024 + 1);
  assert.throws(() => verifyKnowledgeActivation(oneByteOverMaximum, testPublicKey));

  const envelopeOverflow = Buffer.alloc(2 * 1024 * 1024 + 1, 0x78);
  const validManifestBase64 = (JSON.parse(fixtureText('valid-activation.json')) as { manifestEnvelopeBase64: string }).manifestEnvelopeBase64;
  const envelopeOverflowActivation = Buffer.from(JSON.stringify({
    activationFormatVersion: 1,
    bundleEnvelopeBase64: envelopeOverflow.toString('base64'),
    manifestEnvelopeBase64: validManifestBase64,
  }));
  assert.throws(() => verifyKnowledgeActivation(envelopeOverflowActivation, testPublicKey));
  assert.throws(() => createKnowledgeActivation(envelopeOverflow, fixture('valid-manifest-envelope.json')));
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

  const withValidUnsupported = verifyKnowledgeManifest(fixture('manifest-valid-unsupported-envelope.json'), testPublicKey);
  assert.equal(withValidUnsupported.bundles.length, 2);
  assert.equal(selectKnowledgeEntry(withValidUnsupported).version, 'manifest-0000');
  for (const name of [
    'manifest-entry-overflow-envelope.json', 'duplicate-manifest-entry-envelope.json',
    'manifest-invalid-sequence-string-envelope.json', 'manifest-invalid-sequence-fraction-envelope.json', 'manifest-invalid-sequence-overflow-envelope.json',
    'manifest-invalid-version-nonstring-envelope.json', 'manifest-invalid-version-empty-envelope.json', 'manifest-invalid-version-overflow-envelope.json', 'manifest-invalid-version-unsafe-envelope.json',
    'manifest-invalid-digest-nonstring-envelope.json', 'manifest-invalid-digest-uppercase-envelope.json', 'manifest-invalid-digest-short-envelope.json',
    'manifest-invalid-path-nonstring-envelope.json', 'manifest-invalid-path-mismatch-envelope.json',
    'manifest-invalid-format-missing-envelope.json', 'manifest-invalid-format-noninteger-envelope.json',
    'manifest-invalid-policy-missing-envelope.json', 'manifest-invalid-policy-noninteger-envelope.json',
  ]) {
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

test('strict JSON counts root object, root array, and alternating container depth boundaries', () => {
  const nested = (shape: 'object' | 'array' | 'alternating', targetDepth: number): string => {
    let value = '0';
    for (let depth = targetDepth; depth >= 1; depth--) {
      const objectContainer = shape === 'object' || (shape === 'alternating' && depth % 2 === 1);
      value = objectContainer ? `{"next":${value}}` : `[${value}]`;
    }
    return value;
  };

  assert.throws(() => parseStrictJson(Buffer.from('{"text":"\\ud800"}')));
  for (const shape of ['object', 'array', 'alternating'] as const) {
    for (const depth of [31, 32]) assert.doesNotThrow(() => parseStrictJson(Buffer.from(nested(shape, depth))), `${shape} depth ${depth}`);
    assert.throws(() => parseStrictJson(Buffer.from(nested(shape, 33))), `${shape} depth 33`);
  }
});

test('strict JSON accepts escaped surrogate pairs and valid lexical content in unknown fields', () => {
  assert.deepEqual(parseStrictJson(Buffer.from('{"text":"\\uD83D\\uDE00","unknown":{"nested":[0,true,null]}}')), {
    text: '😀',
    unknown: { nested: [0, true, null] },
  });
  assert.doesNotThrow(() => verifyKnowledgeActivation(fixture('unknown-valid-fields.json'), testPublicKey));
});

test('signed unknown metadata follows the shared container-depth corpus', () => {
  for (const shape of ['object', 'array', 'alternating']) {
    for (const depth of [31, 32]) {
      const name = `depth-${shape}-${depth}.json`;
      assert.doesNotThrow(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
    }
    const name = `depth-${shape}-33.json`;
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('signed knowledge accepts zero and maximum safe sequences', () => {
  const zero = verifyKnowledgeActivation(fixture('sequence-zero.json'), testPublicKey);
  const maximum = verifyKnowledgeActivation(fixture('sequence-max-safe.json'), testPublicKey);
  assert.equal(zero.identity.sequence, 0);
  assert.equal(zero.bundle.sequence, 0);
  assert.equal(maximum.identity.sequence, Number.MAX_SAFE_INTEGER);
  assert.equal(maximum.bundle.sequence, Number.MAX_SAFE_INTEGER);
  assert.equal(verifyKnowledgeManifest(fixture('manifest-sequence-zero-envelope.json'), testPublicKey).bundles[0].sequence, 0);
});

test('canonical HTTPS provenance URLs preserve valid normalized spellings', () => {
  for (const name of [
    'provenance-at-outside-authority.json', 'provenance-punycode-host.json',
    'provenance-uppercase-utf8-escapes.json', 'provenance-encoded-hash.json',
  ]) assert.doesNotThrow(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
});

test('canonical timestamp boundaries accept year zero and valid leap dates', () => {
  for (const name of ['timestamp-year-zero.json', 'timestamp-leap-2000.json', 'timestamp-leap-2024.json', 'timestamp-year-9999-end.json']) {
    assert.doesNotThrow(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
  for (const name of [
    'timestamp-invalid-february-30.json', 'timestamp-invalid-nonleap-february-29.json',
    'timestamp-invalid-month-zero.json', 'timestamp-invalid-month-thirteen.json', 'timestamp-invalid-day-zero.json',
    'timestamp-invalid-day-thirty-two.json', 'timestamp-invalid-hour-twenty-four.json', 'timestamp-invalid-minute-sixty.json',
    'timestamp-invalid-second-sixty.json', 'timestamp-invalid-lowercase.json', 'timestamp-invalid-offset.json',
    'timestamp-invalid-expanded-year.json', 'timestamp-invalid-missing-milliseconds.json', 'timestamp-invalid-four-fraction-digits.json',
    'timestamp-invalid-space-separator.json',
  ]) assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
});

test('signed escaped surrogate pairs preserve decoded page content and digest', () => {
  const result = verifyKnowledgeActivation(fixture('escaped-surrogate-content.json'), testPublicKey);
  assert.equal(result.bundle.pages[0].content, '😀');
});

test('BOM bytes are rejected at activation, envelope, and signed payload JSON boundaries', () => {
  for (const name of ['bom-activation.json', 'bom-envelope-activation.json', 'bom-payload-activation.json']) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
  assert.throws(() => parseStrictJson(Buffer.from([0xef, 0xbb, 0xbf, 0x7b, 0x7d])));
});

test('capability, applicability, and version byte lengths accept 128 and reject 129', () => {
  const result = verifyKnowledgeActivation(fixture('string-boundaries-128.json'), testPublicKey);
  assert.equal(Buffer.byteLength(result.bundle.capabilities[0], 'utf8'), 128);
  assert.equal(Buffer.byteLength(result.bundle.pages[0].applicability![0], 'utf8'), 128);
  assert.equal(Buffer.byteLength(result.bundle.version, 'utf8'), 128);
  for (const name of ['capability-129.json', 'applicability-129.json', 'version-129.json']) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('unknown signed payload fields are checked recursively for duplicate keys and numeric syntax', () => {
  for (const name of ['unknown-nested-duplicate.json', 'unknown-fraction.json', 'unknown-exponent.json', 'unknown-negative-zero.json', 'unknown-overflow.json']) {
    assert.throws(() => verifyKnowledgeActivation(fixture(name), testPublicKey), name);
  }
});

test('strict JSON rejects malformed UTF-8 and bytes over the caller bound', () => {
  assert.throws(() => parseStrictJson(Uint8Array.of(0xff)));
  assert.throws(() => parseStrictJson(Buffer.from('{}'), 1));
});
