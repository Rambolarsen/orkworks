import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

import {
  assertCandidateIsNewest,
  expectedReleaseAssetNames,
  getTagTarget,
  selectPublishedNightlyForSource,
  sourceMarker,
  validateReleaseIntegrity,
} from "./dailyRelease.mjs";
import { loadNightlyReleaseState } from "./prepareDailyRelease.mjs";

function headers(token, contentType = "application/vnd.github+json") {
  if (typeof token !== "string" || token.length === 0) throw new Error("GitHub release token is required");
  return {
    accept: "application/vnd.github+json",
    authorization: `Bearer ${token}`,
    "content-type": contentType,
    "x-github-api-version": "2022-11-28",
  };
}

async function json(response, context, expectedStatus = 200) {
  if (response.status !== expectedStatus) {
    throw new Error(`GitHub API ${context} failed with ${response.status}`);
  }
  try {
    return await response.json();
  } catch (error) {
    throw new Error(`GitHub API returned invalid JSON for ${context}`, { cause: error });
  }
}

function apiBase(repository) {
  if (typeof repository !== "string" || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error("GitHub repository must be owner/name");
  }
  return `https://api.github.com/repos/${repository}`;
}

export function readReleaseAssets({ directory, version }) {
  const root = realpathSync(directory);
  const expectedNames = expectedReleaseAssetNames({ version, channel: "nightly" });
  const actualNames = readdirSync(root).sort();
  if (actualNames.join("\n") !== expectedNames.join("\n")) {
    throw new Error("release directory asset set is incomplete or unexpected");
  }
  return Object.fromEntries(expectedNames.map((name) => {
    const path = join(root, name);
    const stats = lstatSync(path);
    const realPath = realpathSync(path);
    const relativePath = relative(root, realPath);
    if (
      !stats.isFile()
      || stats.isSymbolicLink()
      || relativePath === ".."
      || relativePath.startsWith(`..${sep}`)
      || resolve(realPath) !== resolve(path)
    ) {
      throw new Error(`release asset must be a contained regular file: ${name}`);
    }
    return [name, readFileSync(realPath)];
  }));
}

export async function publishDailyRelease({
  repository,
  token,
  identity,
  sourceSha,
  assets,
  fetchImpl = fetch,
  loadState = loadNightlyReleaseState,
}) {
  const expectedNames = expectedReleaseAssetNames({ version: identity?.version, channel: "nightly" });
  if (identity?.tag !== `v${identity.version}`) throw new Error("nightly release identity is invalid");
  const assetNames = Object.keys(assets ?? {}).sort();
  if (assetNames.join("\n") !== expectedNames.join("\n")) throw new Error("local release asset set is incomplete or unexpected");
  for (const name of assetNames) {
    if (!Buffer.isBuffer(assets[name]) || assets[name].length === 0) throw new Error(`local release asset is empty: ${name}`);
  }
  const state = await loadState({ repository, token, sourceSha, fetchImpl });
  const existing = selectPublishedNightlyForSource(state.validated, sourceSha);
  if (existing) return existing.release;
  const tagTargetSha = await getTagTarget({ repository, token, tag: identity.tag, fetchImpl });
  if (tagTargetSha !== sourceSha) throw new Error("nightly tag target does not match its source SHA");
  assertCandidateIsNewest(
    identity.version,
    state.publishedNightlyVersions
      .filter((version) => version !== identity.version)
      .map((version) => ({ version })),
  );

  const base = apiBase(repository);
  const created = await json(await fetchImpl(`${base}/releases`, {
    method: "POST",
    headers: headers(token),
    body: JSON.stringify({
      tag_name: identity.tag,
      name: `OrkWorks ${identity.version}`,
      body: `Automated daily build from main.\n\n${sourceMarker(sourceSha)}\n`,
      draft: true,
      prerelease: true,
    }),
  }), "create draft release", 201);
  if (!Number.isInteger(created?.id) || typeof created.upload_url !== "string") {
    throw new Error("GitHub draft release has an invalid schema");
  }
  const uploadBase = created.upload_url.replace(/\{.*$/, "");
  for (const name of assetNames) {
    const uploadUrl = new URL(uploadBase);
    uploadUrl.searchParams.set("name", name);
    await json(await fetchImpl(uploadUrl.href, {
      method: "POST",
      headers: headers(token, "application/octet-stream"),
      body: assets[name],
    }), `upload asset ${name}`, 201);
  }

  const draft = await json(await fetchImpl(`${base}/releases/${created.id}`, {
    headers: headers(token),
  }), "read uploaded draft");
  const downloadedAssets = {};
  for (const name of ["nightly.yml", "nightly-mac.yml", "SHA256SUMS.txt"]) {
    const asset = draft.assets?.find((candidate) => candidate.name === name);
    if (typeof asset?.url !== "string") throw new Error(`draft asset is missing: ${name}`);
    const response = await fetchImpl(asset.url, {
      headers: { ...headers(token), accept: "application/octet-stream" },
    });
    if (!response.ok) throw new Error(`download draft asset ${name} failed with ${response.status}`);
    downloadedAssets[name] = await response.text();
  }
  downloadedAssets[`OrkWorks-${identity.version}-win-x64.exe`] = assets[`OrkWorks-${identity.version}-win-x64.exe`];
  downloadedAssets[`OrkWorks-${identity.version}-mac-arm64.zip`] = assets[`OrkWorks-${identity.version}-mac-arm64.zip`];
  validateReleaseIntegrity({
    release: draft,
    sourceSha,
    expectedAssetNames: expectedNames,
    tagTargetSha,
    downloadedAssets,
    requiredDraft: true,
  });

  const published = await json(await fetchImpl(`${base}/releases/${created.id}`, {
    method: "PATCH",
    headers: headers(token),
    body: JSON.stringify({ draft: false }),
  }), "publish draft release");
  validateReleaseIntegrity({
    release: published,
    sourceSha,
    expectedAssetNames: expectedNames,
    tagTargetSha,
    downloadedAssets,
    requiredDraft: false,
  });
  return published;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const version = process.env.ORKWORKS_NIGHTLY_VERSION;
  await publishDailyRelease({
    repository: process.env.GITHUB_REPOSITORY,
    token: process.env.RELEASE_GITHUB_TOKEN,
    identity: { version, tag: process.env.ORKWORKS_NIGHTLY_TAG },
    sourceSha: process.env.ORKWORKS_SOURCE_SHA,
    assets: readReleaseAssets({ directory: process.env.ORKWORKS_ASSET_DIR, version }),
  });
}
