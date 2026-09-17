import assert from "node:assert/strict";
import { cp, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("packaged updater loads from an isolated production dependency tree", async () => {
  const manifestUrl = new URL("../package.json", import.meta.url);
  const manifest = JSON.parse(await readFile(manifestUrl, "utf8"));
  assert.equal(manifest.dependencies["electron-updater"], "6.8.9");
  assert.equal(manifest.devDependencies["electron-updater"], undefined);
  const directory = await mkdtemp(path.join(tmpdir(), "orkworks-updater-production-"));
  const copied = new Set();
  // Copy only dependency edges used by production; never copy node_modules
  // wholesale, which would accidentally make development modules resolvable.
  async function copyDependency(name, parentRequire) {
    if (copied.has(name)) return;
    copied.add(name);
    const sourceManifest = parentRequire.resolve(`${name}/package.json`);
    const source = path.dirname(sourceManifest);
    const destination = path.join(directory, "node_modules", name);
    await mkdir(path.dirname(destination), { recursive: true });
    await cp(source, destination, {
      recursive: true,
      filter: (entry) => entry === source || path.basename(entry) !== "node_modules",
    });
    const pkg = JSON.parse(await readFile(sourceManifest, "utf8"));
    for (const dependency of Object.keys(pkg.dependencies ?? {})) {
      await copyDependency(dependency, createRequire(sourceManifest));
    }
  }
  try {
    await copyDependency("electron-updater", createRequire(manifestUrl));
    const result = spawnSync(process.execPath, ["-e", `
      const assert = require('node:assert/strict');
      const updater = require('electron-updater');
      assert.equal(typeof updater.NsisUpdater, 'function');
      assert.equal(typeof updater.MacUpdater, 'function');
      assert.throws(() => require.resolve('electron-builder'));
      assert.throws(() => require.resolve('typescript'));
    `], { cwd: directory, encoding: "utf8", env: { ...process.env, NODE_PATH: "" } });
    assert.equal(result.status, 0, result.stderr || result.stdout);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
