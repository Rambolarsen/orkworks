# Release Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an electron-builder + GitHub Actions release pipeline that produces unsigned DMG / NSIS / AppImage+deb artifacts for macOS, Windows, and Linux when a `v*` tag is pushed.

**Architecture:** electron-builder packages the Vite-built frontend plus the Rust sidecar (`extraResources`). A GitHub Actions workflow triggers on `v*` tag push, builds the sidecar and frontend per platform (matrix), packages, uploads artifacts, and publishes a draft GitHub Release.

**Tech Stack:** electron-builder 26, GitHub Actions, pnpm, Vite, Rust/Cargo (release profile), `softprops/action-gh-release@v2`

**Spec:** `specs/release-pipeline.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `apps/desktop/electron-builder.yml` | Packaging config: platforms, sidecar `extraResources`, artifact naming |
| Create | `.github/workflows/release.yml` | CI: tag-triggered matrix build, artifact upload, draft release publish |
| Modify | `apps/desktop/package.json` | Add `build:rust:release` + `dist` scripts, add `electron-builder` devDep |
| Modify | `apps/desktop/electron/main.ts:147-152` | Windows `.exe` extension in `getSidecarPath()` |
| Create | `skills/cutting-release/SKILL.md` | Agent skill: version bump, tag, push, verify release |

---

### Task 1: Add electron-builder devDependency and release scripts

**Files:**
- Modify: `apps/desktop/package.json`

- [ ] **Step 1: Read current package.json to confirm starting state**

- [ ] **Step 2: Add `build:rust:release` script, `dist` script, and `electron-builder` devDependency**

Add to `scripts` block:

```json
"build:rust:release": "cargo build --release --manifest-path ../../crates/orkworksd/Cargo.toml",
"dist": "npm run build:rust:release && npm run build && electron-builder"
```

Add to `devDependencies` block:

```json
"electron-builder": "^26.1.1"
```

- [ ] **Step 3: Install the new devDependency**

```bash
cd apps/desktop && pnpm install
```

Expected: `electron-builder` appears in `node_modules/.bin/electron-builder`.

- [ ] **Step 4: Verify the script resolution works (dry-run)**

```bash
cd apps/desktop && npx electron-builder --version
```

Expected: prints electron-builder version (e.g., `26.x.x`).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/package.json apps/desktop/pnpm-lock.yaml
git commit -m "chore: add electron-builder devDependency and release scripts"
```

---

### Task 2: Create electron-builder.yml

**Files:**
- Create: `apps/desktop/electron-builder.yml`

- [ ] **Step 1: Write the config file**

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
    - target: dmg
      arch: [x64, arm64]
  artifactName: OrkWorks-${version}-mac-${arch}.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd
      to: orkworksd
win:
  target:
    - nsis
  artifactName: OrkWorks-${version}-win.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd.exe
      to: orkworksd.exe
linux:
  category: Development
  target:
    - AppImage
    - deb
  artifactName: OrkWorks-${version}-linux.${ext}
  extraResources:
    - from: ../../crates/orkworksd/target/release/orkworksd
      to: orkworksd
nsis:
  oneClick: false
  allowToChangeInstallationDirectory: true
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/electron-builder.yml
git commit -m "feat: add electron-builder config for macOS, Windows, and Linux"
```

---

### Task 3: Update getSidecarPath for Windows .exe extension

**Files:**
- Modify: `apps/desktop/electron/main.ts:147-152`

- [ ] **Step 1: Replace the getSidecarPath function**

Old:
```ts
function getSidecarPath(): string {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, "orkworksd");
  }
  return getDevSidecarPath(__dirname);
}
```

New:
```ts
function getSidecarPath(): string {
  if (app.isPackaged) {
    const binaryName = process.platform === "win32" ? "orkworksd.exe" : "orkworksd";
    return path.join(process.resourcesPath, binaryName);
  }
  return getDevSidecarPath(__dirname);
}
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/electron/main.ts
git commit -m "fix: add .exe extension for Windows packaged sidecar path"
```

---

### Task 4: Create GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the workflow file**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: mac
            artifact: mac
          - os: windows-latest
            target: win
            artifact: win
          - os: ubuntu-latest
            target: linux
            artifact: linux
      fail-fast: false
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Guard tag/version drift
        run: |
          TAG="$GITHUB_REF_NAME"
          PKG_VERSION="v$(jq -r .version apps/desktop/package.json)"
          if [ "$TAG" != "$PKG_VERSION" ]; then
            echo "Tag $TAG does not match package.json version $PKG_VERSION"
            exit 1
          fi

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: crates/orkworksd

      - name: Build Rust sidecar (release)
        run: cargo build --release --manifest-path crates/orkworksd/Cargo.toml

      - uses: pnpm/action-setup@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: apps/desktop/pnpm-lock.yaml

      - name: Install frontend deps
        run: cd apps/desktop && pnpm install --frozen-lockfile

      - name: Build frontend
        run: cd apps/desktop && pnpm build

      - name: Package (electron-builder)
        run: cd apps/desktop && npx electron-builder --${{ matrix.target }} --publish never
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: release-${{ matrix.artifact }}
          path: apps/desktop/release/*
          if-no-files-found: error

  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
          pattern: release-*
          merge-multiple: true

      - name: Publish draft GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          draft: true
          files: artifacts/*
          fail_on_unmatched_files: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add GitHub Actions release workflow triggered by v* tags"
```

---

### Task 5: Create cutting-release skill

**Files:**
- Create: `skills/cutting-release/SKILL.md`

- [ ] **Step 1: Write the skill file**

```markdown
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
  - Windows: `OrkWorks-0.2.0-win.exe`
  - Linux: `OrkWorks-0.2.0-linux.AppImage`, `OrkWorks-0.2.0-linux.deb`
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
```

- [ ] **Step 2: Commit**

```bash
git add skills/cutting-release/SKILL.md
git commit -m "feat: add cutting-release skill for release workflow guidance"
```

---

### Task 6: Local verification (optional, macOS only)

**Prerequisite:** Task 1-3 commits applied. A full local release build on macOS to verify the config works end-to-end before pushing to CI.

- [ ] **Step 1: Build the Rust sidecar in release mode**

```bash
cargo build --release --manifest-path crates/orkworksd/Cargo.toml
```

Expected: `crates/orkworksd/target/release/orkworksd` exists.

- [ ] **Step 2: Build the frontend**

```bash
cd apps/desktop && pnpm install && pnpm build
```

Expected: `apps/desktop/dist/` and `apps/desktop/dist-electron/` exist.

- [ ] **Step 3: Run electron-builder (macOS only)**

```bash
cd apps/desktop && npx electron-builder --mac --publish never
```

Expected: `apps/desktop/release/OrkWorks-X.X.X-mac-arm64.dmg` (or x64) is created.

- [ ] **Step 4: Inspect the DMG to confirm sidecar is bundled**

```bash
hdiutil attach apps/desktop/release/OrkWorks-*.dmg -mountpoint /tmp/orkworks-mount
ls -la /tmp/orkworks-mount/OrkWorks.app/Contents/Resources/orkworksd
hdiutil detach /tmp/orkworks-mount
```

Expected: the sidecar binary exists at `Contents/Resources/orkworksd` inside the `.app` bundle.

- [ ] **Step 5: Commit any fixes needed from local verification**

---

### Task 7: Open PR and request review

- [ ] **Step 1: Create the PR**

```bash
gh pr create --title "feat: release pipeline with electron-builder and GitHub Actions" --body "Adds electron-builder packaging config, GitHub Actions release workflow (tag-triggered, cross-platform matrix), Windows sidecar path fix, and cutting-release skill.

Spec: specs/release-pipeline.md"
```

- [ ] **Step 2: Request review per AGENTS.md**

Invoke `requesting-code-review` skill before merging.

- [ ] **Step 3: After review approval, merge and cut first release**

Squash-merge the PR, then follow the `cutting-release` skill to cut `v0.1.0`.
