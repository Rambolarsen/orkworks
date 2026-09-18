import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { createViteServerOptions, electronSpawnConfig } from "../scripts/devConfig.mjs";

test("dev server uses the desktop Vite config and root", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const previous = process.env.ORKWORKS_DEV_PORT;
  delete process.env.ORKWORKS_DEV_PORT;
  try {
    const options = createViteServerOptions(root);

    assert.equal(options.root, root);
    assert.equal(options.configFile, path.resolve(root, "vite.config.mjs"));
    assert.equal(options.server.port, 5173);
    assert.equal(options.server.strictPort, true);
  } finally {
    if (previous !== undefined) {
      process.env.ORKWORKS_DEV_PORT = previous;
    }
  }
});

test("ORKWORKS_DEV_PORT overrides the dev server port", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const previous = process.env.ORKWORKS_DEV_PORT;
  process.env.ORKWORKS_DEV_PORT = "5273";
  try {
    const options = createViteServerOptions(root);
    assert.equal(options.server.port, 5273);
    assert.equal(options.server.strictPort, true);
  } finally {
    if (previous === undefined) {
      delete process.env.ORKWORKS_DEV_PORT;
    } else {
      process.env.ORKWORKS_DEV_PORT = previous;
    }
  }
});

test("ORKWORKS_DEV_PORT accepts the upper port boundary", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const previous = process.env.ORKWORKS_DEV_PORT;
  process.env.ORKWORKS_DEV_PORT = "65535";
  try {
    const options = createViteServerOptions(root);
    assert.equal(options.server.port, 65535);
    assert.equal(options.server.strictPort, true);
  } finally {
    if (previous === undefined) {
      delete process.env.ORKWORKS_DEV_PORT;
    } else {
      process.env.ORKWORKS_DEV_PORT = previous;
    }
  }
});

test("invalid ORKWORKS_DEV_PORT values fall back to the default port", () => {
  const root = path.join("/tmp", "orkworks", "apps", "desktop");
  const previous = process.env.ORKWORKS_DEV_PORT;
  for (const value of ["", " ", "not-a-port", "0", "70000", "5173.5"]) {
    process.env.ORKWORKS_DEV_PORT = value;
    try {
      const options = createViteServerOptions(root);
      assert.equal(options.server.port, 5173);
      assert.equal(options.server.strictPort, true);
    } finally {
      if (previous === undefined) {
        delete process.env.ORKWORKS_DEV_PORT;
      } else {
        process.env.ORKWORKS_DEV_PORT = previous;
      }
    }
  }
});

test("dev script launches Electron through pnpm instead of npx", () => {
  const config = electronSpawnConfig("/tmp/orkworks/apps/desktop", "http://localhost:5173/");

  assert.equal(config.options.cwd, "/tmp/orkworks/apps/desktop");
  assert.equal(config.options.env.VITE_DEV_SERVER_URL, "http://localhost:5173/");

  if (process.platform === "win32") {
    // Windows .CMD wrappers require shell:true; folding args into the
    // command string (instead of a separate args array) avoids Node's
    // DEP0190 shell-arg-concatenation warning.
    assert.equal(config.command, "pnpm.CMD exec electron .");
    assert.deepEqual(config.args, []);
    assert.equal(config.options.shell, true);
  } else {
    assert.equal(config.command, "pnpm");
    assert.deepEqual(config.args, ["exec", "electron", "."]);
    assert.equal(config.options.shell, false);
  }
});

test("desktop dev command rebuilds the Rust sidecar before launching", async () => {
  const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));

  assert.match(packageJson.scripts.dev, /build:rust/);
  assert.match(packageJson.scripts.dev, /build:rust.*tsc/);
});
