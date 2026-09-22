import * as path from "path";
import * as crypto from "crypto";

export function getDevRepoRoot(electronDir: string): string {
  return path.resolve(electronDir, "..", "..", "..");
}

export function getDevSidecarPath(electronDir: string): string {
  return path.join(getDevRepoRoot(electronDir), "crates", "orkworksd", "target", "debug", "orkworksd");
}

export function getDevUserDataPath(electronDir: string, baseUserDataPath: string, devPort: string): string {
  const repoRoot = getDevRepoRoot(electronDir);
  const digest = crypto.createHash("sha256").update(`${repoRoot}:${devPort}`).digest("hex").slice(0, 12);
  return path.join(baseUserDataPath, `dev-${digest}`);
}

export function getPackagedSidecarPath(resourcesPath: string, platform: NodeJS.Platform): string {
  const binaryName = platform === "win32" ? "orkworksd.exe" : "orkworksd";
  return path.join(resourcesPath, binaryName);
}
