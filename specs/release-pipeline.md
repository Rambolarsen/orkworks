# Release Pipeline — Daily Builds and User-Controlled Updates

## Approved scope extension — 2026-09-10

This section defines the next release increment and supersedes conflicting
alpha-only requirements below. The remainder records the existing baseline;
daily builds, signing, and in-app updating are specified here, not yet implemented.

### Purpose and distribution channels

Everyday OrkWorks sessions should run an installed application whose frontend
and bundled sidecar are independent of agent edits, branch switches, and builds
in a development checkout. This removes a source of interference; it does not
establish the cause of the reported blank windows or claim to fix all such bugs.

Keep the existing manually tagged stable release path. Add a nightly channel
for Apple Silicon macOS and Windows x64, using GitHub Releases as the artifact
and update-metadata host. Linux and Intel macOS remain local-only packaging
targets. A stable installation checks stable releases; a nightly installation
checks nightlies. Channel switching and automatic downgrades are out of scope.
The first migration from an unsigned development/alpha build is a manual install.

### Daily workflow and version identity

- Schedule a nightly run at 03:23 UTC, with manual dispatch for the same flow.
  GitHub schedule timing is best effort, not a delivery-time guarantee.
- Resolve one immutable `main` commit at the start and use it for every check,
  platform build, and release record. Manual dispatch must also resolve `main`,
  never publish arbitrary branch content.
- Skip when that commit already has a successfully published nightly. A failed
  attempt must remain retryable; publication, not tag existence, defines success.
- Require the complete desktop and Rust validation used by main CI for that
  exact commit, plus artifact verification and Windows installer smoke testing.
  A green check for another commit or a docs-only no-op is insufficient.
- Serialize nightly publication and recheck the last published source commit
  before publishing, so concurrent schedule/dispatch runs cannot race.
- Retain `package.json` as the stable base version. During CI only, derive a
  SemVer nightly such as `0.1.0-nightly.20260910.123456789.1` from that base,
  UTC date, GitHub run ID, and run attempt. Stage matching desktop and Rust
  versions without committing version bumps back to `main`. Validate native
  platform version fields as well as updater ordering; use separate numeric
  platform build identifiers where the packager requires them.
- Publish a unique, immutable tag and prerelease for each successful nightly,
  with source SHA and build identity in its notes. Do not overwrite old assets,
  move a rolling tag, mark a nightly as the stable latest release, or silently
  prune previously published releases in this increment.
- Stage both platforms and all channel-specific update metadata in a draft;
  publish only after all required checks and assets succeed. A failed platform
  must leave the preceding published update usable. Configure GitHub prerelease
  discovery explicitly and test it against stable and nightly releases together.

### Signing and update artifacts

Use `electron-updater` with the existing `electron-builder` packaging and a
fixed GitHub provider for `Rambolarsen/orkworks`. No separate update server is
needed. Keep provider configuration and network/download operations in Electron
main; the renderer cannot supply a feed URL, executable path, or release token.

macOS builds require a Developer ID signature, notarization, and stapling of
the distributed app/installer as applicable. Sign the bundled Rust executable
and any other nested executable code. Publish both DMG (manual install) and ZIP
(updater payload), with the generated channel metadata and checksums. Windows
NSIS releases require Authenticode signing and verification against the expected
publisher. Keep checksum and publisher/signature verification enabled.

The owner provisions Apple Developer credentials and Windows signing credentials
through protected GitHub Actions secrets/environments. The implementation must
document exact secret names and setup without exposing values. Signing secrets
are available only to trusted release jobs, never pull-request builds. Missing,
expired, or invalid credentials fail the release before publication; no unsigned
fallback is permitted for either update channel. Do not embed a GitHub token in
the app. This design uses public release downloads; private distribution would
require a separate authentication design.

### In-app behavior and lifecycle

Add an Updates section in Settings showing installed version, channel, check
status, available version, release notes, and download progress. Provide a
native menu `Check for updates` entry reaching the same flow. Packaged builds
may check once after startup without interrupting work; explicit checks remain
available. Development builds show updates as unavailable and never install.

The flow is `Check for updates` → `Download` → `Restart and install`. Checking
does not download; downloading does not restart or stop sessions. Disable
automatic download and installation on ordinary quit. A downloaded update waits
for an explicit install action; on a later launch, revalidate it against current
release metadata before allowing installation. Duplicate clicks coalesce into
one operation. Stale events cannot overwrite a newer check/download state.

Before installing, main presents a native confirmation explaining that restarting
OrkWorks stops its sidecar and interrupts live terminal sessions. Determine live
session state from the current backend, not a stale renderer snapshot; if state
is unavailable, state that sessions may be interrupted. Cancel leaves the app
and sessions running. Confirm authorizes this one restart only. Reuse normal
app/sidecar shutdown, await bounded shutdown completion before applying the
update, and surface a shutdown failure without blindly replacing a running
sidecar. Do not promise live PTY continuity or automatically resume harnesses.
The new app restores workspace metadata through its existing startup path.

Use a small main-owned updater service and narrow preload commands/events for
check, download, installation request, and read-only status. Preserve the
`electron/` versus `src/` boundary with independently defined IPC contract types.
Offline checks, GitHub rate limits, no eligible releases, interrupted downloads,
invalid metadata, signature/checksum failures, and install failures produce
readable status and a retry path while the existing app remains usable. Never
silently downgrade or replace user settings/metadata. Diagnostics must omit
credentials and session content.

### Delivery and verification

Deliver this increment as separate tracked units:

1. [Signing and notarization (#509)](https://github.com/Rambolarsen/orkworks/issues/509)
   for the existing release artifacts, plus ZIP and
   updater metadata preparation. Verify signed macOS and Windows artifacts in CI.
2. [Daily workflow (#510)](https://github.com/Rambolarsen/orkworks/issues/510),
   CI-only versions, channel publication, and unchanged-commit
   skipping. Depends on signing; exercise success, partial failure, rerun,
   concurrency, and stable/nightly isolation with fixtures and a manual run.
3. [Main-owned updater and Settings/menu flow (#511)](https://github.com/Rambolarsen/orkworks/issues/511).
   Depends on usable signed channel
   artifacts. Test event ordering, disabled development mode, retry/error paths,
   confirmation/cancellation, unavailable session state, and shutdown sequencing.
   Verify an installed older signed build can download and install a newer one
   on macOS and Windows, preserving settings and requiring explicit restart.

Each unit updates operator/user documentation and records its actual validation.
No release is declared update-ready based solely on source tests or successful
packaging. Credentials and installed-app platform verification are external
delivery prerequisites, not assumed available.

### References

- [electron-builder auto-update requirements](https://www.electron.build/v26/docs/features/auto-update/)
- [Electron code signing](https://www.electronjs.org/docs/latest/tutorial/code-signing)

## Existing alpha baseline (historical scope)

Electron + Rust sidecar cross-platform release pipeline for internal/alpha testing.

## Motivation

OrkWorks needs installable artifacts for early testers on macOS and Windows. There is currently no packaging tooling, no CI/CD, and no release process. This spec adds the minimum infrastructure to build, package, and publish alpha releases via GitHub Releases.

## Scope & Non-Goals

### In scope

- GitHub Release artifacts for macOS (DMG) and Windows (NSIS)
- GitHub Actions workflow triggered by git tag push
- Rust sidecar bundled as `extraResources` for all platforms
- Version source of truth: `apps/desktop/package.json` `version` field
- `cutting-release` repo skill for agent guidance
- Manual version bump, manual tag push
- Windows x64 NSIS install/uninstall smoke validation on the `windows-latest`
  release runner after packaging.

### Non-goals

- Code signing or notarization (alpha artifacts are unsigned; users bypass platform warnings)
- Application-binary auto-update (no update server, no `electron-updater`).
  Independently updated reference knowledge and its packaged fallback are
  governed by [Taskmaster knowledge](taskmaster-knowledge.md).
- Automated version bumping or changelog generation
- Broad Windows-version, architecture, or locale compatibility testing.
- GUI automation or first-launch runtime testing.
- Production distribution, store publishing, or installer branding

## Architecture

```
git tag vX.Y.Z ──> GitHub Actions (release.yml) ──> 3 parallel matrix jobs
                                                         │
                  ┌───────────────────────┬──────────────┬──────────────────────┐
                  ▼                       ▼              ▼
             macos-latest           windows-latest  publish job
             (mac arm64)               (win x64)      (draft)
                  │                       │
      pnpm install && build    pnpm install && build
                  │                       │
         pnpm package:release   pnpm package:release
                  │                       │
                  ▼                       ▼
       OrkWorks-*-mac-arm64.dmg OrkWorks-*-win-x64.exe
```

## File Changes

### New: `apps/desktop/electron-builder.yml`

```yaml
appId: ai.orkworks.desktop
productName: OrkWorks
directories:
  output: release
  buildResources: build
files:
  - dist/**/*
  - dist-electron/**/*
  - package.json
mac:
  category: public.app-category.developer-tools
  target:
    - dmg
  artifactName: OrkWorks-${version}-mac-${arch}.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd
      to: orkworksd
win:
  target:
    - nsis
  artifactName: OrkWorks-${version}-win-${arch}.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd.exe
      to: orkworksd.exe
linux:
  category: Development
  target:
    - AppImage
    - deb
  artifactName: OrkWorks-${version}-linux-${arch}.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd
      to: orkworksd
nsis:
  oneClick: false
  allowToChangeInstallationDirectory: true
```

Linux and Intel macOS packaging configuration remains available for local development, but the tag-driven GitHub Release workflow publishes only Apple Silicon macOS and Windows artifacts.

### New: `.github/workflows/release.yml`

Triggered on tag push matching `v*`. Uses a matrix strategy for OS/arch jobs. Each build job:

1. Checks out the repo
2. Installs Node 22
3. **Guards tag/version drift** with Node, not `jq`, so the check works on Windows and Unix runners:
   `TAG="$GITHUB_REF_NAME"; PKG_VERSION="$(node -e "process.stdout.write('v' + require('./apps/desktop/package.json').version)")"`
4. Installs Rust via `dtolnay/rust-toolchain` and primes the cargo cache via `Swatinem/rust-cache@v2`
5. Installs pnpm
6. Installs deps with frozen lockfile and builds the frontend: `cd apps/desktop && pnpm install --frozen-lockfile && pnpm build`
7. Runs `pnpm package:release`, which:
   - maps the host platform/arch to the matching Rust target triple
   - builds the sidecar for that exact target
   - stages the built binary into `crates/orkworksd/target/release/`
   - runs `electron-builder` with the matching CLI arch flag
8. Verifies `process.arch` matches the matrix architecture before packaging.
9. Runs `pnpm verify:release`, which fails unless the installer, unpacked app,
   Rust sidecar, and each packaged hook-script file all exist.
10. On the Windows runner, runs `pnpm smoke:windows-installer`, which silently
    installs the generated NSIS artifact into a unique temporary directory,
    verifies the installed executable, Rust sidecar, and hook scripts, then
    uninstalls it and requires the directory to disappear within a bounded
    timeout. Before launching NSIS, the smoke test refuses to run if an
    OrkWorks uninstall entry is registered in either the per-user or
    per-machine Windows uninstall registry data. If post-install verification
    fails, it does not invoke the uninstaller, leaving the installation
    directory available for diagnosis.
11. Uploads top-level `OrkWorks-*` artifacts via `actions/upload-artifact`.

After all matrix jobs complete, a `publish` job downloads all artifacts and creates/updates a draft GitHub Release via `softprops/action-gh-release@v2`. Requires `permissions: { contents: write }` at the workflow level.

### Modified: `apps/desktop/package.json`

Additions to `scripts`:
```json
"build:rust:release": "cargo build --release --manifest-path ../../crates/orkworksd/Cargo.toml",
"package:release": "node scripts/package-release.mjs",
"verify:release": "node scripts/verifyReleaseArtifact.mjs",
"dist": "tsc -p tsconfig.node.json && vite build && node scripts/package-release.mjs"
```

Addition to `devDependencies`:
```json
"electron-builder": "^26.1.1"
```

### New: `skills/cutting-release/SKILL.md`

Repo skill that guides agents through the release process:
- Pre-release checks (clean working tree, tests pass)
- Version bump in `apps/desktop/package.json` (and optionally `crates/orkworksd/Cargo.toml`)
- Commit and push
- Tag creation: `git tag vX.Y.Z && git push origin vX.Y.Z`
- CI monitoring (watch the Actions run)
- Verify the draft release and artifacts
- Publish the release

## Sidecar Bundling

The sidecar path resolution exists in `electron/main.ts:147-152` and **must be updated** to handle the Windows `.exe` extension:

```ts
function getSidecarPath(): string {
  if (app.isPackaged) {
    const binaryName = process.platform === "win32" ? "orkworksd.exe" : "orkworksd";
    return path.join(process.resourcesPath, binaryName);
  }
  return getDevSidecarPath(__dirname);
}
```

electron-builder's per-platform `extraResources` blocks (see config above) copy the right binary name into the app's `resources/` directory, matching `process.resourcesPath`. The packaging script builds and stages the sidecar for the current host arch before invoking electron-builder, so each CI job produces an app bundle whose sidecar matches the bundled Electron arch.

## Artifact Verification

`apps/desktop/scripts/verifyReleaseArtifact.mjs` validates the generated
installer and unpacked app before CI uploads anything. It checks the expected
platform-specific sidecar name and the `resources/scripts/` directory used by
installed harness integrations, including each expected reporter script. This
catches incomplete packages while the build job still has the unpacked
application available for inspection.

## Version Management

- **Single source of truth:** `apps/desktop/package.json` `version` field (electron-builder reads it)
- `crates/orkworksd/Cargo.toml` version is bumped in lockstep — the `cutting-release` skill performs both edits in a single commit; the CI tag/version guard (see workflow step 2) fails the build if the tag doesn't match `apps/desktop/package.json`
- Both files currently sit at `0.1.0`, so the first release tag is `v0.1.0`
- **Tag convention:** `vX.Y.Z` (e.g., `v0.1.0`)
- **Cadence:** manual, ad-hoc for alpha
- **No pre-release suffixes** for alpha (no `-alpha.1`, `-beta`, etc.) — `0.x.y` itself communicates pre-1.0 status

## Edge Cases & Known Limitations

| Condition | Behavior |
|-----------|----------|
| macOS unsigned DMG | Gatekeeper blocks first launch; user right-clicks → Open to bypass |
| Windows unsigned NSIS | SmartScreen shows warning; user clicks "More info" → "Run anyway" |
| Local mac packaging | `pnpm package:release` builds the host arch only. The tagged release currently publishes Apple Silicon macOS output from `macos-latest`; Intel macOS packaging remains a local-only path until a reliable Intel runner is selected |
| Sidecar binary missing | `electron-builder` fails with a clear error if `cargo build --release` hasn't run |
| CI runner missing Rust | `dtolnay/rust-toolchain` action installs it; no manual setup needed |
| Tag push without version bump | CI guard (workflow step 2) compares `$GITHUB_REF_NAME` to `apps/desktop/package.json` and fails the job, so a stale tag never produces artifacts |
| Parallel tag pushes | Each tag triggers a new workflow run; they don't conflict |
| Missing `apps/desktop/build/` icons dir | electron-builder falls back to default Electron icons — acceptable for alpha; branded icons deferred |
| Existing OrkWorks uninstall entry | The Windows smoke test refuses to launch NSIS when an OrkWorks installation is registered in either per-user or per-machine uninstall registry data |
| Post-install verification failure | The uninstaller is not invoked, and the installation directory remains available for diagnosis |
| Windows installer or uninstaller fails | The Windows build fails before artifact upload, with the failing executable or installed path in the log |

## Future Upgrades

When moving beyond alpha:
- Add code signing + notarization config (Apple Developer account, Windows code signing certificate)
- Add `@electron/osx-sign` and `@electron/notarize`
- Switch to `electron-updater` + a release server for auto-update
- Add a checksum file per artifact (SHA256)
- Add an ADR for the release strategy
