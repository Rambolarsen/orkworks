# Issue #509 final whole-branch review fixes

## Status

The final signed-release packaging and verification findings are addressed on
`release-signing`. This report uses an issue-specific path so the pre-existing
working-tree edits to `task-3-report.md` and `task-4-report.md` remain untouched
and are excluded from this fix commit.

## Changes

- Added an explicit `verify:release:pre-checksum` mode that skips only the
  not-yet-generated `SHA256SUMS.txt` requirement.
- Added a final default `pnpm verify:release` gate after checksum generation and
  before artifact upload. The default verifier still requires the checksum
  manifest.
- Added a real temporary-release regression for the sequence: pre-checksum
  verification passes, the final gate fails before the manifest exists,
  checksum generation runs, and the final gate passes.
- Removed `xcrun stapler validate "$DMG_PATH"`. The staged app and apps read
  from the mounted DMG and extracted ZIP still receive codesign, Gatekeeper,
  and app-staple validation.
- Restricted update metadata `files[]` URLs to top-level current-version
  `OrkWorks-<version>-mac-{arm64,x64}.zip` or
  `OrkWorks-<version>-win-x64.exe` payloads, selected by metadata filename,
  with adjacent blockmaps still required.
- Added real-file rejection tests for nested, stale-version, and non-updater
  payload references.
- Updated the release runbook, release-pipeline spec, and approved design to
  describe the two verifier gates and app-bundle stapling accurately. Removed
  trailing whitespace from the design header.

## TDD evidence

The focused RED run reported six expected failures: missing pre-checksum script
and final workflow gate, three accepted invalid updater URLs, and pre-checksum
verification still requiring `SHA256SUMS.txt`. After the minimal implementation,
the focused release suite passed 51 tests with zero failures.

## Verification

- `node --experimental-strip-types --test tests/releaseMetadata.test.mjs tests/verifyReleaseArtifact.test.mjs tests/packageRelease.test.mjs` — PASS, 51 tests.
- `pnpm.cmd exec tsc --noEmit` — PASS.
- `node --test docs/.vitepress/site-facts.test.mjs scripts/docs-audit-scope.test.mjs` — PASS, 8 tests.
- `pnpm.cmd docs:build` — PASS; VitePress emitted the existing circular-chunk and large-chunk warnings.
- `bash scripts/doc-check.sh` — PASS.
- `git diff --check` — PASS.

## External verification boundary

No credential-backed macOS notarization or Windows Authenticode release was run
locally. The protected native GitHub Actions release run remains the delivery
evidence for certificate trust, notarization, app stapling, mounted-DMG and
extracted-ZIP app validation, and signed Windows installer validation.

## Commit

Included in `fix: close release signing review findings`; the commit hash is
reported at handoff because embedding it here would change the hash.
