import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import {
  createReleaseArtifactExpectation,
  runCli,
  verifyReleaseArtifact,
} from "../scripts/verifyReleaseArtifact.mjs";
import {
  createWindowsInstallerExpectation,
  readAuthenticodeSignature,
  verifyInstalledWindowsApp,
} from "../scripts/windowsInstallerSmokeTest.mjs";
import {
  verifyAppUpdateMetadata,
  writeChecksumManifest,
} from "../scripts/releaseMetadata.mjs";

const passingMetadataModule = {
  verifyAppUpdateMetadata() {},
  verifyUpdateMetadata() {},
};

function withTempDir(run) {
  const directory = mkdtempSync(join(tmpdir(), "orkworks-release-verifier-"));
  try {
    return run(directory);
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
}

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
    channel: "latest",
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

test("nightly expectations use only nightly updater metadata", () => {
  const mac = createReleaseArtifactExpectation(
    "darwin",
    "arm64",
    "0.2.0-nightly.20260915.123.1",
    "/release",
    "nightly",
  );
  const win = createReleaseArtifactExpectation(
    "win32",
    "x64",
    "0.2.0-nightly.20260915.123.1",
    "/release",
    "nightly",
  );

  assert.equal(mac.metadataPath, join("/release", "nightly-mac.yml"));
  assert.equal(win.metadataPath, join("/release", "nightly.yml"));
  assert.throws(
    () => createReleaseArtifactExpectation("win32", "x64", "0.2.0", "/release", "beta"),
    /release channel/i,
  );
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
      verifyAppUpdateMetadata() {},
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
    channel: "latest",
  }]]);
});

test("packaged updater metadata must match the release channel and GitHub repository", () => {
  const expectations = [
    createReleaseArtifactExpectation(
      "win32", "x64", "0.2.0-nightly.20260915.123.1", "/release", "nightly",
    ),
    createReleaseArtifactExpectation(
      "darwin", "arm64", "0.2.0-nightly.20260915.123.1", "/release", "nightly",
    ),
  ];
  const fakeFs = {
    statSync() {
      return { isFile: () => true, isDirectory: () => true, size: 1 };
    },
  };
  const calls = [];

  for (const expectation of expectations) {
    verifyReleaseArtifact(expectation, fakeFs, {
      verifyAppUpdateMetadata(options) { calls.push(options); },
      verifyUpdateMetadata() {},
    });
  }

  assert.deepEqual(calls, expectations.map((expectation) => ({
    metadataPath: expectation.appUpdateMetadataPath,
    channel: "nightly",
    provider: "github",
    owner: "Rambolarsen",
    repo: "orkworks",
  })));
});

test("app update metadata normalizes the omitted stable channel and requires nightly explicitly", () => withTempDir((directory) => {
  const metadataPath = join(directory, "app-update.yml");
  const expected = {
    metadataPath,
    provider: "github",
    owner: "Rambolarsen",
    repo: "orkworks",
  };

  writeFileSync(metadataPath, "provider: github\nowner: Rambolarsen\nrepo: orkworks\n");
  assert.doesNotThrow(() => verifyAppUpdateMetadata({ ...expected, channel: "latest" }));
  assert.throws(
    () => verifyAppUpdateMetadata({ ...expected, channel: "nightly" }),
    /channel mismatch.*nightly.*latest/i,
  );

  writeFileSync(metadataPath, "provider: github\nowner: Rambolarsen\nrepo: orkworks\nchannel: null\n");
  assert.throws(
    () => verifyAppUpdateMetadata({ ...expected, channel: "latest" }),
    /channel mismatch.*latest.*null/i,
  );

  writeFileSync(metadataPath, "provider: github\nowner: Rambolarsen\nrepo: orkworks\nchannel: nightly\n");
  assert.doesNotThrow(() => verifyAppUpdateMetadata({ ...expected, channel: "nightly" }));

  writeFileSync(metadataPath, "provider: github\nowner: Rambolarsen\nrepo: elsewhere\nchannel: nightly\n");
  assert.throws(
    () => verifyAppUpdateMetadata({ ...expected, channel: "nightly" }),
    /repo mismatch.*orkworks.*elsewhere/i,
  );
}));

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

test("pre-checksum mode passes before checksum generation and the final gate does not", () => withTempDir((releaseDir) => {
  const version = "0.1.0";
  const expectation = createReleaseArtifactExpectation("win32", "x64", version, releaseDir);
  const payload = "signed installer";

  for (const directory of [expectation.appDir, expectation.scriptsDir]) {
    mkdirSync(directory, { recursive: true });
  }
  for (const path of [
    ...expectation.distributablePaths,
    ...expectation.blockmapPaths,
    expectation.appUpdateMetadataPath,
    expectation.appPath,
    expectation.sidecarPath,
    ...expectation.scriptPaths,
    join(expectation.scriptsDir, "..", "knowledge", "starter.json"),
    join(expectation.scriptsDir, "..", "knowledge", "public-key.pem"),
  ]) {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, path === expectation.installerPath
      ? payload
      : path === expectation.appUpdateMetadataPath
        ? "provider: github\nowner: Rambolarsen\nrepo: orkworks\n"
        : "fixture");
  }
  writeFileSync(expectation.metadataPath, [
    `version: ${version}`,
    "files:",
    `  - url: ${basename(expectation.installerPath)}`,
    `    sha512: ${createHash("sha512").update(payload).digest("base64")}`,
    `    size: ${Buffer.byteLength(payload)}`,
  ].join("\n"));

  assert.doesNotThrow(() => runCli({
    platform: "win32",
    arch: "x64",
    version,
    releaseDir,
    preChecksum: true,
    output() {},
  }));
  assert.throws(
    () => runCli({ platform: "win32", arch: "x64", version, releaseDir, output() {} }),
    /checksum manifest/i,
  );

  writeChecksumManifest({ releaseDir, outputPath: expectation.checksumPath, expectedVersion: version });

  assert.doesNotThrow(() => runCli({
    platform: "win32",
    arch: "x64",
    version,
    releaseDir,
    output() {},
  }));
}));

function createInstalledWindowsFixture() {
  const expectation = {
    ...createWindowsInstallerExpectation({
      version: "0.1.0",
      releaseDir: "C:\\release",
      installDir: "C:\\temp\\orkworks-smoke",
      productName: "OrkWorks",
    }),
    expectedPublisher: "CN=OrkWorks Release, O=OrkWorks",
  };
  const fsModule = {
    statSync(path) {
      if (path === expectation.scriptsDir) {
        return { isFile: () => false, isDirectory: () => true, size: 0 };
      }
      return { isFile: () => true, isDirectory: () => false, size: 1 };
    },
  };
  return { expectation, fsModule };
}

test("installed Windows executables require valid Authenticode signatures", () => {
  const { expectation, fsModule } = createInstalledWindowsFixture();
  const verifiedPaths = [];

  verifyInstalledWindowsApp(expectation, fsModule, (path) => {
    verifiedPaths.push(path);
    return { status: "Valid", publisher: expectation.expectedPublisher };
  });

  assert.deepEqual(verifiedPaths, [expectation.appPath, expectation.sidecarPath]);
});

test("installed Windows verification rejects invalid Authenticode status", () => {
  const { expectation, fsModule } = createInstalledWindowsFixture();

  assert.throws(
    () => verifyInstalledWindowsApp(expectation, fsModule, () => ({
      status: "NotSigned",
      publisher: "",
    })),
    /Authenticode status.*NotSigned/i,
  );
});

test("installed Windows verification rejects a publisher mismatch", () => {
  const { expectation, fsModule } = createInstalledWindowsFixture();

  assert.throws(
    () => verifyInstalledWindowsApp(expectation, fsModule, (path) => ({
      status: "Valid",
      publisher: path === expectation.appPath
        ? expectation.expectedPublisher
        : `CN=Unexpected Publisher, OU=${expectation.expectedPublisher}`,
    })),
    /publisher mismatch.*orkworksd\.exe/i,
  );
});

test("Authenticode reader passes the real executable path through a dedicated environment variable", () => {
  const executablePath = "C:\\Program Files\\OrkWorks\\OrkWorks.exe";
  const calls = [];

  const signature = readAuthenticodeSignature(executablePath, (file, args, options) => {
    calls.push({ file, args, options });
    return JSON.stringify({ status: "Valid", publisher: "OrkWorks AS" });
  });

  assert.deepEqual(signature, { status: "Valid", publisher: "OrkWorks AS" });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].file, "powershell.exe");
  assert.deepEqual(calls[0].args.slice(0, 3), ["-NoProfile", "-NonInteractive", "-Command"]);
  assert.equal(calls[0].args.length, 4);
  assert.match(calls[0].args[3], /\$env:ORKWORKS_SIGNATURE_PATH/);
  assert.equal(calls[0].options.env.ORKWORKS_SIGNATURE_PATH, executablePath);
});

test("Authenticode reader preserves the executable path in command failure diagnostics", () => {
  const executablePath = "C:\\Program Files\\OrkWorks\\resources\\orkworksd.exe";

  assert.throws(
    () => readAuthenticodeSignature(executablePath, () => {
      throw new Error("PowerShell failed");
    }),
    (error) => error instanceof Error
      && error.message.includes(executablePath)
      && error.cause?.message === "PowerShell failed",
  );
});
