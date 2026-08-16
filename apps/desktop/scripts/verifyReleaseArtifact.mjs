import { readFileSync, statSync as defaultStatSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const HOOK_SCRIPT_NAMES = [
  "report-harness-event.sh",
  "report-harness-event.ps1",
  "report-opencode-session.sh",
];

export function createReleaseArtifactExpectation(platform, arch, version, releaseDir) {
  if (platform === "darwin" && (arch === "arm64" || arch === "x64")) {
    const appDir = join(releaseDir, `mac-${arch}`, "OrkWorks.app");
    const resourcesDir = join(appDir, "Contents", "Resources");
    return {
      installerPath: join(releaseDir, `OrkWorks-${version}-mac-${arch}.dmg`),
      appDir,
      sidecarPath: join(resourcesDir, "orkworksd"),
      scriptsDir: join(resourcesDir, "scripts"),
      scriptPaths: HOOK_SCRIPT_NAMES.map((name) => join(resourcesDir, "scripts", name)),
    };
  }

  if (platform === "win32" && arch === "x64") {
    const appDir = join(releaseDir, "win-unpacked");
    const resourcesDir = join(appDir, "resources");
    return {
      installerPath: join(releaseDir, `OrkWorks-${version}-win-${arch}.exe`),
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

export function verifyReleaseArtifact(expectation, fsModule = { statSync: defaultStatSync }) {
  assertPath(fsModule, expectation.installerPath, "installer", "file");
  assertPath(fsModule, expectation.appDir, "unpacked app", "directory");
  assertPath(fsModule, expectation.sidecarPath, "Rust sidecar", "file");
  assertPath(fsModule, expectation.scriptsDir, "hook scripts", "directory");
  for (const scriptPath of expectation.scriptPaths) {
    assertPath(fsModule, scriptPath, "hook script", "file");
  }
}

export function runCli({
  platform = process.platform,
  arch = process.arch,
  version,
  releaseDir = resolve(import.meta.dirname, "..", "release"),
  fsModule = { statSync: defaultStatSync },
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
  );
  verifyReleaseArtifact(expectation, fsModule);
  output(`Verified release artifact: ${expectation.installerPath}; sidecar: ${expectation.sidecarPath}`);
  return expectation;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  runCli();
}
