import test from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";

import * as smokeTest from "../scripts/windowsInstallerSmokeTest.mjs";
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
      registryProbe: () => false,
      waitForDirectoryRemoval: async () => {},
    }),
    /pre-existing installation directory/,
  );
  assert.deepEqual(calls, []);
});

test("registered OrkWorks installations are rejected before invoking NSIS", async () => {
  const calls = [];
  let probeCalls = 0;
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\orkworks-smoke",
    productName: "OrkWorks",
  });
  const fsModule = {
    existsSync: (path) => path === expectation.installerPath,
    statSync: (path) => ({
      isFile: () => path === expectation.installerPath,
      isDirectory: () => false,
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
      registryProbe: () => {
        probeCalls += 1;
        return true;
      },
      execFileSync: (file, args) => calls.push({ file, args }),
    }),
    /registered OrkWorks installation/,
  );
  assert.equal(probeCalls, 1);
  assert.deepEqual(calls, []);
});

test("registry probe checks both per-user and per-machine uninstall data", () => {
  for (const registeredRoot of [
    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
  ]) {
    const calls = [];
    assert.equal(typeof smokeTest.isWindowsInstallationRegistered, "function");
    const result = smokeTest.isWindowsInstallationRegistered("OrkWorks", (file, args, options) => {
      calls.push({ file, args, options });
      if (args[1] === registeredRoot) {
        return "DisplayName    REG_SZ    OrkWorks";
      }
      const error = new Error("registry value not found");
      error.status = 1;
      throw error;
    });

    assert.equal(result, true);
    assert.equal(calls[0].file, "reg.exe");
    assert.deepEqual(calls.map(({ args }) => args[1]), [
      "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
      ...(registeredRoot === "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall"
        ? [registeredRoot]
        : []),
    ]);
    assert.deepEqual(calls[0].args, [
      "query",
      calls[0].args[1],
      "/s",
      "/v",
      "DisplayName",
      "/f",
      "OrkWorks",
      "/d",
    ]);
    assert.deepEqual(calls[0].options, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      shell: false,
      windowsHide: true,
    });
  }
});

test("registry probe detects electron-builder's versioned uninstall display name", () => {
  const calls = [];
  const result = smokeTest.isWindowsInstallationRegistered("OrkWorks", (file, args) => {
    calls.push({ file, args });
    if (args.includes("/e")) {
      const error = new Error("exact bare-name match not found");
      error.status = 1;
      throw error;
    }
    return "DisplayName    REG_SZ    OrkWorks 0.1.0";
  });

  assert.equal(result, true);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].file, "reg.exe");
  assert.equal(calls[0].args.includes("/e"), false);
});

test("registry probe ignores reg.exe output without a matching display name", () => {
  const result = smokeTest.isWindowsInstallationRegistered("OrkWorks", () => [
    "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "",
    "End of search: 0 match(es) found.",
  ].join("\\r\\n"));

  assert.equal(result, false);
});

test("successful runs invoke silent install then silent uninstall", async () => {
  const calls = [];
  let installExists = false;
  const expectation = createWindowsInstallerExpectation({
    version: "0.1.0",
    releaseDir: "C:\\release",
    installDir: "C:\\temp\\OrkWorks Smoke",
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
    execFileSync: (file, args, options) => {
      calls.push({ file, args, options });
      if (file === expectation.installerPath) installExists = true;
      if (file === expectation.uninstallerPath) installExists = false;
    },
    registryProbe: () => false,
    waitForDirectoryRemoval: async () => {},
  });

  assert.deepEqual(calls, [
    {
      file: expectation.installerPath,
      args: ["/S", `/D=${expectation.installDir}`],
      options: { stdio: "inherit", shell: false, windowsVerbatimArguments: true },
    },
    {
      file: expectation.uninstallerPath,
      args: ["/S"],
      options: { stdio: "inherit", shell: false, windowsVerbatimArguments: true },
    },
  ]);
});

test("missing post-install resources leave the install directory for diagnosis", async () => {
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
      if (path === expectation.installerPath || path === expectation.appPath) {
        return { isFile: () => true, isDirectory: () => false, size: 1 };
      }
      throw new Error("ENOENT");
    },
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
      registryProbe: () => false,
      execFileSync: (file, args, options) => {
        calls.push({ file, args, options });
        installExists = true;
      },
    }),
    /installed Rust sidecar missing/,
  );
  assert.equal(installExists, true);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].file, expectation.installerPath);
});

test("cleanup wait rejection propagates after the uninstaller runs", async () => {
  const calls = [];
  let installExists = false;
  const cleanupError = new Error("cleanup wait failed");
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

  await assert.rejects(
    () => runWindowsInstallerSmokeTest({
      platform: "win32",
      arch: "x64",
      version: "0.1.0",
      releaseDir: "C:\\release",
      installDir: expectation.installDir,
      productName: "OrkWorks",
      fsModule,
      registryProbe: () => false,
      execFileSync: (file, args, options) => {
        calls.push({ file, args, options });
        if (file === expectation.installerPath) installExists = true;
        if (file === expectation.uninstallerPath) installExists = false;
      },
      waitForDirectoryRemoval: async () => {
        throw cleanupError;
      },
    }),
    (error) => error === cleanupError,
  );
  assert.deepEqual(calls.map(({ file }) => file), [
    expectation.installerPath,
    expectation.uninstallerPath,
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
