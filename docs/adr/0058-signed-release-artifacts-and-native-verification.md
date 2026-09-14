# Signed release artifacts and native verification

- Status: accepted
- Deciders: owner
- Date: 2026-09-14

## Context

OrkWorks distributes desktop artifacts directly through GitHub Releases. macOS
Gatekeeper and electron-updater require a stable trusted code signature, while
Windows update verification must bind the installer and installed executables
to the expected publisher. The release pipeline also needs to prevent malformed
or substituted update payloads from being published. Signing credentials are
security-sensitive and are unavailable to pull-request or non-release jobs.

## Decision

Tag-driven release jobs run in a protected GitHub Environment and build each
platform with its own credentials. macOS releases use a Developer ID
certificate, hardened runtime with Electron's `allow-jit` entitlement,
notarization, stapling, and nested sidecar signing. Windows releases use a
password-protected Authenticode certificate and verify the exact configured
publisher identity. The workflow verifies native signatures and installer
contents before generating deterministic SHA-256 checksums and publishing
allowlisted artifacts. Update metadata, payload names, sizes, digests, and
blockmaps are validated before upload; the ZIP is the macOS updater payload and
the DMG remains a manual-download artifact.

Signing values are injected only into the platform build jobs. The Apple API
key is materialized as a temporary file and removed in cleanup; Windows
certificate material remains file-backed through electron-builder's supported
environment variables. Runtime auto-update behavior and nightly channels are
separate decisions owned by issues #511 and #510.

## Consequences

Release publication fails closed when credentials, publisher configuration,
native signatures, installer smoke tests, metadata, or checksums are invalid.
The pipeline requires an owner-managed certificate/key setup and macOS and
Windows runners for the final native checks, so source tests can verify wiring
but cannot prove a trusted certificate chain or Apple service result. Changes
to signing credentials, artifact formats, or update trust checks must update
this ADR and the [release-signing runbook](../agents/release-signing.md).

The detailed artifact contract remains in
[`specs/release-pipeline.md`](../../specs/release-pipeline.md), and this ADR
does not grant the application permission to update, install, or control
itself at runtime.
