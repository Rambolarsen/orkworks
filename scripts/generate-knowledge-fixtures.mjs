import { createHash, createPublicKey, generateKeyPairSync, sign } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
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
const alternatePrivateKey = readFileSync(path.join(fixtureRoot, 'test-alternate-private-key.pem'));
const alternatePublicKey = readFileSync(path.join(fixtureRoot, 'test-alternate-public-key.pem'));
const derivedAlternatePublicKey = createPublicKey(alternatePrivateKey).export({ format: 'der', type: 'spki' });
const storedAlternatePublicKey = createPublicKey(alternatePublicKey).export({ format: 'der', type: 'spki' });
if (!derivedAlternatePublicKey.equals(storedAlternatePublicKey)) throw new Error('Fixed alternate test key pair does not match');
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

for (const [label, publishedAt] of [
  ['year-zero', '0000-02-29T00:00:00.000Z'],
  ['leap-2000', '2000-02-29T12:34:56.789Z'],
  ['leap-2024', '2024-02-29T23:59:59.999Z'],
  ['year-9999-end', '9999-12-31T23:59:59.999Z'],
]) {
  const bundle = makeBundle(`timestamp-${label}`, 200 + ['year-zero', 'leap-2000', 'leap-2024', 'year-9999-end'].indexOf(label), { publishedAt });
  writeActivation(`timestamp-${label}.json`, signedPair(bundle));
}
for (const [index, [label, url]] of [
  ['at-outside-authority', 'https://example.test/source@user?next=@other&escaped=%40'],
  ['punycode-host', 'https://xn--bcher-kva.example/source/'],
  ['uppercase-utf8-escapes', 'https://example.test/%E2%82%AC?q=%40'],
  ['encoded-hash', 'https://example.test/source%23section/'],
].entries()) {
  const validUrlBundle = makeBundle(`provenance-${label}`, 190 + index);
  validUrlBundle.pages[0].provenance = [{ title: 'source', url }];
  writeActivation(`provenance-${label}.json`, signedPair(validUrlBundle));
}

for (const [index, [label, publishedAt]] of [
  ['february-30', '2024-02-30T00:00:00.000Z'],
  ['nonleap-february-29', '1900-02-29T00:00:00.000Z'],
  ['month-zero', '2024-00-01T00:00:00.000Z'],
  ['month-thirteen', '2024-13-01T00:00:00.000Z'],
  ['day-zero', '2024-01-00T00:00:00.000Z'],
  ['day-thirty-two', '2024-01-32T00:00:00.000Z'],
  ['hour-twenty-four', '2024-01-01T24:00:00.000Z'],
  ['minute-sixty', '2024-01-01T00:60:00.000Z'],
  ['second-sixty', '2024-01-01T00:00:60.000Z'],
  ['lowercase', '2024-01-01t00:00:00.000z'],
  ['offset', '2024-01-01T00:00:00.000+00:00'],
  ['expanded-year', '+010000-02-29T00:00:00.000Z'],
  ['missing-milliseconds', '2024-01-01T00:00:00Z'],
  ['four-fraction-digits', '2024-01-01T00:00:00.0000Z'],
  ['space-separator', '2024-01-01 00:00:00.000Z'],
].entries()) {
  const bundle = makeBundle(`timestamp-invalid-${label}`, 210 + index, { publishedAt });
  writeActivation(`timestamp-invalid-${label}.json`, signedPair(bundle));
}

const unsigned = activation(json(valid.bundle), valid.manifestEnvelope);
safeWrite('unsigned-bundle.json', unsigned);

const wrongKeyPair = signedPair(makeBundle('wrong-key', 2), { key: alternatePrivateKey });
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

for (const [index, [label, field]] of [
  ['parent-null', 'parentId'],
  ['applicability-null', 'applicability'],
  ['provenance-null', 'provenance'],
].entries()) {
  const invalidOptional = makeBundle(`invalid-${label}`, 220 + index);
  invalidOptional.pages[0][field] = null;
  writeActivation(`invalid-${label}.json`, signedPair(invalidOptional));
}

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

const overProvenanceUrl = makeBundle('over-provenance-url', 222);
overProvenanceUrl.pages[0].provenance = [{ title: 'source', url: `https://example.test/${'x'.repeat(2049 - Buffer.byteLength('https://example.test/'))}` }];
writeActivation('provenance-url-overflow.json', signedPair(overProvenanceUrl));

const overProvenance = makeBundle('over-provenance', 25);
overProvenance.pages[0].provenance = Array.from({ length: 17 }, () => ({ title: 'source', url: 'https://example.test/source' }));
writeActivation('provenance-overflow.json', signedPair(overProvenance));

for (const [label, url, sequence] of [
  ['credentials', 'https://user:pass@example.test/source', 68],
  ['fragment', 'https://example.test/source#fragment', 69],
  ['http', 'http://example.test/source', 70],
  ['empty-fragment', 'https://example.test/source#', 71],
  ['empty-credentials', 'https://@example.test/source', 72],
  ['uppercase-scheme', 'HTTPS://example.test/source/', 73],
  ['uppercase-host', 'https://EXAMPLE.test/source/', 74],
  ['default-port', 'https://example.test:443/source/', 75],
  ['missing-root-slash', 'https://example.test', 76],
  ['dot-segment', 'https://example.test/a/../source/', 77],
  ['lowercase-escape', 'https://example.test/source%2f/', 78],
  ['invalid-escape', 'https://example.test/source%GG/', 79],
  ['short-escape', 'https://example.test/source%2/', 80],
  ['backslash', 'https:\\\\example.test\\source/', 81],
  ['unicode-host', 'https://café.example/source/', 82],
  ['unicode-path', 'https://example.test/café/', 83],
  ['space', 'https://example.test/source name/', 84],
  ['control', 'https://example.test/source\nname/', 85],
  ['empty-url', '', 86],
]) {
  const invalidProvenance = makeBundle(`invalid-provenance-${label}`, sequence);
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
const malformedManifestValues = [
  ['sequence-string', { sequence: '2' }],
  ['sequence-fraction', { sequence: 1.5 }],
  ['sequence-overflow', { sequence: 9007199254740992 }],
  ['version-nonstring', { version: 7 }],
  ['version-empty', { version: '' }],
  ['version-overflow', { version: 'v'.repeat(129) }],
  ['version-unsafe', { version: '../bad' }],
  ['digest-nonstring', { sha256: 7 }],
  ['digest-uppercase', { sha256: 'A'.repeat(64) }],
  ['digest-short', { sha256: 'a'.repeat(63) }],
  ['path-nonstring', { path: 7 }],
  ['path-mismatch', { path: 'bundles/other.json' }],
  ['format-missing', { formatVersion: undefined }],
  ['format-noninteger', { formatVersion: 1.5 }],
  ['policy-missing', { privacyPolicyVersion: undefined }],
  ['policy-noninteger', { privacyPolicyVersion: 1.5 }],
];
for (const [label, override] of malformedManifestValues) {
  const invalidEntry = { ...maximumManifestEntries[0], formatVersion: 2, privacyPolicyVersion: 3, ...override };
  safeWrite(`manifest-invalid-${label}-envelope.json`, envelope(json(makeManifest([maximumManifestEntries[0], invalidEntry]))));
}

const maxEnvelopeBytes = 2 * 1024 * 1024;
const maxBundle = makeBundle('maximum-envelope', 30);
const maxBundleSized = exactEnvelope((paddingBytes) => json({ ...maxBundle, padding: 'x'.repeat(paddingBytes) }), maxEnvelopeBytes);
safeWrite('maximum-bundle-envelope.json', maxBundleSized.result);
const maxManifestSized = exactEnvelope((paddingBytes) => json({ ...makeManifest([entry(maxBundle, maxBundleSized.result)]), padding: 'x'.repeat(paddingBytes) }), maxEnvelopeBytes);
safeWrite('maximum-manifest-envelope.json', maxManifestSized.result);


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


const sequenceZero = signedPair(makeBundle('sequence-zero', 0));
writeActivation('sequence-zero.json', sequenceZero);
safeWrite('manifest-sequence-zero-envelope.json', sequenceZero.manifestEnvelope);
const sequenceMax = signedPair(makeBundle('sequence-max-safe', Number.MAX_SAFE_INTEGER));
writeActivation('sequence-max-safe.json', sequenceMax);

const escapedBundle = makeBundle('escaped-surrogate-content', 70);
escapedBundle.pages[0].content = '😀';
escapedBundle.pages[0].sha256 = hash(Buffer.from('😀'));
const escapedPayload = Buffer.from(JSON.stringify(escapedBundle).replace(JSON.stringify('😀'), '"\\uD83D\\uDE00"'));
writeActivation('escaped-surrogate-content.json', signedPair(escapedBundle, { payloadBytes: escapedPayload }));

const bom = Buffer.from([0xef, 0xbb, 0xbf]);
safeWrite('bom-activation.json', Buffer.concat([bom, activation(valid.bundleEnvelope, valid.manifestEnvelope)]));
const bomEnvelope = Buffer.concat([bom, valid.bundleEnvelope]);
writeActivation('bom-envelope-activation.json', signedPair(valid.bundle, { bundleEnvelope: bomEnvelope }));
const bomPayload = Buffer.concat([bom, json(makeBundle('bom-payload', 71))]);
writeActivation('bom-payload-activation.json', signedPair(makeBundle('bom-payload', 71), { payloadBytes: bomPayload }));

const boundary = makeBundle('v'.repeat(128), 72, {
  capabilities: ['c'.repeat(128)],
  pages: [{
    id: 'boundary.md', title: 'Boundary', type: 'concept', status: 'active', content: 'Boundary',
    sha256: hash(Buffer.from('Boundary')), relatedIds: [], applicability: ['a'.repeat(128)],
  }],
});
writeActivation('string-boundaries-128.json', signedPair(boundary));
const capability129 = makeBundle('capability-129', 73, { capabilities: ['c'.repeat(129)] });
writeActivation('capability-129.json', signedPair(capability129));
const applicability129 = makeBundle('applicability-129', 74, { pages: [{
  ...makeBundle('applicability-129', 74).pages[0], applicability: ['a'.repeat(129)],
}] });
writeActivation('applicability-129.json', signedPair(applicability129));
writeActivation('version-129.json', signedPair(makeBundle('v'.repeat(129), 75)));

const withUnknownPayload = (version, suffix) => {
  const bundle = makeBundle(version, 76);
  const payloadBytes = Buffer.from(`${JSON.stringify(bundle).slice(0, -1)},${suffix}}`);
  return signedPair(bundle, { payloadBytes });
};
writeActivation('unknown-valid-fields.json', withUnknownPayload('unknown-valid-fields', '"future":{"nested":[0,true,null]}'));
writeActivation('unknown-nested-duplicate.json', withUnknownPayload('unknown-nested-duplicate', '"future":{"nested":{"a":1,"\\u0061":2}}'));
for (const [name, token] of [
  ['unknown-fraction', '1.0'],
  ['unknown-exponent', '1e0'],
  ['unknown-negative-zero', '-0'],
  ['unknown-overflow', '9007199254740992'],
]) {
  writeActivation(`${name}.json`, withUnknownPayload(name, `"future":{"number":${token}}`));
}

safeWrite('test-public-key.pem', publicKey);

const fixtureOutcomes = [
  { api: 'verifyKnowledgeActivation', expected: 'accept', invariant: 'valid signed proof, immutable snapshot, idempotent re-attestation, or inclusive schema limit', files: ['valid-activation.json', 'same-bundle-newer-manifest.json', 'maximum-id-length.json', 'maximum-page-limits.json', 'maximum-capabilities.json', 'sequence-zero.json', 'sequence-max-safe.json', 'escaped-surrogate-content.json', 'string-boundaries-128.json', 'unknown-valid-fields.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'unsigned or incorrectly attested content cannot activate', files: ['unsigned-bundle.json', 'wrong-key.json', 'wrong-signature.json', 'wrong-digest.json', 'wrong-policy.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'activation, envelope, UTF-8, base64, BOM, and signed-payload schema rules are strict', files: ['duplicate-envelope-field.json', 'duplicate-payload-field.json', 'unknown-envelope-field.json', 'unknown-activation-field.json', 'malformed-utf8.json', 'noncanonical-base64.json', 'bom-activation.json', 'bom-envelope-activation.json', 'bom-payload-activation.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'page metadata, relationships, IDs, and provenance satisfy the signed bundle contract', files: ['invalid-metadata.json', 'invalid-parent-null.json', 'invalid-applicability-null.json', 'invalid-provenance-null.json', 'invalid-page-digest.json', 'missing-related-page.json', 'duplicate-related-ids.json', 'unsupported-page-type.json', 'invalid-provenance-credentials.json', 'invalid-provenance-fragment.json', 'invalid-provenance-empty-fragment.json', 'invalid-provenance-empty-credentials.json', 'invalid-provenance-uppercase-scheme.json', 'invalid-provenance-uppercase-host.json', 'invalid-provenance-default-port.json', 'invalid-provenance-missing-root-slash.json', 'invalid-provenance-dot-segment.json', 'invalid-provenance-lowercase-escape.json', 'invalid-provenance-invalid-escape.json', 'invalid-provenance-short-escape.json', 'invalid-provenance-backslash.json', 'invalid-provenance-unicode-host.json', 'invalid-provenance-unicode-path.json', 'invalid-provenance-space.json', 'invalid-provenance-control.json', 'invalid-provenance-empty-url.json', 'invalid-provenance-http.json', 'provenance-url-overflow.json', 'unsafe-id.json', 'unsafe-related-id.json', 'parent-cycle.json', 'duplicate-page-ids.json', 'invalid-id-space.json', 'invalid-id-colon.json', 'invalid-id-control.json', 'invalid-id-unicode.json', 'invalid-id-percent.json', 'invalid-id-backslash.json', 'invalid-id-emptySegment.json', 'invalid-id-dotSegment.json', 'invalid-id-dotdotSegment.json', 'invalid-id-leadingDotSegment.json', 'invalid-id-dotFileSegment.json', 'invalid-id-uppercaseExtension.json', 'invalid-id-leadingSlash.json', 'invalid-id-leadingHyphen.json', 'invalid-id-tooLong.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'page, text, relationship, provenance, and marker limits reject overflow', files: ['capability-overflow.json', 'content-overflow.json', 'title-overflow.json', 'status-overflow.json', 'applicability-overflow.json', 'provenance-overflow.json', 'page-count-overflow.json', 'capability-129.json', 'applicability-129.json', 'version-129.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'all signed JSON fields, including unknown nested fields, use strict decoded-key and integer lexical rules', files: ['escaped-duplicate-key.json', 'integer-overflow.json', 'lone-surrogate.json', 'fractional-number.json', 'exponent-number.json', 'negative-zero.json', 'too-deep.json', 'unknown-nested-duplicate.json', 'unknown-fraction.json', 'unknown-exponent.json', 'unknown-negative-zero.json', 'unknown-overflow.json'] },
  { api: 'verifyKnowledgeManifest', expected: 'accept', invariant: 'manifest sequence zero and inclusive entry-count bound are valid', files: ['manifest-sequence-zero-envelope.json', 'maximum-manifest-entries-envelope.json'] },
  { api: 'verifyKnowledgeManifest', expected: 'reject', invariant: 'manifest entry-count overflow and duplicate identities are invalid', files: ['manifest-entry-overflow-envelope.json', 'duplicate-manifest-entry-envelope.json'] },
  { api: 'verifyKnowledgeManifest', expected: 'reject', invariant: 'every entry is structurally validated before supported format and policy selection', files: ['manifest-invalid-sequence-string-envelope.json', 'manifest-invalid-sequence-fraction-envelope.json', 'manifest-invalid-sequence-overflow-envelope.json', 'manifest-invalid-version-nonstring-envelope.json', 'manifest-invalid-version-empty-envelope.json', 'manifest-invalid-version-overflow-envelope.json', 'manifest-invalid-version-unsafe-envelope.json', 'manifest-invalid-digest-nonstring-envelope.json', 'manifest-invalid-digest-uppercase-envelope.json', 'manifest-invalid-digest-short-envelope.json', 'manifest-invalid-path-nonstring-envelope.json', 'manifest-invalid-path-mismatch-envelope.json', 'manifest-invalid-format-missing-envelope.json', 'manifest-invalid-format-noninteger-envelope.json', 'manifest-invalid-policy-missing-envelope.json', 'manifest-invalid-policy-noninteger-envelope.json'] },
  { api: 'verifyKnowledgeManifest + selectKnowledgeEntry', expected: 'accept', invariant: 'eligible format and policy are selected before sequence ranking', files: ['ranked-manifest-envelope.json'] },
  { api: 'createKnowledgeActivation', expected: 'accept', invariant: 'helper preserves exact signed-envelope bytes and wire fields', files: ['valid-bundle-envelope.json', 'valid-manifest-envelope.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'accept', invariant: 'both original signed envelopes at the exact 2 MiB inclusive bound remain valid', files: ['maximum-bundle-envelope.json', 'maximum-manifest-envelope.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'accept', invariant: 'canonical UTC timestamps include year zero, leap days, and the year 9999 endpoint', files: ['timestamp-year-zero.json', 'timestamp-leap-2000.json', 'timestamp-leap-2024.json', 'timestamp-year-9999-end.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'accept', invariant: 'canonical HTTPS provenance URLs preserve punycode and uppercase percent escapes', files: ['provenance-at-outside-authority.json', 'provenance-punycode-host.json', 'provenance-uppercase-utf8-escapes.json', 'provenance-encoded-hash.json'] },
  { api: 'verifyKnowledgeActivation', expected: 'reject', invariant: 'invalid or noncanonical UTC timestamps are rejected', files: ['timestamp-invalid-february-30.json', 'timestamp-invalid-nonleap-february-29.json', 'timestamp-invalid-month-zero.json', 'timestamp-invalid-month-thirteen.json', 'timestamp-invalid-day-zero.json', 'timestamp-invalid-day-thirty-two.json', 'timestamp-invalid-hour-twenty-four.json', 'timestamp-invalid-minute-sixty.json', 'timestamp-invalid-second-sixty.json', 'timestamp-invalid-lowercase.json', 'timestamp-invalid-offset.json', 'timestamp-invalid-expanded-year.json', 'timestamp-invalid-missing-milliseconds.json', 'timestamp-invalid-four-fraction-digits.json', 'timestamp-invalid-space-separator.json'] },
  { api: 'generated size cases in desktop and Rust tests', expected: 'case-specific', invariant: 'construct wrappers in memory: accept two 2 MiB envelopes together; reject activation over 6 MiB and any envelope over 2 MiB', files: [] },
  { api: 'test support values', expected: 'accept', invariant: 'expected identity digests for the valid synthetic bundle and public test key', files: ['valid-envelope.sha256', 'test-public-key-spki.sha256'] },
];
const indexedFixtures = new Set(fixtureOutcomes.flatMap((group) => group.files));
const fixtureFiles = readdirSync(fixtureRoot).filter((name) => /\.(?:json|sha256)$/.test(name) && name !== 'fixture-index.json').sort();
const unindexedFixtures = fixtureFiles.filter((name) => !indexedFixtures.has(name));
const missingFixtures = [...indexedFixtures].filter((name) => !fixtureFiles.includes(name));
if (unindexedFixtures.length || missingFixtures.length) {
  throw new Error(`Fixture index mismatch: unindexed=${unindexedFixtures.join(',')} missing=${missingFixtures.join(',')}`);
}
safeWrite('fixture-index.json', json({ formatVersion: 1, groups: fixtureOutcomes }));

console.log(`Generated deterministic synthetic knowledge fixtures in ${fixtureRoot}`);
