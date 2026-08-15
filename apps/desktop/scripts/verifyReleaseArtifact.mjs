import { statSync as defaultStatSync } from "node:fs";
import { join } from "node:path";

export function createReleaseArtifactExpectation(platform, arch, version, releaseDir) {
  if (platform === "darwin" && (arch === "arm64" || arch === "x64")) {
    const appDir = join(releaseDir, `mac-${arch}`, "OrkWorks.app");
    const resourcesDir = join(appDir, "Contents", "Resources");
    return {
      installerPath: join(releaseDir, `OrkWorks-${version}-mac-${arch}.dmg`),
      appDir,
      sidecarPath: join(resourcesDir, "orkworksd"),
      scriptsDir: join(resourcesDir, "scripts"),
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
}
