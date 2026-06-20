import test from "node:test";
import assert from "node:assert/strict";

import { createReleaseBuildPlan } from "../scripts/packageReleaseConfig.mjs";

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
