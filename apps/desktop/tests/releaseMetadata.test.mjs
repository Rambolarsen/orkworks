import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import {
  runChecksumCli,
  verifyUpdateMetadata,
  writeChecksumManifest,
} from "../scripts/releaseMetadata.mjs";

function withTempDir(run) {
  const directory = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  try {
    return run(directory);
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
}

function sha512(value) {
  return createHash("sha512").update(value).digest("base64");
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function writeMetadata(releaseDir, { payloadName, payload, size = Buffer.byteLength(payload), version = "0.2.0", digest = sha512(payload), blockmap = true, metadataName = "latest.yml" }) {
  writeFileSync(join(releaseDir, payloadName), payload);
  if (blockmap) {
    writeFileSync(join(releaseDir, payloadName + ".blockmap"), "blockmap");
  }
  const metadataPath = join(releaseDir, metadataName);
  writeFileSync(metadataPath, [
    `version: ${version}`,
    "files:",
    `  - url: ${payloadName}`,
    `    sha512: ${digest}`,
    `    size: ${size}`,
  ].join("\n"));
  return metadataPath;
}

function createFileSymlinkOrSkip(t, target, linkPath) {
  try {
    symlinkSync(target, linkPath, "file");
    return true;
  } catch (error) {
    if (error?.code === "EACCES" || error?.code === "EPERM") {
      t.skip("file symlinks are unavailable in this environment");
      return false;
    }
    throw error;
  }
}

test("writes complete sorted SHA-256 entries without including the manifest itself", () => withTempDir((releaseDir) => {
  const installer = "installer";
  const metadata = "metadata";
  const blockmap = "blockmap";
  const installerName = "OrkWorks-0.2.0-win-x64.exe";
  const blockmapName = installerName + ".blockmap";
  writeFileSync(join(releaseDir, installerName), installer);
  writeFileSync(join(releaseDir, "latest.yml"), metadata);
  writeFileSync(join(releaseDir, blockmapName), blockmap);
  const outputPath = join(releaseDir, "SHA256SUMS.txt");

  assert.equal(writeChecksumManifest({ releaseDir, outputPath }), outputPath);
  assert.equal(
    readFileSync(outputPath, "utf8"),
    [
      `${sha256(installer)}  ${installerName}`,
      `${sha256(blockmap)}  ${blockmapName}`,
      `${sha256(metadata)}  latest.yml`,
      "",
    ].join("\n"),
  );
}));

test("checksum CLI reads package version, scopes release artifacts, and prints only its output path", () => withTempDir((appRoot) => {
  const releaseDir = join(appRoot, "release");
  const output = [];
  const version = "1.2.3";
  const currentArtifact = `OrkWorks-${version}-win-x64.exe`;
  const staleArtifact = "OrkWorks-1.2.2-win-x64.exe";
  writeFileSync(join(appRoot, "package.json"), JSON.stringify({ version }));
  mkdirSync(releaseDir);
  writeFileSync(join(releaseDir, currentArtifact), "current");
  writeFileSync(join(releaseDir, `${currentArtifact}.blockmap`), "current blockmap");
  writeFileSync(join(releaseDir, staleArtifact), "stale");
  writeFileSync(join(releaseDir, `${staleArtifact}.blockmap`), "stale blockmap");
  writeFileSync(join(releaseDir, "latest.yml"), "metadata");

  const outputPath = runChecksumCli({ appRoot, output: (value) => output.push(value) });

  assert.deepEqual(output, [outputPath]);
  assert.equal(outputPath, join(releaseDir, "SHA256SUMS.txt"));
  const manifest = readFileSync(outputPath, "utf8");
  assert.match(manifest, new RegExp(currentArtifact.replaceAll(".", "\\.")));
  assert.doesNotMatch(manifest, /1\.2\.2/);
}));

test("nightly checksum generation includes only nightly channel metadata", () => withTempDir((appRoot) => {
  const releaseDir = join(appRoot, "release");
  const version = "0.2.0-nightly.20260915.123.1";
  const artifact = `OrkWorks-${version}-win-x64.exe`;
  writeFileSync(join(appRoot, "package.json"), JSON.stringify({ version }));
  mkdirSync(releaseDir);
  writeFileSync(join(releaseDir, artifact), "installer");
  writeFileSync(join(releaseDir, `${artifact}.blockmap`), "blockmap");
  writeFileSync(join(releaseDir, "nightly.yml"), "metadata");

  const outputPath = runChecksumCli({ appRoot, channel: "nightly", output() {} });
  const manifest = readFileSync(outputPath, "utf8");
  assert.match(manifest, /nightly\.yml/);
  assert.doesNotMatch(manifest, /latest\.yml/);
}));

test("checksum generation rejects mixed or unknown release channels", () => withTempDir((appRoot) => {
  const releaseDir = join(appRoot, "release");
  writeFileSync(join(appRoot, "package.json"), JSON.stringify({ version: "0.2.0" }));
  mkdirSync(releaseDir);
  writeFileSync(join(releaseDir, "latest.yml"), "stable");
  writeFileSync(join(releaseDir, "nightly.yml"), "nightly");

  assert.throws(() => runChecksumCli({ appRoot, channel: "latest" }), /mixed release channel/i);
  assert.throws(() => runChecksumCli({ appRoot, channel: "beta" }), /release channel/i);
}));

test("checksum CLI rejects a package version that could escape its artifact-name scope", () => withTempDir((appRoot) => {
  writeFileSync(join(appRoot, "package.json"), JSON.stringify({ version: "../1.2.3" }));
  mkdirSync(join(appRoot, "release"));

  assert.throws(() => runChecksumCli({ appRoot }), /package version is invalid/i);
}));

test("rejects a checksum input symlink whose real path escapes the release directory", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const fileName = "OrkWorks-0.2.0-win-x64.exe";
    const outsidePath = join(outsideDir, fileName);
    writeFileSync(outsidePath, "external installer");
    if (!createFileSymlinkOrSkip(t, outsidePath, join(releaseDir, fileName))) {
      return;
    }

    assert.throws(
      () => writeChecksumManifest({
        releaseDir,
        outputPath: join(releaseDir, "SHA256SUMS.txt"),
      }),
      /escapes release directory/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("rejects a checksum output symlink whose real path escapes the release directory", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const fileName = "OrkWorks-0.2.0-win-x64.exe";
    writeFileSync(join(releaseDir, fileName), "installer");
    const outsideOutputPath = join(outsideDir, "SHA256SUMS.txt");
    writeFileSync(outsideOutputPath, "must not be overwritten");
    const outputPath = join(releaseDir, "SHA256SUMS.txt");
    if (!createFileSymlinkOrSkip(t, outsideOutputPath, outputPath)) {
      return;
    }

    assert.throws(
      () => writeChecksumManifest({ releaseDir, outputPath }),
      /escapes release directory/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("rejects a broken checksum output symlink", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const fileName = "OrkWorks-0.2.0-win-x64.exe";
    writeFileSync(join(releaseDir, fileName), "installer");
    const outputPath = join(releaseDir, "SHA256SUMS.txt");
    if (!createFileSymlinkOrSkip(t, join(outsideDir, "missing-SHA256SUMS.txt"), outputPath)) {
      return;
    }

    assert.throws(
      () => writeChecksumManifest({ releaseDir, outputPath }),
      /checksum output path/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("verifies metadata version, payload digest, size, and blockmap", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-mac-arm64.zip";
  const metadataPath = writeMetadata(releaseDir, {
    metadataName: "latest-mac.yml",
    payloadName,
    payload: "zip payload",
  });

  assert.equal(
    verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }).files[0].url,
    payloadName,
  );
}));

test("verifies nightly metadata only when the explicit channel agrees", () => withTempDir((releaseDir) => {
  const version = "0.2.0-nightly.20260915.123.1";
  const payloadName = `OrkWorks-${version}-win-x64.exe`;
  const metadataPath = writeMetadata(releaseDir, {
    metadataName: "nightly.yml",
    payloadName,
    payload: "installer",
    version,
  });

  assert.doesNotThrow(() => verifyUpdateMetadata({
    metadataPath,
    releaseDir,
    expectedVersion: version,
    channel: "nightly",
  }));
  assert.throws(() => verifyUpdateMetadata({
    metadataPath,
    releaseDir,
    expectedVersion: version,
    channel: "latest",
  }), /metadata.*channel/i);
}));

test("rejects a missing payload", () => withTempDir((releaseDir) => {
  const metadataPath = join(releaseDir, "latest.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - url: OrkWorks-0.2.0-win-x64.exe",
    "    sha512: invalid",
    "    size: 1",
  ].join("\n"));

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /missing payload/i,
  );
}));

test("rejects an incorrect SHA-512 for a real payload", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, {
    digest: sha512("different payload"),
    payloadName,
    payload: "installer",
  });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /SHA-512 digest mismatch/i,
  );
}));

test("rejects a payload with a mismatched size", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer", size: 99 });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /size mismatch/i,
  );
}));

test("rejects a missing blockmap for a real payload", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, { blockmap: false, payloadName, payload: "installer" });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /missing blockmap/i,
  );
}));

test("rejects payload traversal outside the release directory", () => withTempDir((releaseDir) => {
  const metadataPath = join(releaseDir, "latest.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - url: ../outside/OrkWorks-0.2.0-win-x64.exe",
    `    sha512: ${sha512("installer")}`,
    "    size: 9",
  ].join("\n"));

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /escapes release directory/i,
  );
}));

test("rejects nested updater payload URLs even when the files exist", () => withTempDir((releaseDir) => {
  const payloadName = join("nested", "OrkWorks-0.2.0-win-x64.exe");
  mkdirSync(join(releaseDir, "nested"));
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer" });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /top-level updater payload/i,
  );
}));

test("rejects updater payload URLs for a stale version even when the files exist", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.1.9-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer" });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /supported updater payload.*0\.2\.0/i,
  );
}));

test("rejects non-updater macOS payload URLs even when the files exist", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-mac-arm64.dmg";
  const metadataPath = writeMetadata(releaseDir, {
    metadataName: "latest-mac.yml",
    payloadName,
    payload: "disk image",
  });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /supported updater payload.*0\.2\.0/i,
  );
}));

test("rejects a symlinked payload whose real path escapes the release directory", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const payload = "external payload";
    const payloadName = "OrkWorks-0.2.0-win-x64.exe";
    const outsidePayloadPath = join(outsideDir, payloadName);
    writeFileSync(outsidePayloadPath, payload);
    writeFileSync(outsidePayloadPath + ".blockmap", "blockmap");
    if (!createFileSymlinkOrSkip(t, outsidePayloadPath, join(releaseDir, payloadName))) {
      return;
    }
    const metadataPath = join(releaseDir, "latest.yml");
    writeFileSync(metadataPath, [
      "version: 0.2.0",
      "files:",
      `  - url: ${payloadName}`,
      `    sha512: ${sha512(payload)}`,
      `    size: ${Buffer.byteLength(payload)}`,
    ].join("\n"));
    symlinkSync(outsidePayloadPath + ".blockmap", join(releaseDir, payloadName + ".blockmap"), "file");

    assert.throws(
      () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
      /escapes release directory/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("rejects a symlinked blockmap whose real path escapes the release directory", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const payloadName = "OrkWorks-0.2.0-win-x64.exe";
    const payload = "installer";
    writeFileSync(join(releaseDir, payloadName), payload);
    const outsideBlockmapPath = join(outsideDir, payloadName + ".blockmap");
    writeFileSync(outsideBlockmapPath, "blockmap");
    if (!createFileSymlinkOrSkip(t, outsideBlockmapPath, join(releaseDir, payloadName + ".blockmap"))) {
      return;
    }
    const metadataPath = join(releaseDir, "latest.yml");
    writeFileSync(metadataPath, [
      "version: 0.2.0",
      "files:",
      `  - url: ${payloadName}`,
      `    sha512: ${sha512(payload)}`,
      `    size: ${Buffer.byteLength(payload)}`,
    ].join("\n"));

    assert.throws(
      () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
      /escapes release directory/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("rejects metadata whose real path escapes the release directory", (t) => withTempDir((releaseDir) => {
  const outsideDir = mkdtempSync(join(tmpdir(), "orkworks-outside-"));
  try {
    const payloadName = "OrkWorks-0.2.0-win-x64.exe";
    const payload = "installer";
    writeFileSync(join(releaseDir, payloadName), payload);
    writeFileSync(join(releaseDir, payloadName + ".blockmap"), "blockmap");
    const outsideMetadataPath = join(outsideDir, "latest.yml");
    writeFileSync(outsideMetadataPath, [
      "version: 0.2.0",
      "files:",
      `  - url: ${payloadName}`,
      `    sha512: ${sha512(payload)}`,
      `    size: ${Buffer.byteLength(payload)}`,
    ].join("\n"));
    const metadataPath = join(releaseDir, "latest.yml");
    if (!createFileSymlinkOrSkip(t, outsideMetadataPath, metadataPath)) {
      return;
    }

    assert.throws(
      () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
      /metadata.*escapes release directory/i,
    );
  } finally {
    rmSync(outsideDir, { force: true, recursive: true });
  }
}));

test("rejects malformed YAML metadata", () => withTempDir((releaseDir) => {
  const metadataPath = join(releaseDir, "latest.yml");
  writeFileSync(metadataPath, "version: [0.2.0\nfiles:\n");

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /YAML|unexpected|flow sequence/i,
  );
}));

test("rejects non-object metadata file entries", () => withTempDir((releaseDir) => {
  const metadataPath = join(releaseDir, "latest.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - []",
  ].join("\n"));

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /invalid files entry/i,
  );
}));

test("rejects metadata with the wrong version", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer", version: "0.2.1" });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /version mismatch/i,
  );
}));
