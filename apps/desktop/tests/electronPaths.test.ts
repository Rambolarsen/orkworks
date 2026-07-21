import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";

import { getDevRepoRoot, getPackagedSidecarPath } from "../electron/paths.ts";

test("resolves the repo root from the compiled Electron directory", () => {
  const root = path.join("/Users/example/workspace/orkworks");
  const compiledElectronDir = path.join(root, "apps", "desktop", "dist-electron");

  assert.equal(getDevRepoRoot(compiledElectronDir), path.resolve(root));
});

test("uses the .exe suffix for packaged Windows sidecars", () => {
  assert.equal(
    getPackagedSidecarPath("C:\\Users\\example\\AppData\\Local\\Programs\\OrkWorks\\resources", "win32"),
    path.join("C:\\Users\\example\\AppData\\Local\\Programs\\OrkWorks\\resources", "orkworksd.exe"),
  );
});
