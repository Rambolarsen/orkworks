import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";

import {
  createReleaseArtifactExpectation,
  runCli,
  verifyReleaseArtifact,
} from "../scripts/verifyReleaseArtifact.mjs";

const passingMetadataModule = { verifyUpdateMetadata() {} };

test("macOS expectation points at the DMG and packaged resources", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");

  assert.deepEqual(expectation, {
    installerPath: join("/release", "OrkWorks-0.1.0-mac-arm64.dmg"),
    distributablePaths: [
      join("/release", "OrkWorks-0.1.0-mac-arm64.dmg"),
      join("/release", "OrkWorks-0.1.0-mac-arm64.zip"),
    ],
    metadataPath: join("/release", "latest-mac.yml"),
    blockmapPaths: [join("/release", "OrkWorks-0.1.0-mac-arm64.zip.blockmap")],
    appUpdateMetadataPath: join(
      "/release", "mac-arm64", "OrkWorks.app", "Contents", "Resources", "app-update.yml",
    ),
    appPath: join("/release", "mac-arm64", "OrkWorks.app", "Contents", "MacOS", "OrkWorks"),
    releaseDir: "/release",
    version: "0.1.0",
    checksumPath: join("/release", "SHA256SUMS.txt"),
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
    scriptPaths: [
      join("/release", "mac-arm64", "OrkWorks.app", "Contents", "Resources", "scripts", "report-harness-event.sh"),
      join("/release", "mac-arm64", "OrkWorks.app", "Contents", "Resources", "scripts", "report-harness-event.ps1"),
      join("/release", "mac-arm64", "OrkWorks.app", "Contents", "Resources", "scripts", "report-opencode-session.sh"),
    ],
  });
});

test("Windows expectation points at the NSIS installer and exe sidecar", () => {
  const expectation = createReleaseArtifactExpectation("win32", "x64", "0.1.0", "/release");

  assert.equal(expectation.installerPath, join("/release", "OrkWorks-0.1.0-win-x64.exe"));
  assert.deepEqual(expectation.distributablePaths, [
    join("/release", "OrkWorks-0.1.0-win-x64.exe"),
  ]);
  assert.equal(expectation.metadataPath, join("/release", "latest.yml"));
  assert.deepEqual(expectation.blockmapPaths, [
    join("/release", "OrkWorks-0.1.0-win-x64.exe.blockmap"),
  ]);
  assert.equal(
    expectation.appUpdateMetadataPath,
    join("/release", "win-unpacked", "resources", "app-update.yml"),
  );
  assert.equal(expectation.appPath, join("/release", "win-unpacked", "OrkWorks.exe"));
  assert.equal(expectation.releaseDir, "/release");
  assert.equal(expectation.version, "0.1.0");
  assert.equal(expectation.checksumPath, join("/release", "SHA256SUMS.txt"));
  assert.equal(expectation.appDir, join("/release", "win-unpacked"));
  assert.equal(expectation.sidecarPath, join("/release", "win-unpacked", "resources", "orkworksd.exe"));
  assert.equal(expectation.scriptsDir, join("/release", "win-unpacked", "resources", "scripts"));
  assert.deepEqual(expectation.scriptPaths.map((path) => basename(path)), [
    "report-harness-event.sh",
    "report-harness-event.ps1",
    "report-opencode-session.sh",
  ]);
});

test("unsupported release targets are rejected", () => {
  assert.throws(
    () => createReleaseArtifactExpectation("linux", "x64", "0.1.0", "/release"),
    /Unsupported release target/,
  );
});

test("missing starter knowledge fails packaged release verification", () => {
  const expectation = createReleaseArtifactExpectation("win32", "x64", "0.1.0", "/release");
  const missing = join("/release", "win-unpacked", "resources", "knowledge", "starter.json");
  const fakeFs = { statSync(path) {
    if (path === missing) throw new Error("ENOENT");
    return { isFile: () => true, isDirectory: () => true, size: 1 };
  } };
  assert.throws(() => verifyReleaseArtifact(expectation, fakeFs), /starter knowledge/);
});

test("missing packaged resources identify the failing path", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const fakeFs = {
    statSync(path) {
      if (expectation.distributablePaths.includes(path)
        || path === expectation.metadataPath
        || expectation.blockmapPaths.includes(path)
        || path === expectation.appUpdateMetadataPath
        || path === expectation.appPath
        || path === expectation.checksumPath) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      throw new Error("ENOENT");
    },
  };

  assert.throws(
    () => verifyReleaseArtifact(expectation, fakeFs),
    (error) => error instanceof Error && error.message.includes(expectation.appDir),
  );
});

test("installer is verified once when it is also the first distributable", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const installerStats = [];
  const fakeFs = {
    statSync(path) {
      if (path === expectation.installerPath) installerStats.push(path);
      return { isFile: () => true, isDirectory: () => true, size: 1 };
    },
  };

  verifyReleaseArtifact(expectation, fakeFs, passingMetadataModule);

  assert.deepEqual(installerStats, [expectation.installerPath]);
});

test("metadata validation failures identify the metadata path", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const fakeFs = {
    statSync() {
      return { isFile: () => true, isDirectory: () => true, size: 1 };
    },
  };
  const metadataCalls = [];

  assert.throws(
    () => verifyReleaseArtifact(expectation, fakeFs, {
      verifyUpdateMetadata(...args) {
        metadataCalls.push(args);
        throw new Error("invalid metadata");
      },
    }),
    (error) => error instanceof Error && error.message.includes(expectation.metadataPath),
  );
  assert.deepEqual(metadataCalls, [[{
    metadataPath: expectation.metadataPath,
    releaseDir: expectation.releaseDir,
    expectedVersion: expectation.version,
  }]]);
});

test("missing hook scripts identify the exact packaged file", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const fakeFs = {
    statSync(path) {
      if (path === expectation.installerPath || path === expectation.sidecarPath) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      if (path === expectation.appDir || path === expectation.scriptsDir) {
        return { isFile: () => false, isDirectory: () => true, size: 0 };
      }
      if (path === expectation.scriptPaths[1]) throw new Error("ENOENT");
      return { isFile: () => true, isDirectory: () => false, size: 1 };
    },
  };

  assert.throws(
    () => verifyReleaseArtifact(expectation, fakeFs),
    (error) => error instanceof Error && error.message.includes(expectation.scriptPaths[1]),
  );
});

test("runCli verifies a release and reports the installer", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const output = [];
  const fakeFs = {
    statSync(path) {
      if (expectation.distributablePaths.includes(path)
        || path === expectation.metadataPath
        || expectation.blockmapPaths.includes(path)
        || path === expectation.appUpdateMetadataPath
        || path === expectation.appPath
        || path === expectation.checksumPath) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      if (path === expectation.sidecarPath) return { isFile: () => true, isDirectory: () => false, size: 1 };
      if (expectation.scriptPaths.includes(path)) return { isFile: () => true, isDirectory: () => false, size: 1 };
      if (path.includes(`${join("Resources", "knowledge")}`)) return { isFile: () => true, isDirectory: () => false, size: 1 };
      return { isFile: () => false, isDirectory: () => true, size: 0 };
    },
  };

  const result = runCli({
    platform: "darwin",
    arch: "arm64",
    version: "0.1.0",
    releaseDir: "/release",
    fsModule: fakeFs,
    metadataModule: passingMetadataModule,
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

test("packaged app includes the runtime icon assets used by Electron main", () => {
  const electronBuilderConfig = readFileSync(
    new URL("../electron-builder.yml", import.meta.url),
    "utf8",
  );

  for (const asset of [
    "build/icon.png",
    "build/icon-dark.png",
    "build/icon.ico",
    "build/icon-dark.ico",
  ]) {
    assert.match(electronBuilderConfig, new RegExp(`^\\s*- ${asset.replaceAll(".", "\\.")}$`, "m"));
  }
});
