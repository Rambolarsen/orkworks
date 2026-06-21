import * as path from "path";

export function getDevRepoRoot(electronDir: string): string {
  return path.resolve(electronDir, "..", "..", "..");
}

export function getDevSidecarPath(electronDir: string): string {
  return path.join(getDevRepoRoot(electronDir), "crates", "orkworksd", "target", "debug", "orkworksd");
}

export function getPackagedSidecarPath(resourcesPath: string, platform: NodeJS.Platform): string {
  const binaryName = platform === "win32" ? "orkworksd.exe" : "orkworksd";
  return path.join(resourcesPath, binaryName);
}
