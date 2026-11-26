---
name: cutting-release
description: Use when the user asks to prepare or publish a stable OrkWorks release, or operate its nightly release workflow.
---

# Cutting a Release

Use the current [release specification](../../specs/release-pipeline.md) and
[signed release runbook](../../docs/agents/release-signing.md). The checked-in
[workflow](../../.github/workflows/release.yml) owns the command sequence and
artifact gates. Source tests establish wiring; only the credential-backed native
run establishes the requested release's signing and artifact verification.

Work within the requested version, channel and actions. Preparing source or
release notes does not itself authorize tag creation, a nightly dispatch,
protected-environment changes or publication. Reuse existing authorization for
covered steps; resolve missing scope before a dependent external action.

## Prepare the source

Follow the repository's [branch and PR workflow](../../AGENTS.md#branch-and-pr-workflow),
using `starting-work` before edits. For a stable release, update the desktop and
sidecar version declarations and affected lock entries together. Use pnpm for
Node package management and preserve unrelated dependencies. Run the relevant
checks, complete the PR's review/CI gates, and identify the exact merged source
SHA before tagging.

A canonical stable tag is `vMAJOR.MINOR.PATCH` and must match the desktop package
version. Create the requested tag only at the authorized source commit. The
stable path freezes that SHA, runs Main CI, then builds macOS arm64 and Windows
x64 through the protected `release` environment.

Nightlies use the workflow's scheduled/manual path from `main`; their temporary
versions are staged in CI. Follow the runbook for an authorized dispatch rather
than creating a stable tag or changing source versions to imitate a nightly.
The runbook distinguishes resuming the matching draft in the same publication
job from starting a new immutable attempt. Reuse authorization that already
covers the selected recovery path; resolve scope only when an additional
attempt or retry falls outside it.

## Verify and publish

Record the exact run, source SHA, tag and release URL. Check that Main CI and
both platform jobs passed. Missing credentials or environment approval must be
resolved through the protected release process, without weakening its gates.

Current source requires signed/notarized macOS output and Authenticode-signed
Windows output, with native checks before upload. Verify the exact expected
asset set, checksums, updater metadata, bundled sidecar/resources and native
verification evidence as described in the runbook. A local package build is
not a substitute for those release-run receipts.

The stable publisher creates a **draft**. Add and check the release notes before
an authorized publication. The nightly publisher validates its exact draft and
asset set before publishing the immutable prerelease. Do not count a skipped
job, draft, or successful source-only check as a published, verified release.

### Rollup upgrade note for the first containing release

While [issue 717](https://github.com/Rambolarsen/orkworks/issues/717) remains open,
include the following points in the first stable release containing the
rollup-v2 change. Check the actual published GitHub Release body before marking
that issue complete; source documentation alone does not satisfy publication.
The [Taskmaster guide](../../docs/user/taskmaster.md#model-choices-and-analysis-status)
carries the corresponding source-build warning.

> Taskmaster rollup cards now include a model-written proposed change. An
> unusable rollups section degrades gracefully while valid individual
> recommendations still apply. Upgrading from an earlier rollup prompt changes
> the rollup evaluation cache key: each affected workspace may need a one-time
> re-evaluation on its next eligible analysis, using the configured Taskmaster
> provider. Metered providers may charge for that run; automatic analyses still
> count against the daily allowance.

Keep the warning scoped to affected rollup analyses. It does not mean every
workspace starts inference at upgrade time, and it does not add a separate
rollup provider call. Preserve the distinction between current source and the
features verified in the release being published.

## Failure and recovery

Inspect the failed stage and retain the run, source, tag and draft identities
before selecting a correction or retry. Respect the existing authorization and
any finite retry allowance; a failed build grants no additional release scope.

For nightlies, retain immutable attempt tags, failed drafts and existing assets.
Use the runbook's exact matching-draft/job-rerun behavior; never delete,
retarget, overwrite or replace that retained history as a retry shortcut.
For a stable failure, reconcile the requested version, exact source and
publication state before selecting recovery. Do not treat a failed job as
permission to rewrite a published tag or release. Report unresolved gates and
the concrete handoff without claiming background monitoring.
