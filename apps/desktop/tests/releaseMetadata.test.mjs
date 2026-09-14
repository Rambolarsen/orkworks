import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import {
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

function writeMetadata(releaseDir, { payloadName, payload, size = Buffer.byteLength(payload), version = "0.2.0", digest = sha512(payload), blockmap = true }) {
  writeFileSync(join(releaseDir, payloadName), payload);
  if (blockmap) {
    writeFileSync(join(releaseDir, payloadName + ".blockmap"), "blockmap");
  }
  const metadataPath = join(releaseDir, "latest.yml");
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

test("verifies metadata version, payload digest, size, and blockmap", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-mac-arm64.zip";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "zip payload" });

  assert.equal(
    verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }).files[0].url,
    payloadName,
  );
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
  const payloadName = "OrkWorks-0.2.0-linux-x64.AppImage";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer", size: 99 });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /size mismatch/i,
  );
}));

test("rejects a missing blockmap for a real payload", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-linux-x64.AppImage";
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

test("rejects metadata with the wrong version", () => withTempDir((releaseDir) => {
  const payloadName = "OrkWorks-0.2.0-win-x64.exe";
  const metadataPath = writeMetadata(releaseDir, { payloadName, payload: "installer", version: "0.2.1" });

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /version mismatch/i,
  );
}));
