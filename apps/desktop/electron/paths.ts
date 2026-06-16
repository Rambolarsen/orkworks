import * as path from "path";

export function getDevRepoRoot(electronDir: string): string {
  return path.resolve(electronDir, "..", "..", "..");
}

export function getDevSidecarPath(electronDir: string): string {
  return path.join(getDevRepoRoot(electronDir), "crates", "orkworksd", "target", "debug", "orkworksd");
}
