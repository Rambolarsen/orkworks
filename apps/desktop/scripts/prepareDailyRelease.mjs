import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import {
  assertCandidateIsNewest,
  createNightlyIdentity,
  ensureTagAtSource,
  expectedReleaseAssetNames,
  getTagTarget,
  listAllReleases,
  parseNightlyTag,
  parseSourceMarker,
  selectPublishedNightlyForSource,
  sourceMarker,
  validatePublishedRelease,
} from "./dailyRelease.mjs";

function replacePackageVersion(source, packageName, version, header) {
  const blocks = source.split(/(?=^\[\[package\]\]\s*$)/m);
  const matches = [];
  for (let index = 0; index < blocks.length; index += 1) {
    if (new RegExp(`^name = "${packageName}"$`, "m").test(blocks[index])) matches.push(index);
  }
  if (matches.length !== 1) {
    throw new Error(`${header} must contain exactly one ${packageName} package entry`);
  }
  const index = matches[0];
  const versions = blocks[index].match(/^version = "[^"]+"$/gm) ?? [];
  if (versions.length !== 1) throw new Error(`${header} package version is ambiguous`);
  blocks[index] = blocks[index].replace(/^version = "[^"]+"$/m, `version = "${version}"`);
  return blocks.join("");
}

export function stageNightlyVersions({ packageJson, cargoToml, cargoLock, version }) {
  expectedReleaseAssetNames({ version, channel: "nightly" });
  let packageData;
  try {
    packageData = JSON.parse(packageJson);
  } catch (error) {
    throw new Error("desktop package.json is invalid", { cause: error });
  }
  if (!packageData || typeof packageData !== "object" || Array.isArray(packageData)) {
    throw new Error("desktop package.json is invalid");
  }
  packageData.version = version;

  const manifestBlocks = cargoToml.split(/(?=^\[[^\]]+\]\s*$)/m);
  const packageIndexes = manifestBlocks
    .map((block, index) => /^\[package\]\s*$/m.test(block) && /^name = "orkworksd"$/m.test(block) ? index : -1)
    .filter((index) => index >= 0);
  if (packageIndexes.length !== 1) {
    throw new Error("Cargo.toml must contain the orkworksd package");
  }
  const packageIndex = packageIndexes[0];
  const versions = manifestBlocks[packageIndex].match(/^version = "[^"]+"$/gm) ?? [];
  if (versions.length !== 1) throw new Error("Cargo.toml package version is ambiguous");
  manifestBlocks[packageIndex] = manifestBlocks[packageIndex]
    .replace(/^version = "[^"]+"$/m, `version = "${version}"`);

  return {
    packageJson: `${JSON.stringify(packageData, null, 2)}\n`,
    cargoToml: manifestBlocks.join(""),
    cargoLock: replacePackageVersion(cargoLock, "orkworksd", version, "Cargo.lock"),
  };
}

export function stageNightlyVersionFiles({ repoRoot, version }) {
  const packagePath = resolve(repoRoot, "apps", "desktop", "package.json");
  const cargoPath = resolve(repoRoot, "crates", "orkworksd", "Cargo.toml");
  const lockPath = resolve(repoRoot, "crates", "orkworksd", "Cargo.lock");
  const staged = stageNightlyVersions({
    packageJson: readFileSync(packagePath, "utf8"),
    cargoToml: readFileSync(cargoPath, "utf8"),
    cargoLock: readFileSync(lockPath, "utf8"),
    version,
  });
  writeFileSync(packagePath, staged.packageJson);
  writeFileSync(cargoPath, staged.cargoToml);
  writeFileSync(lockPath, staged.cargoLock);
}

export async function loadNightlyReleaseState({ repository, token, sourceSha, fetchImpl = fetch }) {
  sourceMarker(sourceSha);
  const releases = await listAllReleases({ repository, token, fetchImpl });
  const validated = [];
  const publishedNightlyVersions = [];
  const markerCounts = new Map();
  for (const release of releases) {
    if (!release || typeof release !== "object") throw new Error("GitHub release list contains an invalid entry");
    if (release.draft) continue;
    if (typeof release.tag_name !== "string" || !release.tag_name.includes("-nightly.")) continue;
    let version;
    try {
      version = parseNightlyTag(release.tag_name);
    } catch {
      continue;
    }
    publishedNightlyVersions.push(version);
    if (!release.prerelease) continue;
    const expectedAssetNames = expectedReleaseAssetNames({ version, channel: "nightly" });
    let marker;
    try {
      marker = parseSourceMarker(release.body);
    } catch {
      continue;
    }
    if (marker !== sourceSha) continue;
    const tagTargetSha = await getTagTarget({ repository, token, tag: release.tag_name, fetchImpl });
    const downloadedAssets = {};
    let downloadsComplete = true;
    const payloadNames = [
      `OrkWorks-${version}-win-x64.exe`,
      `OrkWorks-${version}-mac-arm64.zip`,
    ];
    for (const name of ["nightly.yml", "nightly-mac.yml", "SHA256SUMS.txt", ...payloadNames]) {
      const asset = Array.isArray(release.assets)
        ? release.assets.find((candidate) => candidate?.name === name)
        : null;
      if (typeof asset?.url !== "string") {
        downloadsComplete = false;
        break;
      }
      const response = await fetchImpl(asset.url, {
        headers: { accept: "application/octet-stream", authorization: `Bearer ${token}` },
      });
      if (!response.ok) throw new Error(`download published asset ${name} failed with ${response.status}`);
      downloadedAssets[name] = payloadNames.includes(name)
        ? Buffer.from(await response.arrayBuffer())
        : await response.text();
    }
    if (!downloadsComplete) continue;
    try {
      const validRelease = validatePublishedRelease({
        release,
        sourceSha: marker,
        expectedAssetNames,
        tagTargetSha,
        downloadedAssets,
      });
      validated.push(validRelease);
      markerCounts.set(marker, (markerCounts.get(marker) ?? 0) + 1);
    } catch {
      // A damaged published release is diagnostic history, not successful delivery.
    }
  }
  for (const [sourceSha, count] of markerCounts) {
    if (count > 1) throw new Error(`multiple published nightlies claim source SHA ${sourceSha}`);
  }
  return { publishedNightlyVersions, validated };
}

export async function prepareDailyRelease({
  baseVersion,
  utcDate,
  runId,
  runNumber,
  runAttempt,
  repository,
  token,
  sourceSha,
  fetchImpl = fetch,
  loadState = loadNightlyReleaseState,
  ensureTag = ensureTagAtSource,
}) {
  const identity = createNightlyIdentity({ baseVersion, utcDate, runId, runNumber, runAttempt });
  const state = await loadState({ repository, token, sourceSha, fetchImpl });
  const existing = selectPublishedNightlyForSource(state.validated, sourceSha);
  if (existing) return { shouldBuild: false, sourceSha };
  assertCandidateIsNewest(identity.version, state.publishedNightlyVersions.map((version) => ({ version })));
  await ensureTag({ repository, token, tag: identity.tag, sourceSha, fetchImpl });
  return { shouldBuild: true, sourceSha, identity };
}

function appendOutputs(path, values) {
  const lines = Object.entries(values).map(([name, value]) => `${name}=${value}`).join("\n");
  writeFileSync(path, `${lines}\n`, { flag: "a" });
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  if (process.argv[2] === "--stage") {
    stageNightlyVersionFiles({
      repoRoot: resolve(import.meta.dirname, "..", "..", ".."),
      version: process.env.ORKWORKS_NIGHTLY_VERSION,
    });
  } else {
    const packageJson = JSON.parse(readFileSync(resolve(import.meta.dirname, "..", "package.json"), "utf8"));
    const result = await prepareDailyRelease({
      baseVersion: packageJson.version,
      utcDate: new Date(process.env.ORKWORKS_RUN_STARTED_AT),
      runId: process.env.GITHUB_RUN_ID,
      runNumber: process.env.GITHUB_RUN_NUMBER,
      runAttempt: process.env.GITHUB_RUN_ATTEMPT,
      repository: process.env.GITHUB_REPOSITORY,
      token: process.env.RELEASE_GITHUB_TOKEN,
      sourceSha: process.env.ORKWORKS_SOURCE_SHA,
    });
    appendOutputs(process.env.GITHUB_OUTPUT, {
      should_build: String(result.shouldBuild),
      source_sha: result.sourceSha,
      version: result.identity?.version ?? "",
      tag: result.identity?.tag ?? "",
      windows_build_version: result.identity?.windowsBuildVersion ?? "",
      mac_bundle_version: result.identity?.macBundleVersion ?? "",
    });
  }
}
