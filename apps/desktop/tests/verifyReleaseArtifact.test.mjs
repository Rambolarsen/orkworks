import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  createReleaseArtifactExpectation,
  runCli,
  verifyReleaseArtifact,
} from "../scripts/verifyReleaseArtifact.mjs";

test("macOS expectation points at the DMG and packaged resources", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");

  assert.deepEqual(expectation, {
    installerPath: join("/release", "OrkWorks-0.1.0-mac-arm64.dmg"),
    appDir: join("/release", "mac-arm64", "OrkWorks.app"),
    sidecarPath: join(
      "/release",
      "mac-arm64",
      "OrkWorks.app",
      "Contents",
      "Resources",
      "orkworksd",
    ),
    scriptsDir: join(
      "/release",
      "mac-arm64",
      "OrkWorks.app",
      "Contents",
      "Resources",
      "scripts",
    ),
  });
});

test("Windows expectation points at the NSIS installer and exe sidecar", () => {
  const expectation = createReleaseArtifactExpectation("win32", "x64", "0.1.0", "/release");

  assert.equal(expectation.installerPath, join("/release", "OrkWorks-0.1.0-win-x64.exe"));
  assert.equal(expectation.appDir, join("/release", "win-unpacked"));
  assert.equal(expectation.sidecarPath, join("/release", "win-unpacked", "resources", "orkworksd.exe"));
  assert.equal(expectation.scriptsDir, join("/release", "win-unpacked", "resources", "scripts"));
});

test("unsupported release targets are rejected", () => {
  assert.throws(
    () => createReleaseArtifactExpectation("linux", "x64", "0.1.0", "/release"),
    /Unsupported release target/,
  );
});

test("missing packaged resources identify the failing path", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const fakeFs = {
    statSync(path) {
      if (path === expectation.installerPath) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      throw new Error("ENOENT");
    },
  };

  assert.throws(
    () => verifyReleaseArtifact(expectation, fakeFs),
    new RegExp(expectation.appDir),
  );
});

test("runCli verifies a release and reports the installer", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const output = [];
  const fakeFs = {
    statSync(path) {
      if (path === expectation.installerPath) return { isFile: () => true, isDirectory: () => false, size: 1 };
      if (path === expectation.sidecarPath) return { isFile: () => true, isDirectory: () => false, size: 1 };
      return { isFile: () => false, isDirectory: () => true, size: 0 };
    },
  };

  const result = runCli({
    platform: "darwin",
    arch: "arm64",
    version: "0.1.0",
    releaseDir: "/release",
    fsModule: fakeFs,
    output: (message) => output.push(message),
  });

  assert.deepEqual(result, expectation);
  assert.equal(output.length, 1);
  assert.match(output[0], /OrkWorks-0\.1\.0-mac-arm64\.dmg/);
});

test("desktop package exposes the release verification command", () => {
  const packageJson = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.equal(packageJson.scripts["verify:release"], "node scripts/verifyReleaseArtifact.mjs");
});
