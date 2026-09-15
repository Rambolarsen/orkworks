import { readFileSync, statSync as defaultStatSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import * as defaultMetadataModule from "./releaseMetadata.mjs";

const HOOK_SCRIPT_NAMES = [
  "report-harness-event.sh",
  "report-harness-event.ps1",
  "report-opencode-session.sh",
];

export function createReleaseArtifactExpectation(platform, arch, version, releaseDir, channel = "latest") {
  if (channel !== "latest" && channel !== "nightly") {
    throw new Error("release channel must be latest or nightly");
  }
  if (platform === "darwin" && (arch === "arm64" || arch === "x64")) {
    const appDir = join(releaseDir, `mac-${arch}`, "OrkWorks.app");
    const resourcesDir = join(appDir, "Contents", "Resources");
    const installerPath = join(releaseDir, `OrkWorks-${version}-mac-${arch}.dmg`);
    return {
      installerPath,
      distributablePaths: [
        installerPath,
        join(releaseDir, `OrkWorks-${version}-mac-${arch}.zip`),
      ],
      metadataPath: join(releaseDir, `${channel}-mac.yml`),
      blockmapPaths: [join(releaseDir, `OrkWorks-${version}-mac-${arch}.zip.blockmap`)],
      appUpdateMetadataPath: join(resourcesDir, "app-update.yml"),
      appPath: join(appDir, "Contents", "MacOS", "OrkWorks"),
      releaseDir,
      version,
      channel,
      checksumPath: join(releaseDir, "SHA256SUMS.txt"),
      appDir,
      sidecarPath: join(resourcesDir, "orkworksd"),
      scriptsDir: join(resourcesDir, "scripts"),
      scriptPaths: HOOK_SCRIPT_NAMES.map((name) => join(resourcesDir, "scripts", name)),
    };
  }

  if (platform === "win32" && arch === "x64") {
    const appDir = join(releaseDir, "win-unpacked");
    const resourcesDir = join(appDir, "resources");
    const installerPath = join(releaseDir, `OrkWorks-${version}-win-${arch}.exe`);
    return {
      installerPath,
      distributablePaths: [installerPath],
      metadataPath: join(releaseDir, `${channel}.yml`),
      blockmapPaths: [join(releaseDir, `OrkWorks-${version}-win-${arch}.exe.blockmap`)],
      appUpdateMetadataPath: join(resourcesDir, "app-update.yml"),
      appPath: join(appDir, "OrkWorks.exe"),
      releaseDir,
      version,
      channel,
      checksumPath: join(releaseDir, "SHA256SUMS.txt"),
      appDir,
      sidecarPath: join(resourcesDir, "orkworksd.exe"),
      scriptsDir: join(resourcesDir, "scripts"),
      scriptPaths: HOOK_SCRIPT_NAMES.map((name) => join(resourcesDir, "scripts", name)),
    };
  }

  throw new Error(`Unsupported release target: ${platform}/${arch}`);
}

function assertPath(fsModule, path, label, kind) {
  try {
    const stats = fsModule.statSync(path);
    const valid = kind === "file"
      ? stats.isFile() && stats.size > 0
      : stats.isDirectory();
    if (!valid) throw new Error("wrong file type or empty file");
  } catch (error) {
    throw new Error(`Packaged release artifact is incomplete: ${label} missing at ${path}`, {
      cause: error,
    });
  }
}

export function verifyReleaseArtifact(
  expectation,
  fsModule = { statSync: defaultStatSync },
  metadataModule = defaultMetadataModule,
  { preChecksum = false } = {},
) {
  for (const distributablePath of expectation.distributablePaths) {
    assertPath(fsModule, distributablePath, "distributable", "file");
  }
  assertPath(fsModule, expectation.metadataPath, "update metadata", "file");
  for (const blockmapPath of expectation.blockmapPaths) {
    assertPath(fsModule, blockmapPath, "blockmap", "file");
  }
  assertPath(fsModule, expectation.appUpdateMetadataPath, "app update metadata", "file");
  assertPath(fsModule, expectation.appPath, "packaged app executable", "file");
  if (!preChecksum) {
    assertPath(fsModule, expectation.checksumPath, "checksum manifest", "file");
  }
  assertPath(fsModule, expectation.appDir, "unpacked app", "directory");
  assertPath(fsModule, expectation.sidecarPath, "Rust sidecar", "file");
  assertPath(fsModule, expectation.scriptsDir, "hook scripts", "directory");
  for (const scriptPath of expectation.scriptPaths) {
    assertPath(fsModule, scriptPath, "hook script", "file");
  }
  assertPath(fsModule, join(expectation.scriptsDir, "..", "knowledge", "starter.json"), "starter knowledge", "file");
  assertPath(fsModule, join(expectation.scriptsDir, "..", "knowledge", "public-key.pem"), "knowledge verification key", "file");
  try {
    metadataModule.verifyAppUpdateMetadata({
      metadataPath: expectation.appUpdateMetadataPath,
      channel: expectation.channel,
      provider: "github",
      owner: "Rambolarsen",
      repo: "orkworks",
    });
  } catch (error) {
    throw new Error(`Packaged app update metadata is invalid at ${expectation.appUpdateMetadataPath}`, {
      cause: error,
    });
  }
  try {
    metadataModule.verifyUpdateMetadata({
      metadataPath: expectation.metadataPath,
      releaseDir: expectation.releaseDir,
      expectedVersion: expectation.version,
      channel: expectation.channel,
    });
  } catch (error) {
    throw new Error(`Packaged release metadata is invalid at ${expectation.metadataPath}`, {
      cause: error,
    });
  }
}

export function runCli({
  platform = process.platform,
  arch = process.arch,
  version,
  releaseDir = resolve(import.meta.dirname, "..", "release"),
  fsModule = { statSync: defaultStatSync },
  metadataModule = defaultMetadataModule,
  preChecksum = false,
  channel = process.env.ORKWORKS_RELEASE_CHANNEL ?? "latest",
  output = (message) => console.log(message),
} = {}) {
  const packageJson = JSON.parse(
    readFileSync(join(import.meta.dirname, "..", "package.json"), "utf8"),
  );
  const expectation = createReleaseArtifactExpectation(
    platform,
    arch,
    version ?? packageJson.version,
    releaseDir,
    channel,
  );
  verifyReleaseArtifact(expectation, fsModule, metadataModule, { preChecksum });
  output(`Verified release artifact: ${expectation.installerPath}; sidecar: ${expectation.sidecarPath}`);
  return expectation;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length === 1 && args[0] !== "--pre-checksum")) {
    throw new Error(`Unsupported release verification arguments: ${args.join(" ")}`);
  }
  runCli({ preChecksum: args[0] === "--pre-checksum" });
}
