# Daily release implementation design

**Status:** approved in-session  
**Date:** 2026-09-15  
**Tracking:** [issue #510](https://github.com/Rambolarsen/orkworks/issues/510)

## Purpose

Publish one verified nightly OrkWorks prerelease from an immutable `main`
commit, without changing the manually tagged stable release contract. This
implements the daily-distribution unit in `specs/release-pipeline.md`; runtime
application updates remain issue #511.

## Approach

Extend `.github/workflows/release.yml` into one stable/nightly pipeline instead
of duplicating the signing and packaging recipe or introducing a reusable
workflow. Tag pushes keep the stable path. The 03:23 UTC schedule and manual
dispatch select the nightly path and always resolve `main`, never an arbitrary
branch.

A small preparation job produces the immutable source SHA, channel, package
version, numeric native build version, unique tag, release notes, and a skip
decision. It treats only a successfully published nightly whose recorded source
SHA matches as complete. Drafts, failed attempts, and merely existing tags do
not suppress a retry.

## Workflow

1. The preparation job resolves `origin/main` once and records the SHA used by
   every downstream checkout. A concurrency group serializes nightly
   publication.
2. Before expensive work, preparation searches published prereleases for the
   exact source-SHA marker. A match ends the nightly as an intentional no-op.
3. A validation job checks out the resolved SHA and runs the full desktop and
   Rust validation used by `main-ci.yml`.
4. The existing macOS arm64 and Windows x64 build jobs check out that same SHA.
   For nightlies they stage matching desktop/Rust SemVer and native numeric
   versions only in the runner workspace. Nothing is committed to `main`.
5. Both platforms retain the current protected signing environment, native
   signature verification, updater metadata validation, checksum generation,
   and Windows installer smoke test.
6. Publication rechecks for an already-published nightly from the source SHA
   while holding the nightly concurrency slot. It assembles both platforms,
   asserts nightly-specific updater metadata, and publishes one immutable
   GitHub prerelease with a unique tag and source identity. Stable releases keep
   their existing draft behavior and stable metadata.

## Version and channel identity

The nightly SemVer is derived from the stable `package.json` base version, UTC
date, GitHub run ID, and run attempt. It is strictly newer across retries while
remaining a prerelease of the stable base. A separate four-component numeric
version is supplied where native packaging requires it; each component is
validated against platform bounds.

Stable and nightly metadata use distinct channel names. A stable build cannot
consume nightly metadata, and a nightly build is configured to discover
prereleases without moving or replacing the stable `latest` release.

## Failure behavior

Validation or either platform failure prevents publication. Assets are staged
in a draft-like prepublication state and become visible only after the complete
set passes. A failed run remains retryable. Concurrent schedule/manual runs are
serialized and the later run exits cleanly if the earlier one published the
same SHA. Existing releases and tags are never overwritten or deleted.

## Test strategy

A dependency-free Node helper owns deterministic release identity and published
release selection so source tests can cover:

- nightly SemVer and native-version validation and ordering;
- stable/nightly channel separation;
- published versus draft/failed duplicate detection;
- rerun and run-attempt identity;
- malformed GitHub release data.

Workflow contract tests assert the schedule, manual dispatch, immutable checkout,
full validation dependency, concurrency, platform matrix, signing gates,
prepublication recheck, prerelease flags, and complete-asset publication. The
existing release metadata, artifact, and installer tests remain authoritative
for package contents and signatures.

One credential-backed manual run must finally demonstrate successful publication
and mixed stable/nightly discovery. That external run is recorded as delivery
evidence; source tests alone do not make the channel release-ready.

## Documentation

Update the release operator documentation with dispatch, skip/retry,
concurrency, version identity, publication inspection, and manual validation
steps. User-facing docs must describe nightly builds only after a real signed
nightly has been published and installed successfully.

## Non-goals

- Runtime checking, downloading, or installing updates (#511).
- Channel switching or downgrade support.
- Rolling tags, asset replacement, or automatic pruning.
- Linux or Intel macOS publication.
- A new release server, packaged credentials, or unsigned fallback.
