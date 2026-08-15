# Release Artifact Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the tag-driven release workflow fail clearly when an installer or its runtime resources are missing, and upload only installable release files.

**Architecture:** A focused Node module computes the expected installer, unpacked app, sidecar, and hook-script paths for the host platform/architecture. Its CLI validates those paths after electron-builder runs; the GitHub Actions matrix invokes the CLI before uploading top-level release artifacts.

**Tech Stack:** Node.js ESM, Node built-in `fs`/`path`, Node test runner, pnpm, electron-builder, GitHub Actions.

## Global Constraints

- Keep the release scope to unsigned macOS arm64 DMG and Windows x64 NSIS artifacts.
- Use `pnpm` for Node package-management commands.
- Keep the Rust sidecar bundled as an Electron `extraResources` file.
- Do not add signing, auto-update, Linux publishing, or automatic public release publication.
- Preserve the Electron-main/renderer boundary; this change touches packaging scripts and CI only.
- Preserve unrelated user changes, including `apps/desktop/tests/scratch-repro.mjs` if present.

---

### Task 1: Add the tested artifact expectation module

**Files:**
- Create: `apps/desktop/scripts/verifyReleaseArtifact.mjs`
- Create: `apps/desktop/tests/verifyReleaseArtifact.test.mjs`

**Interfaces:**
- Produces `createReleaseArtifactExpectation(platform, arch, version, releaseDir)` returning `{ installerPath, appDir, sidecarPath, scriptsDir }`.
- Produces `verifyReleaseArtifact(expectation, fsModule)` returning `void` or throwing an actionable `Error`.

- [ ] **Step 1: Write the failing tests**

Add tests that assert the macOS expectation names the DMG, app bundle sidecar,
and scripts directory, that the Windows expectation uses the NSIS `.exe` and
`.exe` sidecar, and that unsupported platform/architecture combinations throw.
Also test that a missing required path produces an error naming that path.

```js
import test from "node:test";
import assert from "node:assert/strict";
import { createReleaseArtifactExpectation, verifyReleaseArtifact } from "../scripts/verifyReleaseArtifact.mjs";

test("macOS expectation points at the DMG and packaged resources", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  assert.deepEqual(expectation, {
    installerPath: "/release/OrkWorks-0.1.0-mac-arm64.dmg",
    appDir: "/release/mac-arm64/OrkWorks.app",
    sidecarPath: "/release/mac-arm64/OrkWorks.app/Contents/Resources/orkworksd",
    scriptsDir: "/release/mac-arm64/OrkWorks.app/Contents/Resources/scripts",
  });
});

test("Windows expectation points at the NSIS installer and exe sidecar", () => {
  const expectation = createReleaseArtifactExpectation("win32", "x64", "0.1.0", "C:\\release");
  assert.equal(expectation.installerPath, "C:\\release/OrkWorks-0.1.0-win-x64.exe");
  assert.equal(expectation.sidecarPath, "C:\\release/win-unpacked/resources/orkworksd.exe");
});

test("unsupported release targets are rejected", () => {
  assert.throws(() => createReleaseArtifactExpectation("linux", "x64", "0.1.0", "/release"), /Unsupported release target/);
});

test("missing packaged resources identify the failing path", () => {
  const expectation = createReleaseArtifactExpectation("darwin", "arm64", "0.1.0", "/release");
  const fakeFs = {
    statSync(path) {
      if (path === expectation.installerPath) return { isFile: () => true, size: 1 };
      throw new Error("ENOENT");
    },
  };
  assert.throws(() => verifyReleaseArtifact(expectation, fakeFs), new RegExp(expectation.appDir));
});
```

- [ ] **Step 2: Run the focused test and verify it fails for the missing module**

Run from `apps/desktop/`:

```bash
node --test tests/verifyReleaseArtifact.test.mjs
```

Expected: FAIL because `scripts/verifyReleaseArtifact.mjs` does not exist yet.

- [ ] **Step 3: Implement the minimal expectation and verification functions**

Use `resolve` for all generated paths. Return the exact platform-specific
paths above. Validate the installer with `statSync` and require a positive
file size; validate `appDir`, `sidecarPath`, and `scriptsDir` as existing paths
with `statSync`. Wrap filesystem failures in an error that names the failed
path and says the packaged artifact is incomplete. Export the two functions.

- [ ] **Step 4: Run the focused tests and verify they pass**

```bash
node --test tests/verifyReleaseArtifact.test.mjs
```

Expected: all focused tests pass with zero failures.

- [ ] **Step 5: Commit the verifier and tests**

```bash
git add apps/desktop/scripts/verifyReleaseArtifact.mjs apps/desktop/tests/verifyReleaseArtifact.test.mjs
git commit -m "test: define release artifact verification contract"
```

### Task 2: Add the release verification CLI

**Files:**
- Modify: `apps/desktop/scripts/verifyReleaseArtifact.mjs`
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/tests/verifyReleaseArtifact.test.mjs`

**Interfaces:**
- CLI command: `pnpm verify:release`.
- CLI derives `process.platform`, `process.arch`, package version, and the
  `release/` directory, then calls the exported verifier.

- [ ] **Step 1: Write the failing CLI behavior test**

Add a test that confirms the module exposes a callable `runCli` or equivalent
entry function without executing it during import, so unit tests remain
platform-independent. The CLI must print a concise success summary and throw
on verification failure.

- [ ] **Step 2: Run the focused test to verify the entry point is absent**

```bash
node --test tests/verifyReleaseArtifact.test.mjs
```

Expected: FAIL because the CLI entry function is not exported yet.

- [ ] **Step 3: Implement the CLI and package script**

Read `apps/desktop/package.json` using `readFileSync`, compute the release
directory from `import.meta.dirname`, and export `runCli`. Guard execution
with the standard `import.meta.url` check so importing the module never starts
filesystem validation. Add:

```json
"verify:release": "node scripts/verifyReleaseArtifact.mjs"
```

On success print the installer path and sidecar path. On failure let the
actionable error reach stderr and exit non-zero.

- [ ] **Step 4: Run focused tests and the CLI against the existing package**

```bash
node --test tests/verifyReleaseArtifact.test.mjs
pnpm verify:release
```

Expected: tests pass and the CLI reports the generated macOS arm64 installer
and sidecar paths.

- [ ] **Step 5: Commit the CLI**

```bash
git add apps/desktop/scripts/verifyReleaseArtifact.mjs apps/desktop/package.json apps/desktop/tests/verifyReleaseArtifact.test.mjs
git commit -m "feat: verify packaged release resources"
```

### Task 3: Wire CI and documentation

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `specs/release-pipeline.md`
- Modify: `skills/cutting-release/SKILL.md`
- Modify: `README.md`

**Interfaces:**
- The release build job runs `pnpm verify:release` after packaging.
- Artifact upload matches only `apps/desktop/release/OrkWorks-*`, keeping
  installable top-level artifacts and their release metadata.

- [ ] **Step 1: Add the CI verification step and narrow artifact upload**

Insert a `Verify packaged artifact` step after `pnpm package:release` with
`working-directory: apps/desktop` and `run: pnpm verify:release`. Change the
upload path from `apps/desktop/release/*` to
`apps/desktop/release/OrkWorks-*`.

- [ ] **Step 2: Update release documentation**

Document that CI verifies the installer, unpacked sidecar, and hook scripts
before upload, and that the draft release receives top-level `OrkWorks-*`
artifacts. Correct `skills/cutting-release/SKILL.md` so its expected release
list matches the actual two-platform workflow rather than listing Linux and
Intel macOS artifacts.

- [ ] **Step 3: Run YAML/configuration checks**

```bash
git diff --check
node --test apps/desktop/tests/verifyReleaseArtifact.test.mjs
```

Expected: no whitespace errors and all verifier tests pass.

- [ ] **Step 4: Commit the workflow and documentation**

```bash
git add .github/workflows/release.yml specs/release-pipeline.md skills/cutting-release/SKILL.md README.md
git commit -m "ci: verify and publish installable release artifacts"
```

### Task 4: Full verification and review handoff

**Files:**
- No additional source files; inspect all changed files and generated output.

- [ ] **Step 1: Run desktop tests and type-check**

From `apps/desktop/`:

```bash
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

- [ ] **Step 2: Run Rust tests**

From the repository root:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

- [ ] **Step 3: Rebuild and verify the host installer**

From `apps/desktop/`:

```bash
pnpm build
pnpm package:release
pnpm verify:release
```

Expected: the macOS arm64 DMG exists and the verifier confirms the sidecar
and scripts directory.

- [ ] **Step 4: Run repository end-of-session checks**

From the repository root:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
git diff --check
git status --short
```

- [ ] **Step 5: Request code review before handoff**

Review the branch diff against the branch point, address critical/important
findings, and report any remaining inability to run Windows CI locally.
