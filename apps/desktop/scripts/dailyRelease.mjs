import { createHash } from "node:crypto";
import yaml from "js-yaml";

const STABLE_TAG_PATTERN = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const STABLE_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const NIGHTLY_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-nightly\.(\d{8})\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const SOURCE_MARKER_PATTERN = /^<!-- orkworks-nightly-source:([0-9a-f]{40}) -->$/gm;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;

function requireCanonicalPositiveInteger(value, name, maximum = null) {
  if (typeof value !== "string" || !/^[1-9]\d*$/.test(value)) {
    throw new Error(`${name} must be a canonical positive integer`);
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || (maximum !== null && parsed > maximum)) {
    throw new Error(`${name} is outside its supported range`);
  }
  return parsed;
}

function requireStableVersion(version, name = "version") {
  if (typeof version !== "string" || !STABLE_VERSION_PATTERN.test(version)) {
    throw new Error(`${name} is invalid`);
  }
  return version;
}

export function parseStableTag(tag, packageVersion) {
  const match = typeof tag === "string" ? STABLE_TAG_PATTERN.exec(tag) : null;
  if (!match) {
    throw new Error("release tag must be a canonical stable tag");
  }
  const version = match.slice(1).join(".");
  if (packageVersion !== version) {
    throw new Error(`release tag version ${version} does not match package version ${packageVersion}`);
  }
  return { version };
}

export function createNightlyIdentity({
  baseVersion,
  utcDate,
  runId,
  runNumber,
  runAttempt,
}) {
  requireStableVersion(baseVersion, "base version");
  if (!(utcDate instanceof Date) || !Number.isFinite(utcDate.getTime())) {
    throw new Error("UTC date is invalid");
  }
  requireCanonicalPositiveInteger(runId, "run ID");
  const parsedRunNumber = requireCanonicalPositiveInteger(runNumber, "run number", 65535);
  const parsedAttempt = requireCanonicalPositiveInteger(runAttempt, "run attempt", 99);

  const year = utcDate.getUTCFullYear();
  if (year < 0 || year > 65535) {
    throw new Error("UTC year is outside the Windows build-version range");
  }
  const yearStart = Date.UTC(year, 0, 1);
  const dayOfYear = Math.floor((utcDate.getTime() - yearStart) / 86_400_000) + 1;
  const date = [
    String(year).padStart(4, "0"),
    String(utcDate.getUTCMonth() + 1).padStart(2, "0"),
    String(utcDate.getUTCDate()).padStart(2, "0"),
  ].join("");
  const version = `${baseVersion}-nightly.${date}.${runId}.${runAttempt}`;

  const ordinal = (parsedRunNumber - 1) * 100 + parsedAttempt;
  const macMajor = Math.floor(ordinal / 10_000) + 1;
  const macMinor = Math.floor((ordinal % 10_000) / 100);
  const macPatch = ordinal % 100;
  if (macMajor > 9999 || macMinor > 99 || macPatch > 99) {
    throw new Error("macOS bundle version is outside its supported range");
  }

  return {
    version,
    tag: `v${version}`,
    windowsBuildVersion: `${year}.${dayOfYear}.${parsedRunNumber}.${parsedAttempt}`,
    macBundleVersion: `${macMajor}.${macMinor}.${macPatch}`,
  };
}

export function sourceMarker(sourceSha) {
  if (typeof sourceSha !== "string" || !/^[0-9a-f]{40}$/.test(sourceSha)) {
    throw new Error("source SHA must be 40 lowercase hexadecimal characters");
  }
  return `<!-- orkworks-nightly-source:${sourceSha} -->`;
}

export function parseSourceMarker(body) {
  if (typeof body !== "string") {
    throw new Error("release body must be a string");
  }
  const matches = [...body.matchAll(SOURCE_MARKER_PATTERN)].map((match) => match[1]);
  if (matches.length > 1) {
    throw new Error("release body contains multiple source markers");
  }
  return matches[0] ?? null;
}

export function expectedReleaseAssetNames({ version, channel }) {
  if (channel !== "latest" && channel !== "nightly") {
    throw new Error("release channel must be latest or nightly");
  }
  if (channel === "latest") {
    requireStableVersion(version);
  } else if (!NIGHTLY_VERSION_PATTERN.test(version)) {
    throw new Error("nightly release version is invalid");
  }
  return [
    `OrkWorks-${version}-mac-arm64.dmg`,
    `OrkWorks-${version}-mac-arm64.zip`,
    `OrkWorks-${version}-mac-arm64.zip.blockmap`,
    `OrkWorks-${version}-win-x64.exe`,
    `OrkWorks-${version}-win-x64.exe.blockmap`,
    `${channel}-mac.yml`,
    `${channel}.yml`,
    "SHA256SUMS.txt",
  ].sort();
}

function requireNightlyTag(tag) {
  if (typeof tag !== "string" || !tag.startsWith("v")) {
    throw new Error("nightly release tag is invalid");
  }
  const version = tag.slice(1);
  if (!NIGHTLY_VERSION_PATTERN.test(version)) {
    throw new Error("nightly release tag is invalid");
  }
  return version;
}

function parseChecksums(value) {
  if (typeof value !== "string" || value.length === 0 || !value.endsWith("\n")) {
    throw new Error("checksum manifest is invalid");
  }
  const entries = new Map();
  for (const line of value.trimEnd().split("\n")) {
    const match = /^([0-9a-f]{64})  ([^/\\\0]+)$/.exec(line);
    if (!match || entries.has(match[2])) {
      throw new Error("checksum manifest contains an invalid entry");
    }
    entries.set(match[2], match[1]);
  }
  return entries;
}

function requireMetadata({ value, name, version, payloadName, assetsByName }) {
  let metadata;
  try {
    metadata = yaml.load(value);
  } catch (error) {
    throw new Error(`${name} metadata is invalid`, { cause: error });
  }
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) {
    throw new Error(`${name} metadata is invalid`);
  }
  if (metadata.version !== version || !Array.isArray(metadata.files) || metadata.files.length !== 1) {
    throw new Error(`${name} metadata has the wrong version or file set`);
  }
  const entry = metadata.files[0];
  const asset = assetsByName.get(payloadName);
  if (
    !entry
    || typeof entry !== "object"
    || Array.isArray(entry)
    || entry.url !== payloadName
    || typeof entry.sha512 !== "string"
    || entry.sha512.length === 0
    || entry.size !== asset?.size
  ) {
    throw new Error(`${name} metadata does not match its updater payload`);
  }
}

export function validatePublishedRelease({
  release,
  sourceSha,
  expectedAssetNames,
  tagTargetSha,
  downloadedAssets,
}) {
  sourceMarker(sourceSha);
  if (!release || typeof release !== "object" || release.draft !== false || release.prerelease !== true) {
    throw new Error("nightly must be a published prerelease");
  }
  const version = requireNightlyTag(release.tag_name);
  if (tagTargetSha !== sourceSha) {
    throw new Error("nightly tag target does not match its source SHA");
  }
  if (parseSourceMarker(release.body) !== sourceSha) {
    throw new Error("nightly release source marker does not match its source SHA");
  }
  if (!Array.isArray(expectedAssetNames) || new Set(expectedAssetNames).size !== expectedAssetNames.length) {
    throw new Error("expected release asset set is invalid");
  }
  const canonicalExpectedNames = expectedReleaseAssetNames({ version, channel: "nightly" });
  if (expectedAssetNames.toSorted().join("\n") !== canonicalExpectedNames.join("\n")) {
    throw new Error("expected release asset set does not match the nightly identity");
  }
  if (!Array.isArray(release.assets)) {
    throw new Error("release assets are invalid");
  }
  const assetsByName = new Map();
  for (const asset of release.assets) {
    if (
      !asset
      || typeof asset !== "object"
      || typeof asset.name !== "string"
      || assetsByName.has(asset.name)
      || !Number.isInteger(asset.size)
      || asset.size <= 0
      || typeof asset.digest !== "string"
      || !asset.digest.startsWith("sha256:")
      || !SHA256_PATTERN.test(asset.digest.slice(7))
    ) {
      throw new Error("release contains an invalid asset");
    }
    assetsByName.set(asset.name, asset);
  }
  if (
    assetsByName.size !== canonicalExpectedNames.length
    || canonicalExpectedNames.some((name) => !assetsByName.has(name))
  ) {
    throw new Error("release asset set is incomplete or unexpected");
  }

  const requiredDownloads = ["nightly.yml", "nightly-mac.yml", "SHA256SUMS.txt"];
  for (const name of requiredDownloads) {
    const value = downloadedAssets?.[name];
    if (typeof value !== "string") {
      throw new Error(`release ${name} asset was not downloaded`);
    }
    const digest = sha256(value);
    if (assetsByName.get(name).digest !== `sha256:${digest}`) {
      throw new Error(`release ${name} asset digest does not match its download`);
    }
  }

  const checksums = parseChecksums(downloadedAssets["SHA256SUMS.txt"]);
  const checksummedNames = canonicalExpectedNames.filter((name) => name !== "SHA256SUMS.txt");
  if (
    checksums.size !== checksummedNames.length
    || checksummedNames.some((name) => checksums.get(name) !== assetsByName.get(name).digest.slice(7))
  ) {
    throw new Error("checksum manifest does not match the release assets");
  }

  requireMetadata({
    value: downloadedAssets["nightly.yml"],
    name: "nightly.yml",
    version,
    payloadName: `OrkWorks-${version}-win-x64.exe`,
    assetsByName,
  });
  requireMetadata({
    value: downloadedAssets["nightly-mac.yml"],
    name: "nightly-mac.yml",
    version,
    payloadName: `OrkWorks-${version}-mac-arm64.zip`,
    assetsByName,
  });

  return { release, sourceSha, tag: release.tag_name, version };
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

export function selectPublishedNightlyForSource(validatedReleases, sourceSha) {
  sourceMarker(sourceSha);
  if (!Array.isArray(validatedReleases)) {
    throw new Error("validated releases must be an array");
  }
  const matches = validatedReleases.filter((release) => release?.sourceSha === sourceSha);
  if (matches.length > 1) {
    throw new Error("multiple published nightlies match the source SHA");
  }
  return matches[0] ?? null;
}

function parseNightlyVersion(version) {
  const match = typeof version === "string" ? NIGHTLY_VERSION_PATTERN.exec(version) : null;
  if (!match) {
    throw new Error(`nightly version is invalid: ${version}`);
  }
  return match.slice(1).map((component) => BigInt(component));
}

function compareNightlyVersions(left, right) {
  const leftParts = parseNightlyVersion(left);
  const rightParts = parseNightlyVersion(right);
  for (let index = 0; index < leftParts.length; index += 1) {
    if (leftParts[index] < rightParts[index]) return -1;
    if (leftParts[index] > rightParts[index]) return 1;
  }
  return 0;
}

export function assertCandidateIsNewest(candidateVersion, validatedPublishedNightlies) {
  parseNightlyVersion(candidateVersion);
  if (!Array.isArray(validatedPublishedNightlies)) {
    throw new Error("published nightlies must be an array");
  }
  for (const release of validatedPublishedNightlies) {
    if (compareNightlyVersions(candidateVersion, release?.version) <= 0) {
      throw new Error("candidate must be newer than every published nightly");
    }
  }
}

function githubHeaders(token) {
  if (typeof token !== "string" || token.length === 0) {
    throw new Error("GitHub release token is required");
  }
  return {
    accept: "application/vnd.github+json",
    authorization: `Bearer ${token}`,
    "x-github-api-version": "2022-11-28",
  };
}

function repositoryApiBase(repository) {
  if (typeof repository !== "string" || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error("GitHub repository must be owner/name");
  }
  return `https://api.github.com/repos/${repository}`;
}

async function readJson(response, context) {
  let body;
  try {
    body = await response.json();
  } catch (error) {
    throw new Error(`GitHub API returned invalid JSON for ${context}`, { cause: error });
  }
  if (!response.ok) {
    throw new Error(`GitHub API ${context} failed with ${response.status}`);
  }
  return body;
}

function nextLink(header) {
  if (!header) return null;
  for (const part of header.split(",")) {
    const match = /^\s*<([^>]+)>;\s*rel="([^"]+)"\s*$/.exec(part);
    if (match?.[2] === "next") return match[1];
  }
  return null;
}

export async function listAllReleases({ repository, token, fetchImpl = fetch }) {
  const headers = githubHeaders(token);
  let url = `${repositoryApiBase(repository)}/releases?per_page=100`;
  const releases = [];
  while (url !== null) {
    const response = await fetchImpl(url, { headers });
    const page = await readJson(response, "release list");
    if (!Array.isArray(page)) {
      throw new Error("GitHub release list must be an array");
    }
    releases.push(...page);
    url = nextLink(response.headers.get("link"));
  }
  return releases;
}

async function readTag({ repository, token, tag, fetchImpl }) {
  const response = await fetchImpl(
    `${repositoryApiBase(repository)}/git/ref/tags/${encodeURIComponent(tag)}`,
    { headers: githubHeaders(token) },
  );
  if (response.status === 404) return null;
  const ref = await readJson(response, `read tag ${tag}`);
  if (ref?.ref !== `refs/tags/${tag}` || !ref.object || typeof ref.object.sha !== "string") {
    throw new Error(`GitHub tag ${tag} has an invalid schema`);
  }
  if (ref.object.type === "commit") return ref.object.sha;
  if (ref.object.type !== "tag") throw new Error(`GitHub tag ${tag} has an unsupported target`);
  const tagResponse = await fetchImpl(`${repositoryApiBase(repository)}/git/tags/${ref.object.sha}`, {
    headers: githubHeaders(token),
  });
  const annotated = await readJson(tagResponse, `dereference tag ${tag}`);
  if (annotated?.object?.type !== "commit" || typeof annotated.object.sha !== "string") {
    throw new Error(`GitHub annotated tag ${tag} has an invalid target`);
  }
  return annotated.object.sha;
}

export async function ensureTagAtSource({ repository, token, tag, sourceSha, fetchImpl = fetch }) {
  requireNightlyTag(tag);
  sourceMarker(sourceSha);
  const existing = await readTag({ repository, token, tag, fetchImpl });
  if (existing !== null) {
    if (existing !== sourceSha) throw new Error(`tag ${tag} points at a different source SHA`);
    return { tag, sourceSha };
  }

  for (let attempt = 1; attempt <= 3; attempt += 1) {
    let response;
    try {
      response = await fetchImpl(`${repositoryApiBase(repository)}/git/refs`, {
        method: "POST",
        headers: { ...githubHeaders(token), "content-type": "application/json" },
        body: JSON.stringify({ ref: `refs/tags/${tag}`, sha: sourceSha }),
      });
    } catch {
      response = null;
    }
    if (response?.status === 201) {
      await readJson(response, `create tag ${tag}`);
    }
    const observed = await readTag({ repository, token, tag, fetchImpl });
    if (observed === sourceSha) return { tag, sourceSha };
    if (observed !== null) throw new Error(`tag ${tag} points at a different source SHA`);
  }
  throw new Error(`GitHub tag ${tag} was still absent after three creation attempts`);
}
