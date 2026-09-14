import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import {
  verifyUpdateMetadata,
  writeChecksumManifest,
} from "../scripts/releaseMetadata.mjs";

test("writes sorted SHA-256 entries without including the manifest itself", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  writeFileSync(join(releaseDir, "OrkWorks-0.2.0-win-x64.exe"), "installer");
  writeFileSync(join(releaseDir, "latest.yml"), "metadata");
  const outputPath = join(releaseDir, "SHA256SUMS.txt");

  assert.equal(writeChecksumManifest({ releaseDir, outputPath }), outputPath);
  const names = readFileSync(outputPath, "utf8").trim().split("\n")
    .map((line) => line.split("  ")[1]);
  assert.deepEqual(names, ["OrkWorks-0.2.0-win-x64.exe", "latest.yml"]);
});

test("verifies metadata version, payload digest, size, and blockmap", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  const payload = "zip payload";
  const payloadName = "OrkWorks-0.2.0-mac-arm64.zip";
  writeFileSync(join(releaseDir, payloadName), payload);
  writeFileSync(join(releaseDir, payloadName + ".blockmap"), "blockmap");
  const metadataPath = join(releaseDir, "latest-mac.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - url: " + payloadName,
    "    sha512: " + createHash("sha512").update(payload).digest("base64"),
    "    size: 11",
  ].join("\n"));

  assert.equal(
    verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }).files[0].url,
    payloadName,
  );
});

test("rejects missing payloads and mismatched SHA-512 values", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
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
    /missing payload|SHA-512|digest/i,
  );
});
