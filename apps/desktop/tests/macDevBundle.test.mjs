import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

// Catch launching the generic Electron bundle or mutating the installed one.
test("macOS dev prepares and reuses a signed OrkWorks copy of Electron", { skip: process.platform !== "darwin" }, async (t) => {
  const { prepareMacDevBundle } = await import("../scripts/macDevBundle.mjs");
  const require = createRequire(import.meta.url);
  const installedElectron = dirname(require.resolve("electron/package.json"));
  const sourcePlist = join(installedElectron, "dist/Electron.app/Contents/Info.plist");
  const originalPlist = readFileSync(sourcePlist);
  const root = mkdtempSync(join(tmpdir(), "orkworks-dev-name-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "node_modules"));
  symlinkSync(installedElectron, join(root, "node_modules/electron"));
  writeFileSync(join(root, "package.json"), "{}");

  const executable = prepareMacDevBundle(root);
  assert.ok(executable.startsWith(join(root, "node_modules/.cache/")));
  assert.ok(executable.endsWith("/OrkWorks.app/Contents/MacOS/Electron"));
  const bundle = dirname(dirname(dirname(executable)));
  const plist = join(bundle, "Contents/Info.plist");
  const plistValue = (key) => execFileSync("/usr/libexec/PlistBuddy", ["-c", `Print :${key}`, plist], { encoding: "utf8" }).trim();
  assert.equal(plistValue("CFBundleName"), "OrkWorks");
  assert.equal(plistValue("CFBundleDisplayName"), "OrkWorks");
  assert.equal(plistValue("CFBundleIdentifier"), "ai.orkworks.desktop.dev");
  assert.equal(plistValue("CFBundleExecutable"), "Electron");
  // Preserve default_app.asar: passing '.' must still use the development entrypoint.
  assert.ok(statSync(join(bundle, "Contents/Resources/default_app.asar")).isFile());
  execFileSync("/usr/bin/codesign", ["--verify", "--deep", "--strict", bundle]);
  assert.deepEqual(readFileSync(sourcePlist), originalPlist);
  const modifiedAt = statSync(plist).mtimeMs;
  assert.equal(prepareMacDevBundle(root), executable);
  assert.equal(statSync(plist).mtimeMs, modifiedAt);
});

// Architecture reinstalls can replace binaries without changing their path or plist.
test("macOS dev invalidates the bundle cache when the source executable changes", { skip: process.platform !== "darwin" }, async (t) => {
  const { prepareMacDevBundle } = await import("../scripts/macDevBundle.mjs");
  const require = createRequire(import.meta.url);
  const installedElectron = dirname(require.resolve("electron/package.json"));
  const root = mkdtempSync(join(tmpdir(), "orkworks-dev-arch-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const fixtureElectron = join(root, "node_modules/electron");
  cpSync(installedElectron, fixtureElectron, { recursive: true, verbatimSymlinks: true });
  writeFileSync(join(root, "package.json"), "{}");
  const before = prepareMacDevBundle(root);
  const sourceExecutable = join(fixtureElectron, "dist/Electron.app/Contents/MacOS/Electron");
  const sourcePlist = join(fixtureElectron, "dist/Electron.app/Contents/Info.plist");
  const plist = readFileSync(sourcePlist);
  // A valid replacement Mach-O proves invalidation with unchanged path/metadata.
  cpSync("/usr/bin/true", sourceExecutable);
  assert.deepEqual(readFileSync(sourcePlist), plist);
  const after = prepareMacDevBundle(root);
  assert.notEqual(after, before);
});
