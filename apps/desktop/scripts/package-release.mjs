import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { createReleaseBuildPlan, electronBuilderInvocation } from "./packageReleaseConfig.mjs";

const appRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(appRoot, "..", "..");
const sidecarManifestPath = resolve(repoRoot, "crates", "orkworksd", "Cargo.toml");
const sidecarTargetRoot = resolve(repoRoot, "crates", "orkworksd", "target");
const electronBuilderCliPath = resolve(appRoot, "node_modules", "electron-builder", "cli.js");

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
  const cargoArgs = ["build", "--release", "--manifest-path", sidecarManifestPath, "--target", step.rustTarget];
  if (step.builderTarget === "mac") {
    // The packaged sidecar cannot depend on the signing machine's Homebrew libraries.
    cargoArgs.push("--locked", "--features", "vendored-git");
  }
  run("cargo", cargoArgs, repoRoot);
  stageSidecarBinary(step);
  const { command, args } = electronBuilderInvocation(process.execPath, electronBuilderCliPath, step, {
    channel: process.env.ORKWORKS_RELEASE_CHANNEL ?? "latest",
    buildVersion: process.env.ORKWORKS_BUILD_VERSION,
    macBundleVersion: process.env.ORKWORKS_MAC_BUNDLE_VERSION,
  });
  run(command, args);
}
