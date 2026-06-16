import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";

import { createViteServerOptions, electronSpawnConfig } from "../scripts/devConfig.mjs";

test("dev server uses the desktop Vite config and root", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const options = createViteServerOptions(root);

  assert.equal(options.root, root);
  assert.equal(options.configFile, path.join(root, "vite.config.ts"));
  assert.equal(options.server.port, 5173);
  assert.equal(options.server.strictPort, true);
});

test("dev script launches Electron through pnpm instead of npx", () => {
  const config = electronSpawnConfig("/tmp/orkworks/apps/desktop", "http://localhost:5173/");

  assert.equal(config.command, "pnpm");
  assert.deepEqual(config.args, ["exec", "electron", "."]);
  assert.equal(config.options.cwd, "/tmp/orkworks/apps/desktop");
  assert.equal(config.options.env.VITE_DEV_SERVER_URL, "http://localhost:5173/");
});
