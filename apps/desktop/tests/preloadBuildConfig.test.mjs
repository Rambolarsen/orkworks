import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";

import { preloadBuildOptions } from "../scripts/preloadBuildConfig.mjs";

test("preload bundle inlines local imports but keeps electron external", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const options = preloadBuildOptions(root);

  assert.deepEqual(options.entryPoints, [path.join(root, "electron/preload.ts")]);
  assert.equal(options.outfile, path.join(root, "dist-electron/preload.js"));
  assert.equal(options.bundle, true);
  assert.equal(options.platform, "node");
  assert.equal(options.format, "cjs");
  assert.deepEqual(options.external, ["electron"]);
});
