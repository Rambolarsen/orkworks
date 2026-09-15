import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { expectedReleaseAssetNames, sourceMarker } from "../scripts/dailyRelease.mjs";
import {
  loadNightlyReleaseState,
  prepareDailyRelease,
  stageNightlyVersions,
} from "../scripts/prepareDailyRelease.mjs";
import { publishDailyRelease, readReleaseAssets } from "../scripts/publishDailyRelease.mjs";

const SOURCE_SHA = "0123456789abcdef0123456789abcdef01234567";
const VERSION = "0.2.0-nightly.20260915.123456789.2";
const TAG = `v${VERSION}`;

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

test("stages the same nightly version in desktop and Rust manifests", () => {
  const staged = stageNightlyVersions({
    packageJson: JSON.stringify({ name: "orkworks-desktop", version: "0.2.0", private: true }, null, 2) + "\n",
    cargoToml: '[package]\nname = "orkworksd"\nversion = "0.2.0"\nedition = "2021"\n',
    cargoLock: 'version = 4\n\n[[package]]\nname = "orkworksd"\nversion = "0.2.0"\ndependencies = []\n',
    version: VERSION,
  });

  assert.equal(JSON.parse(staged.packageJson).version, VERSION);
  assert.match(staged.cargoToml, new RegExp(`version = "${VERSION.replaceAll(".", "\\.")}"`));
  assert.match(staged.cargoLock, new RegExp(`name = "orkworksd"\nversion = "${VERSION.replaceAll(".", "\\.")}"`));
});

test("version staging rejects ambiguous Rust package entries", () => {
  assert.throws(() => stageNightlyVersions({
    packageJson: '{"version":"0.2.0"}\n',
    cargoToml: '[package]\nname = "orkworksd"\nversion = "0.2.0"\n',
    cargoLock: '[[package]]\nname = "orkworksd"\nversion = "0.2.0"\n[[package]]\nname = "orkworksd"\nversion = "0.2.0"\n',
    version: VERSION,
  }), /exactly one orkworksd/i);
});

function createAssets() {
  const names = expectedReleaseAssetNames({ version: VERSION, channel: "nightly" });
  const assets = Object.fromEntries(names.map((name) => [name, Buffer.from(`${name} contents`)]));
  const win = `OrkWorks-${VERSION}-win-x64.exe`;
  const mac = `OrkWorks-${VERSION}-mac-arm64.zip`;
  assets["nightly.yml"] = Buffer.from([
    `version: ${VERSION}`,
    "files:",
    `  - url: ${win}`,
    "    sha512: d2luZG93cw==",
    `    size: ${assets[win].length}`,
  ].join("\n"));
  assets["nightly-mac.yml"] = Buffer.from([
    `version: ${VERSION}`,
    "files:",
    `  - url: ${mac}`,
    "    sha512: bWFj",
    `    size: ${assets[mac].length}`,
  ].join("\n"));
  assets["SHA256SUMS.txt"] = Buffer.from(names
    .filter((name) => name !== "SHA256SUMS.txt")
    .sort()
    .map((name) => `${sha256(assets[name])}  ${name}`)
    .join("\n") + "\n");
  return assets;
}

function publishedRelease({
  id,
  version = VERSION,
  sourceSha = SOURCE_SHA,
  assets = createAssets(),
  tag = `v${version}`,
}) {
  return {
    id,
    tag_name: tag,
    body: sourceMarker(sourceSha),
    draft: false,
    prerelease: true,
    assets: Object.entries(assets).map(([name, value], index) => ({
      id: id * 100 + index,
      name,
      size: value.length,
      digest: `sha256:${sha256(value)}`,
      browser_download_url: `https://github.com/Rambolarsen/orkworks/releases/download/${tag}/${encodeURIComponent(name)}`,
    })),
  };
}

test("loads validated nightly state across pagination and tag kinds", async () => {
  const assets = createAssets();
  const damagedVersion = "0.2.0-nightly.20260914.9.1";
  const damagedSourceSha = "f".repeat(40);
  const damaged = publishedRelease({ id: 4, version: damagedVersion, sourceSha: damagedSourceSha });
  damaged.assets = damaged.assets.filter((asset) => asset.name !== "nightly.yml");
  const valid = publishedRelease({ id: 5, assets });
  const calls = [];
  const fetchImpl = async (url) => {
    calls.push(url);
    if (url.endsWith("/releases?per_page=100")) {
      return Response.json([damaged], {
        headers: { link: '<https://api.github.com/repos/Rambolarsen/orkworks/releases?per_page=100&page=2>; rel="next"' },
      });
    }
    if (url.endsWith("/releases?per_page=100&page=2")) return Response.json([valid]);
    if (url.endsWith(`/git/ref/tags/${encodeURIComponent(damaged.tag_name)}`)) {
      return Response.json({ ref: `refs/tags/${damaged.tag_name}`, object: { type: "commit", sha: damagedSourceSha } });
    }
    if (url.endsWith(`/git/ref/tags/${encodeURIComponent(valid.tag_name)}`)) {
      return Response.json({ ref: `refs/tags/${valid.tag_name}`, object: { type: "tag", sha: "a".repeat(40) } });
    }
    if (url.endsWith(`/git/tags/${"a".repeat(40)}`)) {
      return Response.json({ object: { type: "commit", sha: SOURCE_SHA } });
    }
    const asset = valid.assets.find((candidate) => candidate.browser_download_url === url);
    if (asset) return new Response(assets[asset.name]);
    throw new Error(`unexpected request: ${url}`);
  };

  const state = await loadNightlyReleaseState({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl,
  });

  assert.deepEqual(state.publishedNightlyVersions, [damagedVersion, VERSION]);
  assert.deepEqual(state.validated.map(({ sourceSha, version }) => ({ sourceSha, version })), [
    { sourceSha: SOURCE_SHA, version: VERSION },
  ]);
  assert.ok(calls.some((url) => url.includes("page=2")));
  assert.ok(calls.some((url) => url.includes("/git/tags/")));
});

test("remote nightly state rejects duplicate source markers", async () => {
  const first = publishedRelease({ id: 6 });
  const second = publishedRelease({ id: 7, version: "0.2.0-nightly.20260916.123456790.1" });
  first.assets = [];
  second.assets = [];
  await assert.rejects(() => loadNightlyReleaseState({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl: async (url) => {
      if (url.endsWith("/releases?per_page=100")) return Response.json([first, second]);
      if (url.includes("/git/ref/tags/")) {
        const tag = decodeURIComponent(url.slice(url.lastIndexOf("/") + 1));
        return Response.json({ ref: `refs/tags/${tag}`, object: { type: "commit", sha: SOURCE_SHA } });
      }
      throw new Error(`unexpected request: ${url}`);
    },
  }), /multiple published nightlies claim source SHA/i);
});

test("publishes only after the uploaded draft passes the full integrity predicate", async () => {
  const localAssets = createAssets();
  const uploaded = new Map();
  const calls = [];
  const releaseJson = (draft) => ({
    id: 77,
    tag_name: TAG,
    body: `Daily build\n${sourceMarker(SOURCE_SHA)}\n`,
    draft,
    prerelease: true,
    upload_url: "https://uploads.github.com/repos/Rambolarsen/orkworks/releases/77/assets{?name,label}",
    assets: [...uploaded].map(([name, value], index) => ({
      id: index + 1,
      name,
      size: value.length,
      digest: `sha256:${sha256(value)}`,
      browser_download_url: `https://downloads.example/${encodeURIComponent(name)}`,
    })),
  });
  const fetchImpl = async (url, options = {}) => {
    const method = options.method ?? "GET";
    calls.push(`${method} ${url}`);
    if (url.includes("/git/ref/tags/")) {
      return Response.json({ ref: `refs/tags/${TAG}`, object: { type: "commit", sha: SOURCE_SHA } });
    }
    if (method === "POST" && url.endsWith("/releases")) {
      const body = JSON.parse(options.body);
      assert.equal(body.target_commitish, undefined);
      assert.equal(body.draft, true);
      assert.equal(body.prerelease, true);
      return Response.json(releaseJson(true), { status: 201 });
    }
    if (method === "POST" && url.startsWith("https://uploads.github.com/")) {
      const name = new URL(url).searchParams.get("name");
      uploaded.set(name, Buffer.from(options.body));
      return Response.json({ name }, { status: 201 });
    }
    if (method === "GET" && url.endsWith("/releases/77")) return Response.json(releaseJson(true));
    if (method === "GET" && url.startsWith("https://downloads.example/")) {
      return new Response(uploaded.get(decodeURIComponent(new URL(url).pathname.slice(1))));
    }
    if (method === "PATCH" && url.endsWith("/releases/77")) {
      assert.equal(uploaded.size, Object.keys(localAssets).length);
      return Response.json(releaseJson(false));
    }
    throw new Error(`unexpected request: ${method} ${url}`);
  };

  const published = await publishDailyRelease({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    identity: { version: VERSION, tag: TAG },
    sourceSha: SOURCE_SHA,
    assets: localAssets,
    fetchImpl,
    loadState: async () => ({ publishedNightlyVersions: [], validated: [] }),
  });

  assert.equal(published.draft, false);
  assert.equal(calls.at(-1), "PATCH https://api.github.com/repos/Rambolarsen/orkworks/releases/77");
});

test("does not publish an incomplete uploaded draft", async () => {
  const assets = createAssets();
  let published = false;
  await assert.rejects(() => publishDailyRelease({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    identity: { version: VERSION, tag: TAG },
    sourceSha: SOURCE_SHA,
    assets,
    loadState: async () => ({ publishedNightlyVersions: [], validated: [] }),
    fetchImpl: async (url, options = {}) => {
      const method = options.method ?? "GET";
      if (url.includes("/git/ref/tags/")) return Response.json({ ref: `refs/tags/${TAG}`, object: { type: "commit", sha: SOURCE_SHA } });
      if (method === "POST" && url.endsWith("/releases")) return Response.json({
        id: 77, tag_name: TAG, body: sourceMarker(SOURCE_SHA), draft: true, prerelease: true,
        upload_url: "https://uploads.example/assets{?name,label}", assets: [],
      }, { status: 201 });
      if (method === "POST" && url.startsWith("https://uploads.example/")) return Response.json({}, { status: 201 });
      if (method === "GET" && url.endsWith("/releases/77")) return Response.json({
        id: 77, tag_name: TAG, body: sourceMarker(SOURCE_SHA), draft: true, prerelease: true, assets: [],
      });
      if (method === "PATCH") published = true;
      throw new Error(`unexpected request: ${method} ${url}`);
    },
  }), /asset/i);
  assert.equal(published, false);
});

test("publication rechecks remote state and skips a newly completed source", async () => {
  let requests = 0;
  const existing = { sourceSha: SOURCE_SHA, version: VERSION, release: { id: 12, draft: false } };
  const result = await publishDailyRelease({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    identity: { version: VERSION, tag: TAG },
    sourceSha: SOURCE_SHA,
    assets: createAssets(),
    loadState: async () => ({ publishedNightlyVersions: [VERSION], validated: [existing] }),
    fetchImpl: async () => { requests += 1; throw new Error("must not mutate GitHub"); },
  });

  assert.equal(result, existing.release);
  assert.equal(requests, 0);
});

test("preparation skips one already-validated nightly for the frozen source", async () => {
  let tagWrites = 0;
  const result = await prepareDailyRelease({
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: "123456789",
    runNumber: "42",
    runAttempt: "2",
    repository: "Rambolarsen/orkworks",
    token: "secret",
    sourceSha: SOURCE_SHA,
    loadState: async () => ({
      publishedNightlyVersions: [VERSION],
      validated: [{ sourceSha: SOURCE_SHA, version: VERSION }],
    }),
    ensureTag: async () => { tagWrites += 1; },
  });

  assert.deepEqual(result, { shouldBuild: false, sourceSha: SOURCE_SHA });
  assert.equal(tagWrites, 0);
});

test("preparation creates only the candidate tag for an eligible source", async () => {
  const tags = [];
  const result = await prepareDailyRelease({
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: "123456789",
    runNumber: "42",
    runAttempt: "2",
    repository: "Rambolarsen/orkworks",
    token: "secret",
    sourceSha: SOURCE_SHA,
    loadState: async () => ({
      publishedNightlyVersions: ["0.2.0-nightly.20260914.9.1"],
      validated: [],
    }),
    ensureTag: async (options) => { tags.push(options); },
  });

  assert.equal(result.shouldBuild, true);
  assert.equal(result.identity.tag, TAG);
  assert.deepEqual(tags.map(({ tag, sourceSha }) => ({ tag, sourceSha })), [{ tag: TAG, sourceSha: SOURCE_SHA }]);
});

test("preparation rejects ambiguous duplicates and out-of-order candidates", async () => {
  const base = {
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: "123456789",
    runNumber: "42",
    runAttempt: "2",
    repository: "Rambolarsen/orkworks",
    token: "secret",
    sourceSha: SOURCE_SHA,
    ensureTag: async () => {},
  };
  await assert.rejects(() => prepareDailyRelease({
    ...base,
    loadState: async () => ({
      publishedNightlyVersions: [VERSION, VERSION],
      validated: [
        { sourceSha: SOURCE_SHA, version: VERSION },
        { sourceSha: SOURCE_SHA, version: VERSION },
      ],
    }),
  }), /multiple published nightlies/i);
  await assert.rejects(() => prepareDailyRelease({
    ...base,
    loadState: async () => ({
      publishedNightlyVersions: ["0.2.0-nightly.20260916.1.1"],
      validated: [],
    }),
  }), /newer than every published nightly/i);
});

test("release asset loading accepts exact files and rejects symlinks", (t) => {
  const directory = mkdtempSync(join(tmpdir(), "orkworks-publish-assets-"));
  const outside = join(tmpdir(), `orkworks-publish-outside-${process.pid}.txt`);
  try {
    const assets = createAssets();
    for (const [name, value] of Object.entries(assets)) writeFileSync(join(directory, name), value);
    assert.deepEqual(Object.keys(readReleaseAssets({ directory, version: VERSION })).sort(), Object.keys(assets).sort());
    rmSync(join(directory, "nightly.yml"));
    writeFileSync(outside, "outside");
    try {
      symlinkSync(outside, join(directory, "nightly.yml"), "file");
    } catch (error) {
      if (error?.code === "EPERM" || error?.code === "EACCES") return t.skip("symlinks unavailable");
      throw error;
    }
    assert.throws(() => readReleaseAssets({ directory, version: VERSION }), /regular file/i);
  } finally {
    rmSync(directory, { recursive: true, force: true });
    rmSync(outside, { force: true });
  }
});
