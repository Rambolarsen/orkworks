# Release Signing and Notarization Design

**Date:** 2026-09-14
**Issue:** [#509](https://github.com/Rambolarsen/orkworks/issues/509)
**Status:** Approved for implementation planning

## Goal

Upgrade the existing tag-driven Electron release pipeline from unsigned alpha
artifacts to signed release artifacts that are ready for the later in-app
updater work. The macOS app bundle will be Developer ID signed, notarized, and
stapled before it is packaged in the DMG and ZIP. Windows releases will be
Authenticode signed and verified. macOS ZIP
artifacts and electron-builder update metadata will be published alongside the
existing DMG and NSIS artifacts.

This unit wires the build and verification path, including verification of the
actual downloadable/installable artifacts. It does not implement runtime
update checks, downloads, restart/install UI, nightly publication, channel
selection, or older-installed-build-to-newer-build update testing. Runtime
updating and installed update testing remain issue #511; nightly publication
remains issue #510.

## Decisions

- Use the existing `electron-builder` integration rather than custom signing
  services or post-sign scripts.
- Use an App Store Connect API key for macOS notarization. The API key is
  supplied as base64-encoded `.p8` content, together with its key ID, issuer
  ID, and Apple team ID.
- Use a base64-encoded Windows `.pfx`/`.p12` certificate through
  `WIN_CSC_LINK` and `WIN_CSC_KEY_PASSWORD`.
- Put signing secrets in a protected GitHub Actions `release` environment.
  They are exposed only to the tag-driven release workflow and never to pull
  request builds.
- Set platform-specific `forceCodeSigning: true` for macOS and Windows. Linux
  remains a local-only unsigned packaging target.
- Keep `--publish never` in the platform build jobs. They produce complete
  local artifacts and metadata; the existing publish job uploads them only
  after every platform has passed verification.
- Require the platform jobs to use the protected GitHub Actions `release`
  environment. The repository must configure required reviewers for that
  environment and restrict the release workflow to protected `v*` tags before
  signing credentials are added.
- Do not hard-code a Windows publisher name before a certificate exists.
  electron-builder derives the publisher from the signing certificate and
  embeds it for updater verification. CI will compare the signed subject to
  the required `WIN_EXPECTED_PUBLISHER` release-environment variable once the
  certificate is provisioned.

## Configuration

Update `apps/desktop/electron-builder.yml` as follows:

- Add top-level `publish.provider: github` with owner `Rambolarsen` and repo
  `orkworks`.
- Change the macOS targets to `dmg` and `zip`.
- Enable `mac.notarize` and `mac.hardenedRuntime`.
- Add minimal parent and inherited entitlement files under
  `apps/desktop/build/` containing Electron's JIT and unsigned-executable
  memory entitlements.
- Set `mac.binaries` to an explicit list containing
  `Contents/Resources/orkworksd`, the path of the bundled Rust executable
  relative to the `.app` bundle.
- Set `mac.forceCodeSigning: true`.
- Keep the Windows NSIS target and set `win.verifyUpdateCodeSignature: true`
  and `win.forceCodeSigning: true`.

The configuration will continue to package the Rust sidecar, hook scripts,
knowledge resources, and icons exactly as it does today. No certificate path,
password, API key, or publisher identity is stored in the repository.

## Secret and environment contract

The protected GitHub `release` environment will provide these values:

| Environment secret/variable | Mapped build variable | Purpose |
| --- | --- | --- |
| `MAC_CSC_LINK` | `CSC_LINK` | Base64-encoded Developer ID Application `.p12` |
| `MAC_CSC_KEY_PASSWORD` | `CSC_KEY_PASSWORD` | macOS certificate password |
| `APPLE_API_KEY` | `APPLE_API_KEY` | Base64-encoded App Store Connect `.p8` |
| `APPLE_API_KEY_ID` | `APPLE_API_KEY_ID` | App Store Connect key ID |
| `APPLE_API_ISSUER` | `APPLE_API_ISSUER` | App Store Connect issuer ID |
| `APPLE_TEAM_ID` | `APPLE_TEAM_ID` | Apple Developer team ID |
| `WIN_CSC_LINK` | `WIN_CSC_LINK` | Base64-encoded Authenticode `.pfx`/`.p12` |
| `WIN_CSC_KEY_PASSWORD` | `WIN_CSC_KEY_PASSWORD` | Windows certificate password |
| `WIN_EXPECTED_PUBLISHER` | verifier input | Exact subject/publisher expected from the Windows certificate |

The workflow maps the macOS certificate secret to electron-builder's shared
`CSC_*` variables only in the macOS job and maps the Windows certificate to
`WIN_CSC_*` only in the Windows job. The App Store Connect values are injected
only in the macOS job. Secret values are never printed, written to tracked
files, or passed to the packaged application.

`APPLE_TEAM_ID` is retained in the macOS environment contract because it
identifies the Apple Developer team associated with the Developer ID
certificate and notarization key. The API-key activation set is
`APPLE_API_KEY`, `APPLE_API_KEY_ID`, and `APPLE_API_ISSUER`; the team ID is
also supplied as required by the selected electron-builder/notarization
configuration.

`WIN_EXPECTED_PUBLISHER` is a protected environment variable rather than a
secret: it must equal the exact publisher subject/CN represented by the
Windows certificate. The verifier will compare the certificate subject and
the generated `app-update.yml` publisher value with this variable. This binds
the artifact to the intended identity without guessing that identity in
source control.

The operator documentation will explain how to export and base64-encode the
`.p12`, `.pfx`, and `.p8` files, create the protected environment, and rotate
credentials. It will explicitly state that self-signed Windows certificates
are for local testing only and do not satisfy this release gate.

## Release data flow

For each tagged release:

1. The platform jobs check out the tag and verify it matches the desktop
   package version.
2. Each job builds the frontend and target Rust sidecar.
3. The job invokes the existing `pnpm package:release` command with its
   platform-specific signing variables and no GitHub publishing token.
4. electron-builder signs the app, nested Electron code, and Rust sidecar;
   notarizes and staples the macOS app; and generates the DMG, ZIP, NSIS
   installer, `latest*.yml`, and blockmap metadata.
5. A pre-checksum release-verifier mode checks the expected artifacts, updater
   metadata, and packaged resources, skipping only the checksum manifest that
   does not exist yet. Platform-native signature checks then validate the
   actual signatures and certificate chains in the staged app.
6. The native smoke checks validate the actual downloadable artifacts: macOS
   mounts the DMG and extracts the ZIP before checking the contained app;
   Windows runs the existing installer smoke flow against the signed NSIS
   artifact and checks the installed app and sidecar.
7. A Node checksum helper writes `SHA256SUMS.txt` for all distributable files.
8. The full release verifier runs again and requires the generated checksum
   manifest.
9. The job uploads the distributables and metadata as a workflow artifact.
10. The existing publish job creates the draft GitHub release only after both
   platform jobs succeed.

The platform jobs remain independent for build execution, but `publish.needs`
continues to require the complete matrix. A signing, notarization, metadata,
checksum, or verification failure therefore prevents publication and cannot
replace the last usable release with a partial one.

## Verification

The final `verifyReleaseArtifact.mjs` contract will be extended to require:

- macOS DMG, ZIP, `latest-mac.yml`, ZIP blockmap, `SHA256SUMS.txt`, unpacked
  app-update metadata, unpacked app, signed Rust sidecar, hook scripts, and
  knowledge resources;
- Windows NSIS, `latest.yml`, installer blockmap, `SHA256SUMS.txt`, unpacked
  app-update metadata, unpacked app, signed app executable, signed Rust
  sidecar, hook scripts, and knowledge resources.

The metadata verifier will parse each `latest*.yml` and assert that its version
matches the package version, every referenced artifact exists, every declared
SHA-512 digest matches the referenced file, and the blockmap points at the
same artifact. The checksum helper will sort entries deterministically and
will exclude `SHA256SUMS.txt` from its own manifest.

The explicit pre-checksum verifier mode skips only `SHA256SUMS.txt`; every
other artifact, metadata, digest, blockmap, and packaged-resource check remains
enabled. After checksum generation, the workflow runs the default verifier as
a final gate before upload, and that default mode always requires the manifest.

The release workflow will run these native checks before artifact upload:

- macOS: mount the produced DMG read-only and extract the produced ZIP; for
  each contained app run `codesign --verify --deep --strict --verbose=2`,
  `spctl --assess --type execute`, and `xcrun stapler validate`. Verify the
  Rust sidecar directly as an in-bundle executable. The DMG container itself
  is not stapler-validated because electron-builder staples the app bundle,
  not the DMG. Detach the DMG in cleanup even when validation fails.
- Windows: run the existing installer smoke test against the exact signed
  NSIS artifact, then run `signtool verify /pa /all /v` or the runner's
  equivalent native Authenticode verifier on the installer, installed app
  executable, and installed Rust sidecar. Compare the certificate subject and
  generated `resources/app-update.yml` publisher with
  `WIN_EXPECTED_PUBLISHER`.

No source-only test will claim that an artifact is trusted. The repository can
test configuration, metadata contents, checksum generation, and workflow
wiring without credentials; actual signing, notarization, certificate-chain,
stapling, DMG/ZIP validation, and installer validation require the trusted
release environment and real platform runners.

Because `forceCodeSigning` is platform-specific, release-equivalent macOS and
Windows packaging intentionally requires signing credentials. Linux packaging
and normal local development commands are unaffected; there is no need to add
an unsigned release mode that could weaken the tagged release gate.

## Tests

Add or extend Node tests to pin:

- macOS target ordering and artifact names for DMG and ZIP;
- Windows NSIS target and update-signature verification;
- explicit sidecar signing configuration;
- platform-specific forced signing;
- GitHub provider ownership and repository;
- expected updater metadata, blockmaps, and checksum manifest;
- metadata versions, referenced filenames, SHA-512 digests, blockmap targets,
  deterministic checksum ordering, and exclusion of the checksum file from
  itself;
- release workflow secret mapping and ordering of packaging, pre-checksum
  verification, native verification, checksum generation, final verification,
  and upload;
- rejection of unsigned/missing expected files in the release verifier.

The platform smoke tests will additionally prove that the signed DMG and ZIP
contain a verifiable app and that the signed NSIS installer produces a
verifiable installed app and sidecar. The Windows tests will pin the
`app-update.yml` publisher identity to `WIN_EXPECTED_PUBLISHER`.

The existing desktop test commands remain the local verification baseline. A
real credential-backed macOS and Windows release run is an external delivery
prerequisite and must be recorded separately from source-test results.

## Documentation

Update the release/operator documentation and user-facing installation notes
to distinguish:

- the current repository state before credentials are provisioned;
- the signed DMG/ZIP/NSIS artifacts produced after the protected environment
  is configured; and
- the fact that runtime in-app updating is not enabled by this unit.

The architecture documentation will record that release signing and
notarization are performed by electron-builder in CI, while runtime updater
network operations remain an Electron-main concern for #511.

## Risks and mitigations

- **No certificate yet:** the workflow is intentionally fail-closed until the
  owner provisions a Developer ID certificate, App Store Connect API key, and
  Windows Authenticode certificate.
- **Certificate rotation:** the expected Windows publisher is an environment
  value rather than a guessed source constant. A future rotation must update
  that value and verify updater compatibility before release.
- **Nested sidecar signing:** `mac.binaries` names the final in-bundle path and
  native verification checks the sidecar directly.
- **Partial publication:** the publish job remains matrix-gated, and all
  metadata/checksums are staged before upload.
- **Credential leakage:** secrets stay in protected environment variables;
  diagnostics and logs contain no credentials, release tokens, or session
  content.

## References

- [Release pipeline specification](../../../specs/release-pipeline.md)
- [Issue #509](https://github.com/Rambolarsen/orkworks/issues/509)
- [electron-builder macOS signing](https://www.electron.build/v26/docs/mac/)
- [electron-builder notarization](https://www.electron.build/v26/docs/notarization/)
- [electron-builder Windows signing](https://www.electron.build/v26/docs/features/code-signing/code-signing-win/)
