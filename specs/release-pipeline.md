# Release Pipeline — Alpha Distribution

Electron + Rust sidecar cross-platform release pipeline for internal/alpha testing.

## Motivation

OrkWorks needs installable artifacts for early testers on macOS, Windows, and Linux. There is currently no packaging tooling, no CI/CD, and no release process. This spec adds the minimum infrastructure to build, package, and publish alpha releases via GitHub Releases.

## Scope & Non-Goals

### In scope

- electron-builder configuration for macOS (DMG), Windows (NSIS), and Linux (AppImage, deb)
- GitHub Actions workflow triggered by git tag push
- Rust sidecar bundled as `extraResources` for all platforms
- Version source of truth: `apps/desktop/package.json` `version` field
- `cutting-release` repo skill for agent guidance
- Manual version bump, manual tag push

### Non-goals

- Code signing or notarization (alpha artifacts are unsigned; users bypass platform warnings)
- Auto-update (no update server, no `electron-updater`)
- Automated version bumping or changelog generation
- Cross-platform testing in CI (artifacts are built but not tested)
- Production distribution, store publishing, or installer branding

## Architecture

```
git tag vX.Y.Z ──> GitHub Actions (release.yml) ──> 4 parallel matrix jobs
                                                         │
                  ┌───────────────────────┬──────────────┼──────────────┬──────────────────────┐
                  ▼                       ▼              ▼              ▼                      ▼
             macos-13                macos-latest   windows-latest  ubuntu-latest       publish job
             (mac x64)               (mac arm64)       (win x64)      (linux x64)         (draft)
                  │                       │              │              │
      pnpm install && build    pnpm install && build    │              │
                  │                       │              │              │
         pnpm package:release   pnpm package:release  pnpm package:release  pnpm package:release
                  │                       │              │              │
                  ▼                       ▼              ▼              ▼
       OrkWorks-*-mac-x64.dmg  OrkWorks-*-mac-arm64.dmg OrkWorks-*-win-x64.exe
                                                               OrkWorks-*-linux-x64.AppImage
                                                               OrkWorks-*-linux-x64.deb
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
8. Uploads platform artifacts via `actions/upload-artifact`

After all matrix jobs complete, a `publish` job downloads all artifacts and creates/updates a draft GitHub Release via `softprops/action-gh-release@v2`. Requires `permissions: { contents: write }` at the workflow level.

### Modified: `apps/desktop/package.json`

Additions to `scripts`:
```json
"build:rust:release": "cargo build --release --manifest-path ../../crates/orkworksd/Cargo.toml",
"package:release": "node scripts/package-release.mjs",
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
| Linux AppImage | Requires `fuse` (pre-installed on ubuntu-latest runner); user may need `--appimage-extract-and-run` on systems without fuse |
| Linux arm64 | Not built — `ubuntu-latest` is x86_64. Linux arm64 testers are unsupported for alpha |
| Local mac packaging | `pnpm package:release` builds the host arch only. Dual-arch mac output comes from two CI jobs (`macos-13` x64 and `macos-latest` arm64), not from one local host cross-compiling both sidecars |
| Sidecar binary missing | `electron-builder` fails with a clear error if `cargo build --release` hasn't run |
| CI runner missing Rust | `dtolnay/rust-toolchain` action installs it; no manual setup needed |
| Tag push without version bump | CI guard (workflow step 2) compares `$GITHUB_REF_NAME` to `apps/desktop/package.json` and fails the job, so a stale tag never produces artifacts |
| Parallel tag pushes | Each tag triggers a new workflow run; they don't conflict |
| Missing `apps/desktop/build/` icons dir | electron-builder falls back to default Electron icons — acceptable for alpha; branded icons deferred |

## Future Upgrades

When moving beyond alpha:
- Add code signing + notarization config (Apple Developer account, Windows code signing certificate)
- Add `@electron/osx-sign` and `@electron/notarize`
- Switch to `electron-updater` + a release server for auto-update
- Add a checksum file per artifact (SHA256)
- Add an ADR for the release strategy
