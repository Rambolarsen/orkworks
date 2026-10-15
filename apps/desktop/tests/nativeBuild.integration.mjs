// Run separately with pnpm test:native: this changes fs-ext between Node and
// Electron ABIs and must not race the ordinary Node test suite.
import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const require = createRequire(import.meta.url);
const electron = require("electron");
const addonRoot = dirname(require.resolve("fs-ext/package.json"));
const pnpmCli = process.env.npm_execpath;

function pnpm(...args) {
  assert.ok(pnpmCli, "run this integration test through pnpm test:native");
  execFileSync(process.execPath, [pnpmCli, ...args], { cwd: root, stdio: "pipe", timeout: 120_000 });
}

function electronEval(code) {
  return execFileSync(electron, ["-e", `try { ${code} } catch (error) { console.error(error.message); process.exit(1); }`], {
    cwd: root,
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    encoding: "utf8",
    timeout: 30_000,
  }).trim();
}

test("build:native repairs a host-Node addon with a stale Electron marker and permits repeated startup", () => {
  const temporary = mkdtempSync(join(tmpdir(), "orkworks-native-abi-"));
  try {
    const abi = electronEval("console.log(process.versions.modules)");
    assert.notEqual(abi, process.versions.modules, "this regression requires distinct host Node and Electron ABIs");
    pnpm("rebuild", "fs-ext");
    execFileSync(process.execPath, ["-e", 'require("fs-ext")'], { cwd: root });
    // Recreate the misleading marker found in the frozen pnpm installation.
    writeFileSync(join(addonRoot, "build/Release/.forge-meta"), `${process.arch}--${abi}`);
    // Characterize the pinned installer's stale-marker skip before our verifier runs.
    pnpm("exec", "electron-builder", "install-app-deps");
    assert.throws(() => electronEval('require("fs-ext")'), /NODE_MODULE_VERSION/);
    pnpm("build:native");
    const lock = join(temporary, "native.lock");
    const probe = `
      const fs = require("node:fs");
      const { flockSync } = require("fs-ext");
      const fd = fs.openSync(${JSON.stringify(lock)}, "a+");
      try { flockSync(fd, "exnb"); flockSync(fd, "un"); }
      finally { fs.closeSync(fd); }
      console.log("locked and unlocked");
    `;
    assert.equal(electronEval(probe), "locked and unlocked");
    const binary = join(addonRoot, "build/Release/fs_ext.node");
    const rebuilt = statSync(binary, { bigint: true }).mtimeNs;
    pnpm("build:native");
    assert.equal(electronEval(probe), "locked and unlocked");
    assert.equal(statSync(binary, { bigint: true }).mtimeNs, rebuilt, "a working addon need not rebuild on each startup");
  } finally {
    pnpm("rebuild", "fs-ext");
    execFileSync(process.execPath, ["-e", 'require("fs-ext")'], { cwd: root });
    rmSync(temporary, { recursive: true, force: true });
  }
});
