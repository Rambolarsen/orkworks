import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const require = createRequire(import.meta.url);
const root = resolve(import.meta.dirname, "..");
const electron = require("electron");
const { version } = require("electron/package.json");

function probe() {
  // Check the binary itself: .forge-meta can survive a host-Node rebuild or
  // arrive from pnpm's cache alongside an addon built for a different ABI.
  const result = spawnSync(electron, ["-e", `
    try { require("fs-ext"); }
    catch (error) { console.error(error.message); process.exit(1); }
  `], {
    cwd: root,
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    encoding: "utf8",
    timeout: 30_000,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`fs-ext failed to load in Electron ${version}: ${result.stderr?.trim() || result.signal || result.status}`);
  }
}

try {
  probe();
} catch (error) {
  console.warn(`[build:native] ${error.message}`);
  console.log(`[build:native] rebuilding fs-ext for Electron ${version} (${process.arch})`);
  const { rebuild } = await import("@electron/rebuild");
  await rebuild({
    buildPath: root,
    electronVersion: version,
    arch: process.arch,
    onlyModules: ["fs-ext"],
    force: true,
    mode: "sequential",
  });
  // A failed repair must stop dev/build/dist rather than defer failure to app startup.
  probe();
}
console.log(`[build:native] fs-ext verified in Electron ${version}`);
