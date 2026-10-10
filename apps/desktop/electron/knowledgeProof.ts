import { createHash, createPublicKey, verify } from 'node:crypto';
import { parseStrictJson } from './strictJson.ts';

const MAX_ACTIVATION_BYTES = 6 * 1024 * 1024;
const MAX_ENVELOPE_BYTES = 2 * 1024 * 1024;
const MAX_PAGE_CONTENT_BYTES = 64 * 1024;
const MAX_SAFE_INTEGER = Number.MAX_SAFE_INTEGER;
const SAFE_IDENTIFIER = /^[A-Za-z0-9._-]{1,128}$/;
const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const PAGE_TYPES = new Set([
  'concept',
  'principle',
  'practice',
  'playbook',
  'reference',
  'implementation-mapping',
  'index',
]);

export type KnowledgeActivation = {
  readonly activationFormatVersion: 1;
  readonly bundleEnvelopeBase64: string;
  readonly manifestEnvelopeBase64: string;
};

export type KnowledgePageProvenance = {
  readonly title: string;
  readonly url: string;
};

export type KnowledgePage = {
  readonly id: string;
  readonly title: string;
  readonly type: string;
  readonly status: string;
  readonly content: string;
  readonly sha256: string;
  readonly relatedIds: readonly string[];
  readonly parentId?: string;
  readonly applicability?: readonly string[];
  readonly provenance?: readonly KnowledgePageProvenance[];
};

export type KnowledgeBundle = {
  readonly formatVersion: 1;
  readonly version: string;
  readonly sequence: number;
  readonly publishedAt: string;
  readonly privacyPolicyVersion: 1;
  readonly capabilities: readonly string[];
  readonly pages: readonly KnowledgePage[];
};

export type KnowledgeManifestEntry = {
  readonly formatVersion: number;
  readonly privacyPolicyVersion: number;
  readonly sequence: number;
  readonly version: string;
  readonly sha256: string;
  readonly path: string;
};

export type KnowledgeManifest = {
  readonly formatVersion: 1;
  readonly bundles: readonly KnowledgeManifestEntry[];
};

export type KnowledgeIdentity = {
  readonly bundleSha256: string;
  readonly formatVersion: 1;
  readonly privacyPolicyVersion: 1;
  readonly version: string;
  readonly sequence: number;
  readonly publicKeySha256: string;
};

export type VerifiedKnowledge = {
  readonly activation: KnowledgeActivation;
  readonly bundle: KnowledgeBundle;
  readonly identity: KnowledgeIdentity;
};

type JsonRecord = Record<string, unknown>;

function fail(message: string): never {
  throw new Error(`Invalid Taskmaster knowledge: ${message}`);
}

function record(value: unknown, label: string): JsonRecord {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail(`${label} must be an object`);
  return value as JsonRecord;
}

function requireKeys(value: JsonRecord, required: string[], optional: string[], label: string): void {
  for (const key of required) if (!(key in value)) fail(`${label} is missing ${key}`);
  const accepted = new Set([...required, ...optional]);
  for (const key of Object.keys(value)) if (!accepted.has(key)) fail(`${label} has unknown field ${key}`);
}

function exactKeys(value: JsonRecord, keys: string[], label: string): void {
  if (Object.keys(value).length !== keys.length || keys.some((key) => !(key in value))) {
    fail(`${label} must contain exactly ${keys.join(', ')}`);
  }
}

function string(value: unknown, label: string): string {
  if (typeof value !== 'string') fail(`${label} must be a string`);
  return value;
}

function boundedString(value: unknown, label: string, minBytes: number, maxBytes: number): string {
  const result = string(value, label);
  const length = Buffer.byteLength(result, 'utf8');
  if (length < minBytes || length > maxBytes) fail(`${label} must be between ${minBytes} and ${maxBytes} UTF-8 bytes`);
  return result;
}

function positiveInteger(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 1 || value > MAX_SAFE_INTEGER) {
    fail(`${label} must be a positive safe integer`);
  }
  return value;
}

function canonicalBase64(value: unknown, label: string): Buffer {
  const encoded = string(value, label);
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) {
    fail(`${label} must use canonical padded standard base64`);
  }
  const decoded = Buffer.from(encoded, 'base64');
  if (decoded.toString('base64') !== encoded) fail(`${label} must use canonical padded standard base64`);
  return decoded;
}

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const nested of Object.values(value as Record<string, unknown>)) deepFreeze(nested);
  }
  return value;
}

function verifySignedEnvelope(bytes: Uint8Array, publicKey: string): { payload: Buffer; value: unknown } {
  if (bytes.byteLength > MAX_ENVELOPE_BYTES) fail('signed envelope exceeds 2 MiB');
  const envelope = record(parseStrictJson(bytes, MAX_ENVELOPE_BYTES), 'envelope');
  exactKeys(envelope, ['payload', 'signature'], 'envelope');
  const payloadText = string(envelope.payload, 'envelope.payload');
  const signature = canonicalBase64(envelope.signature, 'envelope.signature');
  if (signature.byteLength !== 64) fail('Ed25519 signature must be 64 bytes');

  const key = createPublicKey(publicKey);
  if (key.asymmetricKeyType !== 'ed25519') fail('trusted public key must be Ed25519');
  const payload = Buffer.from(payloadText, 'utf8');
  if (!verify(null, payload, key, signature)) fail('signature verification failed');
  return { payload, value: parseStrictJson(payload, MAX_ENVELOPE_BYTES) };
}

function safeVersion(value: unknown, label: string): string {
  const version = string(value, label);
  if (!SAFE_IDENTIFIER.test(version)) fail(`${label} must be a safe identifier`);
  return version;
}

function parsePageId(value: unknown): string {
  const id = string(value, 'page.id');
  const relativeMarkdownPath = /^(?:[A-Za-z0-9_][A-Za-z0-9._-]*\/)*[A-Za-z0-9_][A-Za-z0-9._-]*\.md$/;
  if (Buffer.byteLength(id, 'utf8') > 256 || !relativeMarkdownPath.test(id)) {
    fail('page.id must match the safe relative Markdown path grammar and fit within 256 ASCII bytes');
  }
  return id;
}

function stringArray(value: unknown, label: string, maxItems: number, maxItemBytes: number, identifier = false): string[] {
  if (!Array.isArray(value) || value.length > maxItems) fail(`${label} must be an array of at most ${maxItems} strings`);
  const items = value.map((item, index) => boundedString(item, `${label}[${index}]`, 1, maxItemBytes));
  if (new Set(items).size !== items.length) fail(`${label} entries must be unique`);
  if (identifier && items.some((item) => !SAFE_IDENTIFIER.test(item))) fail(`${label} entries must be safe identifiers`);
  return items;
}

function parseProvenance(value: unknown): KnowledgePageProvenance[] {
  if (!Array.isArray(value) || value.length > 16) fail('page.provenance must contain at most 16 records');
  return value.map((item, index) => {
    const provenance = record(item, `page.provenance[${index}]`);
    exactKeys(provenance, ['title', 'url'], `page.provenance[${index}]`);
    const title = boundedString(provenance.title, 'provenance.title', 1, 512);
    const urlText = boundedString(provenance.url, 'provenance.url', 1, 2048);
    let url: URL;
    try {
      url = new URL(urlText);
    } catch {
      return fail('provenance.url must be a valid HTTPS URL');
    }
    if (url.protocol !== 'https:' || url.username || url.password || url.hash) fail('provenance.url must be HTTPS without credentials or fragments');
    return { title, url: urlText };
  });
}

function parseBundle(value: unknown): KnowledgeBundle {
  const source = record(value, 'bundle');
  const version = safeVersion(source.version, 'bundle.version');
  if (source.formatVersion !== 1) fail('bundle.formatVersion must equal 1');
  if (source.privacyPolicyVersion !== 1) fail('bundle.privacyPolicyVersion must equal 1');
  const sequence = positiveInteger(source.sequence, 'bundle.sequence');
  const publishedAt = string(source.publishedAt, 'bundle.publishedAt');
  if (!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/.test(publishedAt) || new Date(publishedAt).toISOString() !== publishedAt) {
    fail('bundle.publishedAt must be a canonical UTC ISO timestamp with milliseconds');
  }
  const capabilities = stringArray(source.capabilities, 'bundle.capabilities', 16, 128, true);
  if (!Array.isArray(source.pages) || source.pages.length < 1 || source.pages.length > 256) fail('bundle.pages must contain 1 to 256 pages');
  const pages: KnowledgePage[] = source.pages.map((item, index) => {
    const page = record(item, `bundle.pages[${index}]`);
    const id = parsePageId(page.id);
    const title = boundedString(page.title, 'page.title', 1, 512);
    const type = boundedString(page.type, 'page.type', 1, 128);
    if (!PAGE_TYPES.has(type)) fail(`page.type ${type} is unsupported`);
    const status = boundedString(page.status, 'page.status', 1, 128);
    const content = string(page.content, 'page.content');
    const contentBytes = Buffer.from(content, 'utf8');
    if (contentBytes.byteLength > MAX_PAGE_CONTENT_BYTES) fail('page.content exceeds 64 KiB');
    const digest = string(page.sha256, 'page.sha256');
    if (!LOWER_SHA256.test(digest) || digest !== sha256(contentBytes)) fail('page.sha256 must match the exact UTF-8 content bytes');
    const relatedIds = stringArray(page.relatedIds, 'page.relatedIds', 256, 256).map(parsePageId);
    const parentId = page.parentId === undefined ? undefined : parsePageId(page.parentId);
    const applicability = page.applicability === undefined ? undefined : stringArray(page.applicability, 'page.applicability', 32, 128);
    const provenance = page.provenance === undefined ? undefined : parseProvenance(page.provenance);
    return {
      id,
      title,
      type,
      status,
      content,
      sha256: digest,
      relatedIds,
      ...(parentId === undefined ? {} : { parentId }),
      ...(applicability === undefined ? {} : { applicability }),
      ...(provenance === undefined ? {} : { provenance }),
    };
  });

  const pageIds = new Set(pages.map((page) => page.id));
  if (pageIds.size !== pages.length) fail('page IDs must be unique');
  for (const page of pages) {
    for (const relatedId of page.relatedIds) if (!pageIds.has(relatedId)) fail(`page ${page.id} relates to missing page ${relatedId}`);
    if (page.parentId !== undefined && (!pageIds.has(page.parentId) || page.parentId === page.id)) fail(`page ${page.id} has an invalid parent`);
  }
  const parents = new Map(pages.map((page) => [page.id, page.parentId]));
  for (const page of pages) {
    const visited = new Set<string>();
    let parent = parents.get(page.id);
    while (parent !== undefined) {
      if (visited.has(parent)) fail('page parent relationships contain a cycle');
      visited.add(parent);
      parent = parents.get(parent);
    }
  }

  return {
    formatVersion: 1,
    version,
    sequence,
    publishedAt,
    privacyPolicyVersion: 1,
    capabilities,
    pages,
  };
}

function parseManifest(value: unknown): KnowledgeManifest {
  const source = record(value, 'manifest');
  if (source.formatVersion !== 1) fail('manifest.formatVersion must equal 1');
  if (!Array.isArray(source.bundles) || source.bundles.length < 1 || source.bundles.length > 1000) fail('manifest.bundles must contain 1 to 1,000 entries');
  const bundles: KnowledgeManifestEntry[] = source.bundles.map((item, index) => {
    const entry = record(item, `manifest.bundles[${index}]`);
    const formatVersion = positiveInteger(entry.formatVersion, 'manifest entry formatVersion');
    const privacyPolicyVersion = positiveInteger(entry.privacyPolicyVersion, 'manifest entry privacyPolicyVersion');
    const sequence = positiveInteger(entry.sequence, 'manifest entry sequence');
    const version = safeVersion(entry.version, 'manifest entry version');
    const digest = string(entry.sha256, 'manifest entry sha256');
    if (!LOWER_SHA256.test(digest)) fail('manifest entry sha256 must be lowercase hexadecimal');
    const path = string(entry.path, 'manifest entry path');
    if (path !== `bundles/${version}.json`) fail('manifest entry path must match its safe version');
    return { formatVersion, privacyPolicyVersion, sequence, version, sha256: digest, path };
  });
  const tupleKeys = new Set<string>();
  const versions = new Set<string>();
  const paths = new Set<string>();
  for (const entry of bundles) {
    const tuple = `${entry.formatVersion}\0${entry.privacyPolicyVersion}\0${entry.sequence}`;
    if (tupleKeys.has(tuple)) fail('manifest has duplicate format/policy/sequence entries');
    if (versions.has(entry.version)) fail('manifest has duplicate versions');
    if (paths.has(entry.path)) fail('manifest has duplicate paths');
    tupleKeys.add(tuple);
    versions.add(entry.version);
    paths.add(entry.path);
  }
  return { formatVersion: 1, bundles };
}

export function verifyKnowledgeManifest(bytes: Uint8Array, publicKey: string): KnowledgeManifest {
  return deepFreeze(parseManifest(verifySignedEnvelope(bytes, publicKey).value));
}

export function selectKnowledgeEntry(manifest: KnowledgeManifest): KnowledgeManifestEntry {
  if (!manifest || !Array.isArray(manifest.bundles)) fail('manifest is invalid');
  const eligible = manifest.bundles.filter((entry) => entry.formatVersion === 1 && entry.privacyPolicyVersion === 1);
  if (eligible.length === 0) fail('manifest has no supported format and policy entry');
  const selected = eligible.reduce((highest, candidate) => candidate.sequence > highest.sequence ? candidate : highest);
  return deepFreeze({ ...selected });
}

export function createKnowledgeActivation(bundle: Uint8Array, manifest: Uint8Array): KnowledgeActivation {
  if (bundle.byteLength > MAX_ENVELOPE_BYTES || manifest.byteLength > MAX_ENVELOPE_BYTES) fail('signed envelopes exceed 2 MiB');
  const activation: KnowledgeActivation = {
    activationFormatVersion: 1,
    bundleEnvelopeBase64: Buffer.from(bundle).toString('base64'),
    manifestEnvelopeBase64: Buffer.from(manifest).toString('base64'),
  };
  if (Buffer.byteLength(JSON.stringify(activation), 'utf8') > MAX_ACTIVATION_BYTES) fail('activation exceeds 6 MiB');
  return deepFreeze(activation);
}

export function verifyKnowledgeActivation(bytes: Uint8Array, publicKey: string): VerifiedKnowledge {
  const activationRecord = record(parseStrictJson(bytes, MAX_ACTIVATION_BYTES), 'activation');
  exactKeys(activationRecord, ['activationFormatVersion', 'bundleEnvelopeBase64', 'manifestEnvelopeBase64'], 'activation');
  if (activationRecord.activationFormatVersion !== 1) fail('activationFormatVersion must equal 1');
  const activation: KnowledgeActivation = {
    activationFormatVersion: 1,
    bundleEnvelopeBase64: string(activationRecord.bundleEnvelopeBase64, 'activation.bundleEnvelopeBase64'),
    manifestEnvelopeBase64: string(activationRecord.manifestEnvelopeBase64, 'activation.manifestEnvelopeBase64'),
  };
  const bundleEnvelope = canonicalBase64(activation.bundleEnvelopeBase64, 'activation.bundleEnvelopeBase64');
  const manifestEnvelope = canonicalBase64(activation.manifestEnvelopeBase64, 'activation.manifestEnvelopeBase64');
  if (bundleEnvelope.byteLength > MAX_ENVELOPE_BYTES || manifestEnvelope.byteLength > MAX_ENVELOPE_BYTES) fail('signed envelopes exceed 2 MiB');

  const bundleSigned = verifySignedEnvelope(bundleEnvelope, publicKey);
  const bundle = parseBundle(bundleSigned.value);
  const manifest = parseManifest(verifySignedEnvelope(manifestEnvelope, publicKey).value);
  const digest = sha256(bundleEnvelope);
  const matches = manifest.bundles.filter((entry) => entry.formatVersion === bundle.formatVersion
    && entry.privacyPolicyVersion === bundle.privacyPolicyVersion
    && entry.sequence === bundle.sequence
    && entry.version === bundle.version
    && entry.sha256 === digest
    && entry.path === `bundles/${bundle.version}.json`);
  if (matches.length !== 1) fail('signed manifest must contain exactly one entry for the activated bundle identity and envelope digest');

  const key = createPublicKey(publicKey);
  const identity: KnowledgeIdentity = {
    bundleSha256: digest,
    formatVersion: bundle.formatVersion,
    privacyPolicyVersion: bundle.privacyPolicyVersion,
    version: bundle.version,
    sequence: bundle.sequence,
    publicKeySha256: sha256(key.export({ format: 'der', type: 'spki' })),
  };
  return deepFreeze({ activation, bundle, identity });
}
