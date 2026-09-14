# Release Signing and Notarization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make tagged OrkWorks releases produce and verify signed macOS DMG/ZIP and Windows NSIS artifacts with updater metadata and checksums.

**Architecture:** Keep packaging in electron-builder 26.15.x. Platform jobs inject protected credentials, force signing, validate the actual downloadable/installable files, and stage artifacts with `--publish never`; the existing matrix-gated publish job remains the only publisher. Node helpers validate generated metadata/checksums, while native runner commands validate macOS bundles and the installed Windows artifact.

**Tech Stack:** GitHub Actions, electron-builder 26.15.x, Electron 44, Node.js, `js-yaml`, PowerShell/Authenticode, macOS `codesign`/`spctl`/`xcrun`, pnpm, and Node’s built-in test runner.

## Global Constraints

- Use `pnpm` for all Node package-management tasks.
- Use the fixed public GitHub provider `Rambolarsen/orkworks`; do not add an update server.
- macOS builds must use Developer ID signing, hardened runtime, notarization, and stapling.
- Windows NSIS builds must use Authenticode signing and updater publisher verification.
- Use base64-encoded certificate/key values in protected GitHub Actions secrets; never commit or log their contents.
- Set platform-specific `mac.forceCodeSigning: true` and `win.forceCodeSigning: true`; Linux remains local-only and unsigned.
- Keep platform packaging on `--publish never); publish only after every matrix job verifies its artifacts.
- Do not implement runtime update checks/downloads/install UI, nightly publication, channel selection, or older-installed-build update testing; those are issues #511 and #510.
- Preserve the Electron-main/renderer boundary; this plan changes release packaging only.

---

## File Map

| File | Responsibility |
| --- | --- |
| `apps/desktop/package.json` | Direct `js-yaml` dependency and checksum script. |
| `apps/desktop/pnpm-lock.yaml` | Lock the build-tool dependency. |
| `apps/desktop/electron-builder.yml` | Fixed provider, macOS targets/signing, Windows signing/update verification. |
| `apps/desktop/build/entitlements.mac.plist` | Parent hardened-runtime entitlements. |
| `apps/desktop/build/entitlements.mac.inherit.plist` | Child/helper hardened-runtime entitlements. |
| `apps/desktop/scripts/releaseMetadata.mjs` | Deterministic checksum generation and update-metadata validation. |
| `apps/desktop/scripts/verifyReleaseArtifact.mjs` | Expected release files/resources and metadata delegation. |
| `apps/desktop/scripts/windowsInstallerSmokeTest.mjs` | Signed installer install/uninstall and installed-file verification. |
| `apps/desktop/tests/releaseMetadata.test.mjs` | Metadata/checksum tests. |
| `apps/desktop/tests/verifyReleaseArtifact.test.mjs` | Expanded artifact expectation tests. |
| `apps/desktop/tests/packageRelease.test.mjs` | Builder configuration and workflow contract tests. |
| `.github/workflows/release.yml` | Protected signing jobs, native verification, checksums, and artifact allowlist. |
| `README.md` | Release operator summary. |
| `docs/user/getting-started.md` | Installer trust/status wording. |
| `docs/agents/release-signing.md` | Exact credentials and operator runbook. |
| `docs/agents/architecture.md` | Durable release architecture reference. |
| `specs/release-pipeline.md` | Implementation status and external validation boundary. |

---

### Task 1: Add metadata and checksum helpers

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/pnpm-lock.yaml`
- Create: `apps/desktop/scripts/releaseMetadata.mjs`
- Create: `apps/desktop/tests/releaseMetadata.test.mjs`

**Interfaces:**
- `writeChecksumManifest({ releaseDir, outputPath }) -> string`.
- `verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion }) -> { version, files }`.
- The checksum function hashes only top-level distributable files, sorts filenames, and excludes `SHA256SUMS.txt`.
- The metadata function validates version, referenced files, sizes, SHA-512 values, and matching blockmaps.

- [ ] **Step 1: Write failing tests.**

Add `js-yaml` as a direct dev dependency at version `^4.2.0`, run `pnpm install` in `apps/desktop`, then add tests with these behaviors:

```js
test("writes sorted SHA-256 entries without including the manifest itself", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  writeFileSync(join(releaseDir, "OrkWorks-0.2.0-win-x64.exe"), "installer");
  writeFileSync(join(releaseDir, "latest.yml"), "metadata");
  const outputPath = join(releaseDir, "SHA256SUMS.txt");

  assert.equal(writeChecksumManifest({ releaseDir, outputPath }), outputPath);
  const names = readFileSync(outputPath, "utf8").trim().split("\n")
    .map((line) => line.split("  ")[1]);
  assert.deepEqual(names, ["OrkWorks-0.2.0-win-x64.exe", "latest.yml"]);
});

test("verifies metadata version, payload digest, size, and blockmap", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  const payload = "zip payload";
  const payloadName = "OrkWorks-0.2.0-mac-arm64.zip";
  writeFileSync(join(releaseDir, payloadName), payload);
  writeFileSync(join(releaseDir, payloadName + ".blockmap"), "blockmap");
  const metadataPath = join(releaseDir, "latest-mac.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - url: " + payloadName,
    "    sha512: " + createHash("sha512").update(payload).digest("base64"),
    "    size: 11",
  ].join("\n"));

  assert.equal(
    verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }).files[0].url,
    payloadName,
  );
});

test("rejects missing payloads and mismatched SHA-512 values", () => {
  const releaseDir = mkdtempSync(join(tmpdir(), "orkworks-release-"));
  const metadataPath = join(releaseDir, "latest.yml");
  writeFileSync(metadataPath, [
    "version: 0.2.0",
    "files:",
    "  - url: OrkWorks-0.2.0-win-x64.exe",
    "    sha512: invalid",
    "    size: 1",
  ].join("\\n"));

  assert.throws(
    () => verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion: "0.2.0" }),
    /missing payload|SHA-512|digest/i,
  );
});
```

The third test must use a real temporary directory and assert an error that names the missing payload or digest mismatch; do not test only a mock.

- [ ] **Step 2: Run the focused test and verify the expected red failure.**

Run:

```bash
cd apps/desktop
node --experimental-strip-types --test tests/releaseMetadata.test.mjs
```

Expected: FAIL because `releaseMetadata.mjs` does not export the functions.

- [ ] **Step 3: Implement the minimal helper.**

Use `js-yaml` to parse the generated YAML. Use Node `createHash`, `readdirSync`, `readFileSync`, `statSync`, and `writeFileSync`. Reject absolute or path-traversing payload URLs by comparing resolved paths with the resolved release directory. Validate every `files[]` entry’s `url`, `sha512`, and integer `size`; hash the actual payload with SHA-512/base64; require `<payload>.blockmap`; return the verified entries. The checksum CLI must hash only `OrkWorks-*`, `latest*.yml`, and `*.blockmap` files and never itself.

- [ ] **Step 4: Run the focused test green.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/releaseMetadata.test.mjs
```

Expected: all metadata/checksum tests pass.

- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/package.json apps/desktop/pnpm-lock.yaml apps/desktop/scripts/releaseMetadata.mjs apps/desktop/tests/releaseMetadata.test.mjs
git commit -m "build: validate release metadata and checksums"
```

---

### Task 2: Expand packaged-artifact expectations

**Files:**
- Modify: `apps/desktop/scripts/verifyReleaseArtifact.mjs`
- Modify: `apps/desktop/tests/verifyReleaseArtifact.test.mjs`

**Interfaces:**
- `createReleaseArtifactExpectation` adds `distributablePaths`, `metadataPath`, `blockmapPaths`, `appUpdateMetadataPath`, `appPath`, `releaseDir`, `version`, and `checksumPath`.
- `verifyReleaseArtifact(expectation, fsModule, metadataModule)` verifies the required files and delegates metadata validation.

- [ ] **Step 1: Add failing tests.**

For macOS arm64, assert these paths:

```js
distributablePaths: [
  join("/release", "OrkWorks-0.1.0-mac-arm64.dmg"),
  join("/release", "OrkWorks-0.1.0-mac-arm64.zip"),
],
metadataPath: join("/release", "latest-mac.yml"),
blockmapPaths: [join("/release", "OrkWorks-0.1.0-mac-arm64.zip.blockmap")],
appUpdateMetadataPath: join(
  "/release", "mac-arm64", "OrkWorks.app", "Contents", "Resources", "app-update.yml",
),
checksumPath: join("/release", "SHA256SUMS.txt"),
```

Add equivalent Windows assertions for the NSIS installer, `latest.yml`, installer blockmap, `win-unpacked/resources/app-update.yml`, unpacked `OrkWorks.exe`, and `SHA256SUMS.txt`. Add a test that a metadata-module failure is surfaced with the metadata path.

- [ ] **Step 2: Run the focused tests and verify red.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/verifyReleaseArtifact.test.mjs
```

Expected: FAIL because the current expectation object lacks the new fields.

- [ ] **Step 3: Implement the expanded expectation.**

Add the exact platform paths above, retain the current sidecar/hooks/knowledge checks, and inject `metadataModule` with a default import. Call `verifyUpdateMetadata` with the expectation’s `releaseDir`, `metadataPath`, and `version`. Keep `runCli` and unsupported-target behavior unchanged.

- [ ] **Step 4: Run artifact and package tests green.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/verifyReleaseArtifact.test.mjs tests/packageRelease.test.mjs
```

Expected: all tests pass.

- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/scripts/verifyReleaseArtifact.mjs apps/desktop/tests/verifyReleaseArtifact.test.mjs
git commit -m "build: verify updater artifacts in release packages"
```

---

### Task 3: Configure signed electron-builder output

**Files:**
- Modify: `apps/desktop/electron-builder.yml`
- Create: `apps/desktop/build/entitlements.mac.plist`
- Create: `apps/desktop/build/entitlements.mac.inherit.plist`
- Modify: `apps/desktop/tests/packageRelease.test.mjs`

- [ ] **Step 1: Add failing configuration tests.**

Parse `electron-builder.yml` with `js-yaml` and assert:

```js
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
```

Also assert both entitlement files contain the two required keys set to true.

- [ ] **Step 2: Run tests and verify red.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/packageRelease.test.mjs
```

Expected: FAIL because the current config has only a DMG target and no signing contract.

- [ ] **Step 3: Add the configuration.**

Add the fixed provider and preserve all existing resources. The relevant YAML must be:

```yaml
publish:
  provider: github
  owner: Rambolarsen
  repo: orkworks
mac:
  target:
    - dmg
    - zip
  notarize: true
  hardenedRuntime: true
  entitlements: build/entitlements.mac.plist
  entitlementsInherit: build/entitlements.mac.inherit.plist
  binaries:
    - Contents/Resources/orkworksd
  forceCodeSigning: true
win:
  target:
    - nsis
  verifyUpdateCodeSignature: true
  forceCodeSigning: true
```

Do not set a guessed `publisherName`; electron-builder derives it from the supplied certificate and writes it into the packaged updater configuration. Because force signing is platform-specific, Linux packaging is unaffected; release-equivalent local macOS/Windows packaging intentionally requires credentials.

Create both entitlement files with the same content:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key>
  <true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
  <true/>
</dict>
</plist>
```

- [ ] **Step 4: Run config tests green.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/packageRelease.test.mjs
```

Expected: all package/config tests pass.

- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/electron-builder.yml apps/desktop/build/entitlements.mac.plist apps/desktop/build/entitlements.mac.inherit.plist apps/desktop/tests/packageRelease.test.mjs
git commit -m "build: configure signed macOS and Windows releases"
```

---

### Task 4: Harden workflow and verify real artifacts

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `apps/desktop/scripts/windowsInstallerSmokeTest.mjs`
- Modify: `apps/desktop/tests/packageRelease.test.mjs`
- Modify: `apps/desktop/tests/verifyReleaseArtifact.test.mjs`

**Interfaces:**
- Both platform jobs use GitHub Environment `release`.
- macOS decodes the protected `APPLE_API_KEY` content to a temporary `.p8`; packaging receives its path as `APPLE_API_KEY` plus `CSC_LINK`, `CSC_KEY_PASSWORD`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER`, and `APPLE_TEAM_ID`.
- Windows receives `WIN_CSC_LINK`, `WIN_CSC_KEY_PASSWORD`, and `WIN_EXPECTED_PUBLISHER`.
- Packaging, release verification, native signature checks, checksum generation, and upload occur in that order.

- [ ] **Step 1: Add failing workflow/smoke tests.**

Add static assertions that the workflow contains `environment: release`, `permissions: contents: read`, the platform-specific secret mappings, `latest*.yml`, `*.blockmap`, and `SHA256SUMS.txt`; assert verification precedes checksum generation and upload, and that packaging still uses `--publish never`. Add smoke tests for valid/invalid Authenticode status and publisher mismatch.

- [ ] **Step 2: Run tests and verify red.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/packageRelease.test.mjs tests/verifyReleaseArtifact.test.mjs
```

Expected: FAIL because the current workflow does not protect signing jobs or upload updater files.

- [ ] **Step 3: Add the checksum command.**

Add `"checksum:release": "node scripts/releaseMetadata.mjs --checksums"` to `apps/desktop/package.json`. The CLI reads the package version and the existing release directory, calls `writeChecksumManifest`, and prints only the output path.

- [ ] **Step 4: Protect jobs and map secrets.**

Set workflow-level `permissions: contents: read` and `environment: release` on the matrix build job. Before macOS packaging, map the protected secret to a differently named step environment value, decode it to a mode-600 `.p8` under `RUNNER_TEMP`, and export only that path as `APPLE_API_KEY` through `GITHUB_ENV`. The package step must verify the path and remove the file with an exit trap, followed by an `always()` cleanup step. In the macOS packaging step map only:

```yaml
CSC_LINK: ${{ secrets.MAC_CSC_LINK }}
CSC_KEY_PASSWORD: ${{ secrets.MAC_CSC_KEY_PASSWORD }}
APPLE_API_KEY_ID: ${{ secrets.APPLE_API_KEY_ID }}
APPLE_API_ISSUER: ${{ secrets.APPLE_API_ISSUER }}
APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

In the Windows packaging/verification steps map:

```yaml
WIN_CSC_LINK: ${{ secrets.WIN_CSC_LINK }}
WIN_CSC_KEY_PASSWORD: ${{ secrets.WIN_CSC_KEY_PASSWORD }}
WIN_EXPECTED_PUBLISHER: ${{ vars.WIN_EXPECTED_PUBLISHER }}
```

Do not pass `GH_TOKEN` to platform packaging; it uses `--publish never`. Keep `contents: write` only on the publish job. The repository operator must configure required environment reviewers and protected `v*` tags before secrets are added.

- [ ] **Step 5: Verify actual macOS artifacts.**

After `pnpm verify:release`, use a macOS-only shell step to:

1. Read the version from `package.json`.
2. Run `codesign --verify --deep --strict --verbose=2` on the staged app and the in-bundle sidecar.
3. Run `spctl --assess --type execute` and `xcrun stapler validate` on the staged app.
4. Mount the produced DMG read-only with `hdiutil`, verify the contained app with the same commands, and detach in a trap.
5. Extract the produced ZIP with `ditto -x -k`, verify the contained app, and clean temporary directories even after failure.

Use the exact produced artifact paths, not a separately rebuilt app.

- [ ] **Step 6: Verify the signed Windows installer and installed files.**

Keep `pnpm smoke:windows-installer` before upload and make it run against the signed NSIS artifact. Extend its injectable verification seam so the installed app executable and `resources/orkworksd.exe` must have Authenticode status `Valid` and the expected signer subject. Preserve the existing failure behavior that leaves the install directory for diagnosis.

Add a Windows-only PowerShell step that runs `signtool verify /pa /all /v` and `Get-AuthenticodeSignature` for the NSIS installer, unpacked app executable, and sidecar. Require status `Valid`, compare each signer subject to `WIN_EXPECTED_PUBLISHER`, and assert that `release/win-unpacked/resources/app-update.yml` contains the same publisher identity.

- [ ] **Step 7: Generate and upload the complete asset set.**

Run:

```yaml
- name: Generate release checksums
  working-directory: apps/desktop
  run: pnpm checksum:release
```

Replace the current `OrkWorks-*`-only upload with this allowlist:

```yaml
path: |
  apps/desktop/release/OrkWorks-*
  apps/desktop/release/latest*.yml
  apps/desktop/release/*.blockmap
  apps/desktop/release/SHA256SUMS.txt
```

Before publish, assert that both platform metadata files are present. Do not upload unpacked staging directories or builder-effective configuration.

- [ ] **Step 8: Run the focused tests green.**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/packageRelease.test.mjs tests/verifyReleaseArtifact.test.mjs tests/releaseMetadata.test.mjs
```

Expected: all source tests pass. Native signing checks remain credential-backed CI gates.

- [ ] **Step 9: Commit.**

```bash
git add .github/workflows/release.yml apps/desktop/scripts/windowsInstallerSmokeTest.mjs apps/desktop/tests/packageRelease.test.mjs apps/desktop/tests/verifyReleaseArtifact.test.mjs apps/desktop/package.json apps/desktop/scripts/releaseMetadata.mjs
git commit -m "ci: gate releases on signed artifact verification"
```

---

### Task 5: Document operation and status

**Files:**
- Create: `docs/agents/release-signing.md`
- Modify: `README.md`
- Modify: `docs/user/getting-started.md`
- Modify: `docs/agents/architecture.md`
- Modify: `specs/release-pipeline.md`

- [ ] **Step 1: Add the operator runbook.**

Document Apple Developer membership, Developer ID Application `.p12`, App Store Connect `.p8`, trusted Authenticode `.pfx`/managed signing, protected GitHub Environment `release`, required reviewers, protected `v*` tags, and the exact names:

```text
MAC_CSC_LINK
MAC_CSC_KEY_PASSWORD
APPLE_API_KEY
APPLE_API_KEY_ID
APPLE_API_ISSUER
APPLE_TEAM_ID
WIN_CSC_LINK
WIN_CSC_KEY_PASSWORD
WIN_EXPECTED_PUBLISHER
```

Explain base64 export without printing values, credential rotation, exact publisher-subject matching, and that self-signed Windows certificates are local-testing-only. Include commands/evidence for native checks and state that #511 still owns installed older-build update testing.

- [ ] **Step 2: Update user and architecture wording.**

Change README release notes to include signed DMG/ZIP/NSIS, metadata, and verification gates after credentials are provisioned. In `docs/user/getting-started.md`, distinguish historical unsigned alpha installers from credential-backed signed releases. In `docs/agents/architecture.md`, record that electron-builder signs/notarizes in CI and secrets are never packaged; runtime updater operations remain Electron-main work for #511. Update `specs/release-pipeline.md` with source-wiring status while keeping credential-backed platform validation an external prerequisite.

- [ ] **Step 3: Validate documentation.**

```bash
bash scripts/verify-repo.sh
git diff --check
```

Expected: repository checks pass and all links resolve.

- [ ] **Step 4: Commit.**

```bash
git add docs/agents/release-signing.md README.md docs/user/getting-started.md docs/agents/architecture.md specs/release-pipeline.md
git commit -m "docs: document signed release operations"
```

---

### Task 6: Full verification and delivery handoff

**Files:** Verify all files changed by Tasks 1–5.

- [ ] **Step 1: Run desktop checks.**

```bash
cd apps/desktop
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: type-check exits 0 and every desktop test passes.

- [ ] **Step 2: Run repository checks.**

```bash
cd ../..
bash scripts/verify-repo.sh
git diff --check
git status --short --branch
```

Expected: repository verification passes and only the owned `release-signing` branch changes are present.

- [ ] **Step 3: Inspect the workflow before PR.**

Confirm tag-only `v*` triggering, protected `release` environment, read-only build permissions, platform-secret isolation, `--publish never`, native verification before checksums/upload, complete metadata allowlist, and write permission only on publish.

- [ ] **Step 4: Record external prerequisites accurately.**

Do not call a credential-free local package signed. If credentials are unavailable, record the remaining handoff: Developer ID certificate/password, App Store Connect key/ID/issuer/team values, trusted Authenticode certificate/password, exact Windows publisher subject, and one credential-backed macOS arm64/Windows x64 tagged run.

- [ ] **Step 5: Run the explicit `/code-review low` gate.**

This change touches release security and desktop packaging. Address findings or document technical pushback in the PR description before merge.

- [ ] **Step 6: Prepare one issue #509 PR.**

```bash
git log --oneline --decorate -8
git status --short --branch
git diff origin/main...HEAD --stat
```

Include source-test results, secret names without values, native credential-backed verification status, external blockers, and links to issue #509, the design spec, and this plan. Do not claim release readiness until required checks and the external signed-artifact run are recorded.

## Plan Self-Review

- **Spec coverage:** configuration, API-key notarization, Windows certificate signing, nested sidecar signing, DMG/ZIP/NSIS outputs, updater metadata, checksums, protected secrets, fail-closed behavior, native artifact verification, documentation, and separation from #510/#511 are covered.
- **Placeholder scan:** no task depends on TBD/TODO or an unnamed helper; the only owner-provisioned runtime value is `WIN_EXPECTED_PUBLISHER`.
- **Interface consistency:** Task 1 defines the metadata/checksum exports; Task 2 consumes them through the verifier; Task 4 uses the same artifact fields and secret names; Task 5 documents the same contract.
- **Known external gate:** actual credentials, Apple notarization, Windows certificate trust, and installed update testing remain external prerequisites by design.
