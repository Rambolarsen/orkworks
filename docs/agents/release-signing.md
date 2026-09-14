# Signed release operations

This runbook covers the existing tag-driven release workflow in
[`.github/workflows/release.yml](../../.github/workflows/release.yml). It
documents the credentials and native checks required for a credential-backed
release; it does not claim that those external prerequisites are currently
available.

## Release contract

The protected `release` environment is used by the macOS and Windows build
jobs. A push of a protected `v*` tag starts the workflow. The desktop package
version must match the tag before packaging starts.

The source wiring produces and verifies:

- macOS Apple Silicon Developer ID output: DMG for manual installation and
  ZIP for the updater payload;
- Windows x64 Authenticode-signed NSIS output;
- electron-builder `latest-mac.yml`/`latest.yml` metadata and blockmaps;
- packaged app, bundled `orkworksd` sidecar, hook scripts, and knowledge
  resources; and
- `SHA256SUMS.txt` for the distributable files.

The DMG target has update-info generation disabled. It remains a manual
installer and is included in checksums, while `latest-mac.yml` and its blockmap
describe only the ZIP updater payload.

The platform jobs run packaging, pre-checksum artifact verification, native
signature checks, and the Windows installer smoke test before checksum
generation. They then run the full artifact verifier, which requires the
checksum manifest, before upload. The publish job is matrix-gated and creates a
draft GitHub Release. A successful source-only test or packaging run is not
evidence that a release is trusted: the real credential-backed run on the
native runners is still required.

## Provision external signing material

Before adding any value to GitHub, the release operator must have:

1. An active Apple Developer Program membership, the Apple Developer Team ID,
   a Developer ID Application certificate exported as a password-protected
   `.p12`, and an App Store Connect Team Key downloaded as `.p8`. The Team Key
   must have App Manager access, and its key ID and issuer ID must be available
   to the operator.
2. A trusted Authenticode certificate exported as a password-protected
   `.pfx` (or the supported `.p12` form).
3. A Windows publisher value equal to the certificate's exact X.509
   `SimpleName` (the certificate common name used by the workflow's native
   check). Do not use a guessed value or the complete distinguished subject.

Self-signed Windows certificates are local-testing-only. They do not satisfy
the trusted Authenticode or SmartScreen release gate and must not be used for
the protected release environment.

The current workflow supports file-backed signing through base64 `.p12`/`.pfx`
secrets. Managed Windows signing is not wired by this workflow; adopting a
managed-signing service requires separate workflow integration and native
release validation.

## GitHub Environment setup

Create an environment named exactly `release` and configure it before storing
credentials:

- require approval from the release maintainers as environment reviewers;
- restrict deployments to the protected `v*` tag pattern; and
- add a repository ruleset or equivalent tag protection so only authorized
  release actors can create or update matching `v*` tags.

The workflow exposes signing values only to the tag-driven platform jobs. Pull
request workflows do not receive them. Do not put certificates, passwords,
API keys, or decoded files in the repository, workflow source, artifacts, or
diagnostic output.

Create these exact environment secrets and variable:

| Name | Type | Value or purpose |
| --- | --- | --- |
| `MAC_CSC_LINK` | Secret | Base64-encoded Developer ID Application `.p12`; mapped to electron-builder `CSC_LINK` on macOS. |
| `MAC_CSC_KEY_PASSWORD` | Secret | Password for the macOS `.p12`. |
| `APPLE_API_KEY` | Secret | Base64-encoded App Store Connect `.p8` content. |
| `APPLE_API_KEY_ID` | Secret | App Store Connect API key ID. |
| `APPLE_API_ISSUER` | Secret | App Store Connect issuer ID. |
| `APPLE_TEAM_ID` | Secret | Apple Developer Team ID. |
| `WIN_CSC_LINK` | Secret | Base64-encoded trusted Authenticode `.pfx`/`.p12`. |
| `WIN_CSC_KEY_PASSWORD` | Secret | Password for the Windows certificate. |
| `WIN_EXPECTED_PUBLISHER` | Environment variable | Exact Windows certificate `SimpleName`; it is verifier input, not a secret. |

The workflow maps the macOS certificate names to `CSC_LINK` and
`CSC_KEY_PASSWORD` only in the macOS packaging step. Before that step, it maps
the `APPLE_API_KEY` secret to a differently named, non-logged environment
value, decodes the `.p8` under `RUNNER_TEMP`, restricts the file to mode 600,
and exports `APPLE_API_KEY` as the temporary path through `GITHUB_ENV`.
Packaging and an always-run fallback cleanup remove the temporary file. The
workflow maps the Windows certificate names to the corresponding
electron-builder `CSC_*` inputs only in the Windows step, then uses
`WIN_EXPECTED_PUBLISHER` for native and updater-metadata verification.

## Base64 export without printing values

Export each file directly to the clipboard, then paste it into the matching
GitHub Environment secret field. These commands do not write the encoded value
to the terminal or to a repository file:

macOS or another system with the `base64` and `pbcopy` commands:

```bash
base64 < DeveloperIDApplication.p12 | tr -d '\n' | pbcopy
```

Windows PowerShell:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes((Resolve-Path .\code-signing.pfx))) | Set-Clipboard
```

Repeat the same process for the App Store Connect `.p8`, macOS `.p12`, and
Windows `.pfx`/`.p12`, using the corresponding secret name. Do not use a
command that echoes the value, redirect it into a tracked file, include it in
a report, or print it in CI. Clear the clipboard after saving the secret.

## Rotation

Rotate credentials as one controlled release change:

1. Create and test the replacement certificate or API key while the current
   credential remains valid.
2. Update the relevant protected environment secret without changing the
   secret names. For a Windows certificate rotation, update
   `WIN_EXPECTED_PUBLISHER` in the same change to the replacement certificate's
   exact `SimpleName`.
3. Confirm the `release` environment reviewers and `v*` tag protection still
   apply, then run a new tagged release through every native gate.
4. Revoke or retire the old certificate/API key only after the replacement
   release has passed its native checks. Never record either credential in
   source, logs, screenshots, or reports.

## Verification evidence

The checked-in workflow is the authoritative command sequence. From
`apps/desktop`, the source-level and metadata gates are:

```bash
pnpm verify:release:pre-checksum
pnpm smoke:windows-installer       # Windows release runner only
pnpm checksum:release
pnpm verify:release
```

The workflow runs the pre-checksum command before native verification; that
mode skips only the not-yet-generated `SHA256SUMS.txt`. It runs the Windows
smoke test before checksum generation, then runs the default verifier after
checksum generation and before artifact upload. The default gate requires the
manifest. Its platform-native checks are the evidence that must be present in
a real signed run.

On macOS, for the staged app and the app extracted from the DMG and ZIP, the
workflow runs commands equivalent to:

```bash
codesign --verify --deep --strict --verbose=2 "$APP_PATH"
codesign --verify --deep --strict --verbose=2 "$APP_PATH/Contents/Resources/orkworksd"
spctl --assess --type execute --verbose=4 "$APP_PATH"
xcrun stapler validate "$APP_PATH"
```

It mounts the DMG read-only with `hdiutil attach`, extracts the ZIP with
`ditto -x -k`, checks the contained apps, and detaches the DMG during cleanup.
The expected evidence is a successful check for the staged app, nested
sidecar, mounted-DMG app, and extracted-ZIP app. electron-builder notarizes and
staples the app bundle; the workflow does not claim or validate a staple on the
DMG container.

On Windows, for the NSIS installer, unpacked app, and bundled sidecar, the
workflow runs:

```powershell
signtool verify /pa /all /v $Path
Get-AuthenticodeSignature -LiteralPath $Path
```

Each `Get-AuthenticodeSignature` result must have status `Valid`. The workflow
reads the signer's X.509 `SimpleName` and requires an exact match with
`WIN_EXPECTED_PUBLISHER`; it also requires `resources/app-update.yml` to have
a one-item `publisherName` array containing that same value. The signed NSIS
installer is then installed and the installed `OrkWorks.exe`,
`resources/orkworksd.exe`, and hook scripts are checked by the existing smoke
test.

These native commands cannot be meaningfully completed in a checkout without
the trusted certificates, Apple membership/API key, and matching native
runners. Record the workflow run URL and the pass/fail result of each platform
gate without copying secret values into the report.

## Ownership boundary

This runbook covers release signing, notarization, artifact metadata, and
native release verification for issue #509. Issue #511 owns testing that an
installed older signed build can update to a newer signed build. This document
does not claim that installed-build update testing or runtime updater behavior
is implemented.
