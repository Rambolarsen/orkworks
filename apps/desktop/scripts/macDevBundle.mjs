import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { constants, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

// app.setName() changes Electron's internal name, not macOS's application name.
// Keep the dependency bundle pristine; brand only a checkout-local dev copy.
export function prepareMacDevBundle(root) {
  if (process.platform !== "darwin") return undefined;
  const require = createRequire(join(root, "package.json"));
  const sourceExecutable = require("electron");
  const sourceBundle = dirname(dirname(dirname(sourceExecutable)));
  const sourcePlist = readFileSync(join(sourceBundle, "Contents/Info.plist"));
  const fingerprint = createHash("sha256")
    .update("orkworks-dev-bundle-v1\0")
    .update(sourceExecutable)
    .update(sourcePlist)
    .digest("hex").slice(0, 16);
  const cacheRoot = join(root, "node_modules/.cache/orkworks-electron");
  const cache = join(cacheRoot, fingerprint);
  const executable = join(cache, "OrkWorks.app/Contents/MacOS/Electron");
  if (existsSync(executable)) return executable;

  mkdirSync(cacheRoot, { recursive: true });
  const staging = mkdtempSync(join(cacheRoot, "prepare-"));
  try {
    const bundle = join(staging, "OrkWorks.app");
    cpSync(sourceBundle, bundle, { recursive: true, verbatimSymlinks: true, mode: constants.COPYFILE_FICLONE });
    const plist = join(bundle, "Contents/Info.plist");
    for (const [key, value] of [
      ["CFBundleName", "OrkWorks"],
      ["CFBundleDisplayName", "OrkWorks"],
      ["CFBundleIdentifier", "ai.orkworks.desktop.dev"],
    ]) {
      execFileSync("/usr/bin/plutil", ["-replace", key, "-string", value, plist]);
    }
    // Bundle metadata is sealed by the signature. Re-sign only this local dev
    // copy ad hoc, retaining entitlements. The downloaded dev bundle has
    // linker signatures without a resource seal, so include its nested code.
    execFileSync("/usr/bin/codesign", ["--force", "--deep", "--sign", "-", "--preserve-metadata=entitlements", bundle]);
    execFileSync("/usr/bin/codesign", ["--verify", "--deep", "--strict", bundle]);
    try {
      renameSync(staging, cache);
    } catch (error) {
      // A concurrent dev launch may have published the same complete copy.
      if (!["EEXIST", "ENOTEMPTY"].includes(error.code) || !existsSync(executable)) throw error;
    }
    return executable;
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}
