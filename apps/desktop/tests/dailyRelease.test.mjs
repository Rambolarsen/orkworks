import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  assertCandidateIsNewest,
  createNightlyIdentity,
  expectedReleaseAssetNames,
  ensureTagAtSource,
  listAllReleases,
  listNightlyTagVersions,
  parseNightlyTag,
  parseSourceMarker,
  parseStableTag,
  selectPublishedNightlyForSource,
  sourceMarker,
  validatePublishedRelease,
} from "../scripts/dailyRelease.mjs";

const SOURCE_SHA = "0123456789abcdef0123456789abcdef01234567";
const VERSION = "0.2.0-nightly.20260915.123456789.2";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function sha512(value) {
  return createHash("sha512").update(value).digest("base64");
}

function createValidRelease({
  sourceSha = SOURCE_SHA,
  version = VERSION,
  tagTargetSha = sourceSha,
} = {}) {
  const tag = `v${version}`;
  const names = expectedReleaseAssetNames({ version, channel: "nightly" });
  const contents = Object.fromEntries(names.map((name) => [name, `${name} content`]));
  const windowsPayload = `OrkWorks-${version}-win-x64.exe`;
  const macPayload = `OrkWorks-${version}-mac-arm64.zip`;
  contents["nightly.yml"] = [
    `version: ${version}`,
    "files:",
    `  - url: ${windowsPayload}`,
    `    sha512: ${sha512(contents[windowsPayload])}`,
    `    size: ${Buffer.byteLength(contents[windowsPayload])}`,
  ].join("\n");
  contents["nightly-mac.yml"] = [
    `version: ${version}`,
    "files:",
    `  - url: ${macPayload}`,
    `    sha512: ${sha512(contents[macPayload])}`,
    `    size: ${Buffer.byteLength(contents[macPayload])}`,
  ].join("\n");
  contents["SHA256SUMS.txt"] = names
    .filter((name) => name !== "SHA256SUMS.txt")
    .sort()
    .map((name) => `${sha256(contents[name])}  ${name}`)
    .join("\n") + "\n";

  return {
    downloadedAssets: {
      "nightly.yml": contents["nightly.yml"],
      "nightly-mac.yml": contents["nightly-mac.yml"],
      "SHA256SUMS.txt": contents["SHA256SUMS.txt"],
      [windowsPayload]: Buffer.from(contents[windowsPayload]),
      [macPayload]: Buffer.from(contents[macPayload]),
    },
    expectedAssetNames: names,
    release: {
      body: `Nightly build\n\n${sourceMarker(sourceSha)}\n`,
      draft: false,
      prerelease: true,
      tag_name: tag,
      assets: names.map((name, index) => ({
        id: index + 1,
        name,
        size: Buffer.byteLength(contents[name]),
        digest: `sha256:${sha256(contents[name])}`,
      })),
    },
    sourceSha,
    tagTargetSha,
  };
}

function resealDownloadedAsset(fixture, name) {
  const value = fixture.downloadedAssets[name];
  const asset = fixture.release.assets.find((candidate) => candidate.name === name);
  asset.size = Buffer.byteLength(value);
  asset.digest = `sha256:${sha256(value)}`;
}

function resealMetadata(fixture, name) {
  resealDownloadedAsset(fixture, name);
  const checksum = fixture.release.assets.find((asset) => asset.name === name).digest.slice(7);
  fixture.downloadedAssets["SHA256SUMS.txt"] = fixture.downloadedAssets["SHA256SUMS.txt"]
    .replace(new RegExp(`^[0-9a-f]{64}  ${name.replace(".", "\\.")}$`, "m"), `${checksum}  ${name}`);
  resealDownloadedAsset(fixture, "SHA256SUMS.txt");
}

test("accepts only a canonical stable tag matching the package version", () => {
  assert.deepEqual(parseStableTag("v0.2.0", "0.2.0"), { version: "0.2.0" });

  for (const tag of ["v01.2.3", "v0.2.0-rc.1", "v0.2.0+build", "0.2.0"]) {
    assert.throws(() => parseStableTag(tag, "0.2.0"), /canonical stable tag/i);
  }
  assert.throws(() => parseStableTag("v0.2.0", "0.2.1"), /package version/i);
});

test("creates deterministic SemVer and native nightly identities", () => {
  assert.deepEqual(createNightlyIdentity({
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: "123456789",
    runNumber: "42",
    runAttempt: "2",
  }), {
    version: VERSION,
    tag: `v${VERSION}`,
    windowsBuildVersion: "2026.258.42.2",
    macBundleVersion: "1.41.2",
  });
});

test("nightly identities reject noncanonical and out-of-range inputs", () => {
  const valid = {
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: "123456789",
    runNumber: "42",
    runAttempt: "2",
  };

  for (const override of [
    { baseVersion: "0.2.0-nightly.1" },
    { runId: "001" },
    { runNumber: "0" },
    { runNumber: "65536" },
    { runAttempt: "0" },
    { runAttempt: "100" },
    { utcDate: new Date("invalid") },
    { utcDate: (() => {
      const date = new Date(0);
      date.setUTCFullYear(999, 0, 1);
      return date;
    })() },
    { utcDate: new Date(Date.UTC(10_000, 0, 1)) },
  ]) {
    assert.throws(() => createNightlyIdentity({ ...valid, ...override }), /invalid|range|canonical/i);
  }
});

test("remote nightly tags require real dates and locally supported attempts", () => {
  assert.equal(parseNightlyTag(`v${VERSION}`), VERSION);
  assert.throws(
    () => parseNightlyTag("v0.2.0-nightly.20260230.123456789.1"),
    /nightly release tag is invalid/i,
  );
  assert.throws(
    () => parseNightlyTag("v0.2.0-nightly.20260915.123456789.100"),
    /run attempt.*supported range/i,
  );
});

test("native nightly versions are collision-free and increasing", () => {
  const identity = (runNumber, runAttempt) => createNightlyIdentity({
    baseVersion: "0.2.0",
    utcDate: new Date("2026-09-15T03:23:00Z"),
    runId: String(runNumber),
    runNumber: String(runNumber),
    runAttempt: String(runAttempt),
  });
  const first = identity(42, 1);
  const retry = identity(42, 2);
  const next = identity(43, 1);

  assert.notEqual(first.windowsBuildVersion, retry.windowsBuildVersion);
  assert.notEqual(first.macBundleVersion, retry.macBundleVersion);
  assert.deepEqual(
    [first.macBundleVersion, retry.macBundleVersion, next.macBundleVersion],
    ["1.41.1", "1.41.2", "1.42.1"],
  );

});

test("source markers require one exact lowercase SHA line", () => {
  const marker = sourceMarker(SOURCE_SHA);
  assert.equal(marker, `<!-- orkworks-nightly-source:${SOURCE_SHA} -->`);
  assert.equal(parseSourceMarker(`notes\n${marker}\nmore`), SOURCE_SHA);
  assert.equal(parseSourceMarker(`prefix ${marker}`), null);
  assert.equal(parseSourceMarker(marker.toUpperCase()), null);
  assert.throws(() => parseSourceMarker(`${marker}\n${marker}`), /multiple source markers/i);
  assert.throws(() => sourceMarker("abc"), /source SHA/i);
});

test("stable and nightly release assets use disjoint metadata names", () => {
  const latest = expectedReleaseAssetNames({ version: "0.2.0", channel: "latest" });
  const nightly = expectedReleaseAssetNames({ version: VERSION, channel: "nightly" });

  assert(latest.includes("latest.yml"));
  assert(latest.includes("latest-mac.yml"));
  assert(!latest.includes("nightly.yml"));
  assert(nightly.includes("nightly.yml"));
  assert(nightly.includes("nightly-mac.yml"));
  assert(!nightly.includes("latest.yml"));
  assert.throws(
    () => expectedReleaseAssetNames({ version: VERSION, channel: "beta" }),
    /release channel/i,
  );
});

test("validates a complete published nightly release", () => {
  const fixture = createValidRelease();
  const validated = validatePublishedRelease(fixture);

  assert.equal(validated.sourceSha, SOURCE_SHA);
  assert.equal(validated.version, VERSION);
  assert.equal(validated.tag, `v${VERSION}`);
});

test("rejects draft, stable, or wrongly targeted nightly releases", () => {
  for (const mutate of [
    (fixture) => { fixture.release.draft = true; },
    (fixture) => { fixture.release.prerelease = false; },
    (fixture) => { fixture.tagTargetSha = "f".repeat(40); },
  ]) {
    const fixture = createValidRelease();
    mutate(fixture);
    assert.throws(() => validatePublishedRelease(fixture), /published prerelease|tag target/i);
  }
});

test("rejects missing, extra, duplicate, empty, or undigested assets", () => {
  for (const mutate of [
    (fixture) => { fixture.release.assets.pop(); },
    (fixture) => { fixture.release.assets.push({ name: "latest.yml", size: 1, digest: `sha256:${"a".repeat(64)}` }); },
    (fixture) => { fixture.release.assets[1].name = fixture.release.assets[0].name; },
    (fixture) => { fixture.release.assets[0].size = 0; },
    (fixture) => { fixture.release.assets[0].digest = null; },
  ]) {
    const fixture = createValidRelease();
    mutate(fixture);
    assert.throws(() => validatePublishedRelease(fixture), /asset/i);
  }
});

test("rejects malformed or mismatched checksum manifests", () => {
  for (const manifest of [
    "not a checksum\n",
    `${"a".repeat(64)}  missing.exe\n`,
    "",
  ]) {
    const fixture = createValidRelease();
    fixture.downloadedAssets["SHA256SUMS.txt"] = manifest;
    resealDownloadedAsset(fixture, "SHA256SUMS.txt");
    assert.throws(() => validatePublishedRelease(fixture), /asset|checksum/i);
  }
});

test("rejects updater metadata with the wrong version, payload, or size", () => {
  for (const mutate of [
    (fixture) => { fixture.downloadedAssets["nightly.yml"] = fixture.downloadedAssets["nightly.yml"].replace(VERSION, "0.2.1"); },
    (fixture) => { fixture.downloadedAssets["nightly.yml"] = fixture.downloadedAssets["nightly.yml"].replace("-win-x64.exe", "-mac-arm64.dmg"); },
    (fixture) => { fixture.downloadedAssets["nightly.yml"] = fixture.downloadedAssets["nightly.yml"].replace(/size: \d+/, "size: 999"); },
  ]) {
    const fixture = createValidRelease();
    mutate(fixture);
    resealMetadata(fixture, "nightly.yml");
    assert.throws(() => validatePublishedRelease(fixture), /metadata/i);
  }
});

test("selects one valid published nightly for a source and rejects ambiguity", () => {
  const first = validatePublishedRelease(createValidRelease());
  assert.equal(selectPublishedNightlyForSource([first], SOURCE_SHA), first);
  assert.equal(selectPublishedNightlyForSource([first], "f".repeat(40)), null);
  assert.throws(
    () => selectPublishedNightlyForSource([first, { ...first }], SOURCE_SHA),
    /multiple published nightlies/i,
  );
});

test("requires a candidate to exceed every published nightly", () => {
  const releases = [
    { version: "0.2.0-nightly.20260914.20.1" },
    { version: "0.2.0-nightly.20260915.21.1" },
  ];

  assert.doesNotThrow(() => assertCandidateIsNewest("0.2.0-nightly.20260915.21.2", releases));
  assert.throws(
    () => assertCandidateIsNewest("0.2.0-nightly.20260915.21.1", releases),
    /newer than every published nightly/i,
  );
  assert.throws(
    () => assertCandidateIsNewest("0.2.0-nightly.20260913.99.1", releases),
    /newer than every published nightly/i,
  );
});

function jsonResponse(status, body, headers = {}) {
  return new Response(body === null ? null : JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", ...headers },
  });
}

test("release listing follows every GitHub pagination link", async () => {
  const requests = [];
  const fetchImpl = async (url, options) => {
    requests.push({ url, options });
    if (requests.length === 1) {
      return jsonResponse(200, [{ id: 1 }], {
        link: '<https://api.github.com/repositories/1270107877/releases?per_page=100&page=2>; rel="next"',
      });
    }
    return jsonResponse(200, [{ id: 2 }]);
  };

  assert.deepEqual(await listAllReleases({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl,
  }), [{ id: 1 }, { id: 2 }]);
  assert.equal(requests.length, 2);
  assert.equal(requests[0].options.headers.authorization, "Bearer secret");
});

test("release listing fails closed on API and schema errors", async () => {
  await assert.rejects(() => listAllReleases({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl: async () => jsonResponse(403, { message: "forbidden" }),
  }), /GitHub API.*403/i);
  await assert.rejects(() => listAllReleases({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl: async () => jsonResponse(200, { id: 1 }),
  }), /release list.*array/i);
});

test("release listing never forwards credentials to an untrusted pagination link", async () => {
  let leaked = false;
  await assert.rejects(() => listAllReleases({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl: async (url, options) => {
      if (url === "https://attacker.example/steal?page=2") {
        leaked = options.headers.authorization === "Bearer secret";
        return jsonResponse(200, []);
      }
      return jsonResponse(200, [], {
        link: '<https://attacker.example/steal?page=2>; rel="next"',
      });
    },
  }), /pagination URL/i);
  assert.equal(leaked, false);
});

test("matching tag listing returns only valid nightly-channel SemVer and fails closed on schema errors", async () => {
  const fetchImpl = async (url, options) => {
    assert.equal(url, "https://api.github.com/repos/Rambolarsen/orkworks/git/matching-refs/tags/v");
    assert.equal(options.headers.authorization, "Bearer secret");
    return jsonResponse(200, [
      { ref: `refs/tags/v${VERSION}`, object: { type: "commit", sha: SOURCE_SHA } },
      { ref: "refs/tags/v0.2.0", object: { type: "commit", sha: SOURCE_SHA } },
      { ref: "refs/tags/v0.2.0-nightly.not-a-daily-identity", object: { type: "tag", sha: "a".repeat(40) } },
    ]);
  };

  assert.deepEqual(await listNightlyTagVersions({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl,
  }), [VERSION, "0.2.0-nightly.not-a-daily-identity"]);

  await assert.rejects(() => listNightlyTagVersions({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    fetchImpl: async () => jsonResponse(200, [{ ref: null }]),
  }), /matching tag list.*invalid entry/i);
});

test("tag creation adopts only the exact source and retries an absent ambiguous result", async () => {
  const tag = `v${VERSION}`;
  const calls = [];
  let reads = 0;
  let writes = 0;
  const fetchImpl = async (url, options) => {
    calls.push({ url, method: options.method ?? "GET" });
    if ((options.method ?? "GET") === "POST") {
      writes += 1;
      return writes === 1 ? jsonResponse(502, { message: "upstream" }) : jsonResponse(201, {
        ref: `refs/tags/${tag}`,
        object: { type: "commit", sha: SOURCE_SHA },
      });
    }
    reads += 1;
    return reads < 3
      ? jsonResponse(404, { message: "missing" })
      : jsonResponse(200, { ref: `refs/tags/${tag}`, object: { type: "commit", sha: SOURCE_SHA } });
  };

  assert.deepEqual(await ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl,
  }), { tag, sourceSha: SOURCE_SHA });
  assert.equal(calls.filter((call) => call.method === "POST").length, 2);
});

test("tag creation adopts an exact tag after a malformed success response", async () => {
  const tag = `v${VERSION}`;
  let reads = 0;
  let writes = 0;
  const fetchImpl = async (_url, options = {}) => {
    if ((options.method ?? "GET") === "POST") {
      writes += 1;
      return new Response("not json", { status: 201 });
    }
    reads += 1;
    return reads === 1
      ? jsonResponse(404, { message: "missing" })
      : jsonResponse(200, {
        ref: `refs/tags/${tag}`,
        object: { type: "commit", sha: SOURCE_SHA },
      });
  };

  assert.deepEqual(await ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl,
  }), { tag, sourceSha: SOURCE_SHA });
  assert.equal(writes, 1);
});

test("tag creation adopts an exact tag after an ambiguous server response", async () => {
  const tag = `v${VERSION}`;
  let reads = 0;
  let writes = 0;
  const fetchImpl = async (_url, options = {}) => {
    if ((options.method ?? "GET") === "POST") {
      writes += 1;
      return jsonResponse(502, { message: "upstream failed after creating the ref" });
    }
    reads += 1;
    return reads === 1
      ? jsonResponse(404, { message: "missing" })
      : jsonResponse(200, {
        ref: `refs/tags/${tag}`,
        object: { type: "commit", sha: SOURCE_SHA },
      });
  };

  assert.deepEqual(await ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl,
  }), { tag, sourceSha: SOURCE_SHA });
  assert.equal(writes, 1);
});

test("tag creation fails immediately on a definitive authorization response", async () => {
  const tag = `v${VERSION}`;
  let reads = 0;
  let writes = 0;
  await assert.rejects(() => ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "read-only",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl: async (_url, options = {}) => {
      if ((options.method ?? "GET") === "POST") {
        writes += 1;
        return jsonResponse(403, { message: "forbidden" });
      }
      reads += 1;
      return jsonResponse(404, { message: "missing" });
    },
  }), /failed with 403/i);
  assert.equal(writes, 1);
  assert.equal(reads, 1);
});

test("tag creation reports a definite failed write even if the exact tag later appears", async () => {
  const tag = `v${VERSION}`;
  let reads = 0;
  await assert.rejects(() => ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl: async (_url, options = {}) => {
      if ((options.method ?? "GET") === "POST") return jsonResponse(400, { message: "bad request" });
      reads += 1;
      return reads === 1
        ? jsonResponse(404, { message: "missing" })
        : jsonResponse(200, {
          ref: `refs/tags/${tag}`,
          object: { type: "commit", sha: SOURCE_SHA },
        });
    },
  }), /creation failed with 400/i);
});

test("an existing exact tag must still prove write capability", async () => {
  const tag = `v${VERSION}`;
  let writes = 0;
  const exactTag = { ref: `refs/tags/${tag}`, object: { type: "commit", sha: SOURCE_SHA } };

  assert.deepEqual(await ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl: async (_url, options = {}) => {
      if ((options.method ?? "GET") === "POST") {
        writes += 1;
        return jsonResponse(422, { message: "Reference already exists" });
      }
      return jsonResponse(200, exactTag);
    },
  }), { tag, sourceSha: SOURCE_SHA });
  assert.equal(writes, 1);

  await assert.rejects(() => ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "read-only",
    tag,
    sourceSha: SOURCE_SHA,
    fetchImpl: async (_url, options = {}) => (options.method === "POST"
      ? jsonResponse(403, { message: "Resource not accessible by personal access token" })
      : jsonResponse(200, exactTag)),
  }), /write access.*403/i);
});

test("tag creation never overwrites a mismatched existing tag", async () => {
  let writes = 0;
  await assert.rejects(() => ensureTagAtSource({
    repository: "Rambolarsen/orkworks",
    token: "secret",
    tag: `v${VERSION}`,
    sourceSha: SOURCE_SHA,
    fetchImpl: async (_url, options) => {
      if ((options.method ?? "GET") === "POST") writes += 1;
      return jsonResponse(200, {
        ref: `refs/tags/v${VERSION}`,
        object: { type: "commit", sha: "f".repeat(40) },
      });
    },
  }), /different source SHA/i);
  assert.equal(writes, 0);
});
