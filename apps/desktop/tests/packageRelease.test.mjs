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
const releaseWorkflowPath = resolve(desktopRoot, "..", "..", ".github", "workflows", "release.yml");

function findStep(job, name) {
  return job.steps.find((step) => step.name === name);
}

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
  assert.deepEqual(config.win.publish, {
    provider: "github",
    owner: "Rambolarsen",
    repo: "orkworks",
    publisherName: [],
  });
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
  const workflow = yaml.load(readFileSync(releaseWorkflowPath, "utf8"));
  const buildJob = workflow.jobs.build;
  const names = buildJob.steps.map((step) => step.name).filter(Boolean);
  const verifyIndex = names.indexOf("Verify packaged artifact");
  const smokeIndex = names.indexOf("Smoke-test Windows installer");
  const uploadIndex = names.indexOf("Upload artifacts");

  assert.ok(verifyIndex < smokeIndex);
  assert.ok(smokeIndex < uploadIndex);
  assert.equal(
    findStep(buildJob, "Verify packaged artifact").run,
    "pnpm verify:release:pre-checksum",
  );
  assert.equal(findStep(buildJob, "Smoke-test Windows installer").run, "pnpm smoke:windows-installer");
  assert.match(findStep(buildJob, "Smoke-test Windows installer").if, /matrix\.target == ['"]win['"]/);
});

test("release workflow protects platform jobs and maps only their signing credentials", () => {
  const source = readFileSync(releaseWorkflowPath, "utf8");
  const workflow = yaml.load(source);
  const buildJob = workflow.jobs.build;

  assert.deepEqual(workflow.permissions, { contents: "read" });
  assert.equal(buildJob.environment, "release");
  assert.deepEqual(findStep(buildJob, "Package macOS (electron-builder)").env, {
    CSC_LINK: "${{ secrets.MAC_CSC_LINK }}",
    CSC_KEY_PASSWORD: "${{ secrets.MAC_CSC_KEY_PASSWORD }}",
    APPLE_API_KEY_ID: "${{ secrets.APPLE_API_KEY_ID }}",
    APPLE_API_ISSUER: "${{ secrets.APPLE_API_ISSUER }}",
    APPLE_TEAM_ID: "${{ secrets.APPLE_TEAM_ID }}",
  });
  assert.deepEqual(findStep(buildJob, "Package Windows (electron-builder)").env, {
    WIN_CSC_LINK: "${{ secrets.WIN_CSC_LINK }}",
    WIN_CSC_KEY_PASSWORD: "${{ secrets.WIN_CSC_KEY_PASSWORD }}",
    WIN_EXPECTED_PUBLISHER: "${{ vars.WIN_EXPECTED_PUBLISHER }}",
  });
  assert.match(
    findStep(buildJob, "Package Windows (electron-builder)").run,
    /config\.win\.publish\.publisherName = \[process\.env\.WIN_EXPECTED_PUBLISHER\]/,
  );
  const windowsPackageRun = findStep(buildJob, "Package Windows (electron-builder)").run;
  assert.ok(
    windowsPackageRun.indexOf("config.win.publish.publisherName")
      < windowsPackageRun.indexOf("pnpm package:release"),
  );
  assert.equal(
    findStep(buildJob, "Verify native Windows signatures").env.WIN_EXPECTED_PUBLISHER,
    "${{ vars.WIN_EXPECTED_PUBLISHER }}",
  );
  assert.equal(
    findStep(buildJob, "Smoke-test Windows installer").env.WIN_EXPECTED_PUBLISHER,
    "${{ vars.WIN_EXPECTED_PUBLISHER }}",
  );
  assert.doesNotMatch(source, /GH_TOKEN/);
  assert.deepEqual(workflow.jobs.publish.permissions, { contents: "write" });
});

test("release workflow materializes the App Store Connect API key as a temporary path", () => {
  const source = readFileSync(releaseWorkflowPath, "utf8");
  const workflow = yaml.load(source);
  const buildJob = workflow.jobs.build;
  const names = buildJob.steps.map((step) => step.name).filter(Boolean);
  const prepareStep = findStep(buildJob, "Prepare App Store Connect API key");
  const packageStep = findStep(buildJob, "Package macOS (electron-builder)");
  const cleanupStep = findStep(buildJob, "Clean up App Store Connect API key");

  assert.ok(prepareStep, "missing API key preparation step");
  assert.ok(cleanupStep, "missing API key cleanup step");
  assert.ok(names.indexOf(prepareStep.name) < names.indexOf(packageStep.name));
  assert.ok(names.indexOf(packageStep.name) < names.indexOf(cleanupStep.name));
  assert.match(prepareStep.if, /matrix\.target == ['"]mac['"]/);
  assert.deepEqual(prepareStep.env, {
    APPLE_API_KEY_BASE64: "${{ secrets.APPLE_API_KEY }}",
  });
  assert.match(prepareStep.run, /KEY_PATH="\$RUNNER_TEMP\/orkworks-app-store-connect\.p8"/);
  assert.match(prepareStep.run, /base64 --decode > "\$KEY_PATH"/);
  assert.match(prepareStep.run, /chmod 600 "\$KEY_PATH"/);
  assert.match(
    prepareStep.run,
    /printf 'APPLE_API_KEY=%s\\n' "\$KEY_PATH" >> "\$GITHUB_ENV"/,
  );
  assert.doesNotMatch(prepareStep.run, /echo .*APPLE_API_KEY_BASE64/);

  assert.match(packageStep.run, /test -f "\$APPLE_API_KEY"/);
  assert.match(packageStep.run, /trap cleanup EXIT/);
  assert.match(packageStep.run, /rm -f -- "\$APPLE_API_KEY"/);
  assert.ok(packageStep.run.indexOf('test -f "$APPLE_API_KEY"') < packageStep.run.indexOf("pnpm package:release"));
  assert.match(cleanupStep.if, /always\(\).*matrix\.target == ['"]mac['"]/);
  assert.match(cleanupStep.run, /rm -f -- "\$APPLE_API_KEY"/);
  assert.doesNotMatch(source, /APPLE_API_KEY:\s*\$\{\{ secrets\.APPLE_API_KEY \}\}/);
});

test("release workflow verifies real artifacts, creates checksums, and uploads only release files", () => {
  const workflow = yaml.load(readFileSync(releaseWorkflowPath, "utf8"));
  const expectedUploadPath = [
    "apps/desktop/release/OrkWorks-*",
    "apps/desktop/release/latest*.yml",
    "apps/desktop/release/*.blockmap",
    "apps/desktop/release/SHA256SUMS.txt",
  ].join("\n");

  const job = workflow.jobs.build;
  const names = job.steps.map((step) => step.name).filter(Boolean);
  const releaseVerifyIndex = names.indexOf("Verify packaged artifact");
  const smokeIndex = names.indexOf("Smoke-test Windows installer");
  const checksumIndex = names.indexOf("Generate release checksums");
  const finalVerifyIndex = names.indexOf("Verify checksummed release artifact");
  const uploadIndex = names.indexOf("Upload artifacts");

  for (const packageStep of ["Package macOS (electron-builder)", "Package Windows (electron-builder)"]) {
    assert.ok(names.indexOf(packageStep) < releaseVerifyIndex);
  }
  for (const nativeStep of ["Verify native macOS signatures", "Verify native Windows signatures"]) {
    assert.ok(releaseVerifyIndex < names.indexOf(nativeStep));
    assert.ok(names.indexOf(nativeStep) < checksumIndex);
  }
  assert.ok(smokeIndex < checksumIndex);
  assert.ok(checksumIndex < finalVerifyIndex);
  assert.ok(finalVerifyIndex < uploadIndex);
  assert.match(findStep(job, "Package macOS (electron-builder)").run, /pnpm package:release/);
  assert.match(findStep(job, "Package Windows (electron-builder)").run, /pnpm package:release/);
  assert.equal(findStep(job, "Generate release checksums").run, "pnpm checksum:release");
  assert.equal(findStep(job, "Verify packaged artifact").run, "pnpm verify:release:pre-checksum");
  assert.equal(findStep(job, "Verify checksummed release artifact").run, "pnpm verify:release");
  assert.equal(findStep(job, "Upload artifacts").with.path.trim(), expectedUploadPath);

  const macVerification = findStep(job, "Verify native macOS signatures").run;
  assert.match(macVerification, /codesign --verify --deep --strict --verbose=2/);
  assert.match(macVerification, /spctl --assess --type execute/);
  assert.match(macVerification, /xcrun stapler validate "\$app_path"/);
  assert.doesNotMatch(macVerification, /stapler validate "\$DMG_PATH"/);
  assert.match(macVerification, /hdiutil attach .* -readonly/);
  assert.match(macVerification, /trap .*EXIT/);
  assert.match(macVerification, /TEMP_ROOT="\$\(mktemp -d\)"/);
  assert.ok(macVerification.indexOf("trap cleanup EXIT") < macVerification.indexOf("mkdir -p"));
  assert.match(macVerification, /ditto -x -k/);
  assert.match(macVerification, /OrkWorks-\$\{VERSION\}-mac-arm64\.dmg/);
  assert.match(macVerification, /OrkWorks-\$\{VERSION\}-mac-arm64\.zip/);

  const windowsVerification = findStep(job, "Verify native Windows signatures").run;
  assert.match(windowsVerification, /signtool.*verify \/pa \/all \/v/is);
  assert.match(windowsVerification, /Get-AuthenticodeSignature/);
  assert.match(
    windowsVerification,
    /GetNameInfo\(\[System\.Security\.Cryptography\.X509Certificates\.X509NameType\]::SimpleName, \$false\)/,
  );
  assert.match(windowsVerification, /\$publisher -ne \$env:WIN_EXPECTED_PUBLISHER/);
  assert.match(windowsVerification, /OrkWorks-\$version-win-x64\.exe/i);
  assert.match(windowsVerification, /win-unpacked[\\/]OrkWorks\.exe/i);
  assert.match(windowsVerification, /win-unpacked[\\/]resources[\\/]orkworksd\.exe/i);
  assert.match(windowsVerification, /app-update\.yml/);
  assert.match(windowsVerification, /yaml\.load/);
  assert.match(windowsVerification, /publisherName\.length !== 1/);
  assert.match(
    windowsVerification,
    /metadata\.publisherName\[0\] !== process\.env\.WIN_EXPECTED_PUBLISHER/,
  );
  assert.doesNotMatch(windowsVerification, /\.Contains\(/);

  assert.equal(workflow.jobs.publish.needs, "build");
  const publishNames = workflow.jobs.publish.steps.map((step) => step.name).filter(Boolean);
  const downloadStep = workflow.jobs.publish.steps.find((step) => step.uses === "actions/download-artifact@v4");
  const assembleStep = findStep(workflow.jobs.publish, "Assemble release assets");
  assert.equal(downloadStep.with["merge-multiple"], false);
  assert.match(assembleStep.run, /release-mac-arm64\/SHA256SUMS\.txt/);
  assert.match(assembleStep.run, /release-win-x64\/SHA256SUMS\.txt/);
  assert.match(assembleStep.run, /sort > artifacts\/publish\/SHA256SUMS\.txt/);
  const metadataAssertion = findStep(workflow.jobs.publish, "Assert platform update metadata").run;
  assert.match(metadataAssertion, /^test -f artifacts\/publish\/latest-mac\.yml$/m);
  assert.match(metadataAssertion, /^test -f artifacts\/publish\/latest\.yml$/m);
  assert.ok(publishNames.indexOf("Assemble release assets") < publishNames.indexOf("Assert platform update metadata"));
  assert.ok(publishNames.indexOf("Assert platform update metadata") < publishNames.indexOf("Publish draft GitHub Release"));
});

test("desktop package exposes deterministic release checksum generation", () => {
  const packageJson = JSON.parse(readFileSync(resolve(desktopRoot, "package.json"), "utf8"));
  assert.equal(
    packageJson.scripts["verify:release:pre-checksum"],
    "node scripts/verifyReleaseArtifact.mjs --pre-checksum",
  );
  assert.equal(packageJson.scripts["checksum:release"], "node scripts/releaseMetadata.mjs --checksums");
});
