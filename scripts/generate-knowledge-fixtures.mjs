import { createHash, createPublicKey, generateKeyPairSync, sign } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const fixtureRoot = path.join(repoRoot, 'tests', 'fixtures', 'taskmaster-knowledge');
mkdirSync(fixtureRoot, { recursive: true });

const privateKeyPath = path.join(fixtureRoot, 'test-private-key.pem');
const publicKeyPath = path.join(fixtureRoot, 'test-public-key.pem');
if (!existsSync(privateKeyPath) || !existsSync(publicKeyPath)) {
  if (existsSync(privateKeyPath) || existsSync(publicKeyPath)) {
    throw new Error('Refusing to replace an incomplete fixed test key pair. Remove both test-only key files explicitly.');
  }
  const pair = generateKeyPairSync('ed25519');
  writeFileSync(privateKeyPath, pair.privateKey.export({ format: 'pem', type: 'pkcs8' }), { mode: 0o600 });
  writeFileSync(publicKeyPath, pair.publicKey.export({ format: 'pem', type: 'spki' }));
}

const privateKey = readFileSync(privateKeyPath);
const publicKey = readFileSync(publicKeyPath);
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const json = (value) => Buffer.from(JSON.stringify(value));
const safeWrite = (name, bytes) => writeFileSync(path.join(fixtureRoot, name), bytes);
const activation = (bundleEnvelope, manifestEnvelope) => json({
  activationFormatVersion: 1,
  bundleEnvelopeBase64: Buffer.from(bundleEnvelope).toString('base64'),
  manifestEnvelopeBase64: Buffer.from(manifestEnvelope).toString('base64'),
});
const envelope = (payloadBytes, key = privateKey) => {
  const payload = Buffer.from(payloadBytes).toString('utf8');
  const signature = sign(null, Buffer.from(payloadBytes), key).toString('base64');
  return json({ payload, signature });
};
const makeBundle = (version, sequence, overrides = {}) => ({
  formatVersion: 1,
  version,
  sequence,
  publishedAt: '2026-10-10T09:00:00.000Z',
  privacyPolicyVersion: 1,
  capabilities: [],
  pages: [{
    id: 'concepts/reliable-changes.md',
    title: 'Reliable changes',
    type: 'concept',
    status: 'established',
    content: 'Use a small, verifiable change.',
    sha256: hash(Buffer.from('Use a small, verifiable change.')),
    relatedIds: [],
  }],
  ...overrides,
});
const makeManifest = (entries) => ({ formatVersion: 1, bundles: entries });
const entry = (bundle, rawBundleEnvelope, overrides = {}) => ({
  formatVersion: bundle.formatVersion,
  privacyPolicyVersion: bundle.privacyPolicyVersion,
  sequence: bundle.sequence,
  version: bundle.version,
  sha256: hash(rawBundleEnvelope),
  path: `bundles/${bundle.version}.json`,
  ...overrides,
});
const signedPair = (bundle, options = {}) => {
  const payloadBytes = options.payloadBytes ?? json(bundle);
  const bundleEnvelope = options.bundleEnvelope ?? envelope(payloadBytes, options.key ?? privateKey);
  const manifest = options.manifest ?? makeManifest([entry(bundle, bundleEnvelope, options.entryOverrides)]);
  const manifestEnvelope = envelope(json(manifest), options.manifestKey ?? privateKey);
  return { bundle, bundleEnvelope, manifest, manifestEnvelope };
};
const writeActivation = (name, pair) => safeWrite(name, activation(pair.bundleEnvelope, pair.manifestEnvelope));
const exactEnvelope = (makePayload, targetBytes) => {
  let low = 0;
  let high = targetBytes;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (envelope(makePayload(middle)).byteLength < targetBytes) low = middle + 1;
    else high = middle;
  }
  const payloadBytes = makePayload(low);
  const result = envelope(payloadBytes);
  if (result.byteLength !== targetBytes) throw new Error(`Could not make a ${targetBytes}-byte signed envelope`);
  return { payloadBytes, result };
};

const valid = signedPair(makeBundle('synthetic-1', 1));
safeWrite('valid-bundle-envelope.json', valid.bundleEnvelope);
safeWrite('valid-manifest-envelope.json', valid.manifestEnvelope);
safeWrite('valid-envelope.sha256', `${hash(valid.bundleEnvelope)}\n`);
safeWrite('test-public-key-spki.sha256', `${hash(createPublicKey(publicKey).export({ format: 'der', type: 'spki' }))}\n`);
writeActivation('valid-activation.json', valid);

const unsigned = activation(json(valid.bundle), valid.manifestEnvelope);
safeWrite('unsigned-bundle.json', unsigned);

const otherKeys = generateKeyPairSync('ed25519');
const wrongKeyPair = signedPair(makeBundle('wrong-key', 2), { key: otherKeys.privateKey });
writeActivation('wrong-key.json', wrongKeyPair);

const wrongSignatureEnvelopeObject = JSON.parse(valid.bundleEnvelope.toString());
const signatureBytes = Buffer.from(wrongSignatureEnvelopeObject.signature, 'base64');
signatureBytes[0] ^= 1;
wrongSignatureEnvelopeObject.signature = signatureBytes.toString('base64');
const wrongSignatureEnvelope = json(wrongSignatureEnvelopeObject);
const wrongSignatureManifest = envelope(json(makeManifest([entry(valid.bundle, wrongSignatureEnvelope)])));
safeWrite('wrong-signature.json', activation(wrongSignatureEnvelope, wrongSignatureManifest));

const wrongDigestManifest = envelope(json(makeManifest([entry(valid.bundle, valid.bundleEnvelope, { sha256: '0'.repeat(64) })])));
safeWrite('wrong-digest.json', activation(valid.bundleEnvelope, wrongDigestManifest));

const wrongPolicyManifest = envelope(json(makeManifest([entry(valid.bundle, valid.bundleEnvelope, { privacyPolicyVersion: 2 })])));
safeWrite('wrong-policy.json', activation(valid.bundleEnvelope, wrongPolicyManifest));

const duplicateEnvelopeText = `{"payload":${JSON.stringify(JSON.parse(valid.bundleEnvelope.toString()).payload)},"signature":${JSON.stringify(JSON.parse(valid.bundleEnvelope.toString()).signature)},"signature":${JSON.stringify(JSON.parse(valid.bundleEnvelope.toString()).signature)}}`;
safeWrite('duplicate-envelope-field.json', activation(Buffer.from(duplicateEnvelopeText), valid.manifestEnvelope));

const duplicatePayload = `{"formatVersion":1,${JSON.stringify(valid.bundle).slice(1)}`;
const duplicatePayloadEnvelope = envelope(Buffer.from(duplicatePayload));
const duplicatePayloadManifestEnvelope = envelope(json(makeManifest([entry(valid.bundle, duplicatePayloadEnvelope)])));
safeWrite('duplicate-payload-field.json', activation(duplicatePayloadEnvelope, duplicatePayloadManifestEnvelope));

const malformedUtf8 = Buffer.from('{"activationFormatVersion":1,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}');
malformedUtf8[0] = 0xff;
safeWrite('malformed-utf8.json', malformedUtf8);
safeWrite('noncanonical-base64.json', json({ activationFormatVersion: 1, bundleEnvelopeBase64: 'AB==', manifestEnvelopeBase64: Buffer.from(valid.manifestEnvelope).toString('base64') }));

const invalidMetadata = makeBundle('invalid-metadata', 2);
invalidMetadata.pages[0].title = '';
writeActivation('invalid-metadata.json', signedPair(invalidMetadata));

const invalidPageDigest = makeBundle('invalid-page-digest', 62);
invalidPageDigest.pages[0].sha256 = '0'.repeat(64);
writeActivation('invalid-page-digest.json', signedPair(invalidPageDigest));

const missingRelatedPage = makeBundle('missing-related-page', 63);
missingRelatedPage.pages[0].relatedIds = ['missing.md'];
writeActivation('missing-related-page.json', signedPair(missingRelatedPage));

const duplicateRelatedIds = makeBundle('duplicate-related-ids', 64);
duplicateRelatedIds.pages[0].relatedIds = [duplicateRelatedIds.pages[0].id, duplicateRelatedIds.pages[0].id];
writeActivation('duplicate-related-ids.json', signedPair(duplicateRelatedIds));

const unsupportedPageType = makeBundle('unsupported-page-type', 32);
unsupportedPageType.pages[0].type = 'assessment';
writeActivation('unsupported-page-type.json', signedPair(unsupportedPageType));

const unsafeId = makeBundle('unsafe-id', 3);
unsafeId.pages[0].id = '../private.md';
writeActivation('unsafe-id.json', signedPair(unsafeId));

const invalidIds = {
  space: 'bad name.md',
  colon: 'bad:name.md',
  control: 'bad\nname.md',
  unicode: 'café.md',
  percent: 'bad%20name.md',
  backslash: 'folder\\name.md',
  emptySegment: 'a//b.md',
  dotSegment: 'a/./b.md',
  dotdotSegment: 'a/../b.md',
  leadingDotSegment: '.hidden/page.md',
  dotFileSegment: 'folder/.hidden.md',
  uppercaseExtension: 'page.MD',
  leadingSlash: '/page.md',
  leadingHyphen: '-page.md',
  tooLong: `${'a'.repeat(254)}.md`,
};
for (const [label, id] of Object.entries(invalidIds)) {
  const bundle = makeBundle(`invalid-id-${label}`, 40 + Object.keys(invalidIds).indexOf(label));
  bundle.pages[0].id = id;
  writeActivation(`invalid-id-${label}.json`, signedPair(bundle));
}

const duplicatePageIds = makeBundle('duplicate-page-ids', 60, { pages: [
  { id: 'pages/same.md', title: 'First', type: 'concept', status: 'active', content: 'First', sha256: hash(Buffer.from('First')), relatedIds: [] },
  { id: 'pages/same.md', title: 'Second', type: 'concept', status: 'active', content: 'Second', sha256: hash(Buffer.from('Second')), relatedIds: [] },
] });
writeActivation('duplicate-page-ids.json', signedPair(duplicatePageIds));

const maximumLengthId = makeBundle('maximum-id-length', 61);
maximumLengthId.pages[0].id = `${'a'.repeat(253)}.md`;
writeActivation('maximum-id-length.json', signedPair(maximumLengthId));

const unsafeRelatedId = makeBundle('unsafe-related-id', 31);
unsafeRelatedId.pages[0].relatedIds = ['../private.md'];
writeActivation('unsafe-related-id.json', signedPair(unsafeRelatedId));

const cycle = makeBundle('parent-cycle', 4, { pages: [
  { id: 'a.md', title: 'A', type: 'concept', status: 'active', content: 'A', sha256: hash(Buffer.from('A')), relatedIds: [], parentId: 'b.md' },
  { id: 'b.md', title: 'B', type: 'concept', status: 'active', content: 'B', sha256: hash(Buffer.from('B')), relatedIds: [], parentId: 'a.md' },
] });
writeActivation('parent-cycle.json', signedPair(cycle));

const sameBundleManifest = makeManifest([
  entry(valid.bundle, valid.bundleEnvelope),
  { formatVersion: 2, privacyPolicyVersion: 7, sequence: 900, version: 'future-format', sha256: '1'.repeat(64), path: 'bundles/future-format.json' },
]);
const sameBundleManifestEnvelope = envelope(json(sameBundleManifest));
safeWrite('same-bundle-newer-manifest.json', activation(valid.bundleEnvelope, sameBundleManifestEnvelope));

const rankedOldBundle = makeBundle('supported-older', 4);
const rankedNewBundle = makeBundle('supported-newer', 5);
const rankedOldEnvelope = envelope(json(rankedOldBundle));
const rankedNewEnvelope = envelope(json(rankedNewBundle));
const rankedManifest = makeManifest([
  entry(rankedOldBundle, rankedOldEnvelope),
  entry(rankedNewBundle, rankedNewEnvelope),
  { formatVersion: 2, privacyPolicyVersion: 1, sequence: Number.MAX_SAFE_INTEGER, version: 'future-format', sha256: '2'.repeat(64), path: 'bundles/future-format.json' },
  { formatVersion: 1, privacyPolicyVersion: 2, sequence: Number.MAX_SAFE_INTEGER, version: 'future-policy', sha256: '3'.repeat(64), path: 'bundles/future-policy.json' },
]);
safeWrite('ranked-manifest-envelope.json', envelope(json(rankedManifest)));

const maximumPageIds = Array.from({ length: 256 }, (_, index) => `pages/page-${String(index).padStart(3, '0')}.md`);
const maximumUrlPrefix = 'https://example.test/';
const maximumPage = {
  id: maximumPageIds[0],
  title: 't'.repeat(512),
  type: 'concept',
  status: 's'.repeat(128),
  content: 'x'.repeat(64 * 1024),
  sha256: hash(Buffer.from('x'.repeat(64 * 1024))),
  relatedIds: maximumPageIds,
  applicability: Array.from({ length: 32 }, (_, index) => `scope-${String(index).padStart(2, '0')}-${'a'.repeat(110)}`),
  provenance: Array.from({ length: 16 }, (_, index) => ({
    title: 'p'.repeat(512),
    url: `${maximumUrlPrefix}${String(index).padStart(2, '0')}${'x'.repeat(2048 - Buffer.byteLength(`${maximumUrlPrefix}${String(index).padStart(2, '0')}`))}`,
  })),
};
const maximumPagesBundle = makeBundle('maximum-pages', 20, {
  pages: [maximumPage, ...maximumPageIds.slice(1).map((id, index) => ({
    id,
    title: `Page ${index + 1}`,
    type: 'concept',
    status: 'active',
    content: 'page',
    sha256: hash(Buffer.from('page')),
    relatedIds: [],
  }))],
});
writeActivation('maximum-page-limits.json', signedPair(maximumPagesBundle));

const maximumCapabilities = makeBundle('maximum-capabilities', 66, {
  capabilities: Array.from({ length: 16 }, (_, index) => `capability-${String(index).padStart(2, '0')}`),
});
writeActivation('maximum-capabilities.json', signedPair(maximumCapabilities));
const overCapabilities = makeBundle('over-capabilities', 67, {
  capabilities: Array.from({ length: 17 }, (_, index) => `capability-${String(index).padStart(2, '0')}`),
});
writeActivation('capability-overflow.json', signedPair(overCapabilities));

const overContent = makeBundle('over-content', 21);
overContent.pages[0].content = 'x'.repeat(64 * 1024 + 1);
overContent.pages[0].sha256 = hash(Buffer.from(overContent.pages[0].content));
writeActivation('content-overflow.json', signedPair(overContent));

const overTitle = makeBundle('over-title', 22);
overTitle.pages[0].title = 't'.repeat(513);
writeActivation('title-overflow.json', signedPair(overTitle));

const overStatus = makeBundle('over-status', 23);
overStatus.pages[0].status = 's'.repeat(129);
writeActivation('status-overflow.json', signedPair(overStatus));

const overApplicability = makeBundle('over-applicability', 24);
overApplicability.pages[0].applicability = Array.from({ length: 33 }, (_, index) => `scope-${index}`);
writeActivation('applicability-overflow.json', signedPair(overApplicability));

const overProvenance = makeBundle('over-provenance', 25);
overProvenance.pages[0].provenance = Array.from({ length: 17 }, () => ({ title: 'source', url: 'https://example.test/source' }));
writeActivation('provenance-overflow.json', signedPair(overProvenance));

for (const [index, [label, url]] of [
  ['credentials', 'https://user:pass@example.test/source'],
  ['fragment', 'https://example.test/source#fragment'],
  ['http', 'http://example.test/source'],
].entries()) {
  const invalidProvenance = makeBundle(`invalid-provenance-${label}`, 68 + index);
  invalidProvenance.pages[0].provenance = [{ title: 'source', url }];
  writeActivation(`invalid-provenance-${label}.json`, signedPair(invalidProvenance));
}

const overPageCount = makeBundle('over-page-count', 26, {
  pages: Array.from({ length: 257 }, (_, index) => {
    const content = `Page ${index}`;
    return { id: `pages/${index}.md`, title: content, type: 'concept', status: 'active', content, sha256: hash(Buffer.from(content)), relatedIds: [] };
  }),
});
writeActivation('page-count-overflow.json', signedPair(overPageCount));

const maximumManifestEntries = Array.from({ length: 1000 }, (_, index) => ({
  formatVersion: 1,
  privacyPolicyVersion: 1,
  sequence: index + 1,
  version: `manifest-${String(index).padStart(4, '0')}`,
  sha256: 'a'.repeat(64),
  path: `bundles/manifest-${String(index).padStart(4, '0')}.json`,
}));
safeWrite('maximum-manifest-entries-envelope.json', envelope(json(makeManifest(maximumManifestEntries))));
safeWrite('manifest-entry-overflow-envelope.json', envelope(json(makeManifest([
  ...maximumManifestEntries,
  { ...maximumManifestEntries[999], sequence: 1001, version: 'manifest-overflow', path: 'bundles/manifest-overflow.json' },
]))));
safeWrite('duplicate-manifest-entry-envelope.json', envelope(json(makeManifest([
  maximumManifestEntries[0],
  { ...maximumManifestEntries[0], version: 'different-version', path: 'bundles/different-version.json' },
]))));

const maxEnvelopeBytes = 2 * 1024 * 1024;
const maxBundle = makeBundle('maximum-envelope', 30);
const maxBundleSized = exactEnvelope((paddingBytes) => json({ ...maxBundle, padding: 'x'.repeat(paddingBytes) }), maxEnvelopeBytes);
safeWrite('maximum-bundle-envelope.json', maxBundleSized.result);
const maxManifestSized = exactEnvelope((paddingBytes) => json({ ...makeManifest([entry(maxBundle, maxBundleSized.result)]), padding: 'x'.repeat(paddingBytes) }), maxEnvelopeBytes);
safeWrite('maximum-manifest-envelope.json', maxManifestSized.result);
safeWrite('maximum-envelopes-activation.json', activation(maxBundleSized.result, maxManifestSized.result));

const activationOverflow = json({
  ...JSON.parse(activation(valid.bundleEnvelope, valid.manifestEnvelope).toString()),
  overflow: 'x'.repeat(6 * 1024 * 1024),
});
safeWrite('activation-overflow.json', activationOverflow);
const envelopeOverflow = Buffer.alloc(maxEnvelopeBytes + 1, 0x78);
safeWrite('envelope-overflow-envelope.json', envelopeOverflow);
safeWrite('envelope-overflow.json', activation(envelopeOverflow, valid.manifestEnvelope));

const unknownEnvelopeText = valid.bundleEnvelope.toString().replace(/}\s*$/, ',"unknown":true}');
safeWrite('unknown-envelope-field.json', activation(Buffer.from(unknownEnvelopeText), valid.manifestEnvelope));
safeWrite('unknown-activation-field.json', json({ ...JSON.parse(activation(valid.bundleEnvelope, valid.manifestEnvelope).toString()), unknown: true }));

const malformedActivations = {
  'escaped-duplicate-key.json': '{"activationFormatVersion":1,"\\u0061ctivationFormatVersion":1,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}',
  'integer-overflow.json': '{"activationFormatVersion":9007199254740992,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}',
  'lone-surrogate.json': '{"activationFormatVersion":1,"bundleEnvelopeBase64":"\\ud800","manifestEnvelopeBase64":"AA=="}',
  'fractional-number.json': '{"activationFormatVersion":1.0,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}',
  'exponent-number.json': '{"activationFormatVersion":1e0,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}',
  'negative-zero.json': '{"activationFormatVersion":-0,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"AA=="}',
  'too-deep.json': `{"activationFormatVersion":1,"bundleEnvelopeBase64":"AA==","manifestEnvelopeBase64":"${'['.repeat(33)}0${']'.repeat(33)}"}`,
};
for (const [name, contents] of Object.entries(malformedActivations)) safeWrite(name, contents);

safeWrite('test-public-key.pem', publicKey);
console.log(`Generated deterministic synthetic knowledge fixtures in ${fixtureRoot}`);
