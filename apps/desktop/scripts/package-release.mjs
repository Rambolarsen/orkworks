import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { createReleaseBuildPlan } from "./packageReleaseConfig.mjs";

const appRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(appRoot, "..", "..");
const sidecarManifestPath = resolve(repoRoot, "crates", "orkworksd", "Cargo.toml");
const sidecarTargetRoot = resolve(repoRoot, "crates", "orkworksd", "target");

function run(command, args, cwd = appRoot) {
  execFileSync(command, args, { cwd, stdio: "inherit" });
}

function stageSidecarBinary(step) {
  const source = resolve(sidecarTargetRoot, step.rustTarget, "release", step.sidecarBinaryName);
  const destination = resolve(sidecarTargetRoot, "release", step.sidecarBinaryName);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(source, destination);
}

for (const step of createReleaseBuildPlan(process.platform, process.arch)) {
  run("cargo", ["build", "--release", "--manifest-path", sidecarManifestPath, "--target", step.rustTarget], repoRoot);
  stageSidecarBinary(step);
  run("npx", ["electron-builder", `--${step.builderTarget}`, `--${step.electronArch}`, "--publish", "never"]);
}
