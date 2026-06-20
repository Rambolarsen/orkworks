---
name: cutting-release
description: Use when the user asks to cut a release, ship a version, or publish the app. Guides through version bump, tag creation, CI monitoring, and release verification.
---

# Cutting a Release

## Overview

Use this skill to publish a new release of the OrkWorks desktop app. The release pipeline is tag-driven: pushing a `vX.Y.Z` tag triggers a GitHub Actions workflow that builds, packages, and publishes a draft GitHub Release with artifacts for macOS, Windows, and Linux.

## When to use

- User says "cut a release", "ship it", "publish v0.2.0", "create a release"
- A milestone is complete and the user wants to distribute artifacts

## Prerequisites

Before cutting a release, verify:
- [ ] Working tree is clean (`git status` — no uncommitted changes)
- [ ] On the branch that should be released (typically `main`)
- [ ] All tests pass (`cargo test --manifest-path crates/orkworksd/Cargo.toml` and `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`)
- [ ] Latest changes are pushed to GitHub (`git push`)

## Process

### 1. Bump versions

Edit both files to the new version:

**`apps/desktop/package.json`** — update the `version` field:
```json
"version": "0.2.0"
```

**`crates/orkworksd/Cargo.toml`** — update the `version` field:
```toml
version = "0.2.0"
```

### 2. Commit and push

```bash
git add apps/desktop/package.json crates/orkworksd/Cargo.toml
git commit -m "chore: bump version to 0.2.0"
git push
```

### 3. Create and push the tag

```bash
TAG="v0.2.0"
git tag "$TAG"
git push origin "$TAG"
```

### 4. Monitor the release workflow

The tag push triggers `.github/workflows/release.yml`. Watch it:
- Go to https://github.com/Rambolarsen/orkworks/actions
- Find the "Release" workflow run for the tag
- Wait for all three matrix jobs (mac, win, linux) to complete
- If any job fails, fix the issue, delete the tag (`git push --delete origin v0.2.0; git tag -d v0.2.0`), and retry after fixing

### 5. Verify the draft release

- Go to https://github.com/Rambolarsen/orkworks/releases
- Find the draft release (named after the tag)
- Confirm all expected artifacts are attached:
  - macOS: `OrkWorks-0.2.0-mac-arm64.dmg`, `OrkWorks-0.2.0-mac-x64.dmg`
  - Windows: `OrkWorks-0.2.0-win-x64.exe`
  - Linux: `OrkWorks-0.2.0-linux-x64.AppImage`, `OrkWorks-0.2.0-linux-x64.deb`
- Add release notes (describe what changed since the last release)

### 6. Publish

Click "Publish release" on the GitHub Releases page.

## Aborting a bad release

If the CI fails or you need to retract before publishing:
```bash
git push --delete origin v0.2.0   # delete remote tag
git tag -d v0.2.0                 # delete local tag
```
Fix the issue, then re-tag and push.

## Platform notes

- **macOS artifacts are unsigned.** Alpha testers must right-click the DMG and select "Open" to bypass Gatekeeper.
- **Windows artifacts are unsigned.** SmartScreen will show a warning; click "More info" → "Run anyway".
- **Linux AppImage may require fuse.** On systems without it, use `--appimage-extract-and-run`.
