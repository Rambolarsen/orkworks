# Daily release implementation design

**Status:** approved in-session; amended after adversarial review

**Date:** 2026-09-15

**Tracking:** [issue #510](https://github.com/Rambolarsen/orkworks/issues/510)

## Purpose

Publish one verified nightly OrkWorks prerelease from an immutable `main`
commit, without changing the manually tagged stable release contract. This
implements the daily-distribution unit in `specs/release-pipeline.md`; runtime
application updates remain issue #511.

## Approach

Extend `.github/workflows/release.yml` into one stable/nightly pipeline instead
of duplicating the signing and packaging recipe. Reuse the existing Main CI
definition through `workflow_call`; keep the release-specific signing and
publication steps in the release workflow. Tag pushes keep the stable path. The
03:23 UTC schedule and manual dispatch select the nightly path and always use
the workflow commit from `main`, never an arbitrary branch.

A small nightly-only preparation job produces the immutable source SHA, channel,
package version, platform-specific native build versions, unique tag, release
notes, and a skip decision. It treats only a successfully published nightly
whose machine-readable source marker exactly matches as complete. Drafts,
failed attempts, and merely existing tags do not suppress a retry. Stable tag
pushes continue to build the pushed tag commit and retain the existing
tag/package-version guard.

## Workflow

1. One unprivileged preflight runs before any environment-bearing job. A stable
   tag push freezes `github.sha`, requires the canonical stable-tag grammar and
   matching package version, and bypasses nightly deduplication and concurrency.
   Nightly tags are excluded from the stable tag trigger. A schedule or manual
   dispatch is accepted only when the executing workflow ref is
   `refs/heads/main`; the first preflight step visibly fails other refs before
   checkout or dependency installation. Its initial `github.sha` is the one
   immutable source for every nightly checkout. The source is never re-resolved
   during the run.
2. The `release` environment must restrict deployment to the default branch and
   stable release tags before credentials are provisioned. The nightly guard is
   tested in source and runs before any environment-bearing job, preventing an
   accidental dispatch of a stale branch workflow from reaching signing jobs.
   Every signing or `contents: write` job declares that environment. Because
   deployment-branch policy is GitHub-hosted state, operator validation checks
   the live rule; repository tests can only prove the workflow references it.
3. A nightly concurrency group with `cancel-in-progress: false` prevents two
   runs from publishing concurrently. Before expensive work, preparation walks
   every page of GitHub releases and the complete `tags/v` matching-ref
   snapshot. Every public canonical nightly version, including a dangling tag
   left by a failed run before Release creation,
   constrains ordering even if its prerelease flag is damaged; only releases
   with the required prerelease flag can count as successful delivery. It
   validates every eligible nightly carrying an exact
   `<!-- orkworks-nightly-source:<40-lowercase-hex> -->` body line.
   Validation requires the expected prerelease/tag grammar, exact tag target,
   complete expected asset-name set, nonzero asset sizes and GitHub SHA-256
   digests, plus downloadable channel metadata and `SHA256SUMS.txt` whose
   contents cross-check those assets. Authenticated pagination follows only a
   validated next-page URL for the same repository endpoint. API, pagination, schema, or multiple
   fully validated releases for one source fails closed. An authenticated asset URL that
   returns 404 marks that published release as damaged, retryable history;
   every other download failure fails closed. One valid exact-source match ends
   the nightly as an intentional no-op before Main CI or packaging; a damaged
   or partial published release is not success and remains retryable under a
   new tag. To keep the scan
   bounded, full asset downloads apply only to releases claiming the current
   source SHA; release entries must expose boolean `draft` and `prerelease`
   flags, and those candidates also download the two updater payloads and
   verify each metadata SHA-512 against its payload bytes.
   Every public SemVer-valid tag on the `nightly` channel constrains ordering,
   even when it fails the stricter OrkWorks identity and delivery checks.
4. `main-ci.yml` exposes its existing desktop and Rust jobs through
   `workflow_call`, accepting an optional immutable checkout SHA. Normal Main CI
   triggers use their event SHA; nightly release calls the same workflow with
   its identical caller/source `github.sha`. GitHub therefore loads both the
   release and callable Main CI definitions from the commit whose source is
   validated and packaged. This makes validation parity structural rather than
   copied prose.
5. The existing macOS arm64 and Windows x64 build jobs use the event SHA for a
   stable tag and the prepared SHA for a nightly. Nightlies stage matching
   desktop/Rust SemVer and native versions only in the runner workspace.
   Nothing is committed to `main`.
6. Both platforms retain the current protected signing environment, native
   signature verification, updater metadata validation, checksum generation,
   and Windows installer smoke test.
7. After the initial eligibility check, privileged preparation creates the unique
   `refs/tags/v<nightly-semver>` directly through GitHub's Git-reference API at
   frozen SHA and reads it back. It uses a protected, CI-only fine-grained token
   with repository Contents and Workflows write permissions, because GitHub's
   `GITHUB_TOKEN` cannot create a ref/release for a historical commit that
   differs under `.github/workflows/`. Missing, expired, or under-scoped
   credentials fail closed and are documented for rotation; the token is never
   packaged. A definitive 401/403 fails immediately. On an ambiguous transport
   or server response, preparation reads the same tag, adopts it when it now
   targets the frozen SHA, and retries creation up to three times only while it
   remains absent. An existing
   exact tag is adopted only when it resolves to the frozen SHA; a mismatch or
   exhausted retry fails without force-updating or deleting a ref. When the
   exact candidate tag already exists, a duplicate-ref request must return
   GitHub's authorized 422 response before packaging; a read-only token returns
   403 and fails early without mutating the ref. It never
   changes source SHA. Tag existence alone never means publication succeeded.
8. Nightly publication repeats the exhaustive success check while holding the
   concurrency slot. After verifying that the prepared tag still targets the
   frozen source, it excludes only that candidate from the ordering comparison;
   this permits publication and failed-job retries while every other public
   nightly tag continues to constrain ordering. If still eligible, it creates a
   real draft prerelease for the already-existing verified tag, omitting
   `target_commitish` so the release API cannot retarget it. A retry reuses the
   sole exact-tag draft only when its prerelease flag, source marker, upload
   endpoint, and already-uploaded asset sizes and SHA-256 digests match the
   local candidate. Multiple exact-tag drafts or any mismatch fail closed.
9. The job uploads only assets missing from the selected draft, then
   applies the same full success predicate used for duplicate detection: exact
   expected names, nonzero sizes and GitHub SHA-256 digests, downloaded channel
   metadata and checksum-manifest cross-checks, direct updater SHA-512 checks
   against the locally verified payload bytes, source marker, prerelease/draft
   flags, and exact tag target. Credentialed asset downloads accept only the
   repository's GitHub API asset endpoint. A documented `302` is followed only
   to an HTTPS GitHub-content host, without forwarding authorization, and any
   further redirect is rejected. Only then does it publish the draft. Failed drafts and their unique tags remain diagnostic
   records: the same publication job can resume an exact retained draft, while
   a later run attempt can use a new tag. No asset is replaced.
   Stable releases keep their existing draft behavior and stable metadata.
10. Before draft creation, the same exhaustive release snapshot proves the
   candidate SemVer is greater than every published nightly version. An
   out-of-order queued run fails without publishing a lower version; it cannot
   make a newer run disappear from GitHub-provider discovery.

## Version and channel identity

The nightly SemVer is
`<stable-base>-nightly.<YYYYMMDD>.<run-id>.<run-attempt>`. All numeric strings
are canonicalized without leading zeroes and validated before editing package
files. A valid already-published release for the frozen SHA is detected before
constructing a new identity, so an unchanged-SHA no-op remains valid even after
the finite native version ranges are exhausted. Within one stable base it
orders by UTC date, run ID, then rerun attempt;
a stable-base increment orders above every nightly of the prior base. The stable
release of the same base remains SemVer-greater than its prereleases, but it is
not a nightly candidate because the feeds are isolated.

Native identifiers do not reuse one cross-platform format:

- Windows `buildVersion` is
  `<UTC-year>.<UTC-day-of-year>.<run-number>.<run-attempt>`. Each component must
  fit an unsigned 16-bit integer; overflow fails before packaging. The tuple is
  collision-free and increasing for the supported lifetime of the workflow.
- macOS `mac.bundleVersion` encodes the ordinal
  `(run-number - 1) * 100 + run-attempt` as a base-100
  `<major>.<minor>.<patch>` tuple, offsetting `major` by one. The helper requires
  attempts `1..99`, validates Apple's four/two/two digit bounds, and fails when
  the finite encoding is exhausted. It is collision-free and increasing within
  those explicit bounds.

The shared UTC year is limited to four canonical digits (`1000..9999`) so the
numeric `YYYYMMDD` prerelease identifier is valid SemVer without leading
zeroes. The day-of-year calculation sets the full UTC year explicitly.

Stable packaging retains `latest.yml` and `latest-mac.yml`. Nightly packaging
sets the fixed GitHub publisher channel to `nightly` and keeps
`generateUpdatesFilesForAllChannels` false, producing only `nightly.yml` and
`nightly-mac.yml`. Artifact verification parses each packaged `app-update.yml`
and requires the GitHub provider identity (`Rambolarsen/orkworks`) plus the
expected channel; only an absent stable channel is normalized to `latest`
(`null` is rejected), while nightly must be explicit. `nightly` is intentionally a custom channel, not electron-updater's
hierarchical `alpha` or `beta`: with `allowPrerelease` and the explicit custom
channel, the GitHub provider matches only tags whose first prerelease identifier
is `nightly`. Add the pinned `electron-updater` version as a development
dependency and exercise its GitHub provider with an injected mixed
stable/nightly Atom-feed transport; do not copy its selector into a local
approximation. Issue #511 promotes that same version to a runtime dependency and
configures nightly clients that way while keeping
downgrades disabled; stable clients use `latest`. Nightly publication asserts
the two nightly metadata names and rejects either stable metadata name, so a
same-base stable release cannot enter nightly discovery. This exact candidate
separation is also why platform-native build identifiers need monotonicity only
within their channel, not between stable and nightly packages.

The stable path accepts only `v<major>.<minor>.<patch>` with canonical numeric
components and no prerelease or build suffix. Any `v*` tag outside that grammar
fails before credentials, packaging, or draft publication, even when it matches
`package.json`.

## Failure behavior

Validation or either platform failure prevents draft creation. Assembly or
upload failure leaves an unpublished draft; update clients cannot see drafts.
A rerun of that publication job resumes the matching draft without replacing
uploaded assets; a later run remains retryable under a new run-attempt tag. Concurrent
schedule/manual runs are serialized and the later run exits cleanly if the
earlier one published the same SHA. Existing releases, tags, and assets are
never overwritten or deleted.

## Test strategy

A dependency-free Node helper owns deterministic release identity and published
release selection so source tests can cover:

- nightly SemVer plus platform-specific native-version bounds, collisions, and
  ordering;
- exact `latest*`/`nightly*` channel separation and mixed-release selection
  through the pinned GitHub provider behavior;
- exact source-marker parsing, paginated published/draft filtering, duplicate
  ambiguity, remote asset/metadata/checksum validation, and fail-closed
  API/schema behavior;
- rerun and run-attempt identity;
- canonical stable-tag rejection of prerelease/build suffixes;
- malformed GitHub release data.

Workflow contract tests assert the schedule, main-ref dispatch guard, environment
reference on privileged jobs, event-specific immutable checkout, callable Main CI
dependency, concurrency, platform matrix, signing gates, paginated
prepublication recheck, exact tag target, real draft lifecycle, prerelease
flags, and complete-asset publication. The
existing release metadata, artifact, and installer tests remain authoritative
for package contents and signatures.

One credential-backed manual run must finally demonstrate successful publication
and mixed stable/nightly discovery. That external run is recorded as delivery
evidence; source tests alone do not make the channel release-ready.

## Documentation

Update the release operator documentation with dispatch, skip/retry,
concurrency, version identity, publication inspection, and manual validation
steps, including the required `release` environment deployment-branch policy
and CI-only fine-grained token permissions/rotation.
User-facing docs must describe nightly builds only after a real signed nightly
has been published and installed successfully.

## Non-goals

- Runtime checking, downloading, or installing updates (#511).
- Channel switching or downgrade support.
- Rolling tags, asset replacement, or automatic pruning.
- Linux or Intel macOS publication.
- A new release server, packaged credentials, or unsigned fallback.
