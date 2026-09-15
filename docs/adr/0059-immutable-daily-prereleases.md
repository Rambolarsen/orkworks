# Immutable daily prereleases

- Status: accepted
- Deciders: owner
- Date: 2026-09-15

## Context

OrkWorks needs a nightly distribution channel for signed Apple Silicon macOS
and Windows x64 builds from `main`. The existing stable flow starts from a
manual version tag and publishes a draft GitHub Release, while a scheduled
flow must select its source without allowing mutable branch state, damaged
releases, or stable-channel metadata to become an updater candidate.

Nightly publication also crosses a GitHub authorization boundary. Creating a
tag for a commit whose tree changes workflow files can require Workflows write
permission in addition to Contents write permission. The repository's default
`GITHUB_TOKEN` is read-only and cannot be treated as sufficient authority for
that operation. Signing credentials and release-write credentials must remain
unavailable to an ineligible manual dispatch or tag.

## Decision

Scheduled and manually dispatched nightly runs execute only from `main` and
freeze the workflow event's `github.sha` as the single source for validation,
tagging, packaging, and publication. Every successful nightly gets a unique,
immutable SemVer tag and a published GitHub prerelease. The workflow never
moves a rolling tag, retargets an existing tag, replaces assets, or treats a
draft or incomplete release as success.

Stable packages retain the `latest` channel. Nightly packages use the explicit
custom `nightly` electron-updater channel and publish only `nightly.yml` and
`nightly-mac.yml`. Publication and duplicate detection both require the exact
tag target, source marker, asset-name set, nonempty GitHub SHA-256 digests,
checksum manifest, and updater metadata to cross-check. Duplicate detection
downloads payloads only for releases claiming the current source and verifies
their updater SHA-512 values directly; draft publication checks the same values
against the locally verified payload bytes.

Every job that signs artifacts or receives `contents: write` authority uses the
protected `release` environment. The environment permits only `main` and
canonical stable tags. Stable releases use the workflow-scoped GitHub token.
Nightly tag and release operations use a protected, CI-only fine-grained token
with Contents and Workflows write permissions. Missing, expired, or
under-scoped credentials fail closed before packaging. The token is never
written into an artifact or application bundle.

Runtime update checking, channel selection in the installed application,
download, and installation remain the separate decision and implementation
owned by issue #511.

## Consequences

Nightly artifacts can be traced to one immutable, Main-CI-validated source and
cannot enter the stable feed. Failed attempts leave diagnostic drafts and tags
instead of mutating or hiding history, and a later run attempt can retry with a
new identity. Scheduled and manual nightly runs must be serialized and must
recheck remote state before publication.

The repository owner must maintain the `release` environment's deployment
policy and rotate a more capable CI-only token than the default workflow token.
Source tests can prove workflow wiring and release validation, but they cannot
prove the hosted environment policy, signing services, or token scope. At least
one credential-backed manual run is therefore required before issue #510 can
be considered delivered.

The artifact signing and native verification contract remains governed by
[ADR 0058](./0058-signed-release-artifacts-and-native-verification.md). The
detailed daily-release behavior remains governed by
[`specs/release-pipeline.md`](../../specs/release-pipeline.md).
