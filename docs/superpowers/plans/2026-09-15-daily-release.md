# Daily Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Use superpowers:test-driven-development for every behavior change and superpowers:verification-before-completion before each commit, push, PR, or completion claim.

**Goal:** Publish one signed, verified, immutable nightly prerelease for each eligible `main` commit while preserving the existing stable-tag release contract.

**Architecture:** Extend the existing release workflow into a stable/nightly pipeline, expose Main CI as a reusable workflow, and keep release identity and remote-release validation in small dependency-free Node modules. Both channels package the same frozen source commit with channel-specific metadata; a protected GitHub environment and a CI-only fine-grained token form the privileged trust boundary.

**Tech Stack:** GitHub Actions, Node.js 22 ESM and `node:test`, pnpm 11, electron-builder 26, electron-updater 6.8.9, GitHub REST APIs, Rust/Cargo, macOS codesigning/notarization, Windows Azure Trusted Signing.

## Global constraints

- Work only in `C:\Users\froma\source\repos\orkworks-daily-release` on branch `daily-release`.
- Preserve the stable release behavior: canonical `vMAJOR.MINOR.PATCH` tags create signed draft releases with `latest.yml` and `latest-mac.yml`.
- Freeze `github.sha` once. Never fetch a newer `main` SHA or retarget, delete, force-update, or reuse a mismatched tag.
- No signing or `contents: write` job may run without `environment: release`.
- The nightly path must use `secrets.RELEASE_GITHUB_TOKEN`, never package it, and fail before packaging when it is absent or under-scoped.
- Keep nightly update metadata isolated as `nightly.yml` and `nightly-mac.yml`; reject stable metadata in nightly assets.
- Do not close issue #510 until a credential-backed manual run has published and verified a real signed nightly.
- Do not add runtime update behavior; that belongs to issue #511.

## Interfaces and files

- New `apps/desktop/scripts/dailyRelease.mjs`: deterministic versioning, tag/source marker parsing, expected asset names, remote release validation, pagination, and GitHub release/tag operations.
- New `apps/desktop/scripts/prepareDailyRelease.mjs`: GitHub Actions CLI entry point that writes validated outputs and stages nightly versions.
- New `apps/desktop/scripts/publishDailyRelease.mjs`: GitHub Actions CLI entry point that creates, validates, and publishes a nightly draft.
- New `apps/desktop/tests/dailyRelease.test.mjs`: pure release identity and remote integrity tests.
- New `apps/desktop/tests/nightlyUpdateChannel.test.mjs`: pinned electron-updater GitHub-provider mixed-feed contract.
- Changed `apps/desktop/scripts/packageReleaseConfig.mjs`: accept a release channel and native build versions when constructing electron-builder arguments.
- Changed `apps/desktop/scripts/releaseMetadata.mjs` and `verifyReleaseArtifact.mjs`: validate one explicit `latest` or `nightly` channel.
- Changed `.github/workflows/main-ci.yml`: add `workflow_call` and an immutable checkout input.
- Changed `.github/workflows/release.yml`: unified stable/nightly orchestration.
- New `docs/adr/0059-immutable-daily-prereleases.md`, plus ADR index and release operator documentation updates.

## Task 1: Record the release trust-boundary decision

**Files:**

- Create: `docs/adr/0059-immutable-daily-prereleases.md`
- Modify: `docs/adr/README.md`

- [ ] Write the failing documentation assertion by adding this temporary check to the command line, before creating the ADR:

  ```powershell
  rg -n "0059.*Immutable daily prereleases.*accepted" docs/adr/README.md
  ```

  Expected: exit 1 because ADR 0059 is not indexed.

- [ ] Create ADR 0059 using `docs/adr/template.md`. Record these accepted decisions exactly:

  - scheduled/manual nightlies freeze the workflow commit from `main`;
  - unique immutable SemVer tags and GitHub prereleases replace rolling tags;
  - `latest` and custom `nightly` feeds are isolated;
  - stable tags use `GITHUB_TOKEN`, while historical nightly tag/release writes use a protected CI-only fine-grained token with Contents and Workflows write access;
  - all signing/write jobs reference the protected `release` environment;
  - published-release integrity is a prerequisite for deduplication and publication;
  - runtime updater behavior remains #511.

- [ ] Add ADR 0059 to `docs/adr/README.md` as `accepted`, rerun the `rg` assertion, then run:

  ```powershell
  bash scripts/doc-check.sh
  git diff --check
  ```

  Expected: all commands exit 0.

- [ ] Commit:

  ```powershell
  git add docs/adr/0059-immutable-daily-prereleases.md docs/adr/README.md
  git commit -m "docs: record immutable daily release architecture"
  ```

## Task 2: Build the deterministic daily-release contract

**Files:**

- Create: `apps/desktop/scripts/dailyRelease.mjs`
- Create: `apps/desktop/tests/dailyRelease.test.mjs`

- [ ] Write failing tests for the public API below. Use fixture factories in the test file; do not call GitHub or the filesystem from these unit tests.

  ```js
  export function parseStableTag(tag, packageVersion) {}
  export function createNightlyIdentity({
    baseVersion,
    utcDate,
    runId,
    runNumber,
    runAttempt,
  }) {}
  export function sourceMarker(sourceSha) {}
  export function parseSourceMarker(body) {}
  export function expectedReleaseAssetNames({ version, channel }) {}
  export function validatePublishedRelease({
    release,
    sourceSha,
    expectedAssetNames,
    tagTargetSha,
    downloadedAssets,
  }) {}
  export function selectPublishedNightlyForSource(validatedReleases, sourceSha) {}
  export function assertCandidateIsNewest(candidateVersion, validatedPublishedNightlies) {}
  ```

  Cover at least:

  - canonical stable tags accept `v0.2.0` and reject `v01.2.3`, `v0.2.0-rc.1`, `v0.2.0+build`, and a package-version mismatch;
  - nightly identity is `0.2.0-nightly.20260915.123456789.2` and tag `v0.2.0-nightly.20260915.123456789.2`;
  - Windows build version is `2026.258.<runNumber>.<attempt>` and each component is `0..65535`;
  - macOS bundle version uses the documented base-100 tuple, is strictly increasing, and rejects attempts outside `1..99` and overflow;
  - a marker is exactly `<!-- orkworks-nightly-source:<40-lowercase-hex> -->` on its own body line;
  - stable and nightly expected asset sets are exact and disjoint for metadata names;
  - drafts, stable releases, non-boolean release flags, malformed schemas, duplicate markers, wrong tag targets, zero-sized assets, missing/extra assets, absent GitHub `sha256:` digests, checksum mismatches, metadata payload mismatches, and duplicate valid source matches fail closed;
  - an authenticated asset 404 leaves damaged retryable history while other download failures fail closed;
  - a validated unchanged-SHA release no-ops before a new finite native identity is constructed;
  - packaged `app-update.yml` files match the expected GitHub repository and normalized stable or explicit nightly channel;
  - public SemVer-valid nightly-channel tags constrain ordering even when their strict workflow identity is invalid;
  - four-digit UTC years and a non-mutating duplicate-ref write-scope probe fail before packaging;
  - a complete published prerelease succeeds and a candidate version must exceed every validated published nightly.

- [ ] Run the new test and confirm the red state:

  ```powershell
  Set-Location apps/desktop
  node --test tests/dailyRelease.test.mjs
  ```

  Expected: failure because `scripts/dailyRelease.mjs` or its exports do not exist.

- [ ] Implement the smallest dependency-free module that makes the tests pass. Parse SemVer with strict numeric components; use `node:crypto` for SHA-256 checks. Require exact asset-name equality, not subset containment. Treat any ambiguous or malformed remote state as an error.

- [ ] Run:

  ```powershell
  node --test tests/dailyRelease.test.mjs
  node --test tests/*.test.mjs
  ```

  Expected: both commands exit 0.

- [ ] Commit:

  ```powershell
  git add apps/desktop/scripts/dailyRelease.mjs apps/desktop/tests/dailyRelease.test.mjs
  git commit -m "feat: define immutable nightly release contract"
  ```

## Task 3: Make release packaging channel-aware

**Files:**

- Modify: `apps/desktop/scripts/packageReleaseConfig.mjs`
- Modify: `apps/desktop/scripts/package-release.mjs`
- Modify: `apps/desktop/scripts/releaseMetadata.mjs`
- Modify: `apps/desktop/scripts/verifyReleaseArtifact.mjs`
- Modify: `apps/desktop/tests/packageRelease.test.mjs`
- Modify: `apps/desktop/tests/releaseMetadata.test.mjs`
- Modify: `apps/desktop/tests/verifyReleaseArtifact.test.mjs`

- [ ] Add failing tests for `electronBuilderInvocation` with an options object:

  ```js
  electronBuilderInvocation(process.execPath, cliPath, plan, {
    channel: "nightly",
    buildVersion: "2026.258.42.1",
    macBundleVersion: "1.1.42",
  });
  ```

  Assert that stable invocation remains unchanged, while nightly invocation passes explicit electron-builder configuration for `publish.channel=nightly`, `generateUpdatesFilesForAllChannels=false`, `buildVersion`, and macOS `bundleVersion` only on macOS.

- [ ] Add failing tests proving:

  - `createReleaseArtifactExpectation(..., "nightly")` expects `nightly.yml` or `nightly-mac.yml`;
  - `runChecksumCli({ channel: "nightly" })` includes nightly metadata and excludes latest metadata;
  - `verifyUpdateMetadata({ channel: "nightly" })` accepts only the platform's nightly metadata filename;
  - an unknown channel, a mixed latest/nightly directory, or channel/path disagreement fails.

- [ ] Run the focused tests and confirm failure:

  ```powershell
  Set-Location apps/desktop
  node --test tests/packageRelease.test.mjs tests/releaseMetadata.test.mjs tests/verifyReleaseArtifact.test.mjs
  ```

- [ ] Implement an explicit `channel` parameter restricted to `latest | nightly`. Read `ORKWORKS_RELEASE_CHANNEL`, `ORKWORKS_BUILD_VERSION`, and `ORKWORKS_MAC_BUNDLE_VERSION` at the CLI boundary; do not let arbitrary environment values become filenames or builder arguments.

- [ ] Rerun the focused tests and the complete desktop Node suite:

  ```powershell
  node --test tests/packageRelease.test.mjs tests/releaseMetadata.test.mjs tests/verifyReleaseArtifact.test.mjs
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  ```

  Expected: all tests pass.

- [ ] Commit:

  ```powershell
  git add apps/desktop/scripts apps/desktop/tests
  git commit -m "feat: isolate stable and nightly package channels"
  ```

## Task 4: Pin and test electron-updater's custom-channel behavior

**Files:**

- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/pnpm-lock.yaml`
- Create: `apps/desktop/tests/nightlyUpdateChannel.test.mjs`

- [ ] Add the exact development dependency:

  ```powershell
  Set-Location apps/desktop
  pnpm add --save-dev --save-exact electron-updater@6.8.9
  ```

- [ ] Write a failing contract test against the installed `GitHubProvider` implementation. Inject its HTTP executor and return an Atom feed containing, in order, a newer stable tag, an older nightly tag, and the newest nightly tag. Return `nightly.yml`/`nightly-mac.yml` only for the custom channel.

  Assert that a provider configured with `allowPrerelease: true` and explicit channel `nightly` selects the newest `*-nightly.*` entry and requests only nightly metadata. Also assert that a stable-only feed yields no nightly candidate rather than selecting stable.

- [ ] Run:

  ```powershell
  node --test tests/nightlyUpdateChannel.test.mjs
  ```

  Expected first red state: the fixture/import wiring is incomplete. Complete only the transport fixture needed by the real provider; do not reimplement its selection algorithm in OrkWorks.

- [ ] Rerun the test twice, once normally and once with the feed order reversed, then run the full Node suite.

- [ ] Commit:

  ```powershell
  git add apps/desktop/package.json apps/desktop/pnpm-lock.yaml apps/desktop/tests/nightlyUpdateChannel.test.mjs
  git commit -m "test: pin nightly updater channel discovery"
  ```

## Task 5: Make Main CI callable at an immutable SHA

**Files:**

- Modify: `.github/workflows/main-ci.yml`
- Modify: `apps/desktop/tests/packageRelease.test.mjs`

- [ ] Add a failing workflow-contract test that parses `.github/workflows/main-ci.yml` and asserts:

  - existing `push`, `schedule`, and `workflow_dispatch` triggers remain;
  - `workflow_call.inputs.source_sha` exists as an optional string;
  - every checkout uses `${{ inputs.source_sha || github.sha }}`;
  - no job resolves `main` independently.

- [ ] Run the focused test and confirm failure.

- [ ] Add the `workflow_call` trigger and input, then change both desktop and Rust checkouts to:

  ```yaml
  - uses: actions/checkout@v4
    with:
      ref: ${{ inputs.source_sha || github.sha }}
  ```

- [ ] Run:

  ```powershell
  Set-Location apps/desktop
  node --test tests/packageRelease.test.mjs
  ```

  Expected: pass.

- [ ] Commit:

  ```powershell
  git add .github/workflows/main-ci.yml apps/desktop/tests/packageRelease.test.mjs
  git commit -m "ci: expose main validation for frozen release sources"
  ```

## Task 6: Add authenticated GitHub preparation and publication CLIs

**Files:**

- Create: `apps/desktop/scripts/prepareDailyRelease.mjs`
- Create: `apps/desktop/scripts/publishDailyRelease.mjs`
- Modify: `apps/desktop/scripts/dailyRelease.mjs`
- Modify: `apps/desktop/tests/dailyRelease.test.mjs`

- [ ] Add failing tests with an injected `fetch` and injected file/output adapters for:

  ```js
  export async function prepareDailyRelease({ event, repository, token, fetchImpl, outputs }) {}
  export async function publishDailyRelease({ identity, repository, token, assetDir, fetchImpl }) {}
  ```

  Test exhaustive Link-header pagination, exact annotated/lightweight tag dereferencing, missing token, 401/403, rate limits, invalid JSON, wrong existing tag target, ambiguous tag creation with at most three same-tag retries, duplicate successful source skip, incomplete prior release retry, lower candidate rejection, real draft creation with `prerelease: true`, omitted `target_commitish`, complete upload, full draft validation, and publish-only-after-validation.

- [ ] Confirm the focused test fails, then implement the GitHub REST adapter using `fetch`. Keep endpoint construction and response-schema checks centralized in `dailyRelease.mjs`. Never log authorization headers or token-bearing request objects.

- [ ] Stage nightly versions atomically in the runner checkout:

  - update `apps/desktop/package.json` without changing dependencies;
  - update `crates/orkworksd/Cargo.toml` package version;
  - update only the `name = "orkworksd"` package entry in `crates/orkworksd/Cargo.lock`;
  - assert the staged desktop and Rust versions match before writing GitHub outputs.

- [ ] Run:

  ```powershell
  Set-Location apps/desktop
  node --test tests/dailyRelease.test.mjs
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  ```

- [ ] Commit:

  ```powershell
  git add apps/desktop/scripts apps/desktop/tests
  git commit -m "feat: prepare and publish verified nightly releases"
  ```

## Task 7: Unify stable and nightly release orchestration

**Files:**

- Modify: `.github/workflows/release.yml`
- Modify: `apps/desktop/tests/packageRelease.test.mjs`

- [ ] Replace/add workflow-contract tests first. Assert the YAML contains all of these behaviors:

  - tag, `03:23` UTC schedule, and manual triggers;
  - an unprivileged preflight that rejects non-main schedule/dispatch refs and noncanonical stable tags;
  - frozen `source_sha` output used by every checkout, the Main CI reusable call, tag creation, packaging, and publication;
  - nightly concurrency with `cancel-in-progress: false`;
  - `environment: release` on privileged preparation, both signing builds, and publication;
  - `RELEASE_GITHUB_TOKEN` appears only on nightly tag/release API steps;
  - macOS arm64 and Windows x64 matrix remains intact;
  - the nightly package step passes channel and native versions;
  - upload/assembly allows the selected channel metadata and rejects the other channel;
  - stable publication remains a draft; nightly publication uses the verified draft lifecycle;
  - the reusable Main CI job must pass before either build begins.

- [ ] Run the workflow-contract test and confirm failure.

- [ ] Implement `.github/workflows/release.yml` with this job graph:

  ```text
  preflight
     ├─> validate (uses main-ci.yml at source_sha)
     └─> prepare-nightly (release environment; scheduled/manual only)
                └──────────────┐
  validate ─────────────────> build (release environment; signed matrix)
                                  ├─> publish-stable (release environment)
                                  └─> publish-nightly (release environment; serialized recheck)
  ```

  Use a top-level nightly concurrency key that serializes the whole scheduled/manual run while giving stable tag runs a unique key. Keep `cancel-in-progress: false`.

- [ ] In preflight, set one `source_sha=${GITHUB_SHA}`. For nightly, require `GITHUB_REF=refs/heads/main`; for stable, call `parseStableTag` before any environment-bearing job.

- [ ] In `prepare-nightly`, call `prepareDailyRelease.mjs` with `RELEASE_GITHUB_TOKEN`. It must either emit `should_build=false` for one completely valid already-published source or emit the immutable identity after creating/verifying its exact tag.

- [ ] In the build matrix, checkout `source_sha`, stage nightly versions only for nightly, run the existing signing and native verification steps unchanged, and upload the selected channel's metadata plus installers, archives, blockmaps, and checksum manifest.

- [ ] Keep the stable `softprops/action-gh-release@v2` draft path. On nightly, assemble to one directory, reject duplicate basenames, assert exact assets, and call `publishDailyRelease.mjs` for draft creation/upload/full validation/publication.

- [ ] Run:

  ```powershell
  Set-Location apps/desktop
  node --test tests/packageRelease.test.mjs tests/dailyRelease.test.mjs
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  ```

- [ ] Commit:

  ```powershell
  git add .github/workflows/release.yml apps/desktop/tests/packageRelease.test.mjs
  git commit -m "ci: publish signed verified daily prereleases"
  ```

## Task 8: Document operation and external prerequisites

**Files:**

- Modify: `docs/agents/release-signing.md`
- Modify: `docs/agents/project-context.md`
- Modify: `specs/release-pipeline.md`

- [ ] Update the operator runbook with:

  - schedule and manual dispatch behavior;
  - exact skip, retry, immutable-tag, and failed-draft behavior;
  - nightly/stable version and metadata identity;
  - `release` environment protection for the default branch and canonical stable tags;
  - `RELEASE_GITHUB_TOKEN` fine-grained permissions (Contents write and Workflows write), CI-only ownership, rotation, and fail-closed symptoms;
  - how to inspect signatures, tag target, source marker, metadata, checksums, and release assets;
  - the credential-backed manual validation command and expected evidence;
  - the fact that user-facing automatic updates remain unavailable until #511.

- [ ] Update CI routing in `project-context.md` and mark the source-wiring portion of the release-pipeline spec as implemented without claiming the manual validation is complete.

- [ ] Run:

  ```powershell
  bash scripts/doc-check.sh
  git diff --check
  ```

- [ ] Commit:

  ```powershell
  git add docs/agents/release-signing.md docs/agents/project-context.md specs/release-pipeline.md
  git commit -m "docs: add daily release operations runbook"
  ```

## Task 9: Verify the branch and request review

- [ ] Run the full local verification suite from the worktree:

  ```powershell
  Set-Location apps/desktop
  pnpm typecheck
  pnpm build
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  Set-Location ../..
  cargo test --manifest-path crates/orkworksd/Cargo.toml
  bash scripts/doc-check.sh
  git diff --check origin/main...HEAD
  git status --short --branch
  ```

  Expected: every test/build/check exits 0; only intentional branch commits are present.

- [ ] Invoke `/code-review low` as required for the code diff. Address each finding with evidence or record why it is intentional.

- [ ] Re-run the complete verification suite after review changes.

- [ ] Push `daily-release`, open a PR linked to issue #510, and describe the unresolved external validation gate explicitly. Do not use `Closes #510` until the live nightly succeeds.

  Save the concrete PR number returned by GitHub as `$dailyReleasePr`; use that value for every subsequent check, merge, and cleanup command.

- [ ] Use the repository PR-babysitting workflow until CI and actionable reviews are resolved. Do not merge with failing checks.

## Task 10: Provision, run, and verify one real nightly

This task requires repository-owner access and secret material that must not be entered into source, logs, or chat.

- [ ] Before merge, have the repository owner configure GitHub's `release` environment to permit only `main` and canonical stable tags, preserve the existing signing secrets/variables, and add `RELEASE_GITHUB_TOKEN` with the documented fine-grained permissions.

- [ ] After all required checks and `/code-review low` pass, squash-merge the source PR through the documented maintainer path.

- [ ] Dispatch the merged workflow from `main`:

  ```powershell
  gh workflow run release.yml --repo Rambolarsen/orkworks --ref main
  ```

- [ ] Monitor the run. Verify the release is published, prerelease=true, draft=false, its unique tag resolves to the run's frozen SHA, the source marker matches, the asset set is exact, both signatures validate, checksum and updater metadata cross-check, and mixed stable/nightly discovery selects the nightly only for the explicit nightly channel.

- [ ] Trigger one rerun/manual dispatch at the same `main` SHA and verify it exits successfully as a no-op without creating a second published nightly.

- [ ] Record the run URL and release URL on issue #510, check every acceptance criterion, and close it only after the real signed artifacts and deduplication behavior are proven.

- [ ] Clean up the merged worktree with:

  ```powershell
  Set-Location C:\Users\froma\source\repos\orkworks
  bash scripts/finish-pr.sh $dailyReleasePr
  ```
