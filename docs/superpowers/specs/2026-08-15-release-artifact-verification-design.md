# Release Artifact Verification Design

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-08-15

## Context

The tag-driven Electron release pipeline can build a macOS DMG and a Windows
NSIS installer, but a successful packaging job does not currently prove that
the installer exists or that the packaged app contains the Rust sidecar and
hook scripts required at runtime. A local macOS arm64 build currently produces
those files, so the missing protection is CI-level artifact verification and a
clear handoff of installable files to the draft GitHub Release.

## Decision

Add a small cross-platform Node verifier in `apps/desktop/scripts/` with a
pure expectation function and a CLI entry point. The verifier will check the
current platform's non-empty installer, unpacked application directory, Rust
sidecar, and hook-script resource directory. The existing release workflow
will invoke it after `pnpm package:release` and upload only top-level generated
release files, preventing unpacked directories from becoming ambiguous release
inputs.

The release remains an unsigned draft release for macOS arm64 and Windows x64.
This change does not add signing, auto-update, Linux publishing, or automatic
public release publication.

## Verification contract

The CLI is invoked as `pnpm verify:release` from `apps/desktop/`. It derives
the package version, host platform, host architecture, and `release/` output
directory. It fails with a platform-specific error if any required path is
missing or an installer is empty.

Expected packaged paths are:

- macOS: `OrkWorks-${version}-mac-${arch}.dmg`,
  `mac-${arch}/OrkWorks.app/Contents/Resources/orkworksd`, and
  `mac-${arch}/OrkWorks.app/Contents/Resources/scripts/`.
- Windows: `OrkWorks-${version}-win-${arch}.exe`,
  `win-unpacked/resources/orkworksd.exe`, and
  `win-unpacked/resources/scripts/`.

The pure expectation function is unit-tested for both supported release
targets. The CLI's filesystem checks are exercised by the real packaging
command in CI and locally.

## Consequences

Release jobs fail before artifact upload when a packaging regression omits the
installer, sidecar, or hook resources. Release uploads become limited to
top-level generated artifacts, which keeps the draft release focused on files
testers can download. A successful CI run still does not provide code signing
or prove first-launch behavior on every operating-system version.
