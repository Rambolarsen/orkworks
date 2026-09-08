import test from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";

import {
  createWindowsInstallerExpectation,
  runWindowsInstallerSmokeTest,
  verifyInstalledWindowsApp,
  waitForDirectoryRemoval,
} from "../scripts/windowsInstallerSmokeTest.mjs";

test("Windows expectation names the installer and installed resources", () => {
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\orkworks-smoke",
    productName: "OrkWorks",
  });

  assert.equal(expectation.installerPath, join("C:\\release", "OrkWorks-0.1.0-win-x64.exe"));
  assert.equal(expectation.appPath, join("C:\\temp\\orkworks-smoke", "OrkWorks.exe"));
  assert.equal(expectation.uninstallerPath, join("C:\\temp\\orkworks-smoke", "Uninstall OrkWorks.exe"));
  assert.equal(expectation.sidecarPath, join("C:\\temp\\orkworks-smoke", "resources", "orkworksd.exe"));
  assert.deepEqual(expectation.scriptPaths.map((path) => path.split(/[\\/]/).at(-1)), [
    "report-harness-event.sh",
    "report-harness-event.ps1",
    "report-opencode-session.sh",
  ]);
});

test("unsupported targets fail before any installer process is invoked", async () => {
  const calls = [];
  await assert.rejects(
    () => runWindowsInstallerSmokeTest({
      platform: "darwin",
      arch: "arm64",
      execFileSync: (...args) => calls.push(args),
    }),
    /Unsupported Windows installer smoke-test target: darwin\/arm64/,
  );
  assert.deepEqual(calls, []);
});

test("pre-existing installation directories are rejected without invoking NSIS", async () => {
  const calls = [];
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\orkworks-smoke",
    productName: "OrkWorks",
  });
  const fsModule = {
    existsSync: (path) => path === expectation.installerPath || path === expectation.installDir,
    statSync: (path) => ({
      isFile: () => path === expectation.installerPath || path === expectation.appPath || path === expectation.sidecarPath || expectation.scriptPaths.includes(path),
      isDirectory: () => path === expectation.scriptsDir,
      size: 1,
    }),
  };

  await assert.rejects(
    () => runWindowsInstallerSmokeTest({
      platform: "win32",
      arch: "x64",
      version: "0.1.0",
      releaseDir: "C:\\release",
      installDir: expectation.installDir,
      productName: "OrkWorks",
      fsModule,
      execFileSync: (file, args) => calls.push({ file, args }),
      waitForDirectoryRemoval: async () => {},
    }),
    /pre-existing installation directory/,
  );
  assert.deepEqual(calls, []);
});

test("successful runs invoke silent install then silent uninstall", async () => {
  const calls = [];
  let installExists = false;
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\orkworks-smoke",
    productName: "OrkWorks",
  });
  const fsModule = {
    existsSync: (path) => path === expectation.installDir && installExists,
    statSync(path) {
      if (path === expectation.installerPath || path === expectation.appPath || path === expectation.uninstallerPath || path === expectation.sidecarPath || expectation.scriptPaths.includes(path)) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      if (path === expectation.scriptsDir) {
        return { isFile: () => false, isDirectory: () => true, size: 0 };
      }
      throw new Error("ENOENT");
    },
  };

  await runWindowsInstallerSmokeTest({
    platform: "win32",
    arch: "x64",
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: expectation.installDir,
    productName: "OrkWorks",
    fsModule,
    execFileSync: (file, args) => {
      calls.push({ file, args });
      if (file === expectation.installerPath) installExists = true;
      if (file === expectation.uninstallerPath) installExists = false;
    },
    waitForDirectoryRemoval: async () => {},
  });

  assert.deepEqual(calls, [
    { file: expectation.installerPath, args: ["/S", `/D=${expectation.installDir}`] },
    { file: expectation.uninstallerPath, args: ["/S"] },
  ]);
});

test("installed verification identifies the first missing exact path", () => {
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\orkworks-smoke",
    productName: "OrkWorks",
  });
  const fakeFs = {
    statSync(path) {
      if (path === expectation.scriptsDir || path === expectation.sidecarPath || expectation.scriptPaths.includes(path)) {
        return { isFile: () => path !== expectation.scriptsDir, isDirectory: () => path === expectation.scriptsDir, size: 1 };
      }
      throw new Error("ENOENT");
    },
  };

  assert.throws(
    () => verifyInstalledWindowsApp(expectation, fakeFs),
    (error) => error instanceof Error && error.message.includes(expectation.appPath),
  );
});

test("directory wait succeeds after the directory disappears", async () => {
  let now = 0;
  let exists = true;
  await waitForDirectoryRemoval("C:\\temp\\orkworks-smoke", {
    existsSync: () => exists,
    now: () => now,
    sleep: async (milliseconds) => {
      now += milliseconds;
      exists = false;
    },
    timeoutMs: 1_000,
    pollIntervalMs: 100,
  });
});

test("directory wait reports a bounded cleanup timeout", async () => {
  let now = 0;
  await assert.rejects(
    () => waitForDirectoryRemoval("C:\\temp\\orkworks-smoke", {
      existsSync: () => true,
      now: () => {
        now += 1_001;
        return now;
      },
      sleep: async () => {},
      timeoutMs: 1_000,
      pollIntervalMs: 100,
    }),
    /installation directory remained after 1000ms/,
  );
});
