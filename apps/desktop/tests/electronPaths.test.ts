import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";

import { getDevRepoRoot } from "../electron/paths.ts";

test("resolves the repo root from the compiled Electron directory", () => {
  const compiledElectronDir = path.join(
    "/Users/example/workspace/orkworks",
    "apps",
    "desktop",
    "dist-electron",
  );

  assert.equal(getDevRepoRoot(compiledElectronDir), "/Users/example/workspace/orkworks");
});
