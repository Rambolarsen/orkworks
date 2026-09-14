import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import yaml from "js-yaml";

import {
  createReleaseBuildPlan,
  electronBuilderInvocation,
} from "../scripts/packageReleaseConfig.mjs";

const desktopRoot = resolve(import.meta.dirname, "..");

test("electron-builder config declares signed release targets", () => {
  const config = yaml.load(
    readFileSync(resolve(desktopRoot, "electron-builder.yml"), "utf8"),
  );

  assert.deepEqual(config.publish, {
    provider: "github",
    owner: "Rambolarsen",
    repo: "orkworks",
  });
  assert.deepEqual(config.mac.target, ["dmg", "zip"]);
  assert.equal(config.mac.notarize, true);
  assert.equal(config.mac.hardenedRuntime, true);
  assert.deepEqual(config.mac.binaries, ["Contents/Resources/orkworksd"]);
  assert.equal(config.mac.forceCodeSigning, true);
  assert.deepEqual(config.win.target, ["nsis"]);
  assert.equal(config.win.verifyUpdateCodeSignature, true);
  assert.equal(config.win.forceCodeSigning, true);
});

test("macOS entitlements allow the sidecar runtime requirements", () => {
  for (const filename of [
    "build/entitlements.mac.plist",
    "build/entitlements.mac.inherit.plist",
  ]) {
    const entitlements = readFileSync(resolve(desktopRoot, filename), "utf8");

    assert.match(
      entitlements,
      /<key>com\.apple\.security\.cs\.allow-jit<\/key>\s*<true\/>/,
    );
    assert.match(
      entitlements,
      /<key>com\.apple\.security\.cs\.allow-unsigned-executable-memory<\/key>\s*<true\/>/,
    );
  }
});

test("desktop package declares the GitHub repository for release metadata", () => {
  const packageJson = JSON.parse(
    readFileSync(resolve(import.meta.dirname, "..", "package.json"), "utf8"),
  );

  assert.deepEqual(packageJson.repository, {
    type: "git",
    url: "https://github.com/Rambolarsen/orkworks.git",
  });
});

test("release packaging invokes electron-builder's local CLI through Node", () => {
  assert.deepEqual(
    electronBuilderInvocation("node", "/app/node_modules/electron-builder/cli.js", {
      builderTarget: "win",
      electronArch: "x64",
    }),
    {
      command: "node",
      args: ["/app/node_modules/electron-builder/cli.js", "--win", "--x64", "--publish", "never"],
    },
  );
});

test("macOS x64 release plan uses the x64 Rust target", () => {
  assert.deepEqual(createReleaseBuildPlan("darwin", "x64"), [
    {
      builderTarget: "mac",
      electronArch: "x64",
      rustTarget: "x86_64-apple-darwin",
      sidecarBinaryName: "orkworksd",
    },
  ]);
});

test("macOS arm64 release plan uses the arm64 Rust target", () => {
  assert.deepEqual(createReleaseBuildPlan("darwin", "arm64"), [
    {
      builderTarget: "mac",
      electronArch: "arm64",
      rustTarget: "aarch64-apple-darwin",
      sidecarBinaryName: "orkworksd",
    },
  ]);
});

test("Windows release plan uses the .exe sidecar", () => {
  assert.deepEqual(createReleaseBuildPlan("win32", "x64"), [
    {
      builderTarget: "win",
      electronArch: "x64",
      rustTarget: "x86_64-pc-windows-msvc",
      sidecarBinaryName: "orkworksd.exe",
    },
  ]);
});

test("Linux release plan uses the Linux GNU target", () => {
  assert.deepEqual(createReleaseBuildPlan("linux", "x64"), [
    {
      builderTarget: "linux",
      electronArch: "x64",
      rustTarget: "x86_64-unknown-linux-gnu",
      sidecarBinaryName: "orkworksd",
    },
  ]);
});

test("release workflow smoke-tests Windows installers before upload", () => {
  const workflow = readFileSync(resolve(import.meta.dirname, "../../../.github/workflows/release.yml"), "utf8");
  const verifyIndex = workflow.indexOf("Verify packaged artifact");
  const smokeIndex = workflow.indexOf("Smoke-test Windows installer");
  const uploadIndex = workflow.indexOf("Upload artifacts");
  assert.ok(verifyIndex >= 0);
  assert.ok(smokeIndex >= 0);
  assert.ok(uploadIndex >= 0);
  assert.ok(verifyIndex < smokeIndex);
  assert.ok(smokeIndex < uploadIndex);
  assert.match(workflow.slice(verifyIndex, smokeIndex), /run: pnpm verify:release/);
  assert.match(workflow.slice(smokeIndex, uploadIndex), /if: matrix\.target == ['\"]win['\"]/);
  assert.match(workflow.slice(smokeIndex, uploadIndex), /run: pnpm smoke:windows-installer/);
});
