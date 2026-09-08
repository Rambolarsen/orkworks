# Windows Installer Smoke Test Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Windows x64 NSIS release artifact pass a real install, resource verification, uninstall, and cleanup smoke test before CI uploads it.

**Architecture:** Keep `verify:release` as the static unpacked-artifact gate and add a separate parameterized Node module that invokes the generated NSIS installer and uninstaller. The Windows release matrix runs the new module after static verification and before upload; unit tests inject filesystem, process, clock, and sleep dependencies so non-Windows development hosts can test the contract without executing a Windows binary.

**Tech Stack:** Node.js ESM, Node built-in `fs`/`path`/`os`/`child_process`, Node test runner, pnpm, electron-builder 26.15.3, GitHub Actions Windows runner, NSIS.

**Spec:** `docs/superpowers/specs/2026-09-08-windows-installer-smoke-design.md`

## Global Constraints

- Release target remains unsigned Windows x64 NSIS; no signing, SmartScreen, auto-update, branding, or GUI automation.
- NSIS silent install uses `/S` followed by final `/D=<install-dir>`; `/D=` must remain the final installer argument.
- The smoke test refuses a pre-existing installation directory and never recursively deletes a caller-selected path.
- A failed post-install assertion retains the temporary directory for CI inspection; successful cleanup is performed by the NSIS uninstaller.
- Required installed resources are `OrkWorks.exe`, `resources/orkworksd.exe`, `resources/scripts/report-harness-event.sh`, `resources/scripts/report-harness-event.ps1`, and `resources/scripts/report-opencode-session.sh`.
- Use pnpm for desktop package commands and keep the Electron-main/renderer boundary unchanged.
- Every task ends with its own focused test and commit.

---

### Task 1: Build the testable Windows installer smoke-test module

**Files:**
- Create: `apps/desktop/scripts/windowsInstallerSmokeTest.mjs`
- Create: `apps/desktop/tests/windowsInstallerSmokeTest.test.mjs`
- Modify: `apps/desktop/package.json`

**Interfaces:**
- Consumes: `createReleaseArtifactExpectation(platform, arch, version, releaseDir)` from `apps/desktop/scripts/verifyReleaseArtifact.mjs` and the current `productName`/`version` from `apps/desktop/package.json`.
- Produces: `createWindowsInstallerExpectation(options)`, `verifyInstalledWindowsApp(expectation, fsModule)`, `waitForDirectoryRemoval(path, options)`, and `runWindowsInstallerSmokeTest(options)` exports. The CLI calls `runWindowsInstallerSmokeTest()` when the module is executed directly.

- [ ] **Step 1: Write the failing expectation and command tests**

Add `apps/desktop/tests/windowsInstallerSmokeTest.test.mjs` with these concrete cases:

```js
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
      if (path === expectation.appPath || path === expectation.scriptsDir || path === expectation.sidecarPath || expectation.scriptPaths.includes(path)) {
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
  await assert.rejects(
    () => waitForDirectoryRemoval("C:\\temp\\orkworks-smoke", {
      existsSync: () => true,
      now: () => 1_001,
      sleep: async () => {},
      timeoutMs: 1_000,
      pollIntervalMs: 100,
    }),
    /installation directory remained after 1000ms/,
  );
});
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```bash
node --experimental-strip-types --test tests/windowsInstallerSmokeTest.test.mjs
```

Expected: FAIL because `apps/desktop/scripts/windowsInstallerSmokeTest.mjs` does not exist yet.

- [ ] **Step 3: Implement the expectation builder and installed-resource verifier**

Create `apps/desktop/scripts/windowsInstallerSmokeTest.mjs` with these concrete rules:

```js
export function createWindowsInstallerExpectation({
  version,
  releaseDir,
  installDir,
  productName,
}) {
  const packaged = createReleaseArtifactExpectation("win32", "x64", version, releaseDir);
  const scriptsDir = join(installDir, "resources", "scripts");
  return {
    installerPath: packaged.installerPath,
    installDir,
    appPath: join(installDir, `${productName}.exe`),
    uninstallerPath: join(installDir, `Uninstall ${productName}.exe`),
    sidecarPath: join(installDir, "resources", "orkworksd.exe"),
    scriptsDir,
    scriptPaths: packaged.scriptPaths.map((path) => join(scriptsDir, basename(path))),
  };
}

export function verifyInstalledWindowsApp(expectation, fsModule = fs) {
  assertNonEmptyFile(fsModule, expectation.appPath, "installed application");
  assertNonEmptyFile(fsModule, expectation.sidecarPath, "installed Rust sidecar");
  assertDirectory(fsModule, expectation.scriptsDir, "installed hook scripts");
  for (const scriptPath of expectation.scriptPaths) {
    assertNonEmptyFile(fsModule, scriptPath, "installed hook script");
  }
}
```

Reuse the existing release verifier's Windows script list through
`createReleaseArtifactExpectation` rather than defining a second list of
reporter names. Keep error messages path-specific and use `statSync` so empty
files fail just as they do in `verifyReleaseArtifact.mjs`.

- [ ] **Step 4: Implement bounded cleanup polling and the runner**

Add the polling helper with injectable time dependencies:

```js
export async function waitForDirectoryRemoval(
  installDir,
  {
    existsSync = fs.existsSync,
    now = () => performance.now(),
    sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
    timeoutMs = 30_000,
    pollIntervalMs = 250,
  } = {},
) {
  const deadline = now() + timeoutMs;
  while (existsSync(installDir)) {
    if (now() >= deadline) {
      throw new Error(`Windows installer smoke test failed: installation directory remained after ${timeoutMs}ms: ${installDir}`);
    }
    await sleep(pollIntervalMs);
  }
}
```

Implement `runWindowsInstallerSmokeTest(options = {})` with defaults for the
current process platform/architecture, package version/product name, release
directory, and a unique install directory under
`ORKWORKS_INSTALLER_SMOKE_DIR`, `RUNNER_TEMP`, or `os.tmpdir()`. The runner
must perform these checks in this order:

```js
if (platform !== "win32" || arch !== "x64") throw unsupportedTargetError;
if (fsModule.existsSync(installDir)) throw preExistingDirectoryError;
assertNonEmptyFile(fsModule, expectation.installerPath, "installer");
execFileSync(expectation.installerPath, ["/S", `/D=${expectation.installDir}`], { stdio: "inherit" });
verifyInstalledWindowsApp(expectation, fsModule);
assertNonEmptyFile(fsModule, expectation.uninstallerPath, "uninstaller");
execFileSync(expectation.uninstallerPath, ["/S"], { stdio: "inherit" });
await waitForDirectoryRemoval(expectation.installDir, waitOptions);
```

Wrap installer and uninstaller process errors with the executable path and
operation name while retaining the original error as `cause`. Do not remove
the installation directory in a `finally` block. The CLI should read
`package.json`, call the runner, and allow thrown errors to produce a non-zero
exit code.

- [ ] **Step 5: Add the package command and run focused tests**

Add this script to `apps/desktop/package.json`:

```json
"smoke:windows-installer": "node scripts/windowsInstallerSmokeTest.mjs"
```

Run:

```bash
pnpm smoke:windows-installer
```

Expected on this macOS development host: the command exits non-zero with
`Unsupported Windows installer smoke-test target: darwin/arm64`.

Then run:

```bash
node --experimental-strip-types --test tests/windowsInstallerSmokeTest.test.mjs
```

Expected: the focused tests pass.

- [ ] **Step 6: Commit the module and unit tests**

```bash
git add apps/desktop/scripts/windowsInstallerSmokeTest.mjs apps/desktop/tests/windowsInstallerSmokeTest.test.mjs apps/desktop/package.json
git commit -m "test: add Windows installer smoke-test contract"
```

### Task 2: Gate the release workflow on a real Windows install/uninstall

**Files:**
- Modify: `.github/workflows/release.yml:73-83`
- Modify: `apps/desktop/tests/packageRelease.test.mjs` (add release-script wiring coverage)

**Interfaces:**
- Consumes: `pnpm smoke:windows-installer` from Task 1 and the existing Windows matrix fields `matrix.target == "win"`, `matrix.arch == "x64"`, and `runner.temp`.
- Produces: a blocking workflow step that runs before `Upload artifacts` and leaves the existing top-level artifact upload unchanged.

- [ ] **Step 1: Add a failing wiring assertion**

Extend `apps/desktop/tests/packageRelease.test.mjs` with a test that reads the workflow and asserts the smoke step is Windows-only, runs the package script, and precedes the upload step:

```js
test("release workflow smoke-tests Windows installers before upload", () => {
  const workflow = readFileSync(resolve(import.meta.dirname, "../../../.github/workflows/release.yml"), "utf8");
  const smokeIndex = workflow.indexOf("Smoke-test Windows installer");
  const uploadIndex = workflow.indexOf("Upload artifacts");
  assert.ok(smokeIndex >= 0);
  assert.ok(smokeIndex < uploadIndex);
  assert.match(workflow.slice(smokeIndex, uploadIndex), /if: matrix\.target == ['\"]win['\"]/);
  assert.match(workflow.slice(smokeIndex, uploadIndex), /run: pnpm smoke:windows-installer/);
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```bash
node --experimental-strip-types --test tests/packageRelease.test.mjs
```

Expected: FAIL because the workflow has no smoke-test step yet.

- [ ] **Step 3: Add the Windows smoke-test step**

Insert this step after `Verify packaged artifact` and before `Upload artifacts`:

```yaml
      - name: Smoke-test Windows installer
        if: matrix.target == 'win'
        working-directory: apps/desktop
        run: pnpm smoke:windows-installer
        env:
          ORKWORKS_INSTALLER_SMOKE_DIR: ${{ runner.temp }}\orkworks-installer-smoke-${{ github.run_id }}-${{ github.run_attempt }}
```

Keep `Upload artifacts` unchanged. Because the smoke step is in the existing
build matrix and has no `continue-on-error`, an install, resource, uninstall,
or cleanup failure blocks upload and therefore blocks the publish job.

- [ ] **Step 4: Run workflow wiring and desktop tests**

Run:

```bash
node --experimental-strip-types --test tests/packageRelease.test.mjs tests/windowsInstallerSmokeTest.test.mjs
```

Expected: PASS, including the new ordering assertion.

- [ ] **Step 5: Commit the workflow gate**

```bash
git add .github/workflows/release.yml apps/desktop/tests/packageRelease.test.mjs
git commit -m "ci: smoke-test Windows installer before upload"
```

### Task 3: Update release documentation and validate the complete change

**Files:**
- Modify: `specs/release-pipeline.md` sections Scope & Non-Goals, Architecture, and Edge Cases
- Modify: `README.md` release section near the GitHub Releases commands

**Interfaces:**
- Consumes: the workflow behavior and CLI contract from Tasks 1–2.
- Produces: documentation that states the Windows install/uninstall smoke test is a release gate while preserving the unsigned-alpha limitations.

- [ ] **Step 1: Update the release specification**

Change the release spec’s scope and non-goals to include this exact distinction:

```markdown
### In scope

- Windows x64 NSIS install/uninstall smoke validation on the `windows-latest`
  release runner after packaging.

### Non-goals

- Broad Windows-version, architecture, or locale compatibility testing.
- GUI automation or first-launch runtime testing.
```

In the workflow description, add the smoke-test step after `pnpm verify:release`:

```markdown
10. On the Windows runner, runs `pnpm smoke:windows-installer`, which silently
    installs the generated NSIS artifact into a unique temporary directory,
    verifies the installed executable, Rust sidecar, and hook scripts, then
    uninstalls it and requires the directory to disappear within a bounded
    timeout.
```

Add an edge-case row:

```markdown
| Windows installer or uninstaller fails | The Windows build fails before artifact upload, with the failing executable or installed path in the log |
```

- [ ] **Step 2: Update the README release description**

After the existing sentence describing artifact verification, add:

```markdown
The Windows release job also silently installs and uninstalls its NSIS artifact
in a temporary directory, checking the installed executable, Rust sidecar, and
harness hook scripts before the artifact can be uploaded.
```

- [ ] **Step 3: Run documentation and diff checks**

Run:

```bash
git diff --check
bash scripts/doc-check.sh
```

Expected: both commands exit 0 with no flagged documentation drift.

- [ ] **Step 4: Run the complete desktop and Rust validation**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Run from the repository root:

```bash
cargo fmt --check --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: desktop tests pass, Rust tests pass, and formatting is clean. The
real NSIS install/uninstall step is validated by the Windows release workflow;
it cannot be reproduced on this macOS host.

- [ ] **Step 5: Run the fleet worktree check**

```bash
bash .claude/hooks/worktree-check.sh
```

Expected: exit 0. Only act on findings for the owned
`windows-installer-smoke-test` worktree.

- [ ] **Step 6: Commit the documentation updates**

```bash
git add specs/release-pipeline.md README.md
git commit -m "docs: document Windows installer smoke validation"
```

## Final review checklist

- [ ] Issue #497 acceptance criteria map one-to-one to Tasks 1–3.
- [ ] The smoke test never uses an existing directory or broad recursive cleanup.
- [ ] `/D=` is the final installer argument in both implementation and tests.
- [ ] Static artifact verification and installed-layout verification remain separate.
- [ ] The workflow cannot upload artifacts after a smoke-test failure.
- [ ] Documentation no longer claims that Windows artifacts are built but not tested.
- [ ] No code under `electron/` or `src/` changed.
